Set-Location $PSScriptRoot
$repoRoot = Resolve-Path ..
$targetDir = Join-Path $repoRoot "target\run-all"
Set-Location $repoRoot
$env:CARGO_TARGET_DIR = $targetDir

$stale = @("cargo", "rustc", "build-script-build")
foreach ($name in $stale) {
	Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}

cargo clean
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
cargo build -j 1
