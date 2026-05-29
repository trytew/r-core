#![no_std]
#![feature(linkage)]
#![feature(alloc_error_handler)]
extern crate alloc;

pub mod console;
mod lang_items;
mod signal;
mod syscall;

use alloc::vec::Vec;
use bitflags::bitflags;
use buddy_system_allocator::LockedHeap;
use core::ptr::addr_of_mut;
pub use signal::*;
use syscall::*;

const USER_HEAP_SIZE: usize = 32768;

///
/// 堆空间大小
///
/// @author: tryte
///
/// @date: 2026/3/10
static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout)
}

// 使用 Rust 的宏将 _start 这段代码编译后的汇编代码中放在一个名为 .text.entry 的代码段中，
// 在 linker.ld 脚本中指定了 .text.entry 在最开始的位置
// 方便我们在后续链接的时候调整它的位置使得它能够作为用户库的入口
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    // 设置用户进程的堆分配器
    unsafe {
        HEAP.lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, USER_HEAP_SIZE);
    }
    let mut v: Vec<&'static str> = Vec::new();
    // argc 代表参数的个数
    for i in 0..argc {
        // 获取第一个参数内容的起始地址
        let str_start =
            unsafe { ((argv + i * size_of::<usize>()) as *const usize).read_volatile() };

        // 读取内容，\0 结束
        let len = (0_usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();

        // 添加参数内容到切片
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    exit(main(argc, v.as_slice()));
}

// 使用 Rust 的宏将其函数符号 main 标志为弱链接。
// 这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序都有 main 符号，但由于 lib.rs 中的 main 符号是弱链接，
// 链接器会使用 bin 目录下的应用主逻辑作为 main 。这里我们主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，
// 那么编译也能够通过，但会在运行时报错
#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot found main");
}

bitflags! {
pub struct OpenFlags:u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

///
/// 打开文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}

///
/// 关闭文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

///
/// 创建管道
///
/// @author: tryte
///
/// @date: 2026/4/17
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

///
/// 读
///
/// @author: tryte
///
/// @date: 2026/3/10
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

///
/// 写
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

///
/// 休眠
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sleep(sleep_ms: usize) {
    sys_sleep(sleep_ms);
}

///
/// 让出时间片
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn yield_() -> isize {
    sys_yield()
}

///
/// 发送信号
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn kill(pid: usize, signum: i32) -> isize {
    sys_kill(pid, signum)
}

///
/// 设置信号执行的动作
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sigaction(
    signum: i32,
    action: Option<&SignalAction>,
    old_action: Option<&mut SignalAction>,
) -> isize {
    sys_sigaction(
        signum,
        action.map_or(core::ptr::null(), |a| a),
        old_action.map_or(core::ptr::null_mut(), |a| a),
    )
}

///
/// 屏蔽信号
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sig_proc_mask(mask: u32) -> isize {
    sys_sig_proc_mask(mask)
}

///
/// 信号执行动作结束返回
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sig_return() -> isize {
    sys_sig_return()
}

///
/// 获取时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time() -> isize {
    sys_get_time()
}

///
/// 获取进程ID
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn getpid() -> isize {
    sys_getpid()
}

///
/// 创建子进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn fork() -> isize {
    sys_fork()
}

///
/// 执行新进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}

///
/// 等待所有进程退出
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => {
                return exit_pid;
            }
        }
    }
}

///
/// 等待指定进程退出
///
/// @author: tryte
///
/// @date: 2026/3/9
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => {
                return exit_pid;
            }
        }
    }
}

///
/// 非阻塞等待进程退出
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}

///
/// 创建线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn thread_create(entry: usize, arg: usize) -> isize {
    sys_thread_create(entry, arg)
}

///
/// 获取线程ID
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn get_tid() -> isize {
    sys_get_tid()
}

///
/// 等待线程退出
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn wait_tid(tid: usize) -> isize {
    loop {
        match sys_wait_tid(tid) {
            -2 => {
                yield_();
            }
            exit_code => return exit_code,
        }
    }
}

///
/// 创建线程锁（非阻塞）
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn mutex_create() -> isize {
    sys_mutex_create(false)
}

///
/// 创建线程锁（阻塞等待锁）
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn mutex_blocking_create() -> isize {
    sys_mutex_create(true)
}

///
/// 上锁
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn mutex_lock(mutex_id: usize) {
    sys_mutex_lock(mutex_id);
}

///
/// 解锁
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn mutex_unlock(mutex_id: usize) {
    sys_mutex_unlock(mutex_id);
}

///
/// 创建信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn semaphore_create(res_count: usize) -> isize {
    sys_semaphore_create(res_count)
}

///
/// 增加信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn semaphore_up(sem_id: usize) -> isize {
    sys_semaphore_up(sem_id)
}

///
/// 减少信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn semaphore_down(sem_id: usize) -> isize {
    sys_semaphore_down(sem_id)
}

///
/// 创建条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn condvar_create() -> isize {
    sys_condvar_create()
}

///
/// 释放条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn condvar_signal(condvar_id: usize) -> isize {
    sys_condvar_signal(condvar_id)
}

///
/// 等待条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    sys_condvar_wait(condvar_id, mutex_id)
}

#[macro_export]
macro_rules! v_store {
    ($var: expr,$value:expr) => {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!($var), $value);
        }
    };
}

#[macro_export]
macro_rules! v_load {
    ($var:expr) => {
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!($var)) }
    };
}

#[macro_export]
macro_rules! memory_fence {
    () => {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst)
    };
}
