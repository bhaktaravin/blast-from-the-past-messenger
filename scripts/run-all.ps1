Set-Location $PSScriptRoot
$repoRoot = Resolve-Path ..
$targetDir = Join-Path $repoRoot "target\run-all"
$serverExe = Join-Path $targetDir "debug\server.exe"
$clientExe = Join-Path $targetDir "debug\chatmessagediscordclone.exe"
$runner = Join-Path $PSScriptRoot "run-exe.ps1"

if (-not $env:DATABASE_URL) {
	if ($env:SUPABASE_DB_URL) {
		$env:DATABASE_URL = $env:SUPABASE_DB_URL
	} elseif ($env:SUPABASE_URL) {
		$env:DATABASE_URL = $env:SUPABASE_URL
	} else {
		Write-Host "DATABASE_URL, SUPABASE_DB_URL, or SUPABASE_URL must be set."
		Write-Host "Example: postgres://user:pass@localhost:5432/retrochat"
		exit 1
	}
}

$wt = Get-Command wt -ErrorAction SilentlyContinue
if (-not $wt) {
	Write-Host "Windows Terminal (wt) not found. Falling back to PowerShell windows."
	Set-Location $repoRoot
	$env:CARGO_TARGET_DIR = $targetDir
	$env:CARGO_BUILD_JOBS = "1"
	$env:CARGO_INCREMENTAL = "0"
	cargo build --bins -j 1
	if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
	if (-not (Test-Path $serverExe) -or -not (Test-Path $clientExe)) {
		Write-Host "Build completed but executables were not found in $targetDir\debug"
		exit 1
	}
	Start-Process -FilePath $serverExe -WorkingDirectory $repoRoot
	Start-Process -FilePath $clientExe -WorkingDirectory $repoRoot
	exit 0
}

Set-Location $repoRoot
$env:CARGO_TARGET_DIR = $targetDir
$env:CARGO_BUILD_JOBS = "1"
$env:CARGO_INCREMENTAL = "0"
cargo build --bins -j 1
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (-not (Test-Path $serverExe) -or -not (Test-Path $clientExe)) {
	Write-Host "Build completed but executables were not found in $targetDir\debug"
	exit 1
}

$wtArgs = @(
	"new-tab", "-d", "$repoRoot", "--title", "AOL Server", "powershell",
	"-NoExit", "-File", "$runner", "-RepoRoot", "$repoRoot", "-ExePath", "$serverExe",
	";",
	"new-tab", "-d", "$repoRoot", "--title", "AOL Client", "powershell",
	"-NoExit", "-File", "$runner", "-RepoRoot", "$repoRoot", "-ExePath", "$clientExe"
)

if ($env:WT_SESSION) {
	& wt -w 0 @wtArgs
} else {
	& wt @wtArgs
}
