use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, params, Row};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::{thread, time::Duration};
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::time::Instant;
use ascii_table::AsciiTable;
use std::io::Write;

struct InternetOutage {
    start_time: DateTime<Local>,
    end_time: DateTime<Local>,
    duration_seconds: i64,
}

struct OutageStats {
    total_outages: i64,
    total_duration: i64,
    average_duration: f64,
    longest_outage: i64,
    shortest_outage: i64,
}

struct MonthlyOutage {
    year: i32,
    month: u32,
    total_seconds: i64,
    num_outages: i64,
}

impl InternetOutage {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let start_str: String = row.get(1)?;
        let end_str: String = row.get(2)?;
        let duration_seconds: i64 = row.get(3)?;

        let start_time = DateTime::parse_from_rfc3339(&start_str)
            .map(|dt| dt.with_timezone(&Local))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;
        
        let end_time = DateTime::parse_from_rfc3339(&end_str)
            .map(|dt| dt.with_timezone(&Local))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(InternetOutage {
            start_time,
            end_time,
            duration_seconds,
        })
    }
}

fn init_database(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS outages (
            id INTEGER PRIMARY KEY,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn check_internet(addr: SocketAddr) -> bool {
    let timeout = Duration::from_secs(1);
    let start = Instant::now();
    let result = TcpStream::connect_timeout(&addr, timeout);
    
    match result {
        Ok(_) => true,
        Err(e) => {
            println!("Connection failed after {:?}: {}", start.elapsed(), e);
            false
        }
    }
}

fn log_outage(conn: &Connection, outage: &InternetOutage) -> Result<()> {
    conn.execute(
        "INSERT INTO outages (start_time, end_time, duration_seconds) VALUES (?1, ?2, ?3)",
        params![
            outage.start_time.to_rfc3339(),
            outage.end_time.to_rfc3339(),
            outage.duration_seconds
        ],
    )?;
    Ok(())
}

fn get_stats(conn: &Connection) -> Result<OutageStats> {
    let mut stmt = conn.prepare("
        SELECT 
            COUNT(*) as total_outages,
            SUM(duration_seconds) as total_duration,
            AVG(duration_seconds) as avg_duration,
            MAX(duration_seconds) as longest_outage,
            MIN(duration_seconds) as shortest_outage
        FROM outages
    ")?;

    let stats = stmt.query_row([], |row| {
        Ok(OutageStats {
            total_outages: row.get(0).unwrap_or_default(),
            total_duration: row.get(1).unwrap_or_default(),
            average_duration: row.get(2).unwrap_or_default(),
            longest_outage: row.get(3).unwrap_or_default(),
            shortest_outage: row.get(4).unwrap_or_default(),
        })
    })?;

    Ok(stats)
}

fn print_recent_outages(conn: &Connection, limit: i64) -> Result<()> {
    let mut stmt = conn.prepare("
        SELECT * FROM outages 
        ORDER BY start_time DESC 
        LIMIT ?
    ")?;

    let outages = stmt.query_map([limit], InternetOutage::from_row)?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("Start Time").set_align(ascii_table::Align::Left);
    table.column(1).set_header("End Time").set_align(ascii_table::Align::Left);
    table.column(2).set_header("Duration (seconds)").set_align(ascii_table::Align::Right);

    let mut data = Vec::new();

    for outage in outages {
        let outage = outage.map_err(|e| anyhow::anyhow!(e))?;

        data.push(vec![
            outage.start_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            outage.end_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            outage.duration_seconds.to_string(),
        ]);
    }

    table.print(data);

    Ok(())
}

fn generate_csv(conn: &Connection) -> Result<String> {
    let mut wrt = BufWriter::new(Vec::new());

    writeln!(wrt, "Start Time,End Time,Duration (seconds)")?;

    let mut stmt = conn.prepare("SELECT * FROM outages ORDER BY start_time")?;
    let outages = stmt.query_map([], InternetOutage::from_row)?;

    for outage in outages {
        let outage = outage.map_err(|e| anyhow::anyhow!(e))?;
        writeln!(
            wrt,
            "{},{},{}",
            outage.start_time.to_rfc3339(),
            outage.end_time.to_rfc3339(),
            outage.duration_seconds
        )?;
    }

    let data = wrt.into_inner()?;
    String::from_utf8(data).map_err(Into::into)
}

fn export_to_csv(conn: &Connection, filename: &Path) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(filename)?;
    let data = generate_csv(conn)?;
    file.write_all(data.as_bytes())?;

    println!("Data exported to {}", filename.display());
    Ok(())
}

fn calculate_monthly_costs(conn: &Connection) -> Result<Vec<MonthlyOutage>> {
    let mut stmt = conn.prepare("
        SELECT 
            strftime('%Y', start_time) as year,
            strftime('%m', start_time) as month,
            COUNT(*) as num_outages,
            SUM(duration_seconds) as total_duration
        FROM outages 
        GROUP BY year, month
        ORDER BY year DESC, month DESC
    ")?;

    let monthly_outages = stmt.query_map([], |row| {
        Ok(MonthlyOutage {
            year: row.get::<_, String>(0)?.parse().unwrap(),
            month: row.get::<_, String>(1)?.parse().unwrap(),
            num_outages: row.get(2)?,
            total_seconds: row.get(3)?,
        })
    })?;

    Ok(monthly_outages.collect::<Result<Vec<_>, _>>()?)
}

fn print_cost_report(conn: &Connection, monthly_rate: f64, currency: &str) -> Result<()> {
    let monthly_outages = calculate_monthly_costs(conn)?;
    
    println!("\nMonthly Cost Analysis:");

    let mut table = AsciiTable::default();
    table.column(0).set_header("Year").set_align(ascii_table::Align::Left);
    table.column(1).set_header("Month").set_align(ascii_table::Align::Left);
    table.column(2).set_header("Outages").set_align(ascii_table::Align::Right);
    table.column(3).set_header("Total Time").set_align(ascii_table::Align::Right);
    table.column(4).set_header("% Downtime").set_align(ascii_table::Align::Right);
    table.column(5).set_header("Cost Impact").set_align(ascii_table::Align::Right);
    table.column(6).set_header("Rate/Hour").set_align(ascii_table::Align::Right);

    let mut total_cost = 0.0;
    let mut total_seconds = 0_i64;
    let mut total_months = 0;
    let mut data = Vec::new();

    for outage in &monthly_outages {
        let month_name = match outage.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };

        // Calculate month-specific metrics
        let days_in_month = match outage.month {
            4 | 6 | 9 | 11 => 30.0,
            2 => if outage.year % 4 == 0 && (outage.year % 100 != 0 || outage.year % 400 == 0) {
                29.0
            } else {
                28.0
            },
            _ => 31.0,
        };
        
        let seconds_in_month = days_in_month * 24.0 * 60.0 * 60.0;
        let downtime_percentage = (outage.total_seconds as f64 / seconds_in_month) * 100.0;
        let cost = (outage.total_seconds as f64 / seconds_in_month) * monthly_rate;
        let hourly_rate = monthly_rate / (days_in_month * 24.0);

        let hours = outage.total_seconds / 3600;
        let minutes = (outage.total_seconds % 3600) / 60;
        let seconds = outage.total_seconds % 60;

        data.push(vec![
            outage.year.to_string(),
            month_name.to_string(),
            outage.num_outages.to_string(),
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds),
            format!("{:.3}%", downtime_percentage),
            format!("{currency}{:.3}", cost),
            format!("{currency}{:.3}/h", hourly_rate),
        ]);

        total_cost += cost;
        total_seconds += outage.total_seconds;
        total_months += 1;
    }

    table.print(data);

    // Calculate and display overall statistics
    if total_months > 0 {
        let total_hours = total_seconds as f64 / 3600.0;
        let avg_monthly_downtime = total_seconds as f64 / total_months as f64 / 3600.0;
        let avg_cost_per_month = total_cost / total_months as f64;
        let cost_per_hour = if total_hours > 0.0 { total_cost / total_hours } else { 0.0 };

        let mut summary_table = AsciiTable::default();
        summary_table.column(0).set_header("Metric").set_align(ascii_table::Align::Left);
        summary_table.column(1).set_header("Value").set_align(ascii_table::Align::Right);

        let summary_data = vec![
            vec!["Total cost of outages".to_string(), format!("€{:.3}", total_cost)],
            vec!["Average monthly cost".to_string(), format!("€{:.3}", avg_cost_per_month)],
            vec!["Total downtime".to_string(), format!("{:.1} hours ({:.1} hours/month avg)", total_hours, avg_monthly_downtime)],
            vec!["Cost per hour of downtime".to_string(), format!("€{:.3}/h", cost_per_hour)],
        ];

        println!("\nSummary:");
        summary_table.print(summary_data);
        println!();
    } else {
        println!("\nNo outages recorded yet.\n");
    }

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    /// Watch for internet outages
    Watch {
        /// IP address to check
        #[arg(short, long, default_value_t = IpAddr::from([8, 8, 8, 8]))]
        ip: IpAddr,
        /// Port to check
        #[arg(short, long, default_value_t = 53)]
        port: u16,
        /// Interval in seconds
        #[arg(short = 'I', long, default_value_t = 5)]
        interval: u64
    },
    /// Print statistics about internet outages
    Stats,
    /// View recent internet outages
    Recent {
        /// Amount of outages to display
        #[arg(short, long, default_value_t = 5)]
        limit: usize
    },
    /// Export internet outages to a CSV file or stdout
    Export {
        /// Output file path (if not provided, data will be printed to stdout)
        output: Option<PathBuf>
    },
    /// Calculate cost impact of internet outages
    Cost {
        /// Currency symbol
        #[arg(short, long, default_value_t = String::from("€"))]
        currency: String,

        /// Monthly rate for cost analysis
        rate: f64
    }
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let conn = Connection::open("internet_outages.db")
        .context("Failed to open database")?;
    
    init_database(&conn)?;

    match args.command {
        Commands::Watch { ip, port, interval } => {
            let addr = SocketAddr::new(ip, port);
            let interval = Duration::from_secs(interval);
            println!("Starting internet connectivity monitoring...");
            println!("Checking {} every {} seconds", addr, interval.as_secs());
            println!("Press Ctrl+C to stop monitoring.");

            let mut is_connected = true;
            let mut outage_start: Option<DateTime<Local>> = None;
            
            loop {
                let current_status = check_internet(addr);
                
                match (is_connected, current_status) {
                    (true, false) => {
                        outage_start = Some(Local::now());
                        println!("Internet connection lost at {}", outage_start.unwrap());
                        is_connected = false;
                    }
                    (false, true) => {
                        if let Some(start_time) = outage_start {
                            let end_time = Local::now();
                            let duration = end_time.signed_duration_since(start_time);
                            
                            let outage = InternetOutage {
                                start_time,
                                end_time,
                                duration_seconds: duration.num_seconds(),
                            };
                            
                            log_outage(&conn, &outage)?;
                            println!(
                                "Internet connection restored at {}. Outage duration: {} seconds",
                                end_time,
                                duration.num_seconds()
                            );
                            
                            is_connected = true;
                            outage_start = None;
                        }
                    }
                    _ => {}
                }
                
                thread::sleep(interval);
            }
        },
        Commands::Stats => {
            let stats = get_stats(&conn)?;
            println!("\nInternet Outage Statistics:");
            println!("{:-<50}", "");
            println!("Total number of outages: {}", stats.total_outages);
            println!("Total outage duration: {} seconds", stats.total_duration);
            println!("Average outage duration: {:.2} seconds", stats.average_duration);
            println!("Longest outage: {} seconds", stats.longest_outage);
            println!("Shortest outage: {} seconds", stats.shortest_outage);
            println!("{:-<50}\n", "");
        },
        Commands::Recent { limit } => {
            print_recent_outages(&conn, limit as i64)?;
        },
        Commands::Export { output } => {
            if let Some(ref filename) = output {
                export_to_csv(&conn, filename)?;
            } else {
                println!("{}", generate_csv(&conn)?);
            }
        },
        Commands::Cost { currency, rate } => {
            print_cost_report(&conn, rate, &currency)?;
        }
    }

    Ok(())
}
