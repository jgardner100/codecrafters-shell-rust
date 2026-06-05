use std::io::{self, Write};
use std::process;

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
                    else {
                        // Command not found
                        println!("{}: command not found", cmd);
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
