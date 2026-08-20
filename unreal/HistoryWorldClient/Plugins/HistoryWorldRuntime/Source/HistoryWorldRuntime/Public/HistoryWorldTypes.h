#pragma once

#include "CoreMinimal.h"

#include "HistoryWorldTypes.generated.h"

UENUM(BlueprintType)
enum class EHistoryWorldDetail : uint8
{
    Planet,
    Continental,
    Regional,
    Local,
    Street
};

USTRUCT(BlueprintType)
struct HISTORYWORLDRUNTIME_API FHistoryWorldCell
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "HistoryWorld")
    int32 Face = 0;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "HistoryWorld", meta = (ClampMin = "0", ClampMax = "28"))
    int32 Level = 0;

    // Hex avoids Blueprint/JSON precision loss for the unsigned 56-bit path.
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "HistoryWorld")
    FString PathHex = TEXT("0");

    bool IsProtocolValid(FString& OutError) const;
    FString ToProtocolString() const;
};

USTRUCT(BlueprintType)
struct HISTORYWORLDRUNTIME_API FHistoryWorldSurfaceState
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasMeanElevation = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int64 MeanElevationMillimeters = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasRelief = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int64 ReliefMillimeters = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasLandFraction = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 LandFractionPartsPerMillion = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasWaterFraction = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 WaterFractionPartsPerMillion = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasVegetationFraction = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 VegetationFractionPartsPerMillion = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasMeanTemperature = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 MeanTemperatureMillicelsius = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasAnnualPrecipitation = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int64 AnnualPrecipitationMillimeters = 0;
};

USTRUCT(BlueprintType)
struct HISTORYWORLDRUNTIME_API FHistoryWorldAppliedPatch
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString PatchId;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString EpistemicClass;
};

USTRUCT(BlueprintType)
struct HISTORYWORLDRUNTIME_API FHistoryWorldSnapshot
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString Contract;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString MaterializerVersion;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString NumericModelVersion;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString PrngVersion;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString ReleaseId;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString LocalDependencyHash;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString SemanticStateHash;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString RealizationSeed;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString EvaluatedAtSeconds;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 EvaluatedAtNanoseconds = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString RealizationEpochSeconds;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 RealizationEpochNanoseconds = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString Cell;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasStableStart = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString StableStartSeconds;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 StableStartNanoseconds = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    bool bHasStableEnd = false;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FString StableEndSeconds;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 StableEndNanoseconds = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    EHistoryWorldDetail RequestedDetail = EHistoryWorldDetail::Planet;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    EHistoryWorldDetail EffectiveDetail = EHistoryWorldDetail::Planet;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    TArray<FString> DegradationReasons;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 LayerMask = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 CandidatePatches = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 AppliedPatches = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int64 WorkUnits = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int64 ConservativeOutputBoundBytes = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    int32 SuggestedInstanceCeiling = 0;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    FHistoryWorldSurfaceState Surface;

    UPROPERTY(BlueprintReadOnly, Category = "HistoryWorld")
    TArray<FHistoryWorldAppliedPatch> Provenance;
};
