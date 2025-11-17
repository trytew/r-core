#!/bin/bash

rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/r-core \
  -O binary target/riscv64gc-unknown-none-elf/release/r-core.bin