param(
  [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

function Write-Utf8NoBom([string]$Path, [string]$Text) {
  [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding($false)))
}

$cargo = Join-Path $root 'src-tauri\Cargo.toml'
$text = Get-Content -Path $cargo -Raw -Encoding UTF8
$text = [regex]::Replace($text, '(?m)^version\s*=\s*"[^"]+"', "version = `"$Version`"", 1)
Write-Utf8NoBom $cargo $text

$tauri = Join-Path $root 'src-tauri\tauri.conf.json'
$text = Get-Content -Path $tauri -Raw -Encoding UTF8
$text = [regex]::Replace($text, '"version"\s*:\s*"[^"]+"', "`"version`": `"$Version`"", 1)
Write-Utf8NoBom $tauri $text

$package = Join-Path $root 'package.json'
$text = Get-Content -Path $package -Raw -Encoding UTF8
$text = [regex]::Replace($text, '"version"\s*:\s*"[^"]+"', "`"version`": `"$Version`"", 1)
Write-Utf8NoBom $package $text

Write-Host "Version set to $Version"
