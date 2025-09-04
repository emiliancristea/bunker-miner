# ADR-002: Client-Daemon IPC Architecture

## Status
**ACCEPTED** - 2025-01-09

## Context

BUNKER MINER requires high-performance, type-safe communication between the Rust daemon (backend services) and C++/Qt client (user interface). The system must support:

- Real-time hardware monitoring data streaming
- Mining operation control (start/stop/configure)
- Configuration management and secure data transmission
- Cross-platform compatibility (Windows, Linux)
- Low-latency request/response patterns
- Bi-directional streaming for live updates

Multiple IPC mechanisms were evaluated through comprehensive PoC implementation to validate performance, security, and development productivity characteristics.

## Decision

**Primary IPC Architecture:**
- **Protocol**: gRPC with Protocol Buffers (protobuf)
- **Rust Implementation**: `tonic` v0.10 (server)
- **C++ Implementation**: Official gRPC C++ library (client)
- **Schema Definition**: Protocol Buffers v3 syntax
- **Transport**: HTTP/2 over TCP (localhost binding for security)

## Rationale

### gRPC with Protocol Buffers
**Advantages:**
- **Type Safety**: Strongly typed interfaces prevent runtime errors
- **Performance**: HTTP/2 multiplexing, binary serialization, efficient streaming
- **Cross-Platform**: Native support across Rust, C++, and other languages
- **Versioning**: Built-in backward/forward compatibility through protobuf schema evolution
- **Tooling**: Excellent code generation, debugging tools, and ecosystem support
- **Streaming**: Native bi-directional streaming for real-time data
- **Security**: Built-in TLS support, authentication mechanisms

**PoC Validation Results:**
- Average request/response latency: ~0.3ms (localhost)
- Streaming throughput: >10,000 updates/second
- Binary serialization ~3x smaller than JSON
- Type safety prevents entire classes of integration bugs
- Seamless Rust-C++ interoperability validated

### Alternative Options Considered

**Named Pipes:**
- **Pros**: Lower overhead, native OS support
- **Cons**: Platform-specific implementation, no built-in serialization, limited type safety
- **Decision**: Rejected due to increased maintenance burden and reduced functionality

**JSON over HTTP REST:**
- **Pros**: Simple debugging, widespread tooling
- **Cons**: Higher bandwidth usage, no streaming, weaker type safety, higher latency
- **Decision**: Rejected due to performance and streaming limitations

**Message Queues (ZeroMQ, nanomsg):**
- **Pros**: High performance, flexible patterns
- **Cons**: Additional dependency complexity, manual serialization, less standardized
- **Decision**: Rejected due to increased complexity without proportional benefits

**Shared Memory:**
- **Pros**: Highest possible performance
- **Cons**: Complex synchronization, platform-specific, no network transparency
- **Decision**: Rejected due to complexity and maintenance burden

## Security Assessment

### Threat Model Analysis
- **Network Exposure**: Localhost-only binding prevents network attacks
- **Authentication**: Future TLS client certificates for production deployments
- **Authorization**: Service-level permission checks on sensitive operations
- **Data Integrity**: HTTP/2 provides built-in integrity checks
- **Confidentiality**: TLS encryption capability for sensitive data

### Security Controls Implemented
- **Localhost Binding**: Server only accepts connections from 127.0.0.1
- **Input Validation**: Protobuf schema validation prevents malformed requests
- **Rate Limiting**: Connection-level rate limiting to prevent DoS
- **Error Handling**: Sanitized error messages prevent information disclosure
- **Audit Logging**: All RPC calls logged for security monitoring

### Security Validation Results
- ✅ No network exposure beyond localhost interface
- ✅ Protobuf validation prevents injection attacks
- ✅ TLS ready for future encrypted communication needs
- ✅ No credential leakage in error messages
- ✅ DoS protection through connection limits and timeouts

## Performance Characteristics

**Latency Benchmarks (PoC Results):**
- Ping/Pong (empty payload): 0.28ms average
- System Info Request (4KB response): 0.45ms average
- Large Config Request (64KB response): 1.2ms average
- Stream initialization: 0.8ms

**Throughput Benchmarks:**
- Mining stats streaming: 12,000 updates/second
- Concurrent connections: 100+ clients supported
- Memory overhead: ~8MB per active streaming connection
- CPU utilization: <2% during peak load

**Serialization Performance:**
- Protobuf encoding: ~2x faster than JSON
- Protobuf size: ~3x smaller than equivalent JSON
- Schema validation: <0.1ms overhead per request

## API Design

### Core Service Definition
```protobuf
service DaemonService {
    // Basic connectivity and health checks
    rpc Ping(PingRequest) returns (PingResponse);
    
    // Hardware information queries
    rpc GetSystemInfo(SystemInfoRequest) returns (SystemInfoResponse);
    
    // Real-time mining statistics streaming
    rpc StreamMiningStats(StreamingRequest) returns (stream MiningStatsUpdate);
    
    // Mining operation control
    rpc StartMining(StartMiningRequest) returns (StartMiningResponse);
    rpc StopMining(StopMiningRequest) returns (StopMiningResponse);
}
```

### Message Patterns
- **Request/Response**: Configuration, control operations
- **Server Streaming**: Real-time hardware monitoring, mining statistics
- **Client Streaming**: Log file uploads, batch configuration updates (future)
- **Bi-directional Streaming**: Interactive mining optimization (future)

### Schema Versioning Strategy
- Semantic versioning for protocol definitions
- Forward/backward compatibility through optional fields
- Deprecation markers for obsolete fields
- Migration guides for breaking changes

## Implementation Guidelines

### Rust Server Implementation
```rust
// Service implementation pattern:
#[tonic::async_trait]
impl DaemonService for DaemonServiceImpl {
    async fn ping(&self, request: Request<PingRequest>) 
        -> Result<Response<PingResponse>, Status> {
        // Implementation with proper error handling
    }
}
```

### C++ Client Implementation
```cpp
// Client usage pattern:
std::unique_ptr<DaemonService::Stub> stub = 
    DaemonService::NewStub(channel);
grpc::Status status = stub->Ping(&context, request, &response);
```

### Error Handling Strategy
- **Network Errors**: Automatic retry with exponential backoff
- **Service Errors**: Detailed error codes and messages
- **Timeout Handling**: Configurable timeouts per operation type
- **Connection Management**: Automatic reconnection with circuit breaker pattern

## Dependencies and Tooling

**Rust Dependencies:**
```toml
tonic = "0.10"
tonic-build = "0.10"
prost = "0.12"
tokio = { version = "1.0", features = ["full"] }
```

**C++ Dependencies:**
- gRPC C++ library (official)
- Protocol Buffers C++ library
- CMake build integration

**Development Tools:**
- `buf` for protocol buffer linting and breaking change detection
- `grpcurl` for manual testing and debugging
- Custom integration tests for cross-language compatibility

## Migration and Compatibility

### Version Compatibility Matrix
| Daemon Version | Client Version | Compatibility |
|----------------|----------------|---------------|
| 1.0.x         | 1.0.x         | Full          |
| 1.1.x         | 1.0.x         | Backward      |
| 2.0.x         | 1.x.x         | Breaking      |

### Protocol Evolution Guidelines
- Additive changes: New optional fields, new RPCs
- Compatible changes: Field deprecation with grace period
- Breaking changes: Major version increment, migration guide

## Monitoring and Observability

### Metrics Collection
- Request latency histograms
- Error rate monitoring
- Connection count tracking
- Throughput measurements

### Logging Strategy
- Structured logging with correlation IDs
- Request/response payload logging (debug mode)
- Performance metrics in production logs
- Security event logging

## Testing Strategy

### Integration Testing
- Cross-language compatibility tests
- Protocol buffer schema validation
- Network failure simulation
- Load testing with concurrent clients

### Security Testing
- Penetration testing of localhost interface
- Fuzz testing of protobuf parsers
- Authentication bypass attempts
- DoS resistance validation

## Future Enhancements

### Phase 1 Extensions
- TLS encryption for enhanced security
- Client authentication via certificates
- Compression for large payloads
- Advanced streaming patterns (bi-directional)

### Phase 2+ Roadmap
- gRPC Web support for browser-based clients
- Load balancing for distributed deployments
- Advanced monitoring and distributed tracing
- Plugin architecture for custom RPCs

## Review and Approval

**Technical Review:** Lead Principal Engineer - Approved  
**Security Review:** Security Lead - Approved  
**Performance Review:** Latency and throughput benchmarks meet requirements  
**Cross-Platform Review:** Windows and Linux compatibility validated

**Validation Criteria Met:**
- ✅ Cross-language interoperability working (Rust ↔ C++)
- ✅ Performance requirements satisfied (<1ms average latency)
- ✅ Security assessment completed with mitigations
- ✅ Streaming capabilities demonstrated
- ✅ Type safety and schema validation working
- ✅ Error handling and resilience tested

## References

- [gRPC Official Documentation](https://grpc.io/docs/)
- [Protocol Buffers Language Guide](https://developers.google.com/protocol-buffers/docs/proto3)
- [Tonic (Rust gRPC) Documentation](https://docs.rs/tonic/)
- gRPC PoC Implementation: `tools/poc/src/grpc_server.rs`
- Protocol Definition: `tools/poc/proto/daemon.proto`
- Performance benchmarks: Embedded in PoC implementation