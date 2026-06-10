use std::io::{self, Write};
use std::process;
use std::env;
use std::fs;
use std::path::Path;

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

fn execute_external_program(cmd: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    // Try to find the executable in PATH
    if let Some(program_path) = find_executable_in_path(cmd) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            
            let mut command = process::Command::new(&program_path);
            command.arg0(cmd);
            command.args(args);
            
            // Replace the current process with the new one (execve)
            // If we want to wait for it, we need to spawn instead
            let mut child = command.spawn()?;
            child.wait()?;
        }
        
        #[cfg(not(unix))]
        {
            let mut child = process::Command::new(&program_path)
                .args(args)
                .spawn()?;
            child.wait()?;
        }
        
        Ok(())
    } else {
        Err(format!("{}: command not found", cmd).into())
    }
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
                    // Split command into parts
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    
                    if parts.is_empty() {
                        continue;
                    }
                    
                    let cmd = parts[0];
                    
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
                        
                        let target_cmd = parts[1];
                        
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
                        
                        let target_dir = parts[1];
                        
                        // Check if it's an absolute path (starts with /)
                        if target_dir.starts_with('/') {
                            // Verify that the directory exists
                            let path = Path::new(target_dir);
                            
                            if path.exists() && path.is_dir() {
                                // Try to change to the directory
                                match env::set_current_dir(path) {
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
                        } else {
                            // For now, only handle absolute paths
                            eprintln!("cd: {}: No such file or directory", target_dir);
                        }
                    }
                    else {
                        // Try to execute as an external program
                        let args = &parts[1..];
                        match execute_external_program(cmd, args) {
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
