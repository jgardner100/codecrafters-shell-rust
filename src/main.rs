use std::io::Write;
use std::process;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Mutex;
use rustyline::config::CompletionType;

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

fn find_files_in_current_dir_matching(prefix: &str) -> Vec<(String, bool)> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                // Skip . and .. 
                if file_name == "." || file_name == ".." {
                    continue;
                }
                if file_name.starts_with(prefix) {
                    let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                    files.push((file_name, is_dir));
                }
            }
        }
    }
    
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn find_files_in_path_matching(dir_path: &str, prefix: &str) -> Vec<(String, bool)> {
    let mut files = Vec::new();
    
    let path = Path::new(dir_path);
    if !path.is_dir() {
        return files;
    }
    
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                // Skip . and ..
                if file_name == "." || file_name == ".." {
                    continue;
                }
                if file_name.starts_with(prefix) {
                    let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                    files.push((file_name, is_dir));
                }
            }
        }
    }
    
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "echo" | "exit" | "type" | "pwd" | "cd" | "complete")
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

/// Calculate the longest common prefix for files (without considering is_dir)
fn longest_common_prefix_files(files: &[(String, bool)]) -> String {
    if files.is_empty() {
        return String::new();
    }
    
    let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
    longest_common_prefix(&names)
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
        // Store: (last_line, dir_path, prefix, matches, is_first_tab)
        tab_state: Mutex<Option<(String, String, String, Vec<(String, bool)>, bool)>>,
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
                
                // Determine if we need to search in a directory or current dir
                let (dir_path, prefix, replacement_base) = if let Some(last_slash_pos) = partial.rfind('/') {
                    // If there's a slash, split at the last slash
                    let dir = &partial[..=last_slash_pos];
                    let pre = &partial[last_slash_pos + 1..];
                    (dir, pre, dir)
                } else {
                    // No slash, search in current directory
                    (".", partial, "")
                };
                
                // Find matching files
                let matches = if dir_path == "." {
                    find_files_in_current_dir_matching(prefix)
                } else {
                    find_files_in_path_matching(dir_path, prefix)
                };
                
                if matches.is_empty() {
                    // No matches found
                    *self.tab_state.lock().unwrap() = None;
                    return Ok((pos, vec![]));
                }
                
                // For single match, auto-complete it with trailing character
                if matches.len() == 1 {
                    let (match_name, is_dir) = &matches[0];
                    let suffix = if *is_dir { "/" } else { " " };
                    let completion = format!("{}{}{}", replacement_base, match_name, suffix);

                    *self.tab_state.lock().unwrap() = None;

                    return Ok((
                        last_space_pos + 1,
                        vec![Pair {
                            display: match_name.clone(),
                            replacement: completion,
                        }],
                    ));
                }

                // Multiple matches found - use LCP logic
                let lcp = longest_common_prefix_files(&matches);
                
                let mut state = self.tab_state.lock().unwrap();
                
                // Check if this is the same context as the previous tab
                let is_first_tab = if let Some((last_line, last_dir, last_prefix, _last_matches, was_first)) = state.as_ref() {
                    let same_context = last_line == line && last_dir == dir_path && last_prefix == prefix;
                    if same_context {
                        // Same context, this is not the first tab anymore
                        false
                    } else {
                        // Different context, this is a new first tab
                        true
                    }
                } else {
                    // First time with these matches
                    true
                };

                if is_first_tab {
                    // First TAB: try to complete to LCP
                    
                    // Check if LCP is longer than current prefix
                    if lcp.len() > prefix.len() {
                        // LCP extends beyond current input, complete to LCP
                        let completion = format!("{}{}", replacement_base, lcp);
                        
                        *state = Some((
                            line.to_string(),
                            dir_path.to_string(),
                            prefix.to_string(),
                            matches.clone(),
                            true,
                        ));
                        
                        return Ok((
                            last_space_pos + 1,
                            vec![Pair {
                                display: lcp.clone(),
                                replacement: completion,
                            }],
                        ));
                    } else {
                        // LCP is same as current prefix, ring bell
                        print!("\x07");
                        std::io::stdout().flush().ok();
                        
                        *state = Some((
                            line.to_string(),
                            dir_path.to_string(),
                            prefix.to_string(),
                            matches.clone(),
                            true,
                        ));
                        
                        return Ok((pos, vec![]));
                    }
                } else {
                    // Subsequent TABs: list matches
                    // Format matches with directories showing /
                    let formatted_matches: Vec<String> = matches
                        .iter()
                        .map(|(name, is_dir)| {
                            if *is_dir {
                                format!("{}/", name)
                            } else {
                                name.clone()
                            }
                        })
                        .collect();
                    
                    // Print on a new line with two-space separation
                    let output = formatted_matches.join("  ");
                    // Write directly to stdout to display matches
                    println!();
                    print!("{}", output);
                    println!();
                    print!("$ {}", line);
                    std::io::stdout().flush().ok();
                    
                    // Update state to reflect we're still on subsequent tabs
                    *state = Some((
                        line.to_string(),
                        dir_path.to_string(),
                        prefix.to_string(),
                        matches.clone(),
                        false,
                    ));
                    
                    // Return empty to avoid making any modifications to the input
                    return Ok((pos, vec![]));
                }

            } else {
                // We're completing the command itself (no space in the line)
                if !slice.is_empty() {
                    // 1. Gather all matching executables from PATH
                    let mut matches = find_executables_in_path_matching(slice);

                    // 2. Add matching builtins
                    let builtins = ["echo", "exit", "type", "pwd", "cd", "complete"];
                    for builtin in builtins {
                        if builtin.starts_with(slice) && !matches.contains(&builtin.to_string()) {
                            matches.push(builtin.to_string());
                        }
                    }

                    // Sort the total combined list alphabetically
                    matches.sort();

                    // If there are no matches at all, ring the bell and return empty
                    if matches.is_empty() {
                        *self.tab_state.lock().unwrap() = None;
                        return Ok((pos, vec![]));
                    }

                    // For single matches, complete automatically with a trailing space
                    if matches.len() == 1 {
                        let candidate = Pair {
                            display: matches[0].clone(),
                            replacement: format!("{} ", matches[0]),
                        };
                        
                        *self.tab_state.lock().unwrap() = None;
                        
                        return Ok((0, vec![candidate]));
                    }

                    // For multiple matches: use longest common prefix (LCP) logic
                    let lcp = longest_common_prefix(&matches);

                    // Track the tab execution state for multiple matches
                    let mut state = self.tab_state.lock().unwrap();
                    
                    // Check if we're in the same completion context
                    let is_first_tab = if let Some((last_line, _, _, last_matches, _)) = state.as_ref() {
                        let last_matches_names: Vec<String> = last_matches.iter().map(|(n, _)| n.clone()).collect();
                        !(last_line == line && last_matches_names == matches)
                    } else {
                        true
                    };

                    if is_first_tab {
                        // First tab: complete to LCP if it extends beyond current input
                        *state = Some((line.to_string(), String::new(), String::new(), matches.iter().map(|m| (m.clone(), false)).collect(), true));
                        
                        if lcp.len() > slice.len() {
                            let candidate = Pair {
                                display: lcp.clone(),
                                replacement: lcp,
                            };
                            return Ok((0, vec![candidate]));
                        } else {
                            // LCP is same as current input, ring bell
                            print!("\x07");
                            std::io::stdout().flush().ok();
                            return Ok((pos, vec![]));
                        }
                    } else {
                        // Subsequent tab presses: show all matches
                        let output = matches.join("  ");
                        println!();
                        print!("{}", output);
                        println!();
                        print!("$ {}", line);
                        std::io::stdout().flush().ok();
                        
                        *state = Some((line.to_string(), String::new(), String::new(), matches.iter().map(|m| (m.clone(), false)).collect(), false));
                        
                        // Return empty to avoid making any modifications
                        return Ok((pos, vec![]));
                    }
                }
            }

            Ok((0, vec![]))
        }
    }

    let config = Builder::new()
        .completion_type(CompletionType::List)
        .auto_add_history(true)
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
                } else if cmd == "complete" {
                    // Handle the complete builtin command
                    if parts.len() < 2 {
                        // No arguments provided to complete
                        continue;
                    }
                    
                    // Check if -p flag is provided
                    if parts[1] == "-p" {
                        // -p flag requires a command name
                        if parts.len() < 3 {
                            eprintln!("complete: -p: option requires an argument");
                            continue;
                        }
                        
                        let command_name = &parts[2];
                        // Print the error message for no completion specification
                        eprintln!("complete: {}: no completion specification", command_name);
                    }
                    // For other cases or future flags, we can add more handling here
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
