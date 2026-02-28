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

struct ParsedCommand {
    args: Vec<String>,
    redirect_to: Option<String>,
}

fn parse_command(tokens: Vec<String>) -> ParsedCommand {
    let mut args = Vec::new();
    let mut redirect_to = None;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        // Check for redirection operators: >, 1>, 2>
        let is_redirect = token == ">" || token == "1>" || token == "2>";

        if is_redirect {
            if i + 1 < tokens.len() {
                redirect_to = Some(tokens[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else {
            args.push(token.clone());
            i += 1;
        }
    }

    ParsedCommand { args, redirect_to }
}

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

                let tokens: Vec<_> = tokenize(&input);
                if tokens.is_empty() {
                    continue;
                }
                let parsed = parse_command(tokens);
                let command = &parsed.args;

                if command.is_empty() {
                    continue;
                }

                let result = match command[0].as_str() {
                    "exit" => break,
                    "pwd" => match env::current_dir() {
                        Ok(path) => Ok(format!("{}\n", path.display())),
                        Err(e) => Err(format!("Error getting current directory: {}", e)),
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
                                Err(format!("cd: {}: No such file or directory", dir))
                            } else {
                                Ok(String::new())
                            }
                        } else {
                            Err("cd: HOME not set".to_string())
                        }
                    }
                    "type" => {
                        if command.len() < 2 {
                            Ok("type: missing argument\n".to_string())
                        } else if builtins.contains(&command[1]) {
                            Ok(format!("{} is a shell builtin\n", command[1]))
                        } else {
                            match full_path(&command[1]) {
                                Some(path) => Ok(format!("{} is {}\n", command[1], path)),
                                None => Ok(format!("{}: not found\n", command[1])),
                            }
                        }
                    }
                    "echo" => Ok(command[1..].join(" ") + "\n"),
                    _ => {
                        let mut cmd = std::process::Command::new(&command[0]);
                        cmd.args(&command[1..]);
                        match cmd.output() {
                            Ok(output) => {
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                if !stderr.is_empty() {
                                    eprint!("{}", stderr);
                                }
                                Ok(stdout)
                            }
                            Err(_) => Err(format!("{}: command not found", command[0])),
                        }
                    }
                };

                // Handle output
                if let Ok(output) = result {
                    if !output.is_empty() {
                        if let Some(ref file) = parsed.redirect_to {
                            if let Err(e) = std::fs::write(file, &output) {
                                eprintln!("{}: {}", file, e);
                            }
                        } else {
                            print!("{}", output);
                        }
                    }
                } else if let Err(e) = result {
                    if let Some(ref _file) = parsed.redirect_to {
                        // Redirect stderr to file too? For now, just print to stderr
                        eprintln!("{}", e);
                    } else {
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

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        // Handle backslash escape outside quotes and inside double quotes
        if c == '\\' && !in_single_quote {
            if let Some(&next) = chars.peek() {
                chars.next(); // consume the escaped character
                current.push(next);
            }
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c == '>' && !in_single_quote && !in_double_quote {
            // Handle redirection: check if current ends with a digit (e.g., "1>")
            if !current.is_empty() && current.chars().last().unwrap().is_ascii_digit() {
                current.push(c);
                tokens.push(current.clone());
                current.clear();
            } else {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push(c.to_string());
            }
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
