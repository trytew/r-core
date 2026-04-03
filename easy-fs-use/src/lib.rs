use clap::{App, Arg};
use easy_fs::EasyFileSystem;
use std::fs::{read_dir, File, OpenOptions};
use std::io::Read;
use std::sync::{Arc, Mutex};

mod block_file;

pub use block_file::*;

///
/// 将一个目录下的文件复制到文件系统
///
/// @author: tryte
///
/// @date: 2026/4/2
pub fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .get_matches();

    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512)?;
        f
    })));

    // 16MB，接近 4095 个文件
    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps: Vec<_> = read_dir(src_path)?
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        let mut host_file = File::open(format!("{}{}", target_path, app))?;
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data)?;
        let inode = root_inode.create(app.as_str()).unwrap();
        inode.write_at(0, all_data.as_slice());
    }
    Ok(())
}
