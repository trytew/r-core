use crate::fs::{open_file, OpenFlags};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

///
/// 系统读
///
/// @author: tryte
///
/// @date: 2026/3/12
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    // 获取当前进程的 MMU 设置
    let token = current_user_token();
    // 获取当前进程控制块
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        drop(inner);
        // 读取内容
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

///
/// 系统写
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    // 获取当前进程的 MMU 设置
    let token = current_user_token();
    // 获取当前进程控制块
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writeable() {
            return -1;
        }
        let file = file.clone();
        drop(inner);
        // 写入内容
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

///
/// 打开文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn sys_open(path: *const u8, flags: u32) -> isize {
    // 获取当前进程控制块
    let task = current_task().unwrap();
    // 获取当前进程的 MMU 设置
    let token = current_user_token();
    // 读取文件路径
    let path = translated_str(token, path);
    // 打开文件
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        // 将文件设置进新的文件描述符并返回文件描述符
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

///
/// 关闭文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}
