.altmacro
.macro SAVE_SN n
    sd s\n, (\n + 2)*8(a0)
.endm

.macro LOAD_SN n
    ld s\n, (\n + 2)*8(a1)
.endm

    .section .text
    .globl __switch
__switch:
    # 阶段【1】
    # switch(
    #     current_task_cx_ptr: *mut TaskContext,
    #     next_task_cx_ptr: *const TaskContext,
    # )

    # 阶段【2】
    # 保存当前应用的内核栈栈顶，将 sp 的值保存到 a0 + offset
    sd sp, 1 * 8(a0)
    # 保存当前应用的 ra，ra 寄存器记录了函数调用返回后下一个指令的指令地址，因此应用切换时需要记录当前应用恢复后需要执行的指令地址
    sd ra, 0 * 8(a0)
    # 保存当前应用的 s0~s11 寄存器
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr

    # 阶段【3】
    # 加载下一个应用的 ra
    ld ra, 0 * 8(a1)
    # 加载下一个应用的 s0~s11 寄存器
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    # 加载下一个应用的应用栈
    ld sp, 1 * 8(a1)

    # 阶段【4】跳转执行 ra 记录的指令
    ret