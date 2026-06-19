import sys
import re

if len(sys.argv) < 2:
    sys.exit(1)

version = sys.argv[1].strip()

def extract(filepath, ver):
    clean_ver = re.sub(r'^[vV]', '', ver)
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except FileNotFoundError:
        return None
    found = False
    notes = []
    headers = [f"### v{clean_ver}", f"### {clean_ver}"]
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('###'):
            if found:
                break
            if any(stripped.startswith(h) for h in headers):
                found = True
                continue
        if found:
            notes.append(line)
    if not found:
        return None
    return "".join(notes).strip()

en_notes = extract('Changelog.md', version)
zh_notes = extract('Changelog-zh.md', version)

if not en_notes and not zh_notes:
    print(f"Error: Version {version} not found in Changelog.md or Changelog-zh.md", file=sys.stderr)
    sys.exit(1)

combined = []
if en_notes:
    combined.append(en_notes)
if zh_notes:
    combined.append(zh_notes)

content = "\n\n---\n\n".join(combined).strip()
print("Combined release notes:")
print(content)

with open('release_notes.txt', 'w', encoding='utf-8') as f:
    f.write(content)
