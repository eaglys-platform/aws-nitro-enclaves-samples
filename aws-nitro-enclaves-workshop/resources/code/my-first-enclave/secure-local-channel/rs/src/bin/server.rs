use clap::Parser;
use serde::Deserialize;
use std::io::{Read, Write};
use thiserror::Error;
use vsock::{VsockAddr, VsockListener, VsockStream};

#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "Server for VSOCK")]
struct Args {
    #[arg(help = "The local port to listen on.")]
    port: u32,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("VSOCK bind failed on port {port}: {source}")]
    Bind {
        port: u32,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read from stream: {0}")]
    Read(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to serialize response: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct IpRangesResponse {
    prefixes: Vec<Prefix>,
}

#[derive(Deserialize)]
struct Prefix {
    ip_prefix: String,
    region: String,
    service: String,
}

fn get_s3_ip_by_region(region: &str) -> Result<Vec<String>, AppError> {
    let url = "https://ip-ranges.amazonaws.com/ip-ranges.json";
    println!("Full URL: {}", url);
    println!("URL Open");

    let response: IpRangesResponse = reqwest::blocking::get(url)?.json()?;
    println!("Handle Response");

    let ips = response
        .prefixes
        .into_iter()
        .filter(|p| p.service == "S3" && p.region == region)
        .map(|p| p.ip_prefix)
        .collect();

    Ok(ips)
}

fn handle_request(stream: &mut VsockStream, query: &str) -> Result<(), AppError> {
    let ips = get_s3_ip_by_region(query)?;
    let response_bytes = serde_json::to_vec(&ips)?;
    stream.write_all(&response_bytes)?;
    Ok(())
}

fn handle_connection(mut stream: VsockStream) {
    match stream.peer_addr() {
        Ok(peer) => println!("Connection from {:?}", peer),
        Err(e) => eprintln!("Connection from unknown: {}", e),
    }

    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        Ok(_) => {
            println!("Empty request received");
            return;
        }
        Err(e) => {
            eprintln!("Error reading from stream: {}", e);
            return;
        }
    };

    let query = String::from_utf8_lossy(&buf[..n]);
    println!("Message received: {}", query);

    if let Err(e) = handle_request(&mut stream, query.trim()) {
        eprintln!("Error handling request: {}", e);
    }

    println!("Client call closed");
}

fn main() {
    let args = Args::parse();

    let addr = VsockAddr::new(vsock::VMADDR_CID_ANY, args.port);
    let listener = match VsockListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "{}",
                AppError::Bind {
                    port: args.port,
                    source: e
                }
            );
            return;
        }
    };

    println!("Started listening to port: {}", args.port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream),
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}
