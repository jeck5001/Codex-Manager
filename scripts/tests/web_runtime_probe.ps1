param(
  [string]$Base = "http://localhost:48761",
  [string]$RuntimeJsonPath = "",
  [string]$InitializeJsonPath = "",
  [string]$ExpectedMode = "web-gateway",
  [string]$ExpectedServiceName = "codexmanager-service"
)

$ErrorActionPreference = "Stop"

function Read-JsonPayload {
  param(
    [string]$Path
  )

  if ([string]::IsNullOrWhiteSpace($Path)) {
    throw "json path is empty"
  }
  if (-not (Test-Path $Path -PathType Leaf)) {
    throw "json file not found: $Path"
  }
  return Get-Content $Path -Raw | ConvertFrom-Json
}

function Join-BaseUrl {
  param(
    [string]$BaseUrl,
    [string]$RelativeOrAbsolute
  )

  if ([string]::IsNullOrWhiteSpace($RelativeOrAbsolute)) {
    throw "rpcBaseUrl is empty"
  }
  if ($RelativeOrAbsolute -match "^https?://") {
    return $RelativeOrAbsolute
  }

  $baseUri = [Uri]($BaseUrl.TrimEnd("/") + "/")
  return ([Uri]::new($baseUri, $RelativeOrAbsolute)).AbsoluteUri
}

function Read-RuntimePayload {
  param(
    [string]$BaseUrl,
    [string]$Path
  )

  if (-not [string]::IsNullOrWhiteSpace($Path)) {
    return Read-JsonPayload -Path $Path
  }

  $runtimeUrl = ($BaseUrl.TrimEnd("/") + "/api/runtime")
  return Invoke-RestMethod -Method Get -Uri $runtimeUrl -Headers @{
    "Accept" = "application/json"
  }
}

function Read-InitializePayload {
  param(
    [string]$RpcUrl,
    [string]$Path
  )

  if (-not [string]::IsNullOrWhiteSpace($Path)) {
    return Read-JsonPayload -Path $Path
  }

  $body = @{
    jsonrpc = "2.0"
    id = 1
    method = "initialize"
    params = @{}
  } | ConvertTo-Json -Depth 5

  return Invoke-RestMethod -Method Post -Uri $RpcUrl -ContentType "application/json" -Body $body
}

$runtime = Read-RuntimePayload -BaseUrl $Base -Path $RuntimeJsonPath
if ($runtime.mode -ne $ExpectedMode) {
  throw "unexpected runtime mode: expected '$ExpectedMode', got '$($runtime.mode)'"
}

$rpcUrl = Join-BaseUrl -BaseUrl $Base -RelativeOrAbsolute $runtime.rpcBaseUrl

$initializeResponse = Read-InitializePayload -RpcUrl $rpcUrl -Path $InitializeJsonPath
$initializeResult =
  if ($null -ne $initializeResponse.result) {
    $initializeResponse.result
  } else {
    $initializeResponse
  }

$serverName =
  if ($null -ne $initializeResult.serverName -and "$($initializeResult.serverName)".Trim()) {
    "$($initializeResult.serverName)".Trim()
  } elseif ($null -ne $initializeResult.server_name -and "$($initializeResult.server_name)".Trim()) {
    "$($initializeResult.server_name)".Trim()
  } else {
    ""
  }

if ($serverName -ne $ExpectedServiceName) {
  throw "unexpected service name: expected '$ExpectedServiceName', got '$serverName'"
}

$summary = [pscustomobject]@{
  Mode = "$($runtime.mode)"
  RpcUrl = $rpcUrl
  ServiceName = $serverName
  Version = "$($initializeResult.version)"
  CanManageService = [bool]$runtime.canManageService
  CanSelfUpdate = [bool]$runtime.canSelfUpdate
  CanCloseToTray = [bool]$runtime.canCloseToTray
  CanOpenLocalDir = [bool]$runtime.canOpenLocalDir
}

return $summary
