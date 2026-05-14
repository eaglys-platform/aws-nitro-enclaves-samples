## 概要
本プロジェクトは、[AWS Nitro Enclaves Workshop](https://catalog.workshops.aws/nitro-enclaves/en-US)をベースにしています。

workshopの各セクション(secure-local-channel, cryptographic-attestationなど)ごとにディレクトリが分かれていて、その中にオリジナルのPython実装（py/）と、Rustでの再実装（rs/）が配置されています。

### ディレクトリ構成 
- py/ ディレクトリ: 公式workshop通り、Pythonで書かれたオリジナルのソースコード。
- rs/ ディレクトリ: 新たに追加された、Rust言語による実装。
- justfile: ビルドや実環境での実行を簡略化するためのコマンドランナー。

## 環境構築
元のworkshopの前提条件に加えて、Rust実装を動かすために必要な環境の差分を記載します。

- Python版: python3, pip が必要。

- Rust版: Rustツールチェーン（rustup, cargo, rustc）が必要。また、コンパイル環境や依存ライブラリ（thiserror, serde_json など）が Cargo.toml に定義されています。

- [just](https://github.com/casey/just): workshopでは、各セクションごとにdocker imageのビルド〜enclaveの実行まででほとんど同じコマンドを繰り返し実行します。このプロジェクトでは、効率化のために一連の流れを`just`というRust製のコマンドランナーを使ってまとめています。

### Rustツールチェーン
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
sudo yum install -y gcc openssl-devel
```
- インスタンス内で上記コマンドを実行

- `cargo --version`などで確認してください。

- デフォルトではbashrcなどにパスが通っていないので、cargo関連のコマンドを実行する時は`source $HOME/.cargo/env`の実行が必要です。

### Just
```bash
sudo dnf install spal-release -y
sudo dnf install just -y
```
- インスタンス内で上記コマンドを実行

- `just --version`などで確認してください。

- Enclaveのビルドと起動は
```bash
just run <NAME> [debug-mode]
```
- 上記コマンドで実行できます。`<NAME>`にはeifファイルの名称を使ってください。

- workshop内でenclaveの起動に"--debug-mode"オプションをつけている時は、justコマンドの実行にも"debug-mode"引数を追加してください。

## 実行方法
前述の通り、Enclavesの実行にjustを使えるようにしているので、各セクションごとにworkshopで提供されているコマンドとの置き換えを説明します。

### 事前準備(Getting Started/Prerequisites and environment setup)
- Clone the workshop repositoryで使うURLを以下に変更:
    - https://github.com/eaglys-platform/aws-nitro-enclaves-samples.git
- 以降の`cd`コマンドのパスを置き換え:
    - 変更前: `cd ~/workshop/aws-nitro-enclaves-workshop/resources/code/...`
    - 変更後: `cd ~/aws-nitro-enclaves-samples/aws-nitro-enclaves-workshop/resources/code/...`

- また、コマンドの実行は全て各セクションのrs/ディレクトリで行うようにしてください。

### nitro-enclaves-cli
#### Build Nitro Enclave Image File
- 1番`docker build -t hello-app:latest`から、Run, connect, and terminate the enclaveの4番`[ "$ENCLAVE_ID" != "null" ] && nitro-cli console --enclave-id ${ENCLAVE_ID}`までを以下に置き換え:
```bash
just run hello debug-mode
```
- (勉強のために最初はjustを使わず、workshop通りにやってもいいかも知れません)

#### Run, connect, and terminate the enclave
- 5番を以下に置き換え:
```bash
just stop
```

### secure-local-channel
#### Run Server Application
- 3番を以下に置き換え:
```bash
just run secure-channel-example debug-mode
```

#### Run Client Application
- 2番を以下に置き換え:
```bash
ENCLAVE_CID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveCID")
cargo run --release --bin client -- $ENCLAVE_CID 5005 "us-east-1"
```

#### Preparing for the next module
- 1番を以下に置き換え:
```bash
just stop
```

### cryptographic-attestation
#### Build, run, and connect to your enclave in debug mode
- 2番から5番までを以下に置き換え:
```bash
just run data-processing debug-mode
```

#### Interacting with your enclave application
- pip3での依存関係のインストールは不要

- 2番を以下に置き換え:
```bash
cargo run --release --bin client -- prepare --values "values.txt" --alias "my-enclave-key"
```

- 3番を以下に置き換え:
```bash
cargo run --release --bin client -- submit --ciphertext "string.encrypted" --alias "my-enclave-key"
```
- 以降、Validating enclave attestation-based KMS key policy conditionsやModifying the enclave application codeでも同様に置き換え

#### Validating enclave attestation-based KMS key policy conditions
- 1番を以下に置き換え:
```bash
just stop
```

- 2番を以下に置き換え:
```bash
just run data-processing
```
- `debug-mode`は不要

#### Modifying the enclave application code
- server.rsの変更(workshopではserver.pyを編集している)は、70~79行目を以下のように置き換え:
```rust
// 変更前
let plaintext = String::from_utf8_lossy(&decoded).to_string();
let last_four = plaintext.chars().rev().take(4).collect::<String>().chars().rev().collect();
Ok(last_four)
// 変更後
let plaintext = String::from_utf8_lossy(&decoded).to_string();
Ok(plaintext)
```

- 1番から4番までを`just stop`を実行した後、以下に置き換え:
```bash
just run data-processing-modified
```
