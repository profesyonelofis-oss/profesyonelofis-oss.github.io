"""SEKTORLER katalogunda kalan OSM satir artiklarini temizler."""

from pathlib import Path

YOL = Path("src/main.rs")
satirlar = YOL.read_text(encoding="utf-8").splitlines(keepends=True)

bas = next(i for i, s in enumerate(satirlar) if s.startswith("const SEKTORLER"))
son = next(i for i in range(bas, len(satirlar)) if satirlar[i].rstrip() == "];")

izinli = ("Sektor {", "ad:", "grup:", "demo:", "},", "//", "const SEKTORLER")
temiz = []
atilan = 0
for satir in satirlar[bas + 1 : son]:
    if satir.strip() == "" or satir.strip().startswith(izinli):
        temiz.append(satir)
    else:
        atilan += 1

satirlar[bas + 1 : son] = temiz
YOL.write_text("".join(satirlar), encoding="utf-8")
print("atilan satir:", atilan)
