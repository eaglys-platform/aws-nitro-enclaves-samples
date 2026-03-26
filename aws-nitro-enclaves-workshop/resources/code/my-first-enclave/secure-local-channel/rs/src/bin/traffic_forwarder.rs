use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use vsock::{VsockAddr, VsockStream};

#[derive(Debug, thiserror::Error)]
enum ForwarderError {
    #[error("Failed to bind TCP listener on {addr}: {source}")]
    TcpBind {
        addr: String,
        source: std::io::Error,
    },
    #[error("Failed to clone TCP stream: {0}")]
    TcpClone(std::io::Error),
    #[error("Failed to connect to VSOCK (cid={cid}, port={port}): {source}")]
    VsockConnect {
        cid: u32,
        port: u32,
        source: std::io::Error,
    },
    #[error("Failed to clone VSOCK stream: {0}")]
    VsockClone(std::io::Error),
    #[error("Failed to accept TCP connection: {0}")]
    Accept(std::io::Error),
}

#[derive(clap::Parser, Debug)]
#[command(about = "Forwards TCP traffic to a VSOCK endpoint")]
struct Args {
    /// Local IP address to listen on
    local_ip: String,
    /// Local TCP port to listen on
    local_port: u16,
    /// Remote VSOCK CID
    remote_cid: u32,
    /// Remote VSOCK port
    remote_port: u32,
}

fn forward<R: Read, W: Write>(mut source: R, mut destination: W) {
    let mut buffer = [0u8; 1024];
    loop {
        match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if destination.write_all(&buffer[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(
    client_stream: std::net::TcpStream,
    server_stream: VsockStream,
) -> Result<(), ForwarderError> {
    let client_reader = client_stream
        .try_clone()
        .map_err(ForwarderError::TcpClone)?;
    let client_writer = client_stream;
    let server_reader = server_stream
        .try_clone()
        .map_err(ForwarderError::VsockClone)?;
    let server_writer = server_stream;

    thread::spawn(move || forward(client_reader, server_writer));
    thread::spawn(move || forward(server_reader, client_writer));

    Ok(())
}

fn server(
    local_ip: &str,
    local_port: u16,
    remote_cid: u32,
    remote_port: u32,
) -> Result<(), ForwarderError> {
    let addr = format!("{local_ip}:{local_port}");
    let listener = TcpListener::bind(&addr).map_err(|e| ForwarderError::TcpBind {
        addr: addr.clone(),
        source: e,
    })?;

    for stream in listener.incoming() {
        let client_stream = stream.map_err(ForwarderError::Accept)?;

        let v_addr = VsockAddr::new(remote_cid, remote_port);
        match VsockStream::connect(&v_addr) {
            Ok(server_stream) => {
                if let Err(e) = handle_connection(client_stream, server_stream) {
                    eprintln!("forward error: {e}");
                }
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    ForwarderError::VsockConnect {
                        cid: remote_cid,
                        port: remote_port,
                        source: e,
                    }
                );
            }
        }
    }

    Ok(())
}

fn main() {
    use clap::Parser;
    let args = Args::parse();

    println!(
        "starting forwarder on {}:{} {}:{}",
        args.local_ip, args.local_port, args.remote_cid, args.remote_port
    );

    thread::spawn(move || {
        loop {
            if let Err(e) = server(
                &args.local_ip,
                args.local_port,
                args.remote_cid,
                args.remote_port,
            ) {
                eprintln!("server error: {e}");
            }
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    loop {
        thread::sleep(std::time::Duration::from_secs(60));
    }
}
