# 🌫️ Blur

A cross-platform Rust CLI tool designed to redact (blur) sensitive information from your terminal screen before taking screenshots or sharing your screen. 

---

### 📥 [Download Latest Version](https://github.com/mateussm/blur/releases/latest)
Get the pre-compiled binary for your system:
- **[🐧 Linux](https://github.com/mateussm/blur/releases/latest/download/blur-linux)**
- **[🪟 Windows](https://github.com/mateussm/blur/releases/latest/download/blur-windows.exe)**
- **[🍏 MacOS](https://github.com/mateussm/blur/releases/latest/download/blur-macos)**

---

Unlike simple filters, **Blur** can "recreate" your terminal buffer, preserving your original colors and layout while randomizing specific text ranges or regex patterns.

## ✨ Features

- **Terminal Recreation**: Captures your current screen and redraws it with blurs applied.
- **ANSI Color Preservation**: Keeps your terminal colors and styles intact.
- **TMUX Integration**: Automatically captures the active pane if running inside `tmux`.
- **Clipboard Fallback**: If not in tmux, simply copy your terminal text to the clipboard and Blur will process it.
- **Flexible Blurring**: Supports both coordinate-based ranges (`row:col..row:col`) and Regex patterns.
- **Space Preservation**: Keep your alignment and layout perfect with the `--preserve-spaces` flag.

## 🚀 Installation

Ensure you have [Rust](https://rustup.rs/) installed.

```bash
cargo build --release
# The binary will be available at target/release/blur
```

## 📖 Usage

### 1. Basic Redaction (Pipe Mode)
Works like a standard Unix filter:
```bash
cat secrets.txt | blur 1:1..1:20
```

### 2. Screen Recreation (Direct Mode)
Captures your current terminal (via tmux or clipboard) and redraws it:
```bash
# Blur specific ranges
blur 1:33..1:73 4:11..4:27 6:6..6:8

# Blur and preserve spaces (layout)
blur -s 1:33..1:73

# Blur using Regex
blur "password: .*" "[A-Z0-9]{20}"

# Hide the line that called the blur command
blur --hide-cmd 1:1..2:20
```

### 3. Coordinate System
- **Rows/Cols**: 1-indexed. `1:1` is the top-left corner.
- **Spans**: `1:1..2:10` blurs from the start of line 1 to the 10th character of line 2.

## 🛠️ Options

| Flag | Short | Description |
|------|-------|-------------|
| `--preserve-spaces` | `-s` | Do not randomize space characters (keeps layout aligned). |
| `--hide-cmd` | | Removes the `blur ...` command line from the final output. |
| `--help` | `-h` | Show all available options. |

## 💡 Pro Tip: How to use without TMUX
If you aren't using `tmux`, you can still use the screen recreation feature:
1. Select and **Copy** the text in your terminal.
2. Run `blur <ranges>`.
3. The tool will read your clipboard, apply the blur, and print the result!

## 🧪 Testing
The project includes a suite of unit tests for the blurring engine:
```bash
cargo test
```
