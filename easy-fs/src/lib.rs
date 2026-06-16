//! 一个独立于内核的简单文件系统
#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;

/// 块大小为 512 字节
pub const BLOCK_SZ: usize = 512;

pub use block_dev::BlockDevice;

pub use efs::*;

pub use vfs::Inode;
