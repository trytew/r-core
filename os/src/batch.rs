use crate::println;
use crate::sbi::shutdown;
use crate::sync::UpSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::lazy_static;

// 内核栈大小
const KERNEL_STACK_SIZE: usize = 4096 * 2;
// 用户栈大小
const USER_STACK_SIZE: usize = 4096 * 2;

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
        //       high addr - boot_stack_lower_bound = 8kb
        // |-------------------| 栈底
        // |  TrapContext size |
        // |-------------------| --> sp 栈顶
        // |                   |
        // |                   |
        // |                   |
        // |                   | boot_stack_lower_bound 栈的下限位置
        //       lower addr
        // 栈指针下移，为 cx 分配足够的空间
        let cx_ptr = (self.get_sp() - size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            // 将 cx 的全部内容移动到栈中，*cx_ptr = cx 相当于 memcpy(sp, &cx)
            // 这个时候的 memcpy 操作/指针内容写入 操作是遵循内存写入规则（从低到高），因此 sp 指向的是 cx 结构体的起始位置，如下：
            //       high addr - boot_stack_lower_bound = 8kb
            // |-------------------| 栈底
            // |       sepc        | -- 第34个地址，偏移量 33 * 8（x0 的偏移量是0）
            // |       ....        |
            // |      sstatus      | --> cx 内容
            // |       ....        |
            // |        x1         |
            // |        x0         |
            // |-------------------| --> sp 栈顶
            // |                   |
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

// 最大应用数量
const MAX_APP_NUM: usize = 16;
// 应用运行起始地址
const APP_BASE_ADDRESS: usize = 0x80400000;
// 应用大小
const APP_SIZE_LIMIT: usize = 0x20000;

lazy_static! {
    static ref APP_MANAGEER: UpSafeCell<AppManager> = unsafe {
        UpSafeCell::new({
            unsafe extern "C" {
                fn _num_app();
            }
            // 获取 _num_app 标记的地址（在 linker_app.asm 汇编文件中，该文件由 build.rs 构建程序生成）
            let num_app_ptr = _num_app as *const () as *const usize;
            // 读取应用数量，_num_app 是一个数组，应用数量是第一个元素
            let num_app = num_app_ptr.read_volatile();
            // 创建数组存放每个应用的起始地址
            let mut app_start: [usize;MAX_APP_NUM+1] = [0;MAX_APP_NUM+1];
            // 读取指针内容并按 *const usize 即一个指针地址大小分隔内容返回切片
            let app_start_raw: &[usize] = core::slice::from_raw_parts(
                // 从第二地址读取，因为第一个地址存放的是应用数量
                num_app_ptr.add(1),
                num_app+1,
            );
            // 将每个应用的起始地址存入 app_start 数组
            app_start[..=num_app].copy_from_slice(app_start_raw);
            // 初始化应用管理器
            AppManager{
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    ///
    /// 加载应用
    ///
    /// @author: tryte
    ///
    /// @date: 2025/11/29
    fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("All application completed");
            shutdown(false);
        }
        println!("[kernel] Loading app_{}", app_id);
        unsafe {
            // 清空上个应用的内容
            core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
            // 读取下个应用的内容
            let app_src = core::slice::from_raw_parts(
                self.app_start[app_id] as *const u8,
                self.app_start[app_id + 1] - self.app_start[app_id],
            );
            // 将下个应用的内容加载到地址 0x80400000
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
            app_dst.copy_from_slice(app_src);
            // 关于获取指令内存的内存栅栏
            // 保证随后的指令提取必须
            // 观察所有以前对指令存储器的写入。
            // 因此，fence.i必须在我们加载之后执行，它的功能是保证 在它之后的取指过程必须能够看到在它之前的所有对于取指内存区域的修改 （刷新缓存）
            // 将下一个应用程序的代码放入指令存储器。
            asm!("fence.i");
        }
    }

    ///
    /// 打印应用信息
    ///
    /// @author: tryte
    ///
    /// @date: 2025/11/28
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1],
            );
        }
        println!("");
    }

    ///
    /// 获取当前应用
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/1
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    ///
    /// 移动到下一个应用
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/1
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

///
/// 初始化运行
///
/// @author: tryte
///
/// @date: 2025/12/2
pub fn init() {
    print_app_info();
}

///
/// 打印应用信息
///
/// @author: tryte
///
/// @date: 2025/12/2
pub fn print_app_info() {
    APP_MANAGEER.exclusive_access().print_app_info();
}

///
/// 运行下一个应用
///
/// @author: tryte
///
/// @date: 2025/12/2
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGEER.exclusive_access();
    // 获取当前应用
    let current_app = app_manager.get_current_app();
    // 加载应用程序到内存中
    app_manager.load_app(current_app);
    // 将当前应用指针指向到下一个应用
    app_manager.move_to_next_app();
    // 主动释放 app_manager 的引用，因为在执行完 __restore 后会切换到用户态，栈也会切换到用户栈，这个时候在内核栈记录的 app_manager 引用将无法正确释放，
    // 无法释放 app_manager 引用会导致引用计数器计量数错误
    drop(app_manager);
    unsafe extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        // 恢复用户栈并将特权级切换成用户级
        __restore(
            // 根据用户态的栈信息和寄存器信息创建 Trap Context 并压入内核栈中
            KERNEL_STACK.push_context(
                TrapContext::app_init_context(APP_BASE_ADDRESS, USER_STACK.get_sp())
            ) as *const TrapContext as usize,
        );
    }
    // __restore 函数在正常情况下已经结束 S 特权级运行直接返回了
    panic!("Unreachable in batch::run_current_app!");
}