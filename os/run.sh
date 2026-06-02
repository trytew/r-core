#!/bin/bash

#qemu-system-riscv64 -machine virt \
#  -nographic \
#  -bios ./bootloader/rustsbi-qemu.bin \
#  -device loader,file=target/riscv64gc-unknown-none-elf/release/r-core.bin,addr=0x80200000 \
#  -s -S

cd ../user/ && \
./build.sh && \
cd ../easy-fs-use/
./build.sh
cd ../os/ && \
./build.sh && \
qemu-system-riscv64 \
  -machine virt \
  -nographic \
  -bios ./bootloader/rustsbi-qemu.bin \
  -device loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000 \
  -drive file=../user/target/riscv64gc-unknown-none-elf/release/fs.img,if=none,format=raw,id=x0 \
  -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
  -device virtio-keyboard-device