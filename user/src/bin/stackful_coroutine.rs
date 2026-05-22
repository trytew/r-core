#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec;
use alloc::vec::Vec;
use core::arch::naked_asm;
use user_lib::exit;

/// 全局协程运行时
static mut RUNTIME: usize = 0;

const DEFAULT_STACK_SIZE: usize = 4096;

const MAX_TASKS: usize = 5;

#[derive(PartialEq, Eq, Debug)]
enum State {
    Available,
    Running,
    Ready,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct TaskContext {
    /// ra: 指令执行地址
    x1: u64,
    /// sp: 栈顶
    x2: u64,
    /// s0, fp
    x8: u64,
    /// s1
    x9: u64,
    /// x18-27: s2-11
    x18: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    /// 新的执行地址
    nx1: u64,
}

pub struct Task {
    id: usize,
    stack: Vec<u8>,
    ctx: TaskContext,
    state: State,
}

impl Task {
    ///
    /// 创建空协程，设置为可设置运行执行函数
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    fn new(id: usize) -> Self {
        Task {
            id,
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: TaskContext::default(),
            state: State::Available,
        }
    }
}

pub struct Runtime {
    tasks: Vec<Task>,
    current: usize,
}

impl Runtime {
    ///
    /// 实例化运行时
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    pub fn new() -> Self {
        // 创建空协程上下文
        let base_task = Task {
            id: 0,
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: TaskContext::default(),
            state: State::Running,
        };

        // 将协程添加进任务列表
        let mut tasks = vec![base_task];
        let mut available_tasks: Vec<Task> = (1..MAX_TASKS).map(|i| Task::new(i)).collect();
        tasks.append(&mut available_tasks);

        Runtime { tasks, current: 0 }
    }

    ///
    /// 初始化全局协程运行时
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    ///
    /// 切换运行协程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    #[inline(never)]
    fn t_yield(&mut self) -> bool {
        // 获取下一个运行的协程位置
        let mut pos = self.current;
        while self.tasks[pos].state != State::Ready {
            pos += 1;
            if pos == self.tasks.len() {
                pos = 0;
            }
            if pos == self.current {
                return false;
            }
        }

        // 当前协程如果不是可设置执行函数的状态则设置待运行等待下次继续执行
        if self.tasks[self.current].state != State::Available {
            self.tasks[self.current].state = State::Ready;
        }

        // 设置下个协程为运行中
        self.tasks[pos].state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            // 切换协程
            switch(&mut self.tasks[old_pos].ctx, &self.tasks[pos].ctx);
        }

        // 只要还有待运行的协程就返回true，返回false代表所有协程的函数都执行完了
        self.tasks.len() > 0
    }

    pub fn run(&mut self) {
        // 让出时间片给协程执行
        while self.t_yield() {}
        println!("All task finished!");
    }

    ///
    /// 协程结束
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    pub fn t_return(&mut self) {
        if self.current != 0 {
            // 将当前协程设置为可用
            self.tasks[self.current].state = State::Available;
            self.t_yield();
        }
    }

    ///
    /// 给协程设置任务
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/22
    pub fn spawn(&mut self, f: fn()) {
        // 查找可运行的协程
        let available = self
            .tasks
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("no available task.");

        println!("RUNTIME: spawn task {}", available.id);
        // 获取协程栈长度
        let size = available.stack.len();
        unsafe {
            // 移动到数组结尾，也就是栈顶
            let s_ptr = available.stack.as_mut_ptr().offset(size as isize);

            // 将栈顶内存对齐，若不对齐，那么栈顶位置往下移
            let s_ptr = (s_ptr as usize & !7) as *mut u8;

            // 设置协程执行函数结束地址
            available.ctx.x1 = guard as *const () as u64;
            // 设置协程的入口为执行函数入口
            available.ctx.nx1 = f as u64;
            // 栈顶下移，为32个寄存器的值保留位置
            available.ctx.x2 = s_ptr.offset(-32) as u64;
        }

        // 将协程设置为待运行状态
        available.state = State::Ready;
    }
}

fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_return();
    }
}

pub fn yield_task() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn switch(old: *mut TaskContext, new: *const TaskContext) {
    // a0: old, a1: new
    naked_asm!(
        "
        sd x1, 0x00(a0)
        sd x2, 0x08(a0)
        sd x8, 0x10(a0)
        sd x9, 0x18(a0)
        sd x18, 0x20(a0)
        sd x19, 0x28(a0)
        sd x20, 0x30(a0)
        sd x21, 0x38(a0)
        sd x22, 0x40(a0)
        sd x23, 0x48(a0)
        sd x24, 0x50(a0)
        sd x25, 0x58(a0)
        sd x26, 0x60(a0)
        sd x27, 0x68(a0)
        sd x1, 0x70(a0)

        ld x1, 0x00(a1)
        ld x2, 0x08(a1)
        ld x8, 0x10(a1)
        ld x9, 0x18(a1)
        ld x18, 0x20(a1)
        ld x19, 0x28(a1)
        ld x20, 0x30(a1)
        ld x21, 0x38(a1)
        ld x22, 0x40(a1)
        ld x23, 0x48(a1)
        ld x24, 0x50(a1)
        ld x25, 0x58(a1)
        ld x26, 0x60(a1)
        ld x27, 0x68(a1)
        ld t0, 0x70(a1)

        jr t0
        "
    );
}

#[unsafe(no_mangle)]
pub fn main() {
    println!("stackful_coroutine begin...");
    println!("TASK 0(Runtime) STARTING");

    // 创建协程运行时
    let mut runtime = Runtime::new();

    // 初始化协程运行时
    runtime.init();

    runtime.spawn(|| {
        println!("TASK 1 STARTING");
        let id = 1;
        for i in 0..4 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 1 FINISHED");
    });

    runtime.spawn(|| {
        println!("TASK 2 STARTING");
        let id = 2;
        for i in 0..8 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 2 FINISHED");
    });

    runtime.spawn(|| {
        println!("TASK 3 STARTING");
        let id = 3;
        for i in 0..12 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 3 FINISHED");
    });

    runtime.spawn(|| {
        println!("TASK 4 STARTING");
        let id = 4;
        for i in 0..16 {
            println!("task: {} counter: {}", id, i);
            yield_task();
        }
        println!("TASK 4 FINISHED");
    });

    // 开始运行协程
    runtime.run();

    println!("stackful_coroutine PASSED");
    exit(0);
}
