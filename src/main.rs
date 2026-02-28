mod commands;
mod completion;
mod redirection;
mod tokenize;

use commands::{BUILTINS, execute_builtin};
use completion::ShellCompleter;
use redirection::{handle_output, parse_pipeline};
use rustyline::CompletionType;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor, Result};
use std::process::{Command, Stdio};
use tokenize::tokenize;

fn main() -> Result<()> {
    let builtins: Vec<String> = BUILTINS.iter().map(|s| s.to_string()).collect();
    let completer = ShellCompleter::new(builtins.clone());

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut rl: Editor<ShellCompleter, DefaultHistory> = Editor::with_config(config)?;
    rl.set_helper(Some(completer));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(input) => {
                rl.add_history_entry(&input)?;

                let tokens = tokenize(&input);
                if tokens.is_empty() {
                    continue;
                }

                let commands = parse_pipeline(tokens);
                if commands.is_empty() {
                    continue;
                }

                // Check for exit command
                if commands.len() == 1 && commands[0].args.first().is_some_and(|a| a == "exit") {
                    break;
                }

                if commands.len() == 1 {
                    // Single command - handle builtins and redirections
                    let parsed = &commands[0];
                    let command = &parsed.args;

                    if command.is_empty() {
                        continue;
                    }

                    if BUILTINS.contains(&command[0].as_str()) {
                        let result = execute_builtin(&command[0], command);
                        handle_output(&result, parsed);
                    } else {
                        // External commands stream output directly (no buffering)
                        if let Err(e) = execute_external(&command[0], command, parsed) {
                            eprintln!("{}", e);
                        }
                    }
                } else {
                    // Pipeline - execute multiple commands with pipes (output is inherited)
                    if let Err(e) = execute_pipeline(&commands) {
                        eprintln!("{}", e);
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

fn execute_external(
    cmd: &str,
    args: &[String],
    parsed: &redirection::ParsedCommand,
) -> std::result::Result<String, String> {
    let mut command = Command::new(cmd);
    command.args(&args[1..]);

    if let Some(ref r) = parsed.redirect_stderr {
        if let Ok(file) = open_file(&r.file, r.append) {
            command.stderr(file);
        }
    }

    if let Some(ref r) = parsed.redirect_stdout {
        if let Ok(file) = open_file(&r.file, r.append) {
            command.stdout(file);
            return match command.status() {
                Ok(_) => Ok(String::new()),
                Err(_) => Err(format!("{}: command not found", cmd)),
            };
        }
    }

    match command.status() {
        Ok(_) => Ok(String::new()),
        Err(_) => Err(format!("{}: command not found", cmd)),
    }
}

fn open_file(path: &str, append: bool) -> std::result::Result<std::fs::File, std::io::Error> {
    if append {
        std::fs::OpenOptions::new().create(true).append(true).open(path)
    } else {
        std::fs::File::create(path)
    }
}

/// Executes a pipeline of commands, connecting stdout of each to stdin of the next.
fn execute_pipeline(commands: &[redirection::ParsedCommand]) -> std::result::Result<(), String> {
    use std::io::{Read, Write};

    if commands.is_empty() {
        return Ok(());
    }

    let last = commands.last().unwrap();
    let mut children: Vec<std::process::Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;

    for (i, parsed) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;
        let cmd = &parsed.args[0];

        if BUILTINS.contains(&cmd.as_str()) {
            // Wait for all previous children to complete
            for child in &mut children {
                let _ = child.wait();
            }
            children.clear();

            // Consume previous stdout (builtins don't read stdin)
            drop(prev_stdout.take());

            // Execute builtin
            let output = execute_builtin(cmd, &parsed.args);

            if is_last {
                if let Some(ref r) = last.redirect_stdout {
                    if let Ok(content) = &output {
                        let _ = redirection::write_to_file(&r.file, content, r.append);
                    }
                } else if let Ok(content) = &output {
                    print!("{}", content);
                }
            } else {
                // For builtin -> external, buffer output and feed via cat
                if let Ok(content) = &output {
                    let data = content.as_bytes().to_vec();
                    let mut feeder = Command::new("cat")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .map_err(|_| "Failed to spawn cat".to_string())?;
                    
                    if let Some(mut stdin) = feeder.stdin.take() {
                        let _ = stdin.write_all(&data);
                    }
                    
                    prev_stdout = feeder.stdout.take();
                    children.push(feeder);
                }
            }
        } else {
            // External command
            let mut command = Command::new(cmd);
            command.args(&parsed.args[1..]);

            // Set up stdin from previous command
            if let Some(stdout) = prev_stdout.take() {
                command.stdin(Stdio::from(stdout));
            } else if i > 0 {
                command.stdin(Stdio::inherit());
            }

            // Set up stdout
            if is_last {
                if let Some(ref r) = last.redirect_stdout {
                    if let Ok(file) = open_file(&r.file, r.append) {
                        command.stdout(file);
                    }
                } else {
                    command.stdout(Stdio::piped());
                }
            } else {
                command.stdout(Stdio::piped());
            }

            let mut child = command.spawn()
                .map_err(|_| format!("{}: command not found", cmd))?;

            if !is_last {
                prev_stdout = child.stdout.take();
            }
            children.push(child);
        }
    }

    // Stream output from last command if not redirected
    if let Some(last_parsed) = commands.last()
        && last_parsed.redirect_stdout.is_none()
        && let Some(last_child) = children.last_mut()
        && let Some(mut stdout) = last_child.stdout.take()
    {
        let mut buf = [0u8; 1024];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 { break; }
            let _ = std::io::stdout().write_all(&buf[..n]);
            let _ = std::io::stdout().flush();
        }
    }

    // Wait for remaining children
    for child in &mut children {
        let _ = child.wait();
    }

    Ok(())
}
