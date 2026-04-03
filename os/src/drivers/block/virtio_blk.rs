use crate::mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use crate::sync::UpSafeCell;
use alloc::vec::Vec;
use easy_fs::BlockDevice;
use lazy_static::lazy_static;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

const VIRTIO0: usize = 0x1000_1000;

lazy_static! {
    static ref QUEUE_FRAMES: UpSafeCell<Vec<FrameTracker>> = unsafe { UpSafeCell::new(Vec::new()) };
}

///
/// 虚拟设备块
///
/// @author: tryte
///
/// @date: 2026/4/3
pub struct VirtIOBlock(UpSafeCell<VirtIOBlk<'static, VirtioHal>>);

impl VirtIOBlock {
    pub fn new() -> Self {
        unsafe {
            Self(UpSafeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

impl BlockDevice for VirtIOBlock {
    ///
    /// 读取设备块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtBlock");
    }

    ///
    /// 写入内容到设备块中
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/3
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_id, buf)
            .expect("Error where writing VirtBlock");
    }
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            QUEUE_FRAMES.exclusive_access().push(frame);
        }
        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(paddr: usize, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(addr: usize) -> usize {
        addr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}
