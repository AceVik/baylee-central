$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
try {
    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Format failed' }
    & $PSScriptRoot/check-policy.ps1 -CompilerProbe
    & cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    & cargo test --locked --workspace --all-targets --all-features
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    & cargo test --locked --workspace --doc
    if ($LASTEXITCODE -ne 0) { throw 'Documentation tests failed' }
    & cargo build --locked --workspace --release
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
} finally { Pop-Location }
