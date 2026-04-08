#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use eframe::egui;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MP4 Altyazı Üretici")
            .with_inner_size([700.0, 560.0])
            .with_min_inner_size([600.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MP4 Altyazı Üretici",
        options,
        Box::new(|cc| Ok(Box::new(AltyaziApp::new(cc)))),
    )
}

// Uygulama durumu
struct AltyaziApp {
    // Seçilen dosya / klasör
    secilen_dosya: String,
    secilen_klasor: String,

    // Model seçimi
    model: String,

    // Dil seçimi
    dil: String,

    // Log mesajları (shared, thread'den güncellenir)
    log: Arc<Mutex<Vec<String>>>,

    // İşlem devam ediyor mu
    isleniyor: Arc<Mutex<bool>>,

    // İlerleme (0.0 - 1.0)
    ilerleme: Arc<Mutex<f32>>,

    // Bileşenlerin yolları
    whisper_yol: PathBuf,
    ffmpeg_yol: PathBuf,
    model_dizin: PathBuf,
}

impl AltyaziApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let exe_dizin = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();

        let whisper_yol = exe_dizin.join("bin").join("whisper-cli.exe");
        let ffmpeg_yol = exe_dizin.join("bin").join("ffmpeg.exe");
        let model_dizin = exe_dizin.join("models");

        let mut log = Vec::new();
        log.push("═══════════════════════════════════════════".to_string());
        log.push("   MP4 Altyazı Üretici - Hazır".to_string());
        log.push("═══════════════════════════════════════════".to_string());

        // Bileşen kontrolü
        if !whisper_yol.exists() {
            log.push(format!("⚠ whisper-cli.exe bulunamadı: {}", whisper_yol.display()));
            log.push("  → kur.bat dosyasını çalıştırın!".to_string());
        } else {
            log.push(format!("✓ whisper-cli.exe: {}", whisper_yol.display()));
        }

        if !ffmpeg_yol.exists() {
            log.push(format!("⚠ ffmpeg.exe bulunamadı: {}", ffmpeg_yol.display()));
            log.push("  → kur.bat dosyasını çalıştırın!".to_string());
        } else {
            log.push(format!("✓ ffmpeg.exe: {}", ffmpeg_yol.display()));
        }

        // Model dizini kontrolü
        if model_dizin.exists() {
            let modeller: Vec<_> = std::fs::read_dir(&model_dizin)
                .unwrap_or_else(|_| {
                    std::fs::read_dir(".").unwrap()
                })
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.ends_with(".bin"))
                        .unwrap_or(false)
                })
                .collect();
            if modeller.is_empty() {
                log.push("⚠ Model bulunamadı: models/ klasöründe .bin dosyası yok".to_string());
                log.push("  → kur.bat ile tiny model indirebilirsiniz".to_string());
            } else {
                for m in &modeller {
                    log.push(format!("✓ Model: {}", m.file_name().to_str().unwrap_or("")));
                }
            }
        } else {
            log.push(format!("⚠ Model dizini yok: {}", model_dizin.display()));
        }

        log.push("─────────────────────────────────────────".to_string());
        log.push("Kullanım:".to_string());
        log.push("  1. MP4 dosyası veya klasör seçin".to_string());
        log.push("  2. Model ve dil ayarlarını yapın".to_string());
        log.push("  3. 'Altyazı Üret' butonuna tıklayın".to_string());
        log.push("─────────────────────────────────────────".to_string());

        Self {
            secilen_dosya: String::new(),
            secilen_klasor: String::new(),
            model: "tiny".to_string(),
            dil: "tr".to_string(),
            log: Arc::new(Mutex::new(log)),
            isleniyor: Arc::new(Mutex::new(false)),
            ilerleme: Arc::new(Mutex::new(0.0)),
            whisper_yol,
            ffmpeg_yol,
            model_dizin,
        }
    }

    fn log_ekle(&self, mesaj: impl Into<String>) {
        if let Ok(mut log) = self.log.lock() {
            log.push(mesaj.into());
        }
    }

    fn hazir_mi(&self) -> bool {
        self.whisper_yol.exists() && self.ffmpeg_yol.exists()
    }

    fn model_yolu(&self) -> PathBuf {
        self.model_dizin
            .join(format!("ggml-{}.bin", self.model))
    }

    fn islemi_baslat_tek(&self, dosya_yolu: String) {
        let log = Arc::clone(&self.log);
        let isleniyor = Arc::clone(&self.isleniyor);
        let ilerleme = Arc::clone(&self.ilerleme);
        let whisper = self.whisper_yol.clone();
        let ffmpeg = self.ffmpeg_yol.clone();
        let model = self.model_yolu();
        let dil = self.dil.clone();

        thread::spawn(move || {
            *isleniyor.lock().unwrap() = true;
            *ilerleme.lock().unwrap() = 0.0;

            let sonuc = isle_dosya(
                &dosya_yolu,
                &whisper,
                &ffmpeg,
                &model,
                &dil,
                &log,
                &ilerleme,
            );

            match sonuc {
                Ok(_) => {
                    log.lock().unwrap().push("✓ İşlem başarıyla tamamlandı!".to_string());
                }
                Err(e) => {
                    log.lock().unwrap().push(format!("✗ Hata: {}", e));
                }
            }

            *ilerleme.lock().unwrap() = 1.0;
            *isleniyor.lock().unwrap() = false;
        });
    }

    fn islemi_baslat_klasor(&self, klasor_yolu: String) {
        let log = Arc::clone(&self.log);
        let isleniyor = Arc::clone(&self.isleniyor);
        let ilerleme = Arc::clone(&self.ilerleme);
        let whisper = self.whisper_yol.clone();
        let ffmpeg = self.ffmpeg_yol.clone();
        let model = self.model_yolu();
        let dil = self.dil.clone();

        thread::spawn(move || {
            *isleniyor.lock().unwrap() = true;
            *ilerleme.lock().unwrap() = 0.0;

            // MP4 dosyalarını bul
            let mp4_dosyalar = mp4_dosyalaini_bul(&klasor_yolu);

            if mp4_dosyalar.is_empty() {
                log.lock().unwrap().push("⚠ Klasörde MP4 dosyası bulunamadı!".to_string());
                *isleniyor.lock().unwrap() = false;
                return;
            }

            log.lock().unwrap().push(format!(
                "→ {} MP4 dosyası bulundu, işleniyor...",
                mp4_dosyalar.len()
            ));

            let toplam = mp4_dosyalar.len() as f32;

            for (i, dosya) in mp4_dosyalar.iter().enumerate() {
                log.lock().unwrap().push(format!(
                    "\n[{}/{}] İşleniyor: {}",
                    i + 1,
                    mp4_dosyalar.len(),
                    Path::new(dosya).file_name().unwrap_or_default().to_str().unwrap_or("")
                ));

                match isle_dosya(dosya, &whisper, &ffmpeg, &model, &dil, &log, &ilerleme) {
                    Ok(_) => {
                        log.lock().unwrap().push(format!("  ✓ Tamamlandı"));
                    }
                    Err(e) => {
                        log.lock().unwrap().push(format!("  ✗ Hata: {}", e));
                    }
                }

                *ilerleme.lock().unwrap() = (i + 1) as f32 / toplam;
            }

            log.lock().unwrap().push("\n✓ Tüm dosyalar işlendi!".to_string());
            *isleniyor.lock().unwrap() = false;
        });
    }
}

impl eframe::App for AltyaziApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Sürekli yenile (thread güncellemeleri için)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let isleniyor = *self.isleniyor.lock().unwrap();
        let ilerleme = *self.ilerleme.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            // Başlık
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(
                    egui::RichText::new("🎬 MP4 Altyazı Üretici")
                        .size(22.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("whisper.cpp tabanlı • Hızlı • Güvenilir")
                        .size(12.0)
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(8.0);
            });

            ui.separator();
            ui.add_space(8.0);

            // ── Ayarlar satırı ──
            ui.horizontal(|ui| {
                ui.label("Model:");
                egui::ComboBox::from_id_salt("model_sec")
                    .selected_text(&self.model)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.model, "tiny".to_string(), "tiny  (Hızlı, ~75MB)");
                        ui.selectable_value(&mut self.model, "base".to_string(), "base  (Dengeli, ~140MB)");
                        ui.selectable_value(&mut self.model, "small".to_string(), "small  (İyi, ~466MB)");
                        ui.selectable_value(&mut self.model, "medium".to_string(), "medium  (Çok iyi, ~1.5GB)");
                    });

                ui.add_space(16.0);
                ui.label("Dil:");
                egui::ComboBox::from_id_salt("dil_sec")
                    .selected_text(dil_adi(&self.dil))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.dil, "tr".to_string(), "🇹🇷 Türkçe");
                        ui.selectable_value(&mut self.dil, "en".to_string(), "🇬🇧 İngilizce");
                        ui.selectable_value(&mut self.dil, "de".to_string(), "🇩🇪 Almanca");
                        ui.selectable_value(&mut self.dil, "fr".to_string(), "🇫🇷 Fransızca");
                        ui.selectable_value(&mut self.dil, "es".to_string(), "🇪🇸 İspanyolca");
                        ui.selectable_value(&mut self.dil, "ar".to_string(), "🇸🇦 Arapça");
                        ui.selectable_value(&mut self.dil, "ru".to_string(), "🇷🇺 Rusça");
                    });

                // Model dosyası var mı?
                let model_var = self.model_yolu().exists();
                ui.add_space(8.0);
                if model_var {
                    ui.label(egui::RichText::new("✓ Model hazır").color(egui::Color32::GREEN));
                } else {
                    ui.label(
                        egui::RichText::new("⚠ Model yok → kur.bat")
                            .color(egui::Color32::YELLOW),
                    );
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // ── Tek Dosya ──
            ui.label(egui::RichText::new("📄 Tek Dosya İşlemi").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.secilen_dosya)
                        .hint_text("MP4 dosyası seçin...")
                        .desired_width(440.0),
                );
                if ui.button("📁 Seç").clicked() {
                    if let Some(yol) = FileDialog::new()
                        .add_filter("MP4 dosyası", &["mp4", "MP4"])
                        .add_filter("Tüm videolar", &["mp4", "mkv", "avi", "mov"])
                        .pick_file()
                    {
                        self.secilen_dosya = yol.to_string_lossy().to_string();
                    }
                }
                ui.add_enabled_ui(!isleniyor && self.hazir_mi(), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("▶ Altyazı Üret")
                                .color(egui::Color32::WHITE),
                        )
                        .clicked()
                        && !self.secilen_dosya.is_empty()
                    {
                        let model_var = self.model_yolu().exists();
                        if !model_var {
                            self.log_ekle(format!(
                                "✗ Model bulunamadı: {}",
                                self.model_yolu().display()
                            ));
                            self.log_ekle("  → kur.bat çalıştırarak modeli indirin".to_string());
                        } else {
                            self.log_ekle(format!(
                                "→ Başlatılıyor: {}",
                                Path::new(&self.secilen_dosya)
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_str()
                                    .unwrap_or("")
                            ));
                            self.islemi_baslat_tek(self.secilen_dosya.clone());
                        }
                    }
                });
            });

            ui.add_space(12.0);

            // ── Klasör ──
            ui.label(egui::RichText::new("📂 Toplu Klasör İşlemi").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.secilen_klasor)
                        .hint_text("Klasör seçin...")
                        .desired_width(440.0),
                );
                if ui.button("📁 Seç").clicked() {
                    if let Some(yol) = FileDialog::new().pick_folder() {
                        self.secilen_klasor = yol.to_string_lossy().to_string();
                    }
                }
                ui.add_enabled_ui(!isleniyor && self.hazir_mi(), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("▶ Toplu Üret")
                                .color(egui::Color32::WHITE),
                        )
                        .clicked()
                        && !self.secilen_klasor.is_empty()
                    {
                        let model_var = self.model_yolu().exists();
                        if !model_var {
                            self.log_ekle(format!(
                                "✗ Model bulunamadı: {}",
                                self.model_yolu().display()
                            ));
                        } else {
                            self.log_ekle(format!(
                                "→ Klasör taranıyor: {}",
                                &self.secilen_klasor
                            ));
                            self.islemi_baslat_klasor(self.secilen_klasor.clone());
                        }
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            // ── İlerleme ──
            if isleniyor {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("İşleniyor...");
                });
                ui.add(egui::ProgressBar::new(ilerleme).animate(true).show_percentage());
            } else if ilerleme > 0.0 {
                ui.add(egui::ProgressBar::new(ilerleme).text("Tamamlandı"));
            }

            ui.add_space(6.0);

            // ── Log ──
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📋 İşlem Günlüğü").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Temizle").clicked() {
                        self.log.lock().unwrap().clear();
                    }
                });
            });

            ui.add_space(4.0);

            // Log scroll alanı
            let log_satirlar = self.log.lock().unwrap().join("\n");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut log_satirlar.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10),
                    );
                });

            ui.add_space(6.0);
            ui.separator();

            // Alt bilgi
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("whisper.cpp + ffmpeg ile güçlendirilmiştir")
                        .size(10.0)
                        .color(egui::Color32::GRAY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.hazir_mi() {
                        ui.label(
                            egui::RichText::new("⚠ Bileşenler eksik — kur.bat çalıştırın")
                                .size(10.0)
                                .color(egui::Color32::RED),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("✓ Tüm bileşenler hazır")
                                .size(10.0)
                                .color(egui::Color32::GREEN),
                        );
                    }
                });
            });
        });
    }
}

// ── Yardımcı Fonksiyonlar ──────────────────────────────────────────────────

fn dil_adi(kod: &str) -> &'static str {
    match kod {
        "tr" => "🇹🇷 Türkçe",
        "en" => "🇬🇧 İngilizce",
        "de" => "🇩🇪 Almanca",
        "fr" => "🇫🇷 Fransızca",
        "es" => "🇪🇸 İspanyolca",
        "ar" => "🇸🇦 Arapça",
        "ru" => "🇷🇺 Rusça",
        _ => "Otomatik",
    }
}

fn mp4_dosyalaini_bul(klasor: &str) -> Vec<String> {
    let mut dosyalar = Vec::new();
    if let Ok(giris) = std::fs::read_dir(klasor) {
        for girdisi in giris.flatten() {
            let yol = girdisi.path();
            if yol.is_file() {
                if let Some(uzanti) = yol.extension() {
                    if uzanti.to_str().map(|u| u.eq_ignore_ascii_case("mp4")).unwrap_or(false) {
                        dosyalar.push(yol.to_string_lossy().to_string());
                    }
                }
            } else if yol.is_dir() {
                // Alt klasörlere de bak
                let alt = mp4_dosyalaini_bul(&yol.to_string_lossy());
                dosyalar.extend(alt);
            }
        }
    }
    dosyalar
}

fn isle_dosya(
    video_yolu: &str,
    whisper: &Path,
    ffmpeg: &Path,
    model: &Path,
    dil: &str,
    log: &Arc<Mutex<Vec<String>>>,
    ilerleme: &Arc<Mutex<f32>>,
) -> Result<(), String> {
    let video_path = Path::new(video_yolu);
    let dosya_adi = video_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("output");
    let parent = video_path.parent().unwrap_or(Path::new("."));

    // Geçici WAV dosyası
    let wav_yolu = parent.join(format!("{}_temp.wav", dosya_adi));
    let srt_yolu = parent.join(format!("{}.srt", dosya_adi));

    // 1. Ses çıkar
    log.lock().unwrap().push("  → Ses çıkarılıyor...".to_string());
    *ilerleme.lock().unwrap() = 0.1;

    let mut ffmpeg_cmd = Command::new(ffmpeg);
    ffmpeg_cmd.args([
        "-y",
        "-i",
        video_yolu,
        "-ar",
        "16000",
        "-ac",
        "1",
        "-f",
        "wav",
        wav_yolu.to_str().unwrap_or(""),
    ]);
    #[cfg(windows)]
    ffmpeg_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let ffmpeg_cikti = ffmpeg_cmd
        .output()
        .map_err(|e| format!("ffmpeg çalıştırılamadı: {}", e))?;

    if !ffmpeg_cikti.status.success() {
        let hata = String::from_utf8_lossy(&ffmpeg_cikti.stderr);
        return Err(format!("ffmpeg hatası: {}", &hata[hata.len().saturating_sub(200)..hata.len()]));
    }

    log.lock().unwrap().push("  → Ses çıkarıldı. Transkripsiyon başlıyor...".to_string());
    *ilerleme.lock().unwrap() = 0.3;

    // 2. Whisper ile transkripsiyon
    let mut whisper_cmd = Command::new(whisper);
    whisper_cmd.args([
        "-m",
        model.to_str().unwrap_or(""),
        "-f",
        wav_yolu.to_str().unwrap_or(""),
        "-l",
        dil,
        "--output-srt",
        "--output-file",
        parent.join(dosya_adi).to_str().unwrap_or(""),
    ]);
    #[cfg(windows)]
    whisper_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let whisper_cikti = whisper_cmd
        .output()
        .map_err(|e| format!("whisper çalıştırılamadı: {}", e))?;

    // Geçici WAV dosyasını temizle
    let _ = std::fs::remove_file(&wav_yolu);

    *ilerleme.lock().unwrap() = 0.9;

    if !whisper_cikti.status.success() {
        let hata = String::from_utf8_lossy(&whisper_cikti.stderr);
        return Err(format!("whisper hatası: {}", &hata[hata.len().saturating_sub(300)..hata.len()]));
    }

    if srt_yolu.exists() {
        log.lock().unwrap().push(format!(
            "  ✓ SRT oluşturuldu: {}",
            srt_yolu.file_name().unwrap_or_default().to_str().unwrap_or("")
        ));
    } else {
        // whisper bazen .srt yerine dosya adı + .srt oluşturabilir — kontrol et
        let alternatif = parent.join(format!("{}.srt", dosya_adi));
        if !alternatif.exists() {
            return Err("SRT dosyası oluşturulamadı".to_string());
        }
    }

    Ok(())
}
