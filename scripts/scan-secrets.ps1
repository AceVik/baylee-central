$ErrorActionPreference = 'Stop'
# Git environment variables can override an explicit repository path. Reject
# ambient routing before invoking Git, downloading tools or running fixtures.
$routingKeys = @(
    'GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR', 'GIT_INDEX_FILE',
    'GIT_OBJECT_DIRECTORY', 'GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_SHALLOW_FILE',
    'GIT_REPLACE_REF_BASE', 'GIT_NAMESPACE', 'GIT_REFERENCE_BACKEND'
)
foreach ($entry in Get-ChildItem Env:) {
    if ($entry.Name -in $routingKeys) {
        throw 'Secret history scan rejects overridden Git repository scope'
    }
}
Push-Location (Split-Path $PSScriptRoot)
try {
    $actualRoot = & git rev-parse --show-toplevel
    if ($LASTEXITCODE -ne 0) { throw 'Secret history repository identity could not be verified' }
    $comparison = if ($IsWindows) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
    if (-not [string]::Equals([IO.Path]::GetFullPath($actualRoot), [IO.Path]::GetFullPath((Get-Location).Path), $comparison)) {
        throw 'Secret history scan resolved a different repository'
    }
    $shallow = & git rev-parse --is-shallow-repository
    if ($LASTEXITCODE -ne 0 -or $shallow -ne 'false') { throw 'Secret history scan requires full Git history' }
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
    # The Linux job also exercises two distinct environment names that differ
    # only by case. Windows has a case-insensitive environment and uses one.
    $canaryName = 'GIT_CENTRAL_SCANNER_' + [IO.Path]::GetRandomFileName().Replace('.', '').ToUpperInvariant()
    $canaryNames = @($canaryName)
    if (-not $IsWindows) { $canaryNames += $canaryName.ToLowerInvariant() }
    foreach ($name in $canaryNames) {
        if ($null -ne [Environment]::GetEnvironmentVariable($name)) { throw 'Scanner environment canary collision' }
    }
    try {
        for ($index = 0; $index -lt $canaryNames.Count; $index++) {
            [Environment]::SetEnvironmentVariable($canaryNames[$index], "synthetic-$index")
        }
        & $PSScriptRoot/test-secret-scanner.ps1 -Binary $binary
        for ($index = 0; $index -lt $canaryNames.Count; $index++) {
            if ([Environment]::GetEnvironmentVariable($canaryNames[$index]) -cne "synthetic-$index") {
                throw 'Scanner fixture did not restore the caller environment exactly'
            }
        }
    } finally {
        foreach ($name in $canaryNames) {
            [Environment]::SetEnvironmentVariable($name, [NullString]::Value)
        }
    }
    & $binary dir . --config .gitleaks.toml --redact --no-banner
    if ($LASTEXITCODE -ne 0) { throw 'Secret scan failed' }
    & $binary git . --log-opts=--all --config .gitleaks-history.toml --redact --no-banner
    if ($LASTEXITCODE -ne 0) { throw 'Secret history scan failed' }
} finally { Pop-Location }
exit 0
