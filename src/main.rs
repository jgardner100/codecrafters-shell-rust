use std::io::{self, Write};
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Mutex;

fn find_executables_in_path_matching(prefix: &str) -> Vec<String> {
    let mut executables = HashSet::new();
    
    if let Ok(path_var) = env::var("PATH") {
        let path_delimiter = if cfg!(windows) { ";" } else { ":" };
        
        for dir in path_var.split(path_delimiter) {
            if dir.is_empty() {
                continue;
            }
            
            let path = Path::new(dir);
            
            if !path.is_dir() {
                continue;
            }
            
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.starts_with(prefix) {
                            if let Ok(metadata) = entry.metadata() {
                                if metadata.is_file() && is_executable(&entry.path()) {
                                    executables.insert(file_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut result: Vec<String> = executables.into_iter().collect();
    result.sort();
    result
}

fn find_files_in_current_dir_matching(prefix: &str) -> Vec<String> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.starts_with(prefix) {
                    files.push(file_name);
                }
            }
        }
    }
    
    files.sort();
    files
}

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "echo" | "exit" | "type" | "pwd" | "cd")
}

fn find_executable_in_path(command: &str) -> Option<String> {
    if let Ok(path_var) = env::var("PATH") {
        let path_delimiter = if cfg!(windows) { ";" } else { ":" };
        for dir in path_var.split(path_delimiter) {
            let full_path = Path::new(dir).join(command);
            if full_path.exists() && is_executable(&full_path) {
                return Some(full_path.to_string_lossy().to_string());
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
            let mode = metadata.permissions().mode();
            (mode & 0o111) != 0
        } else {
            false
        }
    }
    #[cfg(windows)]
    {
        path.exists()
    }
}

/// Calculate the longest common prefix (LCP) of all strings in the list
fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    
    if strings.len() == 1 {
        return strings[0].clone();
    }
    
    let mut lcp = String::new();
    let min_len = strings.iter().map(|s| s.len()).min().unwrap_or(0);
    
    for i in 0..min_len {
        let ch = strings[0].chars().nth(i).unwrap();
        if strings.iter().all(|s| s.chars().nth(i) == Some(ch)) {
            lcp.push(ch);
        } else {
            break;
        }
    }
    
    lcp
}

#[derive(Debug, Clone)]
struct Redirection {
    stdout_target: Option<(String, bool)>,
    stderr_target: Option<(String, bool)>,
}

fn parse_with_redirection(input: &str) -> (Vec<String>, Redirection) {
    let tokens = parse_command_with_quotes(input);
    let mut command_parts = Vec::new();
    let mut redirection = Redirection {
        stdout_target: None,
        stderr_target: None,
    };
    let mut i = 0;
    
    while i < tokens.len() {
        let token = &tokens[i];
        if token == ">>" || token == "1>>" {
            if i + 1 < tokens.len() {
                redirection.stdout_target = Some((tokens[i + 1].clone(), true));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == ">" || token == "1>" {
            if i + 1 < tokens.len() {
                redirection.stdout_target = Some((tokens[i + 1].clone(), false));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == "2>>" {
            if i + 1 < tokens.len() {
                redirection.stderr_target = Some((tokens[i + 1].clone(), true));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else if token == "2>" {
            if i + 1 < tokens.len() {
                redirection.stderr_target = Some((tokens[i + 1].clone(), false));
                i += 2;
            } else {
                command_parts.push(token.clone());
                i += 1;
            }
        } else {
            command_parts.push(token.clone());
            i += 1;
        }
    }
    (command_parts, redirection)
}

fn execute_external_program(cmd: &str, args: &[String], redirection: Redirection) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(program_path) = find_executable_in_path(cmd) {
        let mut command = process::Command::new(&program_path);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.arg0(cmd);
        }
        for arg in args {
            command.arg(arg);
        }
        if let Some((filename, is_append)) = &redirection.stdout_target {
            let file = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename)?;
            command.stdout(file);
        }
        if let Some((filename, is_append)) = &redirection.stderr_target {
            let file = fs::OpenOptions::new().create(true).append(*is_append).write(!is_append).truncate(!is_append).open(filename)?;
            command.stderr(file);
        }
        let mut child = command.spawn()?;
        child.wait()?;
        Ok(())
    } else {
        Err(format!("{}: command not found", cmd).into())
    }
}

fn resolve_relative_path(target_dir: &str) -> PathBuf {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let mut path = current_dir.clone();
    for component in target_dir.split('/') {
        match component {
            "" | "." => {}
            ".." => { path.pop(); }
            _ => { path.push(component); }
        }
    }
    path
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Ok(home) = env::var("HOME") {
            if path == "~" { home }
            else if path.starts_with("~/") { format!("{}{}", home, &path[1..]) }
            else { path.to_string() }
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

fn parse_command_with_quotes(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quotes => { in_single_quotes = !in_single_quotes; }
            '"' if !in_single_quotes => { in_double_quotes = !in_double_quotes; }
            '\\' if in_double_quotes => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '"' | '\\' | '$' | '`' => {
                            chars.next();
                            current_arg.push(next_ch);
                        }
                        '\n' => { chars.next(); }
                        _ => { current_arg.push('\\'); }
                    }
                } else {
                    current_arg.push('\\');
                }
            }
            '\\' if !in_single_quotes && !in_double_quotes => {
                if let Some(next_ch) = chars.next() {
                    current_arg.push(next_ch);
                }
            }
            ' ' | '\t' => {
                if in_single_quotes || in_double_quotes {
                    current_arg.push(ch);
                } else if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => { current_arg.push(ch); }
        }
    }
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    args
}

fn main() {
    use rustyline::Editor;
    use rustyline::error::ReadlineError;
    use rustyline::config::Builder;
    use rustyline::completion::{Completer, Pair};
    use rustyline::hint::Hinter;
    use rustyline::highlight::Highlighter;
    use rustyline::validate::Validator;
    use rustyline::{Context, Helper};

    struct ShellHelper {
        tab_state: Mutex<Option<(String, usize, Vec<String>)>>,
    }

    impl Helper for ShellHelper {}
    impl Hinter for ShellHelper {
        type Hint = String;
    }
    impl Highlighter for ShellHelper {}
    impl Validator for ShellHelper {}

    impl Completer for ShellHelper {
        type Candidate = Pair;

        fn complete(
            &self,
            line: &str,
            pos: usize,
            _ctx: &Context<'_>,
        ) -> rustyline::Result<(usize, Vec<Pair>)> {
            let slice = &line[..pos];

            // Check if there's a space in the line (indicating we're completing an argument)
            if let Some(last_space_pos) = slice.rfind(' ') {
                // We're completing an argument (not the command)
                // Extract the partial filename/argument after the last space
                let partial = &slice[last_space_pos + 1..];
                
                // Find matching files in the current directory
                let matches = find_files_in_current_dir_matching(partial);
                
                if matches.is_empty() {
                    // No matches found, ring the bell
                    print!("\x07");
                    io::stdout().flush().ok();
                    return Ok((pos, vec![]));
                }
                
                // For single match, complete with trailing space
                if matches.len() == 1 {
                    let candidate = Pair {
                        display: matches[0].clone(),
                        replacement: format!("{} ", matches[0]),
                    };
                    return Ok((pos - partial.len(), vec![candidate]));
                }
                
                // For multiple matches, use longest common prefix logic
                let lcp = longest_common_prefix(&matches);
                
                // Track the tab execution state for multiple matches
                let mut state = self.tab_state.lock().unwrap();
                let (last_prefix, count, last_matches) = state.take().unwrap_or((String::new(), 0, vec![]));
                
                if last_prefix == partial && last_matches == matches {
                    // User pressed tab again on the same input
                    let new_count = count + 1;
                    if new_count >= 2 {
                        // On the second tab press: print matches on a new line separated by spaces
                        println!();
                        println!("{}", matches.join("  "));
                        
                        *state = None; // Reset the cycle
                        
                        // Pass a dummy candidate that matches exactly what the user typed
                        let candidate = Pair {
                            display: partial.to_string(),
                            replacement: partial.to_string(),
                        };
                        return Ok((pos - partial.len(), vec![candidate]));
                    } else {
                        print!("\x07");
                        io::stdout().flush().ok();
                        *state = Some((partial.to_string(), new_count, matches.clone()));
                        return Ok((pos, vec![]));
                    }
                } else {
                    // First tab press or new input: complete to LCP
                    *state = Some((partial.to_string(), 1, matches.clone()));
                    
                    // If LCP is longer than the current partial input, complete to it
                    if lcp.len() > partial.len() {
                        let candidate = Pair {
                            display: lcp.clone(),
                            replacement: lcp,
                        };
                        return Ok((pos - partial.len(), vec![candidate]));
                    } else {
                        // LCP is same as current input, ring bell
                        print!("\x07");
                        io::stdout().flush().ok();
                        return Ok((pos, vec![]));
                    }
                }
            } else {
                // We're completing the command itself (no space in the line)
                if !slice.is_empty() {
                    // 1. Gather all matching executables from PATH
                    let mut matches = find_executables_in_path_matching(slice);

                    // 2. Add matching builtins
                    let builtins = ["echo", "exit", "type", "pwd", "cd"];
                    for builtin in builtins {
                        if builtin.starts_with(slice) && !matches.contains(&builtin.to_string()) {
                            matches.push(builtin.to_string());
                        }
                    }

                    // Sort the total combined list alphabetically
                    matches.sort();

                    // If there are no matches at all, ring the bell and return empty
                    if matches.is_empty() {
                        print!("\x07");
                        io::stdout().flush().ok();
                        return Ok((pos, vec![]));
                    }

                    // For single matches, complete automatically with a trailing space
                    if matches.len() == 1 {
                        let candidate = Pair {
                            display: matches[0].clone(),
                            replacement: format!("{} ", matches[0]),
                        };
                        return Ok((0, vec![candidate]));
                    }

                    // For multiple matches: use longest common prefix (LCP) logic
                    let lcp = longest_common_prefix(&matches);

                    // Track the tab execution state for multiple matches
                    let mut state = self.tab_state.lock().unwrap();
                    let (last_prefix, count, last_matches) = state.take().unwrap_or((String::new(), 0, vec![]));

                    if last_prefix == slice && last_matches == matches {
                        // User pressed tab again on the same input
                        let new_count = count + 1;
                        if new_count >= 2 {
                            // On the second tab press: print matches on a new line separated by spaces
                            println!();
                            println!("{}", matches.join("  "));
                            
                            *state = None; // Reset the cycle
                            
                            // Pass a dummy candidate that matches exactly what the user typed.
                            let candidate = Pair {
                                display: slice.to_string(),
                                replacement: slice.to_string(),
                            };
                            return Ok((0, vec![candidate]));
                        } else {
                            print!("\x07");
                            io::stdout().flush().ok();
                            *state = Some((slice.to_string(), new_count, matches.clone()));
                            return Ok((pos, vec![]));
                        }
                    } else {
                        // First tab press or new input: complete to LCP
                        *state = Some((slice.to_string(), 1, matches.clone()));
                        
                        // If LCP is longer than the current input, complete to it
                        if lcp.len() > slice.len() {
                            let candidate = Pair {
                                display: lcp.clone(),
                                replacement: lcp,
                            };
                            return Ok((0, vec![candidate]));
                        } else {
                            // LCP is same as current input, ring bell
                            print!("\x07");
                            io::stdout().flush().ok();
                            return Ok((pos, vec![]));
                        }
                    }
                }
            }

            Ok((0, vec![]))
        }
    }

    let config = Builder::new()
        .auto_add_history(true)
        .bell_style(rustyline::config::BellStyle::Visible)
        .build();

    let mut rl = Editor::<ShellHelper, _>::with_config(config).unwrap();
    rl.set_helper(Some(ShellHelper {
        tab_state: Mutex::new(None),
    }));

    loop {
        let readline = rl.readline("$ ");
        match readline {
            Ok(line) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }

                let (parts, redirection) = parse_with_redirection(command);
                if parts.is_empty() {
                    continue;
                }

                let cmd = &parts[0];

                if cmd == "exit" {
                    process::exit(0);
                } else if cmd == "echo" {
                    let args = &parts[1..];
                    let output = args.join(" ");

                    if let Some((stderr_filename, is_append)) = &redirection.stderr_target {
                        let _ = fs::OpenOptions::new()
                            .create(true)
                            .append(*is_append)
                            .write(!is_append)
                            .truncate(!is_append)
                            .open(stderr_filename);
                    }

                    if let Some((filename, is_append)) = &redirection.stdout_target {
                        let result = fs::OpenOptions::new()
                            .create(true)
                            .append(*is_append)
                            .write(!is_append)
                            .truncate(!is_append)
                            .open(filename);
                        match result {
                            Ok(mut file) => {
                                let _ = writeln!(file, "{}", output);
                            }
                            Err(e) => {
                                eprintln!("echo: {}: {}", filename, e);
                            }
                        }
                    } else {
                        println!("{}", output);
                    }
                } else if cmd == "type" {
                    if parts.len() < 2 {
                        println!("type: missing argument");
                        continue;
                    }
                    let target_cmd = &parts[1];
                    if is_builtin(target_cmd) {
                        println!("{} is a shell builtin", target_cmd);
                    } else if let Some(full_path) = find_executable_in_path(target_cmd) {
                        println!("{} is {}", target_cmd, full_path);
                    } else {
                        println!("{}: not found", target_cmd);
                    }
                } else if cmd == "pwd" {
                    match env::current_dir() {
                        Ok(path) => {
                            let output = format!("{}", path.display());
                            if let Some((stderr_filename, is_append)) = &redirection.stderr_target {
                                let _ = fs::OpenOptions::new()
                                    .create(true)
                                    .append(*is_append)
                                    .write(!is_append)
                                    .truncate(!is_append)
                                    .open(stderr_filename);
                            }
                            if let Some((filename, is_append)) = &redirection.stdout_target {
                                let result = fs::OpenOptions::new()
                                    .create(true)
                                    .append(*is_append)
                                    .write(!is_append)
                                    .truncate(!is_append)
                                    .open(filename);
                                match result {
                                    Ok(mut file) => {
                                        let _ = writeln!(file, "{}", output);
                                    }
                                    Err(e) => {
                                        eprintln!("pwd: {}: {}", filename, e);
                                    }
                                }
                            } else {
                                println!("{}", output);
                            }
                        }
                        Err(e) => {
                            eprintln!("pwd: {}", e);
                        }
                    }
                } else if cmd == "cd" {
                    if parts.len() < 2 {
                        eprintln!("cd: missing argument");
                        continue;
                    }
                    let target_dir = &parts[1];
                    let expanded_target = expand_tilde(target_dir);
                    let path = if expanded_target.starts_with('/') {
                        PathBuf::from(&expanded_target)
                    } else {
                        resolve_relative_path(&expanded_target)
                    };

                    if path.exists() && path.is_dir() {
                        if let Err(e) = env::set_current_dir(&path) {
                            eprintln!("cd: {}: {}", target_dir, e);
                        }
                    } else {
                        eprintln!("cd: {}: No such file or directory", target_dir);
                    }
                } else {
                    let args = parts[1..].to_vec();
                    if let Err(e) = execute_external_program(cmd, &args, redirection) {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}
