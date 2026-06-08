use crate::hal::{Hal, PhysAddr, VirtAddr};
use crate::PAGE_SIZE;
use alloc::alloc::{alloc_zeroed, dealloc, handle_alloc_error};
use core::alloc::Layout;

#[derive(Debug)]
pub struct FakeHal;

impl Hal for FakeHal {
    fn dma_alloc(pages: usize) -> PhysAddr {
        assert_ne!(pages, 0);
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        ptr as PhysAddr
    }

    fn dma_dealloc(p_addr: PhysAddr, pages: usize) -> i32 {
        assert_ne!(pages, 0);
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        unsafe {
            dealloc(p_addr as *mut u8, layout);
        }
        0
    }

    fn phys_to_virt(p_addr: PhysAddr) -> VirtAddr {
        p_addr
    }

    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        vaddr
    }
}
