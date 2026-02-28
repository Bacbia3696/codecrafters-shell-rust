use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::env;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() -> rustyline::Result<()> {
    let builtins = ["echo", "exit", "type", "pwd", "cd"];
    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(input) => {
                rl.add_history_entry(input.as_str())?;

                let command: Vec<_> = tokenize(&input);
                if command.is_empty() {
                    continue;
                }
                match command[0].as_str() {
                    "exit" => break,
                    "pwd" => match env::current_dir() {
                        Ok(path) => println!("{}", path.display()),
                        Err(e) => eprintln!("Error getting current directory: {}", e),
                    },
                    "cd" => {
                        let target = command.get(1).map_or_else(
                            || env::var("HOME").ok(),
                            |arg| {
                                if *arg == "~" {
                                    env::var("HOME").ok()
                                } else if let Some(rest) = arg.strip_prefix("~/") {
                                    env::var("HOME").map(|h| format!("{}/{}", h, rest)).ok()
                                } else {
                                    Some(arg.to_string())
                                }
                            },
                        );
                        if let Some(dir) = target {
                            if env::set_current_dir(&dir).is_err() {
                                eprintln!("cd: {}: No such file or directory", dir);
                            }
                        } else {
                            eprintln!("cd: HOME not set");
                        }
                    }
                    "type" => {
                        if command.len() < 2 {
                            println!("type: missing argument");
                        } else if builtins.contains(&command[1].as_str()) {
                            println!("{} is a shell builtin", command[1]);
                        } else {
                            match full_path(&command[1]) {
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
                        let mut cmd = std::process::Command::new(command[0].clone());
                        cmd.args(&command[1..]);
                        if cmd.status().is_err() {
                            eprintln!("{}: command not found", command[0]);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let chars = input.chars().peekable();

    for c in chars {
        if c == '\'' && !in_single_quote {
            in_single_quote = true;
        } else if c == '\'' && in_single_quote {
            in_single_quote = false;
        } else if c.is_whitespace() && !in_single_quote {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn full_path(command: &str) -> Option<String> {
    env::var("PATH").ok()?.split(':').find_map(|path| {
        let full = format!("{}/{}", path, command);
        std::fs::metadata(&full)
            .ok()
            .filter(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)?;
        Some(full)
    })
}
