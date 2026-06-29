# What to Look For - Modified Files Guide

## File 1: Cargo.toml

### What Changed
Look for the new dependency at the end of the `[dependencies]` section:

```toml
lazy_static = "1.4"                              # global mutable state
```

### Why It's There
- `lazy_static` provides thread-safe global mutable state
- Used to create a global HashMap for storing completions
- Ensures HashMap is initialized only once at runtime

---

## File 2: src/main.rs

### Change 1: Imports (Near Top, Line ~6)

**Look for:**
```rust
use std::collections::{HashSet, HashMap};
```

**What changed:**
- Added `HashMap` to the existing `HashSet` import
- Before: `use std::collections::HashSet;`
- After: `use std::collections::{HashSet, HashMap};`

### Change 2: Global State (Lines ~10-13)

**Look for:**
```rust
// Global storage for registered completions
lazy_static::lazy_static! {
    static ref COMPLETIONS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}
```

**What this is:**
- Creates a thread-safe global HashMap
- Maps command names → completer paths
- Accessible from anywhere in the program
- Persists for the lifetime of the shell session

### Change 3: Complete Command Handler (Lines ~612-662)

**Look for TWO major sections:**

#### Section A: Registration Handler
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

**What this does:**
- Handles `complete -C` command
- Validates arguments (needs exactly 4: complete, -C, path, command)
- Stores the mapping in global HashMap
- Silent success (no output)

#### Section B: Enhanced Display Handler
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

**What this does:**
- Handles `complete -p` command
- Validates arguments (needs exactly 3: complete, -p, command)
- Retrieves completer path from HashMap
- Outputs in normalized format with single quotes and spaces
- Returns error if not registered

---

## Quick Verification Checklist

### In Cargo.toml
- [ ] Look for `lazy_static` in dependencies
- [ ] Version is `"1.4"` or compatible

### In src/main.rs

#### Imports Section
- [ ] HashMap is imported from std::collections
- [ ] Import line shows: `{HashSet, HashMap}`

#### Top of Main File (Before main() function)
- [ ] `lazy_static::lazy_static!` block exists
- [ ] Contains: `static ref COMPLETIONS`
- [ ] Type: `Mutex<HashMap<String, String>>`

#### Complete Command Handler
- [ ] `if parts[1] == "-C"` block exists
- [ ] Validates `parts.len() < 4`
- [ ] Extracts `parts[2]` and `parts[3]`
- [ ] Uses `COMPLETIONS.lock().unwrap().insert()`

- [ ] `else if parts[1] == "-p"` block exists
- [ ] Validates `parts.len() < 3`
- [ ] Uses `completions.get(command_name)`
- [ ] Prints with single quotes: `'{}' {}`

---

## Expected Behaviors

### Test 1: Registration
```bash
$ complete -C /usr/bin/git git
```
Expected: No output, no error

### Test 2: Display
```bash
$ complete -p git
```
Expected: `complete -C '/usr/bin/git' git`

### Test 3: Normalization
```bash
$ complete   -C   /path   cmd
$ complete -p cmd
```
Expected: `complete -C '/path' cmd` (single spaces)

### Test 4: Unregistered
```bash
$ complete -p unknown
```
Expected: Error to stderr: `complete: unknown: no completion specification`

---

## Code Patterns to Look For

### Pattern 1: Mutex Lock-Unlock
```rust
let mut completions = COMPLETIONS.lock().unwrap();  // Lock
completions.insert(...)                              // Use
// Lock auto-releases when variable goes out of scope
```

### Pattern 2: HashMap Insert
```rust
completions.insert(command_name.clone(), completer_path.clone());
```

### Pattern 3: HashMap Get
```rust
if let Some(completer_path) = completions.get(command_name) {
    // Found: use completer_path
} else {
    // Not found: error
}
```

### Pattern 4: Output Format
```rust
println!("complete -C '{}' {}", completer_path, command_name);
//        Single quotes    ^   ^  Space between path and command
```

---

## How the Flow Works

```
User enters: "complete -C /path/to/script git"
                          ↓
              parse_command_with_quotes()
                          ↓
    parts = ["complete", "-C", "/path/to/script", "git"]
                          ↓
        if cmd == "complete" && parts[1] == "-C"
                          ↓
        completer_path = parts[2]  // "/path/to/script"
        command_name = parts[3]     // "git"
                          ↓
        COMPLETIONS.lock().insert("git", "/path/to/script")
                          ↓
                    (no output)


User enters: "complete -p git"
                          ↓
              parse_command_with_quotes()
                          ↓
        parts = ["complete", "-p", "git"]
                          ↓
        if cmd == "complete" && parts[1] == "-p"
                          ↓
        command_name = parts[2]  // "git"
                          ↓
        completer_path = COMPLETIONS.lock().get("git")
                          ↓
                    (if found)
                          ↓
    println!("complete -C '{}' {}", "/path/to/script", "git")
                          ↓
        "complete -C '/path/to/script' git"
```

---

## Summary

### Key Changes at a Glance
1. **Cargo.toml**: +1 dependency (lazy_static)
2. **src/main.rs imports**: +HashMap to collections import
3. **src/main.rs global**: +4 lines (lazy_static block)
4. **src/main.rs handler**: +25 lines (registration + display logic)

### Most Important Code
- Global state: `static ref COMPLETIONS: Mutex<HashMap<...>>`
- Registration: `completions.insert(command_name, completer_path)`
- Display: `println!("complete -C '{}' {}", path, command)`

### What Not to Miss
- Single quotes around path in output
- Single space between parts
- Use of `clone()` for String values
- Mutex lock/unlock pattern
- Error messages format
