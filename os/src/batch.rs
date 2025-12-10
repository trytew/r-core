use core::arch::asm;
use crate::println;
use crate::sbi::shutdown;
use crate::sync::UpSafeCell;
use lazy_static::lazy_static;

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