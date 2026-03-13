use clap::Parser;
use serde::Deserialize;
use std::io::{Read, Write};
use vsock::{VsockAddr, VsockListener};

#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "Server for VSOCK")]
struct Args {
    #[arg(help = "The local port to listen on.")]
    port: u32,
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

fn get_s3_ip_by_region(region: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let full_query = "https://ip-ranges.amazonaws.com/ip-ranges.json";
    println!("Full URL: {}", full_query);
    println!("URL Open");

    let response: IpRangesResponse = reqwest::blocking::get(full_query)?.json()?;
    println!("Handle Response");

    let mut s3_ips = Vec::new();
    for item in response.prefixes {
        if item.service == "S3" && item.region == region {
            s3_ips.push(item.ip_prefix);
        }
    }
    Ok(s3_ips)
}

fn main() {
    let args = Args::parse();

    let addr = VsockAddr::new(vsock::VMADDR_CID_ANY, args.port);
    let listener = match VsockListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            println!("Failed to bind to port {}: {}", args.port, e);
            return;
        }
    };

    println!("Started listening to port : {}", args.port);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("Let's accept stuff");
                match stream.peer_addr() {
                    Ok(peer) => println!("Connection from {:?}", peer),
                    Err(e) => println!("Connection from unknown: {}", e),
                }

                let mut query_buf = [0; 1024];
                match stream.read(&mut query_buf) {
                    Ok(n) if n > 0 => {
                        let query = String::from_utf8_lossy(&query_buf[..n]).to_string();
                        println!("Message received: {}", query);

                        match get_s3_ip_by_region(&query) {
                            Ok(ips) => {
                                // Pythonの `str(response)` に近い形で文字列化
                                let mut response_str = "[".to_string();
                                for (i, ip) in ips.iter().enumerate() {
                                    if i > 0 {
                                        response_str.push_str(", ");
                                    }
                                    response_str.push_str(&format!("'{}'", ip));
                                }
                                response_str.push(']');

                                let _ = stream.write_all(response_str.as_bytes());
                            }
                            Err(e) => {
                                println!("Error getting IPs: {}", e);
                            }
                        }
                    }
                    Ok(_) => {
                        // Empty read
                    }
                    Err(e) => {
                        println!("Error reading: {}", e);
                    }
                }
                println!("Client call closed");
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }
}
