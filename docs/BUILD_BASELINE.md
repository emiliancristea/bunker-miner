# Build Baseline

Status: active baseline  
Last updated: 2026-05-01

This document records what is currently enforced by the root Rust workspace. It exists to prevent the project from confusing aspirational components with release-ready components.

## Enforced Workspace

The root `Cargo.toml` currently gates:

- `daemon`
- `libs/common-rust`
- `infra/stubs/coin-daemon-stub`
- `infra/stubs/fleet-controller-stub`
- `infra/stubs/pool-api-stub`

Required local gate:

```powershell
$env:RUSTFLAGS='-Dwarnings'
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Quarantined Crates

The following crates are intentionally excluded from the root workspace until their compile blockers and active placeholder paths are resolved:

- `fleet`
- `pool`
- `tools/bunker-miner-cli`
- `tools/poc`

Quarantine is not a success state. Each quarantined crate must move into the root workspace before it can support product or release claims.

## Entry Criteria for Workspace Promotion

A quarantined crate can be promoted into the root workspace when:

- `cargo check --manifest-path <crate>/Cargo.toml` passes.
- `cargo test --manifest-path <crate>/Cargo.toml` passes or has documented, intentionally skipped tests.
- It has no warnings under `RUSTFLAGS=-Dwarnings`.
- It does not require undeclared local tools to compile.
- Generated sources are deterministic or generated under `OUT_DIR`.
- Any remaining placeholder behavior is isolated behind explicit experimental flags or returns typed unimplemented errors.

## Current Next Promotions

1. `tools/bunker-miner-cli`: depends on daemon proto compatibility and should follow daemon API stabilization.
2. `fleet`: beta feature after local daemon MVP.
3. `pool`: separate product tier after mining management foundations are stable.

## Daemon Promotion Notes

The daemon is now part of the enforced workspace. The promoted baseline includes real process lifecycle wiring for `StartMining`/`StopMining`, protobuf API alignment, warning-free strict checks, and daemon integration tests.

Automatic third-party miner downloads remain disabled until checksum verification and archive extraction are implemented. If a miner binary is missing, daemon startup fails with an explicit installation error instead of reporting fake success.
