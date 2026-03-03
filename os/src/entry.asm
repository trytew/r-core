    .section .text.entry
    .globl _start
_start:
    # 设置内核栈起始位置
    la sp, boot_stack_top
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    # 内核栈，在内核启动开始一直使用的栈，大小为 16 * 4kb
    .space 4096 * 16

    # 最高位为栈顶
    .globl boot_stack_top
boot_stack_top: