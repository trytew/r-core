use crate::PAGE_SIZE;
use bitflags::bitflags;
use volatile::{ReadOnly, Volatile, WriteOnly};

const MAGIC_VALUE: u32 = 0x74_726_976;

const CONFIG_SPACE_OFFSET: usize = 0x100;

///
/// 设备类型
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Invalid = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    EntropySource = 4,
    MemoryBallooning = 5,
    IoMemory = 6,
    RpMsg = 7,
    ScsiHost = 8,
    _9P = 9,
    Mac80211 = 10,
    RProcSerial = 11,
    VirtioCAIF = 12,
    MemoryBalloon = 13,
    GPU = 16,
    Timer = 17,
    Input = 18,
    Socket = 19,
    Crypto = 20,
    SignalDistributionModule = 21,
    PStore = 22,
    IOMMU = 23,
    Memory = 24,
}

bitflags! {
    ///
    /// 设备状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    struct DeviceStatus:u32 {
        const ACKNOWLEDGE = 1;
        const DRIVER = 2;
        const FAILED = 128;
        const FEATURES_OK = 8;
        const DRIVER_OK = 4;
        const DEVICE_NEEDS_RESET = 64;
    }
}

///
/// 设备配置头
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C)]
pub struct VirtIOHeader {
    /// 设备身份信息
    /// 魔术变量，固定值，用来判断是否为 VirtIO 设备
    magic: ReadOnly<u32>,
    /// VirtIO 版本
    version: ReadOnly<u32>,
    /// 设备类型：1 = Network; 2 = Block; 3 = Console; 16 = GPU; 18 = Input
    device_id: ReadOnly<u32>,
    /// 厂商编号; QEMU 通常是 0x554d4551，即 ASCII 转换的 UMEQ
    vendor_id: ReadOnly<u32>,
    /// 设备支持什么功能
    device_features: ReadOnly<u32>,
    /// *_sel，因为 feature 是 64 位甚至更多位。MMIO寄存器一次只能读32位，所以：
    /// device_features_sel = 0 ---> 读低32位
    /// device_features_sel = 1 ---> 读高32位
    device_features_sel: WriteOnly<u32>,
    /// 空白占位，也有预留功能的作用
    __r1: [ReadOnly<u32>; 2],
    /// 驱动选择启用什么功能
    driver_features: WriteOnly<u32>,
    /// 如 device_features_sel
    driver_features_sel: WriteOnly<u32>,
    guest_page_size: WriteOnly<u32>,
    /// 如 __r1
    __r2: ReadOnly<u32>,
    /// 选择配置哪个队列，如：queue_sel.write(0) 代表 配置 queue0
    queue_sel: WriteOnly<u32>,
    /// 设备支持的最大队列长度
    queue_num_max: ReadOnly<u32>,
    /// 驱动实际决定使用多少描述符
    queue_num: WriteOnly<u32>,
    /// Used Ring 的对齐要求，一般是 4kb；Legacy VirtIO（0.9 版本）使用，现已被 1.0 取代
    queue_align: WriteOnly<u32>,
    /// Physical Frame Number，VirtQueue所在内存的物理地址页号，为0代表未使用，Legacy VirtIO（0.9 版本）使用，现已被 1.0 取代
    queue_pfn: Volatile<u32>,
    /// 配置完成标识，1代表配置完成
    queue_ready: Volatile<u32>,
    /// 如 __r1
    __r3: [ReadOnly<u32>; 2],
    /// 通知，如提交请求后：queue_notify.write(queue_index) 代表告诉设备来看 Queue
    queue_notify: WriteOnly<u32>,
    /// 如 __r1
    __r4: [ReadOnly<u32>; 3],
    /// 查看为什么产生中断，如：
    /// bit0 ---> queue 完成
    /// bit1 ---> 配置变化
    interrupt_status: ReadOnly<u32>,
    /// 处理中断后的应答
    interrupt_ack: WriteOnly<u32>,
    /// 如 __r1
    __r5: [ReadOnly<u32>; 2],
    /// 设备状态机，驱动必须按规范写：
    /// ---> 初始 0
    /// ---> 发现设备 (ACKNOWLEDGE)
    /// ---> 驱动加载 (ACKNOWLEDGE | DRIVER)
    /// ---> 完成 Feature 协商 (ACKNOWLEDGE | DRIVER | FEATURES_OK)
    /// ---> 队列初始化完成 (ACKNOWLEDGE | DRIVER | FEATURES_OK | DRIVER_OK)
    status: Volatile<DeviceStatus>,
    /// 如 __r1
    __r6: [ReadOnly<u32>; 3],
    /// 描述数据在哪里（地址、长度、读写权限）
    /// 因为 MMIO 一次性只处理 32 位，因此高低地址分两个字段存储，因为 MMIO 一次性只处理 32 位，因此高低地址分两个字段存储
    queue_desc_low: WriteOnly<u32>,
    queue_desc_high: WriteOnly<u32>,
    __r7: [ReadOnly<u32>; 2],
    /// 待处理任务列表，驱动告诉设备“这些 Descriptor 可以处理了”，因为 MMIO 一次性只处理 32 位，因此高低地址分两个字段存储
    queue_avail_low: WriteOnly<u32>,
    queue_avail_high: WriteOnly<u32>,
    __r8: [ReadOnly<u32>; 2],
    /// 已完成任务列表，设备告诉驱动“这些 Descriptor 已经处理完了”，因为 MMIO 一次性只处理 32 位，因此高低地址分两个字段存储
    queue_used_low: WriteOnly<u32>,
    queue_used_high: WriteOnly<u32>,
    __r9: [ReadOnly<u32>; 21],
    /// 配置修改计数器，用于校验配置读取过程中是否有被修改
    config_generation: ReadOnly<u32>,
}

impl VirtIOHeader {
    ///
    /// 校验是否为 VirtIO 设备
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn verify(&self) -> bool {
        self.magic.read() == MAGIC_VALUE && self.version.read() == 1 && self.device_id.read() != 0
    }

    ///
    /// 获取设备类型
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn device_type(&self) -> DeviceType {
        match self.device_id.read() {
            x @ 1..=13 | x @ 16..=24 => unsafe {
                // 将内存值解析成 DeviceType 类型
                core::mem::transmute(x as u8)
            },
            _ => DeviceType::Invalid,
        }
    }

    pub fn vendor_id(&self) -> u32 {
        self.vendor_id.read()
    }

    fn read_device_features(&mut self) -> u64 {
        self.device_features_sel.write(0);
        let mut device_features_bits = self.device_features.read().into();
        self.device_features_sel.write(1);
        device_features_bits += (self.device_features.read() as u64) << 32;
        device_features_bits
    }

    fn write_driver_features(&mut self, driver_features: u64) {
        self.driver_features_sel.write(0);
        self.driver_features.write(driver_features as u32);
        self.driver_features_sel.write(1);
        self.driver_features.write((driver_features >> 32) as u32);
    }

    pub fn begin_init(&mut self, negotiate_features: impl FnOnce(u64) -> u64) {
        self.status.write(DeviceStatus::ACKNOWLEDGE);
        self.status.write(DeviceStatus::DRIVER);

        let features = self.read_device_features();
        self.write_driver_features(negotiate_features(features));
        self.status.write(DeviceStatus::FEATURES_OK);

        self.guest_page_size.write(PAGE_SIZE as u32);
    }

    pub fn finish_init(&mut self) {
        self.status.write(DeviceStatus::DRIVER_OK);
    }

    ///
    /// 设置队列
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn queue_set(&mut self, queue: u32, size: u32, align: u32, pfn: u32) {
        // 设置使用的哪个队列
        self.queue_sel.write(queue);
        // 设置队列内存大小
        self.queue_num.write(size);
        // 设置字节对齐大小
        self.queue_align.write(align);
        // 设置队列的物理地址
        self.queue_pfn.write(pfn);
    }

    ///
    /// 获取队列所在内存页
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn queue_physical_page_number(&mut self, queue: u32) -> u32 {
        self.queue_sel.write(queue);
        self.queue_pfn.read()
    }

    ///
    /// 查看队列是否已使用
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn queue_used(&mut self, queue: u32) -> bool {
        self.queue_physical_page_number(queue) != 0
    }

    ///
    /// 读取队列最大可设置长度
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn max_queue_size(&self) -> u32 {
        self.queue_num_max.read()
    }

    pub fn notify(&mut self, queue: u32) {
        self.queue_notify.write(queue)
    }

    pub fn ack_interrupt(&mut self) -> bool {
        let interrupt = self.interrupt_status.read();
        if interrupt != 0 {
            self.interrupt_ack.write(interrupt);
            true
        } else {
            false
        }
    }

    pub fn config_space(&self) -> *mut u64 {
        (self as *const _ as usize + CONFIG_SPACE_OFFSET) as _
    }

    ///
    /// 构造虚假虚拟设备设置头
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn make_fake_header(
        device_id: u32,
        vendor_id: u32,
        device_features: u32,
        queue_num_max: u32,
    ) -> Self {
        Self {
            magic: ReadOnly::new(MAGIC_VALUE),
            version: ReadOnly::new(1),
            device_id: ReadOnly::new(device_id),
            vendor_id: ReadOnly::new(vendor_id),
            device_features: ReadOnly::new(device_features),
            device_features_sel: WriteOnly::default(),
            __r1: Default::default(),
            driver_features: Default::default(),
            driver_features_sel: Default::default(),
            guest_page_size: Default::default(),
            __r2: Default::default(),
            queue_sel: Default::default(),
            queue_num_max: ReadOnly::new(queue_num_max),
            queue_num: Default::default(),
            queue_align: Default::default(),
            queue_pfn: Default::default(),
            queue_ready: Default::default(),
            __r3: Default::default(),
            queue_notify: Default::default(),
            __r4: Default::default(),
            interrupt_status: Default::default(),
            interrupt_ack: Default::default(),
            __r5: Default::default(),
            status: Volatile::new(DeviceStatus::empty()),
            __r6: Default::default(),
            queue_desc_low: Default::default(),
            queue_desc_high: Default::default(),
            __r7: Default::default(),
            queue_avail_low: Default::default(),
            queue_avail_high: Default::default(),
            __r8: Default::default(),
            queue_used_low: Default::default(),
            queue_used_high: Default::default(),
            __r9: Default::default(),
            config_generation: Default::default(),
        }
    }
}
