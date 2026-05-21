import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Let's see where the end of `pub fn render` is.
# `render_dashboard` is the next function.
# Let's insert the keyboard call right before `pub fn render_dashboard`

content = re.sub(
    r'(    \}\n\n)\s*(pub fn render_dashboard\()',
    r'\1    if state.show_keyboard {\n        render_virtual_keyboard(ctx, state);\n    }\n}\n\n\2',
    content
)

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 4 applied!")
