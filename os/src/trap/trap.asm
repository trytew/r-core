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

    # 当陷入 trap 的时候执行的逻辑
__alltraps:
    # 先将用户态下寄存器的内容记录到内核栈
    # 将 sscratch 当前的值读到 sp 寄存器中，然后将 sp 寄存器的旧值写入该 sscratch，这里起到的是交换 sscratch 和 sp 的效果，
    # 即将用户栈的栈顶记录到 sscratch，sp 刷新成内核栈栈顶
    # 注：
    #   sscratch 中间寄存器，保存切换特权级时数据使用
    #   sp 栈顶寄存器，记录栈顶地址
    csrrw sp, sscratch, sp
    # 先将 sp 寄存器的值下移34个8字节，代表栈已使用34个栈帧，方便下面保存寄存器的代码直接使用向上保存操作，
    # 同时指针的读取是从低到高的，而栈的使用是从高到底，这里先下移也是为了构建内核上下文 TrapContext 的值
    addi sp, sp, -34 * 8
    # 保存 x1，x3，x5-x31 寄存器的值，x0 永久为0，x2 是 sp 寄存器，x4 是 tp 特殊用途寄存器，一般不会使用，因此这三个寄存器不做保存操作
    sd x1, 1 * 8(sp)
    sd x3, 3 * 8(sp)
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
    # 传入内核上下文，trap_handler 形参值，在 RISC-V ABI 中 a0 寄存器存放函数第一个入参值以及第一个返回值
    # 这一个版本每个应用都对应一个内核栈，a0 传入当前内核栈是为了给 __switch 切换内核栈使用
    mv a0, sp
    # 执行 trap 回调
    # 因为有 __switch 函数的加入，trap_handler 会正常返回，再也不会直接退出应用，内核栈内容会在函数返回后正常弹出，sp 也会恢复正常
    call trap_handler

    # 将用户态的寄存器状态恢复，从内核栈的内容中读取
    # __restore 有两种执行时机：
    # 1.当 trap_handler 正常返回之后，会继续执行
    # 2.__restore 同时也是一个函数，可主动调用运行
__restore:
    # 将 a0 寄存器的值移动到 sp 寄存器，即读取内核栈的栈顶
    ## mv sp, a0 # __switch 函数已经将 sp 设置好了，因此不用再执行这句，__restore 也不再需要入参值
    # 将原来保存在内核栈的 t0，t1，t2 的值回写
    ld t0, 32 * 8(sp)
    ld t1, 33 * 8(sp)
    ld t2, 2 * 8(sp)
    # 将 t0，t1，t2 寄存器的值回写到 sstatus，sepc，sscratch 寄存器中
    csrw sstatus, t0
    # 将用户栈中 sepc 记录的地址设置到指令执行寄存器 PC 中，在 sret 指令执行后会跳转到该地址执行，即切换成新应用的起始地址
    csrw sepc, t1
    csrw sscratch, t2

    # 回写通用寄存器的值
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
    # 执行 sret 后会回到用户态，从触发中断后的代码（即 spec 寄存器存放的地址）继续执行
    sret

