#!/bin/bash

riscv64-unknown-elf-gdb \
  -ex 'file target/riscv64gc-unknown-none-elf/release/r-core' \
  -ex 'set arch riscv:rv64' \
  -ex 'target remote localhost:1234'