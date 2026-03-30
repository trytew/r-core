use crate::{BlockDevice, BLOCK_SZ};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::mem::ManuallyDrop;
use core::ptr::{addr_of, addr_of_mut};
use core::slice;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// 全局块缓存管理器
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(BlockCacheManager::new());
}

struct CacheData(ManuallyDrop<Box<[u8; BLOCK_SZ]>>);

impl CacheData {
    pub fn new() -> Self {
        let data = unsafe {
            let raw = alloc::alloc::alloc(Self::layout());
            Box::from_raw(raw as *mut [u8; BLOCK_SZ])
        };
        Self(ManuallyDrop::new(data))
    }

    fn layout() -> Layout {
        Layout::from_size_align(BLOCK_SZ, BLOCK_SZ).unwrap()
    }
}

impl Drop for CacheData {
    fn drop(&mut self) {
        let ptr = self.0.as_mut_ptr();
        unsafe { alloc::alloc::dealloc(ptr, Self::layout()) };
    }
}

impl AsRef<[u8]> for CacheData {
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr();
        unsafe { slice::from_raw_parts(ptr, BLOCK_SZ) }
    }
}

impl AsMut<[u8]> for CacheData {
    fn as_mut(&mut self) -> &mut [u8] {
        let ptr = self.0.as_mut_ptr();
        unsafe { slice::from_raw_parts_mut(ptr, BLOCK_SZ) }
    }
}

///
/// 数据块缓存
///
/// @author: tryte
///
/// @date: 2026/3/16
pub struct BlockCache {
    /// 缓冲区数据
    cache: CacheData,
    /// 数据块id
    block_id: usize,
    /// 块设备
    block_device: Arc<dyn BlockDevice>,
    /// 是否修改
    modified: bool,
}

impl BlockCache {
    ///
    /// 实例化数据块缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = CacheData::new();
        block_device.read_block(block_id, cache.as_mut());
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    ///
    /// 数据块偏移
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    fn add_of_offset(&self, offset: usize) -> *const u8 {
        addr_of!(self.cache.as_ref()[offset])
    }

    ///
    /// 数据块偏移（可变）
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    fn addr_of_offset_mut(&mut self, offset: usize) -> *mut u8 {
        addr_of_mut!(self.cache.as_mut()[offset])
    }

    ///
    /// 获取引用
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.add_of_offset(offset) as *const T;
        unsafe { &*addr }
    }

    ///
    /// 获取引用（可变）
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset_mut(offset) as *mut T;
        unsafe { &mut *addr }
    }

    ///
    /// 读取缓存内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    ///
    /// 修改缓存内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    ///
    /// 写入块设备
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/17
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    ///
    /// 块缓存管理
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/17
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    ///
    /// 获取块缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/17
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1)
        } else {
            // 若缓存块数量超出限制则丢弃
            if self.queue.len() == BLOCK_CACHE_SIZE {
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

///
/// 获取块缓存
///
/// @author: tryte
///
/// @date: 2026/3/17
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

///
/// 刷入块数据
///
/// @author: tryte
///
/// @date: 2026/3/17
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
