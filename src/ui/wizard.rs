use crate::config::AppConfig;
use crate::shortcuts::ShortcutManager;
use crate::ui::load_app_icon;
use crate::ui::theme::{apply_adwaita_theme, AdwaitaThemeMode};
use egui::{Color32, RichText, Rounding};

pub struct SetupWizardApp {
    replace_print: bool,
    enable_record: bool,
    enable_autostart: bool,
    integrate_appimage: bool,
    is_appimage: bool,
    status_message: Option<(String, bool, std::time::Instant)>,
    theme_mode: AdwaitaThemeMode,
}

impl SetupWizardApp {
    pub fn new() -> Self {
        let is_appimage = ShortcutManager::is_running_as_appimage();
        Self {
            replace_print: true,
            enable_record: true,
            enable_autostart: true,
            integrate_appimage: is_appimage,
            is_appimage,
            status_message: None,
            theme_mode: AdwaitaThemeMode::Dark,
        }
    }

    fn apply_settings(&mut self, ctx: &egui::Context) {
        let mut errors = Vec::new();

        if self.replace_print {
            if let Err(e) = ShortcutManager::set_binding("Print") {
                errors.push(format!("Atalho Print: {}", e));
            }
        }

        if self.enable_record {
            if let Err(e) = ShortcutManager::set_record_binding("<Ctrl><Shift>R") {
                errors.push(format!("Atalho Gravação: {}", e));
            }
        }

        if self.enable_autostart {
            if let Err(e) = ShortcutManager::configure_autostart(true) {
                errors.push(format!("Autostart: {}", e));
            }
            if let Err(e) = ShortcutManager::start_daemon_if_not_running() {
                errors.push(format!("Daemon: {}", e));
            }
        }

        if self.is_appimage && self.integrate_appimage {
            if let Err(e) = ShortcutManager::integrate_appimage() {
                errors.push(format!("Integração AppImage: {}", e));
            }
        }

        AppConfig::mark_setup_completed();

        if errors.is_empty() {
            ShortcutManager::send_notification(
                "BenderShot Configurado",
                "Configuração concluída com sucesso! Pressione PrintScreen para capturar.",
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            self.status_message = Some((
                format!("Avisos ao configurar:\n{}", errors.join("\n")),
                false,
                std::time::Instant::now(),
            ));
        }
    }
}

impl eframe::App for SetupWizardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_adwaita_theme(ctx, self.theme_mode);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            // Header Section
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("🎯 Bem-vindo ao BenderShot!")
                        .size(22.0)
                        .strong()
                        .color(Color32::from_rgb(255, 255, 255)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Assistente de Configuração Inicial e Atalhos Globais")
                        .size(13.0)
                        .color(Color32::from_rgb(180, 180, 190)),
                );
            });

            ui.add_space(16.0);

            // Options Group Box
            egui::Frame::none()
                .fill(Color32::from_rgb(38, 38, 46))
                .rounding(Rounding::same(8.0))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // Option 1: PrintScreen Key
                    ui.checkbox(
                        &mut self.replace_print,
                        RichText::new("📸 Substituir a tecla PrintScreen padrão").strong().size(14.0),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("     Desvincula o print nativo do GNOME e abre a captura com anotações vetoriais do BenderShot.")
                            .size(11.5)
                            .color(Color32::from_rgb(170, 170, 180)),
                    );

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Option 2: Record Shortcut
                    ui.checkbox(
                        &mut self.enable_record,
                        RichText::new("🎬 Atalho para Gravação de Vídeo (<Ctrl><Shift>R)").strong().size(14.0),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("     Grava vídeo da tela em alta resolução com editor de cortes integrado.")
                            .size(11.5)
                            .color(Color32::from_rgb(170, 170, 180)),
                    );

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Option 3: Autostart & Daemon
                    ui.checkbox(
                        &mut self.enable_autostart,
                        RichText::new("🚀 Iniciar automaticamente com o sistema (Modo Daemon)").strong().size(14.0),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("     Mantém o ícone no painel superior/bandeja para resposta instantânea.")
                            .size(11.5)
                            .color(Color32::from_rgb(170, 170, 180)),
                    );

                    // Option 4: AppImage integration (only if running as AppImage)
                    if self.is_appimage {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.checkbox(
                            &mut self.integrate_appimage,
                            RichText::new("📦 Integrar AppImage ao sistema operacional").strong().size(14.0),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("     Cria lançador no menu de aplicativos e configura ~/.local/bin/bendershot.")
                                .size(11.5)
                                .color(Color32::from_rgb(170, 170, 180)),
                        );
                    }
                });

            ui.add_space(16.0);

            // Status message if any
            if let Some((ref msg, success, _)) = self.status_message {
                let color = if success {
                    Color32::from_rgb(87, 227, 137)
                } else {
                    Color32::from_rgb(255, 120, 120)
                };
                ui.label(RichText::new(msg).color(color).size(12.0));
                ui.add_space(8.0);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(
                        egui::Button::new(
                            RichText::new("Pular")
                                .size(13.0)
                                .color(Color32::from_rgb(200, 200, 200)),
                        )
                        .min_size(egui::vec2(90.0, 36.0)),
                    ).clicked() {
                        AppConfig::mark_setup_completed();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = egui::Button::new(
                            RichText::new("✔ Aplicar Configurações e Iniciar")
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(53, 132, 228)) // Adwaita blue
                        .min_size(egui::vec2(240.0, 36.0));

                        if ui.add(btn).clicked() {
                            self.apply_settings(ctx);
                        }
                    });
                });
            });
        });
    }
}

pub fn run_wizard_window() -> anyhow::Result<()> {
    let icon_data = load_app_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("BenderShot - Assistente de Configuração")
        .with_inner_size([540.0, 500.0])
        .with_resizable(false);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "BenderShot - Assistente de Configuração",
        native_options,
        Box::new(|_cc| Ok(Box::new(SetupWizardApp::new()))),
    )
    .map_err(|e| anyhow::anyhow!("{e:?}"))
}
