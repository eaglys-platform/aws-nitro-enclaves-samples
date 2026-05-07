use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::Command;
use thiserror::Error;
use vsock::{VsockAddr, VsockListener, VsockStream};

#[derive(Debug, Error)]
enum AppError {
    #[error("VSOCK bind failed on port {port}: {source}")]
    Bind {
        port: u32,
        #[source]
        source: std::io::Error,
    },
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization Error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("KMS Decryption Failed: {0}")]
    KmsDecryptionFailed(String),
}

#[derive(Deserialize, Debug)]
struct Credentials {
    access_key_id: String,
    secret_access_key: String,
    token: String,
    ciphertext: String,
    region: String,
}

#[derive(Serialize)]
struct ResponsePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    last_four: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn decrypt_cipher(creds: &Credentials) -> Result<String, AppError> {
    let output = Command::new("/app/kmstool_enclave_cli")
        .args(&[
            "decrypt",
            "--region",
            &creds.region,
            "--aws-access-key-id",
            &creds.access_key_id,
            "--aws-secret-access-key",
            &creds.secret_access_key,
            "--aws-session-token",
            &creds.token,
            "--ciphertext",
            &creds.ciphertext,
        ])
        .output()
        .map_err(AppError::Io)?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let b64_part = stdout.split(':').nth(1).ok_or_else(|| {
            AppError::KmsDecryptionFailed("Malformed output from kmstool: missing colon".into())
        })?;

        let b64text = b64_part.trim();
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64text)
            .map_err(|e| {
            AppError::KmsDecryptionFailed(format!("Base64 decode error: {}", e))
        })?;

        let plaintext = String::from_utf8_lossy(&decoded).to_string();
        let last_four = plaintext
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        Ok(last_four)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError::KmsDecryptionFailed(format!(
            "KMS Error: {}",
            stderr
        )))
    }
}

fn handle_request(stream: &mut VsockStream, payload: &[u8]) -> Result<(), AppError> {
    let creds: Credentials = serde_json::from_slice(payload)?;

    let response = match decrypt_cipher(&creds) {
        Ok(last_four) => {
            println!("{}", last_four);
            ResponsePayload {
                last_four: Some(last_four),
                error: None,
            }
        }
        Err(e) => {
            eprintln!("Decryption error: {}", e);
            ResponsePayload {
                last_four: None,
                error: Some(format!("KMS Error. Decryption Failed. {}", e)),
            }
        }
    };

    let response_bytes = serde_json::to_vec(&response)?;
    stream.write_all(&response_bytes)?;

    Ok(())
}

fn handle_connection(mut stream: VsockStream) {
    if let Ok(peer) = stream.peer_addr() {
        println!("Connection from {:?}", peer);
    }

    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        Ok(_) => return,
        Err(e) => {
            eprintln!("Error reading from stream: {}", e);
            return;
        }
    };

    if let Err(e) = handle_request(&mut stream, &buf[..n]) {
        eprintln!("Error handling request: {}", e);
        let err_resp = ResponsePayload {
            last_four: None,
            error: Some(e.to_string()),
        };
        if let Ok(bytes) = serde_json::to_vec(&err_resp) {
            let _ = stream.write_all(&bytes);
        }
    }
}

fn main() -> Result<(), AppError> {
    let port = 5000;
    let cid = vsock::VMADDR_CID_ANY;
    let addr = VsockAddr::new(cid, port);

    let listener = VsockListener::bind(&addr).map_err(|e| AppError::Bind { port, source: e })?;
    println!("Started server on port {} and cid {}", port, cid);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream),
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}
