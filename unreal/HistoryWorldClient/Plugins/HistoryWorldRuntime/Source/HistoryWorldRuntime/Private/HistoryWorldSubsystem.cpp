#include "HistoryWorldSubsystem.h"

#include "Dom/JsonObject.h"
#include "Dom/JsonValue.h"
#include "GenericPlatform/GenericPlatformHttp.h"
#include "HistoryWorldResourceSettings.h"
#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "Misc/LexicalConversion.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

namespace
{
constexpr TCHAR SnapshotContract[] = TEXT("historyworld.snapshot.v1");
constexpr int32 MinResponseBytes = 4 * 1024;
constexpr int32 MaxResponseBytes = 32 * 1024 * 1024;
constexpr int32 MaxCandidatePatches = 65'536;
constexpr int32 MaxGeneratedInstances = 100'000;
constexpr int64 MaxWorkUnits = 1'000'000;
constexpr double MaxExactJsonInteger = 9'007'199'254'740'991.0;

struct FParsedWorldInstant
{
    int64 Seconds = 0;
    int32 Nanoseconds = 0;
    FString SecondsText;
};

FString DetailToProtocol(const EHistoryWorldDetail Detail)
{
    switch (Detail)
    {
        case EHistoryWorldDetail::Planet: return TEXT("planet");
        case EHistoryWorldDetail::Continental: return TEXT("continental");
        case EHistoryWorldDetail::Regional: return TEXT("regional");
        case EHistoryWorldDetail::Local: return TEXT("local");
        case EHistoryWorldDetail::Street: return TEXT("street");
    }
    return FString();
}

bool DetailFromProtocol(const FString& Value, EHistoryWorldDetail& OutDetail)
{
    if (Value == TEXT("planet"))
    {
        OutDetail = EHistoryWorldDetail::Planet;
        return true;
    }
    if (Value == TEXT("continental"))
    {
        OutDetail = EHistoryWorldDetail::Continental;
        return true;
    }
    if (Value == TEXT("regional"))
    {
        OutDetail = EHistoryWorldDetail::Regional;
        return true;
    }
    if (Value == TEXT("local"))
    {
        OutDetail = EHistoryWorldDetail::Local;
        return true;
    }
    if (Value == TEXT("street"))
    {
        OutDetail = EHistoryWorldDetail::Street;
        return true;
    }
    return false;
}

bool IsLowerHex(const FString& Value, const int32 RequiredLength)
{
    if (Value.Len() != RequiredLength)
    {
        return false;
    }
    for (const TCHAR Character : Value)
    {
        if (!((Character >= TEXT('0') && Character <= TEXT('9'))
            || (Character >= TEXT('a') && Character <= TEXT('f'))))
        {
            return false;
        }
    }
    return true;
}

bool IsKnownEpistemicClass(const FString& Value)
{
    return Value == TEXT("direct_observation")
        || Value == TEXT("reported_primary_source")
        || Value == TEXT("scholarly_inference")
        || Value == TEXT("model_derived")
        || Value == TEXT("plausible_reconstruction")
        || Value == TEXT("artistic_ambient");
}

bool IsKnownDegradationReason(const FString& Value)
{
    return Value == TEXT("cell_resolution") || Value == TEXT("instance_budget");
}

bool ReadOptionalInteger(
    const TSharedPtr<FJsonObject>& Object,
    const TCHAR* Field,
    const double Minimum,
    const double Maximum,
    bool& bOutPresent,
    int64& OutValue,
    FString& OutError)
{
    bOutPresent = false;
    OutValue = 0;
    const TSharedPtr<FJsonValue>* JsonValue = Object->Values.Find(Field);
    if (JsonValue == nullptr || !JsonValue->IsValid() || (*JsonValue)->Type == EJson::Null)
    {
        return true;
    }

    double Number = 0.0;
    if (!(*JsonValue)->TryGetNumber(Number)
        || !FMath::IsFinite(Number)
        || FMath::Frac(Number) != 0.0
        || Number < Minimum
        || Number > Maximum
        || Number < -MaxExactJsonInteger
        || Number > MaxExactJsonInteger)
    {
        OutError = FString::Printf(TEXT("Snapshot field '%s' was not a bounded exact integer"), Field);
        return false;
    }

    bOutPresent = true;
    OutValue = static_cast<int64>(Number);
    return true;
}

bool ReadRequiredInteger(
    const TSharedPtr<FJsonObject>& Object,
    const TCHAR* Field,
    const double Minimum,
    const double Maximum,
    int64& OutValue,
    FString& OutError)
{
    bool bPresent = false;
    if (!ReadOptionalInteger(Object, Field, Minimum, Maximum, bPresent, OutValue, OutError))
    {
        return false;
    }
    if (!bPresent)
    {
        OutError = FString::Printf(TEXT("Snapshot omitted required integer field '%s'"), Field);
        return false;
    }
    return true;
}

bool DecodeInstant(
    const TSharedPtr<FJsonObject>& Object,
    const TCHAR* Context,
    FParsedWorldInstant& OutInstant,
    FString& OutError)
{
    if (!Object.IsValid() || !Object->TryGetStringField(TEXT("seconds"), OutInstant.SecondsText)
        || !LexTryParseString(OutInstant.Seconds, *OutInstant.SecondsText)
        || OutInstant.SecondsText != FString::Printf(TEXT("%lld"), OutInstant.Seconds))
    {
        OutError = FString::Printf(TEXT("%s.seconds was not a canonical signed 64-bit decimal"), Context);
        return false;
    }

    int64 Nanoseconds = 0;
    if (!ReadRequiredInteger(Object, TEXT("nanos"), 0.0, 999'999'999.0, Nanoseconds, OutError))
    {
        OutError = FString::Printf(TEXT("%s had invalid nanoseconds: %s"), Context, *OutError);
        return false;
    }
    OutInstant.Nanoseconds = static_cast<int32>(Nanoseconds);
    return true;
}

int32 CompareInstants(const FParsedWorldInstant& Left, const FParsedWorldInstant& Right)
{
    if (Left.Seconds != Right.Seconds)
    {
        return Left.Seconds < Right.Seconds ? -1 : 1;
    }
    if (Left.Nanoseconds != Right.Nanoseconds)
    {
        return Left.Nanoseconds < Right.Nanoseconds ? -1 : 1;
    }
    return 0;
}

bool DecodeOptionalInstant(
    const TSharedPtr<FJsonObject>& Object,
    const TCHAR* Field,
    bool& bOutPresent,
    FParsedWorldInstant& OutInstant,
    FString& OutError)
{
    bOutPresent = false;
    const TSharedPtr<FJsonValue>* JsonValue = Object->Values.Find(Field);
    if (JsonValue == nullptr || !JsonValue->IsValid() || (*JsonValue)->Type == EJson::Null)
    {
        return true;
    }
    const TSharedPtr<FJsonObject> InstantObject = (*JsonValue)->AsObject();
    if (!InstantObject.IsValid())
    {
        OutError = FString::Printf(TEXT("stable_time_segment.%s was not an object or null"), Field);
        return false;
    }
    bOutPresent = true;
    const FString Context = FString::Printf(TEXT("stable_time_segment.%s"), Field);
    return DecodeInstant(InstantObject, *Context, OutInstant, OutError);
}
}

void UHistoryWorldSubsystem::Deinitialize()
{
    CancelActiveRequest();
    Super::Deinitialize();
}

void UHistoryWorldSubsystem::AdvanceGeneration()
{
    Generation = Generation == MAX_int64 ? 1 : Generation + 1;
}

int64 UHistoryWorldSubsystem::RequestSnapshot(
    const FString& ReleaseId,
    const FHistoryWorldCell& Cell,
    const int64 AtSeconds,
    const EHistoryWorldDetail Detail)
{
    CancelActiveRequest();
    const int64 RequestGeneration = Generation;

    FString ValidationError;
    if (!IsLowerHex(ReleaseId, 64))
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Release ID must be a 64-character lowercase BLAKE3 digest"));
        return RequestGeneration;
    }
    if (!Cell.IsProtocolValid(ValidationError))
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, ValidationError);
        return RequestGeneration;
    }
    const FString DetailString = DetailToProtocol(Detail);
    if (DetailString.IsEmpty())
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Requested detail level was invalid"));
        return RequestGeneration;
    }

    const UHistoryWorldResourceSettings* Settings = GetDefault<UHistoryWorldResourceSettings>();
    FString BaseUrl = Settings->ServiceBaseUrl;
    BaseUrl.TrimStartAndEndInline();
    BaseUrl.RemoveFromEnd(TEXT("/"));
    if (!(BaseUrl.StartsWith(TEXT("http://")) || BaseUrl.StartsWith(TEXT("https://"))))
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Snapshot service URL must use HTTP or HTTPS"));
        return RequestGeneration;
    }

    const int32 OutputBytes = FMath::Clamp(Settings->MaxHttpResponseBytes, MinResponseBytes, MaxResponseBytes);
    const int32 CandidatePatches = FMath::Clamp(Settings->MaxCandidatePatches, 1, MaxCandidatePatches);
    const int32 GeneratedInstances = FMath::Clamp(Settings->MaxGeneratedInstances, 1, MaxGeneratedInstances);
    const int64 WorkUnits = FMath::Clamp(Settings->MaxWorkUnits, int64{1}, MaxWorkUnits);
    const FString ExpectedCell = Cell.ToProtocolString();
    const FString ExpectedAtSeconds = FString::Printf(TEXT("%lld"), AtSeconds);
    const FString Url = FString::Printf(
        TEXT("%s/v1/releases/%s/snapshot?cell=%s&at_seconds=%lld&detail=%s&max_candidate_patches=%d&max_output_bytes=%d&max_instances=%d&max_work_units=%lld"),
        *BaseUrl,
        *FGenericPlatformHttp::UrlEncode(ReleaseId),
        *FGenericPlatformHttp::UrlEncode(ExpectedCell),
        AtSeconds,
        *DetailString,
        CandidatePatches,
        OutputBytes,
        GeneratedInstances,
        WorkUnits);

    ActiveRequest = FHttpModule::Get().CreateRequest();
    ActiveRequest->SetURL(Url);
    ActiveRequest->SetVerb(TEXT("GET"));
    ActiveRequest->SetTimeout(FMath::Clamp(Settings->RequestTimeoutSeconds, 1.0f, 300.0f));
    ActiveRequest->SetHeader(TEXT("Accept"), TEXT("application/json"));
    ActiveRequest->OnProcessRequestComplete().BindUObject(
        this,
        &UHistoryWorldSubsystem::HandleResponse,
        RequestGeneration,
        ReleaseId,
        ExpectedCell,
        ExpectedAtSeconds,
        Detail);
    if (!ActiveRequest->ProcessRequest())
    {
        ActiveRequest->OnProcessRequestComplete().Unbind();
        ActiveRequest.Reset();
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Failed to start snapshot request"));
    }
    return RequestGeneration;
}

void UHistoryWorldSubsystem::CancelActiveRequest()
{
    AdvanceGeneration();
    TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> RequestToCancel = MoveTemp(ActiveRequest);
    if (RequestToCancel.IsValid())
    {
        RequestToCancel->OnProcessRequestComplete().Unbind();
        RequestToCancel->CancelRequest();
    }
}

void UHistoryWorldSubsystem::HandleResponse(
    TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> Request,
    TSharedPtr<IHttpResponse, ESPMode::ThreadSafe> Response,
    const bool bConnectedSuccessfully,
    const int64 RequestGeneration,
    FString ExpectedReleaseId,
    FString ExpectedCell,
    FString ExpectedAtSeconds,
    const EHistoryWorldDetail ExpectedDetail)
{
    (void)Request;
    if (RequestGeneration != Generation)
    {
        return;
    }
    ActiveRequest.Reset();
    if (!bConnectedSuccessfully || !Response.IsValid())
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Snapshot service did not respond"));
        return;
    }
    if (Response->GetResponseCode() != 200)
    {
        OnSnapshotFailed.Broadcast(
            RequestGeneration,
            FString::Printf(TEXT("Snapshot service returned HTTP %d"), Response->GetResponseCode()));
        return;
    }

    const UHistoryWorldResourceSettings* Settings = GetDefault<UHistoryWorldResourceSettings>();
    const int32 OutputBytes = FMath::Clamp(Settings->MaxHttpResponseBytes, MinResponseBytes, MaxResponseBytes);
    if (Response->GetContent().Num() > OutputBytes)
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Snapshot exceeded the client byte budget"));
        return;
    }

    FHistoryWorldSnapshot Snapshot;
    FString Error;
    if (!DecodeSnapshot(Response->GetContentAsString(), Snapshot, Error))
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, Error);
        return;
    }
    if (Snapshot.ReleaseId != ExpectedReleaseId
        || Snapshot.Cell != ExpectedCell
        || Snapshot.EvaluatedAtSeconds != ExpectedAtSeconds
        || Snapshot.EvaluatedAtNanoseconds != 0
        || Snapshot.RequestedDetail != ExpectedDetail
        || Snapshot.LayerMask != 255)
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Snapshot identity did not match the pinned request"));
        return;
    }

    const int32 CandidatePatches = FMath::Clamp(Settings->MaxCandidatePatches, 1, MaxCandidatePatches);
    const int32 GeneratedInstances = FMath::Clamp(Settings->MaxGeneratedInstances, 1, MaxGeneratedInstances);
    const int64 WorkUnits = FMath::Clamp(Settings->MaxWorkUnits, int64{1}, MaxWorkUnits);
    if (Snapshot.CandidatePatches > CandidatePatches
        || Snapshot.SuggestedInstanceCeiling > GeneratedInstances
        || Snapshot.WorkUnits > WorkUnits
        || Snapshot.ConservativeOutputBoundBytes > OutputBytes
        || Snapshot.ConservativeOutputBoundBytes < Response->GetContent().Num())
    {
        OnSnapshotFailed.Broadcast(RequestGeneration, TEXT("Snapshot exceeded its advertised resource budget"));
        return;
    }
    OnSnapshotReady.Broadcast(RequestGeneration, Snapshot);
}

bool UHistoryWorldSubsystem::DecodeSnapshot(
    const FString& Json,
    FHistoryWorldSnapshot& OutSnapshot,
    FString& OutError)
{
    TSharedPtr<FJsonObject> Root;
    const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Json);
    if (!FJsonSerializer::Deserialize(Reader, Root) || !Root.IsValid())
    {
        OutError = TEXT("Snapshot was not valid JSON");
        return false;
    }
    if (!Root->TryGetStringField(TEXT("contract"), OutSnapshot.Contract)
        || OutSnapshot.Contract != SnapshotContract
        || !Root->TryGetStringField(TEXT("materializer_version"), OutSnapshot.MaterializerVersion)
        || OutSnapshot.MaterializerVersion.IsEmpty()
        || OutSnapshot.MaterializerVersion.Len() > 128
        || !Root->TryGetStringField(TEXT("numeric_model_version"), OutSnapshot.NumericModelVersion)
        || OutSnapshot.NumericModelVersion.IsEmpty()
        || OutSnapshot.NumericModelVersion.Len() > 128
        || !Root->TryGetStringField(TEXT("prng_version"), OutSnapshot.PrngVersion)
        || OutSnapshot.PrngVersion.IsEmpty()
        || OutSnapshot.PrngVersion.Len() > 128
        || !Root->TryGetStringField(TEXT("release_id"), OutSnapshot.ReleaseId)
        || !Root->TryGetStringField(TEXT("local_dependency_hash"), OutSnapshot.LocalDependencyHash)
        || !Root->TryGetStringField(TEXT("semantic_state_hash"), OutSnapshot.SemanticStateHash)
        || !Root->TryGetStringField(TEXT("realization_seed"), OutSnapshot.RealizationSeed)
        || !IsLowerHex(OutSnapshot.ReleaseId, 64)
        || !IsLowerHex(OutSnapshot.LocalDependencyHash, 64)
        || !IsLowerHex(OutSnapshot.SemanticStateHash, 64)
        || !IsLowerHex(OutSnapshot.RealizationSeed, 64))
    {
        OutError = TEXT("Snapshot had an invalid contract or identity digest");
        return false;
    }
    if (!Root->TryGetStringField(TEXT("cell"), OutSnapshot.Cell))
    {
        OutError = TEXT("Snapshot omitted its cell identity");
        return false;
    }

    const TSharedPtr<FJsonObject>* EvaluatedAtObject = nullptr;
    FParsedWorldInstant EvaluatedAt;
    if (!Root->TryGetObjectField(TEXT("evaluated_at"), EvaluatedAtObject)
        || !DecodeInstant(*EvaluatedAtObject, TEXT("evaluated_at"), EvaluatedAt, OutError))
    {
        if (OutError.IsEmpty())
        {
            OutError = TEXT("Snapshot omitted evaluated_at");
        }
        return false;
    }
    OutSnapshot.EvaluatedAtSeconds = EvaluatedAt.SecondsText;
    OutSnapshot.EvaluatedAtNanoseconds = EvaluatedAt.Nanoseconds;

    const TSharedPtr<FJsonObject>* RealizationEpochObject = nullptr;
    FParsedWorldInstant RealizationEpoch;
    if (!Root->TryGetObjectField(TEXT("realization_epoch"), RealizationEpochObject)
        || !DecodeInstant(
            *RealizationEpochObject,
            TEXT("realization_epoch"),
            RealizationEpoch,
            OutError)
        || CompareInstants(RealizationEpoch, EvaluatedAt) > 0)
    {
        if (OutError.IsEmpty())
        {
            OutError = TEXT("Snapshot had an invalid realization_epoch");
        }
        return false;
    }
    OutSnapshot.RealizationEpochSeconds = RealizationEpoch.SecondsText;
    OutSnapshot.RealizationEpochNanoseconds = RealizationEpoch.Nanoseconds;

    const TSharedPtr<FJsonObject>* StableSegment = nullptr;
    FParsedWorldInstant StableStart;
    FParsedWorldInstant StableEnd;
    if (!Root->TryGetObjectField(TEXT("stable_time_segment"), StableSegment)
        || !DecodeOptionalInstant(
            *StableSegment,
            TEXT("start_inclusive"),
            OutSnapshot.bHasStableStart,
            StableStart,
            OutError)
        || !DecodeOptionalInstant(
            *StableSegment,
            TEXT("end_exclusive"),
            OutSnapshot.bHasStableEnd,
            StableEnd,
            OutError))
    {
        if (OutError.IsEmpty())
        {
            OutError = TEXT("Snapshot omitted stable_time_segment");
        }
        return false;
    }
    if ((OutSnapshot.bHasStableStart && CompareInstants(StableStart, EvaluatedAt) > 0)
        || (OutSnapshot.bHasStableEnd && CompareInstants(EvaluatedAt, StableEnd) >= 0)
        || (OutSnapshot.bHasStableStart && OutSnapshot.bHasStableEnd
            && CompareInstants(StableStart, StableEnd) >= 0))
    {
        OutError = TEXT("Snapshot evaluated_at was outside its stable time segment");
        return false;
    }
    if (OutSnapshot.bHasStableStart)
    {
        OutSnapshot.StableStartSeconds = StableStart.SecondsText;
        OutSnapshot.StableStartNanoseconds = StableStart.Nanoseconds;
    }
    if (OutSnapshot.bHasStableEnd)
    {
        OutSnapshot.StableEndSeconds = StableEnd.SecondsText;
        OutSnapshot.StableEndNanoseconds = StableEnd.Nanoseconds;
    }

    FString RequestedDetail;
    FString EffectiveDetail;
    if (!Root->TryGetStringField(TEXT("requested_detail"), RequestedDetail)
        || !DetailFromProtocol(RequestedDetail, OutSnapshot.RequestedDetail)
        || !Root->TryGetStringField(TEXT("effective_detail"), EffectiveDetail)
        || !DetailFromProtocol(EffectiveDetail, OutSnapshot.EffectiveDetail))
    {
        OutError = TEXT("Snapshot contained an unknown detail level");
        return false;
    }
    if (static_cast<uint8>(OutSnapshot.EffectiveDetail)
        > static_cast<uint8>(OutSnapshot.RequestedDetail))
    {
        OutError = TEXT("Snapshot effective detail exceeded the requested detail");
        return false;
    }

    const TArray<TSharedPtr<FJsonValue>>* DegradationReasons = nullptr;
    if (!Root->TryGetArrayField(TEXT("degradation_reasons"), DegradationReasons))
    {
        OutError = TEXT("Snapshot omitted degradation_reasons");
        return false;
    }
    OutSnapshot.DegradationReasons.Reserve(DegradationReasons->Num());
    for (const TSharedPtr<FJsonValue>& Value : *DegradationReasons)
    {
        FString Reason;
        if (!Value.IsValid() || !Value->TryGetString(Reason) || !IsKnownDegradationReason(Reason))
        {
            OutError = TEXT("Snapshot contained an unknown degradation reason");
            return false;
        }
        OutSnapshot.DegradationReasons.Add(MoveTemp(Reason));
    }

    int64 IntegerValue = 0;
    if (!ReadRequiredInteger(Root, TEXT("layers"), 0.0, 255.0, IntegerValue, OutError))
    {
        return false;
    }
    OutSnapshot.LayerMask = static_cast<int32>(IntegerValue);

    const TSharedPtr<FJsonObject>* Metrics = nullptr;
    if (!Root->TryGetObjectField(TEXT("metrics"), Metrics)
        || !ReadRequiredInteger(*Metrics, TEXT("candidate_patches"), 0.0, MaxCandidatePatches, IntegerValue, OutError))
    {
        if (OutError.IsEmpty())
        {
            OutError = TEXT("Snapshot omitted metrics");
        }
        return false;
    }
    OutSnapshot.CandidatePatches = static_cast<int32>(IntegerValue);
    if (!ReadRequiredInteger(*Metrics, TEXT("applied_patches"), 0.0, MaxCandidatePatches, IntegerValue, OutError))
    {
        return false;
    }
    OutSnapshot.AppliedPatches = static_cast<int32>(IntegerValue);
    if (!ReadRequiredInteger(*Metrics, TEXT("work_units"), 0.0, MaxExactJsonInteger, IntegerValue, OutError))
    {
        return false;
    }
    OutSnapshot.WorkUnits = IntegerValue;
    if (!ReadRequiredInteger(
        *Metrics,
        TEXT("conservative_output_bound_bytes"),
        0.0,
        MaxExactJsonInteger,
        IntegerValue,
        OutError))
    {
        return false;
    }
    OutSnapshot.ConservativeOutputBoundBytes = IntegerValue;
    if (!ReadRequiredInteger(
        *Metrics,
        TEXT("suggested_instance_ceiling"),
        0.0,
        MaxGeneratedInstances,
        IntegerValue,
        OutError))
    {
        return false;
    }
    OutSnapshot.SuggestedInstanceCeiling = static_cast<int32>(IntegerValue);
    if (OutSnapshot.AppliedPatches > OutSnapshot.CandidatePatches)
    {
        OutError = TEXT("Snapshot metrics reported more applied than candidate patches");
        return false;
    }

    const TSharedPtr<FJsonObject>* Surface = nullptr;
    if (!Root->TryGetObjectField(TEXT("surface"), Surface))
    {
        OutError = TEXT("Snapshot omitted surface");
        return false;
    }
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("mean_elevation_mm"),
            -MaxExactJsonInteger,
            MaxExactJsonInteger,
            OutSnapshot.Surface.bHasMeanElevation,
            OutSnapshot.Surface.MeanElevationMillimeters,
            OutError)
        || !ReadOptionalInteger(
            *Surface,
            TEXT("relief_mm"),
            0.0,
            MaxExactJsonInteger,
            OutSnapshot.Surface.bHasRelief,
            OutSnapshot.Surface.ReliefMillimeters,
            OutError))
    {
        return false;
    }

    int64 TemporaryValue = 0;
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("land_fraction_ppm"),
            0.0,
            1'000'000.0,
            OutSnapshot.Surface.bHasLandFraction,
            TemporaryValue,
            OutError))
    {
        return false;
    }
    OutSnapshot.Surface.LandFractionPartsPerMillion = static_cast<int32>(TemporaryValue);
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("water_fraction_ppm"),
            0.0,
            1'000'000.0,
            OutSnapshot.Surface.bHasWaterFraction,
            TemporaryValue,
            OutError))
    {
        return false;
    }
    OutSnapshot.Surface.WaterFractionPartsPerMillion = static_cast<int32>(TemporaryValue);
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("vegetation_fraction_ppm"),
            0.0,
            1'000'000.0,
            OutSnapshot.Surface.bHasVegetationFraction,
            TemporaryValue,
            OutError))
    {
        return false;
    }
    OutSnapshot.Surface.VegetationFractionPartsPerMillion = static_cast<int32>(TemporaryValue);
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("mean_temperature_millicelsius"),
            -2'147'483'648.0,
            2'147'483'647.0,
            OutSnapshot.Surface.bHasMeanTemperature,
            TemporaryValue,
            OutError))
    {
        return false;
    }
    OutSnapshot.Surface.MeanTemperatureMillicelsius = static_cast<int32>(TemporaryValue);
    if (!ReadOptionalInteger(
            *Surface,
            TEXT("annual_precipitation_mm"),
            0.0,
            4'294'967'295.0,
            OutSnapshot.Surface.bHasAnnualPrecipitation,
            TemporaryValue,
            OutError))
    {
        return false;
    }
    OutSnapshot.Surface.AnnualPrecipitationMillimeters = TemporaryValue;

    const TArray<TSharedPtr<FJsonValue>>* Provenance = nullptr;
    if (!Root->TryGetArrayField(TEXT("provenance"), Provenance)
        || Provenance->Num() != OutSnapshot.AppliedPatches)
    {
        OutError = TEXT("Snapshot provenance did not match applied_patches");
        return false;
    }
    OutSnapshot.Provenance.Reserve(Provenance->Num());
    for (const TSharedPtr<FJsonValue>& Value : *Provenance)
    {
        const TSharedPtr<FJsonObject> Patch = Value.IsValid() ? Value->AsObject() : nullptr;
        if (!Patch.IsValid())
        {
            OutError = TEXT("Snapshot provenance entry was not an object");
            return false;
        }
        FHistoryWorldAppliedPatch Decoded;
        if (!Patch->TryGetStringField(TEXT("patch_id"), Decoded.PatchId)
            || !IsLowerHex(Decoded.PatchId, 32)
            || !Patch->TryGetStringField(TEXT("epistemic_class"), Decoded.EpistemicClass)
            || !IsKnownEpistemicClass(Decoded.EpistemicClass))
        {
            OutError = TEXT("Snapshot provenance entry had an invalid identity or epistemic class");
            return false;
        }
        OutSnapshot.Provenance.Add(MoveTemp(Decoded));
    }
    return true;
}
