mod inode;
mod stdio;

pub use inode::list_apps;
pub use inode::open_file;
pub use inode::OpenFlags;
pub use stdio::Stdin;
pub use stdio::Stdout;

use crate::mm::UserBuffer;

pub trait File: Send + Sync {
    ///
    /// 判断是否可读
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn readable(&self) -> bool;

    ///
    /// 判断是否可写
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn writeable(&self) -> bool;

    ///
    /// 读取文件写入 UserBuffer
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn read(&self, buf: UserBuffer) -> usize;

    ///
    /// 将 UserBuffer 写入文件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn write(&self, buf: UserBuffer) -> usize;
}
