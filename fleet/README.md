# BUNKER MINER Fleet Controller

A centralized web dashboard and controller API for managing multiple BUNKER MINER rigs.

## Features

- **Web Dashboard**: Modern Vue.js-based interface for fleet monitoring
- **Real-time Telemetry**: WebSocket-based live data streaming
- **Secure Authentication**: JWT-based user authentication with API keys
- **Multi-Rig Management**: Support for unlimited rigs per user
- **Professional UI**: Responsive design with real-time metrics
- **RESTful API**: Complete API for rig management and telemetry

## Architecture

### Backend
- **Rust**: High-performance backend with axum web framework
- **PostgreSQL**: Robust database for user and rig data
- **Redis**: Session management and WebSocket state
- **WebSocket**: Real-time communication with rigs and dashboard

### Frontend
- **Vue.js 3**: Modern reactive frontend framework
- **WebSocket Client**: Real-time telemetry updates
- **Responsive Design**: Works on desktop, tablet, and mobile

## Quick Start

### Prerequisites
- Rust 1.75+
- PostgreSQL 13+
- Redis 6+
- Docker (for containerized deployment)

### Local Development

1. **Database Setup**
   ```bash
   createdb bunker_fleet
   ```

2. **Environment Variables**
   ```bash
   export DATABASE_URL="postgresql://bunker:bunker@localhost:5432/bunker_fleet"
   export REDIS_URL="redis://localhost:6379"
   export JWT_SECRET="your-secure-jwt-secret"
   ```

3. **Build and Run**
   ```bash
   cargo build --release
   cargo run
   ```

4. **Access Dashboard**
   Open http://localhost:8080/dashboard

### Docker Deployment

```bash
# Build image
docker build -t bunker/fleet-controller:latest .

# Run with docker-compose
docker-compose up -d
```

### Kubernetes Deployment

```bash
# Deploy to cluster
./deploy.sh
```

## API Endpoints

### Authentication
- `POST /api/auth/register` - Register new user
- `POST /api/auth/login` - User login
- `POST /api/auth/refresh` - Refresh JWT token

### Fleet Management
- `GET /api/fleet/rigs` - List user's rigs
- `POST /api/fleet/rigs` - Register new rig
- `GET /api/fleet/rigs/{id}` - Get rig details
- `PUT /api/fleet/rigs/{id}` - Update rig
- `DELETE /api/fleet/rigs/{id}` - Delete rig
- `GET /api/fleet/telemetry` - Get telemetry data

### API Keys
- `GET /api/user/api-keys` - List API keys
- `POST /api/user/api-keys` - Create API key
- `DELETE /api/user/api-keys/{id}` - Revoke API key

### WebSocket Endpoints
- `WS /api/fleet/ws` - Rig connection endpoint
- `WS /api/fleet/dashboard/ws` - Dashboard real-time updates

## Rig Integration

### Authentication
Rigs authenticate using API keys generated through the dashboard:

```javascript
// WebSocket connection with API key
const ws = new WebSocket('ws://fleet.bunker.local/api/fleet/ws?api_key=bk_...');
```

### Message Format
```json
{
  "type": "Telemetry",
  "rig_id": "uuid",
  "data": {
    "algorithm": "ethash",
    "total_hashrate": 150000000,
    "total_power": 800,
    "avg_temperature": 65,
    "device_count": 4,
    "shares_accepted": 1250,
    "shares_rejected": 3,
    "pool_url": "stratum+tcp://pool.bunker.local:4444",
    "device_telemetry": [...]
  }
}
```

## Security

### Authentication
- JWT tokens for web users (24-hour expiry)
- API keys for rig authentication (Argon2 hashed)
- Secure password hashing with Argon2

### Network Security
- TLS/SSL encryption for all communications
- WebSocket Secure (WSS) for real-time data
- CORS protection with allowlist

### Authorization
- User-scoped rig access
- Permission-based API key system
- Rate limiting and request validation

## Configuration

### Environment Variables
| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Server bind address |
| `SERVER_PORT` | `8080` | Server port |
| `DATABASE_URL` | - | PostgreSQL connection string |
| `REDIS_URL` | - | Redis connection string |
| `JWT_SECRET` | - | JWT signing secret |
| `JWT_EXPIRATION` | `86400` | Token expiry (seconds) |
| `ENVIRONMENT` | `development` | Runtime environment |

## Database Schema

### Users
- User accounts with email/password authentication
- Created/updated timestamps
- Active/inactive status

### Rigs
- Rig registration and metadata
- Owner association and status tracking
- Location and description fields

### API Keys
- Secure key generation and storage
- Permission-based access control
- Expiration and usage tracking

### Telemetry
- Real-time and historical rig data
- Per-device telemetry storage
- Automatic cleanup of old data

## Monitoring

### Health Checks
- `GET /api/health` - Service health status
- Database connectivity verification
- WebSocket connection statistics

### Metrics
- Active rig connections
- Dashboard sessions
- Telemetry data rates
- API response times

## Development

### Project Structure
```
fleet/
├── src/
│   ├── auth.rs         # Authentication & JWT
│   ├── database.rs     # Database connection
│   ├── error.rs        # Error handling
│   ├── handlers/       # API handlers
│   ├── models.rs       # Data models
│   ├── websocket.rs    # WebSocket manager
│   └── main.rs         # Application entry
├── migrations/         # Database migrations
├── static/            # Dashboard HTML/CSS/JS
├── k8s/              # Kubernetes manifests
└── Dockerfile        # Container build
```

### Testing
```bash
# Run tests
cargo test

# Integration tests
cargo test --test integration
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure security review for authentication changes
5. Submit pull request

## License

MIT License - see LICENSE file for details