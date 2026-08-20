#pragma once

#include "CoreMinimal.h"
#include "UObject/Interface.h"

#include "HistoryWorldTypes.h"
#include "HistoryWorldPresentationAdapter.generated.h"

UINTERFACE(BlueprintType, Blueprintable)
class HISTORYWORLDRUNTIME_API UHistoryWorldPresentationAdapter : public UInterface
{
    GENERATED_BODY()
};

/**
 * Implement this in a project-owned adapter. Cesium, World Partition, PCG,
 * Mass, Nanite, Lumen, and future systems remain behind this semantic seam.
 */
class HISTORYWORLDRUNTIME_API IHistoryWorldPresentationAdapter
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintNativeEvent, BlueprintCallable, Category = "HistoryWorld")
    void ApplySnapshot(int64 Generation, const FHistoryWorldSnapshot& Snapshot);

    UFUNCTION(BlueprintNativeEvent, BlueprintCallable, Category = "HistoryWorld")
    void CancelGeneration(int64 Generation);
};
