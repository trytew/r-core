// 用户栈大小
pub const USER_STACK_SIZE: usize = 4096 * 2;

// 内核栈大小
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;

// 最大 app 数量
pub const MAX_APP_NUM: usize = 4;

// app 内容起始地址，单位：字节
pub const APP_BASE_ADDRESS: usize = 0x8040_0000;

// app 内容大小限制，单位：字节
pub const APP_SIZE_LIMIT: usize = 0x2_0000;
