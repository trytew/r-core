use crate::bitmap::Bitmap;
use crate::block_cache::{block_cache_sync_all, get_block_cache};
use crate::layout::{DiskInode, DiskInodeType, SuperBlock};
use crate::vfs::Inode;
use crate::{BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
use spin::Mutex;

type DataBlock = [u8; BLOCK_SZ];

///
/// 文件管理器
///
/// @author: tryte
///
/// @date: 2026/3/27
pub struct EasyFileSystem {
    /// 块设备
    pub block_device: Arc<dyn BlockDevice>,
    /// 索引节点位图
    pub inode_bitmap: Bitmap,
    /// 数据节点位图
    pub data_bitmap: Bitmap,
    /// 索引节点逻辑块起点
    inode_area_start_block: u32,
    /// 数据逻辑块起点
    data_area_start_block: u32,
}

impl EasyFileSystem {
    ///
    /// 创建文件管理器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // 实例化索引节点位图，从文件系统内部第二个逻辑块开始（下标为1），第一个逻辑块是 Super Block
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        // 计算索引节点数量
        let inode_num = inode_bitmap.maximum();
        // 索引节点占据逻辑块数量
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        // 索引总逻辑块数
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        // 数据总逻辑块数
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        // 数据位图块数 = ceil(data_total_blocks / (4096 + 1))，与下面公式相等
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        // 数据逻辑块数
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        // 实例化数据节点位图
        let data_bitmap = Bitmap::new(
            (1 + inode_total_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        // 实例化文件管理器
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // 清空数据块
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // 写入 Super Block 内容
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        // 创建根目录节点
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        // 将缓存数据刷入磁盘
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }

    ///
    /// 读取硬盘信息创建文件管理器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    ///
    /// 获取根节点信息
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }

    ///
    /// 获取索引节点的块id和块内偏移
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            // inode_id as usize * inode_size % inodes_per_block as usize,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }

    ///
    ///
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }

    ///
    /// 分配块位置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    ///
    /// 分配数据块ID并返回ID值
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    ///
    ///
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/27
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}
