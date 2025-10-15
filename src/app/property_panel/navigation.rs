use super::Action;
use crate::engine::score::track_node::TrackNode;
use crate::shortcuts::{
    shortcut, CLONE, DELETE, FIRST_CHILD, MUTE, PARENT, SELECT_DOWN, SELECT_UP,
};
use egui::Modifiers;

const MOD_PREFIXES: [&str; 3] = ["", "Ctrl+", "Shift+"];

fn smart_button(
    ui: &mut egui::Ui,
    variant: impl Fn(egui::Modifiers) -> (&'static str, Action),
    tooltips: &[(String, &'static str); 3], // owned Strings here
) -> Option<Action> {
    let mods = ui.input(|i| i.modifiers);
    let (label, act) = variant(mods);

    if ui
        .button(label)
        .on_hover_ui(|ui| {
            // emphasize the active row
            let active_idx = if mods.shift {
                2
            } else if mods.ctrl {
                1
            } else {
                0
            };
            for (i, (key, desc)) in tooltips.iter().enumerate() {
                let label = egui::RichText::new(format!("{} {}", key, desc));
                let txt = if i == active_idx { label } else { label.weak() };
                ui.label(txt);
            }
        })
        .clicked()
    {
        return Some(act);
    }
    None
}

fn up_variant(mods: Modifiers) -> (&'static str, Action) {
    if mods.shift {
        Action::GroupAbove.label_and_action()
    } else if mods.ctrl {
        Action::MoveUp.label_and_action()
    } else {
        Action::SelectUp.label_and_action()
    }
}
fn down_variant(mods: Modifiers) -> (&'static str, Action) {
    if mods.shift {
        Action::GroupBelow.label_and_action()
    } else if mods.ctrl {
        Action::MoveDown.label_and_action()
    } else {
        Action::SelectDown.label_and_action()
    }
}
fn parent_variant(mods: Modifiers) -> (&'static str, Action) {
    if mods.shift {
        Action::Dissolve.label_and_action()
    } else if mods.ctrl {
        Action::GroupAbove.label_and_action()
    } else {
        Action::Parent.label_and_action()
    }
}
fn first_child_variant(mods: Modifiers) -> (&'static str, Action) {
    if mods.shift {
        Action::Wrap.label_and_action()
    } else if mods.ctrl {
        Action::Promote.label_and_action()
    } else {
        Action::FirstChild.label_and_action()
    }
}

pub fn navigation(
    ui: &mut egui::Ui,
    action: &mut Action,
    edited_seq: &mut bool,
    track_node_mut: &mut TrackNode,
) {
    // Buttons row
    ui.horizontal_wrapped(|ui| {
        // Mute / Unmute stays simple
        let mute_label = if track_node_mut.is_mute() {
            "Unmute"
        } else {
            "Mute"
        };
        if ui
            .button(mute_label)
            .on_hover_text(shortcut(MUTE))
            .clicked()
            || ui.input(|i| i.key_pressed(MUTE))
        {
            track_node_mut.toggle_mute();
            *edited_seq = true;
        }

        // Delete
        if ui
            .button("Delete")
            .on_hover_text(shortcut(DELETE))
            .clicked()
            || ui.input(|i| i.key_pressed(DELETE))
        {
            *action = Action::Delete;
        }

        // Clone
        if ui.button("Clone").on_hover_text(shortcut(CLONE)).clicked()
            || ui.input(|i| i.key_pressed(CLONE))
        {
            *action = Action::Clone;
        }

        // Up
        let tips_up = make_tooltips(
            &SELECT_UP.name(),
            [
                up_variant(Modifiers::NONE).0,
                up_variant(Modifiers::SHIFT).0,
                up_variant(Modifiers::CTRL).0,
            ],
        );
        if let Some(act) = smart_button(ui, up_variant, &tips_up) {
            *action = act;
        }
        // Down
        let tips_down = make_tooltips(
            &SELECT_DOWN.name(),
            [
                down_variant(Modifiers::NONE).0,
                down_variant(Modifiers::SHIFT).0,
                down_variant(Modifiers::CTRL).0,
            ],
        );
        if let Some(act) = smart_button(ui, down_variant, &tips_down) {
            *action = act;
        }
        // Parent
        let tips_parent = make_tooltips(
            &PARENT.name(),
            [
                parent_variant(Modifiers::NONE).0,
                parent_variant(Modifiers::SHIFT).0,
                parent_variant(Modifiers::CTRL).0,
            ],
        );
        if let Some(act) = smart_button(ui, parent_variant, &tips_parent) {
            *action = act;
        }
        // First child
        let tips_first_child = make_tooltips(
            &FIRST_CHILD.name(),
            [
                first_child_variant(Modifiers::NONE).0,
                first_child_variant(Modifiers::SHIFT).0,
                first_child_variant(Modifiers::CTRL).0,
            ],
        );
        if let Some(act) = smart_button(ui, first_child_variant, &tips_first_child) {
            *action = act;
        }
    });

    // Keyboard handling (unchanged)
    let (mods, sel_up, sel_down, sel_par, sel_ch) = ui.input(|i| {
        (
            i.modifiers,
            i.key_pressed(SELECT_UP),
            i.key_pressed(SELECT_DOWN),
            i.key_pressed(PARENT),
            i.key_pressed(FIRST_CHILD),
        )
    });

    let shift = mods.shift;
    let ctrl = mods.ctrl;

    if sel_up {
        *action = if shift {
            Action::GroupAbove
        } else if ctrl {
            Action::MoveUp
        } else {
            Action::SelectUp
        };
    }
    if sel_down {
        *action = if shift {
            Action::GroupBelow
        } else if ctrl {
            Action::MoveDown
        } else {
            Action::SelectDown
        };
    }
    if sel_ch {
        *action = if shift {
            Action::Wrap
        } else if ctrl {
            Action::Promote
        } else {
            Action::FirstChild
        };
    }
    if sel_par {
        *action = if shift {
            Action::Dissolve
        } else if ctrl {
            Action::GroupAbove
        } else {
            Action::Parent
        };
    }
}
fn make_tooltips(key_name: &str, labels: [&'static str; 3]) -> [(String, &'static str); 3] {
    [
        (format!("{}{}", MOD_PREFIXES[0], key_name), labels[0]),
        (format!("{}{}", MOD_PREFIXES[1], key_name), labels[1]),
        (format!("{}{}", MOD_PREFIXES[2], key_name), labels[2]),
    ]
}
