use crate::drivers::chardev::CharDevice;
use crate::sync::{CondVar, UpIntrFreeCell};
use crate::task::schedule;
use alloc::collections::VecDeque;
use bitflags::*;
use volatile::{ReadOnly, Volatile, WriteOnly};

bitflags! {
    ///
    /// 中断使能寄存器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub struct IER:u8 {
        /// 接收数据可用（RX ready）中断
        const RX_AVAILABLE = 1 << 0;
        /// 发送缓冲区空（TX ready）中断
        const TX_EMPTY = 1 << 1;
    }

    ///
    /// 线路状态寄存器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub struct LSR:u8 {
        // 是否有数据可读
        const DATA_AVAILABLE = 1 << 0;
        /// 是否可以发送（THR是否为空）
        const THR_EMPTY = 1 << 5;
    }

    ///
    /// 调制解调器控制寄存器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub struct MCR:u8{
        /// Data Terminal Ready（终端准备好）
        const DATA_TERMINAL_READY = 1 << 0;
        /// Request To Send（请求发送）
        const REQUEST_TO_SEND = 1 << 1;
        /// 用户自定义输出
        const AUX_OUTPUT1 = 1 << 2;
        /// 控制中断使能
        const AUX_OUTPUT2 = 1 << 3;
    }
}

///
/// 读信号时 NS16550A 设备的寄存器在内存的地址位置
///
/// NS16550A 会根据 CPU 在读或写信号时自动在同一个内存地址返回不同寄存器的内容
/// MMIO 只是抽象成内存地址，并不是真实的内存存储单位，内存地址相当于一个“门牌号”，门后的内容由外设提供，读写状态下“门”连接的寄存器不是同一个
/// 即使是多 CPU 并发不同读写同一个 MMIO 内存地址也不会读到错误的寄存器内容
///
/// @author: tryte
///
/// @date: 2026/6/2
#[repr(C)]
#[allow(dead_code)]
struct ReadWithoutDLAB {
    /// 接收缓冲寄存器：读串口收到的数据
    /// - CPU 从这里读数据
    /// - UART 收到的字节会放这里
    pub rbr: ReadOnly<u8>,
    /// 中断使能寄存器：控制 UART 哪些事件可以触发中断，如：
    /// - 收到数据触发中断
    /// - 发送缓冲空触发中断
    pub ier: Volatile<IER>,
    /// 中断识别寄存器：判断“当前是哪种中断触发的”，如：
    /// - 收到数据中断
    /// - 发送完成中断
    /// - 错误中断
    pub iir: ReadOnly<u8>,
    /// 线路控制寄存器
    /// - 设置数据位（5/6/7/8 bit）
    /// - 停止位
    /// - 校验位（parity）
    /// - 控制 DLAB 位！！！
    pub lcr: Volatile<u8>,
    /// 调制解调器控制寄存器
    /// - RTS / DTR 控制信号
    /// - OUT2（中断输出开关）
    /// - loopback 测试模式
    pub mcr: Volatile<MCR>,
    /// 线路状态寄存器
    /// - 是否收到数据（DR）
    /// - 是否可以发送（THRE）
    /// - 是否有错误（OE/PE/FE）
    pub lsr: ReadOnly<LSR>,
    /// 没用上，占位，实际为：MSR
    /// 调制解调器状态寄存器
    /// - CTS / DSR / RI / DCD 等外部信号状态
    /// - 基本现代 OS 很少用
    _padding1: ReadOnly<u8>,
    /// 没用上，占位，实际为：SCR
    /// 临时寄存器 / 备用寄存器
    /// - 给驱动或 OS 做临时存储用（随便用）
    _padding2: ReadOnly<u8>,
}

///
/// 写信号时 NS16550A 设备的寄存器在内存的地址位置
///
/// @author: tryte
///
/// @date: 2026/6/2
#[repr(C)]
#[allow(dead_code)]
struct WriteWithoutDLAB {
    /// 发送保持寄存器：发送一个字节到串口
    /// - CPU 往这里写数据
    /// - UART 会把数据发送出去
    pub thr: WriteOnly<u8>,
    /// 中断使能寄存器：控制 UART 哪些事件可以触发中断，如：
    /// - 收到数据触发中断
    /// - 发送缓冲空触发中断
    pub ier: Volatile<IER>,
    /// 没用上，占位，实际为：FCR
    /// FIFO 控制寄存器
    /// - 打开 FIFO
    /// - 清空 FIFO
    /// - 设置 FIFO 阈值
    _padding0: ReadOnly<u8>,
    /// 线路控制寄存器
    /// - 设置数据位（5/6/7/8 bit）
    /// - 停止位
    /// - 校验位（parity）
    /// - 控制 DLAB 位！！！
    pub lcr: Volatile<u8>,
    /// 调制解调器控制寄存器
    /// - RTS / DTR 控制信号
    /// - OUT2（中断输出开关）
    /// - loopback 测试模式
    pub mcr: Volatile<MCR>,
    /// factory test（工厂测试），为了少获取一次寄存器位置，修改成lsr语义，方便写数据前的检测
    pub lsr: ReadOnly<LSR>,
    /// 无作用，占位
    _padding1: ReadOnly<u8>,
    // 没定义，实际为：SCR
    // 临时寄存器 / 备用寄存器
    // - 给驱动或 OS 做临时存储用（随便用）
    // _padding2: ReadOnly<u8>,
}

pub struct NS16550aRaw {
    base_addr: usize,
}

impl NS16550aRaw {
    ///
    /// 实例化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    ///
    /// 初始化，设置 NS16550A 功能
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn init(&mut self) {
        let read_end = self.read_end();
        let mut mcr = MCR::empty();
        mcr |= MCR::DATA_TERMINAL_READY;
        mcr |= MCR::REQUEST_TO_SEND;
        mcr |= MCR::AUX_OUTPUT2;
        // 在这里设置 mcr 时理论上应该切换成 write_end 更合理，但是读写信号下mcr都处于同一位置，因此原作者并没有额外获取 write_end
        read_end.mcr.write(mcr);
        let ier = IER::RX_AVAILABLE;
        read_end.ier.write(ier);
    }

    ///
    /// 返回读端访问器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn read_end(&mut self) -> &mut ReadWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut ReadWithoutDLAB) }
    }

    ///
    /// 返回写端访问器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn write_end(&mut self) -> &mut WriteWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut WriteWithoutDLAB) }
    }

    ///
    /// 读取数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn read(&mut self) -> Option<u8> {
        // 获取读端寄存器位置
        let read_end = self.read_end();
        // 查看是否有数据可读
        let lsr = read_end.lsr.read();
        if lsr.contains(LSR::DATA_AVAILABLE) {
            // 读取数据
            Some(read_end.rbr.read())
        } else {
            // 无可读数据
            None
        }
    }

    ///
    /// 写入数据到 NS16550A
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn write(&mut self, ch: u8) {
        // 获取写端寄存器位置
        let write_end = self.write_end();
        loop {
            // 在这里获取 lsr 寄存器状态理论上应该切换成 read_end 更合理，因为在写信号下 lsr 这个位置原本是 工厂测试 寄存器位置，
            // 但是因为现在是读信号获取寄存器内容，因此对应的是读信号下 lsr 寄存器的位置
            if write_end.lsr.read().contains(LSR::THR_EMPTY) {
                write_end.thr.write(ch);
                break;
            }
        }
    }
}

struct NS16550aInner {
    ns16550a: NS16550aRaw,
    /// 待读数据缓冲区
    read_buffer: VecDeque<u8>,
}

///
/// NS16550A
///
/// @author: tryte
///
/// @date: 2026/6/2
pub struct NS16550a<const BASE_ADDR: usize> {
    inner: UpIntrFreeCell<NS16550aInner>,
    cond_var: CondVar,
}

impl<const BASE_ADDR: usize> NS16550a<BASE_ADDR> {
    ///
    /// 实例化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn new() -> Self {
        let inner = NS16550aInner {
            ns16550a: NS16550aRaw::new(BASE_ADDR),
            read_buffer: VecDeque::new(),
        };
        Self {
            inner: unsafe { UpIntrFreeCell::new(inner) },
            cond_var: CondVar::new(),
        }
    }

    ///
    /// 待读缓冲区是否为空
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn read_buffer_is_empty(&self) -> bool {
        self.inner
            .exclusive_session(|inner| inner.read_buffer.is_empty())
    }
}

impl<const BASE_ADDR: usize> CharDevice for NS16550a<BASE_ADDR> {
    ///
    /// 初始化 NS16550A
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn init(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.ns16550a.init();
        drop(inner);
    }

    ///
    /// 读取 NS16550A 数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn read(&self) -> u8 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(ch) = inner.read_buffer.pop_front() {
                return ch;
            } else {
                // 如果没数据可读，阻塞当前线程
                let task_cx_ptr = self.cond_var.wait_no_sched();
                drop(inner);
                // 切换新线程
                schedule(task_cx_ptr);
            }
        }
    }

    ///
    /// 写入数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn write(&self, ch: u8) {
        let mut inner = self.inner.exclusive_access();
        inner.ns16550a.write(ch)
    }

    ///
    /// 处理读中断
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn handle_irq(&self) {
        let mut count = 0;
        self.inner.exclusive_session(|inner| {
            while let Some(ch) = inner.ns16550a.read() {
                count += 1;
                inner.read_buffer.push_back(ch);
            }
        });
        if count > 0 {
            // 如果读取到数据则唤醒线程
            self.cond_var.signal();
        }
    }
}
