use aws_config::BehaviorVersion;
use aws_sdk_kms::Client as KmsClient;
use aws_sdk_kms::primitives::Blob;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::{Parser, Subcommand};
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::Command;
use thiserror::Error;
use vsock::{VsockAddr, VsockStream};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "valid KMS key alias")]
    alias: String,

    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// select a single account number and encrypt it using KMS
    Prepare {
        #[arg(
            long,
            help = "The account values file to select an account number from"
        )]
        values: Option<String>,
    },
    /// submit an encrypted account number to the enclave server application
    Submit {
        #[arg(
            long,
            help = "A file containing a kms-encrypted base64 encoded account number"
        )]
        ciphertext: Option<String>,
    },
}

#[derive(Debug, Error)]
enum AppError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("KMS API Error: {0}")]
    KmsApi(String),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing argument: {0}")]
    MissingArgument(String),
    #[error("Runtime Error: {0}")]
    Misc(String),
}

#[derive(Serialize)]
struct CredentialPayload {
    access_key_id: String,
    secret_access_key: String,
    token: String,
    region: String,
    ciphertext: String,
}

#[derive(Deserialize)]
struct ImdsCredentials {
    #[serde(rename = "AccessKeyId")]
    access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    secret_access_key: String,
    #[serde(rename = "Token")]
    token: String,
}

fn get_imds_token() -> Option<String> {
    reqwest::blocking::Client::new()
        .put("http://169.254.169.254/latest/api/token")
        .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                r.text().ok()
            } else {
                None
            }
        })
}

fn get_imds_json(path: &str) -> Result<serde_json::Value, AppError> {
    let mut req = reqwest::blocking::Client::new().get(format!("http://169.254.169.254{}", path));
    if let Some(token) = get_imds_token() {
        req = req.header("X-aws-ec2-metadata-token", token);
    }

    let res = req.send()?;
    let text = res.text()?;
    serde_json::from_str(&text).map_err(AppError::from)
}

fn get_imds_text(path: &str) -> Result<String, AppError> {
    let mut req = reqwest::blocking::Client::new().get(format!("http://169.254.169.254{}", path));
    if let Some(token) = get_imds_token() {
        req = req.header("X-aws-ec2-metadata-token", token);
    }

    let res = req.send()?;
    Ok(res.text()?.trim().to_string())
}

fn get_region() -> Result<String, AppError> {
    let doc = get_imds_json("/latest/dynamic/instance-identity/document")?;
    doc.get("region")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Misc("Failed to parse region".into()))
}

fn get_credentials(region: &str, ciphertext: &str) -> Result<CredentialPayload, AppError> {
    let role_name = get_imds_text("/latest/meta-data/iam/security-credentials/")?;
    let creds_text = get_imds_text(&format!(
        "/latest/meta-data/iam/security-credentials/{}",
        role_name
    ))?;

    println!("{}", ciphertext);

    let creds: ImdsCredentials = serde_json::from_str(&creds_text)?;

    Ok(CredentialPayload {
        access_key_id: creds.access_key_id,
        secret_access_key: creds.secret_access_key,
        token: creds.token,
        region: region.into(),
        ciphertext: ciphertext.into(),
    })
}

fn get_cid() -> Result<u32, AppError> {
    let output = Command::new("/bin/nitro-cli")
        .arg("describe-enclaves")
        .output()?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    json.as_array()
        .and_then(|arr| arr.first())
        .and_then(|obj| obj.get("EnclaveCID"))
        .and_then(|cid| cid.as_u64())
        .map(|cid| cid as u32)
        .ok_or_else(|| AppError::Misc("Failed to get Enclave CID".into()))
}

fn get_random_value(filepath: &str) -> Result<String, AppError> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    let mut rng = rand::rng();
    lines
        .choose(&mut rng)
        .cloned()
        .ok_or_else(|| AppError::Misc("Values array is empty".into()))
}

async fn handle_prepare(alias: &str, values: Option<&str>, _region: &str) -> Result<(), AppError> {
    let val_path = values
        .ok_or_else(|| AppError::MissingArgument("--values is required for prepare".into()))?;

    let rand_val = get_random_value(val_path)?;

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let client = KmsClient::new(&config);

    let key_id = format!("alias/{}", alias);

    let resp = client
        .encrypt()
        .key_id(key_id)
        .plaintext(Blob::new(rand_val.as_bytes()))
        .send()
        .await
        .map_err(|e| AppError::KmsApi(e.to_string()))?;

    let blob = resp
        .ciphertext_blob
        .ok_or_else(|| AppError::KmsApi("Missing ciphertext_blob in response".into()))?;

    let b64 = STANDARD.encode(blob.as_ref());

    println!("{}", b64);
    if rand_val.len() >= 4 {
        println!("{}", &rand_val[rand_val.len() - 4..]);
    } else {
        println!("{}", rand_val);
    }

    let mut f = File::create("string.encrypted")?;
    f.write_all(b64.as_bytes())?;

    Ok(())
}

fn handle_submit(
    _alias: &str,
    ciphertext_path: Option<&str>,
    region: &str,
) -> Result<(), AppError> {
    let cipher_path = ciphertext_path
        .ok_or_else(|| AppError::MissingArgument("--ciphertext is required for submit".into()))?;

    let mut f = File::open(cipher_path)?;
    let mut ciphertext = String::new();
    f.read_to_string(&mut ciphertext)?;
    let ciphertext = ciphertext.trim().to_string(); // Trim unexpected newlines

    let cred_payload = get_credentials(region, &ciphertext)?;
    let payload_bytes = serde_json::to_vec(&cred_payload)?;

    let cid = get_cid()?;
    let port = 5000;

    let mut stream = VsockStream::connect(&VsockAddr::new(cid, port))?;

    stream.write_all(&payload_bytes)?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;

    let response: serde_json::Value = serde_json::from_slice(&buf[..n])?;
    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    let region = get_region()?;

    match &args.mode {
        Mode::Prepare { values } => {
            handle_prepare(&args.alias, values.as_deref(), &region).await?;
        }
        Mode::Submit { ciphertext } => {
            handle_submit(&args.alias, ciphertext.as_deref(), &region)?;
        }
    }

    Ok(())
}
