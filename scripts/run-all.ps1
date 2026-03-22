Set-Location $PSScriptRoot
$repoRoot = Resolve-Path ..
$targetDir = Join-Path $repoRoot "target\run-all"
$serverExe = Join-Path $targetDir "debug\server.exe"
$clientExe = Join-Path $targetDir "debug\chatmessagediscordclone.exe"
$runner = Join-Path $PSScriptRoot "run-exe.ps1"
$envFile = Join-Path $repoRoot ".env"

if (Test-Path $envFile) {
	Get-Content $envFile | ForEach-Object {
		if ($_ -match "^\s*#") { return }
		if ($_ -match "^\s*$") { return }
		$pair = $_ -split "=", 2
		if ($pair.Length -eq 2) {
			$name = $pair[0].Trim()
			$value = $pair[1].Trim()
			if ($name -and $value) {
				[System.Environment]::SetEnvironmentVariable($name, $value, "Process")
			}
		}
	}
}

if (-not $env:DATABASE_URL) {
	Write-Host "DATABASE_URL must be set."
	Write-Host "Example: postgres://postgres:[YOUR-PASSWORD]@wcllqcbmnnxkllkmdkid.db.us-west-2.nhost.run:5432/wcllqcbmnnxkllkmdkid"
	exit 1
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
