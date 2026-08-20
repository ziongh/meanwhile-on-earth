#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"

#include "HistoryWorldTypes.h"
#include "HistoryWorldSubsystem.generated.h"

class IHttpRequest;
class IHttpResponse;

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
    FHistoryWorldSnapshotReady,
    int64,
    Generation,
    const FHistoryWorldSnapshot&,
    Snapshot);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(
    FHistoryWorldSnapshotFailed,
    int64,
    Generation,
    const FString&,
    Message);

/** Fetches immutable semantic snapshots and cancels stale timeline requests. */
UCLASS()
class HISTORYWORLDRUNTIME_API UHistoryWorldSubsystem final : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    virtual void Deinitialize() override;

    UFUNCTION(BlueprintCallable, Category = "HistoryWorld")
    int64 RequestSnapshot(
        const FString& ReleaseId,
        const FHistoryWorldCell& Cell,
        int64 AtSeconds,
        EHistoryWorldDetail Detail);

    UFUNCTION(BlueprintCallable, Category = "HistoryWorld")
    void CancelActiveRequest();

    UPROPERTY(BlueprintAssignable, Category = "HistoryWorld")
    FHistoryWorldSnapshotReady OnSnapshotReady;

    UPROPERTY(BlueprintAssignable, Category = "HistoryWorld")
    FHistoryWorldSnapshotFailed OnSnapshotFailed;

private:
    void HandleResponse(
        TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> Request,
        TSharedPtr<IHttpResponse, ESPMode::ThreadSafe> Response,
        bool bConnectedSuccessfully,
        int64 RequestGeneration,
        FString ExpectedReleaseId,
        FString ExpectedCell,
        FString ExpectedAtSeconds,
        EHistoryWorldDetail ExpectedDetail);

    static bool DecodeSnapshot(const FString& Json, FHistoryWorldSnapshot& OutSnapshot, FString& OutError);
    void AdvanceGeneration();

    int64 Generation = 0;
    TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> ActiveRequest;
};
