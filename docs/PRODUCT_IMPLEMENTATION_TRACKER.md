# Product Implementation Tracker

Status: active execution tracker  
Last updated: 2026-05-02

This tracker converts `docs/PRODUCT_SPEC.md` and `docs/PRODUCT_GRADE_100_PERCENT_SPEC.md` into durable implementation units. A unit is complete only when code, tests, docs, and release evidence all match the acceptance criteria. Historical phase documents are not evidence unless the current workspace gates still prove them.

## Status Legend

- `done`: implemented, tested, documented, and included in the enforced workspace.
- `active`: current implementation target.
- `blocked`: cannot proceed until a named dependency is done.
- `planned`: accepted product work, not currently implemented.
- `quarantined`: outside release claims until promoted into the enforced workspace.

## Level 2: Local Miner MVP

| ID | Area | Status | Acceptance Gate | Evidence / Next Step |
| --- | --- | --- | --- | --- |
| LM-001 | Root Rust workspace gates | done | `fmt`, `check`, `test`, `clippy -D warnings` pass for enforced crates | See `docs/BUILD_BASELINE.md` |
| LM-002 | Daemon gRPC lifecycle | done | `StartMining`/`StopMining` use real process supervision and typed errors | Daemon workspace tests and CLI smoke evidence |
| LM-003 | CLI control surface | done | CLI compiles in workspace and builds real daemon requests | `tools/bunker-miner-cli` promoted |
| LM-004 | Non-interactive daemon service startup | done | Daemon can run with env/file supplied config secret and isolated config dir | `BUNKER_MINER_CONFIG_DIR`, `BUNKER_MINER_CONFIG_PASSWORD`, `BUNKER_MINER_CONFIG_PASSWORD_FILE` |
| LM-005 | Trusted miner manifest model | done | Manifest schema validates platform, miner identity, executable name, source URL, and SHA-256 | Implemented `daemon::miner_manifest`; checksum trust is wired into miner verification |
| LM-006 | Miner binary install/acquire workflow | done | User can install a manifest-described miner into managed storage with verified checksum and no shell execution | Manifest-backed HTTPS zip installer verifies archive SHA-256, extracts only the expected executable, verifies executable SHA-256, and is exposed through gRPC/CLI |
| LM-007 | Real XMRig start-to-telemetry validation | done | Verified XMRig launches, emits nonzero telemetry, and stops cleanly through daemon + CLI | `scripts/validate-xmrig-local-miner.ps1` passed with official XMRig 6.20.0 archive, manifest install, diagnostic benchmark, 745.40 H/s telemetry, and CLI stop |
| LM-008 | Miner crash recovery | planned | Crashes are classified, restart policy is enforced, and final state is observable | Needs integration harness |
| LM-009 | Mining state API | done | API exposes idle/starting/running/stopping/error state with active config and process health | `GetMiningState` and `bunker-miner-cli status` expose daemon lifecycle snapshot, redacted active config summary, restart count, and latest telemetry availability |
| LM-010 | Config validation/apply workflow | planned | Config validate/set is atomic and secrets are redacted in exports/logs | Current gRPC set path needs schema validation |
| LM-011 | Local web dashboard operator flow | planned | Dashboard reads daemon state and performs start/stop with real pending/error states | No mock success allowed |
| LM-012 | Live pool session validation | blocked | XMRig connects to a real pool with an operator-provided wallet or approved P2Pool profile and reports pool/share state without diagnostic benchmark mode | Parser/CLI/script harness implemented; final evidence still requires approved public pool plus wallet/test account, or external local P2Pool profile work |

## Level 2.5: Non-Custodial Pool Integration

These items are part of the local miner product path. They are intentionally separate from the quarantined centralized `pool/` crate because P2Pool can provide product-grade pool behavior without BUNKER taking payout custody.

| ID | Area | Status | Acceptance Gate | Evidence / Next Step |
| --- | --- | --- | --- | --- |
| P2POOL-001 | External P2Pool profile | planned | CLI/daemon can start verified XMRig against `127.0.0.1:3333` using P2Pool login semantics without passing the wallet to XMRig | Add pool profile enum/config mapping and start-command tests |
| P2POOL-002 | P2Pool readiness probe | planned | Daemon validates local Stratum TCP reachability and returns typed unavailable/error state before mining claims success | Add probe and CLI/status output |
| P2POOL-003 | External monerod health probe | planned | Daemon can report external `monerod` RPC reachability, sync state, network, and ZMQ readiness | Add health model after P2Pool profile exists |
| P2POOL-004 | P2Pool live evidence harness | planned | Validation script can run against external P2Pool and capture XMRig pool/share telemetry with redacted launch args | Extend `scripts/validate-xmrig-local-miner.ps1` |
| P2POOL-005 | Managed P2Pool trust chain | planned | Curated P2Pool manifests verify archive/signature/executable before execution | Depends on production manifest policy |
| P2POOL-006 | Managed P2Pool supervisor | planned | Daemon starts/stops/observes P2Pool as a supervised process with typed failures | Depends on P2POOL-005 and crash model |
| P2POOL-007 | Managed monerod policy | planned | Local-only RPC/ZMQ defaults, disk/sync expectations, pruned-node option, and remote-node warnings are documented/enforced | Requires product decision on managed node scope |
| P2POOL-008 | UI/CLI operator flow | planned | Direct pool, external P2Pool, and managed P2Pool are separate operator workflows with no mock success | Depends on P2POOL-001 through P2POOL-003 |
| P2POOL-009 | Observer/share proof | planned | Local telemetry can be tied to P2Pool-side share/status evidence without BUNKER custody | Requires approved wallet/share proof session |

## Level 3: Safety, Benchmarking, Profit

| ID | Area | Status | Acceptance Gate | Evidence / Next Step |
| --- | --- | --- | --- | --- |
| SBP-001 | Stable device model | planned | Device IDs and capabilities survive daemon restart where hardware permits | Requires persistence design |
| SBP-002 | NVIDIA telemetry validation | planned | NVML metrics have hardware-backed tests or recorded validation evidence | Current detection works locally, not release evidence |
| SBP-003 | CPU telemetry validation | planned | CPU telemetry is real and separated from GPU-only fields | Requires model cleanup |
| SBP-004 | AMD availability policy | planned | AMD paths are either validated or explicitly unavailable in API/UI | No placeholder support claims |
| SBP-005 | Benchmark lifecycle | planned | Warmup, measurement, cooldown, persisted result, and cancellation | Existing benchmarking needs product gate |
| SBP-006 | Safety policy | planned | Emergency stop and unsupported controls are tested | OC/power engines remain disabled by default |
| SBP-007 | Profit recommendations | planned | Recommendations cite stored benchmark and market inputs | Current profit data is not release-grade |
| SBP-008 | Auto-switching | blocked | Opt-in hysteresis and rollback are audited | Depends on SBP-005 and SBP-007 |

## Level 4: Release and Operations

| ID | Area | Status | Acceptance Gate | Evidence / Next Step |
| --- | --- | --- | --- | --- |
| REL-001 | CI parity | planned | GitHub CI runs the same enforced workspace gates | Local gates are defined; CI must mirror them |
| REL-002 | Packaging | planned | Signed installers or release archives with checksums | Requires stable MVP workflow |
| REL-003 | Dependency audit | planned | Audit, license, and secret scans block release | Add release branch gates |
| REL-004 | Support bundle | planned | Logs/config summaries are exported with secret redaction | Requires observability model |
| REL-005 | Upgrade/rollback | planned | Config migrations and binary rollback are tested | Depends on versioned data model |

## Quarantined Product Tiers

| ID | Area | Status | Acceptance Gate | Evidence / Next Step |
| --- | --- | --- | --- | --- |
| FLT-001 | Fleet crate promotion | quarantined | Fleet builds in workspace with no default credentials or SQLx drift | After Local Miner MVP |
| POOL-001 | Centralized pool crate promotion | quarantined | Pool builds and has correctness tests for Stratum/share validation | Separate product tier after P2Pool/direct-pool local workflows; centralized payout service remains out of release claims |
| POC-001 | Developer PoC tools | quarantined | Keep excluded unless a named owner and product purpose exists | No release claims |

## Operating Rules

- New implementation work must reference at least one tracker ID.
- A tracker item cannot move to `done` without tests and documentation.
- A user-visible feature cannot be claimed in README/release notes unless its tracker item is `done`.
- Quarantined crates are not product surface.
- Safety and binary execution changes require fail-closed behavior by default.
