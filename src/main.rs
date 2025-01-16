#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
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
        let env_path = env::var("PATH").unwrap_or_default();
        let paths = env_path.split(':');
        match commands[0] {
            "echo" => {
                println!("{}", commands[1..].join(" "));
            }
            "type" => match commands[1] {
                "echo" | "exit" | "type" => {
                    println!("{} is a shell builtin", commands[1]);
                }
                _ => {
                    let mut found = false;
                    for p in paths {
                        if let Ok(entries) = fs::read_dir(p) {
                            for entry in entries {
                                match entry {
                                    Ok(entry) => {
                                        let command =
                                            entry.file_name().to_string_lossy().to_string();
                                        if command == commands[1] {
                                            println!("{} is {}/{}", command, p, command);
                                            found = true;
                                            break;
                                        }
                                    }
                                    Err(e) => eprintln!("Error reading entry: {}", e),
                                }
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    if !found {
                        println!("{}: not found", commands[1]);
                    }
                }
            },
            _ => {
                println!("{}: command not found", trimed_input.trim());
            }
        }
        io::stdout().flush().unwrap();
    }
}
