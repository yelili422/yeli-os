use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // println!("[failed]\n");
    // println!("Error: {}\n", info);
    // // exit_qemu(QemuExitCode::Failed);
    loop {}
}

