import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Fix user_set in audit_logs
content = content.replace(
    '            let content_h = if user_set { 380.0 } else { 200.0 };\n            ui.add_space((avail.height() - content_h).max(0.0) / 2.0);\n            ui.label(egui::RichText::new("Belum ada catatan aktivitas.").color(text_muted()));',
    '            ui.add_space(20.0);\n            ui.label(egui::RichText::new("Belum ada catatan aktivitas.").color(text_muted()));'
)

# 2. Move keyboard render out of render_system_trash and into render()
# First, remove it from the end of render_system_trash
content = content.replace(
    '    // Overlay Virtual Keyboard (Secure Keyboard)\n    if state.show_keyboard {\n        render_virtual_keyboard(ctx, state);\n    }\n}',
    '}'
)

# Then inject it into render() correctly. render() ends with checking auth, then the TopBottomPanel logic
# We'll just look for `pub fn render(` and its end. Actually, the easiest is to put it right before `pub fn render_dashboard`.
content = content.replace(
    'pub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {',
    '    // Overlay Virtual Keyboard (Secure Keyboard)\n    if state.show_keyboard {\n        render_virtual_keyboard(ctx, state);\n    }\n}\n\npub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {'
)

# We also need to remove the extra closing brace from `render()` since we injected the keyboard block + `}` 
# Wait, let's just do it manually in the file by looking at `pub fn render_dashboard`.
# Wait, `render()` is:
# ```
#     if !state.is_authenticated {
#         egui::CentralPanel::default()
# ...
#     } else {
#         render_dashboard(ui, state, ctrl); ...
#     }
# }
# ```
content = content.replace(
    '        });\n    }\n}\n\npub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {',
    '        });\n    }\n\n    // Overlay Virtual Keyboard (Secure Keyboard)\n    if state.show_keyboard {\n        render_virtual_keyboard(ctx, state);\n    }\n}\n\npub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState, ctrl: &Controller) {'
)

# 3. Fix SetupPassword_confirm
content = content.replace(
    'FocusedField::SetupPassword_confirm',
    'FocusedField::SetupConfirmPassword'
)

# 4. Fix bg_elevated() -> crate::theme::bg_elevated()
content = content.replace(
    '.fill(bg_elevated())',
    '.fill(crate::theme::bg_elevated())'
)
content = content.replace(
    '.color(text_body())',
    '.color(crate::theme::text_body())'
)
content = content.replace(
    '.color(text_muted())',
    '.color(crate::theme::text_muted())'
)
content = content.replace(
    '.fill(bg_surface())',
    '.fill(crate::theme::bg_surface())'
)


with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 2 applied!")
