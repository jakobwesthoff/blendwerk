# Demo Video Recording

This folder contains [VHS](https://github.com/charmbracelet/vhs) tape files for recording the blendwerk demo video.

## Prerequisites

- [VHS](https://github.com/charmbracelet/vhs) installed (`brew install vhs` or `go install github.com/charmbracelet/vhs@latest`)
- [blendwerk](../../) installed and available in PATH
- `tree` and `jq` commands available

## Recording

Run the recording script from this directory:

```bash
./record.sh
```

This will:
1. Create a temporary demo environment in `/tmp/blendwerk-demo`
2. Set up mock files for the demo
3. Record the demo using VHS
4. Move the output videos to `../pages/assets/`
5. Clean up the temporary directory

## Output

The recording produces:
- `demo.webm` - WebM format (smaller, modern browsers)
- `demo.mp4` - MP4 format (broader compatibility)

Both files are moved to `docs/pages/assets/` for use on the landing page.

## Customizing

Edit `demo.tape` to modify the demo content. See [VHS documentation](https://github.com/charmbracelet/vhs) for available commands.

The theme colors match the blendwerk landing page (grass green accent on dark background).
