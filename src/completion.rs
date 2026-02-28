use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use std::env;

/// Shell completer for tab completion.
pub struct ShellCompleter {
    builtins: Vec<String>,
    filename_completer: FilenameCompleter,
}

impl ShellCompleter {
    pub fn new(builtins: Vec<String>) -> Self {
        Self {
            builtins,
            filename_completer: FilenameCompleter::new(),
        }
    }
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
        let is_first_word = line[..pos].split_whitespace().count() <= 1;

        if is_first_word {
            let mut candidates = Vec::new();

            // Complete builtins
            self.builtins
                .iter()
                .filter(|b| b.starts_with(&word))
                .for_each(|builtin| {
                    candidates.push(Pair {
                        display: builtin.clone(),
                        replacement: format!("{} ", builtin),
                    });
                });

            // Complete PATH binaries
            if let Ok(path) = env::var("PATH") {
                for dir in path.split(':') {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        entries
                            .flatten()
                            .filter_map(|e| e.file_name().into_string().ok())
                            .filter(|name| name.starts_with(&word))
                            .for_each(|name| {
                                candidates.push(Pair {
                                    display: name.clone(),
                                    replacement: format!("{} ", name),
                                });
                            });
                    }
                }
            }

            candidates.sort_by(|a, b| a.display.cmp(&b.display));
            candidates.dedup_by(|a, b| a.display == b.display);
            Ok((start, candidates))
        } else {
            self.filename_completer.complete(line, pos, ctx)
        }
    }
}

fn extract_word(line: &str, pos: usize) -> (usize, String) {
    let before = &line[..pos];
    let start = before.rfind(|c: char| c.is_whitespace()).map_or(0, |i| i + 1);
    (start, line[start..pos].to_string())
}

impl Helper for ShellCompleter {}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Highlighter for ShellCompleter {}
impl Validator for ShellCompleter {}
