param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateNotNullOrEmpty()]
  [string] $InputPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not (Test-Path -LiteralPath $InputPath -PathType Leaf)) {
  throw "Windows signing input does not exist: $InputPath"
}

$signingEnv = @("ES_USERNAME", "ES_PASSWORD", "CREDENTIAL_ID", "ES_TOTP_SECRET")
$presentSigningEnv = @()
$missingSigningEnv = @()
foreach ($name in $signingEnv) {
  $value = [Environment]::GetEnvironmentVariable($name)
  if ([string]::IsNullOrWhiteSpace($value)) {
    $missingSigningEnv += $name
  } else {
    $presentSigningEnv += $name
  }
}

if ($presentSigningEnv.Count -eq 0) {
  Write-Host "Windows signing environment not present; leaving unsigned: $InputPath"
  exit 0
}

if ($missingSigningEnv.Count -gt 0) {
  throw "Missing Windows signing environment variables: $($missingSigningEnv -join ', ')"
}

$toolEnv = @("CODESIGNTOOL", "CODE_SIGN_TOOL_PATH")
$missingToolEnv = @()
foreach ($name in $toolEnv) {
  $value = [Environment]::GetEnvironmentVariable($name)
  if ([string]::IsNullOrWhiteSpace($value)) {
    $missingToolEnv += $name
  }
}

if ($missingToolEnv.Count -gt 0) {
  throw "Missing Windows signing tool environment variables: $($missingToolEnv -join ', ')"
}

if (-not (Test-Path -LiteralPath $env:CODESIGNTOOL -PathType Leaf)) {
  throw "CODESIGNTOOL does not point at a file: $env:CODESIGNTOOL"
}

if (-not (Test-Path -LiteralPath $env:CODE_SIGN_TOOL_PATH -PathType Container)) {
  throw "CODE_SIGN_TOOL_PATH does not point at a directory: $env:CODE_SIGN_TOOL_PATH"
}

$codeSignTool = (Resolve-Path -LiteralPath $env:CODESIGNTOOL).ProviderPath
$toolRoot = (Resolve-Path -LiteralPath $env:CODE_SIGN_TOOL_PATH).ProviderPath
$resolvedInput = (Resolve-Path -LiteralPath $InputPath).ProviderPath

Write-Host "Signing Windows artifact: $resolvedInput"
Push-Location -LiteralPath $toolRoot
try {
  & $codeSignTool "sign" `
    "-username=$env:ES_USERNAME" `
    "-password=$env:ES_PASSWORD" `
    "-credential_id=$env:CREDENTIAL_ID" `
    "-totp_secret=$env:ES_TOTP_SECRET" `
    "-input_file_path=$resolvedInput" `
    "-override"
  if ($LASTEXITCODE -ne 0) {
    throw "CodeSignTool failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}
