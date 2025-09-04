# ADR-004: Daemon API Security Design and Threat Model

## Status
**ACCEPTED** - 2025-01-09

## Context

The BUNKER MINER daemon API (`daemon_api.v1.proto`) serves as the primary communication interface between the Rust daemon backend and C++/Qt client applications. This API handles sensitive data including real-time telemetry, mining configurations, wallet addresses, and system control operations. A comprehensive security design review and threat model is essential to ensure the API is secure by design against common attack vectors.

## Decision

**Primary Security Architecture:**
- **Default Binding**: localhost-only (127.0.0.1) for maximum security
- **Remote Access**: Optional TLS with client certificate authentication
- **Rate Limiting**: Per-endpoint and per-client limits to prevent abuse
- **Input Validation**: Comprehensive validation rules embedded in Protocol Buffer schema
- **Authentication**: API key or certificate-based for remote connections
- **Authorization**: Role-based access control for administrative operations

## STRIDE Threat Model Analysis

### Spoofing (Identity Verification)

#### Threat: Malicious Client Impersonation
- **Description**: Attacker attempts to connect to daemon pretending to be legitimate client
- **Impact**: Unauthorized access to mining operations and sensitive data
- **Likelihood**: Medium (requires network access to daemon)

**Mitigations Implemented:**
- ✅ **Localhost Binding**: Default binding to 127.0.0.1 prevents network-based attacks
- ✅ **Client Certificates**: Mutual TLS authentication for remote connections
- ✅ **API Keys**: Cryptographically secure API key validation
- ✅ **Connection Logging**: All connection attempts logged with source IP and authentication status

#### Threat: Daemon Impersonation
- **Description**: Attacker runs fake daemon to intercept client connections
- **Impact**: Credential theft, malicious mining configuration injection
- **Likelihood**: Low (requires local system access)

**Mitigations Implemented:**
- ✅ **Server Certificates**: TLS server certificates for daemon identity verification
- ✅ **Service Discovery**: Client validates daemon process and PID
- ✅ **Certificate Pinning**: Clients can pin expected daemon certificate

### Tampering (Data Integrity)

#### Threat: API Message Tampering
- **Description**: Man-in-the-middle attacker modifies API messages in transit
- **Impact**: Malicious configuration changes, false telemetry data
- **Likelihood**: Medium (if TLS not properly implemented)

**Mitigations Implemented:**
- ✅ **TLS Encryption**: All remote communications encrypted with TLS 1.3
- ✅ **Message Authentication**: TLS provides built-in message authentication codes
- ✅ **Protocol Buffer Integrity**: Binary serialization with built-in consistency checks
- ✅ **Configuration Checksums**: Configuration changes include integrity verification

#### Threat: Persistent Configuration Tampering
- **Description**: Attacker modifies daemon configuration files on disk
- **Impact**: Permanent compromise of daemon security settings
- **Likelihood**: Medium (requires file system access)

**Mitigations Implemented:**
- ✅ **File Permissions**: Restrictive permissions (600) on configuration files
- ✅ **Configuration Encryption**: Sensitive configuration data encrypted at rest (age encryption)
- ✅ **Integrity Monitoring**: Configuration file hash verification on startup
- ✅ **Backup Validation**: Configuration backup integrity verification

### Repudiation (Non-Repudiation)

#### Threat: Administrative Action Denial
- **Description**: Admin denies performing sensitive operations (start/stop mining, config changes)
- **Impact**: Difficulty in forensic analysis and accountability
- **Likelihood**: Low (internal threat)

**Mitigations Implemented:**
- ✅ **Comprehensive Audit Logging**: All administrative operations logged with timestamps and user identity
- ✅ **Immutable Logs**: Log files protected from tampering with cryptographic signatures
- ✅ **Operation Context**: Logs include full context of operations (what changed, from/to values)
- ✅ **External Log Shipping**: Option to ship logs to external SIEM systems

### Information Disclosure (Confidentiality)

#### Threat: Sensitive Data Exposure via API
- **Description**: API inadvertently exposes sensitive information like wallet addresses, API keys, file paths
- **Impact**: Financial loss, privacy violation, system reconnaissance
- **Likelihood**: High (common API design flaw)

**Mitigations Implemented:**
- ✅ **Data Classification**: All API fields classified by sensitivity level (PUBLIC/INTERNAL/CONFIDENTIAL/RESTRICTED)
- ✅ **Minimal Data Exposure**: API only exposes operationally necessary data
- ✅ **Sensitive Data Filtering**: Wallet addresses and API keys never appear in logs or error messages
- ✅ **Field-Level Security**: Protocol buffer validation rules prevent data leakage

#### Threat: Telemetry Data Interception
- **Description**: Attacker intercepts real-time mining telemetry data
- **Impact**: Mining operation intelligence, performance data theft
- **Likelihood**: Medium (valuable for competitors)

**Mitigations Implemented:**
- ✅ **TLS Encryption**: All telemetry streams encrypted in transit
- ✅ **Connection Limits**: Maximum concurrent telemetry streams per client
- ✅ **Data Minimization**: Telemetry includes only essential operational data
- ✅ **Stream Authentication**: Long-lived streams require continuous authentication validation

### Denial of Service (Availability)

#### Threat: API Request Flooding
- **Description**: Malicious client overwhelms daemon with excessive API requests
- **Impact**: Daemon becomes unresponsive, mining operations disrupted
- **Likelihood**: High (easy to execute)

**Mitigations Implemented:**
- ✅ **Per-Endpoint Rate Limiting**: Granular rate limits for each API endpoint based on expected usage
- ✅ **Per-Client Rate Limiting**: Individual client request quotas to prevent single-client abuse
- ✅ **Connection Limits**: Maximum concurrent connections per source IP
- ✅ **Circuit Breakers**: Automatic client disconnection on rate limit violations

#### Threat: Streaming Connection Exhaustion
- **Description**: Attacker opens many telemetry streams to exhaust daemon resources
- **Impact**: Legitimate clients cannot establish connections
- **Likelihood**: Medium (requires some technical knowledge)

**Mitigations Implemented:**
- ✅ **Stream Connection Limits**: Maximum concurrent streams per client (typically 1)
- ✅ **Stream Timeout**: Automatic disconnection of inactive streams
- ✅ **Resource Monitoring**: Memory and CPU usage monitoring for stream management
- ✅ **Graceful Degradation**: Daemon prioritizes essential operations over streaming when under load

#### Threat: Resource-Intensive Operations
- **Description**: Attacker triggers computationally expensive operations
- **Impact**: Daemon performance degradation, mining efficiency reduction
- **Likelihood**: Low (limited expensive operations in API)

**Mitigations Implemented:**
- ✅ **Operation Timeouts**: All operations have maximum execution time limits
- ✅ **Async Processing**: Heavy operations processed asynchronously to prevent blocking
- ✅ **Resource Quotas**: Memory and CPU limits for API request processing
- ✅ **Priority Queuing**: Mining operations prioritized over API requests under load

### Elevation of Privilege (Authorization)

#### Threat: Unauthorized Administrative Operations
- **Description**: Standard user gains access to administrative operations (config changes, mining control)
- **Impact**: System compromise, unauthorized configuration changes
- **Likelihood**: Medium (common authorization flaw)

**Mitigations Implemented:**
- ✅ **Role-Based Access Control**: Distinct roles for read-only users vs administrators
- ✅ **Operation-Level Authorization**: Each API endpoint checks user permissions
- ✅ **Principle of Least Privilege**: Users granted minimum necessary permissions
- ✅ **Administrative Operation Confirmation**: Multi-step confirmation for destructive operations

#### Threat: API Key Compromise
- **Description**: Attacker obtains valid API key through various means
- **Impact**: Unauthorized access with legitimate credentials
- **Likelihood**: Medium (depends on key management practices)

**Mitigations Implemented:**
- ✅ **Key Rotation**: Regular API key rotation with automated key management
- ✅ **Key Scope Limitation**: API keys limited to specific operations and time periods
- ✅ **Anomaly Detection**: Unusual API usage patterns trigger alerts
- ✅ **Key Revocation**: Immediate key revocation capability for compromised keys

## Security Controls Implementation

### Transport Security
```yaml
TLS Configuration:
  - Protocol: TLS 1.3 minimum
  - Cipher Suites: AEAD ciphers only (ChaCha20-Poly1305, AES-GCM)
  - Certificate Validation: Full certificate chain validation
  - Perfect Forward Secrecy: Required for all connections
  - HSTS: HTTP Strict Transport Security enabled
```

### Authentication Framework
```yaml
Authentication Methods:
  - Localhost: No authentication required (implicit trust)
  - Remote API Key: HMAC-SHA256 signed requests
  - Remote Certificate: X.509 client certificate authentication
  - Session Management: JWT tokens with short expiration
```

### Rate Limiting Matrix
| Endpoint | Rate Limit | Justification |
|----------|------------|---------------|
| `GetSystemInfo` | 10/second | Infrequent system queries |
| `HealthCheck` | 100/second | High-frequency monitoring |
| `StartMining` | 5/second | State changes require throttling |
| `StopMining` | 5/second | Emergency operations need higher limit |
| `StreamTelemetry` | 1 concurrent | Resource-intensive streaming |
| `GetProfitability` | 1/minute | Data updated infrequently |
| `GetConfig` | 10/second | Configuration queries |
| `SetConfig` | 1/second | Critical configuration changes |

### Input Validation Rules

#### Protocol Buffer Field Validation
- **String Fields**: Maximum length limits, character set restrictions
- **Numeric Fields**: Range validation, overflow protection
- **Repeated Fields**: Maximum element count limits
- **Map Fields**: Key/value validation, size limits
- **Timestamp Fields**: Reasonable time range validation

#### Business Logic Validation
- **Device IDs**: Must reference existing devices
- **Algorithm Names**: Must be supported algorithms
- **Wallet Addresses**: Cryptocurrency-specific format validation
- **Configuration Values**: Schema validation against known configuration structure

### Error Handling Security

#### Information Leakage Prevention
- **Generic Error Messages**: User-facing errors provide minimal information
- **Detailed Logging**: Full error context logged internally only
- **Stack Trace Suppression**: No stack traces in production API responses
- **Path Sanitization**: File paths never exposed in error messages

#### Error Response Structure
```protobuf
message ErrorDetails {
  string error_code = 1;        // Machine-readable error code
  string error_description = 2; // Safe, generic description
  repeated string remediation_steps = 4; // User-actionable steps
  // Note: No sensitive technical details exposed
}
```

## Performance and Security Trade-offs

### Security vs Performance Analysis

#### TLS Overhead
- **Impact**: ~5-10% CPU overhead for encryption/decryption
- **Mitigation**: Hardware-accelerated crypto when available
- **Justification**: Security benefit outweighs minimal performance cost

#### Rate Limiting Overhead
- **Impact**: ~1-2ms additional latency per request for rate limit checking
- **Mitigation**: In-memory rate limit counters with efficient data structures
- **Justification**: DoS protection essential for production deployments

#### Input Validation Overhead
- **Impact**: ~0.5-1ms per request for comprehensive validation
- **Mitigation**: Pre-compiled validation rules, efficient validation algorithms
- **Justification**: Critical for preventing injection and overflow attacks

## Deployment Security Configuration

### Production Deployment Checklist
- ✅ **TLS Certificates**: Valid certificates from trusted CA or internal PKI
- ✅ **Firewall Rules**: Restrict daemon port access to authorized clients only
- ✅ **Service Account**: Daemon runs under dedicated service account with minimal privileges
- ✅ **Log Monitoring**: Security events forwarded to SIEM system
- ✅ **Backup Security**: Configuration backups encrypted and access-controlled
- ✅ **Update Process**: Secure update mechanism with signature verification

### Security Monitoring and Alerting
```yaml
Security Events Monitored:
  - Authentication failures and suspicious patterns
  - Rate limit violations and potential DoS attacks
  - Configuration changes and administrative operations
  - TLS handshake failures and certificate issues
  - Unusual API usage patterns and anomalies
  - System resource exhaustion and performance degradation
```

## Security Testing Requirements

### Automated Security Testing
- **Static Analysis**: Protocol buffer schema validation and security linting
- **Dynamic Testing**: API fuzzing with malformed inputs and boundary conditions
- **Penetration Testing**: Simulated attacks against all identified threat vectors
- **Load Testing**: DoS resistance validation under high request volumes

### Security Test Cases
1. **Authentication Bypass**: Verify all endpoints properly authenticate remote requests
2. **Authorization Escalation**: Confirm role-based access controls prevent privilege escalation
3. **Input Validation**: Test all validation rules with malicious and edge case inputs
4. **Rate Limit Effectiveness**: Validate rate limiting prevents DoS attacks
5. **TLS Configuration**: Verify TLS settings match security requirements
6. **Error Handling**: Confirm no sensitive information leaked in error responses

## Incident Response Considerations

### Security Incident Classification
- **P1 Critical**: Remote code execution, credential compromise, data breach
- **P2 High**: DoS attacks, authentication bypass, privilege escalation
- **P3 Medium**: Information disclosure, configuration tampering
- **P4 Low**: Rate limiting bypass, minor information leakage

### Response Procedures
1. **Detection**: Automated monitoring alerts on security events
2. **Assessment**: Rapid triage to determine impact and severity
3. **Containment**: Immediate measures to prevent further compromise
4. **Investigation**: Forensic analysis and root cause determination
5. **Recovery**: Service restoration and security hardening
6. **Lessons Learned**: Post-incident review and security improvements

## Compliance and Regulatory Considerations

### Data Privacy
- **Personal Data**: No personally identifiable information processed or stored
- **Financial Data**: Wallet addresses treated as confidential information
- **Operational Data**: Mining telemetry considered business-sensitive
- **Audit Requirements**: All security events logged for compliance auditing

### Security Standards Alignment
- **OWASP Top 10**: All identified risks addressed with appropriate controls
- **NIST Cybersecurity Framework**: Implementation aligned with framework guidelines
- **ISO 27001**: Security controls designed for potential ISO 27001 compliance
- **Industry Standards**: Cryptocurrency and mining industry security best practices

## Future Security Enhancements

### Phase 1 Security Roadmap
- **Advanced Monitoring**: Machine learning-based anomaly detection
- **Zero Trust Architecture**: Enhanced authentication and authorization
- **API Gateway**: Centralized security policy enforcement
- **Encrypted Telemetry**: End-to-end encryption for telemetry streams

### Phase 2+ Security Vision
- **Hardware Security Module**: HSM integration for key management
- **Homomorphic Encryption**: Privacy-preserving telemetry analytics
- **Blockchain Integration**: Tamper-evident audit trails
- **Formal Verification**: Mathematical proof of security properties

## Review and Approval

**Security Architecture Review:** Security Lead - **APPROVED**  
*Comprehensive threat model analysis completed with appropriate mitigations identified for all major threat vectors.*

**API Design Review:** Lead Principal Engineer - **APPROVED**  
*API contract provides comprehensive functionality with security-by-design principles throughout.*

**Performance Impact Review:** Performance evaluation - **ACCEPTABLE**  
*Security controls designed to minimize performance impact while providing strong protection.*

**Implementation Readiness:** Development team - **READY**  
*All security requirements clearly defined with specific implementation guidelines.*

**Validation Criteria Met:**
- ✅ Comprehensive STRIDE threat model completed
- ✅ All identified threats have documented mitigations
- ✅ Security controls integrated into API design
- ✅ Rate limiting and validation rules defined
- ✅ Incident response procedures documented
- ✅ Security testing requirements specified

## References

- [STRIDE Threat Modeling](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [gRPC Security Guide](https://grpc.io/docs/guides/security/)
- [Protocol Buffers Security Best Practices](https://developers.google.com/protocol-buffers/docs/security)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- BUNKER MINER Protocol Buffer Schema: `protos/daemon_api.v1.proto`
- Security scanning results: Continuous integration security pipeline