$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$fixtureRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) ('central-policy-' + [IO.Path]::GetRandomFileName())))
try {
    $reviewed = Join-Path $fixtureRoot 'reviewed'
    foreach ($relative in @('scripts', 'crates/fixture/src', 'docs/dependency-proposal')) {
        [IO.Directory]::CreateDirectory((Join-Path $reviewed $relative)) | Out-Null
    }
    $checker = Join-Path $reviewed 'scripts/check-policy.ps1'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'check-policy.ps1') -Destination $checker
    [IO.File]::WriteAllText((Join-Path $reviewed 'docs/dependency-proposal/approved-packages.json'), '{"packages":[],"features":{}}')
    [IO.File]::WriteAllText((Join-Path $reviewed 'Cargo.toml'), @'
[workspace]
members = ["crates/fixture"]
resolver = "3"
[workspace.lints.rust]
unsafe_code = "forbid"
'@)
    $manifest = @'
[package]
name = "policy-fixture"
version = "0.1.0"
edition = "2024"
publish = false
[lints]
workspace = true
'@
    $manifestPath = Join-Path $reviewed 'crates/fixture/Cargo.toml'
    [IO.File]::WriteAllText($manifestPath, $manifest)
    [IO.File]::WriteAllText((Join-Path $reviewed 'crates/fixture/src/lib.rs'), "#![forbid(unsafe_code)]" + [Environment]::NewLine)
    function Update-FixtureLock {
        Push-Location $reviewed
        try {
            & cargo generate-lockfile --offline
            if ($LASTEXITCODE -ne 0) { throw 'Synthetic workspace fixture failed' }
        } finally { Pop-Location }
    }
    function Assert-PolicyRejects([string]$Expected) {
        try {
            & $checker
            throw 'Policy accepted a forbidden fixture'
        } catch {
            if ($_.Exception.Message -ne $Expected) { throw }
        }
    }
    Update-FixtureLock
    & $checker
    # Root/build-support files and hidden trees are as much first-party code as crates/.
    foreach ($case in @(
        @{ Path='escape.c'; Source='int native_escape(void) { return 0; }'; Error='Native/assembly source forbidden: escape.c' },
        @{ Path='tools/native/escape.hpp'; Source='void native_escape();'; Error='Native/assembly source forbidden: escape.hpp' },
        @{ Path='.hidden/escape.S'; Source='ret'; Error='Native/assembly source forbidden: escape.S' },
        @{ Path='tools/escape.rs'; Source='fn escape() { unsafe {} }'; Error='Forbidden unsafe/assembly marker: escape.rs' },
        @{ Path='crates/fixture/build.rs'; Source='fn main() {}'; Error='Missing crate-root unsafe prohibition: build-script-build' }
    )) {
        $path = Join-Path $reviewed $case.Path
        [IO.Directory]::CreateDirectory((Split-Path $path)) | Out-Null
        try {
            [IO.File]::WriteAllText($path, $case.Source)
            Assert-PolicyRejects $case.Error
        } finally { Remove-Item -LiteralPath $path -Force }
    }
    # A target cannot evade scanning by pointing into an excluded output directory.
    [IO.Directory]::CreateDirectory((Join-Path $reviewed 'target')) | Out-Null
    [IO.File]::WriteAllText((Join-Path $reviewed 'target/fixture.rs'), '#![forbid(unsafe_code)]')
    [IO.File]::WriteAllText($manifestPath, $manifest + [Environment]::NewLine + @'
[lib]
path = "../../target/fixture.rs"
'@)
    Assert-PolicyRejects 'Source target inside excluded output directory: policy_fixture'
    [IO.File]::WriteAllText($manifestPath, $manifest)
    # Directory junctions/symlinks must not import unchecked sources into the tree.
    $linked = Join-Path $fixtureRoot 'linked-source'
    [IO.Directory]::CreateDirectory($linked) | Out-Null
    $link = Join-Path $reviewed 'linked'
    $linkKind = if ($IsWindows) { 'Junction' } else { 'SymbolicLink' }
    New-Item -ItemType $linkKind -Path $link -Target $linked | Out-Null
    try { Assert-PolicyRejects 'Linked source outside the review boundary: linked' }
    finally { Remove-Item -LiteralPath $link -Force }
    # npm's install in web/ is skipped, links and all, but only there: a link
    # elsewhere under web/, a deeper node_modules, a link named
    # web/node_modules and a Cargo target inside it are all still refused.
    $web = Join-Path $reviewed 'web'
    $npmBin = Join-Path $web 'node_modules/.bin'
    [IO.Directory]::CreateDirectory($npmBin) | Out-Null
    $npmLink = Join-Path $npmBin 'tool'
    New-Item -ItemType $linkKind -Path $npmLink -Target $linked | Out-Null
    try {
        & $checker
        $webLink = Join-Path $web 'linked'
        New-Item -ItemType $linkKind -Path $webLink -Target $linked | Out-Null
        try { Assert-PolicyRejects 'Linked source outside the review boundary: linked' }
        finally { Remove-Item -LiteralPath $webLink -Force }
        $nested = Join-Path $web 'tools/node_modules'
        [IO.Directory]::CreateDirectory($nested) | Out-Null
        $nestedLink = Join-Path $nested 'linked'
        New-Item -ItemType $linkKind -Path $nestedLink -Target $linked | Out-Null
        try { Assert-PolicyRejects 'Linked source outside the review boundary: linked' }
        finally { Remove-Item -LiteralPath $nestedLink -Force }
        [IO.File]::WriteAllText((Join-Path $web 'node_modules/fixture.rs'), '#![forbid(unsafe_code)]')
        [IO.File]::WriteAllText($manifestPath, $manifest + [Environment]::NewLine + @'
[lib]
path = "../../web/node_modules/fixture.rs"
'@)
        try { Assert-PolicyRejects 'Source target inside excluded output directory: policy_fixture' }
        finally { [IO.File]::WriteAllText($manifestPath, $manifest) }
    } finally {
        Remove-Item -LiteralPath $npmLink -Force
        Remove-Item -LiteralPath $web -Recurse -Force
    }
    [IO.Directory]::CreateDirectory($web) | Out-Null
    $modulesLink = Join-Path $web 'node_modules'
    New-Item -ItemType $linkKind -Path $modulesLink -Target $linked | Out-Null
    try { Assert-PolicyRejects 'Linked source outside the review boundary: node_modules' }
    finally {
        Remove-Item -LiteralPath $modulesLink -Force
        Remove-Item -LiteralPath $web -Recurse -Force
    }
    # Cargo features are case-sensitive; PowerShell's default -in operator is not.
    # Use an already approved/cached crate to exercise the real registry graph.
    [IO.File]::WriteAllText($manifestPath, $manifest + [Environment]::NewLine + @'
[dependencies]
zeroize = { version = "=1.9.1", default-features = false, features = ["alloc"] }
'@)
    $inventory = Join-Path $reviewed 'docs/dependency-proposal/approved-packages.json'
    [IO.File]::WriteAllText($inventory, '{"packages":["zeroize@1.9.1"],"features":{"zeroize@1.9.1":["ALLOC"]}}')
    Update-FixtureLock
    Assert-PolicyRejects 'Unapproved dependency feature: zeroize@1.9.1/alloc'
    [IO.File]::WriteAllText($inventory, '{"packages":["zeroize@1.9.1"],"features":{"zeroize@1.9.1":["alloc"]}}')
    & $checker
    [IO.File]::WriteAllText($inventory, '{"packages":[],"features":{}}')
    $helper = Join-Path $fixtureRoot 'unreviewed-helper'
    [IO.Directory]::CreateDirectory((Join-Path $helper 'src')) | Out-Null
    [IO.File]::WriteAllText((Join-Path $helper 'Cargo.toml'), @'
[package]
name = "unreviewed-helper"
version = "0.1.0"
edition = "2024"
'@)
    [IO.File]::WriteAllText((Join-Path $helper 'src/lib.rs'), 'pub fn unreviewed() {}')
    [IO.File]::WriteAllText($manifestPath, $manifest + [Environment]::NewLine + @'
[dependencies]
unreviewed-helper = { path = "../../../unreviewed-helper" }
'@)
    # Resolve the local-only graph so --locked itself is not the rejection.
    Update-FixtureLock
    Assert-PolicyRejects 'Local dependency outside reviewed workspace: unreviewed-helper'
    if ($IsLinux) {
        # Linux supports two distinct package paths whose IDs differ only by
        # casing. PowerShell's default membership operator conflates those IDs.
        # macOS/Windows checkouts commonly use case-insensitive filesystems.
        $caseHelper = Join-Path $fixtureRoot 'REVIEWED/crates/fixture'
        [IO.Directory]::CreateDirectory((Join-Path $caseHelper 'src')) | Out-Null
        [IO.File]::WriteAllText((Join-Path $caseHelper 'Cargo.toml'), @'
[package]
name = "POLICY-FIXTURE"
version = "0.1.0"
edition = "2024"
'@)
        [IO.File]::WriteAllText((Join-Path $caseHelper 'src/lib.rs'), 'pub fn unreviewed() {}')
        [IO.File]::WriteAllText($manifestPath, $manifest + [Environment]::NewLine + @'
[dependencies]
unreviewed-case-helper = { package = "POLICY-FIXTURE", path = "../../../REVIEWED/crates/fixture" }
'@)
        Update-FixtureLock
        Push-Location $reviewed
        try {
            $caseMetadata = & cargo metadata --offline --locked --format-version 1
            if ($LASTEXITCODE -ne 0) { throw 'Synthetic case-collision graph failed' }
        } finally { Pop-Location }
        $caseGraph = $caseMetadata | ConvertFrom-Json
        $reviewedId = $caseGraph.workspace_members[0]
        $foreignId = ($caseGraph.packages | Where-Object { $_.name -ceq 'POLICY-FIXTURE' }).id
        if ($foreignId -ceq $reviewedId -or
            -not [string]::Equals($foreignId, $reviewedId, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Synthetic package IDs did not exercise a case collision'
        }
        Assert-PolicyRejects 'Local dependency outside reviewed workspace: POLICY-FIXTURE'
    }
    Write-Output 'Policy regressions passed: root/hidden/native sources, build scripts, excluded targets, links, web/node_modules, exact feature names and local dependencies.'
} finally {
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $fixtureRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe policy-fixture cleanup path' }
    if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
}
exit 0
