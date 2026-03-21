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
$npxCommand = try {
  (Get-Command npx.cmd -ErrorAction Stop).Source
} catch {
  (Get-Command npx -ErrorAction Stop).Source
}

function Invoke-PlaywrightCli {
  param(
    [string]$Session,
    [string[]]$CliArgs
  )

  $args = @("--yes", "--package", "@playwright/cli", "playwright-cli")
  if ($Session) {
    $args += "-s=$Session"
  }
  $args += $CliArgs

  $output = & $npxCommand @args 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) {
    throw "playwright-cli failed:`n$output"
  }
  return $output.Trim()
}

function Get-PlaywrightEvalResult {
  param(
    [string]$Session,
    [string]$Expression
  )

  $output = Invoke-PlaywrightCli -Session $Session -CliArgs @("eval", $Expression)
  $match = [regex]::Match($output, "(?s)### Result\s*(.+?)(?:\r?\n### |\s*$)")
  if (-not $match.Success) {
    throw "Cannot parse playwright eval result:`n$output"
  }
  return $match.Groups[1].Value.Trim()
}

function ConvertTo-JsSingleQuotedLiteral {
  param(
    [string]$Text
  )

  $value = [string]$Text
  $value = $value.Replace("\", "\\")
  $value = $value.Replace("'", "\'")
  $value = $value.Replace("`r", "\r")
  $value = $value.Replace("`n", "\n")
  return "'$value'"
}

function Wait-PageCondition {
  param(
    [string]$Session,
    [string]$Expression,
    [string]$Description,
    [int]$TimeoutMs = 90000
  )

  $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
  $lastResult = ""
  $lastError = ""
  do {
    try {
      $result = Get-PlaywrightEvalResult -Session $Session -Expression $Expression
      $lastResult = $result
      if ($result -match "^\s*true\s*$") {
        return
      }
    } catch {
      $lastError = $_.Exception.Message
    }
    Start-Sleep -Milliseconds 250
  } while ([DateTime]::UtcNow -lt $deadline)

  throw "Timed out waiting for: $Description`nLastResult: $lastResult`nLastError: $lastError"
}

function Wait-PageText {
  param(
    [string]$Session,
    [string]$Text,
    [int]$TimeoutMs = 90000
  )

  $textLiteral = ConvertTo-JsSingleQuotedLiteral -Text $Text
  Wait-PageCondition -Session $Session -Expression "document.body.innerText.includes($textLiteral)" -Description "page text '$Text'" -TimeoutMs $TimeoutMs
}

function Invoke-PageClickByText {
  param(
    [string]$Session,
    [string]$Text,
    [int]$TimeoutMs = 10000
  )

  $xpath = "//*[self::button or @role='button' or @role='menuitem'][contains(normalize-space(.), '$Text')]"
  $xpathLiteral = ConvertTo-JsSingleQuotedLiteral -Text $xpath
  $expression = "(document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue?.click(), Boolean(document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue))"
  Wait-PageCondition -Session $Session -Expression $expression -Description "click '$Text'" -TimeoutMs $TimeoutMs
}

function Assert-NodeEnabledByText {
  param(
    [string]$Session,
    [string]$Text,
    [string]$Description
  )

  $xpath = "//*[self::button or self::input or self::textarea or @role='button'][contains(normalize-space(.), '$Text') or @value='$Text' or @placeholder='$Text']"
  $xpathLiteral = ConvertTo-JsSingleQuotedLiteral -Text $xpath
  $expression = "(window.__codexNode = document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue, Boolean(window.__codexNode) && !window.__codexNode.disabled && ((window.__codexNode.getAttribute && (window.__codexNode.getAttribute('aria-disabled') || '').toLowerCase()) !== 'true'))"
  Wait-PageCondition -Session $Session -Expression $expression -Description $Description
}

function Assert-ElementEnabled {
  param(
    [string]$Session,
    [string]$Selector,
    [string]$Description
  )

  $selectorLiteral = ConvertTo-JsSingleQuotedLiteral -Text $Selector
  Wait-PageCondition -Session $Session -Expression "(window.__codexNode = document.querySelector($selectorLiteral), Boolean(window.__codexNode) && !window.__codexNode.disabled)" -Description $Description
}

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

function Close-PlaywrightSession {
  param(
    [string]$Session
  )

  try {
    Invoke-PlaywrightCli -Session $Session -CliArgs @("close") | Out-Null
  } catch {
  }
}

function Remove-TransientPath {
  param(
    [string]$Path
  )

  if (-not (Test-Path $Path)) {
    return
  }

  try {
    Remove-Item -Path $Path -Recurse -Force -ErrorAction Stop
  } catch {
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
  Invoke-PlaywrightCli -Session "" -CliArgs @("close-all") | Out-Null

  $supportedServer = New-BackgroundProcess -FilePath "node" -ArgumentList @(
    $mockServerScript,
    "--port",
    "$SupportedPort",
    "--mode",
    "supported"
  ) -WorkingDirectory $repoRoot
  Wait-HttpOk -Url "http://127.0.0.1:$SupportedPort/__health" -Description "supported mock server"

  Invoke-PlaywrightCli -Session $supportedSession -CliArgs @("open", "http://127.0.0.1:$SupportedPort/accounts/") | Out-Null
  Start-Sleep -Seconds 3
  Wait-PageText -Session $supportedSession -Text "账号管理"
  Wait-PageText -Session $supportedSession -Text "demo-primary@example.com"

  Invoke-PageClickByText -Session $supportedSession -Text "账号操作"
  Invoke-PageClickByText -Session $supportedSession -Text "添加账号"
  Wait-PageText -Session $supportedSession -Text "新增账号"
  Assert-NodeEnabledByText -Session $supportedSession -Text "登录授权" -Description "login button should be enabled in add account modal"

  Invoke-PlaywrightCli -Session $supportedSession -CliArgs @("goto", "http://127.0.0.1:$SupportedPort/apikeys/") | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -Session $supportedSession -Text "平台密钥"
  Wait-PageText -Session $supportedSession -Text "Web Smoke Key"
  Invoke-PageClickByText -Session $supportedSession -Text "创建密钥"
  Wait-PageText -Session $supportedSession -Text "创建平台密钥"
  Assert-ElementEnabled -Session $supportedSession -Selector "#name" -Description "api key name input should be enabled"

  Invoke-PlaywrightCli -Session $supportedSession -CliArgs @("goto", "http://127.0.0.1:$SupportedPort/logs/") | Out-Null
  Start-Sleep -Seconds 2
  Wait-PageText -Session $supportedSession -Text "请求日志"
  Wait-PageText -Session $supportedSession -Text "/v1/responses"

  Invoke-PageClickByText -Session $supportedSession -Text "密码"
  Wait-PageText -Session $supportedSession -Text "访问密码"
  Assert-ElementEnabled -Session $supportedSession -Selector "#password" -Description "web password input should be enabled"

  Close-PlaywrightSession -Session $supportedSession
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

  Invoke-PlaywrightCli -Session $unsupportedSession -CliArgs @("open", "http://127.0.0.1:$UnsupportedPort/") | Out-Null
  Start-Sleep -Seconds 3
  Wait-PageText -Session $unsupportedSession -Text "当前 Web 运行方式不受支持" -TimeoutMs 45000
  Wait-PageText -Session $unsupportedSession -Text "/api/runtime" -TimeoutMs 45000

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
  Close-PlaywrightSession -Session $supportedSession
  Close-PlaywrightSession -Session $unsupportedSession
  try {
    Invoke-PlaywrightCli -Session "" -CliArgs @("close-all") | Out-Null
  } catch {
  }
  Stop-BackgroundProcess -Process $supportedServer
  Stop-BackgroundProcess -Process $unsupportedServer
  Remove-TransientPath -Path $playwrightCliStateDir
}
