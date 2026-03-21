param(
  [string]$WebAddr = "",
  [string]$ServiceAddr = "",
  [string]$WebRoot = "",
  [switch]$SkipFrontendBuild,
  [switch]$SkipRustBuild,
  [switch]$KeepDataDir
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$webExe = Join-Path $repoRoot "target\debug\codexmanager-web.exe"
$serviceExe = Join-Path $repoRoot "target\debug\codexmanager-service.exe"
$runtimeProbeScript = Join-Path $repoRoot "scripts\tests\web_runtime_probe.ps1"
$playwrightCliStateDir = Join-Path $repoRoot ".playwright-cli"
$browserSession = "codexmanager-web-shell-smoke"
$runStamp = Get-Date -Format "yyyyMMdd_HHmmssfff"
$tempRoot = Join-Path $repoRoot (Join-Path ".tmp_web_shell_smoke" $runStamp)
$dataDir = Join-Path $tempRoot "data"
$dbPath = Join-Path $dataDir "codexmanager.db"
$rpcTokenFile = Join-Path $dataDir "codexmanager.rpc-token"
$resolvedWebRoot =
  if ([string]::IsNullOrWhiteSpace($WebRoot)) {
    Join-Path $repoRoot "apps\out"
  } elseif ([System.IO.Path]::IsPathRooted($WebRoot)) {
    $WebRoot
  } else {
    Join-Path $repoRoot $WebRoot
  }

. (Join-Path $PSScriptRoot "playwright_cli_helpers.ps1")
$npxCommand = Resolve-PlaywrightCliCommand

function Get-HostPort {
  param(
    [string]$Addr
  )

  $value = [string]$Addr
  $value = $value.Trim()
  if (-not $value) {
    throw "address is empty"
  }
  if ($value -match "^https?://") {
    $uri = [Uri]$value
    return [pscustomobject]@{
      Host = if ($uri.Host -eq "localhost") { "127.0.0.1" } else { $uri.Host }
      Port = $uri.Port
    }
  }

  if ($value -match "^[^/]+/") {
    $value = $value.Split("/")[0]
  }

  if ($value -match "^\d+$") {
    return [pscustomobject]@{
      Host = "127.0.0.1"
      Port = [int]$value
    }
  }

  if ($value.StartsWith("localhost:", [System.StringComparison]::OrdinalIgnoreCase)) {
    return [pscustomobject]@{
      Host = "127.0.0.1"
      Port = [int]($value.Substring("localhost:".Length))
    }
  }

  if ($value -match "^\[::\]:(\d+)$" -or $value -match "^\[::1\]:(\d+)$") {
    return [pscustomobject]@{
      Host = "127.0.0.1"
      Port = [int]$Matches[1]
    }
  }

  $host, $port = $value -split ":", 2
  if (-not $host -or -not $port) {
    throw "invalid address: $Addr"
  }
  $connectHost =
    if ($host -in @("0.0.0.0", "::", "[::]", "localhost")) {
      "127.0.0.1"
    } else {
      $host
    }
  return [pscustomobject]@{
    Host = $connectHost
    Port = [int]$port
  }
}

function Get-BrowserBaseUrl {
  param(
    [string]$Addr
  )

  $endpoint = Get-HostPort -Addr $Addr
  return "http://$($endpoint.Host):$($endpoint.Port)"
}

function Get-FreeTcpPort {
  $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
  try {
    $listener.Start()
    return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
  } finally {
    $listener.Stop()
  }
}

function Test-TcpEndpoint {
  param(
    [string]$Addr
  )

  $endpoint = Get-HostPort -Addr $Addr
  $client = New-Object System.Net.Sockets.TcpClient
  try {
    $async = $client.BeginConnect($endpoint.Host, $endpoint.Port, $null, $null)
    if (-not $async.AsyncWaitHandle.WaitOne(300)) {
      return $false
    }
    $client.EndConnect($async)
    return $true
  } catch {
    return $false
  } finally {
    $client.Dispose()
  }
}

function Assert-TcpEndpointAvailable {
  param(
    [string]$Addr,
    [string]$Description
  )

  if (Test-TcpEndpoint -Addr $Addr) {
    throw "$Description already in use: $Addr"
  }
}

function New-ManagedProcess {
  param(
    [string]$Name,
    [string]$FilePath,
    [string]$WorkingDirectory,
    [string[]]$ArgumentList,
    [hashtable]$Environment
  )

  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $FilePath
  $startInfo.WorkingDirectory = $WorkingDirectory
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  foreach ($argument in $ArgumentList) {
    [void]$startInfo.ArgumentList.Add($argument)
  }
  foreach ($entry in $Environment.GetEnumerator()) {
    $startInfo.Environment[$entry.Key] = [string]$entry.Value
  }

  $process = [System.Diagnostics.Process]::Start($startInfo)
  if ($null -eq $process) {
    throw "failed to start process: $Name"
  }

  [pscustomobject]@{
    Name = $Name
    Process = $process
    StdOutTask = $process.StandardOutput.ReadToEndAsync()
    StdErrTask = $process.StandardError.ReadToEndAsync()
  }
}

function Get-ManagedProcessLogText {
  param(
    [object]$ManagedProcess
  )

  if ($null -eq $ManagedProcess) {
    return ""
  }

  $stdout = ""
  $stderr = ""
  if ($ManagedProcess.Process.HasExited) {
    $ManagedProcess.Process.WaitForExit()
  }
  try {
    if ($ManagedProcess.StdOutTask.IsCompleted -or $ManagedProcess.Process.HasExited) {
      $stdout = [string]$ManagedProcess.StdOutTask.Result
    }
  } catch {
  }
  try {
    if ($ManagedProcess.StdErrTask.IsCompleted -or $ManagedProcess.Process.HasExited) {
      $stderr = [string]$ManagedProcess.StdErrTask.Result
    }
  } catch {
  }

  return @(
    "[$($ManagedProcess.Name) stdout]"
    ($stdout.TrimEnd())
    "[$($ManagedProcess.Name) stderr]"
    ($stderr.TrimEnd())
  ) -join [Environment]::NewLine
}

function Stop-ManagedProcess {
  param(
    [object]$ManagedProcess
  )

  if ($null -eq $ManagedProcess) {
    return
  }

  if (-not $ManagedProcess.Process.HasExited) {
    try {
      $ManagedProcess.Process.Kill($true)
    } catch {
    }
    [void]$ManagedProcess.Process.WaitForExit(10000)
  }
}

function Wait-TcpOpen {
  param(
    [string]$Addr,
    [string]$Description,
    [object]$ManagedProcess = $null,
    [int]$TimeoutMs = 20000
  )

  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  do {
    if (Test-TcpEndpoint -Addr $Addr) {
      return
    }
    if ($ManagedProcess -and $ManagedProcess.Process.HasExited) {
      throw "process exited before $Description became reachable:`n$(Get-ManagedProcessLogText -ManagedProcess $ManagedProcess)"
    }
    Start-Sleep -Milliseconds 250
  } while ([DateTime]::UtcNow -lt $deadline)

  if ($ManagedProcess -and $ManagedProcess.Process.HasExited) {
    throw "timed out waiting for $Description and process exited:`n$(Get-ManagedProcessLogText -ManagedProcess $ManagedProcess)"
  }
  throw "timed out waiting for $Description at $Addr"
}

function Wait-HttpOk {
  param(
    [string]$Url,
    [string]$Description,
    [object]$ManagedProcess = $null,
    [int]$TimeoutMs = 20000
  )

  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  do {
    try {
      $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3
      if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 400) {
        return
      }
    } catch {
    }
    if ($ManagedProcess -and $ManagedProcess.Process.HasExited) {
      throw "process exited before $Description became reachable:`n$(Get-ManagedProcessLogText -ManagedProcess $ManagedProcess)"
    }
    Start-Sleep -Milliseconds 250
  } while ([DateTime]::UtcNow -lt $deadline)

  if ($ManagedProcess -and $ManagedProcess.Process.HasExited) {
    throw "timed out waiting for $Description and process exited:`n$(Get-ManagedProcessLogText -ManagedProcess $ManagedProcess)"
  }
  throw "timed out waiting for $Description at $Url"
}

function Remove-TransientPathWithRetry {
  param(
    [string]$Path,
    [int]$RetryCount = 20,
    [int]$DelayMs = 250
  )

  for ($index = 0; $index -lt $RetryCount; $index += 1) {
    if (-not (Test-Path $Path)) {
      return
    }
    try {
      Remove-Item -Path $Path -Recurse -Force -ErrorAction Stop
      return
    } catch {
      Start-Sleep -Milliseconds $DelayMs
    }
  }
}

if (-not $SkipFrontendBuild) {
  & pnpm -C apps run build:desktop
  if ($LASTEXITCODE -ne 0) {
    throw "pnpm build:desktop failed"
  }
}

if (-not $SkipRustBuild) {
  & cargo build -p codexmanager-service -p codexmanager-web
  if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed"
  }
}

if (-not (Test-Path $serviceExe -PathType Leaf)) {
  throw "service executable not found: $serviceExe"
}
if (-not (Test-Path $webExe -PathType Leaf)) {
  throw "web executable not found: $webExe"
}
if (-not (Test-Path (Join-Path $resolvedWebRoot "index.html") -PathType Leaf)) {
  throw "web root missing index.html: $resolvedWebRoot"
}

$resolvedServiceAddr =
  if ([string]::IsNullOrWhiteSpace($ServiceAddr)) {
    "localhost:$(Get-FreeTcpPort)"
  } else {
    $ServiceAddr
  }
$resolvedWebAddr =
  if ([string]::IsNullOrWhiteSpace($WebAddr)) {
    $candidate = "localhost:$(Get-FreeTcpPort)"
    while ($candidate -eq $resolvedServiceAddr) {
      $candidate = "localhost:$(Get-FreeTcpPort)"
    }
    $candidate
  } else {
    $WebAddr
  }

Assert-TcpEndpointAvailable -Addr $resolvedServiceAddr -Description "service address"
Assert-TcpEndpointAvailable -Addr $resolvedWebAddr -Description "web address"

New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

$serviceProcess = $null
$webProcess = $null
$browserBaseUrl = Get-BrowserBaseUrl -Addr $resolvedWebAddr

try {
  $commonEnv = @{
    CODEXMANAGER_DB_PATH = $dbPath
    CODEXMANAGER_RPC_TOKEN_FILE = $rpcTokenFile
  }

  $serviceEnv = @{}
  foreach ($entry in $commonEnv.GetEnumerator()) {
    $serviceEnv[$entry.Key] = $entry.Value
  }
  $serviceEnv["CODEXMANAGER_SERVICE_ADDR"] = $resolvedServiceAddr

  $serviceProcess = New-ManagedProcess -Name "codexmanager-service" -FilePath $serviceExe -WorkingDirectory (Split-Path $serviceExe -Parent) -ArgumentList @() -Environment $serviceEnv
  Wait-TcpOpen -Addr $resolvedServiceAddr -Description "codexmanager-service" -ManagedProcess $serviceProcess

  $webEnv = @{}
  foreach ($entry in $commonEnv.GetEnumerator()) {
    $webEnv[$entry.Key] = $entry.Value
  }
  $webEnv["CODEXMANAGER_WEB_NO_OPEN"] = "1"
  $webEnv["CODEXMANAGER_WEB_NO_SPAWN_SERVICE"] = "1"
  $webEnv["CODEXMANAGER_SERVICE_ADDR"] = $resolvedServiceAddr
  $webEnv["CODEXMANAGER_WEB_ADDR"] = $resolvedWebAddr
  $webEnv["CODEXMANAGER_WEB_ROOT"] = $resolvedWebRoot

  $webProcess = New-ManagedProcess -Name "codexmanager-web" -FilePath $webExe -WorkingDirectory (Split-Path $webExe -Parent) -ArgumentList @() -Environment $webEnv
  Wait-HttpOk -Url ($browserBaseUrl.TrimEnd("/") + "/api/runtime") -Description "codexmanager-web runtime" -ManagedProcess $webProcess

  $runtimeSummary = & $runtimeProbeScript -Base $browserBaseUrl
  if ($runtimeSummary.CanManageService) {
    throw "unexpected capability: Web shell should not manage service directly"
  }
  if ($runtimeSummary.CanSelfUpdate) {
    throw "unexpected capability: Web shell should not enable desktop self-update"
  }

  Remove-TransientPathWithRetry -Path $playwrightCliStateDir
  Invoke-PlaywrightCli -CommandPath $npxCommand -Session "" -CliArgs @("close-all") | Out-Null

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $browserSession -CliArgs @("open", ($browserBaseUrl.TrimEnd("/") + "/accounts/")) | Out-Null
  Start-Sleep -Seconds 3
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "账号管理"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "账号操作"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $browserSession -Text "账号操作"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $browserSession -Text "添加账号"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "新增账号"
  Assert-NodeEnabledByText -CommandPath $npxCommand -Session $browserSession -Text "登录授权" -Description "login button should be enabled in add account modal"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $browserSession -CliArgs @("goto", ($browserBaseUrl.TrimEnd("/") + "/apikeys/")) | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "平台密钥"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "暂无平台密钥"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $browserSession -Text "创建密钥"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "创建平台密钥"
  Assert-ElementEnabled -CommandPath $npxCommand -Session $browserSession -Selector "#name" -Description "api key name input should be enabled"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $browserSession -CliArgs @("goto", ($browserBaseUrl.TrimEnd("/") + "/logs/")) | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "请求日志"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "暂无请求日志"

  Invoke-PlaywrightCli -CommandPath $npxCommand -Session $browserSession -CliArgs @("goto", ($browserBaseUrl.TrimEnd("/") + "/settings/")) | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "应用设置"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "Web / Docker 版不提供桌面应用更新检查"
  Invoke-PageClickByText -CommandPath $npxCommand -Session $browserSession -Text "密码"
  Wait-PageText -CommandPath $npxCommand -Session $browserSession -Text "访问密码"
  Assert-ElementEnabled -CommandPath $npxCommand -Session $browserSession -Selector "#password" -Description "web password input should be enabled"

  [pscustomobject]@{
    BaseUrl = $browserBaseUrl
    RuntimeMode = $runtimeSummary.Mode
    AccountsPage = "ok"
    ApiKeysPage = "ok"
    LogsPage = "ok"
    SettingsPage = "ok"
    PasswordModal = "ok"
    DataDir = if ($KeepDataDir) { $dataDir } else { "" }
  }
} finally {
  Close-PlaywrightSession -CommandPath $npxCommand -Session $browserSession
  try {
    Invoke-PlaywrightCli -CommandPath $npxCommand -Session "" -CliArgs @("close-all") | Out-Null
  } catch {
  }
  Remove-TransientPathWithRetry -Path $playwrightCliStateDir
  try {
    Invoke-WebRequest -Uri ($browserBaseUrl.TrimEnd("/") + "/__quit") -UseBasicParsing -TimeoutSec 3 | Out-Null
  } catch {
  }
  Stop-ManagedProcess -ManagedProcess $webProcess
  Stop-ManagedProcess -ManagedProcess $serviceProcess
  if (-not $KeepDataDir) {
    Remove-TransientPathWithRetry -Path $tempRoot
  }
}
