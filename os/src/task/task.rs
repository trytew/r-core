use crate::task::context::TaskContext;

///
/// 任务状态（进程状态）
///
/// @author: tryte
///
/// @date: 2025/12/18
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit, // 未初始化
    Ready, // 待运行
    Running, // 运行中
    Exited, // 已退出
}

///
/// 任务控制块
///
/// @author: tryte
///
/// @date: 2025/12/18
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus, // 任务状态（进程状态）
    pub task_cx: TaskContext, // 任务上下文（进程上下文）
}