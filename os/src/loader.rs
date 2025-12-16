use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

// 初始化内核栈
static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

// 初始化用户栈
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

///
/// 内核栈
///
/// @author: tryte
///
/// @date: 2025/12/2
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    ///
    /// 获取栈顶
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/2
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    ///
    ///  将 Trap 上下文压入内核栈中
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/10
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        // 返回内核栈的栈顶
        //       high addr
        // |                   | 栈底
        // |        8kb        |
        // |-------------------|
        // |  TrapContext size |
        // |-------------------| --> sp 栈顶
        // |                   |
        // |                   | boot_stack_lower_bound 栈的下限位置
        //       lower addr
        // 栈指针下移，为 cx 分配足够的空间
        let cx_ptr = (self.get_sp() - size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            // 将 cx 的全部内容移动到栈中，*cx_ptr = cx 相当于 memcpy(sp, &cx)
            // 这个时候的 memcpy 操作/指针内容写入 操作是遵循内存写入规则（从低到高），因此 sp 指向的是 cx 结构体的起始位置，如下：
            //       high addr
            // |                   | 栈底
            // |        8kb        |
            // |-------------------|
            // |       sepc        | -- 第34个地址，偏移量 33 * 8（x0 的偏移量是0）
            // |       ....        |
            // |      sstatus      | --> cx 内容
            // |       ....        |
            // |        x1         |
            // |        x0         |
            // |-------------------| --> sp 栈顶
            // |                   |
            // |                   | boot_stack_lower_bound 栈的下限位置
            //       lower addr
            *cx_ptr = cx;
            cx_ptr.as_mut().unwrap()
        }
    }
}

///
/// 用户栈
///
/// @author: tryte
///
/// @date: 2025/12/2
#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    ///
    /// 获取栈顶
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/2
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

///
/// 获取应用数量
///
/// @author: tryte
///
/// @date: 2025/12/16
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        fn _num_app();
    }
    unsafe {
        (_num_app as *const usize).read_volatile()
    }
}

///
/// 获取 app 内容起始地址
///
/// @author: tryte
///
/// @date: 2025/12/16
fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

///
/// 加载应用程序
///
/// @author: tryte
///
/// @date: 2025/12/16
pub fn load_apps() {
    unsafe extern "C" {
        fn _num_app();
    }

    let num_app_ptr = _num_app as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };

    // 加载 app
    for i in 0..num_app {

        // 获取应用起始地址
        let base_i = get_base_i(i);

        // 清空内存区域数据
        (base_i..base_i + APP_SIZE_LIMIT).for_each(|addr| {
            unsafe {
                (addr as *mut u8).write_volatile(0);
            }
        });

        // 将 app 内容加载到指定内存地址
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe {
            core::slice::from_raw_parts_mut(base_i as *mut u8, src.len())
        };
        dst.copy_from_slice(src);
    }

    // 刷新指令视角（刷新指令缓存），保证读取的是新的指令
    // 指令缓存地方：
    //  1.指令缓存（I-cache）
    //  2.流水线预取
    //  3.取指缓冲
    // Memory fence about fetching the instruction memory
    // It is guaranteed that a subsequent instruction fetch must
    // observe all previous writes to the instruction memory.
    // Therefore, fence.i must be executed after we have loaded
    // the code of the next app into the instruction memory.
    // See also: riscv non-priv spec chapter 3, 'Zifencei' extension.
    unsafe {
        asm!("fence.i");
    }
}