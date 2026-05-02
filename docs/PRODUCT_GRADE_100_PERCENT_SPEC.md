# Product Grade 100 Percent Specification

Status: active development control spec  
Last updated: 2026-05-02  
Companion docs: `PRODUCT_SPEC.md`, `PRODUCT_IMPLEMENTATION_TRACKER.md`, `BUILD_BASELINE.md`, `specs/LOCAL_MINER_MVP.md`

## Purpose

This document defines what "100% product grade" means for BUNKER MINER and turns the current readiness assessment into a concrete development target. Percentages are evidence-based readiness scores, not time estimates. A scope reaches 100% only when its code, tests, documentation, release evidence, and operational path all satisfy the gates below.

## Current Readiness

| Scope | Current Readiness | 100% Meaning |
| --- | ---: | --- |
| Real daemon/CLI core | 70% | Daemon and CLI expose complete local operator workflows with typed state, durable process reconciliation, atomic config apply, clear errors, and no mock success paths. |
| Verified miner install + trust chain | 70% | Curated manifests, archive/executable verification, rollback, signing policy, audit logs, and release-owned update process are in place. |
| XMRig real execution/telemetry path | 65% | XMRig is validated in diagnostic and live pool modes with telemetry, pool status, share state, crash/stop behavior, and documented supported versions. |
| Local Miner MVP overall | 60-65% | A user can install, configure, run, observe, stop, recover, and troubleshoot one supported miner locally through CLI and UI. |
| Production local miner release | 35-45% | Signed/reproducible artifacts, CI parity, packaging, support bundle, upgrade/rollback, security scans, and release evidence are complete. |
| Full BUNKER MINER platform with fleet/profit/pool/UI | 20-25% | Local, fleet, profitability, UI, and pool tiers are integrated, secured, tested, observable, and documented. |
| Industry-leading optimized crypto miner | 10-15% | Efficiency gains are measured, reproducible, statistically defensible, hardware-aware, and better than baseline miner configurations. |

## Global 100% Rules

- No user-visible feature can claim success unless it is backed by real execution or a typed unavailable/error state.
- No binary execution path may run unverified third-party code by default.
- No command may use shell interpolation for miner execution.
- Every state-changing operation must be auditable and return structured errors.
- Every release claim must have an automated gate or manual validation evidence recorded in docs.
- Every production secret path must redact secrets in logs, CLI output, UI, support bundles, and crash reports.
- Quarantined crates do not count toward product readiness until promoted into the root workspace.
- Network, pool, fleet, and remote-control features must be opt-in and secure by default.
- Mining must always be explicit user/operator intent. No hidden persistence, stealth execution, or ambiguous background mining behavior.

## Scope 1: Real Daemon/CLI Core

Current readiness: 70%.

### Already Proven

- Root Rust workspace builds, tests, formats, and passes clippy with warnings denied.
- Daemon service can start non-interactively with isolated config.
- CLI can call daemon health/start/stop/watch/config/miner install surfaces.
- `StartMining`/`StopMining` use real process supervision and typed command responses.
- XMRig can be installed, launched, observed, and stopped through daemon/CLI.

### Remaining To 100%

| ID | Requirement | Acceptance Gate | Depends On |
| --- | --- | --- | --- |
| CORE-001 | Mining state API | API exposes idle, installing, starting, running, stopping, stopped, error, crashed, restarting, and degraded states with active miner/config summary. | LM-009 |
| CORE-002 | Durable state reconciliation | Daemon restart detects stale process state, cleans up orphan supervisors, and reports last known run/error state. | CORE-001 |
| CORE-003 | Crash classification | Miner exits are classified as normal, config error, pool error, binary error, permission error, crash, or timeout. | LM-008 |
| CORE-004 | Restart policy enforcement | Restart limits, backoff, final failure state, and operator remediation are observable and tested. | CORE-003 |
| CORE-005 | Atomic config apply | `SetConfig` validates schema, applies atomically, persists encrypted config, reports restart needs, and redacts secrets. | LM-010 |
| CORE-006 | CLI status surface | CLI has `status` or equivalent command reading daemon state, not inferred local state. | CORE-001 |
| CORE-007 | CLI support output | CLI can export a redacted support bundle path from daemon data. | REL-004 |
| CORE-008 | Contract tests | Protobuf/gRPC compatibility tests cover daemon and CLI generated types. | REL-001 |

### 100% Exit Gate

- Local daemon and CLI can complete install, configure, start, watch, status, stop, restart-after-crash, and support export workflows using only real daemon state.
- All daemon lifecycle errors are structured, actionable, and tested.
- Daemon restart does not leave the operator guessing whether mining is running.

## Scope 2: Verified Miner Install And Trust Chain

Current readiness: 70%.

### Already Proven

- Manifest schema validates miner identity, version, platform, executable, source URL, and hashes.
- Manifest-backed installer downloads HTTPS zip archives, verifies archive hash, extracts only the expected executable, verifies executable hash, writes checksum sidecar, and installs into managed storage.
- `StartMining` verifies miner binaries fail-closed before execution.

### Remaining To 100%

| ID | Requirement | Acceptance Gate | Depends On |
| --- | --- | --- | --- |
| TRUST-001 | Curated production manifest | Repository or release channel contains approved manifest records for supported miners and platforms. | LM-006 |
| TRUST-002 | Manifest signing policy | Manifest signatures are verified before production install unless explicitly using local operator manifest mode. | TRUST-001 |
| TRUST-003 | Binary rollback | Installer can roll back to previous verified executable on failed replacement or failed post-install verification. | LM-006 |
| TRUST-004 | Version policy | Supported, deprecated, blocked, and vulnerable miner versions are documented and enforced. | TRUST-001 |
| TRUST-005 | Archive format policy | Supported archive formats are explicit; zip slip, duplicate executable, size limits, and symlink cases are tested. | LM-006 |
| TRUST-006 | Install audit trail | Install attempts record source, hash, version, result, and error without leaking secrets. | CORE-001 |
| TRUST-007 | Offline install | Operator can install from local archive plus manifest/hash without network. | TRUST-001 |
| TRUST-008 | Multi-platform manifests | Windows, Linux, and macOS support is either validated or explicitly unavailable. | TRUST-001 |

### 100% Exit Gate

- Supported miner binaries can be installed, verified, upgraded, rolled back, audited, and blocked if vulnerable.
- Production install never relies on unverified URLs or ad hoc checksums.
- Development escape hatches are clearly excluded from release gates.

## Scope 3: XMRig Execution And Telemetry

Current readiness: 65%.

### Already Proven

- Official XMRig 6.20.0 Windows archive was downloaded and archive SHA-256 verified.
- XMRig executable SHA-256 was captured and installed through daemon/CLI.
- Daemon launched XMRig in bounded diagnostic benchmark mode.
- CLI observed nonzero RandomX telemetry and stopped XMRig through daemon control.

### Remaining To 100%

| ID | Requirement | Acceptance Gate | Depends On |
| --- | --- | --- | --- |
| XMR-001 | Live pool validation | XMRig connects to an approved pool using an operator-approved wallet/test account and reports pool state. | LM-012 |
| XMR-002 | Share telemetry | Accepted, rejected, stale, and acceptance rate are parsed from real XMRig output. | XMR-001 |
| XMR-003 | Pool failure telemetry | Bad host, timeout, auth failure, and disconnect cases produce typed daemon state and CLI/UI messages. | CORE-001 |
| XMR-004 | Supported XMRig versions | Supported XMRig versions are declared with hashes, validation dates, and platform support. | TRUST-001 |
| XMR-005 | Linux validation | XMRig install/start/telemetry/stop is validated on Linux. | TRUST-008 |
| XMR-006 | macOS policy | macOS is validated or explicitly marked unavailable for XMRig. | TRUST-008 |
| XMR-007 | CPU device mapping | CPU threads/device selection are represented consistently in device/state APIs. | SBP-003 |
| XMR-008 | Telemetry units | CLI/UI/API display H/s, kH/s, MH/s correctly without rounding meaningful values to zero. | done in CLI, UI pending |

### 100% Exit Gate

- XMRig has repeatable diagnostic validation and live pool validation.
- Pool and share state are visible through API, CLI, and UI.
- Supported versions/platforms are explicit and backed by release evidence.

## Scope 4: Local Miner MVP

Current readiness: 60-65%.

### Remaining To 100%

| ID | Requirement | Acceptance Gate | Depends On |
| --- | --- | --- | --- |
| MVP-001 | Live pool session | Complete `LM-012` with approved pool/wallet. | XMR-001 |
| MVP-002 | Mining state API | Complete `LM-009`. | CORE-001 |
| MVP-003 | Crash recovery | Complete `LM-008`. | CORE-003 |
| MVP-004 | Config workflow | Complete `LM-010`. | CORE-005 |
| MVP-005 | Local dashboard workflow | Complete `LM-011` using daemon state only. | CORE-001 |
| MVP-006 | Production manifest | Complete curated manifest for at least XMRig on Windows. | TRUST-001 |
| MVP-007 | Operator docs | Setup, manifest, start, stop, troubleshoot, support bundle docs match actual behavior. | MVP-001 |
| MVP-008 | Local install smoke | Fresh-machine or clean-VM smoke validates full MVP workflow. | REL-002 |

### 100% Exit Gate

- A non-developer can install the product, configure wallet/pool, install a supported miner, mine locally, observe state, stop safely, and troubleshoot failures.
- CLI and local UI reflect the same daemon truth.
- No MVP path depends on quarantined crates.

## Scope 5: Production Local Miner Release

Current readiness: 35-45%.

### Remaining To 100%

| ID | Requirement | Acceptance Gate | Depends On |
| --- | --- | --- | --- |
| REL-001 | CI parity | GitHub CI runs fmt, check, test, clippy with warnings denied for enforced workspace. | current gates |
| REL-002 | Packaging | Signed or checksummed release artifact exists for supported OS. | MVP-008 |
| REL-003 | Dependency/security audit | Cargo audit or equivalent, license scan, secret scan, and dependency policy block release. | REL-001 |
| REL-004 | Support bundle | Redacted logs/config/versions/system summary export is available. | CORE-007 |
| REL-005 | Upgrade/rollback | Config migrations and miner binary rollback are tested. | TRUST-003 |
| REL-006 | Release manifest | Release notes list exact supported features, unsupported features, hashes, and validation evidence. | MVP-007 |
| REL-007 | Incident procedure | Incident response, vulnerable miner block, and rollback procedure are documented and rehearsed. | TRUST-004 |
| REL-008 | Telemetry/privacy policy | Local and optional remote telemetry collection behavior is documented and enforced. | SBP-006 |
| REL-009 | Installer smoke matrix | Clean install/upgrade/uninstall smoke runs on each supported OS. | REL-002 |

### 100% Exit Gate

- Release artifacts are reproducible enough for the project stage, signed or checksummed, scanned, documented, and smoke-tested.
- Support and rollback paths exist before users rely on the product.
- CI blocks regressions that would invalidate product claims.

## Scope 6: Full Platform With Fleet, Profit, Pool, UI

Current readiness: 20-25%.

### Local UI

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| UI-001 | Dashboard state | Dashboard reads mining/config/telemetry state from daemon API only. |
| UI-002 | Start/stop workflow | UI start/stop has pending, success, failure, and remediation states. |
| UI-003 | Install workflow | UI can install miners through manifest-backed daemon API. |
| UI-004 | Config workflow | UI edits validated config atomically with secret redaction. |
| UI-005 | Responsive operator surface | UI is usable on desktop and narrow laptop widths without overlaps. |

### Safety, Benchmarking, Profit

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| SBP-001 | Stable device model | Device IDs/capabilities survive daemon restart where hardware permits. |
| SBP-002 | NVIDIA telemetry | NVML metrics validated with hardware evidence. |
| SBP-003 | CPU telemetry | CPU telemetry model is real and separate from GPU-only fields. |
| SBP-004 | AMD policy | AMD paths are validated or explicitly unavailable. |
| SBP-005 | Benchmark lifecycle | Warmup, measurement, cooldown, cancellation, persistence, and result quality checks. |
| SBP-006 | Safety policy | Emergency stop, unsupported controls, thermal thresholds, and opt-in rules are tested. |
| SBP-007 | Profit recommendations | Recommendations cite benchmark, market, pool, fee, and power inputs. |
| SBP-008 | Auto-switching | Opt-in hysteresis, rollback, dwell time, and audit trail. |

### Fleet

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| FLT-001 | Workspace promotion | `fleet/` builds and tests under root workspace with warnings denied. |
| FLT-002 | Enrollment | Rigs enroll with authenticated identities and revocation. |
| FLT-003 | Policy sync | Fleet policies sync safely and are auditable. |
| FLT-004 | Remote commands | Remote commands have authorization, audit, limits, and failure states. |
| FLT-005 | Telemetry ingestion | Fleet telemetry ingest is durable and backpressure-safe. |
| FLT-006 | Multi-rig UI | Operator can filter, inspect, and command rigs predictably. |

### Pool

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| POOL-001 | Workspace promotion | `pool/` builds and tests under root workspace with warnings denied. |
| POOL-002 | Stratum correctness | Subscribe/authorize/job/submit paths are tested against real miners. |
| POOL-003 | Share validation | Difficulty, target, duplicate, stale, and invalid share handling are correct. |
| POOL-004 | Accounting | Miner balances, fees, payouts, and audit trails are internally consistent. |
| POOL-005 | Wallet controls | Hot wallet, payout limits, and incident controls are reviewed before production. |
| POOL-006 | Pool observability | Pool has metrics, logs, alerts, and abuse/rate controls. |

### 100% Exit Gate

- Fleet, profit, pool, and UI are promoted from aspirational docs/quarantined crates to tested product surfaces.
- Full platform features are secure, observable, documented, and integrated with local daemon state.

## Scope 7: Industry-Leading Optimized Crypto Miner

Current readiness: 10-15%.

This scope is not achieved by wrapping XMRig alone. It requires measurable, repeatable operator value beyond baseline miner execution.

### Remaining To 100%

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| IL-001 | Baseline benchmark suite | Representative CPU/GPU rigs compare stock miner configs against BUNKER MINER recommendations. |
| IL-002 | Efficiency model | Hashrate per watt improvements are measured with confidence intervals. |
| IL-003 | Tuning search | Safe bounded tuning explores thread/device/power/profile parameters without unsafe defaults. |
| IL-004 | Explainability | Every optimization explains inputs, expected gain, risk, and rollback. |
| IL-005 | Reproducibility | Same hardware/config produces comparable recommendations across runs. |
| IL-006 | Market-aware profit | Profit decisions use fresh market/pool data, fees, power cost, and measured benchmarks. |
| IL-007 | Stability scoring | Recommendations include reject rate, stale shares, thermal throttling, crash history, and pool latency. |
| IL-008 | External review | Security and performance claims receive independent review or equivalent internal evidence package. |
| IL-009 | Fleet-scale optimization | Recommendations are validated across many rigs without command storms or unsafe simultaneous changes. |

### 100% Exit Gate

- BUNKER MINER demonstrates statistically defensible efficiency or profitability gains against baseline miner configurations on representative hardware.
- Gains are reproducible, explainable, reversible, and safe by default.
- Marketing claims match measured evidence.

## Milestone Order

1. Finish Local Miner MVP:
   - `LM-012`, `LM-008`, `LM-009`, `LM-010`, `LM-011`, `TRUST-001`.
2. Harden production release:
   - `REL-001` through `REL-009`.
3. Expand hardware and safety:
   - `SBP-001` through `SBP-008`.
4. Promote UI and local product polish:
   - `UI-001` through `UI-005`.
5. Promote fleet:
   - `FLT-001` through `FLT-006`.
6. Promote pool:
   - `POOL-001` through `POOL-006`.
7. Prove industry-leading optimization:
   - `IL-001` through `IL-009`.

## Next Implementation Queue

| Order | Work | Why It Is Next |
| ---: | --- | --- |
| 1 | `LM-012` live pool/session/share validation | Converts diagnostic XMRig proof into real mining-session proof. |
| 2 | `LM-009` mining state API | Required for UI, restart reconciliation, support, and product-grade status. |
| 3 | `LM-008` crash recovery | Real miners fail; product must classify and recover predictably. |
| 4 | `LM-010` atomic config apply | Operators need safe config changes before release. |
| 5 | `TRUST-001` curated manifest | Production install cannot depend on ad hoc validation scripts. |
| 6 | `LM-011` local dashboard workflow | UI cannot claim product readiness until it uses daemon truth. |
| 7 | `REL-001` CI parity | Local green gates must become enforced remote gates. |
| 8 | `REL-004` support bundle | Product support needs redacted diagnostics. |
| 9 | `REL-002` packaging | Users need installable release artifacts. |
| 10 | `SBP-003` CPU telemetry cleanup | XMRig RandomX is CPU-first; telemetry must model that cleanly. |

## Readiness Scoring Rules

Use these rules when updating readiness percentages:

- 0-20%: concept, prototype, or quarantined code only.
- 21-40%: compiles or partially works locally, but lacks end-to-end proof.
- 41-60%: core path works with tests, but major product workflows are missing.
- 61-80%: end-to-end path works and has evidence, but release/edge cases are incomplete.
- 81-95%: production candidate with CI, packaging, docs, support, security gates, and broad validation.
- 96-100%: shipped quality with operational evidence, rollback, monitoring, incident readiness, and claims backed by repeated validation.

Readiness cannot increase from code alone. It requires matching tests, documentation, and evidence.

## Definition Of Done For New Work

Every implementation unit must include:

- Tracker ID or new ID added to `PRODUCT_IMPLEMENTATION_TRACKER.md`.
- Code path that fails closed for security-sensitive behavior.
- Unit tests for pure logic.
- Integration or smoke evidence for user-visible workflows.
- Documentation update in the relevant spec or baseline.
- No warnings under enforced workspace gates.
- No secret leakage in logs, CLI output, UI output, support data, or test artifacts.
- Clear unsupported-state behavior when a platform, miner, pool, or hardware path is not ready.

## Explicit Non-Goals Until Prerequisites Are Done

- Do not promote `fleet/` before local MVP and release gates are stable.
- Do not promote `pool/` before Stratum/share validation and accounting design are reviewed.
- Do not ship automatic overclocking before safety policy, opt-in UX, and rollback are tested.
- Do not claim AI or industry-leading optimization before benchmark evidence exists.
- Do not ship remote access without authentication, authorization, TLS policy, and audit logs.
- Do not add hidden persistence, stealth startup, or unclear mining behavior.
