# BUNKER MINER Product Specification

Status: Canonical product target  
Version: 1.0  
Last updated: 2026-05-02

## 1. Purpose

BUNKER MINER is a secure mining management platform for operators who need local rig control, hardware telemetry, profitability-aware miner orchestration, and optional fleet management. The product manages third-party mining software; it is not itself a proof-of-work implementation unless a future adapter explicitly embeds one.

This specification is the source of truth for bringing the repository from prototype state to product-grade state. Historical phase reports and roadmap documents are informative only; a feature is product-ready only when it satisfies the acceptance gates in this document.

Execution tracking lives in [PRODUCT_IMPLEMENTATION_TRACKER.md](PRODUCT_IMPLEMENTATION_TRACKER.md). Focused engineering requirements for the first release target live in [specs/LOCAL_MINER_MVP.md](specs/LOCAL_MINER_MVP.md). Codebase orientation lives in [CODEBASE_MENTAL_MODEL.md](CODEBASE_MENTAL_MODEL.md).

## 2. Product Definition

### 2.1 One-Sentence Product

BUNKER MINER detects mining hardware, securely configures and runs vetted third-party miners, monitors hashrate/power/temperature, and optimizes mining decisions across local and fleet-managed rigs.

### 2.2 Target Users

- Solo miner: runs one workstation or small rig and wants safe defaults, simple setup, and local control.
- Fleet operator: manages many rigs and needs remote configuration, monitoring, alerts, and audit trails.
- Pool operator: operates or integrates with a mining pool and needs validated Stratum/share accounting.
- Developer/operator: extends miner adapters, pool integrations, and deployment infrastructure.

### 2.3 Product Principles

- Local mining must work without fleet or cloud services.
- Users keep control over wallets, pools, hardware limits, and telemetry sharing.
- Security-sensitive features must fail closed.
- Profitability logic must be explainable and reproducible.
- Product claims must be backed by automated tests and documented evidence.

## 3. Product Scope

### 3.1 In Scope

- Local daemon for hardware detection, miner lifecycle management, telemetry, configuration, benchmarking, and API access.
- Desktop client and local web UI for setup, monitoring, and manual control.
- Miner adapter framework for vetted third-party miners.
- Secure miner binary acquisition, verification, storage, and execution.
- Profitability engine using measured device performance, market data, pool data, fees, and power cost.
- Fleet controller for authenticated remote management, policy sync, telemetry ingestion, and command audit.
- Pool server only after daemon and fleet foundations are stable.
- Release pipeline, installers, upgrade path, logs, metrics, security scans, and rollback procedures.

### 3.2 Out of Scope Until Explicitly Approved

- Mobile apps.
- AI-branded optimization claims that cannot be audited.
- Automatic overclocking that changes unsafe hardware limits without explicit user opt-in.
- Custodial wallet management for users.
- Production pool payouts before share validation, accounting, hot wallet controls, and audit trails are independently reviewed.
- Mining malware-style stealth, persistence, or hidden execution behavior.

## 4. Product Maturity Levels

### Level 0: Prototype

Code may be incomplete, mocked, or non-compiling. Documentation may describe intent. This is the historical prototype state.

### Level 1: Developer-Usable

- All workspaces build locally and in CI.
- Unit tests cover core pure logic.
- Local daemon can detect hardware with mocks or real devices.
- No production claims.

Current status: the enforced Rust workspace has reached Level 1 for the promoted daemon, CLI, and stubs. The daemon and CLI are not yet Level 2 because packaged setup and end-user start-to-telemetry validation with a verified real miner binary are still incomplete.

### Level 2: Local Miner MVP

- A user can install the daemon, configure a wallet/pool, run one supported miner, see telemetry, stop mining, and recover from miner crashes.
- Miner binaries are verified before execution.
- Local UI/CLI reflects real daemon state.
- Security defaults are acceptable for local-only operation.

Current miner binary policy: daemon execution is fail-closed. A miner executable must be discovered from the managed binary directory, `BUNKER_MINERS_PATH`, `BUNKER_MINER_<MINER>_PATH`, or `PATH`, and it must have a trusted SHA-256 from a sidecar `.sha256` file, `BUNKER_MINER_<MINER>_SHA256`, `BUNKER_MINER_MANIFEST_PATH`, or managed `miner-manifest.toml`. `BUNKER_MINER_ALLOW_UNVERIFIED_MINERS=1` is a development-only escape hatch and is not acceptable for release builds.

Current local service policy: daemon configuration can be bootstrapped non-interactively with `BUNKER_MINER_CONFIG_DIR` plus either `BUNKER_MINER_CONFIG_PASSWORD` or `BUNKER_MINER_CONFIG_PASSWORD_FILE`. Configuration templates may contain placeholder wallets, but mining operations must reject placeholder wallets and unavailable/untrusted miner binaries.

### Level 3: Product Beta

- Two or more miner adapters are production-quality.
- NVIDIA and CPU telemetry are validated; AMD support is either validated or clearly unavailable.
- Benchmarks persist and feed profitability decisions.
- Automatic switching has hysteresis, rollback, and user-visible reasoning.
- Fleet mode supports authenticated enrollment, policy sync, remote commands, and audit logs.

### Level 4: Production Release

- Signed installers/releases exist for supported platforms.
- CI/CD blocks release on compile, tests, lint, audit, and packaging failures.
- TLS and auth are enforced for remote access.
- Documentation reflects actual behavior.
- Observability, incident response, and rollback procedures are tested.

### Level 5: Industry-Leading

- Real-world efficiency gains are measured against baseline miners on representative hardware.
- Optimization decisions are statistically defensible and reproducible.
- Fleet management scales under load tests.
- Security model has external review or equivalent independent assessment.
- Pool functionality, if shipped, has validated share accounting and payout controls.

## 5. System Architecture

```text
Desktop Client / CLI / Local Web UI
        |
        v
Daemon API: gRPC + optional local HTTP dashboard
        |
        +-- Config service: encrypted local config, wallet/pool settings
        +-- Hardware service: device discovery, telemetry, capability model
        +-- Miner service: adapter registry, verified binaries, process lifecycle
        +-- Benchmark service: repeatable hashrate/power measurements
        +-- Profit service: profitability model, switching decisions, explanations
        +-- Safety service: thermal/power limits, kill switches, rollback
        |
        v
Third-party miner processes and hardware vendor APIs

Optional fleet mode:

Daemon agent <----TLS/auth/WebSocket or gRPC----> Fleet Controller
                                                |
                                                +-- Postgres
                                                +-- Redis
                                                +-- Web admin/API

Optional pool mode:

External miners ---> Stratum Server ---> Share Processor ---> Accounting/Payout
```

## 6. Core Components

### 6.1 Daemon

The daemon is the primary product surface. It owns local state, miner process control, hardware telemetry, and local APIs.

Required capabilities:

- Detect supported devices and expose stable device IDs.
- Validate configuration before writing or applying it.
- Encrypt secrets at rest.
- Start, stop, restart, and supervise miner processes.
- Parse miner output into normalized hashrate, accepted shares, rejected shares, pool status, and errors.
- Expose real state through gRPC/CLI/UI, never placeholder success.
- Apply safety limits before any optimization action.
- Record audit events for user-visible changes.

Release blockers:

- No daemon API method may return a hardcoded success for an unimplemented action.
- Miner process state must be authoritative and recoverable after daemon restart.
- All process execution paths must use validated adapter output, never untrusted shell strings.

### 6.2 Miner Adapter Framework

Each miner adapter must define:

- Miner identity, version, license, official source URL, and supported platforms.
- Supported algorithms and device classes.
- Required binary artifacts with cryptographic checksums and signatures where available.
- Command-line argument builder from validated config.
- Output parser with fixture-based tests.
- Health checks and crash classification.
- Graceful shutdown behavior.
- Upgrade and rollback behavior.

Minimum product adapters:

- XMRig for RandomX CPU mining.
- One NVIDIA GPU miner for a currently viable algorithm.
- AMD GPU support only if detection, telemetry, and adapter behavior are validated.

### 6.3 Hardware and Telemetry

The hardware model must separate device identity, capabilities, live telemetry, and user-applied policy.

Required telemetry:

- Device name, vendor, driver/runtime version, memory size, and bus identity where available.
- Temperature, fan speed, power draw, clocks, utilization, and memory usage where supported.
- Capability flags for overclocking, power limit changes, and fan control.
- Error state when a metric cannot be read.

Acceptance criteria:

- NVIDIA path validated through NVML.
- CPU path validated through system APIs.
- AMD path either implemented and tested or explicitly disabled in product surfaces.
- Telemetry refresh must not block miner supervision or API responsiveness.

### 6.4 Configuration and Secrets

Configuration must be durable, validated, auditable, and recoverable.

Requirements:

- Wallet addresses and pool credentials are treated as secrets when appropriate.
- Config writes are atomic.
- Invalid config cannot partially apply.
- Defaults must not contain fake production wallets or unsafe endpoints.
- Import/export must redact secrets by default.
- Configuration migrations must be versioned.

### 6.5 Benchmarking

Benchmarking is the foundation for efficiency claims.

Requirements:

- Benchmarks run against actual miner adapters.
- Results include hashrate, power draw, temperature, rejected shares, runtime, device ID, driver version, miner version, algorithm, and pool/test endpoint.
- Benchmarks have warmup, measurement, and cooldown phases.
- Results are persisted with enough metadata to compare over time.
- Profitability decisions must cite benchmark inputs used.

### 6.6 Profit and Switching Engine

The profit engine recommends or applies mining choices. It must be conservative and explainable.

Inputs:

- Measured device hashrate and power draw.
- Electricity cost.
- Pool fees and payout method.
- Coin price, network difficulty, block reward, and stale/reject rate.
- Miner fees and switching overhead.

Rules:

- Automatic switching requires user opt-in.
- Switching must use hysteresis to prevent flapping.
- Switching must include cooldowns and rollback when hashrate or stability regresses.
- Every recommendation must expose a plain-language explanation and numeric assumptions.
- Missing or stale market data must degrade to hold-current, not random switching.

### 6.7 Overclocking and Power Tuning

This feature is safety-critical.

Requirements:

- Disabled by default.
- User must explicitly opt in per device or policy group.
- Hardware capabilities and vendor limits must be detected before applying settings.
- Settings must be bounded, reversible, and audited.
- Emergency shutdown triggers must exist for temperature, fan failure, runaway power, and repeated miner crashes.
- Unsupported platforms must show unavailable, not pretend success.

### 6.8 Desktop Client and Local Web UI

The UI must be a real operator surface, not a status mock.

Required workflows:

- First-run setup.
- Wallet and pool configuration.
- Hardware inventory and telemetry.
- Start/stop/restart mining.
- Miner logs and errors.
- Benchmark run and results.
- Profit recommendations and switching controls.
- Safety settings and emergency stop.

UI acceptance criteria:

- UI state must come from daemon APIs.
- Actions must show pending/success/failure states from real responses.
- No success toast may be emitted for TODO behavior.
- Destructive or safety-critical actions require confirmation.

### 6.9 Fleet Controller

Fleet mode is optional but must be secure when enabled.

Required capabilities:

- Device enrollment using short-lived tokens or signed enrollment flow.
- Mutual authentication or equivalent strong daemon identity.
- TLS enforced for remote control.
- Role-based access control.
- Policy assignment and versioned config sync.
- Remote command queue with idempotency and audit trail.
- Telemetry ingestion with rate limits and retention policy.
- Clear offline/stale device state.

Release blockers:

- No default admin password in migrations or deploy manifests.
- No checked-in production secrets.
- CORS must be explicit and environment-specific.
- SQLx/offline query validation must be reliable in CI.

### 6.10 Pool Server

The pool is a separate product tier and must not block the local miner MVP.

Required before production:

- Correct Stratum implementation for each supported algorithm.
- Cryptographic share validation using canonical block/job data.
- Duplicate share detection.
- Worker authentication policy.
- Difficulty adjustment.
- Accounting model with reconciliation.
- Payout engine with manual approval limits.
- Hot wallet isolation and monitoring.
- Public pool stats that reconcile with stored shares and blocks.

## 7. Security Requirements

### 7.1 Threat Model Surfaces

- Third-party miner binary download and execution.
- Wallet addresses, pool credentials, and fleet enrollment tokens.
- Remote daemon control.
- Local API access.
- Miner output parsing.
- Configuration import/export.
- Pool share submissions and payout flows.

### 7.2 Mandatory Controls

- TLS for all non-localhost remote control channels.
- Least-privilege process execution.
- Cryptographic verification of miner binaries.
- No shell interpolation for miner commands.
- Redacted logs for secrets.
- Rate limiting on network APIs.
- Input validation on every external boundary.
- Dependency vulnerability scanning in CI.
- Security-sensitive changes require threat model updates.

### 7.3 Prohibited Release Conditions

- Hardcoded production secrets.
- Default admin credentials.
- TODO security controls on active execution paths.
- Unverified binary downloads.
- Remote command execution without auth, audit, and TLS.
- Product docs claiming secure production behavior that is not implemented.

## 8. Reliability and Recovery Requirements

- Miner crashes must be detected and classified.
- Restart policy must use backoff and max retry limits.
- Daemon restart must reconcile actual miner process state.
- Config corruption must fail to last known good config or setup mode.
- Telemetry failures must not crash mining supervision.
- Fleet disconnect must not stop local mining unless policy says so.
- Logs must be structured enough for support and incident response.

## 9. Performance Requirements

Local daemon:

- Idle CPU overhead below 2% on supported baseline hardware.
- Memory footprint target below 250 MB excluding miner processes.
- API status response p95 below 100 ms on local machine.
- Telemetry refresh does not block control-plane commands.

Mining efficiency:

- Benchmark variance tracked across repeated runs.
- Optimization must demonstrate net gain after power cost and switching overhead.
- Any "efficiency improvement" claim must include baseline, hardware, driver, miner version, algorithm, sample size, and measurement method.

Fleet:

- Support at least 1,000 enrolled devices in beta load tests before production fleet claims.
- Telemetry ingestion and command dispatch must have backpressure.

Pool:

- Pool performance targets are not product claims until the Stratum server and share processor pass correctness tests.

## 10. Observability

Required outputs:

- Structured logs with component, device ID, miner ID, event type, severity, and correlation ID where applicable.
- Local status API for current mining state.
- Metrics for process uptime, crashes, hashrate, shares, reject rate, telemetry freshness, API latency, and config changes.
- Audit log for start/stop, config writes, remote commands, optimizer actions, and safety shutdowns.
- Support bundle export with secret redaction.

## 11. Data Model Requirements

Core entities:

- Device: stable hardware identity and capabilities.
- MinerBinary: source, version, platform, checksum, verification status.
- MinerInstance: process identity, adapter, config, state, health.
- PoolConfig: URL, algorithm, wallet, worker, fee metadata, failover priority.
- BenchmarkRun: measured performance with metadata.
- ProfitDecision: inputs, output, confidence, action taken.
- SafetyPolicy: thermal/power/fan/crash limits.
- FleetAgent: enrolled daemon identity, policy, telemetry state.
- AuditEvent: who/what/when/where/result.

Every entity that affects mining behavior must be serializable, versioned, and migration-safe.

## 12. API Requirements

Daemon API must support:

- Health and version.
- Device inventory and telemetry stream.
- Config get/validate/set.
- Miner binary list/install/verify.
- Start/stop/restart mining.
- Miner logs/status.
- Benchmark start/status/results.
- Profit recommendations and switching policy.
- Safety policy get/set.
- Audit event retrieval.

API rules:

- Mutating calls must be idempotent where practical.
- Errors must be typed and actionable.
- Unimplemented methods must return explicit unimplemented errors, not success.
- API compatibility must be tracked through protobuf versioning.

## 13. Testing Strategy and Release Gates

### 13.1 Required Test Layers

- Unit tests for config validation, parsing, profitability math, and safety rules.
- Fixture tests for miner output parsers.
- Integration tests for daemon API, miner lifecycle, and config persistence.
- Hardware abstraction tests using mocks.
- Real hardware validation on supported device matrix before production claims.
- Security tests for input validation, auth, TLS, secret redaction, and binary verification.
- E2E tests for first-run setup, start mining, stop mining, crash recovery, and benchmark workflow.

### 13.2 CI Gates

Every pull request to a release branch must pass:

- Root workspace `cargo fmt --check`.
- Root workspace `cargo clippy --all-targets --all-features -- -D warnings`.
- Root workspace `cargo test --all`.
- C++ client configure/build/test where enabled.
- Protobuf generation check.
- Dependency audit.
- Secret scan.
- License check for bundled or downloaded miners.
- Container build and scan for deployable services.

No required gate may use `continue-on-error` on release branches.

### 13.3 Release Evidence

Every release must include:

- Build artifacts and checksums.
- Supported platform matrix.
- Known limitations.
- Test summary.
- Security scan summary.
- Upgrade and rollback notes.
- Documentation version aligned to the release.

## 14. Documentation Requirements

Documentation must be separated by audience:

- User guide: install, setup, configure, mine, troubleshoot.
- Operator guide: fleet deployment, monitoring, incident response.
- Developer guide: architecture, adapters, APIs, testing.
- Security guide: threat model, controls, reporting.
- Release notes: actual changes and known issues.

Documentation rules:

- No future-tense feature may be presented as implemented.
- Historical phase documents must not override current product status.
- README status badges and feature tables must reflect tested reality.

## 15. Productization Roadmap

### Phase A: Build and Truth Baseline

Goals:

- Add root Cargo workspace.
- Make daemon, pool, fleet, common libs, and tools compile or explicitly exclude incomplete crates.
- Install/document protobuf toolchain.
- Remove or quarantine invalid tests.
- Make CI hard-fail on compile, fmt, clippy, and unit tests.
- Update README status to prototype until gates pass.

Exit criteria:

- Clean `cargo check --workspace`.
- CI mirrors local build.
- No docs claim production readiness.

### Phase B: Local Daemon MVP

Goals:

- Implement real `start_mining`, `stop_mining`, status, and config update API paths.
- Implement one production-quality miner adapter.
- Implement secure miner binary verification.
- Persist config and process state.
- Implement real CLI start/stop/status against daemon.
- Add fixture tests for miner output parsing.

Exit criteria:

- Fresh install can configure and run one supported miner.
- Daemon can stop and recover miner state.
- UI/CLI shows real state.

### Phase C: Telemetry, Benchmarking, and Safety

Goals:

- Stabilize device model.
- Validate NVIDIA and CPU telemetry.
- Disable unsupported AMD/OC paths until real.
- Implement benchmark lifecycle.
- Implement safety policy and emergency stop.

Exit criteria:

- Benchmark data is persisted and visible.
- Safety triggers are tested.
- Unsupported controls cannot fake success.

### Phase D: Profit Optimization

Goals:

- Replace hardcoded profitability assumptions.
- Implement market/pool data freshness tracking.
- Add switching hysteresis and rollback.
- Show decision explanations.

Exit criteria:

- Profit recommendations are reproducible from stored inputs.
- Auto-switching is opt-in and can be audited.

### Phase E: Fleet Beta

Goals:

- Fix fleet build and SQLx validation.
- Remove default credentials and checked-in secrets.
- Enforce TLS/auth.
- Implement enrollment, policy sync, telemetry ingestion, and command audit.

Exit criteria:

- Multi-rig beta test with authenticated remote commands.
- Fleet disconnect behavior is safe and documented.

### Phase F: Production Release

Goals:

- Signed installers.
- Upgrade/rollback.
- Support bundles.
- Release evidence.
- External security review or equivalent independent assessment.

Exit criteria:

- Product can be installed, operated, upgraded, and supported by non-developers.
- Claims match measured behavior.

### Phase G: Pool Productization

Goals:

- Compile and test pool server.
- Implement correct Stratum/share validation for one algorithm.
- Implement accounting reconciliation.
- Add payout controls and hot wallet isolation.

Exit criteria:

- Pool can be audited independently.
- Payouts reconcile with shares, blocks, and balances.

## 16. Current Repository Gap Summary

The current repository is not product-grade because the Level 2 local miner workflow is not complete end to end:

- Live XMRig pool/session/share validation with an approved pool and wallet has not been completed.
- Curated production manifest/release evidence for supported miner archives is not yet published.
- Mining state is not yet persisted and reconciled after daemon restart.
- Local web UI is not yet a complete operator workflow backed entirely by daemon state.
- CI parity for the enforced local gates is not yet confirmed.
- Hardware telemetry and safety policies need release-grade validation.
- Fleet, pool, and PoC crates remain quarantined outside product claims.
- Documentation must continue to be updated as tracker items move to `done`.

These gaps must be treated as release blockers, not polish items.

## 17. Definition of Done

A product feature is done only when all are true:

- It compiles in the root workspace.
- It has unit and integration coverage appropriate to risk.
- It has no TODO placeholder on an active execution path.
- It has typed errors and observable logs/metrics.
- It has security controls for its threat surface.
- It is documented for users or operators where applicable.
- It is included in CI gates.
- Its README/product claim is accurate.

## 18. Immediate Engineering Priorities

1. Establish root workspace and hard CI gates.
2. Make incomplete crates either compile or be explicitly marked experimental.
3. Replace placeholder daemon mining API behavior with real lifecycle control.
4. Implement one verified miner adapter end to end.
5. Build credible tests around real local mining workflows.
6. Remove default credentials, checked-in secrets, and production overclaims.
