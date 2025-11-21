use std::fs::{File, read_dir};
use std::io::Result;
use std::io::Write;
use std::process::id;

static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn main() {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
}

///
/// 创建用户态应用程序连接汇编文件
///
/// @author: tryte
///
/// @date: 2025/11/21
fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/linker_app.asm")?;
    let mut apps: Vec<_> = read_dir("../user/src/bin")?
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find(".").unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();

    apps.sort();

    writeln!(
        f,
        r#"    .align 3
    .section .data
    .globl _num_app
_num_app:
    .quard {}"#,
        apps.len(),
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quard app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quard app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .globl app_{0}_start
    .globl app_{0}_end
app_{0}_start:
    .incbin "{2}{1}.bin"
app_{0}_end:"#,
            idx, app, TARGET_PATH,
        )?;
    }

    Ok(())
}
