# BUNKER MINER - Supported Mining Software

This document maintains the canonical list of all supported third-party mining software, including security validation and integration status.

## Currently Supported Miners

| Miner Name | Official Source URL | Pinned Version | SHA256 Checksum | Supported Algorithms | License | Status |
|------------|-------------------|----------------|-----------------|---------------------|---------|--------|
| lolMiner | https://github.com/Lolliedieb/lolMiner-releases | TBD | TBD | Ethash, EtcHash, Kaspa, Flux | Custom | Planned |
| XMRig | https://github.com/xmrig/xmrig | TBD | TBD | RandomX, CryptoNight variants | GPL v3 | Planned |
| GMiner | https://github.com/develsoftware/GMinerRelease | TBD | TBD | Ethash, EtcHash, Kaspa, RVN | Custom | Planned |
| T-Rex | https://github.com/trexminer/T-Rex | TBD | TBD | Ethash, EtcHash, Octopus, Kawpow | Custom | Planned |

## Miner Evaluation Queue

| Miner Name | Official Source URL | Supported Algorithms | Priority | Evaluation Status |
|------------|-------------------|---------------------|----------|-------------------|
| NBMiner | https://github.com/NebuTech/NBMiner | Ethash, EtcHash, Kaspa | High | Not Started |
| Rigel | https://github.com/rigelminer/rigel | Ethash, EtcHash, Kaspa, Alephium | Medium | Not Started |
| BzMiner | https://www.bzminer.com/ | Multi-algorithm | Medium | Not Started |
| SRBMiner | https://github.com/doktor83/SRBMiner-Multi | RandomX, CryptoNight | Low | Not Started |

## Algorithm Support Matrix

| Algorithm | Primary Miners | Hardware Support | Network Hashrate | Profitability Tier |
|-----------|----------------|------------------|------------------|-------------------|
| Kaspa (kHeavyHash) | lolMiner, GMiner | NVIDIA, AMD | High | Tier 1 |
| Ethash | lolMiner, GMiner, T-Rex | NVIDIA, AMD | Very High | Tier 1 |
| EtcHash | lolMiner, GMiner, T-Rex | NVIDIA, AMD | High | Tier 1 |
| RandomX | XMRig | CPU | Very High | Tier 2 |
| Kawpow (RVN) | T-Rex, GMiner | NVIDIA, AMD | Medium | Tier 2 |
| Flux (ZelHash) | lolMiner, GMiner | NVIDIA, AMD | Medium | Tier 2 |
| Octopus (CFX) | T-Rex, GMiner | NVIDIA, AMD | Medium | Tier 3 |

## Security Validation Status

### Phase 0.2 PoC Validation (In Progress)
- [ ] **lolMiner**: Security scan, checksum verification, adapter development
- [ ] **XMRig**: Security scan, checksum verification, adapter development  
- [ ] **GMiner**: Security scan, checksum verification, adapter development
- [ ] **T-Rex**: Security scan, checksum verification, adapter development

### Security Requirements
All miners must pass the following security validation:

#### Binary Analysis
- [ ] Static analysis for malicious behavior
- [ ] Dynamic analysis in sandboxed environment
- [ ] Network traffic monitoring and validation
- [ ] File system access pattern analysis

#### Checksum Verification
- [ ] Official SHA256 checksums obtained from source
- [ ] GPG signature verification (where available)
- [ ] Reproducible build verification (where available)
- [ ] Supply chain validation

#### Runtime Security
- [ ] Process isolation and sandboxing
- [ ] Network access restrictions
- [ ] File system access limitations  
- [ ] Resource usage monitoring

## Integration Status

### Miner Adapter Implementation
Each supported miner requires a custom adapter implementation:

```rust
pub struct LolMinerAdapter {
    // Configuration and state
}

impl MinerAdapter for LolMinerAdapter {
    fn get_name(&self) -> &str { "lolMiner" }
    fn get_supported_algorithms(&self) -> Vec<Algorithm> { /* ... */ }
    fn build_command_args(&self, config: &MinerConfig) -> Vec<String> { /* ... */ }
    fn get_output_parser(&self) -> Box<dyn OutputParser> { /* ... */ }
    fn get_process_monitor(&self) -> Box<dyn ProcessMonitor> { /* ... */ }
    fn validate_configuration(&self, config: &MinerConfig) -> Result<(), String> { /* ... */ }
}
```

### Implementation Progress
- [ ] **lolMiner Adapter**: Not started
- [ ] **XMRig Adapter**: Not started
- [ ] **GMiner Adapter**: Not started
- [ ] **T-Rex Adapter**: Not started

## Download and Verification Process

### Automated Download System
The daemon will automatically download and verify miner binaries on first use:

1. **Source Validation**: Verify download URL against official source
2. **Secure Download**: Download over HTTPS with certificate validation
3. **Checksum Verification**: Validate SHA256 against known good checksum
4. **Signature Verification**: Verify GPG signature where available
5. **Quarantine**: Place in quarantine directory for security scanning
6. **Security Scan**: Run through security analysis tools
7. **Approval**: Move to active miners directory after validation

### Manual Override Process
For advanced users, manual miner installation is supported:

1. User downloads miner from official source
2. User places binary in designated directory
3. Daemon verifies checksum against database
4. Security scan is performed
5. Manual approval required for unknown binaries

## Hardware Compatibility

### NVIDIA GPUs
| GPU Architecture | Driver Requirement | CUDA Version | Supported Miners | Notes |
|------------------|-------------------|--------------|------------------|-------|
| RTX 40 Series | 522.06+ | 12.0+ | All | Full support |
| RTX 30 Series | 470.05+ | 11.4+ | All | Full support |
| RTX 20 Series | 418.81+ | 10.1+ | All | Full support |
| GTX 16 Series | 418.81+ | 10.1+ | All | Full support |
| GTX 10 Series | 418.81+ | 10.1+ | lolMiner, GMiner | Limited support |

### AMD GPUs
| GPU Architecture | Driver Requirement | ROCm Version | Supported Miners | Notes |
|------------------|-------------------|--------------|------------------|-------|
| RDNA 3 (RX 7000) | 22.20.3+ | 5.4+ | lolMiner, GMiner | Full support |
| RDNA 2 (RX 6000) | 21.30.2+ | 4.5+ | All | Full support |
| RDNA (RX 5000) | 20.45.2+ | 4.0+ | All | Full support |
| GCN 5 (RX Vega) | 18.50+ | 3.5+ | lolMiner, GMiner | Limited support |

### CPU Mining
| Architecture | Supported Miners | Performance Tier | Notes |
|--------------|------------------|------------------|-------|
| x86_64 | XMRig | High | Full RandomX optimization |
| ARM64 | XMRig | Medium | Limited optimization |

## Performance Benchmarks

### Reference Performance (Placeholder Data)
| Hardware | Algorithm | Miner | Hashrate | Power | Efficiency |
|----------|-----------|-------|----------|-------|------------|
| RTX 4090 | Kaspa | lolMiner | TBD TH/s | TBD W | TBD MH/J |
| RTX 4090 | Ethash | lolMiner | TBD MH/s | TBD W | TBD KH/J |
| RX 7900 XTX | Kaspa | lolMiner | TBD TH/s | TBD W | TBD MH/J |
| Ryzen 9 7950X | RandomX | XMRig | TBD KH/s | TBD W | TBD H/J |

*Note: Actual benchmarks will be populated during Phase 1.1 (Device Detection & Benchmarking)*

## Miner-Specific Configuration

### lolMiner Configuration
```toml
[miners.lolminer]
executable_name = "lolMiner.exe"
config_format = "json"
api_port = 44444
special_args = ["--4g-alloc-size", "4024"]
```

### XMRig Configuration
```toml
[miners.xmrig]
executable_name = "xmrig.exe"
config_format = "json"
api_port = 44445
special_args = ["--randomx-1gb-pages"]
```

### GMiner Configuration
```toml
[miners.gminer]
executable_name = "miner.exe"
config_format = "cli"
api_port = 44446
special_args = ["--mt", "4"]
```

### T-Rex Configuration
```toml
[miners.trex]
executable_name = "t-rex.exe"
config_format = "json"
api_port = 44447
special_args = ["--exit-on-cuda-error"]
```

## Troubleshooting

### Common Issues
| Issue | Affected Miners | Solution |
|-------|----------------|----------|
| Insufficient VRAM | lolMiner, GMiner | Reduce memory allocation parameters |
| Driver compatibility | All | Update to minimum required driver version |
| Permission errors | All | Run daemon with appropriate privileges |
| Network connectivity | All | Verify pool configuration and firewall settings |

### Debug Mode
All miners support debug mode for troubleshooting:
```bash
bunker-miner-daemon start --debug --miner lolminer --algorithm kaspa
```

## Future Roadmap

### Phase 2 Enhancements
- [ ] Automatic miner selection based on profitability
- [ ] Multi-algorithm switching
- [ ] Advanced overclocking profiles per miner

### Phase 3 Enhancements
- [ ] BUNKER POOL integration for all miners
- [ ] Pool failover configuration
- [ ] Advanced fee optimization

### Phase 5 Enhancements
- [ ] Community plugin system for new miners
- [ ] WebAssembly-based miner adapters
- [ ] Marketplace integration for miner selection

## Compliance and Legal

### License Compliance
- All miners are evaluated for license compatibility
- GPL-licensed miners are isolated to prevent license contamination
- Commercial miners are used within their license terms

### Disclaimer
BUNKER MINER does not distribute third-party mining software. Users are responsible for:
- Downloading miners from official sources
- Complying with miner license terms
- Following local regulations regarding cryptocurrency mining

---

*This document is maintained as part of Phase 0 and updated throughout the project lifecycle as new miners are added and validated.*