    .align 3
    .section .data
    .globl _num_app
_num_app:
    .quard 5
    .quard app_0_start
    .quard app_1_start
    .quard app_2_start
    .quard app_3_start
    .quard app_4_start
    .quard app_4_end

    .section .data
    .globl app_0_start
    .globl app_0_end
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/00_hello_word.bin"
app_0_end:

    .section .data
    .globl app_1_start
    .globl app_1_end
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/01_store_fault.bin"
app_1_end:

    .section .data
    .globl app_2_start
    .globl app_2_end
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/02_power.bin"
app_2_end:

    .section .data
    .globl app_3_start
    .globl app_3_end
app_3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/03_priv_inst.bin"
app_3_end:

    .section .data
    .globl app_4_start
    .globl app_4_end
app_4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/04_priv_csr.bin"
app_4_end:
