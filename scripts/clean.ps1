Set-Location $PSScriptRoot
$repoRoot = Resolve-Path ..
$targetDir = Join-Path $repoRoot "target\run-all"
Set-Location $repoRoot
$env:CARGO_TARGET_DIR = $targetDir
cargo clean
