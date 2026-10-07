$ErrorActionPreference = 'Stop'

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo fmt --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo clippy --all-targets --all-features --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo test --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    if ($env:DION_UNC_ROOT) {
        cargo test --locked --test unc -- --ignored
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } else {
        Write-Host 'UNVERIFIED: real UNC tests require DION_UNC_ROOT (see docs/unc-validation.md).'
    }

    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
