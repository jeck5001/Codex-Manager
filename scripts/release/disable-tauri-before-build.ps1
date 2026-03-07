param(
    [string]$ConfigPath = 'apps/src-tauri/tauri.conf.json'
)

if (-not (Test-Path $ConfigPath -PathType Leaf)) {
    throw "tauri config not found: $ConfigPath"
}

$config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
if (-not $config.build) {
    throw 'tauri.conf.json missing build section'
}

$hadBeforeBuildCommand = $config.build.PSObject.Properties.Name -contains 'beforeBuildCommand'
$config.build.PSObject.Properties.Remove('beforeBuildCommand') | Out-Null
$config | ConvertTo-Json -Depth 100 | Set-Content -Encoding utf8 $ConfigPath

if ($hadBeforeBuildCommand) {
    Write-Host "Removed beforeBuildCommand from $ConfigPath"
} else {
    Write-Host "beforeBuildCommand already absent in $ConfigPath"
}
