mod commands;
mod completion;
mod redirection;
mod tokenize;

use commands::{execute_builtin, BUILTINS};
use completion::ShellCompleter;
use redirection::{handle_output, parse_command};
use rustyline::error::ReadlineError;
use rustyline::{Editor, Result};
use rustyline::history::DefaultHistory;
use tokenize::tokenize;

fn main() -> Result<()> {
    let builtins: Vec<String> = BUILTINS.iter().map(|s| s.to_string()).collect();
    let completer = ShellCompleter::new(builtins.clone());
    let mut rl: Editor<ShellCompleter, DefaultHistory> = Editor::new()?;
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

                let parsed = parse_command(tokens);
                let command = &parsed.args;

                if command.is_empty() {
                    continue;
                }

                let result = if command[0] == "exit" {
                    break;
                } else if BUILTINS.contains(&command[0].as_str()) {
                    execute_builtin(&command[0], command)
                } else {
                    execute_external(&command[0], command, &parsed)
                };

                handle_output(&result, &parsed);
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
    let mut command = std::process::Command::new(cmd);
    command.args(&args[1..]);

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

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !stderr.is_empty() && parsed.redirect_stderr.is_none() {
                eprint!("{}", stderr);
            }
            Ok(stdout)
        }
        Err(_) => Err(format!("{}: command not found", cmd)),
    }
}
