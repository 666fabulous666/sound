use crate::app::GuiApp;

impl GuiApp {
    pub fn start_page(&mut self, ctx: &egui::Context) {
        use egui::{Align, CornerRadius, Frame, RichText, Stroke, Vec2};
        use epaint::Margin;

        // Theme colors
        let accent = ctx.style().visuals.selection.bg_fill;
        let weak_text = ctx.style().visuals.weak_text_color();
        let sep_stroke = Stroke::new(
            1.0,
            ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
        );

        egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.set_max_width(820.0);

            let mut card = Frame::new();
            card.fill = ui.visuals().extreme_bg_color;
            card.stroke = sep_stroke;
            card.corner_radius = CornerRadius::same(16);
            card.inner_margin = Margin::same(20);
            card.outer_margin = Margin::symmetric(16, 0);

            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo {
                        let size = egui::Vec2::new(250.0, 125.0);
                        ui.image((logo.id(), size));
                    }
                    ui.vertical(|ui| {
                        ui.add_space(6.0);
                        ui.label(
                            // RichText::new("🎶 Quantum Harmonics’ Oscillator 🎶")
                            //     .size(32.0)
                            //     .strong(),
                            RichText::new("' Oscillator")
                                .size(64.0)
                                .italics()
                                .strong(),
                        );
                        ui.label(
                            RichText::new("A probability-driven music sequencer")
                                .size(20.0)
                                .color(weak_text),
                        );
                    });
                });

                ui.add_space(14.0);
                ui.separator();

                ui.add_space(14.0);
                ui.spacing_mut().item_spacing.y = 10.0;

                ui.label(
                    RichText::new("This is not a Quantum Mechanics 101 course — oh no, no.")
                        .size(18.0),
                );
                ui.label(
                    RichText::new("Here you’ll find a “quantum” music generator: a sequencer driven by randomness.")
                        .size(18.0),
                );

                ui.add_space(10.0);
                bullet(ui, "Fine-tune instruments with pitch bend, vibrato, and chorus.");
                bullet(ui, "Assign probabilities to both rhythm and harmony.");
                bullet(ui, "Jam endlessly with virtual “quantum musicians.”");

                ui.add_space(10.0);
                ui.label(
                    RichText::new("No AI: you remain the sole master of your music.")
                        .size(18.0)
                        .strong(),
                );

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Expect the unexpected. If you seek only safe and familiar sounds, you may not feel at home here.")
                        .size(17.0),
                );
                ui.label(
                    RichText::new("If you’re ready to hear the unheard, take your time, experiment freely, and let the tooltips guide you.")
                        .size(17.0),
                );

                ui.add_space(12.0);
                ui.separator();

                // CTAs
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        // Bigger, more prominent buttons
                        let btn_size = Vec2::new(200.0, 44.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Examples").size(18.0).strong(),
                                )
                                .min_size(btn_size)
                                .fill(accent)
                                .stroke(Stroke::NONE)
                                .corner_radius(12),
                            )
                            .clicked()
                        {
                            self.try_load_default(ctx);
                        }

                        if ui
                            .add(
                                egui::Button::new(RichText::new("New Score").size(18.0))
                                    .min_size(btn_size)
                                    .corner_radius(12),
                            )
                            .clicked()
                        {
                            self.new_score();
                            self.selected = None;
                            self.show_start = false;
                        }

                        if ui
                            .add(
                                egui::Button::new(RichText::new("Read Full README").size(18.0))
                                    .min_size(btn_size)
                                    .corner_radius(12),
                            )
                            .clicked()
                        {
                            self.show_doc = true;
                        }

                        if ui
                            .add(
                                egui::Button::new(RichText::new("Open GitHub").size(18.0))
                                    .min_size(btn_size)
                                    .corner_radius(12),
                            )
                            .clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(
                                "https://github.com/fmath92/sound",
                            ));
                        }
                    });
                });
            });

            ui.add_space(16.0);
        });
    });

        fn bullet(ui: &mut egui::Ui, text: impl Into<String>) {
            use egui::RichText;
            ui.horizontal(|ui| {
                ui.label(RichText::new("•").size(18.0).strong());
                ui.label(RichText::new(text.into()).size(17.0));
            });
        }
    }
}
