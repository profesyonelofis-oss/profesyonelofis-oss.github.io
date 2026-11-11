"""Katman::etiket yöntemini geri koyar, OSM `etiket` yardımcısını siler."""

from pathlib import Path

YOL = Path("src/main.rs")
satirlar = YOL.read_text(encoding="utf-8").splitlines(keepends=True)

# 1) Katman::etiket yöntemini kisa()'dan önce ekle.
ANCA = "    fn kisa(self) -> &'static str {\n"
yer = next(i for i, s in enumerate(satirlar) if s == ANCA)
yeni = '''    /// Günlükte ve katman geçişlerinde kullanılan uzun ad.
    fn etiket(self) -> &'static str {
        match self {
            Katman::Kesif => "Keşif & Radar",
            Katman::Onizleme => "Demo Önizleme",
            Katman::Yayin => "Domain & Yayın",
            Katman::GelenKutusu => "Gelen Kutusu",
        }
    }

'''
satirlar[yer:yer] = [yeni]

# 2) Üst düzey OSM `etiket` yardımcısını sil.
bas = next(i for i, s in enumerate(satirlar) if s.startswith("fn etiket<'a>"))
son = next(i for i in range(bas, len(satirlar)) if satirlar[i].rstrip("\r\n") == "}")
del satirlar[bas : son + 1]

# 3) Kalan kaynak doc satırını temizle.
satirlar = [
    s for s in satirlar if s.strip() != "/// Seçili arama kaynağı (OSM / Google / Hibrit)."
]

YOL.write_text("".join(satirlar), encoding="utf-8", newline="")
print("katman etiketi geri eklendi, satir:", len(satirlar))
