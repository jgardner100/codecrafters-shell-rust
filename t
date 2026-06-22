Initiating test run...

Welcome back! Your first build could take slightly longer, please bear with us.
Subsequent ones will be snappy ⚡

Running tests on your code. Logs should appear shortly...

[compile]    Compiling codecrafters-shell v0.1.0 (/app)
[compile]     Finished `release` profile [optimized] target(s) in 2.05s
[compile] Moved ./.codecrafters/run.sh → ./your_program.sh
[compile] Compilation successful.

[tester::#ZV2] Running tests for Stage #ZV2 (Filename Completion - File completion)
[tester::#ZV2] [working_dir] - grape-83.txt
[tester::#ZV2] Running ./your_program.sh
[tester::#ZV2] ✓ Received prompt ($ )
[tester::#ZV2] Typed 'stat grape-'
[tester::#ZV2] ✓ Prompt line matches '$ stat grape-'
[tester::#ZV2] Pressed '<TAB>' (expecting autocomplete to 'stat grape-83.txt' followed by a space)
[your-program] $ grape-83.txt
[tester::#ZV2] ^ Line does not match expected value.
[tester::#ZV2] Expected: '$ stat grape-83.txt '
[tester::#ZV2] Received: '$ grape-83.txt '
[tester::#ZV2] Test failed

View our article on debugging test failures: https://codecrafters.io/debug
