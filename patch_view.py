import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Center Login
content = content.replace(
    '        ui.vertical_centered(|ui| {\n            ui.add_space(44.0);\n\n            // Shield icon',
    '        ui.vertical_centered(|ui| {\n            let content_h = if user_set { 380.0 } else { 200.0 };\n            ui.add_space((avail.height() - content_h).max(0.0) / 2.0);\n\n            // Shield icon'
)

# 2. Center Setup
content = content.replace(
    '    ui.allocate_ui_at_rect(avail, |ui| {\n        ui.add_space(32.0);\n\n        ui.horizontal(|ui| {',
    '    ui.allocate_ui_at_rect(avail, |ui| {\n        let y_padding = (avail.height() - 480.0).max(0.0) / 2.0;\n        ui.add_space(y_padding.max(32.0));\n\n        ui.horizontal(|ui| {'
)

# 3. Add focused_field states to TextEdits
# login_username
content = content.replace(
    '.font(FontId::new(16.0, FontFamily::Proportional))\n                                    .frame(false));',
    '.font(FontId::new(16.0, FontFamily::Proportional))\n                                    .interactive(true));\n                                if resp.gained_focus() || resp.clicked() { state.focused_field = crate::app_state::FocusedField::LoginUsername; state.show_keyboard = true; }'
)
# We need to capture `ui.add` into `let resp = ui.add` for login_username
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.login_username)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.login_username)'
)

# login_password
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.login_password)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.login_password)'
)
# setup_username
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.setup_username)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_username)'
)
# setup_display_name
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.setup_display_name)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_display_name)'
)
# setup_password
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.setup_password)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password)'
)
# setup_password_confirm
content = content.replace(
    'ui.add(egui::TextEdit::singleline(&mut state.setup_password_confirm)',
    'let resp = ui.add(egui::TextEdit::singleline(&mut state.setup_password_confirm)'
)

# Now fix the .frame(false) -> .interactive(true) for the rest.
# Note: since they all look identical, we can use regex or just target the specific blocks.
# Actually, the easiest is to just use regex to replace all `let resp = ui.add(egui::TextEdit... .frame(false));`
content = re.sub(
    r'(let resp = ui\.add\(egui::TextEdit::singleline\(&mut state\.(.*?)\)[\s\S]*?)\.frame\(false\)\);',
    r'\1.interactive(true));\n                            if resp.gained_focus() || resp.clicked() {\n                                state.focused_field = crate::app_state::FocusedField::\2;\n                                state.show_keyboard = true;\n                            }',
    content
)

# Fix enum names for focused_field mappings
content = content.replace('FocusedField::login_username', 'FocusedField::LoginUsername')
content = content.replace('FocusedField::login_password', 'FocusedField::LoginPassword')
content = content.replace('FocusedField::setup_username', 'FocusedField::SetupUsername')
content = content.replace('FocusedField::setup_display_name', 'FocusedField::SetupDisplayName')
content = content.replace('FocusedField::setup_password', 'FocusedField::SetupPassword')
content = content.replace('FocusedField::setup_password_confirm', 'FocusedField::SetupConfirmPassword')


# 4. Add render_virtual_keyboard at the end
keyboard_code = """

    // Overlay Virtual Keyboard (Secure Keyboard)
    if state.show_keyboard {
        render_virtual_keyboard(ctx, state);
    }
}

// ── VIRTUAL SECURE KEYBOARD ──────────────────────────────────
fn render_virtual_keyboard(ctx: &egui::Context, state: &mut AppState) {
    let mut close_keyboard = false;
    egui::TopBottomPanel::bottom("virtual_keyboard")
        .exact_height(260.0)
        .frame(egui::Frame::none().fill(bg_surface()).inner_margin(8.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔒 Secure Keyboard").color(text_muted()).size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Tutup ⬇").clicked() {
                        close_keyboard = true;
                    }
                });
            });
            ui.add_space(8.0);
            
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

            let btn_size = egui::vec2((ui.available_width() - 40.0) / 10.0, 40.0);
            for row in keys {
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - (row.iter().filter(|c| !c.is_empty()).count() as f32 * (btn_size.x + 4.0))) / 2.0);
                    for key in row {
                        if key.is_empty() { continue; }
                        let mut label = key.to_string();
                        let mut w = btn_size.x;
                        if key == "DEL" || key == "SFT" { w = btn_size.x * 1.5; }
                        
                        let btn = egui::Button::new(egui::RichText::new(&label).size(18.0).color(text_body()))
                            .min_size(egui::vec2(w, btn_size.y))
                            .fill(bg_elevated())
                            .rounding(6.0);
                            
                        if ui.add(btn).clicked() {
                            if key == "DEL" {
                                target_str.pop();
                            } else if key != "SFT" {
                                target_str.push_str(key);
                            }
                        }
                    }
                });
                ui.add_space(4.0);
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(btn_size.x * 2.0);
                if ui.add(egui::Button::new(egui::RichText::new("SPACE").size(16.0).color(text_body()))
                    .min_size(egui::vec2(ui.available_width() - btn_size.x * 4.0, btn_size.y))
                    .fill(bg_elevated())
                    .rounding(6.0)).clicked() {
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

content = re.sub(r'    \}\n\}$', keyboard_code, content)

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch applied!")
