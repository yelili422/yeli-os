use core::panic::PanicInfo;

use crate::syscall::sbi::shutdown;

use log::error;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {

    if let Some(location) = info.location() {
        error!("Panicked at {}:{} {}", location.file(), location.line(), info.message().unwrap());
    } else {
        error!("Panicked: {}", info.message().unwrap());
    }
    shutdown()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("[failed]\n");
    error!("Error: {}\n", info);
    shutdown()
}

