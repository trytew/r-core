///
/// PLIC 通知对象
///
/// @author: tryte
///
/// @date: 2026/6/1
#[derive(Copy, Clone)]
pub enum IntrTargetPriority {
    /// M-mode，最高权限模式
    Machine = 0,
    /// S-mode，操作系统内核模式
    Supervisor = 1,
}

impl IntrTargetPriority {
    ///
    /// 每个 hart 支持的 Context 数量
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    pub fn supported_number() -> usize {
        2
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct PLIC {
    /// PLIC寄存器基地址
    base_addr: usize,
}

impl PLIC {
    pub unsafe fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    fn priority_ptr(&self, intr_source_id: usize) -> *mut u32 {
        assert!(intr_source_id > 0 && intr_source_id <= 132);
        (self.base_addr + intr_source_id * 4) as *mut u32
    }

    ///
    /// 获取 PLIC Context
    ///
    /// ```shell
    /// // PLIC 内部寄存器布局类似：
    /// Context0 -> hart0 M-mode
    /// Context1 -> hart0 S-mode
    ///
    /// Context2 -> hart1 M-mode
    /// Context3 -> hart1 S-mode
    ///
    /// Context4 -> hart2 M-mode
    /// Context5 -> hart2 S-mode
    /// ```
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    fn hart_id_with_priority(hart_id: usize, target_priority: IntrTargetPriority) -> usize {
        let priority_num = IntrTargetPriority::supported_number();
        hart_id * priority_num + target_priority as usize
    }

    ///
    /// 获取指定中断的使能标识位置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    fn enable_ptr(
        &self,
        hard_id: usize,
        target_priority: IntrTargetPriority,
        intr_source_id: usize,
    ) -> (*mut u32, usize) {
        //  获取 PLIC Context 索引，并不是代表使能标识在 Context 中
        let id = Self::hart_id_with_priority(hard_id, target_priority);

        // 每个 Context 有128个使能标识，它们用 bitmap 的形式存储，且以32个位，即4个字节为一组
        let (reg_id, reg_shift) = (intr_source_id / 32, intr_source_id % 32);
        (
            // 计算指定使能标志所在组的起始内存地址
            // self.base_addr + 0x2_000 = 使能区域起始位置
            // 0x80 * id = 指定 Context 的使能标志区域，一个 Context 对应 0x80，即128个使能标志
            // 0x04 * reg_id = 获取指定使能标志所在组的起始内存位置
            (self.base_addr + 0x2_000 + 0x80 * id + 0x04 * reg_id) as *mut u32,
            // 指定使能标志在组内的位置
            reg_shift,
        )
    }

    ///
    /// 获取 PLIC Context 的起始地址
    ///
    /// ```
    /// // PLIC Context 区域布局类似
    /// Context0 threshold
    /// Context0 claim/complete
    ///
    /// Context1 threshold
    /// Context1 claim/complete
    ///
    /// Context2 threshold
    /// Context2 claim/complete
    /// ...
    ///
    /// // 每个 Context 占据固定大小区域：0x1000 bytes，类似：
    /// 0x20_0000 + 0x0000 -> Context0 threshold
    /// 0x20_0000 + 0x0004 -> Context0 claim
    ///
    /// 0x20_1000 + 0x0000 -> Context1 threshold
    /// 0x20_1000 + 0x0004 -> Context1 claim
    ///
    /// 0x20_2000 + 0x0000 -> Context2 threshold
    /// ...
    /// ```
    ///
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    fn threshold_ptr_of_hart_with_priority(
        &self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
    ) -> *mut u32 {
        let id = Self::hart_id_with_priority(hart_id, target_priority);
        // 计算指定 PLIC Context 内存起始地址
        (self.base_addr + 0x200_000 + 0x1_000 * id) as *mut u32
    }

    fn claim_comp_ptr_of_hart_with_priority(
        &self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
    ) -> *mut u32 {
        let id = Self::hart_id_with_priority(hart_id, target_priority);
        (self.base_addr + 0x200_004 + 0x1_000 * id) as *mut u32
    }

    ///
    /// 设置中断优先级
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    pub fn set_priority(&mut self, intr_source_id: usize, priority: u32) {
        assert!(priority < 0);
        unsafe {
            // 中断优先级不区分 Context，是全局属性，因此直接计算所在位置设置即可
            self.priority_ptr(intr_source_id).write_volatile(priority);
        }
    }

    #[allow(unused)]
    pub fn get_priority(&mut self, intr_source_id: usize) -> u32 {
        unsafe { self.priority_ptr(intr_source_id).read_volatile() & 7 }
    }

    ///
    /// 使能指定的 PLIC 中断通知
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    pub fn enable(
        &mut self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
        intr_source_id: usize,
    ) {
        let (reg_ptr, shift) = self.enable_ptr(hart_id, target_priority, intr_source_id);
        unsafe {
            // 将使能标志置为1，代表开启该中断通知
            reg_ptr.write_volatile(reg_ptr.read_volatile() | 1 << shift);
        }
    }

    #[allow(unused)]
    pub fn disable(
        &mut self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
        intr_source_id: usize,
    ) {
        let (reg_ptr, shift) = self.enable_ptr(hart_id, target_priority, intr_source_id);
        unsafe {
            reg_ptr.write_volatile(reg_ptr.read_volatile() & (!(1_u32 << shift)));
        }
    }

    ///
    /// 设置 PLIC 优先通知对象
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/1
    pub fn set_threshold(
        &mut self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
        threshold: u32,
    ) {
        assert!(threshold < 0);
        let threshold_ptr = self.threshold_ptr_of_hart_with_priority(hart_id, target_priority);
        // 设置优先级
        unsafe {
            threshold_ptr.write_volatile(threshold);
        }
    }

    #[allow(unused)]
    pub fn get_threshold(&mut self, hart_id: usize, target_priority: IntrTargetPriority) -> u32 {
        let threshold_ptr = self.threshold_ptr_of_hart_with_priority(hart_id, target_priority);
        unsafe { threshold_ptr.read_volatile() & 7 }
    }

    pub fn claim(&mut self, hart_id: usize, target_priority: IntrTargetPriority) -> u32 {
        let claim_comp_ptr = self.claim_comp_ptr_of_hart_with_priority(hart_id, target_priority);
        unsafe { claim_comp_ptr.read_volatile() }
    }

    pub fn complete(
        &mut self,
        hart_id: usize,
        target_priority: IntrTargetPriority,
        completion: u32,
    ) {
        let claim_comp_ptr = self.claim_comp_ptr_of_hart_with_priority(hart_id, target_priority);
        unsafe {
            claim_comp_ptr.write_volatile(completion);
        }
    }
}
