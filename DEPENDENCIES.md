# BUNKER MINER - Core Dependencies

This document tracks all core software dependencies for the BUNKER MINER project. All dependencies must be evaluated according to the criteria outlined in `docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md`.

## Rust Dependencies

### Core Daemon Dependencies
| Crate Name | Version | License | Purpose | Security Review Status | Last Updated |
|------------|---------|---------|---------|------------------------|--------------|
| tokio | 1.x | MIT | Async runtime for daemon | Pending | TBD |
| tonic | 0.x | MIT | gRPC server/client implementation | Pending | TBD |
| serde | 1.x | MIT/Apache-2.0 | Serialization framework | Pending | TBD |
| toml | 0.x | MIT/Apache-2.0 | TOML configuration parsing | Pending | TBD |
| age | 0.x | MIT/Apache-2.0 | File encryption | Pending | TBD |
| clap | 4.x | MIT/Apache-2.0 | Command-line argument parsing | Pending | TBD |
| nvml-wrapper | 0.x | MIT | NVIDIA GPU management | Pending | TBD |
| sysinfo | 0.x | MIT | System hardware information | Pending | TBD |
| regex | 1.x | MIT/Apache-2.0 | Regular expressions for parsing | Pending | TBD |
| reqwest | 0.x | MIT/Apache-2.0 | HTTP client for API calls | Pending | TBD |

### Development Dependencies
| Crate Name | Version | License | Purpose | Security Review Status | Last Updated |
|------------|---------|---------|---------|------------------------|--------------|
| cargo-audit | latest | MIT/Apache-2.0 | Security vulnerability scanning | N/A | TBD |
| cargo-deny | latest | MIT/Apache-2.0 | License and security compliance | N/A | TBD |
| tonic-build | 0.x | MIT | gRPC code generation | Pending | TBD |

## C++ Dependencies

### Core Client Dependencies  
| Library Name | Version | License | Purpose | Security Review Status | Last Updated |
|--------------|---------|---------|---------|------------------------|--------------|
| Qt6 | 6.x | GPL v3/Commercial | GUI framework | Pending | TBD |
| gRPC | 1.x | Apache-2.0 | Client-daemon communication | Pending | TBD |
| Protocol Buffers | 3.x | BSD-3-Clause | API serialization | Pending | TBD |

### Build Dependencies
| Tool Name | Version | License | Purpose | Security Review Status | Last Updated |
|-----------|---------|---------|---------|------------------------|--------------|
| CMake | 3.x | BSD-3-Clause | Build system | N/A | TBD |
| clang-format | latest | Apache-2.0/LLVM | Code formatting | N/A | TBD |

## System Dependencies

### Runtime Requirements
| Component | Version | Purpose | Security Review Status | Last Updated |
|-----------|---------|---------|------------------------|--------------|
| NVIDIA Drivers | 470+ | GPU hardware access | Pending | TBD |
| AMD ROCm | 5.x+ | AMD GPU hardware access | Pending | TBD |
| CUDA SDK | 11.x+ | NVIDIA development | Pending | TBD |

### Development Tools
| Tool | Version | Purpose | Security Review Status | Last Updated |
|------|---------|---------|------------------------|--------------|
| Docker | 20.x+ | Containerization | Pending | TBD |
| Docker Compose | 2.x+ | Local development environment | Pending | TBD |
| Terraform | 1.x+ | Infrastructure as Code | Pending | TBD |
| kubectl | 1.x+ | Kubernetes cluster management | Pending | TBD |

## Security and Compliance

### Security Scanning Tools
| Tool | Version | License | Purpose | Status |
|------|---------|---------|---------|--------|
| trivy | latest | Apache-2.0 | Container vulnerability scanning | Required |
| cargo-audit | latest | MIT/Apache-2.0 | Rust dependency vulnerability scanning | Required |
| cppcheck | latest | GPL v3 | C++ static analysis | Optional |

### License Compliance
All dependencies must be compatible with the project's license requirements:
- **Approved Licenses**: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause
- **Conditional Approval**: GPL v3 (for optional components only)
- **Prohibited Licenses**: GPL v2, AGPL, proprietary licenses

## Dependency Management Process

### Adding New Dependencies
1. Propose dependency with justification in GitHub issue
2. Security Lead performs security assessment
3. Technical review by Lead Principal Engineer
4. Update this document with approval status
5. Add dependency to appropriate build files

### Updating Dependencies
1. Evaluate security implications of update
2. Test in development environment
3. Run full security scan suite
4. Update version in this document
5. Deploy to staging for validation

### Vulnerability Response
1. Automated scanning detects vulnerability
2. Assess severity and impact
3. Develop mitigation plan
4. Test fixes in development environment
5. Deploy fix to production
6. Update documentation

## Notes

- This document is updated as part of Phase 0 Task 0.2 (Technology PoCs)
- All version numbers are placeholders and will be finalized during PoC validation
- Security review status will be updated as each dependency is evaluated
- Dependencies marked as "Required" are mandatory for CI/CD pipeline

---

*This document is part of the BUNKER MINER project governance framework and must be kept up-to-date as dependencies are added, updated, or removed.*