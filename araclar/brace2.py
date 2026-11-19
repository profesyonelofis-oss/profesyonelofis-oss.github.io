"""main.rs süslü parantez dengesi: gerçek Rust sözdizimi taraması."""

import re
from pathlib import Path

s = Path("src/main.rs").read_text(encoding="utf-8")
n = len(s)
i = 0
satir = 1
derinlik = 0
# (satır, derinlik) — üst düzey ve modül içi fn başlangıçları
yapi = []
anomali = []
while i < n:
    c = s[i]
    if c == "\n":
        satir += 1
        i += 1
        continue
    if c == "/" and i + 1 < n and s[i + 1] == "/":
        j = s.find("\n", i)
        i = n if j < 0 else j
        continue
    if c == "/" and i + 1 < n and s[i + 1] == "*":
        j = s.find("*/", i + 2)
        if j < 0:
            break
        satir += s.count("\n", i, j)
        i = j + 2
        continue
    # ham dizgi: r"..." / r#"..."# / br#"..."#
    m = re.match(r'(?:b)?r(#*)"', s[i:])
    if m:
        kapat = '"' + m.group(1)
        j = s.find(kapat, i + m.end())
        if j < 0:
            break
        satir += s.count("\n", i, j)
        i = j + len(kapat)
        continue
    if c == '"' or (c == "b" and i + 1 < n and s[i + 1] == '"'):
        if c == "b":
            i += 1
        i += 1
        while i < n:
            if s[i] == "\\":
                i += 2
                continue
            if s[i] == '"':
                i += 1
                break
            if s[i] == "\n":
                satir += 1
            i += 1
        continue
    if c == "'":
        # karakter sabiti mi, ömür mü?
        if i + 2 < n and s[i + 2] == "'":
            i += 3
            continue
        if i + 3 < n and s[i + 1] == "\\" and s[i + 3] == "'":
            i += 4
            continue
        i += 1
        continue
    if c == "{":
        derinlik += 1
    elif c == "}":
        derinlik -= 1
        if derinlik < 0:
            anomali.append((satir, derinlik))
            derinlik = 0
    i += 1

for no, metin in enumerate(s.split("\n"), 1):
    kirp = metin.strip()
    if re.match(r"^(pub(\(crate\))? )?fn \w+", kirp) or kirp.startswith(("impl ", "mod ")):
        beklenen = 1 if metin.startswith("    ") and not metin.startswith("        ") else 0
        yapi.append((no, kirp[:60]))
print("son derinlik:", derinlik)
for no, metin in yapi:
    print(f"{no:6d} {metin}")
print("negatif:", anomali[:10])
