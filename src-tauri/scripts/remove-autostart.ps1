param(
  [string]$TaskName = 'SeewoHtmlSplash\Boot',
  [string]$BackupDir = ''
)

$ErrorActionPreference = 'Continue'
$errors = New-Object System.Collections.ArrayList
$ok = $false

function Write-JsonFile([string]$Path, $Object) {
  $json = $Object | ConvertTo-Json -Depth 8
  $dir = Split-Path $Path -Parent
  if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  [System.IO.File]::WriteAllText($Path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

try {
  Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
  $ok = $true
} catch {
  [void]$errors.Add($_.Exception.Message)
}

if (-not [string]::IsNullOrWhiteSpace($BackupDir)) {
  $result = [ordered]@{ ok = $ok; errors = @($errors) }
  Write-JsonFile (Join-Path $BackupDir 'remove-autostart-result.json') $result
}
