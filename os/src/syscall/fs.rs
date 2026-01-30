use crate::mm::translated_byte_buffer;
use crate::print;
use crate::task::current_user_token;

const FD_STDOUT: usize = 1;

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
