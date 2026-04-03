use crate::fs::File;
use crate::mm::UserBuffer;
use crate::println;
use crate::sync::UpSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
    };
}

pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

pub struct OSInode {
    readable: bool,
    writeable: bool,
    inner: UpSafeCell<OSInodeInner>,
}

impl OSInode {
    pub fn new(readable: bool, writeable: bool, indoe: Arc<Inode>) -> Self {
        Self {
            readable,
            writeable,
            inner: unsafe { UpSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0_u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

impl File for OSInode {
    ///
    /// 是否可读
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn readable(&self) -> bool {
        self.readable
    }

    ///
    /// 是否可写
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn writeable(&self) -> bool {
        self.writeable
    }

    ///
    /// 从节点指向的数据读取内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0_usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    ///
    /// 向节点写入数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0_usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}

bitflags! {
    ///
    /// 打开标志
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    pub struct OpenFlags: u32 {
        /// 只读
        const RDONY = 0;
        /// 只写
        const WRONY = 1 << 0;
        /// 读写
        const RDRW = 1 << 1;
        /// 可创建
        const CREATE = 1 << 9;
        /// 清空文件并返回空文件
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    ///
    /// 获取读写权限
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONY) {
            (false, true)
        } else {
            (true, true)
        }
    }

    pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
        let (readable, writeable) = flags.read_write();
        if flags.contains(OpenFlags::CREATE) {
            if let Some(inode) = ROOT_INODE.find(name) {
                inode.clear();
                Some(Arc::new(OSInode::new(readable, writeable, inode)))
            } else {
                ROOT_INODE.find(name).map(|inode| {
                    if flags.contains(OpenFlags::TRUNC) {
                        inode.clear();
                    }
                    Arc::new(OSInode::new(readable, writeable, inode))
                })
            }
        } else {
            ROOT_INODE.find(name).map(|inode| {
                if flags.contains(Self::TRUNC) {
                    inode.clear();
                }
                Arc::new(OSInode::new(readable, writeable, inode))
            })
        }
    }
}

///
/// 列出应用
///
/// @author: tryte
///
/// @date: 2026/4/3
pub fn list_apps() {
    println!("/**** APPS ****/");

    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }

    println!("/**************/");
}

///
/// 打开文件
///
/// @author: tryte
///
/// @date: 2026/4/3
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writeable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writeable, inode)))
        } else {
            ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writeable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear()
            }
            Arc::new(OSInode::new(readable, writeable, inode))
        })
    }
}
