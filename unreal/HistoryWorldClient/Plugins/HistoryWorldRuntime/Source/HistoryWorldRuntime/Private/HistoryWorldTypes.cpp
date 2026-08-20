#include "HistoryWorldTypes.h"

namespace
{
int32 HexDigitValue(const TCHAR Character)
{
    if (Character >= TEXT('0') && Character <= TEXT('9'))
    {
        return Character - TEXT('0');
    }
    if (Character >= TEXT('a') && Character <= TEXT('f'))
    {
        return Character - TEXT('a') + 10;
    }
    if (Character >= TEXT('A') && Character <= TEXT('F'))
    {
        return Character - TEXT('A') + 10;
    }
    return -1;
}
}

bool FHistoryWorldCell::IsProtocolValid(FString& OutError) const
{
    if (Face < 0 || Face > 5)
    {
        OutError = TEXT("Cell face must be between 0 and 5");
        return false;
    }
    if (Level < 0 || Level > 28)
    {
        OutError = TEXT("Cell level must be between 0 and 28");
        return false;
    }
    if (PathHex.IsEmpty() || PathHex.Len() > 14)
    {
        OutError = TEXT("Cell path must contain 1 to 14 hexadecimal digits");
        return false;
    }

    uint64 Path = 0;
    for (const TCHAR Character : PathHex)
    {
        const int32 Digit = HexDigitValue(Character);
        if (Digit < 0)
        {
            OutError = TEXT("Cell path contained a non-hexadecimal character");
            return false;
        }
        Path = (Path << 4) | static_cast<uint64>(Digit);
    }

    const uint32 PathBits = static_cast<uint32>(Level) * 2;
    const bool bPathFits = PathBits == 0 ? Path == 0 : Path < (uint64{1} << PathBits);
    if (!bPathFits)
    {
        OutError = TEXT("Cell path does not fit the requested level");
        return false;
    }
    return true;
}

FString FHistoryWorldCell::ToProtocolString() const
{
    FString CanonicalPath = PathHex.ToLower();
    int32 FirstSignificantDigit = 0;
    while (FirstSignificantDigit < CanonicalPath.Len() - 1
        && CanonicalPath[FirstSignificantDigit] == TEXT('0'))
    {
        ++FirstSignificantDigit;
    }
    if (FirstSignificantDigit > 0)
    {
        CanonicalPath = CanonicalPath.RightChop(FirstSignificantDigit);
    }
    return FString::Printf(TEXT("%d:%d:%s"), Face, Level, *CanonicalPath);
}
