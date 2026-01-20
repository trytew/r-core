#!/bin/bash

cargo build --release

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/00_hello_world \
  -O binary target/riscv64gc-unknown-none-elf/release/00_hello_world.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/01_store_fault \
  -O binary target/riscv64gc-unknown-none-elf/release/01_store_fault.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/02_power \
  -O binary target/riscv64gc-unknown-none-elf/release/02_power.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/03_priv_inst \
  -O binary target/riscv64gc-unknown-none-elf/release/03_priv_inst.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/04_priv_csr \
  -O binary target/riscv64gc-unknown-none-elf/release/04_priv_csr.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/05_power_3 \
  -O binary target/riscv64gc-unknown-none-elf/release/05_power_3.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/06_power_5 \
  -O binary target/riscv64gc-unknown-none-elf/release/06_power_5.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/07_power_7 \
  -O binary target/riscv64gc-unknown-none-elf/release/07_power_7.bin

rust-objcopy --binary-architecture=riscv64 --strip-all target/riscv64gc-unknown-none-elf/release/08_sleep \
  -O binary target/riscv64gc-unknown-none-elf/release/08_sleep.bin