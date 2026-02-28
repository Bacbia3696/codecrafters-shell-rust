use std::io::Write;

/// Represents a redirection operator.
#[derive(Debug, Clone)]
pub struct Redirection {
    pub file: String,
    pub append: bool,
}

/// A parsed command with arguments and redirections.
#[derive(Debug, Default)]
pub struct ParsedCommand {
    pub args: Vec<String>,
    pub redirect_stdout: Option<Redirection>,
    pub redirect_stderr: Option<Redirection>,
}

/// Parses tokens into a ParsedCommand, extracting redirection operators.
pub fn parse_command(tokens: Vec<String>) -> ParsedCommand {
    let mut args = Vec::new();
    let mut redirect_stdout = None;
    let mut redirect_stderr = None;
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            ">" | "1>" => {
                redirect_stdout = tokens.get(i + 1).map(|f| Redirection {
                    file: f.clone(),
                    append: false,
                });
                i += 2;
            }
            ">>" | "1>>" => {
                redirect_stdout = tokens.get(i + 1).map(|f| Redirection {
                    file: f.clone(),
                    append: true,
                });
                i += 2;
            }
            "2>" => {
                redirect_stderr = tokens.get(i + 1).map(|f| Redirection {
                    file: f.clone(),
                    append: false,
                });
                i += 2;
            }
            "2>>" => {
                redirect_stderr = tokens.get(i + 1).map(|f| Redirection {
                    file: f.clone(),
                    append: true,
                });
                i += 2;
            }
            _ => {
                args.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    ParsedCommand {
        args,
        redirect_stdout,
        redirect_stderr,
    }
}

/// Writes content to a file, with optional append mode.
pub fn write_to_file(file: &str, content: &str, append: bool) -> Result<(), std::io::Error> {
    if append {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut f| f.write_all(content.as_bytes()))
    } else {
        std::fs::write(file, content)
    }
}

/// Creates or truncates a file.
pub fn create_file(file: &str, append: bool) -> Result<(), std::io::Error> {
    if append {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .map(|_| ())
    } else {
        std::fs::File::create(file).map(|_| ())
    }
}

/// Handles output redirection for command results.
pub fn handle_output(result: &Result<String, String>, parsed: &ParsedCommand) {
    use crate::commands::BUILTINS;
    use std::io::{self, Write};

    // Handle stdout redirection
    if let Some(ref redirection) = parsed.redirect_stdout {
        let output = result.as_ref().ok().map(|s| s.as_str()).unwrap_or("");
        if !output.is_empty() {
            let _ = write_to_file(&redirection.file, output, redirection.append);
        } else {
            let _ = create_file(&redirection.file, redirection.append);
        }
    } else if let Ok(output) = result
        && !output.is_empty()
    {
        print!("{}", output);
        // Flush stdout for commands like `clear` that need immediate effect
        if parsed.args.first().is_some_and(|a| a == "clear") {
            let _ = io::stdout().flush();
        }
    }

    // Handle stderr redirection for builtins
    if let Some(ref redirection) = parsed.redirect_stderr {
        let is_external = !BUILTINS.contains(&parsed.args[0].as_str());
        if !is_external {
            if let Err(e) = result {
                let _ = write_to_file(&redirection.file, e, redirection.append);
            } else {
                let _ = create_file(&redirection.file, redirection.append);
            }
        }
    } else if let Err(e) = result {
        eprintln!("{}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stdout_redirect() {
        let tokens = vec!["echo".to_string(), "hi".to_string(), ">".to_string(), "out.txt".to_string()];
        let parsed = parse_command(tokens);
        assert_eq!(parsed.args, vec!["echo", "hi"]);
        assert!(parsed.redirect_stdout.is_some());
        assert_eq!(parsed.redirect_stdout.unwrap().file, "out.txt");
    }

    #[test]
    fn test_parse_stderr_redirect() {
        let tokens = vec!["ls".to_string(), "2>".to_string(), "err.txt".to_string()];
        let parsed = parse_command(tokens);
        assert!(parsed.redirect_stderr.is_some());
    }
}
