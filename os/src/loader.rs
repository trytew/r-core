use crate::println;
use alloc::vec::Vec;
use lazy_static::lazy_static;

lazy_static! {
    ///
    /// 记录所有应用名称
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/6
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        unsafe extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as *const () as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                // 循环读取应用名称
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}

///
/// 获取进程数量
///
/// @author: tryte
///
/// @date: 2025/12/16
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as *const usize).read_volatile() }
}

///
/// 读取应用内容
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    unsafe extern "C" {
        fn _num_app();
    }

    let num_app_ptr = _num_app as *const () as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

///
/// 根据应用名称获取应用内容
///
/// @author: tryte
///
/// @date: 2026/3/5
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    (0..num_app)
        .find(|&i| {
            // 根据应用名称查找对应的应用序号
            println!("{}", APP_NAMES[i]);
            APP_NAMES[i] == name
        })
        .map(get_app_data)
}

///
/// 列出所有应用
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn list_apps() {
    println!("/**** APPS ****/");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("/**************/");
}
