use std::{env, fs::OpenOptions, path::Path, sync::Arc};

use fs::{
    block_dev::{BlockDevice, InodeType, BLOCK_SIZE},
    FileSystem,
};
use log::{LevelFilter, Metadata, Record};
use spin::Mutex;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
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

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        unimplemented!()
    }

    fn log(&self, record: &Record) {
        println!("{} - {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

const FS_SIZE: u64 = 16 * 1024 * 1024; // 16 MiB

fn main() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Debug);

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

    let bin_dir_lock = fs_root.create("/bin", InodeType::Directory).unwrap();
    let mut bin_dir = bin_dir_lock.lock();

    for i in 2..args.len() {
        let file_path = &args[i];
        let short_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();

        let mut source_file = OpenOptions::new().read(true).open(file_path).unwrap();
        let source_len = source_file.metadata().unwrap().len();

        let file_lock = bin_dir.create(short_name, InodeType::File).unwrap();
        let mut file = file_lock.lock();
        file.resize(source_len as usize).unwrap();

        let mut buffer = [0u8; BLOCK_SIZE];
        let mut read_count = 0;
        loop {
            let offset = source_file.read(&mut buffer).unwrap();
            if offset == 0 {
                break;
            }

            file.write_data(read_count, &buffer);
            read_count += offset;
        }
    }
}
