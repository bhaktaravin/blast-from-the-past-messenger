param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,
    [Parameter(Mandatory = $true)]
    [string]$ExePath
)

Set-Location $RepoRoot
& $ExePath
