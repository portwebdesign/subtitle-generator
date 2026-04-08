<div align="center">

# 🎬 Subtitle Generator

**A fast, lightweight MP4 subtitle generator built with Rust**

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-lightblue?logo=windows)](https://github.com/portwebdesign/subtitle-generator/releases)
[![Powered by whisper.cpp](https://img.shields.io/badge/Powered%20by-whisper.cpp-green)](https://github.com/ggerganov/whisper.cpp)

Generate `.srt` subtitle files from MP4 videos — **no Python, no setup scripts, no cloud**.  
Just run `subtitle-generator.exe`. It handles everything automatically.

</div>

---

## ✨ Why Rust?

Most subtitle tools are Python + PyInstaller. They ship with problems:

| | Python EXE | **This App (Rust)** |
|-|-----------|---------------------|
| Binary size | ~300 MB | **~4 MB** |
| Startup time | 10–30 seconds | **< 1 second** |
| AI model loading | Crashes inside frozen EXE | ✅ Subprocess — always works |
| Runtime required | Python bundled | ✅ No runtime |
| Setup | External `.bat` script | ✅ Built-in, automatic |

---

## 🚀 Getting Started

### 1. Download the release

Grab `subtitle-generator.exe` from the [Releases](https://github.com/portwebdesign/subtitle-generator/releases) page.

### 2. Run it

Double-click `subtitle-generator.exe`.

**On first launch**, the app detects that it needs `ffmpeg`, `whisper-cli`, and the AI model.  
A **First-Time Setup** screen appears and downloads everything automatically (~175 MB).

```
subtitle-generator.exe
│
├── First launch?
│   └── Shows setup screen → downloads tools → proceeds automatically
│
└── Already set up?
    └── Opens directly, ready to use
```

No `.bat` files. No manual steps. No terminal.

---

## 🛠 How It Works

```
┌─────────────────────────────────────────────────┐
│             Subtitle Generator (Rust)            │
│             egui GUI · 4 MB binary               │
└──────────────────┬──────────────────────────────┘
                   │  User selects MP4
                   ▼
         ┌──────────────────┐
         │   ffmpeg.exe     │  Extracts audio → 16 kHz mono WAV
         └────────┬─────────┘
                  │
                  ▼
      ┌───────────────────────┐
      │   whisper-cli.exe     │  AI transcription (runs locally)
      │   + ggml-tiny.bin     │  No internet needed after setup
      └───────────┬───────────┘
                  │
                  ▼
         ┌──────────────────┐
         │   video.srt      │  Saved next to your MP4
         └──────────────────┘
```

**Step by step:**

1. **Audio extraction** — `ffmpeg` strips the audio track and converts it to a 16 kHz mono WAV (the format whisper expects)
2. **Transcription** — `whisper-cli` runs the AI model locally and outputs timed text
3. **SRT output** — saved as `video.srt` next to your original video file

Everything runs **offline**. No API keys. No subscriptions.

---

## 🎯 Usage

### Single File
1. Click **Browse…** next to "Single File"
2. Pick your MP4
3. Click **▶ Generate**
4. The `.srt` appears in the same folder as the video

### Batch Processing
1. Click **Browse…** next to "Batch — Folder"
2. Pick a folder (subfolders are scanned too)
3. Click **▶ Generate All**

### Settings

| Setting | Options | Notes |
|---------|---------|-------|
| **Model** | tiny, base, small, medium | Larger = better quality, slower |
| **Language** | English, Turkish, German, French, Spanish, Arabic, Russian, Chinese, Japanese, Korean | Match the spoken language for best results |

---

## 📦 Project Structure

```
subtitle-generator/
├── src/
│   └── main.rs          # Full Rust app — GUI + processing + auto-setup
├── Cargo.toml           # Dependencies: eframe, egui, rfd
│
│   (created on first run, gitignored)
├── bin/
│   ├── ffmpeg.exe       # Downloaded automatically
│   └── whisper-cli.exe  # Downloaded automatically
└── models/
    └── ggml-tiny.bin    # Downloaded automatically
```

---

## 🤔 Whisper Models

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| `tiny` | ~75 MB | ⚡⚡⚡⚡ | Good |
| `base` | ~142 MB | ⚡⚡⚡ | Better |
| `small` | ~466 MB | ⚡⚡ | Great |
| `medium` | ~1.5 GB | ⚡ | Very good |

The app downloads `tiny` automatically. For other models, grab the `.bin` file from  
[Hugging Face — ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) and drop it in the `models/` folder.

---

## 🔧 Build from Source

You need [Rust](https://rustup.rs/) (1.70+).

```bash
git clone https://github.com/portwebdesign/subtitle-generator.git
cd subtitle-generator
cargo build --release
# Output: target/release/subtitle-generator.exe
```

**No extra system dependencies.** No LLVM, no Python, no CMake.

---

## 🔧 Technical Notes

### Why subprocess instead of `whisper-rs` (Rust bindings)?

Rust bindings to whisper.cpp require LLVM/Clang on Windows — significant build complexity. Using the prebuilt `whisper-cli.exe` via subprocess instead means:
- Build with just `cargo build` — no extra toolchain
- `CREATE_NO_WINDOW` flag prevents console flashing
- Always compatible with the latest whisper.cpp release

### Thread model

The GUI runs on the main thread (required by Windows). All file processing runs in `std::thread` workers. Communication uses `Arc<Mutex<>>` — no async runtime, no tokio, no overhead.

---

## 📝 License

MIT — see [LICENSE](LICENSE)

---

<div align="center">
  <sub>Built with ❤️ in Rust · Powered by <a href="https://github.com/ggerganov/whisper.cpp">whisper.cpp</a> and <a href="https://ffmpeg.org">FFmpeg</a></sub>
</div>
