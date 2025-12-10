use core::arch::asm;
use crate::println;
use crate::sbi::shutdown;
use crate::sync::UpSafeCell;
use lazy_static::lazy_static;
use crate::trap::TrapContext;

// 内核栈大小
const KERNEL_STACK_SIZE: usize = 4096 * 2;
// 用户栈大小
const USER_STACK_SIZE: usize = 4096 * 2;

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

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
    ///  将栈存入 context
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/10
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = ((self.get_sp()) - size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
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
            let num_app_ptr = _num_app as *const () as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize;MAX_APP_NUM+1] = [0;MAX_APP_NUM+1];
            let app_start_raw: &[usize] = core::slice::from_raw_parts(
                num_app_ptr.add(1),
                num_app+1,
            );
            app_start[..=num_app].copy_from_slice(app_start_raw);
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
            // 回收应用内存
            core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
            let app_src = core::slice::from_raw_parts(
                self.app_start[app_id] as *const u8,
                self.app_start[app_id + 1] - self.app_start[app_id],
            );
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
    let current_app = app_manager.get_current_app();
    app_manager.load_app(current_app);
    app_manager.move_to_next_app();
    drop(app_manager);
    unsafe extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(
            TrapContext::app_init_context(APP_BASE_ADDRESS,USER_STACK.get_sp())
        ) as *const TrapContext as usize);
    }
    // __restore 函数在正常情况下已经结束 S 特权级运行直接返回了
    panic!("Unreachable in batch::run_current_app!");
}