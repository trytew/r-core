use crate::config::KERNEL_HEAP_SIZE;
use crate::println;
use buddy_system_allocator::LockedHeap;
use core::ptr::addr_of_mut;

///
/// 堆分配器
/// LockedHeap 是一个所有数据都在栈上的结构体，因此不会涉及到堆内存的分配
///
/// @author: tryte
///
/// @date: 2026/1/19
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

///
/// 初始化堆空间
///
/// @author: tryte
///
/// @date: 2026/1/8
pub fn init_heap() {
    unsafe {
        // 设置堆空间的分配
        HEAP_ALLOCATOR
            .lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
    }
}

///
/// 处理堆空间分配错误
///
/// @author: tryte
///
/// @date: 2026/1/8
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    // 获取堆空间地址范围
    let bss_range = sbss as *const () as usize..ebss as *const () as usize;
    // 实例化变量 a，Box::new() 底层会调用 HEAP_ALLOCATOR 这个全局堆内存分配器
    let a = Box::new(5);
    assert_eq!(*a, 5);
    // 查看变量 a 的堆内存地址是否在 bss_range 内
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    // 实例化变量 v，Vec::new() 底层会调用 HEAP_ALLOCATOR 这个全局堆内存分配器
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    // 查看变量 v 的堆内存地址是否在 bss_range 内
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
