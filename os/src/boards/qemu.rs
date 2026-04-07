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

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
