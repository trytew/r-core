use crate::sync::UpSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
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
