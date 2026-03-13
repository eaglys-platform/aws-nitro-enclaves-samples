// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT-0

use std::thread;
use std::time::Duration;

fn main() {
    let mut count = 1;
    loop {
        println!("[{:4}] Hello from the enclave side!", count);
        count += 1;
        thread::sleep(Duration::from_secs(5));
    }
}
