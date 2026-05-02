# BUNKER POOL - Architecture Design Document

## 2026 Product Reality Check

This document describes the long-term centralized BUNKER POOL platform. It is not the immediate product path for the local miner release.

Current decision: BUNKER should ship pool integration in this order:

1. Direct public pool validation through verified XMRig.
2. External P2Pool mode for non-custodial Monero pool mining.
3. Managed P2Pool supervision after installer trust and process lifecycle gates are complete.
4. Managed `monerod` after node sync, RPC/ZMQ, disk, privacy, and verification gates are complete.
5. Centralized BUNKER POOL only after Stratum correctness, share accounting, payout wallet controls, abuse handling, observability, and legal/operational review.

The current `pool/` crate remains quarantined and must not be used for production claims until promoted through `docs/PRODUCT_IMPLEMENTATION_TRACKER.md`. The actionable near-term pool specification is `docs/specs/BUNKER_P2POOL_MODE.md`.

This document outlines the comprehensive architecture for the BUNKER POOL mining pool infrastructure, covering all components from the Stratum server to payout processing.

## Executive Summary

BUNKER POOL is a proprietary mining pool designed to serve as the default backend for BUNKER MINER clients while providing competitive services to the broader mining community. The architecture emphasizes security, scalability, and transparency.

### Key Design Goals
- **High Performance**: Support for 10,000+ concurrent miners
- **Security First**: Defense-in-depth security architecture
- **Transparency**: Open payout calculations and pool statistics
- **Scalability**: Horizontal scaling for growth
- **Reliability**: 99.9% uptime with automated failover

## System Overview

### High-Level Architecture
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   BUNKER MINER  │    │    Web Portal    │    │  Mobile Apps    │
│     Clients     │    │    (React SPA)   │    │   (Flutter)     │
└─────────┬───────┘    └─────────┬────────┘    └─────────┬───────┘
          │                      │                       │
          │              ┌───────┴───────┐               │
          │              │  Load Balancer │               │
          │              │   (AWS ALB)    │               │
          └──────────────┼────────────────┴───────────────┘
                         │
                ┌────────┴─────────┐
                │  API Gateway     │
                │  (Kong/Nginx)    │
                └────────┬─────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
┌───────▼───────┐ ┌──────▼──────┐ ┌──────▼──────┐
│ Stratum Server │ │  Public API │ │ Fleet API   │
│   (Rust/Tokio) │ │ (Rust/Axum) │ │(Rust/WS)    │
└───────┬───────┘ └──────┬──────┘ └──────┬──────┘
        │                │               │
        └────────────────┼───────────────┘
                         │
            ┌────────────┴─────────────┐
            │     Message Bus          │
            │    (Redis Streams)       │
            └────────────┬─────────────┘
                         │
    ┌────────────────────┼─────────────────────┐
    │                    │                     │
┌───▼───┐        ┌───────▼────────┐    ┌──────▼──────┐
│ Share │        │    Payout      │    │   Block     │
│Processor│        │    Engine      │    │  Watcher    │
│(Rust) │        │   (Rust)       │    │  (Rust)     │
└───┬───┘        └───────┬────────┘    └──────┬──────┘
    │                    │                     │
    └────────────────────┼─────────────────────┘
                         │
                ┌────────▼─────────┐
                │   PostgreSQL     │
                │   (Primary DB)   │
                └──────────────────┘
                         │
                ┌────────▼─────────┐
                │      Redis       │
                │   (Cache/Queue)  │
                └──────────────────┘
```

## Core Components

### 1. Stratum Server
**Purpose**: High-performance TCP server handling miner connections and work distribution

#### Technical Specifications
- **Language**: Rust with Tokio async runtime
- **Protocol**: Stratum v1 (with v2 planned)
- **Concurrency**: Support for 10,000+ concurrent connections
- **Performance Target**: <1ms job distribution latency

#### Key Features
- **Multi-Algorithm Support**: Kaspa, Ethash, EtcHash, RandomX
- **Dynamic Difficulty Adjustment**: Per-miner difficulty optimization
- **Connection Pooling**: Efficient resource management
- **Rate Limiting**: Protection against DoS attacks

#### Architecture
```rust
struct StratumServer {
    listener: TcpListener,
    miners: HashMap<MinerId, MinerConnection>,
    work_dispatcher: WorkDispatcher,
    share_validator: ShareValidator,
}

struct MinerConnection {
    socket: TcpStream,
    subscription: Subscription,
    difficulty: u64,
    last_activity: Instant,
}
```

### 2. Share Processing Backend
**Purpose**: Validate submitted shares and record mining contributions

#### Processing Pipeline
1. **Share Reception**: Receive shares from Stratum server via message bus
2. **Validation**: Cryptographic validation of proof-of-work
3. **Difficulty Check**: Verify share meets required difficulty
4. **Duplicate Detection**: Prevent duplicate share submission
5. **Database Storage**: Record valid shares with miner attribution

#### Performance Requirements
- **Throughput**: 10,000 shares/second processing capacity
- **Validation Time**: <10ms per share validation
- **Storage**: Efficient batch writes to PostgreSQL

#### Data Schema
```sql
CREATE TABLE shares (
    id BIGSERIAL PRIMARY KEY,
    miner_address VARCHAR(42) NOT NULL,
    algorithm VARCHAR(20) NOT NULL,
    difficulty NUMERIC(20,0) NOT NULL,
    block_height BIGINT NOT NULL,
    timestamp TIMESTAMP DEFAULT NOW(),
    is_block BOOLEAN DEFAULT FALSE,
    nonce VARCHAR(64),
    hash VARCHAR(64)
);

CREATE INDEX idx_shares_miner_time ON shares (miner_address, timestamp);
CREATE INDEX idx_shares_algorithm_height ON shares (algorithm, block_height);
```

### 3. Payout Engine
**Purpose**: Calculate and distribute mining rewards using PPLNS scheme

#### PPLNS Implementation
- **N Value**: Configurable N value per algorithm (typically 2x network difficulty)
- **Share Window**: Rolling window of last N shares for reward calculation
- **Proportional Distribution**: Rewards distributed proportional to share contribution
- **Fee Deduction**: Configurable pool fee (1-3% depending on algorithm)

#### Payout Process
1. **Block Detection**: Monitor blockchain for blocks found by pool
2. **Reward Calculation**: Calculate individual miner rewards using PPLNS
3. **Fee Deduction**: Apply pool fees and operational costs
4. **Balance Update**: Update miner account balances
5. **Payment Execution**: Batch payments to miner wallets
6. **Confirmation Tracking**: Monitor payment confirmations

#### Database Schema
```sql
CREATE TABLE blocks (
    id BIGSERIAL PRIMARY KEY,
    algorithm VARCHAR(20) NOT NULL,
    height BIGINT NOT NULL,
    hash VARCHAR(64) NOT NULL,
    reward NUMERIC(20,8) NOT NULL,
    timestamp TIMESTAMP DEFAULT NOW(),
    status VARCHAR(20) DEFAULT 'pending'
);

CREATE TABLE payouts (
    id BIGSERIAL PRIMARY KEY,
    block_id BIGINT REFERENCES blocks(id),
    miner_address VARCHAR(42) NOT NULL,
    amount NUMERIC(20,8) NOT NULL,
    fee NUMERIC(20,8) NOT NULL,
    tx_hash VARCHAR(64),
    status VARCHAR(20) DEFAULT 'pending'
);
```

### 4. Hot Wallet Security
**Purpose**: Secure management of pool funds for automatic payouts

#### Security Architecture
- **Multi-Signature Wallets**: 2-of-3 multi-sig for major funds
- **Hot/Cold Separation**: Limited funds in hot wallet for automated payouts
- **Automated Limits**: Daily/hourly payout limits with manual approval for large amounts
- **Audit Trail**: Complete transaction logging and monitoring

#### Risk Management
- **Balance Monitoring**: Automated alerts for unusual balance changes
- **Transaction Limits**: Per-transaction and daily limits
- **Manual Approval**: Human approval required for transactions >$10,000
- **Cold Storage**: 90% of funds stored in offline cold storage

#### Implementation
```rust
struct HotWallet {
    private_key: SecretKey,
    balance_tracker: BalanceTracker,
    transaction_limits: TransactionLimits,
    approval_queue: Vec<PendingTransaction>,
}

struct PendingTransaction {
    to_address: String,
    amount: Decimal,
    approval_status: ApprovalStatus,
    created_at: DateTime<Utc>,
}
```

### 5. Public API & Statistics
**Purpose**: Provide transparent pool statistics and miner account information

#### API Endpoints
```
GET /api/v1/pool/stats
GET /api/v1/pool/{algorithm}/stats
GET /api/v1/miner/{address}/stats
GET /api/v1/miner/{address}/payments
GET /api/v1/blocks/recent
GET /api/v1/blocks/{algorithm}/recent
```

#### Real-Time Features
- **WebSocket API**: Real-time pool statistics and miner updates
- **Live Hashrate**: Real-time pool and miner hashrate display
- **Share Statistics**: Live share acceptance rates and difficulty changes
- **Block Notifications**: Real-time block finding notifications

#### Performance Requirements
- **Response Time**: <100ms for all API endpoints
- **WebSocket Capacity**: Support 1,000+ concurrent connections
- **Rate Limiting**: 100 requests/minute per IP for public endpoints

## Database Architecture

### PostgreSQL Schema Design

#### Core Tables
```sql
-- Miner accounts and configuration
CREATE TABLE miners (
    address VARCHAR(42) PRIMARY KEY,
    email VARCHAR(255),
    notification_settings JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    last_seen TIMESTAMP,
    total_shares BIGINT DEFAULT 0,
    total_earned NUMERIC(20,8) DEFAULT 0
);

-- Real-time miner statistics
CREATE TABLE miner_stats (
    miner_address VARCHAR(42) REFERENCES miners(address),
    algorithm VARCHAR(20),
    timestamp TIMESTAMP,
    hashrate BIGINT,
    shares_per_hour INTEGER,
    efficiency NUMERIC(10,4),
    PRIMARY KEY (miner_address, algorithm, timestamp)
);

-- Pool statistics aggregation
CREATE TABLE pool_stats (
    algorithm VARCHAR(20),
    timestamp TIMESTAMP,
    total_hashrate BIGINT,
    total_miners INTEGER,
    shares_per_hour INTEGER,
    network_difficulty NUMERIC(20,0),
    estimated_time_to_block INTEGER,
    PRIMARY KEY (algorithm, timestamp)
);
```

#### Performance Optimization
- **Partitioning**: Time-based partitioning for shares and statistics tables
- **Indexing**: Optimized indexes for common query patterns
- **Archival**: Automated archival of old data to reduce active dataset size
- **Read Replicas**: Dedicated read replicas for analytics and reporting

### Redis Architecture

#### Caching Strategy
- **Pool Statistics**: Cache frequently requested pool stats (5-minute TTL)
- **Miner Statistics**: Cache individual miner stats (1-minute TTL)
- **Share Buffering**: Buffer incoming shares before batch database writes
- **Rate Limiting**: Store rate limiting counters and windows

#### Message Queues
- **Share Processing**: Queue for validated shares awaiting database storage
- **Payout Processing**: Queue for pending payout calculations
- **Notification Queue**: Queue for email/push notifications

## Security Architecture

### Network Security
- **DDoS Protection**: CloudFlare or AWS Shield for DDoS mitigation
- **Rate Limiting**: Multiple layers of rate limiting for different attack vectors
- **IP Whitelisting**: Optional IP restrictions for high-value accounts
- **Geographic Filtering**: Optional country-based access controls

### Application Security
- **Input Validation**: Comprehensive validation of all user inputs
- **SQL Injection Prevention**: Parameterized queries and ORM usage
- **Authentication**: JWT-based authentication for API access
- **Authorization**: Role-based access control for administrative functions

### Data Protection
- **Encryption at Rest**: Database encryption using PostgreSQL TDE
- **Encryption in Transit**: TLS 1.3 for all network communications
- **PII Protection**: Minimal collection and secure handling of personal information
- **Backup Encryption**: Encrypted database backups with secure key management

### Operational Security
- **Monitoring**: Comprehensive logging and monitoring of security events
- **Incident Response**: Defined procedures for security incident handling
- **Access Control**: Least-privilege access for all system components
- **Regular Audits**: Quarterly security audits and penetration testing

## Scalability and Performance

### Horizontal Scaling Strategy
- **Stateless Services**: All application services designed to be stateless
- **Load Balancing**: Dynamic load balancing across multiple instances
- **Database Sharding**: Algorithm-based sharding for high-volume tables
- **CDN Integration**: Static content delivery via CDN

### Performance Optimization
- **Connection Pooling**: Efficient database connection management
- **Batch Processing**: Batch operations for high-volume data writes
- **Async Processing**: Non-blocking I/O for all network operations
- **Caching Layers**: Multi-tier caching for frequently accessed data

### Monitoring and Observability
- **Metrics Collection**: Comprehensive metrics via Prometheus
- **Log Aggregation**: Centralized logging via ELK stack
- **Distributed Tracing**: Request tracing for performance debugging
- **Alerting**: Automated alerting for performance and security issues

## Deployment Architecture

### Container Orchestration
- **Kubernetes**: Production deployment on managed Kubernetes (EKS)
- **Service Mesh**: Istio for service communication and security
- **Auto Scaling**: Horizontal pod autoscaling based on metrics
- **Rolling Updates**: Zero-downtime deployments

### Infrastructure as Code
```hcl
# AWS EKS cluster for production deployment
module "bunker_pool_cluster" {
  source = "./modules/eks-cluster"
  
  cluster_name = "bunker-pool-prod"
  node_groups = {
    general = {
      instance_types = ["m5.xlarge"]
      min_size      = 3
      max_size      = 10
      desired_size  = 5
    }
    high_memory = {
      instance_types = ["r5.2xlarge"]  
      min_size      = 2
      max_size      = 5
      desired_size  = 3
    }
  }
}

# RDS PostgreSQL for primary database
resource "aws_db_instance" "primary_db" {
  identifier = "bunker-pool-primary"
  engine     = "postgres"
  engine_version = "14.6"
  instance_class = "db.r5.2xlarge"
  storage_type   = "gp3"
  allocated_storage = 1000
  
  multi_az = true
  backup_retention_period = 30
  backup_window = "03:00-04:00"
  
  performance_insights_enabled = true
  monitoring_interval = 60
}

# ElastiCache Redis for caching and queues
resource "aws_elasticache_replication_group" "redis_cluster" {
  replication_group_id = "bunker-pool-redis"
  description         = "Redis cluster for BUNKER POOL"
  
  node_type = "cache.r6g.xlarge"
  port      = 6379
  
  num_cache_clusters = 3
  automatic_failover_enabled = true
  multi_az_enabled = true
  
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
}
```

## Disaster Recovery

### Backup Strategy
- **Database Backups**: Automated daily backups with 30-day retention
- **Point-in-Time Recovery**: 5-minute RPO with transaction log shipping
- **Configuration Backups**: Version-controlled infrastructure configuration
- **Application Backups**: Container image registry with historical versions

### High Availability
- **Multi-AZ Deployment**: Database and cache clusters across multiple zones
- **Service Redundancy**: Multiple instances of all critical services
- **Health Checks**: Automated health monitoring and failover
- **Circuit Breakers**: Failure isolation to prevent cascade failures

### Recovery Procedures
- **RTO Target**: 15 minutes for critical services
- **RPO Target**: 5 minutes for transaction data
- **Runbook**: Detailed procedures for common failure scenarios
- **Testing**: Monthly DR testing and validation

## Compliance and Governance

### Data Governance
- **Data Classification**: Classification of all data types and sensitivity levels
- **Retention Policies**: Automated data retention and deletion policies
- **Access Controls**: Strict access controls for sensitive data
- **Audit Trails**: Comprehensive audit logging for compliance

### Regulatory Compliance
- **GDPR Compliance**: EU data protection regulation compliance
- **KYC/AML**: Know Your Customer and Anti-Money Laundering procedures
- **Financial Regulations**: Compliance with relevant financial regulations
- **Regular Audits**: Annual third-party security and compliance audits

## Integration with BUNKER MINER

### Default Pool Integration
- **Seamless Configuration**: BUNKER POOL as default option in client
- **Optimized Protocol**: Custom optimizations for BUNKER MINER clients
- **Enhanced Features**: Advanced features exclusive to BUNKER MINER users
- **Unified Experience**: Single sign-on and unified account management

### API Integration
- **Real-Time Telemetry**: Enhanced telemetry for BUNKER MINER users
- **Advanced Analytics**: Detailed mining analytics and optimization suggestions
- **Fleet Management**: Integration with multi-rig management features
- **Marketplace Integration**: Connection to hashpower marketplace features

## Future Enhancements

### Phase 3 Deliverables (Current)
- Core Stratum server and share processing
- Basic PPLNS payout system
- Public API for pool statistics
- Integration with BUNKER MINER daemon

### Phase 4 Enhancements
- Advanced fleet management features
- Enhanced security controls
- Performance optimizations
- Mobile application support

### Phase 5 Enhancements
- Marketplace integration for hashpower trading
- Advanced analytics and machine learning
- Multi-coin support expansion
- Community features and social mining

---

*This architecture document serves as the blueprint for BUNKER POOL development and will be updated as the system evolves through each development phase.*
