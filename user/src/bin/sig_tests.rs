#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
    close, exit, fork, getpid, kill, pipe, read, sig_proc_mask, sig_return, sigaction, sleep,
    wait, write, SignalAction, SignalFlags, SIGCONT, SIGKILL, SIGSTOP, SIGUSR1,
};

fn func() {
    println!("func triggered");
    sig_return();
}

fn user_sig_test_fail_signum() {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();

    new.handler = func as *const () as usize;
    if sigaction(50, Some(&new), Some(&mut old)) >= 0 {
        panic!("Wrong sigaction but success!")
    }
}

fn user_sig_test_kill() {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();

    new.handler = func as *const () as usize;
    if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
        panic!("Sigaction failed!");
    }
    if kill(getpid() as usize, SIGUSR1) < 0 {
        println!("Kill failed!");
        exit(1);
    }
}

fn user_sig_test_multi_proc_signals() {
    let pid = fork();
    if pid == 0 {
        let mut new = SignalAction::default();
        let mut old = SignalAction::default();

        new.handler = func as *const () as usize;
        if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
            panic!("Sigaction failed!");
        }
    } else {
        if kill(pid as usize, SIGUSR1) < 0 {
            println!("Kill failed!");
            exit(1);
        }
        let mut exit_code = 0;
        wait(&mut exit_code);
    }
}

fn user_sig_test_restore() {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();
    let mut old2 = SignalAction::default();

    new.handler = func as *const () as usize;
    if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
        panic!("Sigaction failed!");
    }
    if sigaction(SIGUSR1, Some(&old), Some(&mut old2)) < 0 {
        panic!("Sigaction failed!");
    }

    if old2.handler != new.handler {
        println!("Restore failed!");
        exit(-1);
    }
}

fn kernel_sig_test_ignore() {
    sig_proc_mask(SignalFlags::SIGSTOP.bits() as u32);
    if kill(getpid() as usize, SignalFlags::SIGSTOP.bits()) < 0 {
        println!("kill failed\n");
        exit(-1);
    }
}

fn kernel_sig_test_stop_cont() {
    let pid = fork();
    if pid == 0 {
        kill(getpid() as usize, SIGSTOP);
        sleep(5000);
        exit(-1);
    } else {
        sleep(1000);
        kill(pid as usize, SIGCONT);
        let mut exit_code = 0;
        wait(&mut exit_code);
    }
}

fn kernel_sig_test_fail_ignore_kill() {
    let new = SignalAction::default();
    let mut old = SignalAction::default();

    if sigaction(SIGKILL, Some(&new), Some(&mut old)) >= 0 {
        panic!("Should not set sigaction to kill!");
    }

    if sigaction(SIGKILL, Some(&new), None) >= 0 {
        panic!("Should not set sigaction to kill!");
    }

    if sigaction(SIGKILL, None, Some(&mut old)) >= 0 {
        panic!("Should not set sigaction to kill!");
    }
}

fn final_sig_test() {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();

    new.handler = func as *const () as usize;

    let mut pipe_fd = [0_usize; 2];
    pipe(&mut pipe_fd);

    let pid = fork();
    if pid == 0 {
        close(pipe_fd[0]);
        if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
            panic!("Sigaction failed!");
        }
        write(pipe_fd[1], &[0_u8]);
        close(pipe_fd[1]);
        loop {}
    } else {
        close(pipe_fd[1]);
        let mut buf = [0_u8; 1];
        assert_eq!(read(pipe_fd[0], &mut buf), 1);
        close(pipe_fd[0]);
        if kill(pid as usize, SIGUSR1) < 0 {
            println!("Kill failed!");
            exit(-1);
        }
        sleep(100);
        kill(pid as usize, SIGKILL);
        let mut exit_code: i32 = 0;
        wait(&mut exit_code);
    }
}

fn run(f: fn()) -> bool {
    let pid = fork();
    if pid == 0 {
        f();
        exit(0);
    } else {
        let mut exit_code: i32 = 0;
        wait(&mut exit_code);
        if exit_code != 0 {
            println!("FAILED!");
        } else {
            println!("OK!");
        }
        exit_code == 0
    }
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let tests: [(fn(), &str); 8] = [
        (user_sig_test_fail_signum, "user_sig_test_fail_signum"),
        (user_sig_test_kill, "user_sig_test_kill"),
        (
            user_sig_test_multi_proc_signals,
            "user_sig_test_multi_proc_signals",
        ),
        (user_sig_test_restore, "user_sig_test_restore"),
        (kernel_sig_test_ignore, "kernel_sig_test_ignore"),
        (kernel_sig_test_stop_cont, "kernel_sig_test_stop_cont"),
        (
            kernel_sig_test_fail_ignore_kill,
            "kernel_sig_test_fail_ignore_kill",
        ),
        (final_sig_test, "final_sig_test"),
    ];

    let mut fail_num = 0;
    for test in tests {
        println!("Testing {}", test.1);
        if !run(test.0) {
            fail_num += 1;
        }
    }
    if fail_num == 0 {
        println!("ALL TEST PASSED");
        0
    } else {
        println!("SOME TEST FAILED");
        -1
    }
}
