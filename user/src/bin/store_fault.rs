#![no_std]
#![no_main]
#![allow(clippy::empty_loop)]

#[macro_use]
extern crate user_lib;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Into Test store_fault, We will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe {
        core::ptr::null_mut::<u8>().write_volatile(0);
    }
    0
}
