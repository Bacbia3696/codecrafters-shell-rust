use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    loop {
        print!("$ ");
        if io::stdout().flush().is_err() {
            eprintln!("Failed to flush stdout");
            continue;
        }

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("Error reading input");
            continue;
        }
        let input = input.trim();
        let commands: Vec<String> = shell_words::split(input).expect("Failed to parsed command");
        if commands.is_empty() {
            println!();
            continue;
        }

        match commands[0].as_str() {
            "echo" => {
                // Print even if there are no additional arguments.
                println!("{}", commands[1..].join(" "));
            }
            "exit" => return ExitCode::from(0),
            "pwd" => match env::current_dir() {
                Ok(dir) => println!("{}", dir.to_string_lossy()),
                Err(e) => eprintln!("Error retrieving current directory: {}", e),
            },
            "cd" => {
                if commands.len() < 2 {
                    eprintln!("cd: missing operand");
                    continue;
                }
                let mut new_dir_str = commands[1].to_string();
                if commands[1] == "~" {
                    // Using env::var for portability; consider using the `dirs` crate for a robust solution.
                    if let Ok(home) = env::var("HOME") {
                        new_dir_str = home;
                    } else {
                        eprintln!("cd: Unable to determine home directory");
                        continue;
                    }
                }
                let new_dir = Path::new(&new_dir_str);
                if env::set_current_dir(new_dir).is_err() {
                    eprintln!("cd: {}: No such file or directory", commands[1]);
                }
            }
            "type" => {
                if commands.len() < 2 {
                    eprintln!("type: missing operand");
                    continue;
                }
                match commands[1].as_str() {
                    "echo" | "exit" | "type" | "pwd" | "cd" => {
                        println!("{} is a shell builtin", commands[1]);
                    }
                    _ => match find_command_path(&commands[1]) {
                        Some(command_path) => {
                            println!("{} is {}", commands[1], command_path);
                        }
                        None => {
                            println!("{}: not found", commands[1]);
                        }
                    },
                }
            }
            cmd => {
                if let Some(_command_path) = find_command_path(cmd) {
                    match Command::new(cmd).args(&commands[1..]).output() {
                        Ok(output) => {
                            print!("{}", String::from_utf8_lossy(&output.stdout));
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                        Err(e) => eprintln!("Error executing {}: {}", cmd, e),
                    }
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}

fn find_command_path(command: &str) -> Option<String> {
    let paths = env::var_os("PATH")?;
    // Use env::split_paths for cross-platform compatibility.
    for path in env::split_paths(&paths) {
        let cmd_path = path.join(command);
        if cmd_path.exists() {
            // Optionally, check if the file is executable using metadata.
            return cmd_path.to_str().map(String::from);
        }
    }
    None
}
