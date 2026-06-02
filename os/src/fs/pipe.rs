use crate::fs::File;
use crate::mm::UserBuffer;
use crate::sync::UpIntrFreeCell;
use crate::task::suspend_current_and_run_next;
use alloc::sync::{Arc, Weak};

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

///
/// 管道数据缓冲区
///
/// @author: tryte
///
/// @date: 2026/4/21
pub struct PipeRingBuffer {
    /// 缓冲区数据
    arr: [u8; 32],
    /// 迭代头
    head: usize,
    /// 迭代尾
    tail: usize,
    /// 缓冲区状态
    status: RingBufferStatus,
    /// 结束写
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    ///
    /// 实例化管道缓冲区
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/18
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    ///
    /// 设置 pipe 弱引用，用来判断 pipe 是否已被所有持有的进程关闭
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/22
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    ///
    /// 写入单字节数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    pub fn write_byte(&mut self, byte: u8) {
        // 设置管道缓冲区状态为正常
        self.status = RingBufferStatus::Normal;
        // 向管道尾部添加数据
        self.arr[self.tail] = byte;
        // 计算管道缓冲区是否已满
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    ///
    /// 读取单字节数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/22
    pub fn read_byte(&mut self) -> u8 {
        // 设置管道缓冲区状态为正常
        self.status = RingBufferStatus::Normal;
        // 读取上次迭代后的第一个字节
        let c = self.arr[self.head];
        // 读取位置后移
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            // 管道已读取结束
            self.status = RingBufferStatus::Empty;
        }
        c
    }

    ///
    /// 返回缓冲区已读数量
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    ///
    /// 返回缓冲区可写数量
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    ///
    /// 判断 pipe 是否已被关闭
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/22
    pub fn all_write_end_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

pub struct Pipe {
    readable: bool,
    writeable: bool,
    buffer: Arc<UpIntrFreeCell<PipeRingBuffer>>,
}

impl Pipe {
    ///
    /// 读端设置缓冲区和状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    pub fn read_end_with_buffer(buffer: Arc<UpIntrFreeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writeable: false,
            buffer,
        }
    }

    ///
    /// 写端设置缓冲区和状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    pub fn write_end_with_buffer(buffer: Arc<UpIntrFreeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writeable: true,
            buffer,
        }
    }
}

impl File for Pipe {
    ///
    /// 是否可读
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/22
    fn readable(&self) -> bool {
        self.readable
    }

    ///
    /// 是否可写
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/22
    fn writeable(&self) -> bool {
        self.writeable
    }

    ///
    /// 读取管道数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    fn read(&self, buf: UserBuffer) -> usize {
        // 判断管道是否可读
        assert!(self.readable());
        // 需要读取长度
        let want_to_read = buf.len();
        // 获取需要读取的迭代器
        let mut buf_iter = buf.into_iter();
        // 已读长度
        let mut already_read = 0_usize;
        loop {
            // 获取管道缓冲区
            let mut ring_buffer = self.buffer.exclusive_access();
            // 判断管道是否可读
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                // 判断 pipe 是否已被关闭，是则直接返回已读数据长度
                if ring_buffer.all_write_end_closed() {
                    return already_read;
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    already_read += 1;
                    if already_read == want_to_read {
                        return want_to_read;
                    }
                } else {
                    return already_read;
                }
            }
        }
    }

    ///
    /// 写入管道数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/21
    fn write(&self, buf: UserBuffer) -> usize {
        // 判断是否可写
        assert!(self.writeable());
        // 获取需要写入数据的长度
        let want_to_write = buf.len();
        // 获取需要写入数据的迭代器
        let mut buf_iter = buf.into_iter();
        // 已写长度
        let mut already_write = 0_usize;
        // 循环写入
        loop {
            // 获取管道缓冲区
            let mut ring_buffer = self.buffer.exclusive_access();
            // 获取可写长度
            let loop_write = ring_buffer.available_write();
            // 可写长度为0（暂时已满，需要等待数据读取才有位置继续写入）
            if loop_write == 0 {
                // 释放可变引用，因为 suspend_current_and_run_next 是汇编代码跳转，不会触发 Defer 特征
                drop(ring_buffer);
                // 让出 CPU 时间片
                suspend_current_and_run_next();
                continue;
            }

            // 写入指定长度数据
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    // 写入单字节数据
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    // 已写数量+1
                    already_write += 1;
                    // 若已写数量和需要写入的数量相同则代表写入结束，返回写入数量结果
                    if already_write == want_to_write {
                        return want_to_write;
                    }
                } else {
                    return already_write;
                }
            }
        }
    }
}

///
/// 创建管道读端和写端
///
/// @author: tryte
///
/// @date: 2026/4/18
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    // 创建数据缓冲区
    let buffer = Arc::new(unsafe { UpIntrFreeCell::new(PipeRingBuffer::new()) });
    // 创建读端
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    // 创建写端
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    // 设置写结束
    buffer.exclusive_access().set_write_end(&write_end);
    // 返回读写端
    (read_end, write_end)
}
