use std::env;
#[allow(unused_imports)]
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    let builtins = ["echo", "exit", "type", "pwd", "cd"];
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
            "pwd" => match env::current_dir() {
                Ok(path) => println!("{}", path.display()),
                Err(e) => eprintln!("Error getting current directory: {}", e),
            },
            "cd" => {
                if command.len() < 2 {
                    // cd with no args goes to home directory
                    match env::var("HOME") {
                        Ok(home) => {
                            if let Err(e) = env::set_current_dir(&home) {
                                eprintln!("cd: {}: {}", home, e);
                            }
                        }
                        Err(_) => eprintln!("cd: HOME not set"),
                    }
                } else {
                    let target = if command[1] == "~" {
                        env::var("HOME").unwrap_or_else(|_| "~".to_string())
                    } else if let Some(rest) = command[1].strip_prefix("~/") {
                        format!("{}/{}", env::var("HOME").unwrap_or_default(), rest)
                    } else {
                        command[1].to_string()
                    };
                    if env::set_current_dir(&target).is_err() {
                        eprintln!("cd: {}: No such file or directory", target);
                    }
                }
            }
            "type" => {
                if command.len() < 2 {
                    println!("type: missing argument");
                } else if builtins.contains(&command[1]) {
                    println!("{} is a shell builtin", command[1]);
                } else {
                    match full_path(command[1]) {
                        Some(path) => println!("{} is {}", command[1], path),
                        None => println!("{}: not found", command[1]),
                    }
                }
            }
            "echo" => {
                let output = command[1..].join(" ");
                println!("{}", output);
            }
            _ => {
                let status = std::process::Command::new(command[0])
                    .args(&command[1..])
                    .status();
                match status {
                    Ok(status) => {
                        if !status.success() {
                            eprintln!("{}: exit code {}", command[0], status.code().unwrap_or(-1));
                        }
                    }
                    Err(_) => eprintln!("{}: command not found", command[0]),
                }
            }
        }
    }
}

fn full_path(command: &str) -> Option<String> {
    let paths = env::var("PATH").unwrap_or_default();
    for path in paths.split(':') {
        let full_path = format!("{}/{}", path, command);
        if let Ok(metadata) = std::fs::metadata(&full_path)
            && metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
        {
            return Some(full_path);
        }
    }
    None
}
