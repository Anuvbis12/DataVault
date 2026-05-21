import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# find index of 'fn render_virtual_keyboard'
idx = content.find('fn render_virtual_keyboard')
if idx != -1:
    content = content[:idx]

new_keyboard = """fn render_virtual_keyboard(ctx: &egui::Context, state: &mut AppState) {
    let mut close_keyboard = false;
    egui::TopBottomPanel::bottom("virtual_keyboard")
        .exact_height(300.0)
        .frame(egui::Frame::none().fill(crate::theme::bg_surface()).inner_margin(egui::Margin::symmetric(8.0, 12.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔐 Secure Keyboard").color(crate::theme::text_muted()).size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Tutup 🔽").size(13.0)).clicked() {
                        close_keyboard = true;
                    }
                });
            });
            ui.add_space(12.0);
            
            let target_str = match state.focused_field {
                crate::app_state::FocusedField::LoginUsername => &mut state.login_username,
                crate::app_state::FocusedField::LoginPassword => &mut state.login_password,
                crate::app_state::FocusedField::SetupUsername => &mut state.setup_username,
                crate::app_state::FocusedField::SetupDisplayName => &mut state.setup_display_name,
                crate::app_state::FocusedField::SetupPassword => &mut state.setup_password,
                crate::app_state::FocusedField::SetupConfirmPassword => &mut state.setup_password_confirm,
                crate::app_state::FocusedField::None => { close_keyboard = true; return; }
            };

            let keys = [
                ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
                ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
                ["a", "s", "d", "f", "g", "h", "j", "k", "l", ""],
                ["SFT", "z", "x", "c", "v", "b", "n", "m", "DEL", ""],
            ];

            let spacing = 6.0;
            let btn_width_base = (ui.available_width() - (spacing * 9.0)) / 10.0;
            let btn_height = 42.0;

            // OVERRIDE STYLE SO BUTTONS DON'T EXPAND DUE TO PADDING!
            let mut style = (*ui.style()).clone();
            style.spacing.item_spacing = egui::vec2(spacing, spacing);
            style.spacing.button_padding = egui::vec2(0.0, 0.0); // CRUCIAL: Remove padding so buttons fit!
            ui.set_style(style);

            for (r_idx, row) in keys.iter().enumerate() {
                ui.horizontal(|ui| {
                    // Calculate precise row width
                    let mut row_width = 0.0;
                    for key in row {
                        if key.is_empty() { continue; }
                        let w = if *key == "DEL" || *key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        row_width += w + spacing;
                    }
                    row_width -= spacing;
                    
                    let indent = (ui.available_width() - row_width) / 2.0;
                    if indent > 1.0 {
                        ui.add_space(indent);
                    }
                    
                    for key in row {
                        if key.is_empty() { continue; }
                        let label = key.to_string();
                        let w = if *key == "DEL" || *key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        
                        let display_label = match label.as_str() {
                            "SFT" => "⇧",
                            "DEL" => "⌫",
                            _ => label.as_str(),
                        };
                        
                        let bg_color = if label == "SFT" || label == "DEL" {
                            Color32::from_rgb(45, 50, 60)
                        } else {
                            crate::theme::bg_card()
                        };

                        let btn = egui::Button::new(egui::RichText::new(display_label).size(18.0).color(Color32::WHITE))
                            .min_size(egui::vec2(w, btn_height))
                            .fill(bg_color)
                            .rounding(6.0);
                            
                        if ui.add(btn).clicked() {
                            if label == "DEL" {
                                target_str.pop();
                            } else if label != "SFT" {
                                target_str.push_str(&label);
                            }
                        }
                    }
                });
            }
            
            // SPACE BAR ROW
            ui.horizontal(|ui| {
                let space_w = btn_width_base * 5.0 + spacing * 4.0;
                let indent = (ui.available_width() - space_w) / 2.0;
                if indent > 1.0 {
                    ui.add_space(indent);
                }
                let space_btn = egui::Button::new(egui::RichText::new("SPACE").size(14.0).color(Color32::WHITE))
                    .min_size(egui::vec2(space_w, btn_height))
                    .fill(Color32::from_rgb(45, 50, 60))
                    .rounding(6.0);
                if ui.add(space_btn).clicked() {
                    target_str.push(' ');
                }
            });
        });

    if close_keyboard {
        state.show_keyboard = false;
        state.focused_field = crate::app_state::FocusedField::None;
    }
}
"""

content += new_keyboard

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch virtual keyboard applied!")
