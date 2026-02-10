Set-Location $PSScriptRoot
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
