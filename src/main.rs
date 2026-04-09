#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

// ── Download URLs ──────────────────────────────────────────────────────────────
const FFMPEG_URL: &str =
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
// whisper.cpp moved from ggerganov/ to ggml-org/ — use whisper-bin (no OpenBLAS dep)
const WHISPER_URL: &str =
    "https://github.com/ggml-org/whisper.cpp/releases/download/v1.8.4/whisper-bin-x64.zip";
const MODEL_TINY_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";

// ── Entry point ────────────────────────────────────────────────────────────────
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Subtitle Generator")
            .with_inner_size([720.0, 580.0])
            .with_min_inner_size([620.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Subtitle Generator",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

// ── Setup state ────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq, Debug)]
enum ComponentStatus {
    Pending,
    Downloading,
    Done,
    Error(String),
}

struct SetupState {
    ffmpeg: ComponentStatus,
    whisper: ComponentStatus,
    model: ComponentStatus,
    log: Vec<String>,
    started: bool,
    all_done: bool,
}

impl SetupState {
    fn new() -> Self {
        Self {
            ffmpeg: ComponentStatus::Pending,
            whisper: ComponentStatus::Pending,
            model: ComponentStatus::Pending,
            log: Vec::new(),
            started: false,
            all_done: false,
        }
    }
}

// ── App ────────────────────────────────────────────────────────────────────────
struct App {
    bin_dir: PathBuf,
    model_dir: PathBuf,

    // Setup phase
    needs_setup: bool,
    setup: Arc<Mutex<SetupState>>,

    // Main UI
    selected_file: String,
    selected_folder: String,
    model: String,
    language: String,
    log: Arc<Mutex<Vec<String>>>,
    processing: Arc<Mutex<bool>>,
    progress: Arc<Mutex<f32>>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let exe_dir = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();

        let bin_dir = exe_dir.join("bin");
        let model_dir = exe_dir.join("models");

        let ffmpeg_ok = bin_dir.join("ffmpeg.exe").exists();
        let whisper_ok = bin_dir.join("whisper-cli.exe").exists();
        let model_ok = model_dir.join("ggml-tiny.bin").exists();
        let needs_setup = !ffmpeg_ok || !whisper_ok || !model_ok;

        let mut setup = SetupState::new();
        if ffmpeg_ok { setup.ffmpeg = ComponentStatus::Done; }
        if whisper_ok { setup.whisper = ComponentStatus::Done; }
        if model_ok { setup.model = ComponentStatus::Done; }

        let log = vec![
            "Subtitle Generator ready.".to_string(),
            "Select a file or folder and click Generate.".to_string(),
        ];

        Self {
            bin_dir,
            model_dir,
            needs_setup,
            setup: Arc::new(Mutex::new(setup)),
            selected_file: String::new(),
            selected_folder: String::new(),
            model: "tiny".to_string(),
            language: "en".to_string(),
            log: Arc::new(Mutex::new(log)),
            processing: Arc::new(Mutex::new(false)),
            progress: Arc::new(Mutex::new(0.0)),
        }
    }

    fn start_setup(&self) {
        let setup = Arc::clone(&self.setup);
        let bin_dir = self.bin_dir.clone();
        let model_dir = self.model_dir.clone();

        thread::spawn(move || {
            setup.lock().unwrap().started = true;
            let _ = std::fs::create_dir_all(&bin_dir);
            let _ = std::fs::create_dir_all(&model_dir);

            // 1. FFmpeg
            if !matches!(setup.lock().unwrap().ffmpeg, ComponentStatus::Done) {
                {
                    let mut s = setup.lock().unwrap();
                    s.ffmpeg = ComponentStatus::Downloading;
                    s.log.push("Downloading ffmpeg (~80 MB)...".to_string());
                }
                match download_ffmpeg(&bin_dir) {
                    Ok(_) => {
                        let mut s = setup.lock().unwrap();
                        s.ffmpeg = ComponentStatus::Done;
                        s.log.push("✓ ffmpeg.exe ready".to_string());
                    }
                    Err(e) => {
                        let mut s = setup.lock().unwrap();
                        s.ffmpeg = ComponentStatus::Error(e.clone());
                        s.log.push(format!("✗ ffmpeg failed: {}", e));
                    }
                }
            }

            // 2. Whisper CLI
            if !matches!(setup.lock().unwrap().whisper, ComponentStatus::Done) {
                {
                    let mut s = setup.lock().unwrap();
                    s.whisper = ComponentStatus::Downloading;
                    s.log.push("Downloading whisper.cpp (~20 MB)...".to_string());
                }
                match download_whisper(&bin_dir) {
                    Ok(_) => {
                        let mut s = setup.lock().unwrap();
                        s.whisper = ComponentStatus::Done;
                        s.log.push("✓ whisper-cli.exe ready".to_string());
                    }
                    Err(e) => {
                        let mut s = setup.lock().unwrap();
                        s.whisper = ComponentStatus::Error(e.clone());
                        s.log.push(format!("✗ whisper failed: {}", e));
                    }
                }
            }

            // 3. Whisper Tiny Model
            if !matches!(setup.lock().unwrap().model, ComponentStatus::Done) {
                {
                    let mut s = setup.lock().unwrap();
                    s.model = ComponentStatus::Downloading;
                    s.log.push("Downloading Whisper tiny model (~75 MB)...".to_string());
                }
                let model_dest = model_dir.join("ggml-tiny.bin");
                match download_file(MODEL_TINY_URL, model_dest.to_str().unwrap_or("")) {
                    Ok(_) => {
                        let mut s = setup.lock().unwrap();
                        s.model = ComponentStatus::Done;
                        s.log.push("✓ ggml-tiny.bin ready".to_string());
                    }
                    Err(e) => {
                        let mut s = setup.lock().unwrap();
                        s.model = ComponentStatus::Error(e.clone());
                        s.log.push(format!("✗ model failed: {}", e));
                    }
                }
            }

            // Finalize
            let mut s = setup.lock().unwrap();
            let all_ok = matches!(s.ffmpeg, ComponentStatus::Done)
                && matches!(s.whisper, ComponentStatus::Done)
                && matches!(s.model, ComponentStatus::Done);

            if all_ok {
                s.log.push("✓ All done! Starting the app...".to_string());
                s.all_done = true;
            } else {
                s.log.push("⚠ Setup incomplete. Click Retry to try again.".to_string());
                s.started = false; // allow the button to show again
            }
        });
    }

    fn model_path(&self) -> PathBuf {
        self.model_dir.join(format!("ggml-{}.bin", self.model))
    }

    fn all_ready(&self) -> bool {
        self.bin_dir.join("ffmpeg.exe").exists()
            && self.bin_dir.join("whisper-cli.exe").exists()
            && self.model_path().exists()
    }

    fn generate_file(&self, file: String) {
        let log = Arc::clone(&self.log);
        let processing = Arc::clone(&self.processing);
        let progress = Arc::clone(&self.progress);
        let ffmpeg = self.bin_dir.join("ffmpeg.exe");
        let whisper = self.bin_dir.join("whisper-cli.exe");
        let model = self.model_path();
        let lang = self.language.clone();

        thread::spawn(move || {
            *processing.lock().unwrap() = true;
            *progress.lock().unwrap() = 0.0;
            match process_file(&file, &ffmpeg, &whisper, &model, &lang, &log, &progress) {
                Ok(_) => log.lock().unwrap().push("✓ Subtitle generated!".to_string()),
                Err(e) => log.lock().unwrap().push(format!("✗ Error: {}", e)),
            }
            *progress.lock().unwrap() = 1.0;
            *processing.lock().unwrap() = false;
        });
    }

    fn generate_folder(&self, folder: String) {
        let log = Arc::clone(&self.log);
        let processing = Arc::clone(&self.processing);
        let progress = Arc::clone(&self.progress);
        let ffmpeg = self.bin_dir.join("ffmpeg.exe");
        let whisper = self.bin_dir.join("whisper-cli.exe");
        let model = self.model_path();
        let lang = self.language.clone();

        thread::spawn(move || {
            *processing.lock().unwrap() = true;
            *progress.lock().unwrap() = 0.0;

            let files = find_mp4_files(&folder);
            if files.is_empty() {
                log.lock().unwrap().push("No MP4 files found in folder.".to_string());
                *processing.lock().unwrap() = false;
                return;
            }

            log.lock().unwrap().push(format!("Found {} MP4 file(s). Processing...", files.len()));
            let total = files.len() as f32;

            for (i, file) in files.iter().enumerate() {
                let name = Path::new(file).file_name().unwrap_or_default().to_str().unwrap_or("");
                log.lock().unwrap().push(format!("[{}/{}] {}", i + 1, files.len(), name));

                match process_file(file, &ffmpeg, &whisper, &model, &lang, &log, &progress) {
                    Ok(_) => log.lock().unwrap().push("  ✓ Done".to_string()),
                    Err(e) => log.lock().unwrap().push(format!("  ✗ {}", e)),
                }
                *progress.lock().unwrap() = (i + 1) as f32 / total;
            }

            log.lock().unwrap().push("All files processed.".to_string());
            *processing.lock().unwrap() = false;
        });
    }
}

// ── UI ─────────────────────────────────────────────────────────────────────────
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(150));

        if self.needs_setup && self.setup.lock().unwrap().all_done {
            self.needs_setup = false;
        }

        if self.needs_setup {
            self.show_setup(ctx);
        } else {
            self.show_main(ctx);
        }
    }
}

impl App {
    fn show_setup(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.heading(egui::RichText::new("⚙  First-Time Setup").size(24.0).strong());
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Subtitle Generator needs three components to work.\nThey will be downloaded automatically — about 175 MB total.",
                    )
                    .size(13.0)
                    .color(egui::Color32::GRAY),
                );
                ui.add_space(24.0);
            });

            ui.separator();
            ui.add_space(16.0);

            let (f, w, m, started, log_text) = {
                let s = self.setup.lock().unwrap();
                (s.ffmpeg.clone(), s.whisper.clone(), s.model.clone(), s.started, s.log.join("\n"))
            };

            let items = [
                ("ffmpeg.exe", "Audio extraction — converts your video to WAV", "~80 MB", &f),
                ("whisper-cli.exe", "AI speech engine — runs Whisper locally", "~20 MB", &w),
                ("ggml-tiny.bin", "Whisper tiny model — fast, offline transcription", "~75 MB", &m),
            ];

            for (name, desc, size, status) in &items {
                ui.horizontal(|ui| {
                    let (icon, color) = match status {
                        ComponentStatus::Pending      => ("○", egui::Color32::from_gray(140)),
                        ComponentStatus::Downloading  => ("↓", egui::Color32::YELLOW),
                        ComponentStatus::Done         => ("✓", egui::Color32::GREEN),
                        ComponentStatus::Error(_)     => ("✗", egui::Color32::RED),
                    };
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(icon).size(18.0).color(color));
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(*name).strong());
                        ui.label(
                            egui::RichText::new(format!("{desc}  ·  {size}"))
                                .size(11.0)
                                .color(egui::Color32::GRAY),
                        );
                    });
                });
                ui.add_space(10.0);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(12.0);

            let has_error = matches!(f, ComponentStatus::Error(_))
                || matches!(w, ComponentStatus::Error(_))
                || matches!(m, ComponentStatus::Error(_));

            let any_downloading = matches!(f, ComponentStatus::Downloading)
                || matches!(w, ComponentStatus::Downloading)
                || matches!(m, ComponentStatus::Downloading);

            if !started {
                // Initial download button
                ui.vertical_centered(|ui| {
                    if ui
                        .add_sized([220.0, 40.0], egui::Button::new(
                            egui::RichText::new("⬇  Download & Install").size(15.0),
                        ))
                        .clicked()
                    {
                        self.start_setup();
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("This only happens once. Tools are saved next to the app.")
                            .size(11.0)
                            .color(egui::Color32::GRAY),
                    );
                });
            } else if has_error && !any_downloading {
                // Retry button when something failed
                ui.vertical_centered(|ui| {
                    if ui
                        .add_sized([180.0, 36.0], egui::Button::new(
                            egui::RichText::new("↺  Retry Failed").size(14.0),
                        ))
                        .clicked()
                    {
                        {
                            let mut s = self.setup.lock().unwrap();
                            if matches!(s.ffmpeg, ComponentStatus::Error(_)) {
                                s.ffmpeg = ComponentStatus::Pending;
                            }
                            if matches!(s.whisper, ComponentStatus::Error(_)) {
                                s.whisper = ComponentStatus::Pending;
                            }
                            if matches!(s.model, ComponentStatus::Error(_)) {
                                s.model = ComponentStatus::Pending;
                            }
                            s.log.push("--- Retrying failed downloads ---".to_string());
                        }
                        self.start_setup();
                    }
                });
            }

            if started {
                if any_downloading {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Downloading... this may take a few minutes.");
                    });
                    ui.add_space(8.0);
                }

                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut log_text.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(9),
                        );
                    });
            }
        });
    }

    fn show_main(&mut self, ctx: &egui::Context) {
        let processing = *self.processing.lock().unwrap();
        let progress = *self.progress.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading(egui::RichText::new("🎬  Subtitle Generator").size(22.0).strong());
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Powered by whisper.cpp · Works offline · No cloud, no Python")
                        .size(12.0)
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(10.0);
            });

            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Model:");
                egui::ComboBox::from_id_salt("model_combo")
                    .selected_text(&self.model)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.model, "tiny".to_string(),   "tiny   — Fast  (~75 MB)");
                        ui.selectable_value(&mut self.model, "base".to_string(),   "base   — Balanced (~142 MB)");
                        ui.selectable_value(&mut self.model, "small".to_string(),  "small  — Accurate (~466 MB)");
                        ui.selectable_value(&mut self.model, "medium".to_string(), "medium — Best  (~1.5 GB)");
                    });

                ui.add_space(16.0);
                ui.label("Language:");
                egui::ComboBox::from_id_salt("lang_combo")
                    .selected_text(lang_label(&self.language))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.language, "en".to_string(), "English");
                        ui.selectable_value(&mut self.language, "tr".to_string(), "Turkish");
                        ui.selectable_value(&mut self.language, "de".to_string(), "German");
                        ui.selectable_value(&mut self.language, "fr".to_string(), "French");
                        ui.selectable_value(&mut self.language, "es".to_string(), "Spanish");
                        ui.selectable_value(&mut self.language, "ar".to_string(), "Arabic");
                        ui.selectable_value(&mut self.language, "ru".to_string(), "Russian");
                        ui.selectable_value(&mut self.language, "zh".to_string(), "Chinese");
                        ui.selectable_value(&mut self.language, "ja".to_string(), "Japanese");
                        ui.selectable_value(&mut self.language, "ko".to_string(), "Korean");
                    });

                ui.add_space(12.0);
                if self.model_path().exists() {
                    ui.label(egui::RichText::new("✓ Ready").color(egui::Color32::GREEN).size(12.0));
                } else {
                    ui.label(egui::RichText::new("⚠ Model not found").color(egui::Color32::YELLOW).size(12.0));
                }
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(12.0);

            ui.label(egui::RichText::new("📄  Single File").strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.selected_file)
                        .hint_text("Select an MP4 file...")
                        .desired_width(450.0),
                );
                if ui.button("Browse…").clicked() {
                    if let Some(p) = FileDialog::new()
                        .add_filter("Video files", &["mp4", "MP4", "mkv", "mov", "avi"])
                        .pick_file()
                    {
                        self.selected_file = p.to_string_lossy().to_string();
                    }
                }
                ui.add_enabled_ui(!processing && self.all_ready(), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("▶ Generate").strong())).clicked()
                        && !self.selected_file.is_empty()
                    {
                        let name = Path::new(&self.selected_file)
                            .file_name().unwrap_or_default().to_str().unwrap_or("").to_string();
                        self.log.lock().unwrap().push(format!("Processing: {name}"));
                        self.generate_file(self.selected_file.clone());
                    }
                });
            });

            ui.add_space(14.0);

            ui.label(egui::RichText::new("📂  Batch — Folder").strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.selected_folder)
                        .hint_text("Select a folder (searches subdirectories too)...")
                        .desired_width(450.0),
                );
                if ui.button("Browse…").clicked() {
                    if let Some(p) = FileDialog::new().pick_folder() {
                        self.selected_folder = p.to_string_lossy().to_string();
                    }
                }
                ui.add_enabled_ui(!processing && self.all_ready(), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("▶ Generate All").strong())).clicked()
                        && !self.selected_folder.is_empty()
                    {
                        self.log.lock().unwrap().push(format!("Scanning: {}", &self.selected_folder));
                        self.generate_folder(self.selected_folder.clone());
                    }
                });
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);

            if processing {
                ui.horizontal(|ui| { ui.spinner(); ui.label("Processing..."); });
                ui.add_space(4.0);
                ui.add(egui::ProgressBar::new(progress).animate(true));
            } else if progress > 0.0 {
                ui.add(egui::ProgressBar::new(progress).text("Complete"));
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Log").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Clear").clicked() {
                        self.log.lock().unwrap().clear();
                    }
                });
            });
            ui.add_space(4.0);

            let log_text = self.log.lock().unwrap().join("\n");
            egui::ScrollArea::vertical()
                .max_height(190.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut log_text.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10),
                    );
                });

            ui.add_space(4.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("whisper.cpp + ffmpeg").size(10.0).color(egui::Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.all_ready() {
                        ui.label(egui::RichText::new("✓ All components ready").size(10.0).color(egui::Color32::GREEN));
                    } else {
                        ui.label(egui::RichText::new("⚠ Missing components").size(10.0).color(egui::Color32::RED));
                    }
                });
            });
        });
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────
fn lang_label(code: &str) -> &'static str {
    match code {
        "en" => "English", "tr" => "Turkish", "de" => "German",
        "fr" => "French",  "es" => "Spanish",  "ar" => "Arabic",
        "ru" => "Russian", "zh" => "Chinese",  "ja" => "Japanese",
        "ko" => "Korean",  _ => "Auto",
    }
}

fn find_mp4_files(folder: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.extension().and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("mp4")).unwrap_or(false)
                {
                    files.push(path.to_string_lossy().to_string());
                }
            } else if path.is_dir() {
                files.extend(find_mp4_files(&path.to_string_lossy()));
            }
        }
    }
    files
}

fn process_file(
    video: &str,
    ffmpeg: &Path,
    whisper: &Path,
    model: &Path,
    lang: &str,
    log: &Arc<Mutex<Vec<String>>>,
    progress: &Arc<Mutex<f32>>,
) -> Result<(), String> {
    let vp = Path::new(video);
    let stem = vp.file_stem().unwrap_or_default().to_str().unwrap_or("out");
    let parent = vp.parent().unwrap_or(Path::new("."));
    let wav = parent.join(format!("{}_temp.wav", stem));
    let srt = parent.join(format!("{}.srt", stem));

    log.lock().unwrap().push("  Extracting audio...".to_string());
    *progress.lock().unwrap() = 0.1;

    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-y", "-i", video, "-ar", "16000", "-ac", "1", "-f", "wav", wav.to_str().unwrap_or("")]);
    #[cfg(windows)] cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| format!("ffmpeg launch failed: {}", e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffmpeg: {}", &err[err.len().saturating_sub(300)..]));
    }

    log.lock().unwrap().push("  Transcribing with Whisper...".to_string());
    *progress.lock().unwrap() = 0.3;

    let out_base = parent.join(stem);
    let mut cmd = Command::new(whisper);
    cmd.args(["-m", model.to_str().unwrap_or(""), "-f", wav.to_str().unwrap_or(""),
              "-l", lang, "--output-srt", "--output-file", out_base.to_str().unwrap_or("")]);
    #[cfg(windows)] cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| format!("whisper launch failed: {}", e))?;

    let _ = std::fs::remove_file(&wav);
    *progress.lock().unwrap() = 0.9;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("whisper: {}", &err[err.len().saturating_sub(300)..]));
    }

    if srt.exists() {
        log.lock().unwrap().push(format!("  Saved: {}", srt.file_name().unwrap_or_default().to_str().unwrap_or("")));
    }
    Ok(())
}

// ── Download via PowerShell (always available on Windows) ─────────────────────
fn run_ps(script: &str) -> Result<(), String> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-Command", script]);
    #[cfg(windows)] cmd.creation_flags(0x08000000);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let msg = if !stderr.trim().is_empty() { stderr.lines().last().unwrap_or("").to_string() }
                  else if !stdout.trim().is_empty() { stdout.lines().last().unwrap_or("").to_string() }
                  else { "PowerShell exited with error (no message)".to_string() };
        Err(msg)
    }
}

fn download_file(url: &str, dest: &str) -> Result<(), String> {
    let script = format!(
        "if (Get-Command curl.exe -ErrorAction SilentlyContinue) {{ \
            curl.exe -sL -f -o '{}' '{}' \
         }} else {{ \
            [Net.ServicePointManager]::SecurityProtocol = 'Tls12'; \
            (New-Object System.Net.WebClient).DownloadFile('{}', '{}') \
         }}",
        dest, url, url, dest
    );
    run_ps(&script)
}

fn check_zip_size(path: &Path, min_bytes: u64) -> Result<(), String> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size < min_bytes {
        let _ = std::fs::remove_file(path);
        Err(format!("Downloaded file is too small ({} KB) — likely a 404 or redirect page", size / 1024))
    } else {
        Ok(())
    }
}

fn download_ffmpeg(bin_dir: &Path) -> Result<(), String> {
    let dest = bin_dir.join("ffmpeg.exe");
    if dest.exists() { return Ok(()); }

    let tmp = std::env::temp_dir().join("_sg_ffmpeg.zip");
    download_file(FFMPEG_URL, tmp.to_str().unwrap_or(""))?;
    check_zip_size(&tmp, 5_000_000)?; // expect at least 5 MB

    let zip_s = tmp.to_str().unwrap_or("").replace('\'', "''");
    let dst_s = dest.to_str().unwrap_or("").replace('\'', "''");

    let script = format!(
        "Add-Type -Assembly System.IO.Compression.FileSystem; \
         $z = [System.IO.Compression.ZipFile]::OpenRead('{zip_s}'); \
         $e = $z.Entries | Where-Object {{ $_.Name -eq 'ffmpeg.exe' }} | Select-Object -First 1; \
         if ($e) {{ [System.IO.Compression.ZipFileExtensions]::ExtractToFile($e, '{dst_s}', $true) }}; \
         $z.Dispose(); \
         Remove-Item '{zip_s}' -Force -ErrorAction SilentlyContinue"
    );
    run_ps(&script)?;
    if dest.exists() { Ok(()) } else { Err("ffmpeg.exe not found after extraction".into()) }
}

fn download_whisper(bin_dir: &Path) -> Result<(), String> {
    let dest = bin_dir.join("whisper-cli.exe");
    if dest.exists() { return Ok(()); }

    let tmp = std::env::temp_dir().join("_sg_whisper.zip");

    download_file(WHISPER_URL, tmp.to_str().unwrap_or(""))?;
    check_zip_size(&tmp, 1_000_000)?;

    let bin_s = bin_dir.to_str().unwrap_or("").replace('\'', "''");
    let zip_s = tmp.to_str().unwrap_or("").replace('\'', "''");
    let dst_s = dest.to_str().unwrap_or("").replace('\'', "''");

    // Extract ALL files (exe + DLLs), then standardize the exe name
    let script = format!(
        "Add-Type -Assembly System.IO.Compression.FileSystem; \
         try {{ \
           $z = [System.IO.Compression.ZipFile]::OpenRead('{zip_s}'); \
           foreach ($e in $z.Entries) {{ \
             if ($e.Name -ne '') {{ \
               $d = Join-Path '{bin_s}' $e.Name; \
               try {{ [System.IO.Compression.ZipFileExtensions]::ExtractToFile($e, $d, $true) }} catch {{}} \
             }} \
           }}; \
           $z.Dispose() \
         }} catch {{ Write-Error $_.Exception.Message; exit 1 }}; \
         if (-not (Test-Path '{dst_s}')) {{ \
           $f = Get-ChildItem '{bin_s}' -Filter '*.exe' | \
                Where-Object {{ $_.Name -match 'main|whisper' }} | \
                Select-Object -First 1; \
           if ($f) {{ Copy-Item $f.FullName '{dst_s}' -Force }} \
         }}; \
         Remove-Item '{zip_s}' -Force -ErrorAction SilentlyContinue"
    );

    run_ps(&script)?;

    if dest.exists() {
        Ok(())
    } else {
        // List bin_dir to help diagnose
        let found: Vec<String> = std::fs::read_dir(bin_dir)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        Err(format!(
            "whisper exe not found in archive. Files extracted: {}",
            if found.is_empty() { "(none)".to_string() } else { found.join(", ") }
        ))
    }
}
