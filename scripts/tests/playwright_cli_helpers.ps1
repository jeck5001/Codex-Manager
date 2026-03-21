function Resolve-PlaywrightCliCommand {
  try {
    return (Get-Command npx.cmd -ErrorAction Stop).Source
  } catch {
    return (Get-Command npx -ErrorAction Stop).Source
  }
}

function Invoke-PlaywrightCli {
  param(
    [string]$CommandPath,
    [string]$Session,
    [string[]]$CliArgs
  )

  $args = @("--yes", "--package", "@playwright/cli", "playwright-cli")
  if ($Session) {
    $args += "-s=$Session"
  }
  $args += $CliArgs

  $output = & $CommandPath @args 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) {
    throw "playwright-cli failed:`n$output"
  }
  return $output.Trim()
}

function Get-PlaywrightEvalResult {
  param(
    [string]$CommandPath,
    [string]$Session,
    [string]$Expression
  )

  $output = Invoke-PlaywrightCli -CommandPath $CommandPath -Session $Session -CliArgs @("eval", $Expression)
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
    [string]$CommandPath,
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
      $result = Get-PlaywrightEvalResult -CommandPath $CommandPath -Session $Session -Expression $Expression
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
    [string]$CommandPath,
    [string]$Session,
    [string]$Text,
    [int]$TimeoutMs = 90000
  )

  $textLiteral = ConvertTo-JsSingleQuotedLiteral -Text $Text
  Wait-PageCondition -CommandPath $CommandPath -Session $Session -Expression "document.body.innerText.includes($textLiteral)" -Description "page text '$Text'" -TimeoutMs $TimeoutMs
}

function Invoke-PageClickByText {
  param(
    [string]$CommandPath,
    [string]$Session,
    [string]$Text,
    [int]$TimeoutMs = 10000
  )

  $xpath = "//*[self::button or @role='button' or @role='menuitem'][contains(normalize-space(.), '$Text')]"
  $xpathLiteral = ConvertTo-JsSingleQuotedLiteral -Text $xpath
  $expression = "(document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue?.click(), Boolean(document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue))"
  Wait-PageCondition -CommandPath $CommandPath -Session $Session -Expression $expression -Description "click '$Text'" -TimeoutMs $TimeoutMs
}

function Assert-NodeEnabledByText {
  param(
    [string]$CommandPath,
    [string]$Session,
    [string]$Text,
    [string]$Description
  )

  $xpath = "//*[self::button or self::input or self::textarea or @role='button'][contains(normalize-space(.), '$Text') or @value='$Text' or @placeholder='$Text']"
  $xpathLiteral = ConvertTo-JsSingleQuotedLiteral -Text $xpath
  $expression = "(window.__codexNode = document.evaluate($xpathLiteral, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue, Boolean(window.__codexNode) && !window.__codexNode.disabled && ((window.__codexNode.getAttribute && (window.__codexNode.getAttribute('aria-disabled') || '').toLowerCase()) !== 'true'))"
  Wait-PageCondition -CommandPath $CommandPath -Session $Session -Expression $expression -Description $Description
}

function Assert-ElementEnabled {
  param(
    [string]$CommandPath,
    [string]$Session,
    [string]$Selector,
    [string]$Description
  )

  $selectorLiteral = ConvertTo-JsSingleQuotedLiteral -Text $Selector
  Wait-PageCondition -CommandPath $CommandPath -Session $Session -Expression "(window.__codexNode = document.querySelector($selectorLiteral), Boolean(window.__codexNode) && !window.__codexNode.disabled)" -Description $Description
}

function Close-PlaywrightSession {
  param(
    [string]$CommandPath,
    [string]$Session
  )

  try {
    Invoke-PlaywrightCli -CommandPath $CommandPath -Session $Session -CliArgs @("close") | Out-Null
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
