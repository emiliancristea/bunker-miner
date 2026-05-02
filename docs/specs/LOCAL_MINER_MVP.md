# Local Miner MVP Specification

Status: active implementation spec  
Tracker IDs: LM-001 through LM-011  
Last updated: 2026-05-02

## Purpose

The Local Miner MVP is the first product-grade release target. It must let an operator run one supported, verified miner from local daemon control, observe real telemetry, stop it safely, and understand every failure without mock success.

## Supported Initial Workflow

1. Operator installs BUNKER MINER.
2. Operator provides a config password from an interactive prompt, `BUNKER_MINER_CONFIG_PASSWORD`, or `BUNKER_MINER_CONFIG_PASSWORD_FILE`.
3. Daemon creates or loads encrypted config under the default config directory or `BUNKER_MINER_CONFIG_DIR`.
4. Operator installs a supported miner binary through the managed binary workflow or points to it with an explicit path.
5. Daemon verifies the miner binary against a trusted SHA-256 from sidecar, environment, or validated manifest.
6. Operator starts mining through CLI, local UI, or daemon API.
7. Daemon starts the miner without shell interpolation, parses telemetry, and exposes state through gRPC.
8. Operator stops mining and daemon exits the miner cleanly.

## Product Requirements

### Configuration

- Config files are encrypted at rest.
- Config templates may contain placeholder wallets, but mining startup must reject placeholder wallets.
- Config writes are validated and must not partially apply.
- Non-interactive service operation must not require a TTY.
- Secret values must not be printed in logs or support output.

### Miner Binary Trust

- Supported miners must have a manifest record containing name, version, platform, executable name, source URL, and SHA-256.
- Manifest records with invalid SHA-256 or mismatched identity are not trusted.
- Sidecar and explicit environment checksums may override manifest records for operator-managed installs.
- Automatic network downloads remain disabled until archive signature/checksum validation and rollback are implemented.
- `BUNKER_MINER_ALLOW_UNVERIFIED_MINERS=1` is development-only and cannot satisfy release gates.

### Miner Launch

- Miner arguments are built from typed config.
- No miner command path may use shell interpolation.
- Pool endpoints must be normalized so schemes are not duplicated.
- Start failures must return typed, actionable daemon responses.
- Stop must support graceful shutdown first and force kill only when requested or timed out.

### Telemetry

- Telemetry must be emitted from actual miner output and hardware APIs.
- Hashrate units must normalize to H/s internally and display in useful units at the UI/CLI boundary.
- Share counters must track accepted, rejected, stale, and acceptance rate.
- Missing hardware metrics must be explicit unavailable values, not zeros that imply real readings.

### Operator Surfaces

- CLI and local UI must display daemon state from gRPC.
- Start/stop commands must exit nonzero on daemon command failure after printing structured details.
- Dashboard failure must not take down the daemon control plane.
- UI success states must be driven by daemon responses only.

## Acceptance Tests

- Workspace gates pass with strict warnings.
- Unit tests validate manifest parsing, hash matching, pool endpoint normalization, config readiness, and telemetry parsing.
- Service smoke starts daemon with non-interactive config, calls CLI health, and verifies missing miner returns `MINER_BINARY_UNAVAILABLE`.
- Real miner validation runs XMRig through daemon + CLI with a trusted binary and captures start, telemetry, stop, and restart behavior.

## Release Blockers

- No verified miner install/acquire workflow.
- No real XMRig start-to-telemetry validation evidence.
- No persistent mining state recovery after daemon restart.
- No local UI workflow backed entirely by daemon state.
- No CI parity for local release gates.
