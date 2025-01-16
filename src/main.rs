#[allow(unused_imports)]
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    // Wait for user input
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        let trimed_input = input.trim();
        if trimed_input == "exit 0" {
            return ExitCode::from(0);
        }
        let commands = trimed_input.split_whitespace().collect::<Vec<_>>();
        match commands[0] {
            "echo" => {
                println!("{}", commands[1..].join(" "));
            }
            _ => {
                println!("{}: command not found", trimed_input.trim());
            }
        }
        io::stdout().flush().unwrap();
    }
}
