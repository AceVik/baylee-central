param([switch]$CompilerProbe)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot)
try {
    $raw = & cargo metadata --all-features --locked --offline --format-version 1 --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $meta = $raw | ConvertFrom-Json
    $workspaceManifest = Get-Content Cargo.toml -Raw
    if ($workspaceManifest -notmatch '(?m)^unsafe_code\s*=\s*"forbid"\s*$') {
        throw 'Workspace unsafe prohibition is missing'
    }
    foreach ($package in $meta.packages) {
        $manifest = Get-Content -LiteralPath $package.manifest_path -Raw
        if ($manifest -notmatch '(?ms)\[lints\]\s*workspace\s*=\s*true') {
            throw "Missing inherited lints: $($package.name)"
        }
        if ($package.publish.Count -ne 0) { throw "Package publication must be disabled: $($package.name)" }
        foreach ($target in $package.targets) {
            $source = Get-Content -LiteralPath $target.src_path -Raw
            if ($source -notmatch '(?m)^#!\[forbid\(unsafe_code\)\]\s*$') {
                throw "Missing crate-root unsafe prohibition: $($target.name)"
            }
        }
    }
    foreach ($file in Get-ChildItem crates -Recurse -File) {
        if ($file.Extension -in @('.s', '.asm', '.c', '.cpp', '.cc', '.h')) {
            throw "Native/assembly source forbidden: $($file.Name)"
        }
        if ($file.Extension -eq '.rs') {
            $source = Get-Content -LiteralPath $file.FullName -Raw
            if ($source -match '\bunsafe\s*(\{|fn\b|impl\b|trait\b|extern\b)' -or
                $source -match '\b(asm|global_asm|llvm_asm)\s*!' -or
                $source -match '#!\[allow\(unsafe_code\)\]') {
                throw "Forbidden unsafe/assembly marker: $($file.Name)"
            }
        }
    }
    $raw = & cargo metadata --all-features --locked --offline --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Full Cargo metadata failed' }
    $graph = $raw | ConvertFrom-Json
    $approved = Get-Content docs/dependency-proposal/approved-packages.json -Raw | ConvertFrom-Json
    foreach ($package in $graph.packages | Where-Object source) {
        $identity = "$($package.name)@$($package.version)"
        if ($identity -notin $approved.packages) { throw "Dependency not approved: $identity" }
        $node = $graph.resolve.nodes | Where-Object id -eq $package.id
        $allowedFeatures = $approved.features.$identity
        foreach ($feature in $node.features) {
            if ($feature -notin $allowedFeatures) { throw "Unapproved dependency feature: $identity/$feature" }
        }
        if ($package.source -ne 'registry+https://github.com/rust-lang/crates.io-index') {
            throw "Non-registry dependency forbidden: $identity"
        }
    }
    if ($CompilerProbe) {
        $probeRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) ('central-guard-' + [IO.Path]::GetRandomFileName())))
        [IO.Directory]::CreateDirectory($probeRoot) | Out-Null
        try {
            $probeSource = Join-Path $probeRoot 'probe.rs'
            [IO.File]::WriteAllText($probeSource, "#![forbid(unsafe_code)]\nfn main() { unsafe {} }".Replace('\n', [Environment]::NewLine))
            $diagnostic = (& rustc $probeSource --out-dir $probeRoot 2>&1 | Out-String)
            if ($LASTEXITCODE -eq 0 -or $diagnostic -notmatch 'usage of an.*unsafe') {
                throw 'Compiler did not reject the deliberate unsafe probe'
            }
        } finally {
            $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
            if (-not $probeRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe cleanup path' }
            Remove-Item -LiteralPath $probeRoot -Recurse -Force
        }
    }
    Write-Output 'Policy checks passed (first-party code and approved package versions).'
} finally { Pop-Location }
exit 0
