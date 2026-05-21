import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix focus field for setup_username
content = content.replace(
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_username)\n                                .hint_text("Min. 3 karakter")\n                                .desired_width(field_w - 80.0)\n                                .font(FontId::new(16.0, FontFamily::Proportional))\n                                .interactive(true));\n                            if resp.gained_focus() || resp.clicked() {\n                                state.focused_field = crate::app_state::FocusedField::LoginUsername;\n                                state.show_keyboard = true;\n                            }',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_username)\n                                .hint_text("Min. 3 karakter")\n                                .desired_width(field_w - 80.0)\n                                .font(FontId::new(16.0, FontFamily::Proportional))\n                                .interactive(true));\n                            if resp.gained_focus() || resp.clicked() {\n                                state.focused_field = crate::app_state::FocusedField::SetupUsername;\n                                state.show_keyboard = true;\n                            }'
)

# Fix focus field for login_password
content = content.replace(
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.login_password)\n                                    .password(true)\n                                    .hint_text("Masukkan password")\n                                    .desired_width(field_w - 80.0)\n                                    .font(FontId::new(16.0, FontFamily::Proportional))\n                                    .interactive(true));\n                                if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginUsername; state.show_keyboard = true; }',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.login_password)\n                                    .password(true)\n                                    .hint_text("Masukkan password")\n                                    .desired_width(field_w - 80.0)\n                                    .font(FontId::new(16.0, FontFamily::Proportional))\n                                    .interactive(true));\n                                if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginPassword; state.show_keyboard = true; }'
)


# Completely replace render_virtual_keyboard
# We will use regex to find and replace the whole function.

new_keyboard = """fn render_virtual_keyboard(ctx: &egui::Context, state: &mut AppState) {
    let mut close_keyboard = false;
    egui::TopBottomPanel::bottom("virtual_keyboard")
        .exact_height(320.0)
        .frame(egui::Frame::none().fill(crate::theme::bg_surface()).inner_margin(12.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔐 Secure Keyboard").color(crate::theme::text_muted()).size(14.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Tutup 🔽").size(14.0)).clicked() {
                        close_keyboard = true;
                    }
                });
            });
            ui.add_space(16.0);
            
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
            let btn_height = 48.0;

            for row in keys {
                ui.horizontal(|ui| {
                    // Calculate total width of this row to center it
                    let mut row_width = 0.0;
                    for key in row {
                        if key.is_empty() { continue; }
                        let w = if key == "DEL" || key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        row_width += w + spacing;
                    }
                    row_width -= spacing; // remove last spacing
                    
                    ui.add_space((ui.available_width() - row_width) / 2.0);
                    
                    for key in row {
                        if key.is_empty() { continue; }
                        let label = key.to_string();
                        let w = if key == "DEL" || key == "SFT" { btn_width_base * 1.5 + spacing * 0.5 } else { btn_width_base };
                        
                        let btn = egui::Button::new(egui::RichText::new(&label).size(20.0).color(crate::theme::text_body()))
                            .min_size(egui::vec2(w, btn_height))
                            .fill(crate::theme::bg_card())
                            .rounding(8.0);
                            
                        if ui.add(btn).clicked() {
                            if key == "DEL" {
                                target_str.pop();
                            } else if key != "SFT" {
                                target_str.push_str(key);
                            }
                        }
                    }
                });
                ui.add_space(spacing);
            }
            ui.add_space(spacing);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - (btn_width_base * 6.0)) / 2.0);
                let space_btn = egui::Button::new(egui::RichText::new("SPACE").size(16.0).color(crate::theme::text_body()))
                    .min_size(egui::vec2(btn_width_base * 6.0, btn_height))
                    .fill(crate::theme::bg_card())
                    .rounding(8.0);
                if ui.add(space_btn).clicked() {
                    target_str.push(' ');
                }
            });
        });

    if close_keyboard {
        state.show_keyboard = false;
        state.focused_field = crate::app_state::FocusedField::None;
    }
}"""

# regex to replace
content = re.sub(
    r'fn render_virtual_keyboard\(ctx: &egui::Context, state: &mut AppState\)\s*\{.*?(?=\n\n|\Z)',
    new_keyboard,
    content,
    flags=re.DOTALL
)

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 5 applied!")
