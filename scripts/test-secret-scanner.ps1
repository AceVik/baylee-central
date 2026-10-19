param([Parameter(Mandatory)][string]$Binary)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$project = Split-Path $PSScriptRoot
$fixtureRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) ('central-secret-scan-' + [IO.Path]::GetRandomFileName())))
$previousGitEnvironment = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
foreach ($entry in Get-ChildItem Env:) {
    if ($entry.Name.StartsWith('GIT_', [StringComparison]::OrdinalIgnoreCase)) {
        $previousGitEnvironment[$entry.Name] = $entry.Value
    }
}
try {
    [IO.Directory]::CreateDirectory($fixtureRoot) | Out-Null
    $empty = Join-Path $fixtureRoot 'empty'
    [IO.Directory]::CreateDirectory($empty) | Out-Null
    $gitConfig = Join-Path $empty 'config'
    [IO.File]::WriteAllText($gitConfig, '')
    # Synthetic commits must not invoke user hooks, signers, custom indexes or UI.
    foreach ($key in $previousGitEnvironment.Keys) { [Environment]::SetEnvironmentVariable($key, [NullString]::Value) }
    [Environment]::SetEnvironmentVariable('GIT_CONFIG_NOSYSTEM', '1')
    [Environment]::SetEnvironmentVariable('GIT_CONFIG_GLOBAL', $gitConfig)
    function Assert-AmbientGitScopeRejected([string]$Name) {
        # Mock Git only inside this invocation: a missing guard must fail before
        # any real repository discovery, fixture recursion or tool download.
        function git { throw 'Untrusted repository scope reached Git' }
        [Environment]::SetEnvironmentVariable($Name, 'SYNTHETIC_SCOPE_CANARY')
        try {
            try {
                & (Join-Path $project 'scripts/scan-secrets.ps1')
                throw 'Secret scanner accepted overridden repository scope'
            } catch {
                if ($_.Exception.Message -ne 'Secret history scan rejects overridden Git repository scope') { throw }
            }
        } finally { [Environment]::SetEnvironmentVariable($Name, [NullString]::Value) }
    }
    foreach ($key in @(
        'GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR', 'GIT_INDEX_FILE',
        'GIT_OBJECT_DIRECTORY', 'GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_SHALLOW_FILE',
        'GIT_REPLACE_REF_BASE', 'GIT_NAMESPACE', 'GIT_REFERENCE_BACKEND'
    )) { Assert-AmbientGitScopeRejected $key }
    $repo = Join-Path $fixtureRoot 'repository'
    & git init --quiet "--template=$empty" $repo
    if ($LASTEXITCODE -ne 0) { throw 'Secret-scanner fixture initialization failed' }
    $paths = @('config.local.json', 'target/config.json', 'source.txt')
    foreach ($path in $paths) {
        $file = Join-Path $repo $path
        [IO.Directory]::CreateDirectory((Split-Path $file)) | Out-Null
        $canary = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
        [IO.File]::WriteAllText($file, '{"api_key":"' + $canary + '"}')
    }
    & git -C $repo add --force .
    if ($LASTEXITCODE -ne 0) { throw 'Secret-scanner fixture staging failed' }
    & git -C $repo -c "core.hooksPath=$empty" -c commit.gpgSign=false -c user.name=Fixture -c user.email=fixture@example.invalid commit --quiet -m 'Synthetic scanner fixtures'
    if ($LASTEXITCODE -ne 0) { throw 'Secret-scanner fixture commit failed' }
    $workingReport = Join-Path $fixtureRoot 'working-report.json'
    & $Binary dir $repo --config (Join-Path $project '.gitleaks.toml') --redact=100 --no-banner --report-format=json "--report-path=$workingReport" *> $null
    if ($LASTEXITCODE -ne 1) { throw 'Working-tree scanner missed its positive control' }
    $workingFindings = @(Get-Content -LiteralPath $workingReport -Raw | ConvertFrom-Json)
    if ($workingFindings.Count -ne 1) { throw 'Working-tree exclusions changed unexpectedly' }
    $reportedPath = [IO.Path]::GetFullPath($workingFindings[0].File, $repo)
    if ([IO.Path]::GetRelativePath($repo, $reportedPath) -ne 'source.txt') {
        throw 'Working-tree exclusions changed unexpectedly'
    }
    & git -C $repo rm --quiet --force -- $paths
    if ($LASTEXITCODE -ne 0) { throw 'Secret-scanner fixture deletion failed' }
    & git -C $repo -c "core.hooksPath=$empty" -c commit.gpgSign=false -c user.name=Fixture -c user.email=fixture@example.invalid commit --quiet -m 'Remove synthetic credentials from current tree'
    if ($LASTEXITCODE -ne 0) { throw 'Secret-scanner fixture deletion commit failed' }
    $historyReport = Join-Path $fixtureRoot 'history-report.json'
    & $Binary git $repo --log-opts=--all --config (Join-Path $project '.gitleaks-history.toml') --redact=100 --no-banner --report-format=json "--report-path=$historyReport" *> $null
    if ($LASTEXITCODE -ne 1) { throw 'History scanner missed deleted synthetic credentials' }
    $historyFindings = @(Get-Content -LiteralPath $historyReport -Raw | ConvertFrom-Json)
    if ($historyFindings.Count -ne $paths.Count) { throw 'History scanner retained a path exclusion' }
    foreach ($path in $paths) {
        if ($path -cnotin $historyFindings.File) { throw 'History scanner skipped a synthetic credential path' }
    }
    Write-Output 'Secret-scanner regressions passed: committed/deleted local metadata and output paths are scanned.'
} finally {
    try {
        # Preserve distinct names on Linux, including GIT_FOO and git_foo.
        foreach ($entry in Get-ChildItem Env:) {
            if ($entry.Name.StartsWith('GIT_', [StringComparison]::OrdinalIgnoreCase)) {
                [Environment]::SetEnvironmentVariable($entry.Name, [NullString]::Value)
            }
        }
        foreach ($key in $previousGitEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($key, $previousGitEnvironment[$key])
        }
    } finally {
        # An environment restoration failure must not retain the synthetic repo.
        $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
        if (-not $fixtureRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe secret-scanner fixture cleanup path' }
        if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
    }
}
exit 0
