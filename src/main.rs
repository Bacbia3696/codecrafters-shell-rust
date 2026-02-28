#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin()
            .read_line(&mut command)
            .expect("Failed to read line");
        let command: Vec<_> = command.split_whitespace().collect();
        if command.is_empty() {
            continue;
        }
        match command[0] {
            "exit" => break,
            "echo" => {
                let output = command[1..].join(" ");
                println!("{}", output);
            }
            _ => println!("{}: command not found", command[0]),
        }
    }
}
