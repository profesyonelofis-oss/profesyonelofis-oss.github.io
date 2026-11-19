"""Arayüz sadeleştirmesi, 3. adım: kaldırılan sekmelere ait WebView2 medya
panellerini ve demo hattı kalıntılarını temizler."""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from seo_gecis2 import degistir, fn_sil  # noqa: E402

print("1) WebView2 medya panelleri siliniyor")
fn_sil("src/katmanlar.rs", "medya_tarayici_isle")
fn_sil("src/katmanlar.rs", "medya_bos_panel")

print("2) main.rs: alt pencere cagrisi kaldiriliyor")
degistir(
    "src/main.rs",
    """        // Katmandan Keşif'e dönülürse WebView2 alt penceresi görünür
        // kalmasın; diğer katmanlarda da pencere kapanır.
        #[cfg(target_os = "windows")]
        self.medya_tarayici_isle(ctx, frame);
""",
    "",
    "medya_tarayici_isle cagrisi",
)

print("3) medya_isle: demo hatti kalintisi kaldiriliyor")
degistir(
    "src/katmanlar.rs",
    """                                if secim.hazir() {
                                    self.pipeline_stok_uygula(idx, &secim);
                                    if self.medya_oto_demo || self.pipeline_aktif {
                                        self.medya_oto_ac = true;
                                        self.medya_oto_hedef = hedef.to_string().into();
                                    }
                                }""",
    """                                let _ = idx;
                                // Demo hattı kaldırıldı: onaylanan dosya
                                // doğrudan SEO motorunun arşivine düşer.
                                self.seo_calistir();""",
    "medya_isle demo blogu",
)

print("4) mod seo_motoru ekleniyor")
dosya = pathlib.Path("src/main.rs")
metin = dosya.read_text(encoding="utf-8")
if "mod seo_motoru;" not in metin:
    metin = metin.replace("mod katmanlar;", "mod katmanlar;\nmod seo_motoru;", 1)
    dosya.write_text(metin, encoding="utf-8")
    print("  eklendi: mod seo_motoru;")
else:
    print("  zaten var")
print("bitti")
