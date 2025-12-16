use std::process::Command;
use std::{fs, io};

///
/// 应用程序构建脚本
///
/// @author: tryte
///
/// @date: 2025/12/16
fn main() -> io::Result<()> {

    // riscv64 架构芯片 bios 运行起始地址为：0x80000000，硬件决定
    // riscv64 架构芯片 bios 运行初始化后第一条指令（即内核） 运行起始地址为：0x80200000，bios决定，可在 qemu 运行指令中设置
    // riscv64 架构芯片 应用程序 运行起始地址为：0x80400000，代码决定
    // 应用起始地址
    let base_address: u64 = 0x8040_0000;
    let step: u64 = 0x20_000;
    let linker = "src/linker.ld";

    let mut app_id: u64 = 0;

    // 读取 scr/bin 目录
    let mut apps: Vec<String> = fs::read_dir("src/bin")?
        .filter_map(|entry| {
            entry.ok()
        })
        .filter_map(|entry| {
            entry.path()
                .file_name()
                .and_then(|n| { n.to_str() })
                .map(|s| { s.to_string() })
        })
        .collect();

    apps.sort();

    for app in apps {

        // 去掉 .rs 后缀
        let app_name = match app.rfind('.') {
            None => {
                continue
            }
            Some(pos) => {
                &app[..pos]
            }
        };

        // 读取 linker.ld 原始内容
        let original = fs::read_to_string(linker)?;

        let old_addr = format!("{:#x}", base_address);
        let new_addr = format!("{:#x}", base_address + step * app_id);

        // 替换地址
        let modified = original.replace(&old_addr, &new_addr);

        fs::write(linker, modified)?;

        // 执行 cargo build
        let status = Command::new("cargo").
            args(["build", "--bin", app_name, "--release"]).
            status().
            expect("failed to execute cargo");

        if !status.success() {
            panic!("cargo build failed for {}", app_name);
        }

        println!(
            "[build.rs] application {} start with address {}",
            app_name, new_addr,
        );

        // 恢复 linker.ld
        fs::write(linker, original)?;

        app_id += 1;
    }

    Ok(())
}
