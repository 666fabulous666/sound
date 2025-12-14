use super::super::ParameterImpact;
use crate::app::property_panel::helpers::ParameterBehavior;
use crate::engine::score::{track_node::TrackNode, DelayChannel, DelayTap};
use egui::Ui;

pub fn show_delay_section(ui: &mut Ui, track_node: &mut TrackNode, impact: &mut ParameterImpact) {
    ui.heading("Delay");
    ui.label("Delays are measured in beats and applied per stereo channel.");
    ui.add_space(4.0);

    let mut changed = false;
    ui.columns(2, |cols| {
        cols[0].vertical(|ui_left| {
            ui_left.label("Left");
            changed |= edit_delay_channel(ui_left, &mut track_node.delays.left);
        });
        cols[1].vertical(|ui_right| {
            ui_right.label("Right");
            changed |= edit_delay_channel(ui_right, &mut track_node.delays.right);
        });
    });
    if changed {
        impact.register_behavior(ParameterBehavior::Mix);
    }
}

fn edit_delay_channel(ui: &mut Ui, channel: &mut DelayChannel) -> bool {
    let mut changed = false;
    {
        let mut dry = channel.dry.unwrap_or(0.0);
        let dry_resp = ui
            .add(
                egui::Slider::new(&mut dry, 0.0..=1.0)
                    .text("Dry")
                    .clamping(egui::SliderClamping::Always),
            )
            .on_hover_text("Dry factor for this channel");
        if dry_resp.changed() {
            channel.dry = Some(dry.clamp(0.0, 1.0));
            changed = true;
        }
    }
    ui.separator();

    let taps = &mut channel.taps;
    let initial_len = taps.len();
    taps.retain_mut(|tap| {
        let mut keep = true;
        ui.horizontal(|ui| {
            let beat_response = ui
                .add(
                    egui::DragValue::new(&mut tap.beat)
                        .range(0.0..=32.0)
                        .speed(0.01),
                )
                .on_hover_text("Delay in beats");
            let mut weight = tap.weight.max(0.0);
            let weight_response = ui
                .add(
                    egui::DragValue::new(&mut weight)
                        .range(0.0..=10.0)
                        .speed(0.05),
                )
                .on_hover_text("Relative wet weight");
            changed |= beat_response.changed() || weight_response.changed();
            if weight_response.changed() {
                tap.weight = weight.max(0.0);
            }
            if ui
                .small_button("✖")
                .on_hover_text("Remove delay")
                .clicked()
            {
                keep = false;
                changed = true;
            }
        });
        keep
    });
    if taps.len() != initial_len {
        changed = true;
    }
    if ui.button("+").on_hover_text("Add a delay tap").clicked() {
        taps.push(DelayTap {
            beat: 0.0,
            weight: 1.0,
        });
        changed = true;
    }
    changed
}
