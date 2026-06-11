#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::event_get;
use virtio_input_decoder::{DecodeType, Key, KeyType};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Input device event test");
    loop {
        if let Some(event) = event_get() {
            println!("{:?}", event.event_type);
            if let Some(decode_type) = event.decode() {
                println!("{:?}", decode_type);
                if let DecodeType::Key(key, key_type) = decode_type {
                    if key == Key::Enter && key_type == KeyType::Press {
                        break;
                    }
                }
            }
        }
    }
    0
}
