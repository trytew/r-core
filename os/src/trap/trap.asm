.altmacro
# 汇编宏
.macro SAVE_GP n
    # 等效于 xn, n * 8(sp)
    sd x\n, \n * 8(sp)
.endm

.macro LOAD_GP n
    ld x\n, \n * 8(sp)
.endm

    .section .text
    .globl __alltraps
    .globl __restore
    .align 2

__alltraps:
    # 将 sscratch 当前的值读到 sp 寄存器中，然后将 sp 寄存器的旧值写入该 sscratch，这里起到的是交换 sscratch 和 sp 的效果
    # sscratch 中间寄存器，保存切换特权级时数据使用
    # sp 栈顶寄存器，记录栈顶地址
    csrrw sp, sscratch, sp
    # 先将 sp 寄存器的值下移34个8字节，代表栈已使用34个栈帧，方便下面保存寄存器的代码直接使用向上保存操作，这里使用的内核栈
    addi sp, sp, -34 * 8
    # 保存 x1，x3，x5-x31 寄存器的值，x0 永久为0，x2 是 sp 寄存器，x4 是 tp 特殊用途寄存器，一般不会使用，因此这三个寄存器不做保存操作
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    # 循环调用汇编宏
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 读取 sstatus，sepc 的值到 t0，t1 寄存器并保存到内核栈中
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32 * 8(sp)
    sd t1, 33 * 8(sp)
    # 读取 sscratch 的值保存到内核栈，sscratch 的值是用户栈的地址
    csrr t2, sscratch
    sd t2, 2 * 8(sp)
    # 保存内核栈栈顶值
    mv a0, sp
    call trap_handler

    # 当 trap_handler 返回之后，使用 __restore 从保存在内核栈上的 Trap 上下文恢复寄存器。最后通过一条 sret 指令回到应用程序执行。
    # __restore 同时也是一个函数，可独立运行
__restore:
    # 将 a0 寄存器的值移动到 sp 寄存器
    mv sp, a0
    # 将 sp 移动 32 个 8 位后地址的值复制给 t0 寄存器
    ld t0, 32 * 8(sp)
    ld t1, 33 * 8(sp)
    ld t2, 2 * 8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    ld x1, 1 * 8(sp)
    ld x3, 3 * 8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n + 1
    .endr
    # 在此之前，sp 指向保存了 Trap 上下文之后的内核栈栈顶， sscratch 指向用户栈栈顶
    # 在这在内核栈上回收 Trap 上下文所占用的内存，回归进入 Trap 之前的内核栈栈顶
    addi sp, sp, 34 * 8
    # 再次交换 sscratch 和 sp，现在 sp 重新指向用户栈栈顶，sscratch 也依然保存进入 Trap 之前的状态并指向内核栈栈顶
    csrrw sp, sscratch, sp
    sret

