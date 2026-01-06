    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    # 设置每个栈大小为 64kb
    .space 4096 * 16

    # 最高位为栈顶
    .globl boot_stack_top
boot_stack_top: