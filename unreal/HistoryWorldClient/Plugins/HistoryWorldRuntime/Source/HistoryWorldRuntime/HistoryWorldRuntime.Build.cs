using UnrealBuildTool;

public class HistoryWorldRuntime : ModuleRules
{
    public HistoryWorldRuntime(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicDependencyModuleNames.AddRange(new[]
        {
            "Core",
            "CoreUObject",
            "DeveloperSettings",
            "Engine"
        });
        PrivateDependencyModuleNames.AddRange(new[]
        {
            "HTTP",
            "Json"
        });
    }
}
