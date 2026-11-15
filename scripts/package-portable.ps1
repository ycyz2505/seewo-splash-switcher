param(
  [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$target = Join-Path $root 'src-tauri\target\release'
$out = Join-Path $root 'dist'
$name = "SeewoHtmlSplash-v$Version"
$portableRoot = Join-Path $out 'portable'
$appRoot = Join-Path $portableRoot $name

if (Test-Path $portableRoot) { Remove-Item $portableRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path $appRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'app') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'tools') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'data') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'logs') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'backup') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'updates') | Out-Null

$files = @(
  @{ Source = (Join-Path $target 'seewo-splash-switcher.exe'); Dest = (Join-Path $appRoot 'app\seewo-splash-switcher.exe') },
  @{ Source = (Join-Path $target 'seewo-apply.exe'); Dest = (Join-Path $appRoot 'app\seewo-apply.exe') },
  @{ Source = (Join-Path $target 'seewo-updater.exe'); Dest = (Join-Path $appRoot 'tools\seewo-updater.exe') },
  @{ Source = (Join-Path $target 'seewo-launcher-shim.exe'); Dest = (Join-Path $appRoot 'tools\seewo-launcher-shim.exe') }
)
foreach ($file in $files) {
  if (-not (Test-Path $file.Source)) { throw "Missing build output: $($file.Source)" }
  Copy-Item -Path $file.Source -Destination $file.Dest -Force
}

Copy-Item -Path (Join-Path $root 'default-data\*') -Destination (Join-Path $appRoot 'data') -Recurse -Force
Copy-Item -Path (Join-Path $root 'README.md') -Destination (Join-Path $appRoot 'README.txt') -Force

$bat = @'
@echo off
start "" "%~dp0app\seewo-splash-switcher.exe"
'@
[System.IO.File]::WriteAllText(
  (Join-Path $appRoot '启动希沃启动页切换器.bat'),
  $bat,
  (New-Object System.Text.UTF8Encoding($false))
)

$zip = Join-Path $out "$name-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $portableRoot $name) -DestinationPath $zip -CompressionLevel Optimal

$hash = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLowerInvariant()
$size = (Get-Item $zip).Length
$info = [ordered]@{
  version = $Version
  zip     = (Split-Path $zip -Leaf)
  size    = $size
  sha256  = $hash
}
$json = $info | ConvertTo-Json -Depth 4
[System.IO.File]::WriteAllText(
  (Join-Path $out 'package-info.json'),
  $json,
  (New-Object System.Text.UTF8Encoding($false))
)
[System.IO.File]::WriteAllText(
  (Join-Path $out 'sha256sums.txt'),
  "$hash  $name-portable.zip`n",
  (New-Object System.Text.UTF8Encoding($false))
)

Write-Host "Portable package: $zip"
Write-Host "SHA256: $hash"
