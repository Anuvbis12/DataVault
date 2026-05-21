import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the style Arc code with ui.spacing_mut()
old_code = """            let mut style = (*ui.style()).clone();
            style.spacing.item_spacing = egui::vec2(spacing, spacing);
            ui.set_style(style);"""

new_code = """            ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);"""

content = content.replace(old_code, new_code)

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 8 applied!")
