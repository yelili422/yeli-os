use crate::{print, println, syscall::shutdown};
use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("[test] {}...\t", core::any::type_name::<T>());
        self();
        println!("ok");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    // TODO: parse args...

    // run tests
    println!("\n[test] Running {} test(s)...", tests.len());
    for test in tests {
        test.run();
    }
    println!("[test] Test finished.");

    // TODO: communicate through stdio

    // TODO: exit code
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\n[panic] at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("[panic] {}", info.message().unwrap());
    }
    shutdown()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("failed\n{}\n", &info);
    shutdown()
}
