#!/bin/bash

cargo build --release

./target/release/easy-fs-use -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/