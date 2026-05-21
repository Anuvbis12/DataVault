import re

with open('src/view.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('crate::theme::bg_elevated()', 'crate::theme::bg_card()')

with open('src/view.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch 3 applied!")
