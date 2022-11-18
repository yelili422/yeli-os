#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yeli_os::test_runner)]
#![reexport_test_harness_main = "test_main"]


#[test_case]
fn test_boot() {
    assert_eq!(1, 1);
}
