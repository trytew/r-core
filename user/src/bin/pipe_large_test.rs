#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::format;
use user_lib::{close, fork, get_time, pipe, read, wait, write};

const LENGTH: usize = 3000;

#[unsafe(no_mangle)]
fn main() -> i32 {
    let mut down_pipe_fd = [0_usize; 2];
    let mut up_pipe_fd = [0_usize; 2];
    pipe(&mut down_pipe_fd);
    pipe(&mut up_pipe_fd);
    let mut random_str = [0_u8; LENGTH];
    if fork() == 0 {
        close(down_pipe_fd[1]);
        close(up_pipe_fd[0]);
        assert_eq!(read(down_pipe_fd[0], &mut random_str) as usize, LENGTH);
        close(down_pipe_fd[0]);
        let sum = random_str.iter().map(|v| *v as usize).sum::<usize>();
        println!("sum = {}(child)", sum);
        let sum_str = format!("{}", sum);
        write(up_pipe_fd[1], sum_str.as_bytes());
        close(up_pipe_fd[1]);
        println!("Child process exited!");
        0
    } else {
        close(down_pipe_fd[0]);
        close(up_pipe_fd[1]);
        for ch in random_str.iter_mut() {
            *ch = get_time() as u8;
        }
        assert_eq!(
            write(down_pipe_fd[1], &random_str) as usize,
            random_str.len()
        );
        close(down_pipe_fd[1]);
        let sum: usize = random_str.iter().map(|v| *v as usize).sum::<usize>();
        println!("sum = {}(parent)", sum);
        let mut child_result = [0_u8; 32];
        let result_len = read(up_pipe_fd[0], &mut child_result) as usize;
        close(up_pipe_fd[0]);
        assert_eq!(
            sum,
            str::parse::<usize>(core::str::from_utf8(&child_result[..result_len]).unwrap())
                .unwrap()
        );
        let mut _unused: i32 = 0;
        wait(&mut _unused);
        println!("pipe-large-test passed!");
        0
    }
}
