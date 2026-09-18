param(
    [string]$Version = "0.1.0-alpha"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root

Write-Host "Building MONORYX release binary..."
& cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }

$exe = Join-Path $root "target\release\monoryx.exe"
if (-not (Test-Path -LiteralPath $exe)) { throw "Release exe not found: $exe" }

$candidates = @()
$cmd = Get-Command iscc -ErrorAction SilentlyContinue
if ($cmd) { $candidates += $cmd.Source }
$candidates += "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
$candidates += "C:\Program Files\Inno Setup 6\ISCC.exe"
$iscc = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $iscc) {
    throw "Inno Setup 6 (ISCC.exe) not found. Install it from https://jrsoftware.org/isdl.php and re-run this script."
}

Write-Host "Compiling installer with $iscc ..."
& "$iscc" "/DAppVersion=$Version" (Join-Path $PSScriptRoot "monoryx.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }

$setup = Join-Path $root "installer\dist\MONORYX-Setup-$Version.exe"
if (Test-Path -LiteralPath $setup) {
    Write-Host "Installer ready: $setup"
} else {
    Write-Host "ISCC finished; see installer\dist for output."
}
