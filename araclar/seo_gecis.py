"""Arayüz sadeleştirmesi (tek seferlik geçiş aracı).

- `Katman` enum'unu yalnız `Kesif` + `GelenKutusu` bırakacak şekilde yazar.
- Kaldırılan sekmelerin ekran fonksiyonlarını (medya/demo/yayın) dosyadan siler.
- `katman_ekrani` yönlendirmesini iki katmana indirir.
- Üst bar düğmesini SEO motoruna bağlar.

Kullanım: python araclar/seo_gecis.py
"""

from __future__ import annotations

import pathlib
import re

KOK = pathlib.Path(__file__).resolve().parent.parent


def fn_sil(yol: str, ad: str) -> None:
    """`fn <ad>(` tanımını (yorumlarıyla) eşleşen kapanış parantezine kadar siler."""
    dosya = KOK / yol
    metin = dosya.read_text(encoding="utf-8")
    kalip = re.compile(r"^[ \t]*(?:pub(?:\(crate\))?[ \t]+)?fn[ \t]+" + re.escape(ad) + r"\(", re.M)
    eslesme = kalip.search(metin)
    if not eslesme:
        print(f"  atlandı (yok): {yol} · {ad}")
        return
    bas = metin.rfind("\n", 0, eslesme.start()) + 1
    # Tanımdan hemen önceki yorum satırları da silinsin.
    while bas > 0:
        onceki = metin.rfind("\n", 0, bas - 1) + 1
        satir = metin[onceki:bas].strip()
        if satir.startswith("///") or satir.startswith("//") or satir.startswith("#["):
            bas = onceki
        else:
            break
    i = metin.index("{", eslesme.end())
    derinlik = 0
    j = i
    while j < len(metin):
        karakter = metin[j]
        if karakter == '"':
            j += 1
            while j < len(metin) and metin[j] != '"':
                if metin[j] == "\\":
                    j += 1
                j += 1
        elif karakter == "/" and metin.startswith("//", j):
            j = metin.index("\n", j)
        elif karakter == "{":
            derinlik += 1
        elif karakter == "}":
            derinlik -= 1
            if derinlik == 0:
                break
        j += 1
    son = metin.index("\n", j) + 1
    dosya.write_text(metin[:bas] + metin[son:], encoding="utf-8")
    print(f"  silindi: {yol} · {ad}")


YENI_ENUM = '''/// Üst şeritteki iki çalışma alanı. Medya hazırlığı, demo ve yayın adımları
/// arayüzden kaldırıldı: bunları arka plandaki otomatik SEO motoru
/// (`seo_motoru`) yürütür. Kullanıcı yalnız keşif yapar ve gelen talepleri
/// yanıtlar; site/meta/şema/sitemap üretimi kendiliğinden çalışır.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Katman {
    Kesif,
    GelenKutusu,
}

impl Katman {
    const HEPSI: [Katman; 2] = [Katman::Kesif, Katman::GelenKutusu];

    fn no(self) -> &'static str {
        match self {
            Katman::Kesif => "1",
            Katman::GelenKutusu => "5",
        }
    }

    /// Günlükte ve katman geçişlerinde kullanılan uzun ad.
    fn etiket(self) -> &'static str {
        match self {
            Katman::Kesif => "Keşif & Radar",
            Katman::GelenKutusu => "Gelen Kutusu",
        }
    }

    fn kisa(self) -> &'static str {
        match self {
            Katman::Kesif => "Keşif",
            Katman::GelenKutusu => "Gelen",
        }
    }
}
'''


def enum_degistir() -> None:
    dosya = KOK / "src/main.rs"
    metin = dosya.read_text(encoding="utf-8")
    bas = metin.index("/// Üst şeritteki beş çalışma katmanı.")
    son_isaret = '/// "2 Medya" adımında yüklenebilecek dosya türü.'
    son = metin.index(son_isaret)
    metin = metin[:bas] + YENI_ENUM + "\n" + metin[son:]
    dosya.write_text(metin, encoding="utf-8")
    print("  yazıldı: src/main.rs · Katman enum (2 değer)")


def metin_degistir(yol: str, eski: str, yeni: str, ad: str) -> None:
    dosya = KOK / yol
    metin = dosya.read_text(encoding="utf-8")
    if eski not in metin:
        print(f"  BULUNAMADI: {ad}")
        return
    dosya.write_text(metin.replace(eski, yeni, 1), encoding="utf-8")
    print(f"  değişti: {ad}")


def main() -> None:
    print("1) Katman enum sadeleştiriliyor")
    enum_degistir()

    print("2) Kaldırılan ekranlar siliniyor")
    for ad in [
        "medya_ekrani",
        "onizleme_ekrani",
        "medya_demoyu_ac",
        "pipeline_lead",
        "pipeline_baslat",
        "pipeline_foto_adimi",
        "pipeline_stok_uygula",
        "pipeline_demoyu_uret",
        "yerel_ai_akisi",
    ]:
        fn_sil("src/katmanlar.rs", ad)
    for ad in ["yayin_ekrani", "temiz_derle_ve_yaz", "temiz_yayinla", "domain_sorgula"]:
        fn_sil("src/katman34.rs", ad)

    print("3) Katman yönlendirmesi iki katmana indiriliyor")
    metin_degistir(
        "src/katmanlar.rs",
        """        // Yerel AI akışı katmandan bağımsızdır: tek tuş hattı kullanıcı başka
        // sekmeye geçse de üretime devam eder.
        self.yerel_ai_akisi(ctx, gonderen);
        // Veri köprüsü: SEO sunucusu bu dosyadan okuyor.
        self.web_verisini_yaz();
        match self.katman {
            Katman::Kesif => {}
            Katman::Medya => self.medya_ekrani(ctx, gonderen),
            Katman::Onizleme => self.onizleme_ekrani(ctx, gonderen),
            Katman::Yayin => self.yayin_ekrani(ctx, gonderen),
            Katman::GelenKutusu => self.gelen_ekrani(ctx),
        }""",
        """        // Otomatik SEO motoru her karede tetiklenebilir: bağlam/medya
        // değişmediyse diske yazmaz, yalnız iki derleme döngüsünü besler.
        self.seo_motorunu_tazele();
        match self.katman {
            Katman::Kesif => {}
            Katman::GelenKutusu => self.gelen_ekrani(ctx),
        }
        let _ = gonderen;""",
        "katman_ekrani yönlendirmesi",
    )

    print("4) Üst bar: tek tuş → SEO motoru")
    metin_degistir(
        "src/katmanlar.rs",
        """                        if birincil_buton(
                            ui,
                            "TEK TUŞLA ÜRET",
                            true,
                            Some(egui::vec2(126.0, 26.0)),
                        )
                        .on_hover_text(
                            "Keşifte seçili işletmenin 2 fotoğrafını hazırlar (eksikse yerel AI üretir) ve demoyu açar; video isteğe bağlıdır",
                        )
                        .clicked()
                        {
                            self.pipeline_baslat();
                        }""",
        """                        if birincil_buton(
                            ui,
                            "SEO MOTORU",
                            true,
                            Some(egui::vec2(116.0, 26.0)),
                        )
                        .on_hover_text(
                            "Seçili işletmelerin sektör medyasını bağlar; başlık, açıklama, canonical, Open Graph, JSON-LD ve sitemap üretimini anında yayına yazar",
                        )
                        .clicked()
                        {
                            self.seo_calistir();
                        }""",
        "üst bar düğmesi",
    )
    print("bitti")


if __name__ == "__main__":
    main()
