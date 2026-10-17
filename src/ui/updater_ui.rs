use crate::ui::theme::{apply_adwaita_theme, AdwaitaThemeMode};
use crate::updater::{check_for_updates, detect_install_method, perform_in_place_update, InstallMethod, ReleaseInfo};
use egui::{Color32, RichText, Rounding, ScrollArea};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub enum UpdaterEvent {
    CheckComplete(Result<Option<ReleaseInfo>, String>),
    Progress(String),
    UpdateComplete(Result<(), String>),
}

pub struct UpdaterApp {
    current_version: String,
    install_method: InstallMethod,
    release_info: Option<ReleaseInfo>,
    is_checking: bool,
    is_updating: bool,
    status_text: String,
    status_is_error: bool,
    event_tx: Sender<UpdaterEvent>,
    event_rx: Receiver<UpdaterEvent>,
    theme_mode: AdwaitaThemeMode,
}

impl UpdaterApp {
    pub fn new() -> Self {
        let (event_tx, event_rx) = channel();
        let current_version = crate::config::AppConfig::get_effective_version();
        let install_method = detect_install_method();

        let app = Self {
            current_version,
            install_method,
            release_info: None,
            is_checking: false,
            is_updating: false,
            status_text: "Clique em 'Verificar Agora' para buscar atualizações.".to_string(),
            status_is_error: false,
            event_tx,
            event_rx,
            theme_mode: AdwaitaThemeMode::Dark,
        };

        // Trigger an initial check
        app.trigger_check();
        app
    }

    pub fn trigger_check(&self) {
        let tx = self.event_tx.clone();
        thread::spawn(move || {
            let res = check_for_updates();
            let _ = tx.send(UpdaterEvent::CheckComplete(res));
        });
    }

    pub fn trigger_update(&mut self, release: ReleaseInfo) {
        self.is_updating = true;
        self.status_text = "Iniciando atualização...".to_string();
        self.status_is_error = false;

        let tx = self.event_tx.clone();
        thread::spawn(move || {
            let tx_prog = tx.clone();
            let res = perform_in_place_update(&release, move |msg| {
                let _ = tx_prog.send(UpdaterEvent::Progress(msg.to_string()));
            });
            let _ = tx.send(UpdaterEvent::UpdateComplete(res));
        });
    }
}

impl eframe::App for UpdaterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_adwaita_theme(ctx, self.theme_mode);

        // Process incoming events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                UpdaterEvent::CheckComplete(res) => {
                    self.is_checking = false;
                    match res {
                        Ok(Some(info)) => {
                            self.status_text = format!("Nova versão disponível: {}!", info.tag_name);
                            self.status_is_error = false;
                            self.release_info = Some(info);
                        }
                        Ok(None) => {
                            self.status_text = "O BenderShot já está na versão mais recente!".to_string();
                            self.status_is_error = false;
                            self.release_info = None;
                        }
                        Err(e) => {
                            self.status_text = format!("Erro ao checar atualizações: {e}");
                            self.status_is_error = true;
                        }
                    }
                }
                UpdaterEvent::Progress(msg) => {
                    self.status_text = msg;
                }
                UpdaterEvent::UpdateComplete(res) => {
                    self.is_updating = false;
                    match res {
                        Ok(()) => {
                            self.status_text = "Atualização aplicada com sucesso! Reinicie o aplicativo se necessário.".to_string();
                            self.status_is_error = false;
                            self.current_version = crate::config::AppConfig::get_effective_version();
                            self.release_info = None;
                        }
                        Err(e) => {
                            self.status_text = format!("Falha na atualização: {e}");
                            self.status_is_error = true;
                        }
                    }
                }
            }
        }

        let mut should_close = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("🔄 Atualizações do BenderShot");
            ui.label(RichText::new("Verifique e instale as versões mais recentes disponíveis no GitHub").italics());
            ui.add_space(12.0);

            // Version info cards
            egui::Frame::group(ui.style())
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Versão Instalada:").strong());
                        ui.label(RichText::new(&self.current_version).color(Color32::from_rgb(53, 132, 228)));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Método de Instalação:").strong());
                        ui.label(format!("{}", self.install_method));
                    });
                });

            ui.add_space(12.0);

            // Status display
            let status_color = if self.status_is_error {
                Color32::from_rgb(224, 27, 36)
            } else if self.release_info.is_some() {
                Color32::from_rgb(46, 194, 126)
            } else {
                Color32::from_rgb(200, 200, 200)
            };

            ui.horizontal(|ui| {
                if self.is_checking || self.is_updating {
                    ui.spinner();
                }
                ui.colored_label(status_color, RichText::new(&self.status_text).strong());
            });

            ui.add_space(8.0);

            // Release details if available
            if let Some(ref release) = self.release_info {
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new(format!("📦 Novidades da Versão {}", release.tag_name)).strong());

                ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    let notes = release.body.as_deref().unwrap_or("Sem notas de versão adicionais.");
                    ui.label(notes);
                });
                ui.add_space(8.0);
            }

            ui.separator();
            ui.add_space(10.0);

            // Actions bar
            ui.horizontal(|ui| {
                if let Some(ref release) = self.release_info.clone() {
                    let update_btn = egui::Button::new(
                        RichText::new(format!("⬆️ Instalar Atualização ({})", release.tag_name)).strong(),
                    )
                    .fill(Color32::from_rgb(46, 194, 126))
                    .rounding(Rounding::same(6.0));

                    if ui.add_enabled(!self.is_updating, update_btn).clicked() {
                        self.trigger_update(release.clone());
                    }
                }

                let check_btn = egui::Button::new(RichText::new("🔍 Verificar Novamente").strong())
                    .rounding(Rounding::same(6.0));

                if ui.add_enabled(!self.is_checking && !self.is_updating, check_btn).clicked() {
                    self.is_checking = true;
                    self.status_text = "Verificando lançamentos no GitHub...".to_string();
                    self.status_is_error = false;
                    self.trigger_check();
                }

                if let Some(ref release) = self.release_info {
                    if ui.button("🌐 Ver no GitHub").clicked() {
                        crate::updater::open_url(&release.html_url);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Fechar").clicked() {
                        should_close = true;
                    }
                });
            });
        });

        if should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn run_updater_window() -> Result<(), eframe::Error> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("BenderShot - Atualização do Sistema")
        .with_app_id("bendershot")
        .with_inner_size([520.0, 380.0])
        .with_min_inner_size([460.0, 320.0]);

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
        Box::new(|_cc| Ok(Box::new(UpdaterApp::new()))),
    )
}
