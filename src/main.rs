#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    env, fs,
    path::Path,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        let trimed_input = input.trim();
        let commands = trimed_input.split_whitespace().collect::<Vec<_>>();
        match commands[0] {
            "echo" => {
                println!("{}", commands[1..].join(" "));
            }
            "exit" => {
                return ExitCode::from(0);
            }
            "pwd" => {
                let current_dir = env::current_dir().expect("can get current dir");
                println!("{}", current_dir.to_string_lossy());
            }
            "cd" => {
                let new_dir = Path::new(commands[1]);
                if env::set_current_dir(new_dir).is_err() {
                    eprintln!("cd: {}: No such file or directory", commands[1])
                }
            }
            "type" => match commands[1] {
                "echo" | "exit" | "type" | "pwd" | "cd" => {
                    println!("{} is a shell builtin", commands[1]);
                }
                _ => match find_command_path(commands[1]) {
                    Some(command_path) => {
                        println!("{} is {}", commands[1], command_path)
                    }
                    None => {
                        println!("{}: not found", commands[1])
                    }
                },
            },
            _ => match find_command_path(commands[0]) {
                Some(_) => {
                    let output = Command::new(commands[0])
                        .args(&commands[1..])
                        .output() // Capture the output
                        .expect("Failed to execute command");

                    io::stdout().write_all(&output.stdout).unwrap();
                }
                None => {
                    println!("{}: command not found", commands[0])
                }
            },
        }
        io::stdout().flush().unwrap();
    }
}

fn find_command_path(command: &str) -> Option<String> {
    let env_path = env::var("PATH").unwrap_or_default();
    let paths = env_path.split(':');
    for p in paths {
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let file = entry.file_name().to_string_lossy().to_string();
                        if file == command {
                            return Some(format!("{}/{}", p, command));
                        }
                    }
                    Err(e) => eprintln!("Error reading entry: {}", e),
                }
            }
        }
    }
    None
}
