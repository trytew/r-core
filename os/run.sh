#!/bin/bash

#qemu-system-riscv64 -machine virt \
#  -nographic \
#  -bios ./bootloader/rustsbi-qemu.bin \
#  -device loader,file=target/riscv64gc-unknown-none-elf/release/r-core.bin,addr=0x80200000 \
#  -s -S

cd ../user/ && \
./build.sh && \
cd ../os/ && \
./build.sh && \
qemu-system-riscv64 -machine virt \
  -nographic \
  -bios ./bootloader/rustsbi-qemu.bin \
  -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000