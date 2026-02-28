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
use std::os::unix::io::{AsRawFd, FromRawFd};
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

    // Handle stderr redirection
    if let Some(ref redirection) = parsed.redirect_stderr {
        let file_result = if redirection.append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirection.file)
        } else {
            std::fs::File::create(&redirection.file)
        };
        if let Ok(file) = file_result {
            command.stderr(file);
        }
    }

    // Handle stdout redirection
    if let Some(ref redirection) = parsed.redirect_stdout {
        let file_result = if redirection.append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&redirection.file)
        } else {
            std::fs::File::create(&redirection.file)
        };
        if let Ok(file) = file_result {
            command.stdout(file);
            // For stdout redirection, return empty string (output goes to file)
            match command.status() {
                Ok(_) => return Ok(String::new()),
                Err(_) => return Err(format!("{}: command not found", cmd)),
            }
        }
    }

    // For interactive commands (no redirection), inherit stdout/stderr for streaming
    // This allows commands like `tail -f` to work correctly
    match command.status() {
        Ok(_) => Ok(String::new()),
        Err(_) => Err(format!("{}: command not found", cmd)),
    }
}

/// Executes a pipeline of commands, connecting stdout of each to stdin of the next.
fn execute_pipeline(commands: &[redirection::ParsedCommand]) -> std::result::Result<(), String> {
    use std::io::{Read, Write};
    
    if commands.is_empty() {
        return Ok(());
    }

    let mut children: Vec<std::process::Child> = Vec::new();
    let mut prev_stdout: Option<std::os::unix::io::RawFd> = None;
    let last_command = commands.last().unwrap();

    for (i, parsed) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;
        let cmd = &parsed.args[0];
        let args = &parsed.args[1..];

        let mut command = Command::new(cmd);
        command.args(args);

        // Set up stdin from previous command's stdout
        if let Some(fd) = prev_stdout {
            unsafe {
                command.stdin(Stdio::from_raw_fd(fd));
            }
        } else {
            command.stdin(Stdio::inherit());
        }

        if is_last {
            // Last command - handle stdout redirection or capture for streaming
            if let Some(ref redirection) = last_command.redirect_stdout {
                let file_result = if redirection.append {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&redirection.file)
                } else {
                    std::fs::File::create(&redirection.file)
                };
                if let Ok(file) = file_result {
                    command.stdout(file);
                }
            } else {
                // Capture stdout for streaming
                command.stdout(Stdio::piped());
            }
        } else {
            // Intermediate command - pipe stdout to next command
            command.stdout(Stdio::piped());
        }

        let mut child = command.spawn()
            .map_err(|_| format!("{}: command not found", cmd))?;

        // Save stdout fd for next command
        if !is_last
            && let Some(stdout) = child.stdout.take()
        {
            prev_stdout = Some(stdout.as_raw_fd());
            // Don't drop stdout - we need it for the next command
            std::mem::forget(stdout);
        }

        children.push(child);
    }

    // Stream output from last command if not redirected
    if let Some(last_child) = children.last_mut()
        && last_command.redirect_stdout.is_none()
        && let Some(mut stdout) = last_child.stdout.take()
    {
        // Copy stdout to shell's stdout in real-time
        let mut buffer = [0u8; 1024];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let _ = std::io::stdout().write_all(&buffer[..n]);
                    let _ = std::io::stdout().flush();
                }
                Err(_) => break,
            }
        }
    }

    // Wait for all children to complete
    for child in children.iter_mut() {
        let _ = child.wait();
    }

    Ok(())
}
