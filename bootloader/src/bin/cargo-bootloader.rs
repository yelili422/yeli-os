use anyhow::{anyhow, Result};
use std::{env, fs, path::Path, process::Command};

pub fn main() -> Result<()> {
    // println!("{:?}", env::args());

    let mut args = env::args();

    args.next();
    args.next();

    let image_path = args.next().clone().ok_or_else(|| anyhow!("parsing image path error"))?;
    let base_dir = fs::canonicalize(Path::new(&image_path).parent().unwrap())?;
    let bin_file_path = format!("{}/tmp.bin", &base_dir.to_str().unwrap());
    let sbi_path = "/Users/tang/Projects/yeli-os/bootloader/opensbi-0.6-rv64-qemu.bin";

    Command::new("rust-objcopy")
        .arg(&image_path)
        .arg("--strip-all")
        .args(["-O", "binary"])
        .arg(&bin_file_path)
        .spawn()?;

    Command::new("qemu-system-riscv64")
        .args(["-machine", "virt"])
        .arg("-nographic")
        .args(["-bios", sbi_path])
        .args([
            "-device",
            format!("loader,file={},addr=0x80200000", bin_file_path).as_str(),
        ])
        .spawn()?;

    Ok(())
}
