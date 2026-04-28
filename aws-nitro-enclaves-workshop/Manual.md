## 概要
本プロジェクトは、[AWS Nitro Enclaves Workshop](https://catalog.workshops.aws/nitro-enclaves/en-US)をベースにしています。

workshopの各セクション(secure-local-channel, cryptographic-attestationなど)ごとにディレクトリが別れており、その中にオリジナルのPython実装（py/）と、Rustでの再実装（rs/）が配置されています。

### ディレクトリ構成 
- py/ ディレクトリ: 公式ワークショップ通り、Pythonで書かれたオリジナルのソースコード。
- rs/ ディレクトリ: 新たに追加された、Rust言語による実装。
- justfile: ビルドや実環境での実行を簡略化するためのコマンドランナー。

## 環境構築
元のワークショップの前提条件に加えて、Rust実装を動かすために必要な環境の差分を記載します。

Python版: python3, pip が必要。

Rust版: Rustツールチェーン（rustup, cargo, rustc）が必要。また、コンパイル環境や依存ライブラリ（thiserror, serde_json など）が Cargo.toml に定義されています。

[just](https://github.com/casey/just): workshopでは、各セクションごとにdocker imageのビルド〜Enclavesの実行まででほとんど同じコマンドを繰り返し実行します。このプロジェクトでは、効率化のために一連の流れを`just`というRust製のコマンドランナーを使ってまとめています。

### Rustツールチェーン
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
sudo yum install -y gcc openssl-devel
```
インスタンス内で上記コマンドを実行

`cargo --version`などで確認してください。

デフォルトではbashrcなどにパスが通っていないので、cargo関連のコマンドを実行する時は`source $HOME/.cargo/env`の実行が必要です。

### Just
```
sudo dnf install spal-release -y
sudo dnf install just -y
```
インスタンス内で上記コマンドを実行

`just --version`などで確認してください。

Enclaveのビルドと起動は
```
just run <NAME> [debug-mode]
```
上記コマンドで実行できます。<NAME>にはeifファイルの名称を使ってください。

workshop内でenclaveの起動に"--debug-mode"オプションをつけている時は、justコマンドの実行にも"debug-mode"引数を追加してください。

## 実行方法
前述の通り、Enclavesの実行にjustを使えるようにしているので、各セクションごとにworkshopで提供されているコマンドとの置き換えを説明します。

### 事前準備(Getting Started/Prerequisites and environment setup)
Clone the workshop repositoryのところを
```
https://github.com/eaglys-platform/aws-nitro-enclaves-samples.git
```
に変更してください。

これ以降、`cd ~/workshop/aws-nitro-enclaves-workshop/resources/code/...`は`cd ~/aws-nitro-enclaves-samples/aws-nitro-enclaves-workshop/resources/code/...`に置き換えてください。

また、コマンドの実行は全て各セクションのrs/ディレクトリで行うようにしてください。

### nitro-enclaves-cli
Build Nitro Enclave Image Fileの`docker build -t hello-app:latest`からRun, connect, and terminate the enclaveの4番`[ "$ENCLAVE_ID" != "null" ] && nitro-cli console --enclave-id ${ENCLAVE_ID}`までを
```
just run hello debug-mode
```
で置き換えてください。(勉強のために最初はjustを使わず、workshop通りにやってもいいかも知れません)

Run, connect, and terminate the enclaveの5番は`just stop`で置き換えてください。

### secure-local-channel
Run Server Applicationの3番を
```
just run secure-channel-example debug-mode
```
に置き換えてください。

Run Client Applicationの2番を
```
ENCLAVE_CID=$(nitro-cli describe-enclaves | jq -r ".[0].EnclaveCID")
cargo run --release --bin client -- $ENCLAVE_CID 5005 "us-east-1"
```
に置き換えてください。

Preparing for the next moduleの1番は`just stop`で置き換えてください。

### cryptographic-attestation
Build, run, and connect to your enclave in debug modeの2番から5番までを
```
just run data-processing debug-mode
```
に置き換えてください。

Interacting with your enclave applicationのpip3での依存関係のインストールは必要ありません。

Interacting with your enclave Applicationの2番は
```
cargo run --release --bin client -- prepare --values "values.txt" --alias "my-enclave-key"
```
に、3番は
```
cargo run --release --bin client -- submit --ciphertext "string.encrypted" --alias "my-enclave-key"
```
に置き換えてください。

Validating enclave attestation-based KMS key policy conditionsの1番は`just stop`に置き換えてください。

Validating enclave attestation-based KMS key policy conditionsの2番は
```
just run data-processing
```
に置き換えてください。`debug-mode`は必要ありません。

Modifying the enclave application codeのserver.rsの変更(workshopではserver.pyを編集している)は、70~79行目を以下のように変更してください。
```rust
// 変更前
let plaintext = String::from_utf8_lossy(&decoded).to_string();
let last_four = plaintext.chars().rev().take(4).collect::<String>().chars().rev().collect();
Ok(last_four)
// 変更後
let plaintext = String::from_utf8_lossy(&decoded).to_string();
Ok(plaintext)
```

Validating enclave attestation-based KMS key policy conditionsの1番から4番までを`just stop`を実行した後、
```
just run data-processing-modified
```
に置き換えてください。
