use crate::fs::{make_pipe, open_file, OpenFlags};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use alloc::sync::Arc;

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

///
/// 创建管道
///
/// @author: tryte
///
/// @date: 2026/4/18
pub fn sys_pipe(pipe: *mut usize) -> isize {
    // 获取当前进程
    let task = current_task().unwrap();
    // 获取进程 MMU
    let token = current_user_token();
    // 获取进程控制块
    let mut inner = task.inner_exclusive_access();
    let (pipe_read, pipe_write) = make_pipe();
    // 分配进程文件描述符
    let read_fd = inner.alloc_fd();
    // 将读端添加到进程文件已打开文件描述符列表
    inner.fd_table[read_fd] = Some(pipe_read);
    // 分配进程文件描述符
    let write_fd = inner.alloc_fd();
    // 将写端添加到进程文件已打开文件描述符列表
    inner.fd_table[write_fd] = Some(pipe_write);
    // 返回读端内存地址
    *translated_refmut(token, pipe) = read_fd;
    // 返回写端内存地址
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

///
/// 复制文件描述符
///
/// @author: tryte
///
/// @date: 2026/5/14
pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}
