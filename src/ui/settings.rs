use crate::shortcuts::ShortcutManager;
use crate::ui::editor::ShortcutTarget;
use crate::ui::theme::{apply_adwaita_theme, AdwaitaThemeMode};
use egui::{Color32, RichText, Rounding};

pub struct SettingsApp {
    candidate_shortcut: String,
    candidate_record_shortcut: String,
    shortcut_conflict: Option<String>,
    record_shortcut_conflict: Option<String>,
    recording_shortcut_target: Option<ShortcutTarget>,
    status_message: Option<(String, std::time::Instant)>,
    theme_mode: AdwaitaThemeMode,
    auto_check_updates: bool,
    update_check_status: Option<String>,
    available_release: Option<crate::updater::ReleaseInfo>,
    is_checking_updates: bool,
    update_rx: std::sync::mpsc::Receiver<Result<Option<crate::updater::ReleaseInfo>, String>>,
    update_tx: std::sync::mpsc::Sender<Result<Option<crate::updater::ReleaseInfo>, String>>,
}

impl SettingsApp {
    pub fn new() -> Self {
        let cfg = crate::config::AppConfig::load();
        let candidate_shortcut = ShortcutManager::get_current_binding();
        let candidate_record_shortcut = ShortcutManager::get_current_record_binding();
        let shortcut_conflict = ShortcutManager::detect_conflict(&candidate_shortcut)
            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
        let record_shortcut_conflict = ShortcutManager::detect_conflict(&candidate_record_shortcut)
            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));

        let (update_tx, update_rx) = std::sync::mpsc::channel();

        Self {
            candidate_shortcut,
            candidate_record_shortcut,
            shortcut_conflict,
            record_shortcut_conflict,
            recording_shortcut_target: None,
            status_message: None,
            theme_mode: AdwaitaThemeMode::Dark,
            auto_check_updates: cfg.auto_check_updates,
            update_check_status: None,
            available_release: None,
            is_checking_updates: false,
            update_rx,
            update_tx,
        }
    }

    fn check_updates_async(&mut self) {
        self.is_checking_updates = true;
        self.update_check_status = Some("Verificando atualizações no GitHub...".into());
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let res = crate::updater::check_for_updates();
            let _ = tx.send(res);
        });
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_adwaita_theme(ctx, self.theme_mode);

        // Handle interactive key sequence recording
        if let Some(target) = self.recording_shortcut_target {
            let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

            if escape_pressed {
                self.recording_shortcut_target = None;
                self.set_status("Captura de atalho cancelada");
            } else if enter_pressed {
                self.recording_shortcut_target = None;
                match target {
                    ShortcutTarget::Screenshot => {
                        self.shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_shortcut)
                            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                    }
                    ShortcutTarget::ScreenRecord => {
                        self.record_shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_record_shortcut)
                            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                    }
                }
                self.set_status("Atalho gravado! Clique em 'Salvar Atalhos' para aplicar");
            } else {
                let new_binding = ctx.input(|i| {
                    let mut parts = Vec::new();
                    if i.modifiers.ctrl {
                        parts.push("<Ctrl>");
                    }
                    if i.modifiers.alt {
                        parts.push("<Alt>");
                    }
                    if i.modifiers.shift {
                        parts.push("<Shift>");
                    }
                    if i.modifiers.mac_cmd {
                        parts.push("<Super>");
                    }

                    for event in &i.events {
                        if let egui::Event::Key { key, pressed: true, .. } = event {
                            let key_name = match key {
                                egui::Key::F1 => Some("F1"),
                                egui::Key::F2 => Some("F2"),
                                egui::Key::F3 => Some("F3"),
                                egui::Key::F4 => Some("F4"),
                                egui::Key::F5 => Some("F5"),
                                egui::Key::F6 => Some("F6"),
                                egui::Key::F7 => Some("F7"),
                                egui::Key::F8 => Some("F8"),
                                egui::Key::F9 => Some("F9"),
                                egui::Key::F10 => Some("F10"),
                                egui::Key::F11 => Some("F11"),
                                egui::Key::F12 => Some("F12"),
                                egui::Key::Space => Some("space"),
                                egui::Key::A => Some("a"),
                                egui::Key::B => Some("b"),
                                egui::Key::C => Some("c"),
                                egui::Key::D => Some("d"),
                                egui::Key::E => Some("e"),
                                egui::Key::F => Some("f"),
                                egui::Key::G => Some("g"),
                                egui::Key::H => Some("h"),
                                egui::Key::I => Some("i"),
                                egui::Key::J => Some("j"),
                                egui::Key::K => Some("k"),
                                egui::Key::L => Some("l"),
                                egui::Key::M => Some("m"),
                                egui::Key::N => Some("n"),
                                egui::Key::O => Some("o"),
                                egui::Key::P => Some("p"),
                                egui::Key::Q => Some("q"),
                                egui::Key::R => Some("r"),
                                egui::Key::S => Some("s"),
                                egui::Key::T => Some("t"),
                                egui::Key::U => Some("u"),
                                egui::Key::V => Some("v"),
                                egui::Key::W => Some("w"),
                                egui::Key::X => Some("x"),
                                egui::Key::Y => Some("y"),
                                egui::Key::Z => Some("z"),
                                egui::Key::Num0 => Some("0"),
                                egui::Key::Num1 => Some("1"),
                                egui::Key::Num2 => Some("2"),
                                egui::Key::Num3 => Some("3"),
                                egui::Key::Num4 => Some("4"),
                                egui::Key::Num5 => Some("5"),
                                egui::Key::Num6 => Some("6"),
                                egui::Key::Num7 => Some("7"),
                                egui::Key::Num8 => Some("8"),
                                egui::Key::Num9 => Some("9"),
                                _ => None,
                            };
                            if let Some(name) = key_name {
                                parts.push(name);
                                return Some(parts.concat());
                            }
                        }
                    }
                    None
                });

                if let Some(binding) = new_binding {
                    match target {
                        ShortcutTarget::Screenshot => {
                            self.candidate_shortcut = binding;
                        }
                        ShortcutTarget::ScreenRecord => {
                            self.candidate_record_shortcut = binding;
                        }
                    }
                }
            }
        }

        let mut should_close = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("⚙️ Configurações do BenderShot");
            ui.label(RichText::new("Personalize os atalhos globais de captura e gravação").italics());
            ui.add_space(12.0);

            // 1. Screenshot Shortcut
            ui.label(RichText::new("📷 Atalho para Captura de Tela:").strong());
            ui.horizontal(|ui| {
                let is_rec = self.recording_shortcut_target == Some(ShortcutTarget::Screenshot);
                let btn_label = if is_rec {
                    "🔴 Pressione as teclas... (Enter confirma, Esc descarta)"
                } else {
                    "🎹 Pressionar Teclas para Definir"
                };
                let btn = egui::Button::new(
                    RichText::new(btn_label)
                        .color(if is_rec { Color32::from_rgb(255, 120, 0) } else { Color32::WHITE })
                        .strong(),
                )
                .fill(if is_rec { Color32::from_rgb(70, 30, 20) } else { Color32::from_rgb(53, 132, 228) })
                .rounding(Rounding::same(6.0));

                if ui.add(btn).clicked() {
                    if is_rec {
                        self.recording_shortcut_target = None;
                    } else {
                        self.recording_shortcut_target = Some(ShortcutTarget::Screenshot);
                    }
                }

                let response = ui.text_edit_singleline(&mut self.candidate_shortcut);
                if response.changed() {
                    self.shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_shortcut)
                        .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                }
            });

            ui.add_space(2.0);
            if let Some(ref conflict) = self.shortcut_conflict {
                ui.colored_label(Color32::from_rgb(224, 27, 36), conflict);
            } else {
                ui.colored_label(Color32::from_rgb(46, 194, 126), "✔ Nenhum conflito detectado");
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Predefinições recomendadas para Captura:").italics());
            ui.horizontal_wrapped(|ui| {
                let mut quick_btn = |label: &str| {
                    if ui.button(label).clicked() {
                        self.candidate_shortcut = label.to_string();
                        self.shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_shortcut)
                            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                    }
                };
                quick_btn("Ctrl+Print");
                quick_btn("<Ctrl>Print");
                quick_btn("Ctrl+Shift+S");
                quick_btn("<Super><Shift>S");
                quick_btn("Print");
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);

            // 2. Screen Recording Shortcut
            ui.label(RichText::new("🎥 Atalho para Iniciar / Parar Gravação de Vídeo:").strong());
            ui.horizontal(|ui| {
                let is_rec = self.recording_shortcut_target == Some(ShortcutTarget::ScreenRecord);
                let btn_label = if is_rec {
                    "🔴 Pressione as teclas... (Enter confirma, Esc descarta)"
                } else {
                    "🎹 Pressionar Teclas para Definir"
                };
                let btn = egui::Button::new(
                    RichText::new(btn_label)
                        .color(if is_rec { Color32::from_rgb(255, 120, 0) } else { Color32::WHITE })
                        .strong(),
                )
                .fill(if is_rec { Color32::from_rgb(70, 30, 20) } else { Color32::from_rgb(46, 194, 126) })
                .rounding(Rounding::same(6.0));

                if ui.add(btn).clicked() {
                    if is_rec {
                        self.recording_shortcut_target = None;
                    } else {
                        self.recording_shortcut_target = Some(ShortcutTarget::ScreenRecord);
                    }
                }

                let response_rec = ui.text_edit_singleline(&mut self.candidate_record_shortcut);
                if response_rec.changed() {
                    self.record_shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_record_shortcut)
                        .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                }
            });

            ui.add_space(2.0);
            if let Some(ref conflict) = self.record_shortcut_conflict {
                ui.colored_label(Color32::from_rgb(224, 27, 36), conflict);
            } else {
                ui.colored_label(Color32::from_rgb(46, 194, 126), "✔ Nenhum conflito detectado");
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Predefinições recomendadas para Gravação:").italics());
            ui.horizontal_wrapped(|ui| {
                let mut quick_btn = |label: &str| {
                    if ui.button(label).clicked() {
                        self.candidate_record_shortcut = label.to_string();
                        self.record_shortcut_conflict = ShortcutManager::detect_conflict(&self.candidate_record_shortcut)
                            .map(|c| format!("⚠️ Conflito com: {} ({})", c.key, c.description));
                    }
                };
                quick_btn("Ctrl+Shift+R");
                quick_btn("<Ctrl><Shift>R");
                quick_btn("<Ctrl>Print");
                quick_btn("<Super><Alt>R");
                quick_btn("<Control><Alt>v");
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);

            // 3. System Updates Section
            ui.heading("🔄 Atualizações do BenderShot");
            ui.add_space(4.0);

            let current_ver = crate::config::AppConfig::get_effective_version();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Versão Atual:").strong());
                ui.label(RichText::new(&current_ver).color(Color32::from_rgb(53, 132, 228)));

                if ui.checkbox(&mut self.auto_check_updates, "Checar atualizações automaticamente").changed() {
                    let mut cfg = crate::config::AppConfig::load();
                    cfg.auto_check_updates = self.auto_check_updates;
                    let _ = cfg.save();
                }
            });

            // Check if async update response arrived
            while let Ok(res) = self.update_rx.try_recv() {
                self.is_checking_updates = false;
                match res {
                    Ok(Some(release)) => {
                        self.update_check_status = Some(format!("Nova versão disponível: {}!", release.tag_name));
                        self.available_release = Some(release);
                    }
                    Ok(None) => {
                        self.update_check_status = Some("Você já está usando a versão mais recente.".into());
                        self.available_release = None;
                    }
                    Err(e) => {
                        self.update_check_status = Some(format!("Erro ao verificar: {}", e));
                        self.available_release = None;
                    }
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let check_btn = egui::Button::new(
                    RichText::new(if self.is_checking_updates { "⏳ Verificando..." } else { "🔍 Verificar Atualizações" }).strong()
                ).rounding(Rounding::same(6.0));

                if ui.add_enabled(!self.is_checking_updates, check_btn).clicked() {
                    self.check_updates_async();
                }

                if let Some(ref release) = self.available_release {
                    let update_btn = egui::Button::new(
                        RichText::new(format!("⬆️ Instalar {}", release.tag_name)).strong()
                    )
                    .fill(Color32::from_rgb(46, 194, 126))
                    .rounding(Rounding::same(6.0));

                    if ui.add(update_btn).clicked() {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).args(["update", "--gui"]).spawn();
                        }
                    }
                }

                if let Some(ref status) = self.update_check_status {
                    let color = if self.available_release.is_some() {
                        Color32::from_rgb(46, 194, 126)
                    } else if status.starts_with("Erro") {
                        Color32::from_rgb(224, 27, 36)
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    };
                    ui.colored_label(color, status);
                }
            });

            ui.add_space(16.0);
            if let Some((ref msg, time)) = self.status_message {
                if time.elapsed().as_secs() < 4 {
                    ui.colored_label(Color32::from_rgb(46, 194, 126), msg);
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(RichText::new("💾 Salvar Configurações").strong())
                    .fill(Color32::from_rgb(46, 194, 126))
                    .rounding(Rounding::same(6.0));
                if ui.add(save_btn).clicked() {
                    let mut ok = true;
                    if let Err(e) = ShortcutManager::set_binding(&self.candidate_shortcut) {
                        self.set_status(format!("Erro ao definir atalho de captura: {}", e));
                        ok = false;
                    }
                    if let Err(e) = ShortcutManager::set_record_binding(&self.candidate_record_shortcut) {
                        self.set_status(format!("Erro ao definir atalho de gravação: {}", e));
                        ok = false;
                    }
                    let mut cfg = crate::config::AppConfig::load();
                    cfg.auto_check_updates = self.auto_check_updates;
                    let _ = cfg.save();

                    if ok {
                        self.set_status("Configurações salvas com sucesso!");
                    }
                }

                if ui.button("🧙 Assistente").clicked() {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).arg("wizard").spawn();
                    }
                }

                if ui.button("Fechar").clicked() {
                    should_close = true;
                }
            });
        });

        if should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn run_settings_window() -> Result<(), eframe::Error> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("BenderShot - Configurações")
        .with_app_id("bendershot")
        .with_inner_size([560.0, 560.0])
        .with_min_inner_size([480.0, 480.0]);
    if let Some(icon) = crate::ui::load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "bendershot",
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new()))),
    )
}
