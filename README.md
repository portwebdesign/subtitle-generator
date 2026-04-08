<div align="center">

# 🎬 Subtitle Generator

**A fast, lightweight MP4 subtitle generator built with Rust**

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-lightblue?logo=windows)](https://github.com/portwebdesign/subtitle-generator/releases)
[![Powered by whisper.cpp](https://img.shields.io/badge/Powered%20by-whisper.cpp-green)](https://github.com/ggerganov/whisper.cpp)

Generate `.srt` subtitle files from MP4 videos in seconds.  
No Python. No runtime. No 300 MB bloat. Just a **4 MB Rust binary**.

</div>

---

## ✨ Why Rust?

Most subtitle tools are built with Python + PyInstaller. They come with problems:

| Issue | Python EXE | **This App (Rust)** |
|-------|-----------|---------------------|
| Binary size | ~300 MB | **~4 MB** |
| Startup time | 10–30 seconds | **< 1 second** |
| AI model loading | Fails inside frozen EXE | ✅ Works via subprocess |
| Runtime required | Python bundled inside | ✅ No runtime needed |
| Memory usage | High (PyTorch) | Low (no ML overhead) |

Rust compiles to a **native Windows executable** — no interpreter, no virtual machine, no dependency hell. The binary starts instantly because there's nothing to unpack or load at startup.

---

## 🛠 How It Works

```
┌─────────────────────────────────────────────────────┐
│                  Subtitle Generator                  │
│                   (Rust + egui)                      │
└───────────────────────┬─────────────────────────────┘
                        │ User selects MP4
                        ▼
              ┌─────────────────┐
              │   ffmpeg.exe    │  Extracts audio → 16kHz WAV
              └────────┬────────┘
                       │
                       ▼
           ┌───────────────────────┐
           │   whisper-cli.exe     │  AI speech recognition
           │   (whisper.cpp)       │  Runs locally, offline
           └───────────┬───────────┘
                       │
                       ▼
              ┌─────────────────┐
              │   output.srt    │  Subtitle file saved
              │                 │  next to your video
              └─────────────────┘
```

**Step by step:**

1. **Audio Extraction** — `ffmpeg` pulls the audio track from the MP4 and converts it to a 16 kHz mono WAV file (the format whisper.cpp expects)
2. **Speech Recognition** — `whisper.cpp` runs the Whisper AI model locally on your machine, no internet needed after the first model download
3. **SRT Output** — The transcription is saved as a `.srt` file in the same folder as your video, ready to use in any video player

**Architecture choices:**

| Component | Technology | Why |
|-----------|-----------|-----|
| GUI | [egui](https://github.com/emilk/egui) + [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) | Pure Rust, immediate-mode, no web engine |
| File dialogs | [rfd](https://github.com/PolyMeilex/rfd) | Native OS file picker |
| AI engine | [whisper.cpp](https://github.com/ggerganov/whisper.cpp) | C++ port of OpenAI Whisper — fast, no Python |
| Audio tool | [ffmpeg](https://ffmpeg.org/) | Industry standard, prebuilt static binary |
| Threading | `std::thread` + `Arc<Mutex<>>` | Standard library, zero-cost |

---

## 🚀 Getting Started

### Requirements

- Windows 10 / 11 (64-bit)
- Internet connection for first-time setup (downloads ~150 MB of tools)

### Installation

**1. Clone the repository**
```bash
git clone https://github.com/portwebdesign/subtitle-generator.git
cd subtitle-generator
```

**2. Run the setup script**

Double-click `setup.bat` (or run from terminal):
```cmd
setup.bat
```

This will automatically download:
- `ffmpeg.exe` — audio extraction tool
- `whisper-cli.exe` — AI speech recognition engine
- `ggml-tiny.bin` — Whisper tiny model (~75 MB)

**3. Launch the app**
```cmd
altyazi_uretici.exe
```

### Build from Source

You need [Rust](https://rustup.rs/) installed (1.70+).

```bash
cargo build --release
```

The binary will be at `target/release/altyazi_uretici.exe`.

---

## 🎯 Usage

### Single File

1. Click **"📁 Seç"** (Select) next to the file input
2. Choose your MP4 video
3. Click **"▶ Altyazı Üret"** (Generate Subtitle)
4. The `.srt` file appears next to your video

### Batch Processing

1. Click **"📁 Seç"** next to the folder input
2. Choose a folder containing MP4 files (subfolders are included)
3. Click **"▶ Toplu Üret"** (Batch Generate)
4. All MP4 files are processed in sequence

### Language & Model Selection

| Setting | Options | Notes |
|---------|---------|-------|
| **Model** | tiny, base, small, medium | Larger = better quality, slower |
| **Language** | Turkish, English, German, French, Spanish, Arabic, Russian | Select the spoken language to improve accuracy |

---

## 📦 Project Structure

```
subtitle-generator/
├── src/
│   └── main.rs          # Full application (~600 lines of Rust)
├── Cargo.toml           # Rust dependencies
├── kur.bat              # Setup script (Turkish: "install")
├── bin/                 # Downloaded binaries (gitignored)
│   ├── ffmpeg.exe
│   └── whisper-cli.exe
└── models/              # AI models (gitignored)
    └── ggml-tiny.bin
```

---

## 🤔 Whisper Models

| Model | Size | Speed | Quality | Best for |
|-------|------|-------|---------|----------|
| `tiny` | 75 MB | ⚡⚡⚡⚡ | Good | Quick transcription |
| `base` | 142 MB | ⚡⚡⚡ | Better | Balanced use |
| `small` | 466 MB | ⚡⚡ | Great | High accuracy |
| `medium` | 1.5 GB | ⚡ | Very good | Professional use |

Download additional models manually from [Hugging Face](https://huggingface.co/ggerganov/whisper.cpp) and place them in the `models/` folder.

---

## 🔧 Technical Details

### Why not use `whisper-rs` (Rust bindings)?

Rust bindings to whisper.cpp require LLVM/Clang to compile on Windows, which adds significant complexity to the build environment. By calling the prebuilt `whisper-cli.exe` binary via subprocess instead, the app:

- Builds with just `cargo build` (no LLVM needed)
- Uses `CREATE_NO_WINDOW` flag so no console window flashes
- Always uses the latest whisper.cpp release

### Thread Safety

The UI runs on the main thread (required by Windows GUI). Processing happens in a `std::thread` worker. Communication uses `Arc<Mutex<Vec<String>>>` for the log buffer and `Arc<Mutex<f32>>` for progress — safe, simple, no async runtime overhead.

---

## 📝 License

MIT — see [LICENSE](LICENSE)

---

<div align="center">
  <sub>Built with ❤️ in Rust · Powered by <a href="https://github.com/ggerganov/whisper.cpp">whisper.cpp</a> & <a href="https://ffmpeg.org">FFmpeg</a></sub>
</div>
