param(
  [int]$SupportedPort = 49681,
  [int]$UnsupportedPort = 49682,
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$mockServerScript = Join-Path $repoRoot "scripts\tests\web_ui_mock_server.mjs"
$supportedSession = "codexmanager-web-ui-smoke-supported"
$unsupportedSession = "codexmanager-web-ui-smoke-unsupported"
$playwrightCliStateDir = Join-Path $repoRoot ".playwright-cli"
. (Join-Path $PSScriptRoot "playwright_cli_helpers.ps1")
$npxCommand = Resolve-PlaywrightCliCommand

function New-BackgroundProcess {
  param(
    [string]$FilePath,
    [string[]]$ArgumentList,
    [string]$WorkingDirectory
  )

  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $FilePath
  $startInfo.WorkingDirectory = $WorkingDirectory
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  foreach ($argument in $ArgumentList) {
    [void]$startInfo.ArgumentList.Add($argument)
  }

  return [System.Diagnostics.Process]::Start($startInfo)
}

function Wait-HttpOk {
  param(
    [string]$Url,
    [string]$Description,
    [int]$TimeoutMs = 10000
  )

  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  do {
    try {
      $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2
      if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 400) {
        return
      }
    } catch {
    }
    Start-Sleep -Milliseconds 200
  } while ([DateTime]::UtcNow -lt $deadline)

  throw "Timed out waiting for $Description at $Url"
}

function Stop-BackgroundProcess {
  param(
    [System.Diagnostics.Process]$Process
  )

  if ($null -eq $Process) {
    return
  }
  if (-not $Process.HasExited) {
    $Process.Kill($true)
    $Process.WaitForExit()
  }
}

if (-not $SkipBuild) {
  & pnpm -C apps run build:desktop
  if ($LASTEXITCODE -ne 0) {
    throw "pnpm build:desktop failed"
  }
}

$supportedServer = $null
$unsupportedServer = $null

try {
  Remove-TransientPath -Path $playwrightCliStateDir
  Invoke-PlaywrightCli -CommandPath $npxCommand -Session "" -CliArgs @("close-all") | Out-Null

  $supportedServer = New-BackgroundProcess -FilePath "node" -ArgumentList @(
    $mockServerScript,
    "--port",
    "$SupportedPort",
    "--mode",
    "supported"
  ) -WorkingDirectory $repoRoot
  Wait-HttpOk -Url "http://127.0.0.1:$SupportedPort/__health" -Description "supported mock server"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $supportedSession -CliArgs @("open", "http://127.0.0.1:$SupportedPort/accounts/") | Out-Null
  Start-Sleep -Seconds 3
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "账号管理"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "demo-primary@example.com"

  Invoke-PageClickByText -CommandPath $npxCommand -Session $supportedSession -Text "账号操作"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $supportedSession -Text "添加账号"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "新增账号"
  Assert-NodeEnabledByText -CommandPath $npxCommand -Session $supportedSession -Text "登录授权" -Description "login button should be enabled in add account modal"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $supportedSession -CliArgs @("goto", "http://127.0.0.1:$SupportedPort/apikeys/") | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "平台密钥"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "Web Smoke Key"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $supportedSession -Text "创建密钥"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "创建平台密钥"
  Assert-ElementEnabled -CommandPath $npxCommand -Session $supportedSession -Selector "#name" -Description "api key name input should be enabled"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $supportedSession -CliArgs @("goto", "http://127.0.0.1:$SupportedPort/logs/") | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "请求日志"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "/v1/responses"

  Invoke-PageClickByText -CommandPath $npxCommand -Session $supportedSession -Text "密码"
  Wait-PageText -CommandPath $npxCommand -Session $supportedSession -Text "访问密码"
  Assert-ElementEnabled -CommandPath $npxCommand -Session $supportedSession -Selector "#password" -Description "web password input should be enabled"

  Close-PlaywrightSession -CommandPath $npxCommand -Session $supportedSession
  Stop-BackgroundProcess -Process $supportedServer
  $supportedServer = $null

  $unsupportedServer = New-BackgroundProcess -FilePath "node" -ArgumentList @(
    $mockServerScript,
    "--port",
    "$UnsupportedPort",
    "--mode",
    "unsupported"
  ) -WorkingDirectory $repoRoot
  Wait-HttpOk -Url "http://127.0.0.1:$UnsupportedPort/" -Description "unsupported mock server"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $unsupportedSession -CliArgs @("open", "http://127.0.0.1:$UnsupportedPort/") | Out-Null
  Start-Sleep -Seconds 3
  Wait-PageText -CommandPath $npxCommand -Session $unsupportedSession -Text "当前 Web 运行方式不受支持" -TimeoutMs 45000
  Wait-PageText -CommandPath $npxCommand -Session $unsupportedSession -Text "/api/runtime" -TimeoutMs 45000

  [pscustomobject]@{
    SupportedBase = "http://127.0.0.1:$SupportedPort"
    UnsupportedBase = "http://127.0.0.1:$UnsupportedPort"
    AccountsPage = "ok"
    ApiKeysPage = "ok"
    LogsPage = "ok"
    PasswordModal = "ok"
    UnsupportedOverlay = "ok"
  }
} finally {
  Close-PlaywrightSession -CommandPath $npxCommand -Session $supportedSession
  Close-PlaywrightSession -CommandPath $npxCommand -Session $unsupportedSession
  try {
    Invoke-PlaywrightCli -CommandPath $npxCommand -Session "" -CliArgs @("close-all") | Out-Null
  } catch {
  }
  Stop-BackgroundProcess -Process $supportedServer
  Stop-BackgroundProcess -Process $unsupportedServer
  Remove-TransientPath -Path $playwrightCliStateDir
}
