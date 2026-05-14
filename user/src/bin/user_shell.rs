#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{close, dup, exec, fork, getpid, open, pipe, println, waitpid, OpenFlags};

const LF: u8 = 0x0a_u8;
const CR: u8 = 0x0d_u8;
const DL: u8 = 0x7f_u8;
const BS: u8 = 0x08_u8;

#[derive(Debug)]
struct ProcessArguments {
    input: String,
    output: String,
    args_copy: Vec<String>,
    args_addr: Vec<*const u8>,
}

impl ProcessArguments {
    ///
    /// 解析启动参数
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/24
    pub fn new(command: &str) -> Self {
        // 以空格符分割参数
        let args: Vec<_> = command.split(" ").collect();

        // 记录参数
        let mut args_copy: Vec<String> = args
            .iter()
            .filter(|&args| !args.is_empty())
            .map(|&arg| {
                let mut string = String::new();
                string.push_str(arg);
                string.push('\0');
                string
            })
            .collect();

        // 从入参列表中挑出重定向输入
        let mut input = String::new();
        if let Some((idx, _)) = args_copy
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == "<\0")
        {
            input = args_copy[idx + 1].clone();
            args_copy.drain(idx..=idx + 1);
        }

        // 从入参列表中挑出重定向输出
        let mut output = String::new();
        if let Some((idx, _)) = args_copy
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == ">\0")
        {
            output = args_copy[idx + 1].clone();
            args_copy.drain(idx..=idx + 1);
        }

        // 记录入参地址
        let mut args_addr: Vec<*const u8> = args_copy.iter().map(|arg| arg.as_ptr()).collect();
        args_addr.push(core::ptr::null::<u8>());

        Self {
            input,
            output,
            args_copy,
            args_addr,
        }
    }
}

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
                    // 以管道符分隔需要执行的指令
                    let split_arr: Vec<_> = line.as_str().split('|').collect();
                    let process_arguments_list: Vec<_> = split_arr
                        .iter()
                        .map(|&cmd| ProcessArguments::new(cmd))
                        .collect();

                    // 校验是否有输入输出
                    let mut valid = true;
                    for (i, process_args) in process_arguments_list.iter().enumerate() {
                        if i == 0 {
                            if !process_args.output.is_empty() {
                                valid = false;
                            }
                        } else if i == process_arguments_list.len() - 1 {
                            if !process_args.input.is_empty() {
                                valid = false;
                            }
                        } else if !process_args.output.is_empty() || !process_args.input.is_empty()
                        {
                            valid = false;
                        }
                    }

                    // 当没有管道符时不需要输入输出
                    if process_arguments_list.len() == 1 {
                        valid = true;
                    }

                    if !valid {
                        println!("Invalid command: Inputs/Outputs cannot be correctly bind")
                    } else {
                        // 创建执行 程序数-1 对管道
                        let mut pipes_fd: Vec<[usize; 2]> = Vec::new();
                        if !process_arguments_list.is_empty() {
                            for _ in 0..process_arguments_list.len() - 1 {
                                let mut pipe_fd = [0_usize; 2];
                                pipe(&mut pipe_fd);
                                pipes_fd.push(pipe_fd);
                            }
                        }

                        // 执行程序
                        let mut children: Vec<_> = Vec::new();
                        for (i, process_argument) in process_arguments_list.iter().enumerate() {
                            // fork 进程
                            let pid = fork();
                            if pid == 0 {
                                // 子进程
                                let input = &process_argument.input;
                                let output = &process_argument.output;
                                let args_copy = &process_argument.args_copy;
                                let args_addr = &process_argument.args_addr;

                                // 重定向输入
                                if !input.is_empty() {
                                    let input_fd = open(input.as_str(), OpenFlags::RDONLY);
                                    if input_fd == -1 {
                                        println!("Error when opening file {}", input);
                                        return -4;
                                    }
                                    let input_fd = input_fd as usize;
                                    // 关闭终端输入
                                    close(0);
                                    // 复制一份新的文件描述符，输入使用新的文件描述符
                                    assert_eq!(dup(input_fd), 0);
                                    // 关闭刚刚打开的文件，因为持有文件进程数 != 0，因此输入不会被关闭，生命周期交由操作系统控制
                                    close(input_fd);
                                }

                                // 重定向输出，流程如输入
                                if !output.is_empty() {
                                    let output_fd = open(
                                        output.as_str(),
                                        OpenFlags::CREATE | OpenFlags::WRONLY,
                                    );
                                    if output_fd == -1 {
                                        println!("Error when opening file {}", output);
                                        return -4;
                                    }
                                    let output_fd = output_fd as usize;
                                    close(1);
                                    assert_eq!(dup(output_fd), 1);
                                    close(output_fd);
                                }

                                // 除了第一个运行的程序外其余程序都关闭输入流，因为是通过管道输入数据的
                                if i > 0 {
                                    close(0);
                                    let read_end = pipes_fd.get(i - 1).unwrap()[0];
                                    assert_eq!(dup(read_end), 0);
                                }

                                // 除了最后一个运行的程序其余程序都关闭输出流，因为输出数据是下一个程序的输入，通过管道输入
                                if i < process_arguments_list.len() - 1 {
                                    close(1);
                                    let write_end = pipes_fd.get(1).unwrap()[1];
                                    assert_eq!(dup(write_end), 1);
                                }

                                // 关闭管道，因为上面每个进程都对管道的输入输出进行了按需复制，因此这里的关闭只是 持有数-1，管道的生命周期交由操作系统控制
                                for pipe_fd in pipes_fd.iter() {
                                    close(pipe_fd[0]);
                                    close(pipe_fd[1]);
                                }

                                // 执行程序
                                if exec(args_copy[0].as_str(), args_addr.as_slice()) == -1 {
                                    println!("Error when executing");
                                    return -4;
                                }
                                unreachable!();
                            } else {
                                // 父进程
                                children.push(pid);
                            }
                        }
                        // 关闭进程管道
                        for pipe_fd in pipes_fd.iter() {
                            close(pipe_fd[0]);
                            close(pipe_fd[1]);
                        }
                        // 等待子进程执行完毕
                        let mut exit_code: i32 = 0;
                        for pid in children.into_iter() {
                            let exit_pid = waitpid(pid as usize, &mut exit_code);
                            assert_eq!(pid, exit_pid);
                        }
                    }

                    // 退出 shell
                    if line.as_str() == "exit" {
                        println!("Shell pid: {}, exit", getpid());
                        return 0;
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
