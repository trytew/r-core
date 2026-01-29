///
/// 任务上下文
///
/// @author: tryte
///
/// @date: 2025/12/17
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,      // 记录任务恢复后需要执行的下一条指令地址
    sp: usize,      // 应用栈指针
    s: [usize; 12], // s0~s11 寄存器的值
}

impl TaskContext {
    ///
    /// 初始化
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/17
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    ///
    /// 设置 __restore 函数位置
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/17
    pub fn goto_restore(k_stack_ptr: usize) -> Self {
        unsafe extern "C" {
            unsafe fn __restore();
        }
        Self {
            ra: __restore as *const () as usize,
            sp: k_stack_ptr,
            s: [0; 12],
        }
    }
}
