use core::any::Any;

///
/// 块设备读写特征
///
/// @author: tryte
///
/// @date: 2026/3/17
pub trait BlockDevice: Send + Sync + Any {
    ///
    /// 从块设备读入内存中的缓冲区
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    fn read_block(&self, block_id: usize, buf: &mut [u8]);

    ///
    /// 从内存中的缓冲区写入到块设备
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/16
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
