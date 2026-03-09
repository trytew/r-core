#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Rust user shell");

    let mut line: String = String::new();
    println!(">> ");
    loop {
        let c = getchar();
    }

    0
}
