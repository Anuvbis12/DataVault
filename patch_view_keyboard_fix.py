import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_code = """            // OVERRIDE STYLE SO BUTTONS DON'T EXPAND DUE TO PADDING!
            let mut style = (*ui.style()).clone();
            style.spacing.item_spacing = egui::vec2(spacing, spacing);
            style.spacing.button_padding = egui::vec2(0.0, 0.0); // CRUCIAL: Remove padding so buttons fit!
            ui.set_style(style);

            for (r_idx, row) in keys.iter().enumerate() {"""

new_code = """            // OVERRIDE STYLE SO BUTTONS DON'T EXPAND DUE TO PADDING!
            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0); // CRUCIAL: Remove padding so buttons fit!

            for (_r_idx, row) in keys.iter().enumerate() {"""

content = content.replace(old_code, new_code)

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch virtual keyboard fixed applied!")
