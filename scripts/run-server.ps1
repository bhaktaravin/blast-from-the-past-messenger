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
cargo run --bin server
