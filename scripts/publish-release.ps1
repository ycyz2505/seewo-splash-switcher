param(
  [Parameter(Mandatory = $true)][string]$Tag,
  [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$out = Join-Path $root 'dist'

$assets = @()
foreach ($pattern in @('*.zip', '*.sig', 'latest.json', 'mirrors.json', 'sha256sums.txt')) {
  $assets += Get-ChildItem -Path (Join-Path $out $pattern) -File -ErrorAction SilentlyContinue
}
$assets = $assets | Sort-Object FullName -Unique

if (-not $assets) { throw 'No release assets found in dist/' }

gh release view $Tag 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
  gh release create $Tag --draft --title "v$Version" --notes "See GitHub release notes."
}

$paths = $assets | ForEach-Object { $_.FullName }
gh release upload $Tag $paths --clobber
if ($LASTEXITCODE -ne 0) { throw 'gh release upload failed' }

gh release edit $Tag --draft=false --latest
if ($LASTEXITCODE -ne 0) { throw 'gh release edit failed' }

Write-Host "Published $Tag"
