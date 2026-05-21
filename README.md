# Just Input

Just Input is a tiny Windows-first utility for typing text into a focused window when clipboard paste is unavailable.

It does not copy anything to the clipboard. On Windows it sends keyboard input with `SendInput`.

## Usage

Run without arguments to open the GUI:

```powershell
just-input.exe
```

Release builds start as a Windows GUI app and do not open a console window.

Paste or type text into the app, press `Type`, focus the target window before the timer ends, and Just Input will type the text.

Shortcuts and controls:

- `Ctrl+Enter` starts the default timer.
- `Cancel` stops a pending timer before typing starts.
- `Arm Ctrl+Alt+J` enables a global hotkey while the app is open.

CLI mode:

```powershell
just-input.exe --text "hello" --delay 3
just-input.exe --file input.txt --delay 3
```

## Build

```powershell
cargo test
cargo build --release
```
