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

/// 虚拟地址最高位
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// “陷入”处理函数地址
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

///
/// 获取应用内核栈底和栈顶
///
/// @author: tryte
///
/// @date: 2026/1/30
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub use crate::boards::*;

/// 最大 app 数量
pub const MAX_APP_NUM: usize = 9;

/// app 内容起始地址，单位：字节
pub const APP_BASE_ADDRESS: usize = 0x8040_0000;

/// app 内容大小限制，单位：字节
pub const APP_SIZE_LIMIT: usize = 0x2_0000;
