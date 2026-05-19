#![no_std]
#![no_main]
#![allow(clippy::empty_loop)]

#[macro_use]
extern crate user_lib;

use riscv::register::sstatus;
use riscv::register::sstatus::SPP;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Try to access privileged CSR in U Mode");
    println!("Kernel should kill this application!");
    unsafe {
        sstatus::set_spp(SPP::User);
    }
    0
}
