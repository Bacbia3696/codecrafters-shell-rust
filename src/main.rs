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
                    let paths = env::var("PATH").unwrap_or_default();
                    let mut found = false;
                    for path in paths.split(':') {
                        let full_path = format!("{}/{}", path, command[1]);
                        if let Ok(metadata) = std::fs::metadata(&full_path)
                            && metadata.is_file()
                            && metadata.permissions().mode() & 0o111 != 0
                        {
                            println!("{} is {}", command[1], full_path);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        println!("{}: not found", command[1]);
                    }
                }
            }
            "echo" => {
                let output = command[1..].join(" ");
                println!("{}", output);
            }
            _ => println!("{}: command not found", command[0]),
        }
    }
}
