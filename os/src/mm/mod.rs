mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use heap_allocator::*;

pub use frame_allocator::*;

pub use address::*;

pub use page_table::*;

use crate::mm;
pub use memory_set::*;

pub fn init() {
    init_heap();
    init_frame_allocator();
    mm::KERNEL_SPACE.exclusive_access().activate();
}
