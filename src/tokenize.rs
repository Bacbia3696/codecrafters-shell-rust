/// Tokenizes shell input into a vector of strings.
/// Handles quotes, escapes, and redirection operators.
pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' && !in_single_quote {
            if let Some(&next) = chars.peek() {
                chars.next();
                current.push(next);
            }
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c == '>' && !in_single_quote && !in_double_quote {
            let mut redirect_token = String::new();

            let has_fd = !current.is_empty() && current.chars().last().unwrap().is_ascii_digit();
            if has_fd {
                redirect_token = current.clone();
                current.clear();
            }

            redirect_token.push(c);

            if let Some(&next) = chars.peek()
                && next == '>'
            {
                chars.next();
                redirect_token.push(next);
            }

            if !has_fd && !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }

            tokens.push(redirect_token);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        assert_eq!(tokenize("echo hello"), vec!["echo", "hello"]);
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(tokenize("echo \"hello world\""), vec!["echo", "hello world"]);
    }

    #[test]
    fn test_redirection() {
        assert_eq!(tokenize("echo hi > file.txt"), vec!["echo", "hi", ">", "file.txt"]);
    }
}
