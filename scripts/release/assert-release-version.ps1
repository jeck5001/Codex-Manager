param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$CargoTomlPath = 'apps/src-tauri/Cargo.toml',
    [string]$TauriConfigPath = 'apps/src-tauri/tauri.conf.json'
)

$tagInput = $Tag
if ([string]::IsNullOrWhiteSpace($tagInput)) {
    throw 'release tag is required'
}

$normalizedTag = if ($tagInput.StartsWith('v')) { $tagInput } else { "v$tagInput" }
if ($normalizedTag -notmatch '^v(\d+\.\d+\.\d+)(?:[-+].*)?$') {
    throw "tag must look like vX.Y.Z or vX.Y.Z-suffix, got: $normalizedTag"
}
$tagVersion = $Matches[1]

if (-not (Test-Path $CargoTomlPath -PathType Leaf)) {
    throw "Cargo.toml not found: $CargoTomlPath"
}
$cargoToml = Get-Content $CargoTomlPath -Raw
if ($cargoToml -notmatch '(?ms)\[package\].*?^\s*version\s*=\s*"([^"]+)"') {
    throw "failed to read [package].version from $CargoTomlPath"
}
$cargoVersion = $Matches[1]

if (-not (Test-Path $TauriConfigPath -PathType Leaf)) {
    throw "tauri config not found: $TauriConfigPath"
}
$tauriConf = (Get-Content $TauriConfigPath -Raw) | ConvertFrom-Json
$tauriVersion = $tauriConf.version
if ([string]::IsNullOrWhiteSpace($tauriVersion)) {
    throw "$TauriConfigPath missing version"
}

if ($cargoVersion -ne $tauriVersion) {
    throw "version mismatch: $CargoTomlPath=$cargoVersion $TauriConfigPath=$tauriVersion"
}
if ($cargoVersion -ne $tagVersion) {
    throw "tag/version mismatch: tag=$normalizedTag expects $tagVersion, but app version is $cargoVersion"
}

Write-Host "Version OK: $cargoVersion (tag $normalizedTag)"
