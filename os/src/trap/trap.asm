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
    # 读取 sstatus，spec 的值到 t0，t1 寄存器并保存到内核栈中
    csrr t0, sstatus
    csrr t1, spec
    sd t0, 32 * 8(sp)
    sd t1, 33 * 8(sp)
    # 读取 sscratch 的值保存到内核栈，sscratch 的值是用户栈的地址
    csrr t2, sscratch
    sd t2, 2 * 8(sp)
    # 保存内核栈栈顶值
    mv a0, sp
    call trap_handler

