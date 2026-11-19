"""anahtar_kaydet'i yeni modüle bağlar."""

import re
from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")

# 1) Modül bildirimi.
if "mod anahtar;" not in metin:
    yer = metin.index("\nmod ")
    metin = metin[:yer] + "\nmod anahtar;" + metin[yer:]

# 2) Eski fonksiyonu sil.
bas = metin.index("fn anahtar_kaydet(")
# Doc comment satırlarını da al.
geri = metin.rfind("\n\n", 0, bas)
govde_bas = bas
son = metin.index("\n}\n", bas) + 3
metin = metin[:govde_bas] + metin[son:]

# 3) Çağrı yerini güncelle.
metin = metin.replace(
    "match anahtar_kaydet(&self.anahtar_girdi) {",
    "match anahtar::kaydet(&self.anahtar_girdi) {",
)

YOL.write_text(metin, encoding="utf-8", newline="")
print("anahtar modulu baglandi")
