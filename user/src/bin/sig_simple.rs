#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, getpid, kill, sig_return, sigaction, SignalAction, SIGUSR1};

fn func() {
    println!("user_sig_test passed");
    sig_return();
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut new = SignalAction::default();
    let mut old = SignalAction::default();
    new.handler = func as *const () as usize;

    println!("signal_simple: sigaction");
    if sigaction(SIGUSR1, Some(&new), Some(&mut old)) < 0 {
        panic!("Sigaction failed!");
    }

    println!("signal_simple: kill");
    if kill(getpid() as usize, SIGUSR1) < 0 {
        println!("Kill failed!");
        exit(1);
    }
    println!("signal_simple: Done");
    0
}
