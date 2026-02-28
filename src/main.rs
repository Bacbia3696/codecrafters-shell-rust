use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Editor, Helper};
use std::env;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

struct ShellCompleter {
    builtins: Vec<String>,
    filename_completer: FilenameCompleter,
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        let (start, word) = extract_word(line, pos);

        // Check if completing the first word (command)
        let is_first_word = line[..pos].split_whitespace().count() <= 1;

        if is_first_word {
            let mut candidates = Vec::new();

            // Complete builtins
            for builtin in &self.builtins {
                if builtin.starts_with(&word) {
                    candidates.push(Pair {
                        display: builtin.clone(),
                        replacement: builtin.clone(),
                    });
                }
            }

            // Complete PATH binaries
            if let Ok(path) = env::var("PATH") {
                for dir in path.split(':') {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            if let Ok(name) = entry.file_name().into_string()
                                && name.starts_with(&word)
                            {
                                candidates.push(Pair {
                                    display: name.clone(),
                                    replacement: name.clone(),
                                });
                            }
                        }
                    }
                }
            }

            candidates.sort_by(|a, b| a.display.cmp(&b.display));
            candidates.dedup_by(|a, b| a.display == b.display);
            Ok((start, candidates))
        } else {
            // Use filename completer for arguments
            self.filename_completer.complete(line, pos, ctx)
        }
    }
}

fn extract_word(line: &str, pos: usize) -> (usize, String) {
    let before = &line[..pos];
    let start = before
        .rfind(|c: char| c.is_whitespace())
        .map_or(0, |i| i + 1);
    (start, line[start..pos].to_string())
}

impl Helper for ShellCompleter {}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Highlighter for ShellCompleter {}
impl Validator for ShellCompleter {}

fn main() -> rustyline::Result<()> {
    let builtins: Vec<String> = vec![
        "echo".into(),
        "exit".into(),
        "type".into(),
        "pwd".into(),
        "cd".into(),
    ];
    let completer = ShellCompleter {
        builtins: builtins.clone(),
        filename_completer: FilenameCompleter::new(),
    };
    let mut rl: Editor<ShellCompleter, DefaultHistory> = Editor::new()?;
    rl.set_helper(Some(completer));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(input) => {
                rl.add_history_entry(&input)?;

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
                        } else if builtins.contains(&command[1]) {
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
    let mut in_double_quote = false;
    let chars = input.chars();

    for c in chars {
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
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
