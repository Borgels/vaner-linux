param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,

  [string]$ArtifactDir = "artifacts/windows-smoke"
)

$ErrorActionPreference = "Stop"

function Fail($message) {
  Write-Error "::error::$message"
  exit 1
}

function Find-VanerExe {
  $roots = @(
    (Join-Path $env:LOCALAPPDATA "Programs\Vaner"),
    (Join-Path $env:LOCALAPPDATA "Vaner"),
    (Join-Path $env:ProgramFiles "Vaner")
  )

  if (${env:ProgramFiles(x86)}) {
    $roots += (Join-Path ${env:ProgramFiles(x86)} "Vaner")
  }

  foreach ($root in $roots) {
    if (-not (Test-Path $root)) {
      continue
    }
    $exe = Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -in @("Vaner.exe", "vaner-desktop.exe") } |
      Sort-Object FullName |
      Select-Object -First 1
    if ($exe) {
      return $exe.FullName
    }
  }

  return $null
}

New-Item -ItemType Directory -Force -Path $ArtifactDir | Out-Null
$logPath = Join-Path $ArtifactDir "windows-ui-smoke.log"
$screenshotPath = Join-Path $ArtifactDir "vaner-window.png"

"installer=$InstallerPath" | Out-File -FilePath $logPath -Encoding utf8

Get-Process -Name "Vaner", "vaner-desktop" -ErrorAction SilentlyContinue |
  Stop-Process -Force -ErrorAction SilentlyContinue

Start-Process -FilePath $InstallerPath -ArgumentList "/S" -Wait

$exePath = Find-VanerExe
if (-not $exePath) {
  Fail "Vaner executable was not found after NSIS install."
}
"exe=$exePath" | Out-File -FilePath $logPath -Encoding utf8 -Append

$env:VANER_DISABLE_UPDATER = "1"
$env:VANER_DESKTOP_SHOW_ON_START = "1"
$env:VANER_DESKTOP_LOCAL_BUILD = "1"

$process = Start-Process -FilePath $exePath -PassThru
try {
  $handle = [IntPtr]::Zero
  $deadline = (Get-Date).AddSeconds(45)
  while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
    $process.Refresh()
    if ($process.HasExited) {
      Fail "Vaner exited during Windows UI smoke test with code $($process.ExitCode)."
    }
    if ($process.MainWindowHandle -ne 0) {
      $handle = $process.MainWindowHandle
      break
    }
  }

  if ($handle -eq [IntPtr]::Zero) {
    Fail "Vaner did not create a visible main window within 45 seconds."
  }

  Add-Type @"
using System;
using System.Runtime.InteropServices;

public struct RECT {
  public int Left;
  public int Top;
  public int Right;
  public int Bottom;
}

public static class VanerWin32 {
  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);
}
"@

  [VanerWin32]::SetForegroundWindow($handle) | Out-Null
  Start-Sleep -Seconds 4

  $rect = New-Object RECT
  if (-not [VanerWin32]::GetWindowRect($handle, [ref]$rect)) {
    Fail "Could not read Vaner window bounds."
  }

  $width = $rect.Right - $rect.Left
  $height = $rect.Bottom - $rect.Top
  "window=$($rect.Left),$($rect.Top),${width}x${height}" |
    Out-File -FilePath $logPath -Encoding utf8 -Append

  if ($width -lt 300 -or $height -lt 250) {
    Fail "Vaner window is unexpectedly small: ${width}x${height}."
  }

  Add-Type -AssemblyName System.Drawing
  $bitmap = New-Object System.Drawing.Bitmap $width, $height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
  $bitmap.Save($screenshotPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $graphics.Dispose()

  $unique = New-Object 'System.Collections.Generic.HashSet[int]'
  $samples = 0
  $nonBackground = 0
  $stepX = [Math]::Max(1, [int]($width / 40))
  $stepY = [Math]::Max(1, [int]($height / 40))

  for ($x = 0; $x -lt $width; $x += $stepX) {
    for ($y = 0; $y -lt $height; $y += $stepY) {
      $argb = $bitmap.GetPixel($x, $y).ToArgb()
      [void]$unique.Add($argb)
      $samples += 1
      $color = [System.Drawing.Color]::FromArgb($argb)
      if ($color.R -gt 8 -or $color.G -gt 8 -or $color.B -gt 8) {
        $nonBackground += 1
      }
    }
  }
  $bitmap.Dispose()

  "samples=$samples unique_colors=$($unique.Count) non_background=$nonBackground screenshot=$screenshotPath" |
    Out-File -FilePath $logPath -Encoding utf8 -Append

  if ($unique.Count -lt 8 -or $nonBackground -lt 20) {
    Fail "Vaner window screenshot appears blank. See $screenshotPath."
  }

  Write-Host "Windows UI smoke test passed: $screenshotPath"
}
finally {
  if ($process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }
}
