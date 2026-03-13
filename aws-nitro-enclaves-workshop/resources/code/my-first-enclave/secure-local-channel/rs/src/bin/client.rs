use clap::Parser;
use std::io::{Read, Write};
use vsock::{VsockAddr, VsockStream};

#[derive(Parser, Debug)]
#[command(version = "0.1.0", about = "Client for VSOCK")]
struct Args {
    #[arg(help = "Enclave CID.")]
    cid: u32,
    #[arg(help = "The remote endpoint port.")]
    port: u32,
    #[arg(help = "Region")]
    query: String,
}

fn main() {
    let args = Args::parse();
    println!("Endpoint Arguments {} {}", args.cid, args.port);

    let addr = VsockAddr::new(args.cid, args.port);
    let mut stream = match VsockStream::connect(&addr) {
        Ok(s) => s,
        Err(e) => {
            println!("Caught error {}", e);
            return;
        }
    };

    if let Ok(peer) = stream.peer_addr() {
        println!("Connected to {:?}", peer);
    }

    if let Err(e) = stream.write_all(args.query.as_bytes()) {
        println!("Error sending data: {}", e);
        return;
    }
    println!("Data Sent  {}", args.query);

    let mut response = String::new();
    if let Err(e) = stream.read_to_string(&mut response) {
        println!("Error receiving data: {}", e);
        return;
    }
    println!("Received from server: {}", response);
}
