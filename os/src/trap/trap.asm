.altmacro
# 汇编宏
.macro SAVE_GP n
    # 等效于 xn, n * 8(sp)
    sd x\n, \n * 8(sp)
.endm

.macro LOAD_GP n
    ld x\n, \n * 8(sp)
.endm

    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2

    # 当陷入 trap 的时候执行的逻辑
    # 内核态和用户态的更迭在陷入 trap （系统调用/中断/异常）就会被触发，从用户态切换到内核态。
    # 用户态（U-mode）和内核态（S-mode）是特权级的切换，由 CPU 自动完成，与 sp 指向无关，不是 sp 执行内核栈就代表处在内核态
    # sscratch 只是一个用于临时存储 sp 状态的寄存器，无特殊作用
    # sp 只是一个普通寄存器，它的栈顶指针意义是由 ABI 赋予的，并不是硬件特指，所有符合 ABI 规范的语言都会使用 sp 作为栈顶指针寄存器
    # 栈对硬件来说只是一段普通的内存，CPU 也不会意识到 栈 的存在，栈的意义是由软件赋予的
__alltraps:
    # 先将用户态下寄存器的内容记录到应用的 TrapContext，与前面的教程不一样的是应用的内核栈和应用的 TrapContext 是分开的
    #
    # 将 sscratch 当前的值读到 sp 寄存器中，然后将 sp 寄存器的旧值写入该 sscratch，这里起到的是交换 sscratch 和 sp 的效果，
    # 就是将用户栈的栈顶记录到 sscratch，sp 刷新成应用的 TrapContext
    # 注：
    #   sscratch 中间寄存器，保存切换特权级时数据使用
    #   sp 栈顶寄存器，记录栈顶地址
    csrrw sp, sscratch, sp
    # 保存 x1，x3，x5-x31 寄存器的值，x0 永久为0，x2 是 sp 寄存器，x4 是 tp 特殊用途寄存器，一般不会使用，因此这三个寄存器不做保存操作
    sd x1, 1 * 8(sp)
    sd x3, 3 * 8(sp)
    # 循环调用汇编宏
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 读取 sstatus，sepc 的值到 t0，t1 寄存器并保存到 TrapContext 中
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32 * 8(sp)
    sd t1, 33 * 8(sp)
    # 读取 sscratch 的值保存到 TrapContext，sscratch 的值是用户栈的地址
    csrr t2, sscratch
    sd t2, 2 * 8(sp)
    # 加载内核 stap 到 t0
    ld t0, 34 * 8(sp)
    # 加载 trap_handler 到 t1
    ld t1, 36 * 8(sp)
    # 移动到内核栈
    ld sp, 35 * 8(sp)
    # 切换到内核空间
    csrw satp, t0
    sfence.vma
    # 跳转到 trap_handler
    jr t1

    # 将用户态的寄存器状态恢复，从 TrapContext 的内容中读取
    # __restore 有两种执行时机：
    # 1.当 trap_handler 正常返回之后，会继续执行
    # 2.__restore 同时也是一个函数，可主动调用运行
__restore:
    # a0: 当前应用的寄存器状态上下文 TrapContext; a1: 用户空间地址
    # 切换到用户空间
    csrw satp, a1
    # 刷新虚拟内存地址快表内容
    sfence.vma
    # 存入当前应用的寄存器状态上下文 TrapContext
    csrw sscratch, a0
    # 将栈顶切换成当前应用的寄存器状态上下文 TrapContext
    mv sp, a0
    # 现在 sp 栈顶指针指向用户态的上下文 TrapContext，开始恢复用户态数据
    # 将 32 * 8(sp)、33 * 8(sp) 的值设置到 t0、t1
    ld t0, 32 * 8(sp)
    ld t1, 33 * 8(sp)
    # 将 t0，t1 寄存器的值回写到 sstatus，sepc 寄存器中
    csrw sstatus, t0
    # 将用户栈中 sepc 记录的地址设置到指令执行寄存器 PC 中，在 sret 指令执行后会跳转到该地址执行，即切换成新应用的起始地址
    csrw sepc, t1

    # 回写通用寄存器的值
    ld x1, 1 * 8(sp)
    ld x3, 3 * 8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n + 1
    .endr
    # 将 sp 设置回用户栈
    ld sp, 2 * 8(sp)
    sret

