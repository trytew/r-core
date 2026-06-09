//! VirtIO guest drivers

#![no_std]
#![deny(unused_must_use, missing_docs)]
#![allow(clippy::identity_op)]
#![allow(dead_code)]

extern crate alloc;

mod hal;
mod header;
mod input;
mod queue;

pub use hal::{Hal, PhysAddr, VirtAddr};
pub use header::VirtIOHeader;
pub use input::{InputEvent, VirtIOInput};

const PAGE_SIZE: usize = 0x1_000;

/// 虚拟驱动返回类型
pub type Result<T = ()> = core::result::Result<T, Error>;

///
/// 错误信息
///
/// @author: tryte
///
/// @date: 2026/6/3
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// 缓冲区太小
    BufferTooSmall,
    /// 未准备好
    NotReady,
    /// 已经被使用
    AlreadyUsed,
    /// 参数错误
    InvalidParam,
    /// 分配 DMA 内存错误
    DmaError,
    /// I/O 错误
    IoError,
}

unsafe trait AsBuf: Sized {
    fn as_buf(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as _, size_of::<Self>()) }
    }

    fn as_buf_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as _, size_of::<Self>()) }
    }
}

///
/// 按内存页大小取整
///
/// @author: tryte
///
/// @date: 2026/6/9
fn align_up(size: usize) -> usize {
    (size + PAGE_SIZE) & !(PAGE_SIZE - 1)
}
