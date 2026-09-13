# axum-proxy

Production-quality async HTTP reverse proxy with load balancing, built with Axum and Tokio.

## Features

- **Load Balancing**: Round-robin, least-connections, and weighted algorithms
- **Health Checks**: Configurable intervals with automatic unhealthy backend detection
- **Rate Limiting**: Token bucket algorithm per client IP
- **Request Logging**: Method, URI, status, latency tracking
- **WebSocket Support**: Transparent WebSocket upgrade proxying
- **Connection Pooling**: Persistent connections to backends via reqwest
- **Header Manipulation**: Add/strip request headers, X-Forwarded-For, X-Real-IP, Via
- **CORS**: Configurable cross-origin resource sharing
- **Graceful Shutdown**: Handles SIGINT/SIGTERM cleanly
- **Health Dashboard**: Built-in web UI at `/__health`
- **Stats API**: JSON endpoint at `/__stats`

## Quick Start

```bash
# Build and run
cargo build --release
cargo run --release

# Or with custom config
PROXY_CONFIG=config.toml cargo run --release
```

## Configuration

Configuration is loaded in order of priority:

1. `PROXY_CONFIG` environment variable pointing to a TOML file
2. `config.toml` in the current directory
3. `proxy.toml` in the current directory
4. `PROXY_CONFIG_JSON` environment variable (JSON format)
5. Built-in defaults

### config.toml

```toml
[server]
bind_addr = "0.0.0.0:8080"
request_timeout_secs = 30
max_connections = 1024

[[backends]]
name = "backend-1"
url = "http://127.0.0.1:3001"
weight = 1
max_retries = 3

[load_balancer]
algorithm = "round_robin"  # round_robin | least_connections | weighted
sticky_sessions = false

[health_check]
enabled = true
interval_secs = 10
timeout_secs = 5
healthy_threshold = 2
unhealthy_threshold = 3

[rate_limit]
enabled = true
max_requests = 1000
window_secs = 60

[proxy]
add_x_forwarded_for = true
add_x_real_ip = true
add_via_header = true
via_value = "axum-proxy"
strip_request_headers = ["proxy-connection"]
websocket_enabled = true
```

## Endpoints

| Endpoint    | Description              |
|-------------|--------------------------|
| `/*`        | Proxied to backends      |
| `/__health` | Health dashboard (HTML)  |
| `/__stats`  | Backend stats (JSON)     |

## Load Balancing Algorithms

- **round_robin**: Cycles through backends sequentially
- **least_connections**: Routes to backend with fewest active connections
- **weighted**: Random selection weighted by backend weight

## License

MIT
