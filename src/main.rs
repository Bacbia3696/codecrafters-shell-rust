mod commands;
mod completion;
mod redirection;
mod tokenize;

use commands::{BUILTINS, execute_builtin};
use completion::ShellCompleter;
use redirection::{handle_output, parse_pipeline};
use rustyline::CompletionType;
use rustyline::error::ReadlineError;
use rustyline::history::{DefaultHistory, History};
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
    let _ = rl.history_mut().ignore_dups(false);
    let _ = rl.history_mut().clear();

    load_history(&mut rl);
    let mut last_written_index: usize = 0;

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(input) => {
                rl.add_history_entry(&input)?;

                let commands = parse_pipeline(tokenize(&input));
                if commands.is_empty() {
                    continue;
                }

                if commands.len() == 1 && commands[0].args.first().is_some_and(|a| a == "exit") {
                    break;
                }

                let parsed = &commands[0];
                if commands.len() == 1 && !parsed.args.is_empty() {
                    match parsed.args[0].as_str() {
                        "history" => handle_history(&mut rl, &parsed.args, &mut last_written_index),
                        cmd if BUILTINS.contains(&cmd) => {
                            let result = execute_builtin(cmd, &parsed.args);
                            handle_output(&result, parsed);
                        }
                        cmd => {
                            if let Err(e) = execute_external(cmd, &parsed.args, parsed) {
                                eprintln!("{}", e);
                            }
                        }
                    }
                } else if let Err(e) = execute_pipeline(&commands) {
                    eprintln!("{}", e);
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    save_history(&rl);
    Ok(())
}

fn load_history(rl: &mut Editor<ShellCompleter, DefaultHistory>) {
    if let Ok(histfile) = std::env::var("HISTFILE")
        && let Ok(content) = std::fs::read_to_string(&histfile)
    {
        for line in content.lines() {
            if !line.is_empty() {
                let _ = rl.add_history_entry(line);
            }
        }
    }
}

fn save_history(rl: &Editor<ShellCompleter, DefaultHistory>) {
    if let Ok(histfile) = std::env::var("HISTFILE") {
        let content = rl
            .history()
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let _ = std::fs::write(histfile, content);
    }
}

fn handle_history(
    rl: &mut Editor<ShellCompleter, DefaultHistory>,
    args: &[String],
    last_written_index: &mut usize,
) {
    match args.get(1).map(|s| s.as_str()) {
        Some("-r") => {
            if let Some(path) = args.get(2)
                && let Ok(content) = std::fs::read_to_string(path)
            {
                for line in content.lines() {
                    if !line.is_empty() {
                        let _ = rl.add_history_entry(line);
                    }
                }
            }
        }
        Some("-w") => {
            if let Some(path) = args.get(2) {
                let content = rl
                    .history()
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                let _ = std::fs::write(path, content);
                *last_written_index = rl.history().len();
            }
        }
        Some("-a") => {
            if let Some(path) = args.get(2) {
                let current_len = rl.history().len();
                if current_len > *last_written_index {
                    let content: String = rl
                        .history()
                        .iter()
                        .skip(*last_written_index)
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                        + "\n";
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                    {
                        let _ = std::io::Write::write_all(&mut file, content.as_bytes());
                    }
                }
                *last_written_index = current_len;
            }
        }
        Some(n) => display_history(rl, n.parse::<usize>().ok()),
        None => display_history(rl, None),
    }
}

fn display_history(rl: &Editor<ShellCompleter, DefaultHistory>, limit: Option<usize>) {
    let entries: Vec<&str> = rl.history().iter().map(|s| s.as_str()).collect();
    let start = limit.map_or(0, |n| entries.len().saturating_sub(n));

    for (i, entry) in entries.iter().enumerate().skip(start) {
        println!("{:>4}  {}", i + 1, entry);
    }
}

fn execute_external(
    cmd: &str,
    args: &[String],
    parsed: &redirection::ParsedCommand,
) -> std::result::Result<String, String> {
    let mut command = Command::new(cmd);
    command.args(&args[1..]);

    if let Some(ref r) = parsed.redirect_stderr
        && let Ok(file) = open_file(&r.file, r.append)
    {
        command.stderr(file);
    }

    if let Some(ref r) = parsed.redirect_stdout
        && let Ok(file) = open_file(&r.file, r.append)
    {
        command.stdout(file);
        return match command.status() {
            Ok(_) => Ok(String::new()),
            Err(_) => Err(format!("{}: command not found", cmd)),
        };
    }

    match command.status() {
        Ok(_) => Ok(String::new()),
        Err(_) => Err(format!("{}: command not found", cmd)),
    }
}

fn open_file(path: &str, append: bool) -> std::result::Result<std::fs::File, std::io::Error> {
    if append {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    } else {
        std::fs::File::create(path)
    }
}

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
            for child in &mut children {
                let _ = child.wait();
            }
            children.clear();
            drop(prev_stdout.take());

            let output = execute_builtin(cmd, &parsed.args);

            if is_last {
                if let Some(ref r) = last.redirect_stdout {
                    if let Ok(content) = &output {
                        let _ = redirection::write_to_file(&r.file, content, r.append);
                    }
                } else if let Ok(content) = &output {
                    print!("{}", content);
                }
            } else if let Ok(content) = &output {
                let mut feeder = Command::new("cat")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .map_err(|_| "Failed to spawn cat".to_string())?;

                if let Some(mut stdin) = feeder.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }
                prev_stdout = feeder.stdout.take();
                children.push(feeder);
            }
        } else {
            let mut command = Command::new(cmd);
            command.args(&parsed.args[1..]);

            if let Some(stdout) = prev_stdout.take() {
                command.stdin(Stdio::from(stdout));
            } else if i > 0 {
                command.stdin(Stdio::inherit());
            }

            if is_last && let Some(ref r) = last.redirect_stdout {
                if let Ok(file) = open_file(&r.file, r.append) {
                    command.stdout(file);
                }
            } else {
                command.stdout(Stdio::piped());
            }

            let mut child = command
                .spawn()
                .map_err(|_| format!("{}: command not found", cmd))?;

            if !is_last {
                prev_stdout = child.stdout.take();
            }
            children.push(child);
        }
    }

    if let Some(last_child) = children.last_mut()
        && last.redirect_stdout.is_none()
        && let Some(mut stdout) = last_child.stdout.take()
    {
        let mut buf = [0u8; 1024];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 {
                break;
            }
            let _ = std::io::stdout().write_all(&buf[..n]);
            let _ = std::io::stdout().flush();
        }
    }

    for child in &mut children {
        let _ = child.wait();
    }

    Ok(())
}
