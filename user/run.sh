#!/bin/bash

echo "00_hello_world"
file target/riscv64gc-unknown-none-elf/release/00_hello_world
qemu-riscv64 target/riscv64gc-unknown-none-elf/release/00_hello_world
echo ""

echo "01_store_fault"
file target/riscv64gc-unknown-none-elf/release/01_store_fault
qemu-riscv64 target/riscv64gc-unknown-none-elf/release/01_store_fault
echo ""

echo "02_power"
file target/riscv64gc-unknown-none-elf/release/02_power
qemu-riscv64 target/riscv64gc-unknown-none-elf/release/02_power
echo ""

echo "03_priv_inst"
file target/riscv64gc-unknown-none-elf/release/03_priv_inst
qemu-riscv64 target/riscv64gc-unknown-none-elf/release/03_priv_inst
echo ""

echo "04_priv_csr"
file target/riscv64gc-unknown-none-elf/release/04_priv_csr
qemu-riscv64 target/riscv64gc-unknown-none-elf/release/04_priv_csr
echo ""