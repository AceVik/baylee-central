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
    # A valid workspace must pass before the unreviewed path dependency is introduced.
    Push-Location $reviewed
    try {
        & cargo generate-lockfile --offline
        if ($LASTEXITCODE -ne 0) { throw 'Synthetic workspace fixture failed' }
    } finally { Pop-Location }
    & $checker
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
    # Resolve the synthetic local-only graph so --locked itself is not the rejection.
    Push-Location $reviewed
    try {
        & cargo generate-lockfile --offline
        if ($LASTEXITCODE -ne 0) { throw 'Synthetic dependency fixture failed' }
    } finally { Pop-Location }
    try {
        & $checker
        throw 'Policy accepted an unreviewed local dependency'
    } catch {
        if ($_.Exception.Message -ne 'Local dependency outside reviewed workspace: unreviewed-helper') { throw }
    }
    Write-Output 'Policy bypass regression passed.'
} finally {
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $fixtureRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe policy-fixture cleanup path' }
    if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
}
exit 0
