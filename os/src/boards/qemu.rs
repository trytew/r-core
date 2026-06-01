use crate::drivers::{IntrTargetPriority, PLIC};
use riscv::register::sie;

///
/// 时钟频率
///
/// @author: tryte
///
/// @date: 2026/1/9
pub const CLOCK_FREQ: usize = 12_500_000;

///
/// 内存大小（128M）
///
/// @author: tryte
///
/// @date: 2026/1/9
pub const MEMORY_END: usize = 0x88_000_000;

///
/// MMU 相关寄存器
///
/// @author: tryte
///
/// @date: 2026/1/17
pub const MMIO: &[(usize, usize)] = &[
    (0x00_100_000, 0x002_000), // VIRT_TEST/RTC in virt machine
    (0x10_001_000, 0x001_000), // Virtio Block in virt machine
];

/// PLIC 寄存器内存地址，PLIC 是一个 MMIO 设备（Memory Mapped I/O（内存映射 I/O）），即把设备寄存器映射到物理内存地址空间中
pub const VIRT_PLIC: usize = 0xC_000_000;
pub const VIRT_UART: usize = 0x10_000_000;

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;

pub fn device_init() {
    // 实例化 PLIC 寄存器抽象类
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    // 使用硬件线程0，一般来说一个核心对应一个硬件线程，即 CPU 核数
    // hart = Hardware Thread
    let hart_id = 0;

    // 设置 PLIC 中断的优先通知对象为特权级 S-Mode Context
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;
    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);
    // 使能串口中断并设置优先级
    for intr_src_id in [10_usize] {
        // 使能中断
        plic.enable(hart_id, supervisor, intr_src_id);
        // 设置中断优先级
        plic.set_priority(intr_src_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match intr_src_id {
        10 => UART.handler_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}
