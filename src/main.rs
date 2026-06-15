use std::io::{self, Write};
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "echo" | "exit" | "type" | "pwd" | "cd")
}

fn find_executable_in_path(command: &str) -> Option<String> {
    // Get PATH environment variable
    if let Ok(path_var) = env::var("PATH") {
        // Split PATH by delimiter (: on Unix, ; on Windows)
        let path_delimiter = if cfg!(windows) { ";" } else { ":" };
        
        for dir in path_var.split(path_delimiter) {
            let full_path = Path::new(dir).join(command);
            
            // Check if file exists
            if full_path.exists() {
                // Check if file has execute permissions
                if is_executable(&full_path) {
                    return Some(full_path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    None
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // Check if any execute bit is set (owner, group, or other)
            (mode & 0o111) != 0
        } else {
            false
        }
    }

    #[cfg(windows)]
    {
        // On Windows, if the file exists and is readable, it's generally considered executable
        // based on file extension, so we just check existence
        path.exists()
    }
}

fn execute_external_program(cmd: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Try to find the executable in PATH
    if let Some(program_path) = find_executable_in_path(cmd) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            
            let mut command = process::Command::new(&program_path);
            command.arg0(cmd);
            
            for arg in args {
                command.arg(arg);
            }
            
            // Replace the current process with the new one (execve)
            // If we want to wait for it, we need to spawn instead
            let mut child = command.spawn()?;
            child.wait()?;
        }
        
        #[cfg(not(unix))]
        {
            let mut child = process::Command::new(&program_path);
            for arg in args {
                child.arg(arg);
            }
            child.spawn()?;
            child.wait()?;
        }
        
        Ok(())
    } else {
        Err(format!("{}: command not found", cmd).into())
    }
}

fn resolve_relative_path(target_dir: &str) -> PathBuf {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let mut path = current_dir.clone();
    
    // Normalize the path components
    for component in target_dir.split('/') {
        match component {
            "" | "." => {
                // Empty string (from leading/trailing/double slashes) or current dir - do nothing
            }
            ".." => {
                // Parent directory
                path.pop();
            }
            _ => {
                // Regular directory name
                path.push(component);
            }
        }
    }
    
    path
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        // Get the HOME environment variable
        if let Ok(home) = env::var("HOME") {
            if path == "~" {
                // Just "~" means the home directory
                home
            } else if path.starts_with("~/") {
                // "~/" followed by a path
                format!("{}{}", home, &path[1..])
            } else {
                // "~user" or similar - not handling this for now, return as-is
                path.to_string()
            }
        } else {
            // HOME not set, return path as-is
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

/// Parse a command line, respecting both single and double quotes, and backslash escaping.
/// Returns a vector of arguments where:
/// - Characters inside single quotes are treated literally (no escaping)
/// - Characters inside double quotes:
///   - Backslash escapes: \", \\, \$, \`, and \<newline>
///   - For other characters, backslash is treated literally
/// - Outside quotes, backslash acts as an escape character: it removes the special meaning
///   of the next character and is itself removed
/// - Adjacent quoted/unquoted strings are concatenated
fn parse_command_with_quotes(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quotes => {
                // Toggle single quote mode (only if not in double quotes)
                in_single_quotes = !in_single_quotes;
            }
            '"' if !in_single_quotes => {
                // Toggle double quote mode (only if not in single quotes)
                in_double_quotes = !in_double_quotes;
            }
            '\\' if in_double_quotes => {
                // Within double quotes, backslash only escapes special characters: \", \\, \$, \`, \<newline>
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '"' | '\\' | '$' | '`' => {
                            // These characters are escapable with backslash - consume the backslash and add the character
                            chars.next(); // consume the next character
                            current_arg.push(next_ch);
                        }
                        '\n' => {
                            // Backslash followed by newline: the backslash and newline are removed (line continuation)
                            chars.next(); // consume the newline
                        }
                        _ => {
                            // For all other characters, backslash is treated literally
                            current_arg.push('\\');
                        }
                    }
                } else {
                    // Backslash at end of string (shouldn't happen in well-formed input)
                    current_arg.push('\\');
                }
            }
            '\\' if !in_single_quotes && !in_double_quotes => {
                // Backslash outside quotes acts as an escape character
                // Consume the next character as a literal
                if let Some(next_ch) = chars.next() {
                    current_arg.push(next_ch);
                }
                // The backslash itself is removed (consumed)
            }
            ' ' | '\t' => {
                if in_single_quotes || in_double_quotes {
                    // Preserve whitespace inside any quotes
                    current_arg.push(ch);
                } else {
                    // Outside quotes, whitespace is a delimiter
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }

    // Don't forget the last argument
    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

fn main() {
    loop {
        // Display the prompt
        print!("$ ");
        io::stdout().flush().unwrap();

        // Read user input
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF reached (Ctrl+D), exit gracefully
                break;
            }
            Ok(_) => {
                // Parse and execute the command
                let command = input.trim();
                if !command.is_empty() {
                    // Parse command with quote support and backslash escaping
                    let parts = parse_command_with_quotes(command);
                    
                    if parts.is_empty() {
                        continue;
                    }
                    
                    let cmd = &parts[0];
                    
                    // Check for exit builtin
                    if cmd == "exit" {
                        process::exit(0);
                    }
                    // Check for echo builtin
                    else if cmd == "echo" {
                        // Get all arguments after "echo"
                        let args = &parts[1..];
                        // Print arguments separated by spaces with newline at end
                        println!("{}", args.join(" "));
                    }
                    // Check for type builtin
                    else if cmd == "type" {
                        if parts.len() < 2 {
                            println!("type: missing argument");
                            continue;
                        }
                        
                        let target_cmd = &parts[1];
                        
                        // First check if it's a builtin
                        if is_builtin(target_cmd) {
                            println!("{} is a shell builtin", target_cmd);
                        } else if let Some(full_path) = find_executable_in_path(target_cmd) {
                            // Then search in PATH
                            println!("{} is {}", target_cmd, full_path);
                        } else {
                            // Not found anywhere
                            println!("{}: not found", target_cmd);
                        }
                    }
                    // Check for pwd builtin
                    else if cmd == "pwd" {
                        // Get the current working directory
                        match env::current_dir() {
                            Ok(path) => {
                                println!("{}", path.display());
                            }
                            Err(e) => {
                                eprintln!("pwd: {}", e);
                            }
                        }
                    }
                    // Check for cd builtin
                    else if cmd == "cd" {
                        if parts.len() < 2 {
                            eprintln!("cd: missing argument");
                            continue;
                        }
                        
                        let target_dir = &parts[1];
                        
                        // Expand tilde if present
                        let expanded_target = expand_tilde(target_dir);
                        
                        // Resolve the target path
                        let path = if expanded_target.starts_with('/') {
                            // Absolute path
                            PathBuf::from(&expanded_target)
                        } else {
                            // Relative path - resolve it
                            resolve_relative_path(&expanded_target)
                        };
                        
                        // Verify that the directory exists and is a directory
                        if path.exists() && path.is_dir() {
                            // Try to change to the directory
                            match env::set_current_dir(&path) {
                                Ok(()) => {
                                    // Successfully changed directory
                                }
                                Err(e) => {
                                    eprintln!("cd: {}: {}", target_dir, e);
                                }
                            }
                        } else {
                            // Directory doesn't exist
                            eprintln!("cd: {}: No such file or directory", target_dir);
                        }
                    }
                    else {
                        // Try to execute as an external program
                        let args = parts[1..].to_vec();
                        match execute_external_program(cmd, &args) {
                            Ok(()) => {
                                // Program executed successfully
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Error reading input, exit
                break;
            }
        }
    }
}
