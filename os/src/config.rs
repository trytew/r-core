/// 用户栈大小
pub const USER_STACK_SIZE: usize = 4096 * 2;

/// 内核栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;

/// 内核堆内存大小 3MB
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;

/// 页容量（4kb）
pub const PAGE_SIZE: usize = 0x1_000;

/// 页容量长度（偏移位数量）12
pub const PAGE_SIZE_BITS: usize = 0x0C;

/// “跳板”虚拟地址
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// “陷入”上下文起始地址
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

/// “陷入”上下文
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub use crate::boards::*;
