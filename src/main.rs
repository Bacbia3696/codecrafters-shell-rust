use std::env;
#[allow(unused_imports)]
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    let builtins = ["echo", "exit", "type"];
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
                let output = std::process::Command::new(command[0])
                    .args(&command[1..])
                    .output();
                match output {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            print!("{}", String::from_utf8_lossy(&output.stdout));
                        }
                        if !output.stderr.is_empty() {
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
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
