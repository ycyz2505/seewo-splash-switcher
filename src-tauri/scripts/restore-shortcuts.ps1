param(
  [Parameter(Mandatory = $true)][string]$BackupDir
)

$ErrorActionPreference = 'Continue'
$errors = New-Object System.Collections.ArrayList
$restored = 0

function Write-JsonFile([string]$Path, $Object) {
  $json = $Object | ConvertTo-Json -Depth 8
  $dir = Split-Path $Path -Parent
  if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  [System.IO.File]::WriteAllText($Path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

function Resolve-IconLocation([string]$Icon, [string]$Target) {
  $fallback = if ([string]::IsNullOrWhiteSpace($Target)) { '' } else { $Target + ',0' }
  if ([string]::IsNullOrWhiteSpace($Icon)) { return $fallback }
  $iconText = $Icon.Trim()
  if ($iconText -match '^,\d+$') { return $fallback }
  $pathPart = ($iconText -split ',')[0]
  if ([string]::IsNullOrWhiteSpace($pathPart)) { return $fallback }
  return $iconText
}

try {
  $backupRoot = Join-Path $BackupDir 'shortcuts'
  if (Test-Path $backupRoot) {
    $shell = New-Object -ComObject WScript.Shell
    foreach ($file in Get-ChildItem -Path $backupRoot -Filter *.json -ErrorAction SilentlyContinue) {
      try {
        $record = Get-Content -Path $file.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
        if ([string]::IsNullOrWhiteSpace([string]$record.link)) { continue }
        if (-not (Test-Path ([string]$record.link))) { continue }
        $shortcut = $shell.CreateShortcut([string]$record.link)
        $shortcut.TargetPath = [string]$record.target
        $shortcut.Arguments = [string]$record.arguments
        $shortcut.WorkingDirectory = [string]$record.workingDirectory
        $icon = Resolve-IconLocation ([string]$record.iconLocation) ([string]$record.target)
        if (-not [string]::IsNullOrWhiteSpace($icon)) {
          $shortcut.IconLocation = $icon
        }
        if (-not [string]::IsNullOrWhiteSpace([string]$record.description)) {
          $shortcut.Description = [string]$record.description
        }
        $shortcut.Save()
        $restored++
      } catch {
        [void]$errors.Add(($file.FullName + ': ' + $_.Exception.Message))
      }
    }
  }
} catch {
  [void]$errors.Add($_.Exception.Message)
}

$result = [ordered]@{ restored = $restored; errors = @($errors) }
Write-JsonFile (Join-Path $BackupDir 'restore-shortcuts-result.json') $result
