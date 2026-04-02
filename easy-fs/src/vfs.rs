use crate::block_cache::{block_cache_sync_all, get_block_cache};
use crate::efs::EasyFileSystem;
use crate::layout::{DirEntry, DiskInode, DiskInodeType, DIRENT_SZ};
use crate::BlockDevice;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

///
/// 文件节点
///
/// @author: tryte
///
/// @date: 2026/3/28
pub struct Inode {
    /// 所在硬盘扇区ID
    block_id: usize,
    /// 所在硬盘扇区偏移
    block_offset: usize,
    /// 所属文件管理器
    fs: Arc<Mutex<EasyFileSystem>>,
    /// 块设备
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    ///
    /// 读取硬盘扇区数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn read_disk_node<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    ///
    /// 修改硬盘扇区数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/28
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    ///
    /// 查找文件/文件夹所在扇区ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    fn find_inode_id(&self, name: &str, disk_node: &DiskInode) -> Option<u32> {
        assert!(disk_node.is_dir());
        // 根据当前索引节点对应的数据大小计算有多少个目录项
        let file_count = (disk_node.size as usize) / DIRENT_SZ;
        // 记录目录名
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                // 读取数据中记录的目录名
                disk_node.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device),
                DIRENT_SZ
            );
            if dirent.name() == name {
                return Some(dirent.inode_number());
            }
        }
        None
    }

    ///
    /// 查找文件/目录所在的索引节点
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_node(|disk_inode: &DiskInode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    ///
    /// 节点扩容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/28
    fn increase_size(
        &self,
        new_size: u32,
        disk_node: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_node.size {
            return;
        }
        // 计算扩容所需逻辑块数
        let block_needed = disk_node.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..block_needed {
            v.push(fs.alloc_data());
        }
        disk_node.increase_size(new_size, v, &self.block_device);
    }

    ///
    /// 创建文件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/28
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        // 查找文件名是否已存在
        let op = |root_inode: &DiskInode| {
            assert!(root_inode.is_dir());
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_node(op).is_some() {
            return None;
        }
        // 分配空索引块ID
        let new_inode_id = fs.alloc_inode();
        // 根据索引块ID获取索引块和块内偏移
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        // 初始化索引块
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                // 初始化文件类型的节点
                new_inode.initialize(DiskInodeType::File);
            });

        self.modify_disk_inode(|root_inode| {
            // 扩充节点容量
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            let dirent = DirEntry::new(name, new_inode_id);
            // 写入新增内容
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        // 获取文件索引节点所在的索引块ID和偏移
        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    ///
    /// 列出节点里的文件/目录名
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_node(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    ///
    /// 读取索引块指向的数据块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_node(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    ///
    /// 向索引块指向的数据块写入内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    ///
    /// 清理文件内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/2
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert_eq!(
                data_blocks_dealloc.len(),
                DiskInode::total_blocks(size) as usize
            );
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
