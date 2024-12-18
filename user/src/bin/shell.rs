#![no_std]
#![no_main]

use system::print;

use alloc::string::String;

extern crate alloc;

fn main() {
    loop {
        print!("> ");
        let mut input = String::new();
        // system::io::stdin().read_line(&mut input).unwrap();
        // let input = input.trim();
        // if input == "exit" {
        //     break;
        // }
        // let output = system::process::execute(input);
        // println!("{}", output);
    }
}
