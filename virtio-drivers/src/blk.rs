use crate::queue::VirtQueue;
use crate::Result;
use crate::{Hal, VirtIOHeader};
use bitflags::bitflags;
use log::info;
use volatile::Volatile;

bitflags! {
    struct BlkFeature:u64 {
        const BARRIR   = 1 << 0;
        const SIZE_MAX = 1 << 1;
        const SEG_MAX = 1 << 2;
        const GEOMETRY = 1 << 4;
        const RO = 1 << 5;
        const BLK_SIZE = 1 << 6;
        const SCSI = 1 << 7;
        const FLUSH = 1 << 9;
        const TOPOLOGY = 1 << 10;
        const CONFIG_WCE = 1 << 11;
        const DISCARD = 1 << 13;
        const WRITE_ZEROES = 1 << 14;

        const NOTIFY_ON_EMPTY    = 1 << 24;
        const ANY_LAYOUT         = 1 << 27;
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX     = 1 << 29;
        const UNUSED             = 1 << 30;
        const VERSION_1          = 1 << 32;

        const ACCESS_PLATFORM   = 1 << 33;
        const RING_PACKED       = 1 << 34;
        const IN_ORDER          = 1 << 35;
        const ORDER_PLATFORM    = 1 << 36;
        const SR_IOV            = 1 << 37;
        const NOTIFICATION_DATA = 1 << 38;
    }
}

#[repr(C)]
#[derive(Debug)]
struct BlkConfig {
    capacity: Volatile<u64>,
    size_max: Volatile<u32>,
    seg_max: Volatile<u32>,
    cylinders: Volatile<u16>,
    heads: Volatile<u8>,
    sectors: Volatile<u8>,
    blk_size: Volatile<u32>,
    physical_block_exp: Volatile<u8>,
    alignment_offset: Volatile<u8>,
    min_io_size: Volatile<u16>,
    opt_io_size: Volatile<u32>,
    // ..ignored
}

pub struct VirtIOBlk<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    queue: VirtQueue<'a, H>,
    capacity: usize,
}

impl<H: Hal> VirtIOBlk<'_, H> {
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        header.begin_init(|features| {
            let features = BlkFeature::from_bits_truncate(features);
            info!("device features: {:?}", features);
            let supported_features = BlkFeature::empty();
            (features & supported_features).bits()
        });

        let config = unsafe { &mut *(header.config_space() as *mut BlkConfig) };
        info!("config: {:?}", config);
        info!(
            "found a block device of size {}KB",
            config.capacity.read() / 2
        );

        let queue = VirtQueue::new(header, 0, 16)?;
        header.finish_init();

        Ok(VirtIOBlk {
            header,
            queue,
            capacity: config.capacity.read() as usize,
        })
    }
}
