use crate::PAGE_SIZE;
use bitflags::bitflags;
use volatile::{ReadOnly, Volatile, WriteOnly};

const MAGIC_VALUE: u32 = 0x74_726_976;

const CONFIG_SPACE_OFFSET: usize = 0x100;

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
    struct DeviceStatus:u32 {
        const ACKNOWLEDGE = 1;
        const DRIVER = 2;
        const FAILED = 128;
        const FEATURES_OK = 8;
        const DRIVER_OK = 4;
        const DEVICE_NEEDS_RESET = 64;
    }
}

#[repr(C)]
pub struct VirtIOHeader {
    magic: ReadOnly<u32>,
    version: ReadOnly<u32>,
    device_id: ReadOnly<u32>,
    vendor_id: ReadOnly<u32>,
    device_features: ReadOnly<u32>,
    device_features_sel: WriteOnly<u32>,
    __r1: [ReadOnly<u32>; 2],
    driver_features: WriteOnly<u32>,
    driver_features_sel: WriteOnly<u32>,
    guest_page_size: WriteOnly<u32>,
    __r2: ReadOnly<u32>,
    queue_sel: WriteOnly<u32>,
    queue_num_max: ReadOnly<u32>,
    queue_num: WriteOnly<u32>,
    queue_align: WriteOnly<u32>,
    queue_pfn: Volatile<u32>,
    queue_ready: Volatile<u32>,
    __r3: [ReadOnly<u32>; 2],
    queue_notify: WriteOnly<u32>,
    __r4: [ReadOnly<u32>; 3],
    interrupt_status: ReadOnly<u32>,
    interrupt_ack: WriteOnly<u32>,
    __r5: [ReadOnly<u32>; 2],
    status: Volatile<DeviceStatus>,
    __r6: [ReadOnly<u32>; 3],
    queue_desc_low: WriteOnly<u32>,
    queue_desc_high: WriteOnly<u32>,
    __r7: [ReadOnly<u32>; 2],
    queue_avail_low: WriteOnly<u32>,
    queue_avail_high: WriteOnly<u32>,
    __r8: [ReadOnly<u32>; 2],
    queue_used_low: WriteOnly<u32>,
    queue_used_high: WriteOnly<u32>,
    __r9: [ReadOnly<u32>; 21],
    config_generation: ReadOnly<u32>,
}

impl VirtIOHeader {
    pub fn verify(&self) -> bool {
        self.magic.read() == MAGIC_VALUE && self.version.read() == 1 && self.device_id.read() != 0
    }

    pub fn device_type(&self) -> DeviceType {
        match self.device_id.read() {
            x @ 1..=13 | x @ 16..=24 => unsafe { core::mem::transmute(x as u8) },
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

    pub fn queue_set(&mut self, queue: u32, size: u32, align: u32, pfn: u32) {
        self.queue_sel.write(queue);
        self.queue_num.write(size);
        self.queue_align.write(align);
        self.queue_pfn.write(pfn);
    }

    pub fn queue_physical_page_number(&mut self, queue: u32) -> u32 {
        self.queue_sel.write(queue);
        self.queue_pfn.read()
    }

    pub fn queue_used(&mut self, queue: u32) -> bool {
        self.queue_physical_page_number(queue) != 0
    }

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
