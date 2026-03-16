#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, waitpid};

static SUCCESS_TESTS: &[(&str, &str, &str, &str, i32)] = &[
    ("exit\0", "\0", "\0", "\0", 0),
    ("fantastic-text\0", "\0", "\0", "\0", 0),
    ("fork-test-simple\0", "\0", "\0", "\0", 0),
    ("fork-test\0", "\0", "\0", "\0", 0),
    ("fork-test-2\0", "\0", "\0", "\0", 0),
    ("fork-tree\0", "\0", "\0", "\0", 0),
    ("hello-world\0", "\0", "\0", "\0", 0),
    ("matrix\0", "\0", "\0", "\0", 0),
    ("sleep-simple\0", "\0", "\0", "\0", 0),
    ("sleep\0", "\0", "\0", "\0", 0),
    ("yield\0", "\0", "\0", "\0", 0),
];

static FAIL_TESTS: &[(&str, &str, &str, &str, i32)] = &[("stack-overflow\0", "\0", "\0", "\0", -2)];

fn run_tests(tests: &[(&str, &str, &str, &str, i32)]) -> i32 {
    let mut pass_num = 0;
    let mut arr: [*const u8; 4] = [
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
        core::ptr::null::<u8>(),
    ];

    for test in tests {
        println!("user tests: Running {}", test.0);
        arr[0] = test.0.as_ptr();
        arr[1] = core::ptr::null();
        arr[2] = core::ptr::null();
        arr[3] = core::ptr::null();
        if test.1 != "\0" {
            arr[1] = test.1.as_ptr();
            if test.2 != "\0" {
                arr[2] = test.2.as_ptr();
                if test.3 != "\0" {
                    arr[3] = test.3.as_ptr();
                }
            }
        }

        let pid = fork();
        if pid == 0 {
            exec(test.0);
            panic!("unreachable!");
        } else {
            let mut exit_code: i32 = Default::default();
            let wait_pid = waitpid(pid as usize, &mut exit_code);
            assert_eq!(pid, wait_pid);
            if exit_code == test.4 {
                pass_num = pass_num + 1
            }
            println!(
                "\x1b[32m user tests: Test {} in Process {} exited with code {} \x1b[0m",
                test.0, pid, exit_code
            );
        }
    }
    pass_num
}

///
/// 打印 hello world
///
/// @author: tryte
///
/// @date: 2026/3/10
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let succ_num = run_tests(SUCCESS_TESTS);
    let err_num = run_tests(FAIL_TESTS);
    if succ_num == SUCCESS_TESTS.len() as i32 && err_num == FAIL_TESTS.len() as i32 {
        println!(
            "{} of success apps, {} of failed apps run correctly.\nuser tests passed!",
            succ_num, err_num
        );
        return 0;
    } else if succ_num != SUCCESS_TESTS.len() as i32 {
        println!(
            "all success apps is {}, but only passed {}",
            SUCCESS_TESTS.len(),
            succ_num
        );
    }

    if err_num != FAIL_TESTS.len() as i32 {
        println!(
            "all failed app is {}, but only passed {}",
            FAIL_TESTS.len(),
            err_num
        );
    }
    println!("user tests failed!");
    -1
}
