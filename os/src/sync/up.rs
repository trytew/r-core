use core::cell::{RefCell, RefMut};

///
/// 多线程 RefCell
///
/// @author: tryte
///
/// @date: 2025/11/28
pub struct UpSafeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UpSafeCell<T> {}

impl<T> UpSafeCell<T> {
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    ///
    /// 获取可变借用
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
