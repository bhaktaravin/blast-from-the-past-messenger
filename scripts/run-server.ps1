Set-Location $PSScriptRoot
$repoRoot = Resolve-Path ..
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
cargo run --bin server
