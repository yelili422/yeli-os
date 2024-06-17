use fs::{
    block_dev::{BlockDevice, InodeType, BLOCK_SIZE},
    inode::Inode,
    FileSystem,
};
use spin::{Mutex, MutexGuard};
use std::{
    env,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};

pub struct BlockFile(pub Mutex<File>);

impl BlockDevice for BlockFile {
    fn read(&self, block_id: u64, buf: &mut [u8]) {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * (BLOCK_SIZE as u64)))
            .unwrap();
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE);
    }

    fn write(&self, block_id: u64, buf: &[u8]) {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * (BLOCK_SIZE as u64)))
            .unwrap();
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE);
    }
}

const FS_SIZE: u64 = 16 * 1024 * 1024; // 16 MiB

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        panic!("Usage: mkfs <fs.img> [files]")
    }

    let fs_name = &args[1];
    let fs_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(fs_name)
        .unwrap();
    fs_fd.set_len(FS_SIZE).unwrap();

    let fs = FileSystem::create(Arc::new(BlockFile(Mutex::new(fs_fd))), 4096, 1).unwrap();

    let fs_root_lock = fs.root();
    let mut fs_root = fs_root_lock.lock();

    let bin_dir_lock = fs
        .create_inode(&mut fs_root, "/bin", InodeType::Directory)
        .unwrap();
    let mut bin_dir = bin_dir_lock.lock();

    for i in 2..args.len() {
        let file_path = Path::new(&args[i]);
        if !file_path.exists() {
            panic!("File not found: {}", file_path.display());
        }

        if file_path.is_dir() {
            for entry in file_path.read_dir().unwrap() {
                let entry = entry.unwrap();
                let file_path = entry.path();
                if file_path.is_file() {
                    eprintln!("copying {} to /bin ...", file_path.display());
                    copy2(&fs, &file_path, &mut bin_dir);
                }
            }
        } else if file_path.is_file() {
            eprintln!("copying {} to /bin ...", file_path.display());
            copy2(&fs, file_path, &mut bin_dir);
        }
    }
}

fn copy2(fs: &Arc<FileSystem>, src: &Path, dst: &mut MutexGuard<Inode>) {
    assert!(src.is_file());
    assert!(dst.type_ == InodeType::Directory);

    let short_name = src.file_name().unwrap().to_str().unwrap();

    let mut source_file = OpenOptions::new().read(true).open(src).unwrap();
    let source_len = source_file.metadata().unwrap().len();

    let file_lock = fs.create_inode(dst, short_name, InodeType::File).unwrap();
    let mut file = file_lock.lock();
    fs.resize_inode(&mut file, source_len as usize).unwrap();

    let mut buffer = [0u8; BLOCK_SIZE];
    let mut read_count = 0;
    loop {
        let offset = source_file.read(&mut buffer).unwrap();
        if offset == 0 {
            break;
        }

        fs.write_inode(&mut file, read_count, &buffer);
        read_count += offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::process::Command;

    #[test]
    fn test_mkfs() {
        let fs_img_path = "./target/test_fs.img";

        Command::new("make")
            .arg("install")
            .current_dir("../user")
            .env("INSTALL_DIR", "../fs/target/bins")
            .assert()
            .success();

        Command::new("cargo")
            .arg("build")
            .arg("--bin")
            .arg("mkfs")
            .assert()
            .success();

        Command::cargo_bin("mkfs")
            .unwrap()
            .arg(fs_img_path)
            .arg("./target/bins/")
            .assert()
            .success();

        let fs_img = OpenOptions::new()
            .read(true)
            .write(true)
            .open(fs_img_path)
            .unwrap();
        let fs = FileSystem::open(Arc::new(BlockFile(Mutex::new(fs_img))), true).unwrap();
        let fs_root_lock = fs.root();
        let fs_root = fs_root_lock.lock();

        let bin_dir_lock = fs.look_up(&fs_root, "/bin").unwrap();
        let bin_dir = bin_dir_lock.lock();
        assert_eq!(bin_dir.type_, InodeType::Directory);

        let hello_lock = fs.look_up(&bin_dir, "hello").unwrap();
        let hello = hello_lock.lock();
        assert_eq!(hello.type_, InodeType::File);
    }
}
