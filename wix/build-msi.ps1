# Usage: .\wix\build-msi.ps1 -Target <rustpos|printclient>
# Requires: WiX Toolset v3 (candle.exe, light.exe, heat.exe) in PATH
param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("rustpos", "printclient")]
    [string]$Target
)

$ErrorActionPreference = "Stop"

$Version = (Select-String -Path "frontend\Cargo.toml" -Pattern '^version = "(.*)"' | ForEach-Object { $_.Matches[0].Groups[1].Value })
Write-Host "Building MSI for $Target version $Version" -ForegroundColor Cyan

$BinDir = "target\release"
$SrcDir = "."
$WixDir = "wix"
$OutDir = "."

if ($Target -eq "rustpos") {
    # Harvest the site/ directory into a WiX fragment
    Write-Host "Harvesting site directory..." -ForegroundColor Yellow
    heat.exe dir "site" `
        -cg SiteFiles `
        -dr INSTALLFOLDER `
        -ag -sfrag -srd -sreg `
        -var "var.SiteDir" `
        -o "$WixDir\site.wxs"
    if ($LASTEXITCODE -ne 0) { throw "heat.exe failed" }

    # Compile WiX sources
    Write-Host "Compiling WiX sources..." -ForegroundColor Yellow
    candle.exe `
        -dBinDir="$BinDir" `
        -dAssetsDir="frontend\assets" `
        -dSiteDir="site" `
        -arch x64 `
        -o "$WixDir\rustpos.wixobj" `
        "$WixDir\rustpos.wxs"
    if ($LASTEXITCODE -ne 0) { throw "candle.exe failed for rustpos.wxs" }

    candle.exe `
        -dSiteDir="site" `
        -arch x64 `
        -o "$WixDir\site.wixobj" `
        "$WixDir\site.wxs"
    if ($LASTEXITCODE -ne 0) { throw "candle.exe failed for site.wxs" }

    # Link into MSI
    Write-Host "Linking MSI..." -ForegroundColor Yellow
    light.exe `
        -ext WixUIExtension `
        -spdb `
        -o "$OutDir\rustpos-${Version}-win64.msi" `
        "$WixDir\rustpos.wixobj" `
        "$WixDir\site.wixobj"
    if ($LASTEXITCODE -ne 0) { throw "light.exe failed" }

    Write-Host "Built: rustpos-${Version}-win64.msi" -ForegroundColor Green

} elseif ($Target -eq "printclient") {
    # Compile WiX source
    Write-Host "Compiling WiX sources..." -ForegroundColor Yellow
    candle.exe `
        -dBinDir="$BinDir" `
        -dSrcDir="$SrcDir" `
        -arch x64 `
        -o "$WixDir\printclient.wixobj" `
        "$WixDir\printclient.wxs"
    if ($LASTEXITCODE -ne 0) { throw "candle.exe failed" }

    # Link into MSI
    Write-Host "Linking MSI..." -ForegroundColor Yellow
    light.exe `
        -ext WixUIExtension `
        -spdb `
        -o "$OutDir\rustpos-printclient-${Version}-win64.msi" `
        "$WixDir\printclient.wixobj"
    if ($LASTEXITCODE -ne 0) { throw "light.exe failed" }

    Write-Host "Built: rustpos-printclient-${Version}-win64.msi" -ForegroundColor Green
}
