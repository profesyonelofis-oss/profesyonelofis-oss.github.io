"""main.rs süslü parantez dengesini tarar: bozulan blok nerede?"""

import re
from pathlib import Path

metin = Path("src/main.rs").read_text(encoding="utf-8")
satirlar = metin.split("\n")

# Yorumları ve dizgileri maskele (satır yapısını koru).
temiz = []
for satir in satirlar:
    # satır yorumu
    if "//" in satir:
        tirnak = satir.find('"')
        if tirnak < 0 or satir.find("//") < tirnak:
            satir = satir[: satir.find("//")]
    temiz.append(satir)

s = "\n".join(temiz)
# dizgileri boşalt (kaçışlı tırnaklar dahil)
s = re.sub(r'r#*"(?:\\.|[^"\\])*"#*', '""', s)
s = re.sub(r'"(?:\\.|[^"\\])*"', '""', s)
# karakter sabitleri
s = re.sub(r"'(?:\\.|[^'\\])'", "''", s)

derinlik = 0
rapor = []
for no, satir in enumerate(s.split("\n"), 1):
    kirp = satir.strip()
    if kirp.startswith(("fn ", "pub fn ", "pub(crate) fn ", "impl ", "mod ", "struct ", "enum ")):
        rapor.append((no, derinlik, satirlar[no - 1].strip()[:70]))
    derinlik += satir.count("{") - satir.count("}")

print("son derinlik:", derinlik)
print("--- yapı satırları (satır, derinlik, metin) ---")
for no, d, m in rapor:
    print(f"{no:6d} d={d:2d}  {m}")
