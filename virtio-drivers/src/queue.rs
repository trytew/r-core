use crate::hal::{Hal, DMA};
use crate::header::VirtIOHeader;
use crate::Result;
use crate::{align_up, Error, PAGE_SIZE};
use bitflags::bitflags;
use core::slice;
use core::sync::atomic::{fence, Ordering};
use volatile::Volatile;

bitflags! {
    ///
    /// 数据缓冲区标志位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    struct DescFlags: u16 {
        /// 还有下一个数据缓冲区；数据可能由多个数据缓冲区一起承载，当 NEXT 标志存在时 Descriptor.next 才有作用
        const NEXT = 1;
        /// 设备可以向该缓冲区写数据
        const WRITE = 2;
        /// Descriptor.addr 指向另一张 Descriptor Table
        const INDIRECT = 4;
    }
}

///
/// 数据缓冲区
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C, align(16))]
#[derive(Debug)]
struct Descriptor {
    /// 数据缓冲区物理地址
    addr: Volatile<u64>,
    /// 数据大小
    len: Volatile<u32>,
    /// 标志，具体值可查看 DescFlags 结构体
    flags: Volatile<DescFlags>,
    /// 下一个数据缓冲区
    next: Volatile<u16>,
}

impl Descriptor {
    ///
    /// 写入数据到数据缓冲区
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn set_buf<H: Hal>(&mut self, buf: &[u8]) {
        // 写入数据
        self.addr
            .write(H::virt_to_phys(buf.as_ptr() as usize) as u64);

        // 记录数据大小
        self.len.write(buf.len() as u32);
    }
}

///
/// 待处理任务队列
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C)]
#[derive(Debug)]
struct AvailRing {
    flags: Volatile<u16>,
    // 待处理任务空闲位置
    idx: Volatile<u16>,
    // 待处理任务队列
    ring: [Volatile<u16>; 32],
    //
    used_event: Volatile<u16>,
}

///
/// 已完成的任务
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C)]
#[derive(Debug)]
struct UsedElem {
    /// 任务ID
    id: Volatile<u32>,
    /// 任务长度
    len: Volatile<u32>,
}

///
/// 已完成任务队列
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C)]
#[derive(Debug)]
struct UsedRing {
    flags: Volatile<u16>,
    idx: Volatile<u16>,
    ring: [UsedElem; 32],
    // optional: u16 avail_event;
}

///
/// 队列内存布局
///
/// @author: tryte
///
/// @date: 2026/6/9
struct VirtQueueLayout {
    /// 待处理任务队列起始内存地址偏移
    avail_offset: usize,
    /// 已完成任务队列起始内存地址偏移
    used_offset: usize,
    /// 总使用内存大小
    size: usize,
}

impl VirtQueueLayout {
    ///
    /// 实例化队列内存布局
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn new(queue_size: u16) -> Self {
        // 判断队列长度是否是 2 的幂
        assert!(
            queue_size.is_power_of_two(),
            "queue size should be a power of 2",
        );
        let queue_size = queue_size as usize;
        // 计算各区域内存大小
        let desc = size_of::<Descriptor>() * queue_size;
        // 计算 avail ring 和 used ring 大小，查看 AvailRing 和 UsedRing 结构体大小
        let avail = size_of::<u16>() * (3 + queue_size);
        let used = size_of::<u16>() * 3 + size_of::<UsedElem>() * queue_size;
        VirtQueueLayout {
            avail_offset: desc,
            used_offset: align_up(desc + avail),
            size: align_up(desc + avail) + align_up(used),
        }
    }
}

///
/// 驱动和设备之间的共享消息队列
///
/// @author: tryte
///
/// @date: 2026/6/9
#[derive(Debug)]
pub struct VirtQueue<'a, H: Hal> {
    dma: DMA<H>,
    /// 数据缓冲区描述信息，虚拟地址
    desc: &'a mut [Descriptor],
    /// 待处理描述符队列，虚拟地址
    avail: &'a mut AvailRing,
    /// 已处理描述符队列，虚拟地址
    used: &'a mut UsedRing,
    /// 使用的队列编号
    queue_idx: u32,
    /// 队列长度
    queue_size: u16,
    /// 已使用队列长度
    num_used: u16,
    /// 队列空闲起始位
    free_head: u16,
    /// 空闲待处理任务位置
    avail_idx: u16,
    last_used_idx: u16,
}

impl<H: Hal> VirtQueue<'_, H> {
    ///
    /// 实例化消息队列
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn new(header: &mut VirtIOHeader, idx: usize, size: u16) -> Result<Self> {
        // 判断队列是否已使用
        if header.queue_used(idx as u32) {
            return Err(Error::AlreadyUsed);
        }
        // 判断 size 是否是 2 的幂或者是否超过队列最大可设置长度
        if !size.is_power_of_two() || header.max_queue_size() < size as u32 {
            return Err(Error::InvalidParam);
        }
        // 分配内存
        let layout = VirtQueueLayout::new(size);
        let dma = DMA::new(layout.size / PAGE_SIZE)?;

        // 设置队列
        header.queue_set(idx as u32, size as u32, PAGE_SIZE as u32, dma.pfn());

        // 获取队列中数据缓冲区、待完成任务队列、已完成任务队列的起始虚拟内存地址
        let desc =
            unsafe { slice::from_raw_parts_mut(dma.vaddr() as *mut Descriptor, size as usize) };
        let avail = unsafe { &mut *((dma.vaddr() + layout.avail_offset) as *mut AvailRing) };
        let used = unsafe { &mut *((dma.vaddr() + layout.used_offset) as *mut UsedRing) };

        // 设置每个数据缓冲区的下个数据缓冲区编号
        for i in 0..(size - 1) {
            desc[i as usize].next.write(i + 1);
        }

        Ok(VirtQueue {
            dma,
            desc,
            avail,
            used,
            queue_idx: idx as u32,
            queue_size: size,
            num_used: 0,
            free_head: 0,
            avail_idx: 0,
            last_used_idx: 0,
        })
    }

    ///
    /// 添加任务
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn add(&mut self, inputs: &[&[u8]], outputs: &[&mut [u8]]) -> Result<u16> {
        // 检查输入输出是否同时为空
        if inputs.is_empty() && outputs.is_empty() {
            return Err(Error::InvalidParam);
        }

        // 查看队列是否已满
        if inputs.len() + outputs.len() + self.num_used as usize > self.queue_size as usize {
            return Err(Error::BufferTooSmall);
        }

        // 获取空闲位置
        let head = self.free_head;
        let mut last = self.free_head;

        // 向数据缓冲区写入数据（从驱动写入到设备）
        for input in inputs.iter() {
            // 获取空闲数据缓冲区
            let desc = &mut self.desc[self.free_head as usize];
            // 写入数据
            desc.set_buf::<H>(input);
            // 设置还有下一个数据缓冲区，最后一次循环时会多占用一个 NEXT，但是在设置 output 的时候就会被顶替掉
            desc.flags.write(DescFlags::NEXT);
            // 设置空闲数据缓冲区位置
            last = self.free_head;
            // 修改记录的空闲数据缓冲区位置
            self.free_head = desc.next.read();
        }

        // 从数据缓冲区获取数据（设备写入数据到驱动，以 output 数组接收设备输出的数据）
        for output in outputs.iter() {
            let desc = &mut self.desc[self.free_head as usize];
            desc.set_buf::<H>(output);
            desc.flags.write(DescFlags::NEXT | DescFlags::WRITE);
            last = self.free_head;
            self.free_head = desc.next.read();
        }

        // 无论是仅有 input 或者是还有 output，最后都会多占用一个数据缓冲区，需要移除占用 NEXT 标志
        {
            let desc = &mut self.desc[last as usize];
            let mut flags = desc.flags.read();
            flags.remove(DescFlags::NEXT);
            desc.flags.write(flags);
        }

        // 计算已使用数据缓冲区数量
        self.num_used += (inputs.len() + outputs.len()) as u16;

        // 新增待处理任务，下面的计算等价于 self.avail_idx % self.queue_size，是一个环形队列
        let avail_slot = self.avail_idx & (self.queue_size - 1);

        // 记录当次任务的数据缓冲区起始位置
        self.avail.ring[avail_slot as usize].write(head);

        // 阻止编译器和硬件对内存操作进行重排序
        // 作用是在这句之后的代码保证看到的内存内容都是一致的，不会有指令乱序执行时每个 cpu 指令看到的内存布局不一致
        // 即先把 Descriptor 和 ring[] 完全写好
        // 再更新 avail.idx
        //
        // 这句代码不代表 cpu 指令不会乱序执行，在 cpu 指令执行过程中不会立马将内存新值刷进内存，CPU有多级缓存，
        // 在计算的过程中会先将值放入缓存，待计算结果完成后才会将新值刷进内存，因此多个变量的内存值可能不会同步更新，如先刷了变量a，
        // 然后变量b因后续有还有计算先放入缓存，在这个过程中就会有多个内存观察者读取的内存不是最新的情况，
        // 这句代码就是保证在它之前的内存值所有观察者读取的都是一致的
        fence(Ordering::SeqCst);

        // 设置下个待处理任务位置
        self.avail_idx = self.avail_idx.wrapping_add(1);
        self.avail.idx.write(self.avail_idx);
        Ok(head)
    }

    ///
    /// 返回是否有已完成的任务
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn can_pop(&self) -> bool {
        self.last_used_idx != self.used.idx.read()
    }

    ///
    /// 返回队列空闲长度
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn available_desc(&self) -> usize {
        (self.queue_size - self.num_used) as usize
    }

    fn recycle_descriptors(&mut self, mut head: u16) {
        let origin_free_head = self.free_head;
        self.free_head = head;
        loop {
            let desc = &mut self.desc[head as usize];
            let flags = desc.flags.read();
            self.num_used -= 1;
            if flags.contains(DescFlags::NEXT) {
                head = desc.next.read();
            } else {
                desc.next.write(origin_free_head);
                return;
            }
        }
    }

    pub fn pop_used(&mut self) -> Result<(u16, u32)> {
        if !self.can_pop() {
            return Err(Error::NotReady);
        }
        fence(Ordering::SeqCst);

        let last_used_slot = self.last_used_idx & (self.queue_size - 1);
        let index = self.used.ring[last_used_slot as usize].id.read() as u16;
        let len = self.used.ring[last_used_slot as usize].len.read();

        self.recycle_descriptors(index);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        Ok((index, len))
    }

    pub fn size(&self) -> u16 {
        self.queue_size
    }
}

#[cfg(test)]
mod tests {
    use crate::hal::fake::FakeHal;
    use crate::header::VirtIOHeader;
    use crate::queue::{DescFlags, VirtQueue};
    use crate::Error;
    use core::mem::zeroed;

    #[test]
    fn invalid_queue_size() {
        let mut header = unsafe { zeroed() };
        assert_eq!(
            VirtQueue::<FakeHal>::new(&mut header, 0, 3).unwrap_err(),
            Error::InvalidParam,
        );
    }

    #[test]
    fn queue_too_big() {
        let mut header = VirtIOHeader::make_fake_header(0, 0, 0, 4);
        assert_eq!(
            VirtQueue::<FakeHal>::new(&mut header, 0, 6).unwrap_err(),
            Error::InvalidParam,
        );
    }

    #[test]
    fn queue_already_used() {
        let mut header = VirtIOHeader::make_fake_header(0, 0, 0, 4);
        VirtQueue::<FakeHal>::new(&mut header, 0, 4).unwrap();
        assert_eq!(
            VirtQueue::<FakeHal>::new(&mut header, 0, 4).unwrap_err(),
            Error::AlreadyUsed,
        );
    }

    #[test]
    fn add_empty() {
        let mut header = VirtIOHeader::make_fake_header(0, 0, 0, 4);
        let mut queue = VirtQueue::<FakeHal>::new(&mut header, 0, 4).unwrap();
        assert_eq!(queue.add(&[], &[]).unwrap_err(), Error::InvalidParam);
    }

    #[test]
    fn add_too_big() {
        let mut header = VirtIOHeader::make_fake_header(0, 0, 0, 4);
        let mut queue = VirtQueue::<FakeHal>::new(&mut header, 0, 4).unwrap();
        assert_eq!(queue.available_desc(), 4);
        assert_eq!(
            queue
                .add(&[&[], &[], &[]], &[&mut [], &mut []])
                .unwrap_err(),
            Error::BufferTooSmall,
        );
    }

    #[test]
    fn add_buffers() {
        let mut header = VirtIOHeader::make_fake_header(0, 0, 0, 4);
        let mut queue = VirtQueue::<FakeHal>::new(&mut header, 0, 4).unwrap();
        assert_eq!(queue.size(), 4);
        assert_eq!(queue.available_desc(), 4);

        let token = queue
            .add(&[&[1, 2], &[3]], &[&mut [0, 0], &mut [0]])
            .unwrap();

        assert_eq!(queue.available_desc(), 0);
        assert!(!queue.can_pop());

        let first_descriptor_index = queue.avail.ring[0].read();
        assert_eq!(first_descriptor_index, token);
        assert_eq!(queue.desc[first_descriptor_index as usize].len.read(), 2);
        assert_eq!(
            queue.desc[first_descriptor_index as usize].flags.read(),
            DescFlags::NEXT
        );

        let second_descriptor_index = queue.desc[first_descriptor_index as usize].next.read();
        assert_eq!(queue.desc[second_descriptor_index as usize].len.read(), 1);
        assert_eq!(
            queue.desc[second_descriptor_index as usize].flags.read(),
            DescFlags::NEXT,
        );

        let third_descriptor_index = queue.desc[second_descriptor_index as usize].next.read();
        assert_eq!(queue.desc[third_descriptor_index as usize].len.read(), 2);
        assert_eq!(
            queue.desc[third_descriptor_index as usize].flags.read(),
            DescFlags::NEXT | DescFlags::WRITE,
        );

        let fourth_descriptor_index = queue.desc[third_descriptor_index as usize].next.read();
        assert_eq!(queue.desc[fourth_descriptor_index as usize].len.read(), 1);
        assert_eq!(
            queue.desc[fourth_descriptor_index as usize].flags.read(),
            DescFlags::WRITE,
        );
    }
}
