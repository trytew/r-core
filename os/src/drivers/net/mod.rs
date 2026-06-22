use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::UpIntrFreeCell;
use alloc::sync::Arc;
use core::any::Any;
use lazy_static::lazy_static;
use virtio_drivers::VirtIOHeader;
use virtio_drivers::VirtIONet;

lazy_static! {
    /// 网络设备
    pub static ref NET_DEVICE: Arc<dyn NetDevice> = Arc::new(VirtIONetWrapper::new());
}

/// 网络外设 MMIO 地址
const VIRTIO8: usize = 0x10_004_000;

pub trait NetDevice: Send + Sync + Any {
    ///
    /// 发送数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn transmit(&self, data: &[u8]);

    ///
    /// 接收数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn receive(&self, data: &mut [u8]) -> usize;
}

///
/// 网络外设驱动包装器
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct VirtIONetWrapper(UpIntrFreeCell<VirtIONet<'static, VirtioHal>>);

impl VirtIONetWrapper {
    ///
    /// 创建网络外设驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    pub fn new() -> Self {
        unsafe {
            let virtio = VirtIONet::<VirtioHal>::new(&mut *(VIRTIO8 as *mut VirtIOHeader))
                .expect("can't create net device by virtio");
            VirtIONetWrapper(UpIntrFreeCell::new(virtio))
        }
    }
}

impl NetDevice for VirtIONetWrapper {
    ///
    /// 发送数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn transmit(&self, data: &[u8]) {
        self.0
            .exclusive_access()
            .send(data)
            .expect("can't send data")
    }

    ///
    /// 接收数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn receive(&self, data: &mut [u8]) -> usize {
        self.0
            .exclusive_access()
            .recv(data)
            .expect("can't receive data")
    }
}
