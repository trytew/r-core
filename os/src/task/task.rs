use crate::config::{TRAP_CONTEXT, kernel_stack_position};
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum, VirtAddr};
use crate::println;
use crate::task::context::TaskContext;
use crate::trap::{TrapContext, trap_handler};

///
/// 应用状态
///
/// “内核里任务切换时”，内核线程自己的现场
/// 保存当前内核执行流的状态，方便以后从这里继续跑
///
/// @author: tryte
///
/// @date: 2025/12/18
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,   // 待运行
    Running, // 运行中
    Exited,  // 已退出
}

///
/// 任务控制块
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskControlBlock {
    pub task_status: TaskStatus,  // 应用状态
    pub task_cx: TaskContext,     // 应用上下文
    pub memory_set: MemorySet,    // 应用内存区域
    pub trap_cx_ppn: PhysPageNum, // 应用“陷入”上下文的物理地址
    #[allow(unused)]
    pub base_size: usize,
    pub heap_bottom: usize,
    pub program_brk: usize,
}

impl TaskControlBlock {
    ///
    /// 返回应用“陷入”上下文的物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/30
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    ///
    /// 获取用户空间的 MMU 设置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/31
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    ///
    /// 创建应用控制器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/30
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 获取应用内存区域集合
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 获取应用“陷入”上下文的物理地址
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // 设置应用状态为待运行
        let task_status = TaskStatus::Ready;

        // 创建内核栈
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        // 创建应用控制器
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 创建“陷入”上下文，这里看起来是在直接操作物理地址，但是因为在内核态的情况下（已经使用了内核页表的 MMU 设置），所以这里还是使用
        // 虚拟内存地址访问，因为内核态下页表的虚拟内存地址使用的恒等映射。
        // 这里记录“陷入”上下文的物理地址也是因为这个上下文只在内核态下会用到
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    ///
    /// 堆虚拟地址空间管理
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/2
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_break = self.program_brk as isize + size as isize;
        if new_break < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            // 回收堆空间
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_break as usize))
        } else {
            // 分配堆空间
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_break as usize))
        };
        if result {
            self.program_brk = new_break as usize;
            Some(old_break)
        } else {
            None
        }
    }
}
