#pragma once

#include "CoreMinimal.h"
#include "Engine/DeveloperSettings.h"

#include "HistoryWorldResourceSettings.generated.h"

/** One governor shared by HTTP, semantic caches, PCG, Mass, and renderer adapters. */
UCLASS(Config = Game, DefaultConfig, meta = (DisplayName = "HistoryWorld Resource Budgets"))
class HISTORYWORLDRUNTIME_API UHistoryWorldResourceSettings final : public UDeveloperSettings
{
    GENERATED_BODY()

public:
    UPROPERTY(Config, EditAnywhere, Category = "Network")
    FString ServiceBaseUrl = TEXT("http://127.0.0.1:8080");

    UPROPERTY(Config, EditAnywhere, Category = "Network", meta = (ClampMin = "4096", ClampMax = "33554432"))
    int32 MaxHttpResponseBytes = 2 * 1024 * 1024;

    /** Hard ceiling on semantic candidates the service may inspect for one request. */
    UPROPERTY(Config, EditAnywhere, Category = "Network", meta = (ClampMin = "1", ClampMax = "65536"))
    int32 MaxCandidatePatches = 4096;

    /** Deterministic server-side work ceiling; this is not a wall-clock timeout. */
    UPROPERTY(Config, EditAnywhere, Category = "Network", meta = (ClampMin = "1", ClampMax = "1000000"))
    int64 MaxWorkUnits = 100000;

    UPROPERTY(Config, EditAnywhere, Category = "Network", meta = (ClampMin = "1.0", ClampMax = "300.0"))
    float RequestTimeoutSeconds = 30.0f;

    UPROPERTY(Config, EditAnywhere, Category = "Memory", meta = (ClampMin = "1"))
    int32 MaxResidentSemanticMegabytes = 64;

    UPROPERTY(Config, EditAnywhere, Category = "Realization", meta = (ClampMin = "1"))
    int32 MaxGeneratedInstances = 20000;

    UPROPERTY(Config, EditAnywhere, Category = "Realization", meta = (ClampMin = "0.1"))
    float ProceduralFrameBudgetMilliseconds = 2.0f;

    UPROPERTY(Config, EditAnywhere, Category = "Realization", meta = (ClampMin = "0"))
    int32 MaxMassEntities = 20000;

    UPROPERTY(Config, EditAnywhere, Category = "Streaming", meta = (ClampMin = "1"))
    int32 MaxConcurrentRegionRequests = 2;
};
