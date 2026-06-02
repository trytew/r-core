use core::cell::{RefCell, RefMut, UnsafeCell};
use core::ops::{Deref, DerefMut};
use lazy_static::lazy_static;
use riscv::register::sstatus;

lazy_static! {
    static ref INTR_MASKING_INFO: UpSafeCellRaw<IntrMaskingInfo> =
        unsafe { UpSafeCellRaw::new(IntrMaskingInfo::new()) };
}

// ///
// /// 多线程 RefCell
// ///
// /// @author: tryte
// ///
// /// @date: 2025/11/28
// pub struct UpIntrFreeCell<T> {
//     inner: RefCell<T>,
// }
//
// unsafe impl<T> Sync for UpSafeCell<T> {}
//
// impl<T> UpSafeCell<T> {
//     pub unsafe fn new(value: T) -> Self {
//         Self {
//             inner: RefCell::new(value),
//         }
//     }
//
//     ///
//     /// 获取可变借用
//     ///
//     /// @author: tryte
//     ///
//     /// @date: 2025/12/18
//     pub fn exclusive_access(&self) -> RefMut<'_, T> {
//         self.inner.borrow_mut()
//     }
// }

///
/// 多线程 RefCell
///
/// @author: tryte
///
/// @date: 2026/6/2
pub struct UpSafeCellRaw<T> {
    inner: UnsafeCell<T>,
}

impl<T> UpSafeCellRaw<T> {
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut (*self.inner.get()) }
    }
}

unsafe impl<T> Sync for UpSafeCellRaw<T> {}

///
/// 中断屏蔽信息
///
/// @author: tryte
///
/// @date: 2026/6/2
pub struct IntrMaskingInfo {
    /// 屏蔽状态
    nested_level: usize,
    /// 记录屏蔽前中断接收总开关状态
    side_before_masking: bool,
}

impl IntrMaskingInfo {
    ///
    /// 实例化中断屏蔽信息
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn new() -> Self {
        Self {
            nested_level: 0,
            side_before_masking: false,
        }
    }

    ///
    /// 进入免中断打断状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn enter(&mut self) {
        // 记录中断接收总开关
        let sie = sstatus::read().sie();
        // 关闭中断接收
        unsafe {
            sstatus::clear_sie();
        }
        // 判断当前层级，假设一个被 UpIntrFreeCell 包裹的全局变量被借用 exclusive_access，
        // 那么这个变量有可能在函数内被多次借用，这个时候就不能单次借用后又恢复中断，因为如果单次借用后直接恢复中断，这个时候如果被中断打断了
        // 那还是会产生借用 panic，因此需要记录借用次数，当第一次被借用时记录中断接收状态，在最后一次归还时恢复
        if self.nested_level == 0 {
            self.side_before_masking = sie;
        }
        // 借用次数+1
        self.nested_level += 1;
    }

    ///
    /// 退出免中断打断状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn exit(&mut self) {
        self.nested_level -= 1;
        // 借用结束后恢复中断接收状态
        if self.nested_level == 0 && self.side_before_masking {
            unsafe {
                sstatus::set_sie();
            }
        }
    }
}

///
/// 可变借用
///
/// @author: tryte
///
/// @date: 2026/6/2
pub struct UpIntrRefMut<'a, T>(Option<RefMut<'a, T>>);

impl<'a, T> Drop for UpIntrRefMut<'a, T> {
    ///
    /// 销毁钩子
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn drop(&mut self) {
        // 当被销毁时需要恢复中断接收状态
        self.0 = None;
        INTR_MASKING_INFO.get_mut().exit();
    }
}

impl<'a, T> Deref for UpIntrRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap().deref()
    }
}

impl<'a, T> DerefMut for UpIntrRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap().deref_mut()
    }
}

///
/// 多线程 RefCell
///
/// @author: tryte
///
/// @date: 2026/6/2
pub struct UpIntrFreeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UpIntrFreeCell<T> {}

impl<T> UpIntrFreeCell<T> {
    ///
    /// 实例化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    ///
    /// 获取借用
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn exclusive_access(&self) -> UpIntrRefMut<'_, T> {
        INTR_MASKING_INFO.get_mut().enter();
        UpIntrRefMut(Some(self.inner.borrow_mut()))
    }

    ///
    /// 获取会话形式的可变借用，回调函数结束后自动回收
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn exclusive_session<F, V>(&self, f: F) -> V
    where
        F: FnOnce(&mut T) -> V,
    {
        let mut inner = self.exclusive_access();
        f(inner.deref_mut())
    }
}
