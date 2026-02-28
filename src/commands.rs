use std::env;

/// List of builtin commands
pub const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd"];

/// Executes a builtin command and returns the output or error.
pub fn execute_builtin(cmd: &str, args: &[String]) -> Result<String, String> {
    match cmd {
        "pwd" => env::current_dir()
            .map(|p| format!("{}\n", p.display()))
            .map_err(|e| format!("Error getting current directory: {}", e)),
        "cd" => execute_cd(args),
        "type" => execute_type(args),
        "echo" => Ok(args[1..].join(" ") + "\n"),
        _ => Err(format!("{}: command not found", cmd)),
    }
}

fn execute_cd(args: &[String]) -> Result<String, String> {
    let target = args.get(1).map_or_else(
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
    match target {
        Some(dir) => env::set_current_dir(&dir)
            .map(|_| String::new())
            .map_err(|_| format!("cd: {}: No such file or directory", dir)),
        None => Err("cd: HOME not set".to_string()),
    }
}

fn execute_type(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Ok("type: missing argument\n".to_string());
    }

    let arg = &args[1];
    if BUILTINS.contains(&arg.as_str()) {
        Ok(format!("{} is a shell builtin\n", arg))
    } else {
        match full_path(arg) {
            Some(path) => Ok(format!("{} is {}\n", arg, path)),
            None => Ok(format!("{}: not found\n", arg)),
        }
    }
}

/// Finds the full path of a command by searching PATH.
pub fn full_path(command: &str) -> Option<String> {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    env::var("PATH").ok()?.split(':').find_map(|path| {
        let full = format!("{}/{}", path, command);
        std::fs::metadata(&full)
            .ok()
            .filter(|m| {
                m.is_file() && {
                    #[cfg(unix)]
                    {
                        m.permissions().mode() & 0o111 != 0
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                }
            })?;
        Some(full)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo() {
        let args = vec!["echo".to_string(), "hello".to_string(), "world".to_string()];
        assert_eq!(execute_builtin("echo", &args), Ok("hello world\n".to_string()));
    }

    #[test]
    fn test_type_builtin() {
        let args = vec!["type".to_string(), "echo".to_string()];
        assert!(execute_builtin("type", &args).unwrap().contains("builtin"));
    }
}
