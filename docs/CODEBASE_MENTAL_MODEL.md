# Codebase Mental Model

Status: active orientation document  
Last updated: 2026-05-02

## Project Identity

**BUNKER MINER** is a mining management platform that securely configures, verifies, runs, monitors, and eventually optimizes third-party crypto miners for solo operators and fleet operators. Built primarily with Rust, Tokio, tonic gRPC, Axum, protobuf, and a quarantined C++/Qt desktop client.

Domain: cryptocurrency mining operations, local rig management, fleet management, hardware telemetry, and mining profitability.

Problem solved: operators need a trusted control plane around miner binaries, wallets, pools, hardware safety limits, telemetry, and eventually profitability decisions. The project is not product-grade until the tracker gates in `docs/PRODUCT_IMPLEMENTATION_TRACKER.md` are complete.

## Architecture Overview

```text
Operator
  |
  | CLI / future desktop UI / local dashboard
  v
tools/bunker-miner-cli --------> daemon gRPC API <-------- local dashboard
                                      |
                                      v
                         daemon process supervisor
                                      |
                 verified third-party miner executable
                                      |
                                      v
                         pool endpoint + wallet config

daemon also owns:
  encrypted config
  hardware discovery / telemetry
  miner adapter selection
  manifest and checksum trust
  profile, benchmark, profit, safety engines

quarantined future tiers:
  fleet/        remote fleet controller
  pool/         mining pool service
  tools/poc/    proof-of-concept tooling
```

## Directory Guide

| Directory | Purpose | Key Files |
| --- | --- | --- |
| `daemon/` | Core Rust daemon: gRPC API, config, miner lifecycle, hardware, telemetry, safety/profit engines | `src/main.rs`, `src/grpc.rs`, `src/miners.rs`, `src/config.rs`, `src/miner_manifest.rs` |
| `tools/bunker-miner-cli/` | Enforced CLI client for daemon gRPC operations | `src/main.rs`, `Cargo.toml` |
| `libs/common-rust/` | Shared Rust domain types used by promoted workspace crates | `src/lib.rs` |
| `protos/` | Daemon API contract and generated Rust source inputs | `daemon_api.v1.proto` |
| `infra/stubs/` | Enforced local development stubs for pool, fleet, and coin daemon APIs | `*/src/main.rs` |
| `docs/` | Product specification, build baseline, ADRs, progress logs, implementation tracking | `PRODUCT_SPEC.md`, `PRODUCT_IMPLEMENTATION_TRACKER.md`, `BUILD_BASELINE.md` |
| `docs/specs/` | Focused engineering specs for active release targets | `LOCAL_MINER_MVP.md` |
| `client/` | C++/Qt desktop client area; not currently part of enforced Rust gates | `src/`, `ui/` |
| `fleet/` | Future fleet controller crate, currently quarantined outside root workspace | `Cargo.toml`, `src/` |
| `pool/` | Future mining pool crate, currently quarantined outside root workspace | `Cargo.toml`, `src/` |
| `scripts/` | Development and operational scripts | repo-specific helper scripts |
| `tests/` | Higher-level test assets and harness area | integration/support files |

## Structure Pattern

- Monorepo-style repository with a root Rust workspace.
- Layered daemon architecture: API layer (`grpc.rs`, `web_dashboard.rs`), domain/service modules (`miners.rs`, `hardware.rs`, `config.rs`), engines (`profit_engine.rs`, `benchmarking.rs`, `overclocking.rs`, `power_tuning.rs`), and shared API schema (`protos/`).
- Adapter pattern for third-party miners through `MinerAdapter`.
- File-based API generation from `protos/daemon_api.v1.proto`.
- Quarantine pattern for crates that are not ready to support product claims.

## Entry Points

| Runtime | Entry Point | Notes |
| --- | --- | --- |
| Daemon binary | `daemon/src/main.rs` | Parses daemon commands, service startup, setup helpers, and legacy command paths. |
| Daemon gRPC service | `daemon/src/grpc.rs` | Implements `BunkerMinerDaemon`, including `StartMining`, `StopMining`, telemetry, config, health, overclock APIs. |
| CLI | `tools/bunker-miner-cli/src/main.rs` | Clap command surface that calls daemon gRPC. |
| Local dashboard | `daemon/src/web_dashboard.rs` | Axum route setup; not yet a complete product workflow. |
| Protocol contract | `protos/daemon_api.v1.proto` | Source contract for generated daemon/CLI API types. |

## Core Concepts

| Concept | Location | Description |
| --- | --- | --- |
| Enforced workspace | `Cargo.toml`, `docs/BUILD_BASELINE.md` | Crates that must pass fmt, check, test, and clippy with warnings denied. |
| Product tracker | `docs/PRODUCT_IMPLEMENTATION_TRACKER.md` | Release gates and implementation status by stable ID. |
| Local Miner MVP | `docs/specs/LOCAL_MINER_MVP.md` | First product-grade release target: verified miner launch, telemetry, stop, and clear failure states. |
| Config | `daemon/src/config.rs` | Encrypted daemon configuration, wallet/pool config, security settings, validation. |
| Miner adapter | `daemon/src/miners.rs` | Supported miner identity, argument construction, binary verification, telemetry parsing. |
| Miner manifest | `daemon/src/miner_manifest.rs` | Validated TOML trust source for miner archive and executable checksums. |
| Miner installer | `daemon/src/miner_installer.rs` | Explicit manifest-backed acquisition path with archive hash verification, constrained zip extraction, executable hash verification, and managed install. |
| Process supervisor | `daemon/src/miners.rs` | Starts miner process without shell interpolation, parses output, handles stop/restart state. |
| Hardware detector | `daemon/src/hardware.rs` | Discovers devices and exposes metrics/permission status. |
| Telemetry | `daemon/src/telemetry.rs`, `daemon/src/miners.rs`, `protos/daemon_api.v1.proto` | Mining and device state flowing from daemon to CLI/UI/API consumers. |
| Profit engine | `daemon/src/profit_engine.rs` | Profitability data and switching decision logic; not yet release-grade. |
| Quarantined crates | `fleet/`, `pool/`, `tools/poc/` | Existing code not allowed to support product claims until promoted into the enforced workspace. |

## Key Files Reference

| Task | Look Here |
| --- | --- |
| Understand product target | `docs/PRODUCT_SPEC.md` |
| See current implementation status | `docs/PRODUCT_IMPLEMENTATION_TRACKER.md` |
| Work on Local Miner MVP | `docs/specs/LOCAL_MINER_MVP.md` |
| Verify build/test policy | `docs/BUILD_BASELINE.md` |
| Add/change daemon API | `protos/daemon_api.v1.proto`, `daemon/src/grpc.rs`, CLI generated bindings |
| Add CLI command | `tools/bunker-miner-cli/src/main.rs` |
| Add miner adapter | `daemon/src/miners.rs` |
| Change miner trust model | `daemon/src/miner_manifest.rs`, `daemon/src/miner_installer.rs`, `daemon/src/miners.rs` |
| Change config model | `daemon/src/config.rs` |
| Change hardware detection | `daemon/src/hardware.rs` |
| Change telemetry stream | `daemon/src/grpc.rs`, `daemon/src/telemetry.rs`, `daemon/src/miners.rs` |
| Add UI route | `daemon/src/web_dashboard.rs` |
| Promote quarantined crate | `Cargo.toml`, target crate `Cargo.toml`, `docs/BUILD_BASELINE.md` |

## Conventions Detected

- File naming: Rust modules use `snake_case.rs`; docs use uppercase canonical files for major product docs and kebab/underscore variants in historical docs.
- Code style: Rust 2021, `cargo fmt`, `clippy -D warnings`, strict root workspace gates.
- Error handling: `anyhow::Result` in daemon/CLI service code, typed daemon command responses at gRPC boundary.
- Async/runtime: Tokio across daemon, CLI, and stubs.
- API style: protobuf-first gRPC contract with generated Rust modules.
- Configuration: environment variables use `BUNKER_MINER_*`; config storage defaults to OS config dir with `BUNKER_MINER_CONFIG_DIR` override.
- Security posture: fail closed for miner execution, encrypted config at rest, no unverified miner binaries for release.
- Tests: Rust unit/integration tests are colocated in modules and crate `tests/` folders; enforced through root `cargo test --workspace`.

## Current State

Maturity: Level 1 developer-usable for promoted Rust workspace; moving toward Level 2 Local Miner MVP. It is not product-ready.

Active product path:

- Finish Local Miner MVP before fleet/pool expansion.
- Keep `fleet/`, `pool/`, and `tools/poc/` quarantined until each passes promotion criteria.
- Continue moving tracker IDs from `planned` to `active` to `done` only with code, tests, and docs.

Product blockers:

- No live XMRig pool/session/share validation with an approved pool and wallet.
- No curated production manifest/release evidence for supported miner archives.
- Mining state is not persisted and reconciled after daemon restart.
- Local dashboard is not yet a complete operator workflow.
- CI parity with local gates still needs confirmation.
- Hardware safety and telemetry need release-grade validation.

Enforced local gate:

```powershell
$env:RUSTFLAGS='-Dwarnings'
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
