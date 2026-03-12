use crate::mm::translated_byte_buffer;
use crate::print;
use crate::sbi::console_get_char;
use crate::task::{current_user_token, suspend_current_and_run_next};

/// 终端输入文件描述符
const FD_STDIN: usize = 0;

/// 终端输出文件描述符
const FD_STDOUT: usize = 1;

///
/// 系统读
///
/// @author: tryte
///
/// @date: 2026/3/12
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_get_char();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}

///
/// 系统写
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!")
        }
    }
}
