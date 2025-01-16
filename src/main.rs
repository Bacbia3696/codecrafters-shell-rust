#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // Wait for user input
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        println!("{}: command not found", input.trim());
        io::stdout().flush().unwrap();
    }
}
