$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
try {
    $version = '8.30.1'
    $toolPath = Join-Path (Get-Location) 'target/gitleaks'
    [IO.Directory]::CreateDirectory($toolPath) | Out-Null
    if ($IsWindows) {
        $asset = "gitleaks_${version}_windows_x64.zip"
        $expected = 'd29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e'
        $binary = Join-Path $toolPath 'gitleaks.exe'
    } elseif ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq [Runtime.InteropServices.Architecture]::X64 -and $IsLinux) {
        $asset = "gitleaks_${version}_linux_x64.tar.gz"
        $expected = '551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb'
        $binary = Join-Path $toolPath 'gitleaks'
    } else { throw 'Secret scanner supports the CI x64 Windows/Linux targets only' }
    $archive = Join-Path $toolPath $asset
    if (-not (Test-Path -LiteralPath $archive)) {
        Invoke-WebRequest -Uri "https://github.com/gitleaks/gitleaks/releases/download/v$version/$asset" -OutFile $archive
    }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
        throw 'Secret scanner archive checksum mismatch'
    }
    if ($IsWindows) { Expand-Archive -LiteralPath $archive -DestinationPath $toolPath -Force }
    else {
        & tar -xzf $archive -C $toolPath
        if ($LASTEXITCODE -ne 0) { throw 'Secret scanner extraction failed' }
    }
    & $binary dir . --config .gitleaks.toml --redact --no-banner
    if ($LASTEXITCODE -ne 0) { throw 'Secret scan failed' }
} finally { Pop-Location }
exit 0
