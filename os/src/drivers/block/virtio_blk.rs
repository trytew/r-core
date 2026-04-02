use crate::mm::{frame_alloc, frame_dealloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne};
use crate::sync::UpSafeCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use virtio_drivers::Hal;

lazy_static! {
    static ref QUEUE_FRAMES: UpSafeCell<Vec<FrameTracker>> = unsafe { UpSafeCell::new(Vec::new()) };
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

    fn phys_to_virt(paddr: usize) -> usize {
        todo!()
    }

    fn virt_to_phys(vaddr: usize) -> PhysAddr {
        todo!()
    }
}

pub struct VirtIOBlock(UpSafeCell<VirtIOBlock<'static, VirtioHal>>);
