[CmdletBinding()]
param(
  [ValidateSet("nsis", "msi")]
  [string]$Bundle = "nsis",
  [switch]$NoBundle,
  [switch]$CleanDist,
  [switch]$Portable,
  [string]$PortableDir,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$appsRoot = Join-Path $root "apps"
$tauriDir = Join-Path $appsRoot "src-tauri"
$rootTarget = Join-Path $root "target"
$tauriTarget = Join-Path $tauriDir "target"
$distDir = Join-Path $appsRoot "dist"
$tauriConfig = Join-Path $tauriDir "tauri.conf.json"

$appName = "CodexManager"
if (Test-Path $tauriConfig) {
  $appName = (Get-Content $tauriConfig -Raw | ConvertFrom-Json).productName
}

$portableRoot = if ($PortableDir) { $PortableDir } else { Join-Path $root "portable" }
$portableExe = Join-Path $portableRoot "$appName.exe"
$appExe = Join-Path $tauriDir "target\\release\\$appName.exe"

function Write-Step {
  param([string]$Message)
  Write-Output $Message
}

function Remove-Dir {
  param([string]$Path)
  if (-not (Test-Path $Path)) {
    Write-Step "skip: $Path not found"
    return
  }
  if ($DryRun) {
    Write-Step "DRY RUN: remove $Path"
    return
  }
  & cmd /c "rmdir /s /q `"$Path`""
  if ($LASTEXITCODE -ne 0) {
    throw "failed to remove $Path"
  }
}

function Run-Cargo {
  param([string]$CommandLine, [scriptblock]$Action)
  if ($DryRun) {
    Write-Step "DRY RUN: $CommandLine"
    return
  }
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "command failed: $CommandLine"
  }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo not found in PATH"
}

if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
  Write-Warning "pnpm not found; tauri beforeBuildCommand may fail."
}

Push-Location $root
try {
  Remove-Dir $rootTarget
  Remove-Dir $tauriTarget
  if ($CleanDist) {
    Remove-Dir $distDir
  }

  Push-Location $tauriDir
  try {
    if ($NoBundle) {
      Run-Cargo "cargo tauri build --no-bundle" { cargo tauri build --no-bundle }
    } else {
      Run-Cargo "cargo tauri build --bundles $Bundle" { cargo tauri build --bundles $Bundle }
    }
  } finally {
    Pop-Location
  }

  if ($Portable) {
    if ($DryRun) {
      Write-Step "DRY RUN: stage portable -> $portableRoot"
      Write-Step "DRY RUN: copy $appExe -> $portableExe"
    } else {
      if (-not (Test-Path $portableRoot)) {
        New-Item -ItemType Directory -Force $portableRoot | Out-Null
      }
      if (-not (Test-Path $appExe)) {
        throw "missing app exe: $appExe"
      }
      Copy-Item -Force $appExe $portableExe
    }
  }
} finally {
  Pop-Location
}

Write-Step "done"
