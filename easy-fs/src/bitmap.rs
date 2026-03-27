use crate::block_cache::get_block_cache;
use crate::{BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;

/// bitmap 块位数
const BLOCK_BITS: usize = BLOCK_SZ * 8;

type BitmapBlock = [u64; 64];

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    ///
    /// 根据起始块ID和块数创建 Bitmap
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/26
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    ///
    /// 分配块位置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/26
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 遍历 64 个 64位数的数组
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| {
                        // 当数字不等于 64 位数最大值时代表有空块可分配
                        **bits64 != u64::MAX
                    })
                    .map(|(bits64_pos, bit64)| {
                        // 获取第几个 64 位数未满（即第几行），以及所在偏移位（行内第几位）
                        (bits64_pos, bit64.trailing_ones() as usize)
                    })
                {
                    bitmap_block[bits64_pos] |= 1_u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos)
                } else {
                    None
                }
            });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    ///
    /// 释放块空间
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/26
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_lock: &mut BitmapBlock| {
                assert!(bitmap_lock[bits64_pos] & (1_u64 << inner_pos) > 0);
                bitmap_lock[bits64_pos] -= 1_u64 << inner_pos;
            });
    }

    ///
    /// 最大值
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/26
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

///
/// 根据 bit 数获取所在块数、块里行数，行内位数
///
/// @author: tryte
///
/// @date: 2026/3/27
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}
