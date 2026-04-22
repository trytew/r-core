use easy_fs_use::easy_fs_pack;

mod block_file;

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

#[test]
fn efs_test() -> std::io::Result<()> {
    use easy_fs::{EasyFileSystem, BLOCK_SZ};
    use easy_fs_use::BlockFile;
    use std::fs::OpenOptions;
    use std::sync::{Arc, Mutex};

    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    // 创建文件系统
    EasyFileSystem::create(block_file.clone(), 4096, 1);
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);
    // 创建文件
    root_inode.create("file_a");
    root_inode.create("file_b");
    // 列出跟目录下的所有文件/目录
    for name in root_inode.ls() {
        println!("{}", name);
    }
    // 向 file_a 文件写入内容
    let file_a = root_inode.find("file_a").unwrap();
    let greet_str = "Hello, world!";
    file_a.write_at(0, greet_str.as_bytes());
    // 读取 file_a 内容并返回读取长度
    let mut buffer = [0_u8; 233];
    let len = file_a.read_at(0, &mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap());

    // 随机字符写入测试
    let mut random_str_test = |len: usize| {
        // 清空 file_a 文件内容
        file_a.clear();
        assert_eq!(file_a.read_at(0, &mut buffer), 0);
        let mut str = String::new();
        // 生成随机字符
        use rand;
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        file_a.write_at(0, str.as_bytes());
        // 读取文件内容
        let mut read_buffer = [0_u8; 127];
        let mut offset = 0_usize;
        let mut read_str = String::new();
        loop {
            let len = file_a.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
        assert_eq!(str, read_str);
    };

    println!("4 * BLOCK_SZ");
    random_str_test(4 * BLOCK_SZ);
    println!("8 * BLOCK_SZ + BLOCK_SZ / 2");
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    println!("100 * BLOCK_SZ");
    random_str_test(100 * BLOCK_SZ);
    println!("70 * BLOCK_SZ + BLOCK_SZ / 7");
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    println!("(12 + 128) * BLOCK_SZ");
    random_str_test((12 + 128) * BLOCK_SZ);
    println!("400 * BLOCK_SZ");
    random_str_test(400 * BLOCK_SZ);
    println!("1000 * BLOCK_SZ");
    random_str_test(1000 * BLOCK_SZ);
    println!("2000 * BLOCK_SZ");
    random_str_test(2000 * BLOCK_SZ);

    Ok(())
}
