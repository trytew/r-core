#!/bin/bash

rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/os \
  -O binary target/riscv64gc-unknown-none-elf/release/os.bin