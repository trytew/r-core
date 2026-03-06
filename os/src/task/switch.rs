use crate::task::context::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.asm"));

unsafe extern "C" {
    ///
    /// 封装汇编函数 __switch
    /// 作用：保存当前进程上下文，切换成下一个进程上下文
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    pub unsafe fn __switch(
        current_task_cx_ptr: *mut TaskContext,
        next_task_cx_ptr: *const TaskContext,
    );
}
