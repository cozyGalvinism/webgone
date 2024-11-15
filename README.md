# webgone

A Rust-based internet connectivity monitor that tracks and logs internet outages. This tool helps you keep track of when your internet connection drops, for how long, and provides statistics about your connection reliability.

## Features

- 🔍 Real-time internet connectivity monitoring
- 📊 Detailed statistics about outages
- 📅 Historical outage data
- 💾 SQLite database for persistent storage
- 📈 CSV export functionality
- 💰 Cost impact analysis
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
./target/release/webgone --help
```

## Usage

### Basic Commands

- Start monitoring (with default settings):
```bash
webgone watch
```

- Start monitoring with custom settings:
```bash
# Check 1.1.1.1:53 every 10 seconds
webgone watch --ip 1.1.1.1 --port 53 --interval 10

# Check Google DNS with custom interval
webgone watch --interval 30  # check every 30 seconds

# Check custom IP with default port and interval
webgone watch --ip 9.9.9.9  # check Quad9 DNS
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

- Calculate cost impact (with monthly rate in EUR):
```bash
webgone cost 45.99
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
docker-compose run webgone /app/webgone watch --ip 1.1.1.1 --interval 10
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

- Calculate cost impact:
```bash
docker-compose exec webgone /app/webgone cost 45.99
```

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

## Cost Analysis

The cost analysis feature helps you understand the monetary impact of your internet outages:

- Calculates the proportional cost of downtime based on your monthly rate
- Uses exact number of days per month (accounting for leap years)
- Provides detailed monthly breakdown including:
  * Number of outages
  * Total downtime in HH:MM:SS format
  * Percentage of downtime
  * Cost impact with 3 decimal precision
  * Hourly rate for the month
- Shows comprehensive statistics:
  * Total cost across all outages
  * Average monthly cost
  * Total downtime in hours
  * Average monthly downtime
  * Effective cost per hour of downtime

Example output:
```
Monthly Cost Analysis:
----------------------------------------------------------------------------------------------------
| Year      | Month       | Outages | Total Time     | % Downtime  | Cost Impact    | Rate/Hour    |
----------------------------------------------------------------------------------------------------
| 2024      | February    | 5       | 01:23:45      | 0.205%     | €      0.833   | €    0.066/h |
| 2024      | January     | 3       | 00:45:30      | 0.102%     | €      0.452   | €    0.062/h |
----------------------------------------------------------------------------------------------------
Total cost of outages: €1.285
Average monthly cost: €0.643
Total downtime: 2.2 hours (1.1 hours/month avg)
Effective cost per hour of downtime: €0.584/h
----------------------------------------------------------------------------------------------------
```

The cost is calculated using the exact number of days in each month:
- Regular months: 31 days
- Short months: 30 days (Apr, Jun, Sep, Nov)
- February: 28/29 days (accounting for leap years)

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
