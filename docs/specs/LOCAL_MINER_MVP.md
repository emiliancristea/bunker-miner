# Local Miner MVP Specification

Status: active implementation spec  
Tracker IDs: LM-001 through LM-012  
Last updated: 2026-05-02

## Purpose

The Local Miner MVP is the first product-grade release target. It must let an operator run one supported, verified miner from local daemon control, observe real telemetry, stop it safely, and understand every failure without mock success.

## Supported Initial Workflow

1. Operator installs BUNKER MINER.
2. Operator provides a config password from an interactive prompt, `BUNKER_MINER_CONFIG_PASSWORD`, or `BUNKER_MINER_CONFIG_PASSWORD_FILE`.
3. Daemon creates or loads encrypted config under the default config directory or `BUNKER_MINER_CONFIG_DIR`.
4. Operator installs a supported miner binary through `bunker-miner-cli miner install --name <miner> --version <version>` or points to an already installed binary with an explicit path.
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
- Manifest-backed install requires `archive_sha256`; the archive and extracted executable are both verified before the executable is moved into managed storage.
- Manifest records with invalid SHA-256 or mismatched identity are not trusted.
- Sidecar and explicit environment checksums may override manifest records for operator-managed installs.
- StartMining never performs implicit network downloads; acquisition must be an explicit install command/API call.
- The current managed acquisition path supports HTTPS zip archives and extracts only the expected executable name.
- `BUNKER_MINER_ALLOW_UNVERIFIED_MINERS=1` is development-only and cannot satisfy release gates.

Manifest file locations:

- `BUNKER_MINER_MANIFEST_PATH`
- `<managed config dir>/miner-manifest.toml`

Manifest record shape:

```toml
schema_version = 1

[[miners]]
name = "XMRig"
version = "6.20.0"
platform = "windows-x86_64"
executable = "xmrig.exe"
sha256 = "<64-character executable sha256>"
source_url = "https://example.com/xmrig.zip"
archive_sha256 = "<64-character archive sha256>"
signature_url = "https://example.com/xmrig.zip.sig"
```

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
- `bunker-miner-cli status` must read `GetMiningState` and display lifecycle, active config summary, restart count, and latest telemetry availability without exposing raw wallet or pool password values.
- CLI must expose manifest-backed miner installation and return nonzero when daemon installation fails.
- Start/stop commands must exit nonzero on daemon command failure after printing structured details.
- Dashboard failure must not take down the daemon control plane.
- UI success states must be driven by daemon responses only.

## Acceptance Tests

- Workspace gates pass with strict warnings.
- Unit tests validate manifest parsing, archive/executable hash matching, safe zip extraction, no-force replacement guards, pool endpoint normalization, config readiness, and telemetry parsing.
- Service smoke starts daemon with non-interactive config, calls CLI health, and verifies missing miner returns `MINER_BINARY_UNAVAILABLE`.
- State smoke starts daemon with non-interactive config and verifies `bunker-miner-cli status` returns the daemon-owned idle/stopped state.
- Real miner validation runs XMRig through daemon + CLI with a trusted binary and captures start, nonzero telemetry, and stop behavior.
- Live pool validation runs XMRig against an approved pool/wallet pair and captures pool connection/share state.
- `scripts/validate-xmrig-local-miner.ps1 -LivePool <host:port> -LiveWallet <wallet> [-LivePassword <password>]` is the current evidence harness for live XMRig validation; it must not be run with placeholder credentials.

## Release Blockers

- No recorded live pool/session/share validation with an approved pool and wallet.
- No curated production manifest/release evidence for supported miner archives.
- No persistent mining state recovery after daemon restart.
- No local UI workflow backed entirely by daemon state.
- No CI parity for local release gates.
