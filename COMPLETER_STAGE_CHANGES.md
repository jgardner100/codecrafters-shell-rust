# Completer Script Invocation Stage - Implementation Summary

## Overview
This stage implements the ability to invoke registered completer scripts when the user presses TAB to complete a command. The shell now executes registered completer scripts and uses their output to complete user input.

## Key Changes Made to `src/main.rs`

### 1. New Function: `invoke_completer()`
```rust
fn invoke_completer(script_path: &str) -> Option<String>
```

**Purpose**: Executes a registered completer script and returns its output.

**Implementation Details**:
- Uses `std::process::Command` to execute the script via `sh -c`
- Waits for the script to complete before reading output
- Reads stdout from the script
- Returns the first line of output as a `String`
- Returns `None` if the script fails or produces no output

**Key behaviors**:
- Blocks until script completion (ensures full output is captured)
- Extracts only the first line (as per spec: completer always prints exactly one line)
- Handles UTF-8 conversion and error cases gracefully

### 2. Modified: Completer Logic in `ShellHelper::complete()`

**For command completion (when no space in input)**:

#### Before (old behavior):
- Only checked for matching executables and builtins
- Used LCP (Longest Common Prefix) logic for multiple matches

#### After (new behavior):
- **NEW**: First checks if a completer is registered for the partial command
- **NEW**: If registered, invokes the completer script and uses its output
- Falls back to default behavior (executables + builtins) if no completer registered

**Implementation approach**:
```rust
let completer_for_cmd = {
    let completions = COMPLETIONS.lock().unwrap();
    completions.get(partial_cmd).cloned()
};

if let Some(completer_path) = completer_for_cmd {
    if let Some(completion) = invoke_completer(&completer_path) {
        return Ok((
            slice.len(),
            vec![Pair {
                display: completion.clone(),
                replacement: format!("{} ", completion),
            }],
        ));
    }
}
```

**Important details**:
- Scopes the mutex lock to minimize lock duration
- Clones the completer path before releasing the lock
- Adds trailing space to completion (as per spec)
- Returns immediately if completer succeeds

### 3. Completion Flow

When user types command + TAB:
1. Shell checks if completer is registered for that command
2. If yes:
   - Invokes the completer script as a separate process
   - Reads the single line from stdout
   - Uses that line as the completion candidate
   - Appends a trailing space
   - Returns the completion
3. If no:
   - Falls back to default completion (executables + builtins)
   - Uses existing LCP logic for multiple matches

## Usage Example

```bash
# Register a completer for 'docker' command
$ complete -C /path/to/completer_script docker

# Create a completer script that prints "run"
$ cat /path/to/completer_script
#!/usr/bin/env python3
print("run")

# Now when user types and presses TAB:
$ docker <TAB>
$ docker run 
```

## Testing Considerations

The tester verifies:
✅ Shell runs the registered completer script on TAB
✅ Single line output is used as completion
✅ Trailing space is added after completed word
✅ Completer script is executed as a separate process
✅ Shell waits for completer to finish before using output

## Thread Safety

- Uses `lazy_static::lazy_static!` for global COMPLETIONS HashMap
- Uses `Mutex` to protect concurrent access
- Locks are scoped minimally to prevent deadlocks
- Completer path is cloned before lock release

## Error Handling

- If script doesn't execute: returns `None`, falls back to default behavior
- If script fails (non-zero exit): returns `None`
- If output can't be converted to UTF-8: returns `None`
- If no output: returns `None` (empty line from `lines().next()`)

## Performance Impact

- Minimal when no completer registered (same as before)
- Single process spawn per TAB when completer registered
- No busy-waiting; async I/O handled by process completion

## Backward Compatibility

✅ Fully backward compatible
- Commands without registered completers work exactly as before
- Default completion logic unchanged
- Only new feature is invoking registered completers
