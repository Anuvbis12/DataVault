import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Fix Virtual Keyboard spacing and alignment
# We need to temporarily override the item_spacing for the horizontal layouts in the keyboard
# so that our exact manual calculations hold true.

old_keyboard_setup = """            let spacing = 6.0;
            let btn_width_base = (ui.available_width() - (spacing * 9.0)) / 10.0;
            let btn_height = 48.0;

            for row in keys {"""

new_keyboard_setup = """            let spacing = 6.0;
            let btn_width_base = (ui.available_width() - (spacing * 9.0)) / 10.0;
            let btn_height = 48.0;

            let mut style = (*ui.style()).clone();
            style.spacing.item_spacing = egui::vec2(spacing, spacing);
            ui.set_style(style);

            for row in keys {"""

content = content.replace(old_keyboard_setup, new_keyboard_setup)


# 2. Add ScrollArea to render_setup_account
# Replace ui.allocate_ui_at_rect(avail, |ui| { with egui::ScrollArea::vertical().show(ui, |ui| {
# But we need to make sure we don't accidentally replace the closing brace wrongly.

old_setup = """    ui.allocate_ui_at_rect(avail, |ui| {
        let y_padding = (avail.height() - 480.0).max(0.0) / 2.0;
        ui.add_space(y_padding.max(32.0));"""

new_setup = """    egui::ScrollArea::vertical().show(ui, |ui| {
        let y_padding = (avail.height() - 480.0).max(0.0) / 2.0;
        ui.add_space(y_padding.max(32.0));"""

content = content.replace(old_setup, new_setup)


# 3. Add ScrollArea to render_login
old_login = """    ui.allocate_ui_at_rect(avail, |ui| {
        ui.vertical_centered(|ui| {
            let content_h = if user_set { 380.0 } else { 200.0 };
            let y_padding = (avail.height() - content_h).max(0.0) / 2.0;
            ui.add_space(y_padding.max(40.0));"""

new_login = """    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            let content_h = if user_set { 380.0 } else { 200.0 };
            let y_padding = (avail.height() - content_h).max(0.0) / 2.0;
            ui.add_space(y_padding.max(40.0));"""

content = content.replace(old_login, new_login)


with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 7 applied!")
