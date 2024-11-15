use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{Connection, params, Row};
use std::{thread, time::Duration};
use std::env;
use std::net::{TcpStream, SocketAddr};
use std::time::Instant;

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

fn check_internet() -> bool {
    // Try connecting to Google's DNS server (8.8.8.8:53)
    let addr = SocketAddr::from(([8, 8, 8, 8], 53));
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
            total_outages: row.get(0)?,
            total_duration: row.get(1)?,
            average_duration: row.get(2)?,
            longest_outage: row.get(3)?,
            shortest_outage: row.get(4)?,
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

    println!("\nRecent Outages:");
    println!("{:-<80}", "");
    for outage in outages {
        let outage = outage.map_err(|e| anyhow::anyhow!(e))?;
        println!(
            "Start: {}, End: {}, Duration: {} seconds",
            outage.start_time.format("%Y-%m-%d %H:%M:%S"),
            outage.end_time.format("%Y-%m-%d %H:%M:%S"),
            outage.duration_seconds
        );
    }
    println!("{:-<80}\n", "");

    Ok(())
}

fn export_to_csv(conn: &Connection, filename: &str) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(filename)?;
    writeln!(file, "Start Time,End Time,Duration (seconds)")?;

    let mut stmt = conn.prepare("SELECT * FROM outages ORDER BY start_time")?;
    let outages = stmt.query_map([], InternetOutage::from_row)?;

    for outage in outages {
        let outage = outage.map_err(|e| anyhow::anyhow!(e))?;
        writeln!(
            file,
            "{},{},{}",
            outage.start_time.to_rfc3339(),
            outage.end_time.to_rfc3339(),
            outage.duration_seconds
        )?;
    }

    println!("Data exported to {}", filename);
    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  webgone                    - Start monitoring internet connection");
    println!("  webgone stats              - Show statistics about outages");
    println!("  webgone recent [n]         - Show n most recent outages (default: 5)");
    println!("  webgone export [filename]  - Export data to CSV (default: outages.csv)");
}

fn main() -> Result<()> {
    let conn = Connection::open("internet_outages.db")
        .context("Failed to open database")?;
    
    init_database(&conn)?;

    let args: Vec<String> = env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        None => {
            println!("Starting internet connectivity monitoring...");
            println!("Press Ctrl+C to stop monitoring.");
            
            let mut is_connected = true;
            let mut outage_start: Option<DateTime<Local>> = None;
            
            loop {
                let current_status = check_internet();
                
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
                
                thread::sleep(Duration::from_secs(5));
            }
        }
        Some("stats") => {
            let stats = get_stats(&conn)?;
            println!("\nInternet Outage Statistics:");
            println!("{:-<50}", "");
            println!("Total number of outages: {}", stats.total_outages);
            println!("Total outage duration: {} seconds", stats.total_duration);
            println!("Average outage duration: {:.2} seconds", stats.average_duration);
            println!("Longest outage: {} seconds", stats.longest_outage);
            println!("Shortest outage: {} seconds", stats.shortest_outage);
            println!("{:-<50}\n", "");
        }
        Some("recent") => {
            let limit = args.get(2)
                .and_then(|n| n.parse().ok())
                .unwrap_or(5);
            print_recent_outages(&conn, limit)?;
        }
        Some("export") => {
            let filename = args.get(2)
                .map(|s| s.as_str())
                .unwrap_or("outages.csv");
            export_to_csv(&conn, filename)?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}
