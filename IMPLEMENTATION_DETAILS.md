# Complete Implementation Details

## File Changes

### 1. `Cargo.toml`
**Added dependency:**
```toml
lazy_static = "1.4"
```

### 2. `src/main.rs`

#### Imports (Lines 1-13)
```rust
use std::collections::{HashSet, HashMap};  // ← Added HashMap
use std::sync::Mutex;
use rustyline::config::CompletionType;

// Global storage for registered completions
lazy_static::lazy_static! {
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}
```

#### Complete Command Handler (Lines 612-662)
The handler now supports two operations:

**A. Register Completion with `-C` flag:**
```rust
if parts[1] == "-C" {
    // -C flag requires at least: complete -C <path> <command>
    if parts.len() < 4 {
        eprintln!("complete: -C: option requires an argument");
        continue;
    }
    
    let completer_path = &parts[2];
    let command_name = &parts[3];
    
    // Register the completion
    let mut completions = COMPLETIONS.lock().unwrap();
    completions.insert(command_name.clone(), completer_path.clone());
    
    // The -C flag produces no output on success
}
```

**B. Display Completion with `-p` flag:**
```rust
else if parts[1] == "-p" {
    // -p flag requires a command name
    if parts.len() < 3 {
        eprintln!("complete: -p: option requires an argument");
        continue;
    }
    
    let command_name = &parts[2];
    let completions = COMPLETIONS.lock().unwrap();
    
    if let Some(completer_path) = completions.get(command_name) {
        // Print in normalized format: complete -C '<path>' <command>
        println!("complete -C '{}' {}", completer_path, command_name);
    } else {
        // Print error message for no completion specification
        eprintln!("complete: {}: no completion specification", command_name);
    }
}
```

## Behavior Summary

| Operation | Input | Output |
|-----------|-------|--------|
| Register | `complete -C /path/to/completer git` | (no output, silent success) |
| Register | `complete   -C   /path/to/completer   git` | (no output, handles extra whitespace) |
| Display | `complete -p git` | `complete -C '/path/to/completer' git` |
| Display | `complete -p unknown` | `complete: unknown: no completion specification` (stderr) |

## Key Features

✅ **Global State**: Uses `lazy_static` for thread-safe global HashMap
✅ **Registration**: `-C flag` stores completer scripts by command name
✅ **Display**: `-p flag` retrieves and prints in normalized format
✅ **Formatting**: Always outputs as `complete -C '<path>' <command>`
✅ **Quotes**: Single quotes around path literal
✅ **Spacing**: Exactly one space between parts
✅ **Error Handling**: Proper error messages for missing specifications
✅ **Whitespace**: Automatically normalizes input whitespace

## Testing Scenarios

```bash
# Test 1: Basic registration
$ complete -C /usr/bin/git_completer git
# (no output - success)

# Test 2: Display registered completion
$ complete -p git
complete -C '/usr/bin/git_completer' git

# Test 3: Extra whitespace handling
$ complete    -C    /usr/bin/docker    docker
# (no output - success)

# Test 4: Display with normalized formatting
$ complete -p docker
complete -C '/usr/bin/docker' docker

# Test 5: Query unregistered command
$ complete -p unknown
complete: unknown: no completion specification
```

## Design Decisions

1. **Lazy Static HashMap**: Ensures single initialization and thread-safe access
2. **Mutex Lock**: Protects HashMap during read/write operations
3. **String Storage**: Both path and command name stored as plain strings
4. **Normalized Output**: Always formats output consistently regardless of input
5. **No Echo**: Doesn't repeat the input command, only stores and retrieves
