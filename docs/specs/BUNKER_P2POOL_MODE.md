# BUNKER P2Pool Mode Specification

Status: planned product track  
Last updated: 2026-05-02  
Primary scope: Local Miner MVP, pool integration path, non-custodial Monero mining

## Product Decision

BUNKER should not build and operate a centralized custodial BUNKER Pool as the next pool milestone.

The product-grade path is:

1. Support direct public pool mining.
2. Add external P2Pool mode where the operator runs `monerod` and P2Pool, and BUNKER connects XMRig to the local P2Pool Stratum endpoint.
3. Add managed P2Pool supervision where BUNKER verifies, installs, starts, observes, and stops P2Pool.
4. Add managed `monerod` only after node sync, disk, RPC, ZMQ, privacy, and release trust gates are explicit.
5. Treat a centralized BUNKER Pool as a later platform tier because it adds share accounting, payout custody, hot wallet controls, abuse handling, operational uptime, financial risk, and independent correctness requirements.

P2Pool-first gives us real pool behavior, real share evidence, and direct wallet payouts without making BUNKER the payout custodian.

## Current Upstream Facts

Verified online on 2026-05-02:

| Component | Current finding | Product impact |
| --- | --- | --- |
| P2Pool | Official site describes P2Pool as a decentralized Monero mining pool. XMRig connects to the P2Pool Stratum server on port `3333`, and the wallet is supplied to P2Pool, not XMRig. | BUNKER needs a pool mode where XMRig login can be `x`/worker difficulty while the wallet belongs to P2Pool configuration. Current direct-pool wallet assumptions are not enough. |
| P2Pool release | Latest GitHub release observed: `v4.15`, published 2026-05-01, with Windows/Linux/macOS assets and `sha256sums.txt.asc`. | Managed P2Pool install must verify archives through release-owned hashes/signatures before execution. |
| Monero daemon | P2Pool setup requires a synced `monerod` with ZMQ publishing enabled. The P2Pool docs recommend local ZMQ on `127.0.0.1:18083`; Monero docs state RPC defaults to localhost because the API has administrative capabilities. | First product pass should support external `monerod` health checks. Managed node operation is a later, heavier feature because initial sync can take hours or days and needs substantial disk/network resources. |
| XMRig | XMRig pool config supports pool `url`, `user`, `pass`, `tls`, `rig-id`, keepalive, daemon mode, and related fields. Latest GitHub release observed: `v6.26.0`, published 2026-03-28. | Existing XMRig direct pool support remains valid, but P2Pool needs a profile-specific launch mapping. |
| Monero downloads | Official downloads page states canonical SHA256 hashes are in a GPG-signed hash list. P2Pool release notes call for Monero daemon `v0.18.4.6` or newer. | Managed `monerod` install must not rely on GitHub release assets alone; it must use official Monero download and verification procedures. |

Sources:

- P2Pool setup and mining flow: https://p2pool.io/
- P2Pool releases and verification notes: https://github.com/SChernykh/p2pool/releases
- XMRig pool configuration: https://xmrig.com/docs/miner/config/pool
- Monero daemon reference: https://docs.getmonero.org/interacting/monerod-reference/
- Monero downloads and canonical hash policy: https://www.getmonero.org/downloads/
- XMRig releases: https://github.com/xmrig/xmrig/releases

## Target Architecture

```text
Phase A: Direct pool

BUNKER daemon
  -> verified XMRig
  -> approved external pool endpoint
  -> operator wallet passed as XMRig user

Phase B: External P2Pool

operator-managed monerod
  -> operator-managed P2Pool --wallet <primary wallet>
  -> local Stratum endpoint 127.0.0.1:3333
  -> BUNKER daemon
  -> verified XMRig -o 127.0.0.1:3333 -u x

Phase C: Managed P2Pool

BUNKER daemon
  -> verified monerod or externally configured monerod
  -> verified P2Pool supervised process
  -> verified XMRig supervised process
  -> one typed state model for node, pool, miner, shares, and errors
```

## Configuration Model

The current direct-pool shape is not enough for P2Pool because P2Pool owns the payout wallet and XMRig does not need the wallet address in its pool login.

Target model:

```text
PoolProfile::DirectPool
  coin
  algorithm
  url
  wallet_as_user
  password
  worker_name
  tls

PoolProfile::P2PoolExternal
  coin = monero
  algorithm = randomx
  stratum_url = 127.0.0.1:3333 by default
  xmrig_user = x or x+<fixed_difficulty>
  xmrig_password = x
  wallet_address optional for proof/observer linking, never passed to XMRig by default
  expected_network = mainnet | stagenet | testnet

PoolProfile::P2PoolManaged
  includes P2PoolExternal fields
  wallet_address required for P2Pool
  optional subaddress
  monerod_rpc_url
  monerod_zmq_url
  p2pool_variant = main | mini | nano
  p2pool_light_mode
```

Release rule: direct-pool mode requires a wallet/login for XMRig. P2Pool mode requires a wallet only when BUNKER starts or validates the P2Pool process itself.

## Implementation Requirements

| ID | Requirement | Acceptance Gate |
| --- | --- | --- |
| P2POOL-001 | External P2Pool profile | CLI/daemon can start verified XMRig against `127.0.0.1:3333` using P2Pool login semantics without passing the wallet to XMRig. |
| P2POOL-002 | P2Pool readiness probe | Daemon validates local Stratum TCP reachability and returns typed unavailable/error states before claiming mining has started. |
| P2POOL-003 | External monerod health probe | Daemon can report external `monerod` RPC reachability, sync state, network, and ZMQ configuration status without requiring BUNKER to own the node. |
| P2POOL-004 | P2Pool live evidence harness | Validation script can run against external P2Pool, capture XMRig pool connected state, accepted/rejected share counters, and redacted launch args. |
| P2POOL-005 | Managed P2Pool trust chain | Installer supports curated P2Pool manifests, archive hash verification, executable hash verification, and signature policy before execution. |
| P2POOL-006 | Managed P2Pool supervisor | Daemon can start, observe, stop, classify failures, and redact logs for P2Pool as a sibling supervised process to XMRig. |
| P2POOL-007 | Managed monerod policy | Product docs and config define local-only RPC/ZMQ defaults, disk/sync expectations, pruned-node option, and remote-node warnings before BUNKER starts `monerod`. |
| P2POOL-008 | UI/CLI operator flow | Operator can choose direct pool, external P2Pool, or managed P2Pool with clear pending/error/remediation states and no mock success. |
| P2POOL-009 | Observer/share proof | Product evidence can link local share telemetry to P2Pool-side status without leaking secrets or requiring BUNKER custody. |

## Security And Product Rules

- Mining must remain explicit operator intent. No hidden startup, persistence, or stealth execution.
- Default RPC and ZMQ endpoints must bind to localhost unless the operator explicitly configures and accepts remote exposure.
- BUNKER must not collect or store Monero private keys.
- P2Pool wallet addresses are public in P2Pool context; UI/docs must recommend a mining-specific wallet.
- Managed binary installation must be fail-closed if hashes or signatures do not match.
- Support bundles must redact wallet-like identifiers unless the operator explicitly chooses share-proof export.
- Remote public pool mining and P2Pool mining must be separate profiles so wallet/login semantics cannot be confused.

## Centralized BUNKER Pool Gate

The `pool/` crate remains a quarantined future product tier until these gates are satisfied:

- Stratum protocol correctness against real miners.
- Correct RandomX/Monero job construction or an explicit supported-coin decision if not Monero.
- Share validation for difficulty, target, duplicates, stale work, invalid work, and block candidates.
- Durable accounting and auditable PPLNS/PPS policy.
- Payout wallet controls, hot/cold separation, limits, incident procedures, and reconciliation.
- Abuse handling, DDoS/rate controls, monitoring, and on-call operational readiness.
- Legal/compliance review for operating a public payout service.

Until those gates are done, product messaging must not imply BUNKER operates a production payout pool.

## Next Engineering Order

1. Complete `LM-012` direct live-pool validation if an approved public pool endpoint and wallet/test login are available.
2. Implement `P2POOL-001` and `P2POOL-002` so BUNKER can use an existing local P2Pool instance.
3. Extend the validation harness for `P2POOL-004`.
4. Add `P2POOL-003` external `monerod` health checks.
5. Add managed P2Pool install/supervision only after the external profile is proven.
6. Revisit the centralized `pool/` crate after local miner release quality is established.
