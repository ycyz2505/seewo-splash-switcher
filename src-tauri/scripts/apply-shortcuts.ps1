param(
  [Parameter(Mandatory = $true)][string]$Root,
  [Parameter(Mandatory = $true)][string]$AppExe,
  [Parameter(Mandatory = $true)][string]$BackupDir
)

$ErrorActionPreference = 'Continue'
$errors = New-Object System.Collections.ArrayList
$rewritten = 0

function Write-JsonFile([string]$Path, $Object) {
  $json = $Object | ConvertTo-Json -Depth 8
  $dir = Split-Path $Path -Parent
  if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  [System.IO.File]::WriteAllText($Path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

function Get-ShortcutKey([string]$Path) {
  $sha1 = [System.Security.Cryptography.SHA1]::Create()
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($Path.ToLowerInvariant())
  return ([System.BitConverter]::ToString($sha1.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
}

# Windows 以 ",0" 表示“未显式指定图标”。这种值写回快捷方式会变成空白图标，
# 必须回退到原程序的 exe（取第 0 个图标）。
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
  if (-not (Test-Path $backupRoot)) { New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null }
  $shell = New-Object -ComObject WScript.Shell

  $searchRoots = @(
    (Join-Path $env:PUBLIC 'Desktop'),
    (Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs')
  )

  $links = @()
  foreach ($searchRoot in $searchRoots) {
    if (Test-Path $searchRoot) {
      $links += Get-ChildItem -Path $searchRoot -Filter *.lnk -Recurse -ErrorAction SilentlyContinue
    }
  }

  foreach ($link in $links) {
    try {
      $shortcut = $shell.CreateShortcut($link.FullName)
      $target = [string]$shortcut.TargetPath
      if ([string]::IsNullOrWhiteSpace($target)) { continue }
      if ($target -notmatch 'swenlauncher\.exe$' -and
          $target -notmatch 'EasiNote\.exe$' -and
          $target -notmatch 'seewo-splash-switcher\.exe$') {
        continue
      }

      $backupFile = Join-Path $backupRoot ((Get-ShortcutKey $link.FullName) + '.json')
      $originalTarget = $target
      $originalIcon = [string]$shortcut.IconLocation
      if (Test-Path $backupFile) {
        try {
          $record = Get-Content -Raw $backupFile -Encoding UTF8 | ConvertFrom-Json
          if (-not [string]::IsNullOrWhiteSpace([string]$record.target)) {
            $originalTarget = [string]$record.target
          }
          if (-not [string]::IsNullOrWhiteSpace([string]$record.iconLocation)) {
            $originalIcon = [string]$record.iconLocation
          }
        } catch {
          [void]$errors.Add(($link.FullName + ': read backup failed: ' + $_.Exception.Message))
        }
      } else {
        $record = [ordered]@{
          link             = $link.FullName
          target           = $target
          arguments        = [string]$shortcut.Arguments
          workingDirectory = [string]$shortcut.WorkingDirectory
          iconLocation     = (Resolve-IconLocation ([string]$shortcut.IconLocation) $target)
          description      = [string]$shortcut.Description
        }
        Write-JsonFile $backupFile $record
      }

      $originalIcon = Resolve-IconLocation $originalIcon $originalTarget

      $shortcut.TargetPath = $AppExe
      if ([string]::IsNullOrWhiteSpace([string]$shortcut.Arguments)) {
        $shortcut.Arguments = '--splash'
      } elseif ([string]$shortcut.Arguments -notmatch '^\s*--splash(\s|$)') {
        $shortcut.Arguments = '--splash ' + [string]$shortcut.Arguments
      }
      $shortcut.WorkingDirectory = Split-Path $AppExe -Parent
      $shortcut.IconLocation = $originalIcon
      $shortcut.Save()
      $rewritten++
    } catch {
      [void]$errors.Add(($link.FullName + ': ' + $_.Exception.Message))
    }
  }
} catch {
  [void]$errors.Add($_.Exception.Message)
}

$result = [ordered]@{ rewritten = $rewritten; errors = @($errors) }
Write-JsonFile (Join-Path $BackupDir 'apply-shortcuts-result.json') $result
