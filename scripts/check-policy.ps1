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
    $workspaceRoot = [IO.Path]::GetFullPath($meta.workspace_root).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    # Tool output, never reviewed source: Git's store, Cargo's target, and the
    # npm installs at the root and in web/ (`npm ci` fills web/node_modules from
    # the reviewed web/package-lock.json; its .bin holds symbolic links). No
    # Cargo target may live in any of them.
    $excludedRoots = @('.git', 'target', 'node_modules', 'web/node_modules') | ForEach-Object {
        $workspaceRoot + $_.Replace('/', [string][IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    }
    foreach ($package in $meta.packages) {
        $manifest = Get-Content -LiteralPath $package.manifest_path -Raw
        if ($manifest -notmatch '(?ms)\[lints\]\s*workspace\s*=\s*true') {
            throw "Missing inherited lints: $($package.name)"
        }
        # `publish = false` is an empty list; an absent `publish` (null) allows crates.io.
        if ($null -eq $package.publish -or $package.publish.Count -ne 0) {
            throw "Package publication must be disabled: $($package.name)"
        }
        if (-not [IO.Path]::GetFullPath($package.manifest_path).StartsWith($workspaceRoot, [StringComparison]::Ordinal)) {
            throw "Workspace member outside reviewed root: $($package.name)"
        }
        foreach ($excluded in $excludedRoots) {
            if ([IO.Path]::GetFullPath($package.manifest_path).StartsWith($excluded, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Workspace member inside excluded output directory: $($package.name)"
            }
        }
        foreach ($target in $package.targets) {
            foreach ($excluded in $excludedRoots) {
                if ([IO.Path]::GetFullPath($target.src_path).StartsWith($excluded, [StringComparison]::OrdinalIgnoreCase)) {
                    throw "Source target inside excluded output directory: $($target.name)"
                }
            }
            if (-not [IO.Path]::GetFullPath($target.src_path).StartsWith($workspaceRoot, [StringComparison]::Ordinal)) {
                throw "Source target outside reviewed root: $($target.name)"
            }
            $source = Get-Content -LiteralPath $target.src_path -Raw
            if ($source -notmatch '(?m)^#!\[forbid\(unsafe_code\)\]\s*$') {
                throw "Missing crate-root unsafe prohibition: $($target.name)"
            }
        }
    }
    # Targets/build scripts may live outside crates/. Scan the whole reviewed
    # tree, skipping only tool/output directories that cannot contain targets.
    $webRoot = Join-Path $meta.workspace_root 'web'
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($meta.workspace_root)
    $reviewedFiles = [Collections.Generic.List[IO.FileInfo]]::new()
    while ($pending.Count -gt 0) {
        $directory = $pending.Pop()
        foreach ($entry in Get-ChildItem -LiteralPath $directory -Force) {
            if ($directory -eq $meta.workspace_root -and $entry.Name -in @('.git', 'target', 'node_modules')) { continue }
            if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Linked source outside the review boundary: $($entry.Name)"
            }
            # Only web/node_modules itself, only as a real directory (a link of
            # that name is refused above), exactly cased; node_modules anywhere
            # else under web/ is walked like any other directory.
            if ($directory -eq $webRoot -and $entry.Name -ceq 'node_modules' -and $entry.PSIsContainer) { continue }
            if ($entry.PSIsContainer) { $pending.Push($entry.FullName) }
            else { $reviewedFiles.Add($entry) }
        }
    }
    foreach ($file in $reviewedFiles) {
        if ($file.Extension -in @('.s', '.asm', '.c', '.cpp', '.cc', '.cxx', '.h', '.hh', '.hpp', '.hxx')) {
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
    $approvedPackages = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($identity in $approved.packages) { [void]$approvedPackages.Add($identity) }
    $nodesById = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    foreach ($node in $graph.resolve.nodes) { $nodesById.Add($node.id, $node) }
    # Path dependencies are first-party code too. Never let a helper outside the
    # checked workspace bypass lint/root/native-source checks or the inventory.
    foreach ($package in $graph.packages | Where-Object { -not $_.source }) {
        if ($package.id -cnotin $graph.workspace_members) {
            throw "Local dependency outside reviewed workspace: $($package.name)"
        }
    }
    foreach ($package in $graph.packages | Where-Object source) {
        $identity = "$($package.name)@$($package.version)"
        if (-not $approvedPackages.Contains($identity)) { throw "Dependency not approved: $identity" }
        $node = $nodesById[$package.id]
        $allowedFeatures = $approved.features.$identity
        foreach ($feature in $node.features) {
            if ($feature -cnotin $allowedFeatures) { throw "Unapproved dependency feature: $identity/$feature" }
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
            $probes = @(
                '#![forbid(unsafe_code)]\nfn main() { unsafe {} }',
                '#![forbid(unsafe_code)]\nmacro_rules! generated { () => { unsafe {} } }\nfn main() { generated!(); }',
                '#![forbid(unsafe_code)]\ninclude!("generated.rs");\nfn main() { generated(); }'
            )
            [IO.File]::WriteAllText((Join-Path $probeRoot 'generated.rs'), 'fn generated() { unsafe {} }')
            foreach ($probe in $probes) {
                [IO.File]::WriteAllText($probeSource, $probe.Replace('\n', [Environment]::NewLine))
                $diagnostic = (& rustc $probeSource --out-dir $probeRoot 2>&1 | Out-String)
                if ($LASTEXITCODE -eq 0 -or $diagnostic -notmatch 'usage of an.*unsafe') {
                    throw 'Compiler did not reject the direct/generated unsafe probe'
                }
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
