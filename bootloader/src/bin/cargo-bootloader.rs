
use std::process::Command;

pub fn main() {
    Command::new("rust-objcopy")
        .arg("/Users/tang/Projects/yeli-os/kernel/target/riscv64gc-unknown-none-elf/debug/yeli-os")
        .arg("--strip-all")
        .args(["-O", "binary"])
        .arg("/Users/tang/Projects/yeli-os/kernel/target/riscv64gc-unknown-none-elf/debug/test.bin")

        .spawn()
        .unwrap();

    Command::new("qemu-system-riscv64")
        .args(["-machine", "virt"])
        .arg("-nographic")
        .args(["-bios", "/Users/tang/Projects/yeli-os/bootloader/opensbi-0.6-rv64-qemu.bin"])
        .args(["-device", "loader,file=/Users/tang/Projects/yeli-os/kernel/target/riscv64gc-unknown-none-elf/debug/test.bin,addr=0x80200000"])
        .spawn()
        .unwrap();
}
