use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use vsock::{VsockAddr, VsockStream};

fn forward<R: Read, W: Write>(mut source: R, mut destination: W) {
    let mut buffer = [0; 1024];
    loop {
        match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if let Err(_) = destination.write_all(&buffer[..n]) {
                    break;
                }
            }
            Err(_) => {
                // Ignore connection reset error
                break;
            }
        }
    }
}

fn server(local_ip: &str, local_port: u16, remote_cid: u32, remote_port: u32) {
    let addr = format!("{}:{}", local_ip, local_port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind TCP listener");

    for stream in listener.incoming() {
        match stream {
            Ok(client_stream) => {
                let v_addr = VsockAddr::new(remote_cid, remote_port);
                match VsockStream::connect(&v_addr) {
                    Ok(server_stream) => {
                        let client_reader = client_stream.try_clone().unwrap();
                        let client_writer = client_stream;
                        let server_reader = server_stream.try_clone().unwrap();
                        let server_writer = server_stream;

                        thread::spawn(move || {
                            forward(client_reader, server_writer);
                        });
                        thread::spawn(move || {
                            forward(server_reader, client_writer);
                        });
                    }
                    Err(e) => {
                        println!(
                            "Failed to connect to VSOCK {}:{}: {}",
                            remote_cid, remote_port, e
                        );
                    }
                }
            }
            Err(e) => {
                println!("Failed to accept TCP connection: {}", e);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        eprintln!(
            "Usage: {} <local_ip> <local_port> <remote_cid> <remote_port>",
            args[0]
        );
        std::process::exit(1);
    }

    let local_ip = args[1].clone();
    let local_port: u16 = args[2].parse().unwrap();
    let remote_cid: u32 = args[3].parse().unwrap();
    let remote_port: u32 = args[4].parse().unwrap();

    let local_ip_clone = local_ip.clone();
    thread::spawn(move || {
        // Pythonの実装を真似て、プロセスがクラッシュした際に再起動するループ
        loop {
            server(&local_ip_clone, local_port, remote_cid, remote_port);
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    println!(
        "starting forwarder on {}:{} {}:{}",
        local_ip, local_port, remote_cid, remote_port
    );

    loop {
        thread::sleep(std::time::Duration::from_secs(60));
    }
}
