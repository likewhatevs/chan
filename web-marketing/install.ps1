# chan installer for Windows (PowerShell 5+).
#
#   irm https://chan.app/install.ps1 | iex
#
# Detects arch (amd64 / arm64), downloads the matching .zip from
# chan.app, extracts chan.exe into %USERPROFILE%\.chan\bin and adds
# it to the user PATH.
#
# Override:
#   $env:CHAN_BASE   = 'https://staging.chan.app' ; irm ... | iex
#   $env:CHAN_PREFIX = 'C:\Tools\chan' ; irm ... | iex

$ErrorActionPreference = 'Stop'

$base   = if ($env:CHAN_BASE)   { $env:CHAN_BASE }   else { 'https://chan.app' }
$prefix = if ($env:CHAN_PREFIX) { $env:CHAN_PREFIX } else { Join-Path $env:USERPROFILE '.chan' }

$arch = (Get-CimInstance Win32_Processor).Architecture
# Win32_Processor.Architecture: 9=x64, 12=ARM64.
switch ($arch) {
    9  { $asset = 'chan-x86_64-pc-windows-msvc.zip' }
    12 { $asset = 'chan-aarch64-pc-windows-msvc.zip' }
    default { throw "Unsupported CPU architecture code: $arch" }
}

$url    = "$base/dl/latest/$asset"
$bindir = Join-Path $prefix 'bin'
New-Item -ItemType Directory -Force -Path $bindir | Out-Null

$tmp = Join-Path $env:TEMP ("chan-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
try {
    $zip = Join-Path $tmp 'chan.zip'
    Write-Host "install: downloading $url"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $zip

    Expand-Archive -LiteralPath $zip -DestinationPath $tmp -Force
    $exe = Get-ChildItem -Path $tmp -Recurse -Filter 'chan.exe' | Select-Object -First 1
    if (-not $exe) { throw "chan.exe not found inside $asset" }

    Copy-Item -Force -Path $exe.FullName -Destination (Join-Path $bindir 'chan.exe')
    Write-Host "install: $(Join-Path $bindir 'chan.exe')"
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmp
}

# Ensure the bin dir is on the user PATH for future shells. Skip the
# rewrite if it's already there; warn that the current shell still
# has the old PATH.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not ($userPath -split ';' | Where-Object { $_ -ieq $bindir })) {
    $newPath = if ([string]::IsNullOrEmpty($userPath)) { $bindir } else { "$userPath;$bindir" }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Write-Host "install: added $bindir to user PATH (open a new terminal to pick it up)"
}
