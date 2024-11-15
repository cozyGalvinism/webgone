# webgone

A Rust-based internet connectivity monitor that tracks and logs internet outages. This tool helps you keep track of when your internet connection drops, for how long, and provides statistics about your connection reliability.

## Features

- 🔍 Real-time internet connectivity monitoring
- 📊 Detailed statistics about outages
- 📅 Historical outage data
- 💾 SQLite database for persistent storage
- 📈 CSV export functionality
- 🐳 Docker support with multi-arch builds

## Installation

### Using Docker (Recommended)

1. Clone the repository:
```bash
git clone https://github.com/yourusername/webgone.git
cd webgone
```

2. Run with Docker Compose:
```bash
docker-compose up -d
```

### Building from Source

Requirements:
- Rust 1.75 or later
- SQLite3

```bash
cargo build --release
./target/release/webgone
```

## Usage

### Basic Commands

- Start monitoring (with default settings):
```bash
webgone
```

- Start monitoring with custom settings:
```bash
# Check 1.1.1.1:53 every 10 seconds
webgone --ip 1.1.1.1 --port 53 --interval 10

# Check Google DNS with custom interval
webgone --interval 30  # check every 30 seconds

# Check custom IP with default port and interval
webgone --ip 9.9.9.9  # check Quad9 DNS
```

- View statistics:
```bash
webgone stats
```

- View recent outages (default: last 5):
```bash
webgone recent
```
or specify a number:
```bash
webgone recent 10
```

- Export data to CSV:
```bash
webgone export outages.csv
```

### Docker Commands

- Start monitoring:
```bash
docker-compose up -d
```

- Start monitoring with custom settings:
```bash
# Using default settings
docker-compose up -d

# With custom IP and interval
docker-compose run webgone /app/webgone --ip 1.1.1.1 --interval 10
```

- View statistics:
```bash
docker-compose exec webgone /app/webgone stats
```

- View recent outages:
```bash
docker-compose exec webgone /app/webgone recent
```

- Export data:
```bash
docker-compose exec webgone /app/webgone export outages.csv
```

## Configuration

The application can be configured using command-line arguments:

| Argument | Description | Default |
|----------|-------------|---------|
| --ip | IP address to check | 8.8.8.8 (Google DNS) |
| --port | Port to check | 53 (DNS port) |
| --interval | Check interval in seconds | 5 |

Example configurations:
- Google DNS (default): `8.8.8.8:53`
- Cloudflare DNS: `1.1.1.1:53`
- Quad9 DNS: `9.9.9.9:53`
- Custom server: `192.168.1.1:80`

## How It Works

The application performs TCP connection tests to Google's DNS server (8.8.8.8) every 5 seconds to check internet connectivity. When a connection fails:

1. The start time of the outage is recorded
2. The application continues monitoring until the connection is restored
3. Once restored, it calculates the outage duration and stores it in the SQLite database
4. Real-time notifications are printed to the console

## Data Storage

- All outage data is stored in a SQLite database (`internet_outages.db`)
- When using Docker, the database is stored in a persistent volume (`./data`)
- Data can be exported to CSV format for further analysis

## Docker Support

The application includes:
- Multi-stage Docker builds for minimal image size
- Multi-architecture support (AMD64 and ARM64)
- Automated builds via GitHub Actions
- Layer caching for faster builds
- Persistent volume for database storage

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
