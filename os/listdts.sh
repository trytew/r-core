#!/bin/bash

# 查看 qemu 外设
qemu-system-riscv64 -machine virt -machine dumpdtb=riscv64-virt.dtb -bios default
dtc -I dtb -O dts -o riscv64-virt.dts riscv64-virt.dtb
