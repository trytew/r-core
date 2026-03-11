#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::string::String;
use user_lib::console::getchar;
use user_lib::{exec, fork, println, waitpid};

const LF: u8 = 0x0a_u8;
const CR: u8 = 0x0d_u8;
const DL: u8 = 0x7f_u8;
const BS: u8 = 0x08_u8;

///
/// shell 进程
///
/// @author: tryte
///
/// @date: 2026/3/10
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Rust user shell");

    let mut line: String = String::new();
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    line.push('\0');
                    let pid = fork();
                    if pid == 0 {
                        // 子进程
                        if exec(line.as_str()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
