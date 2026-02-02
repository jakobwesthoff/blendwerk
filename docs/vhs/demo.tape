# blendwerk demo - single terminal
# Run with: vhs demo.tape

Output demo.webm
Output demo.mp4

Set Shell "zsh"
Set FontFamily "JetBrainsMono NFM SemiBold"
Set FontSize 28
Set Width 1800
Set Height 700
Set Theme { "name": "Blendwerk", "black": "#0f0f12", "red": "#ef6487", "green": "#22c55e", "yellow": "#fdd877", "blue": "#22c55e", "magenta": "#a78bfa", "cyan": "#43c1be", "white": "#e5e5e5", "brightBlack": "#9ca3af", "brightRed": "#ef6487", "brightGreen": "#16a34a", "brightYellow": "#fdd877", "brightBlue": "#16a34a", "brightMagenta": "#a78bfa", "brightCyan": "#43c1be", "brightWhite": "#ffffff", "background": "#0f0f12", "foreground": "#e5e5e5", "selection": "#1c1c22", "cursor": "#22c55e" }
Set Padding 20

# Hidden setup - cd to demo directory and set green prompt
Set TypingSpeed 0
Hide
Type "cd /tmp/blendwerk-demo"
Enter
Sleep 100ms
Type "PROMPT='%F{#22c55e}> %f'"
Enter
Sleep 100ms
Type "clear"
Enter
Sleep 300ms
Show
Set TypingSpeed 35ms

# == VISIBLE RECORDING STARTS ==

Sleep 500ms

# Show the simple directory structure
Type "tree mocks"
Enter
Sleep 2s

# Start blendwerk in background with logging
Type "blendwerk ./mocks --http-only --request-log ./logs > /dev/null 2>&1 &"
Enter
Sleep 2s

# Make requests
Type "curl http://localhost:8080/api/hello"
Enter
Sleep 2s

Type `curl -X POST -d '{"name": "Demo"}' http://localhost:8080/api/hello`
Enter
Sleep 2s

# Show logs
Type "tree logs"
Enter
Sleep 2s

# Show the posted data from the log
Type "jq .request.body logs/api/hello/POST/*.json"
Enter
Sleep 5s
