"""Arayüz sadeleştirmesi, 2. adım: artık `impl Katman` bloğunu, demo
pipeline kalıntılarını ve üst bar düğmesini temizler; SEO motoru
yöntemlerini `LeadApp`'e ekler."""

from __future__ import annotations

import pathlib
import re

KOK = pathlib.Path(__file__).resolve().parent.parent


def blok_sil(yol: str, baslangic: str) -> None:
    dosya = KOK / yol
    metin = dosya.read_text(encoding="utf-8")
    if baslangic not in metin:
        print("  yok:", baslangic)
        return
    i = metin.index(baslangic)
    bas = metin.rfind("\n", 0, i) + 1
    sus = metin.index("{", i)
    derinlik = 0
    j = sus
    while j < len(metin):
        if metin[j] == '"':
            j += 1
            while j < len(metin) and metin[j] != '"':
                if metin[j] == "\\":
                    j += 1
                j += 1
        elif metin[j] == "/" and metin.startswith("//", j):
            j = metin.index("\n", j)
        elif metin[j] == "{":
            derinlik += 1
        elif metin[j] == "}":
            derinlik -= 1
            if derinlik == 0:
                break
        j += 1
    son = metin.index("\n", j) + 1
    dosya.write_text(metin[:bas] + metin[son:], encoding="utf-8")
    print("  silindi:", yol, baslangic[:40])


def fn_sil(yol: str, ad: str) -> None:
    dosya = KOK / yol
    metin = dosya.read_text(encoding="utf-8")
    kalip = re.compile(r"^[ \t]*(?:pub(?:\(crate\))?[ \t]+)?fn[ \t]+" + re.escape(ad) + r"\(", re.M)
    eslesme = kalip.search(metin)
    if not eslesme:
        print("  yok:", yol, ad)
        return
    bas = metin.rfind("\n", 0, eslesme.start()) + 1
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
        if metin[j] == '"':
            j += 1
            while j < len(metin) and metin[j] != '"':
                if metin[j] == "\\":
                    j += 1
                j += 1
        elif metin[j] == "/" and metin.startswith("//", j):
            j = metin.index("\n", j)
        elif metin[j] == "{":
            derinlik += 1
        elif metin[j] == "}":
            derinlik -= 1
            if derinlik == 0:
                break
        j += 1
    son = metin.index("\n", j) + 1
    dosya.write_text(metin[:bas] + metin[son:], encoding="utf-8")
    print("  silindi:", yol, ad)


def degistir(yol: str, eski: str, yeni: str, ad: str) -> None:
    dosya = KOK / yol
    metin = dosya.read_text(encoding="utf-8")
    if eski not in metin:
        print("  BULUNAMADI:", ad)
        return
    dosya.write_text(metin.replace(eski, yeni, 1), encoding="utf-8")
    print("  degisti:", ad)


SEO_YONTEMLERI = '''    /// Otomatik SEO motorunun tek çalıştırması: bağlamı kurar, sektör
    /// medyasını bağlar, meta/şema üretir ve yayın dosyasını tazeler.
    pub(crate) fn seo_calistir(&mut self) {
        let (il, ilce) = self.seo_konumu();
        let ozet = crate::seo_motoru::uygula(&mut self.leads, &il, &ilce);
        self.web_verisini_yaz();
        let satir = format!(
            "[✓] SEO motoru: {} site · {} medya bağı · başlık/açıklama/JSON-LD hazır · sitemap tazelendi",
            ozet.siteler, ozet.medya
        );
        if ozet.siteler == 0 {
            self.logs.push(
                "[i] SEO motoru: sektör arşivinde (media/stock/<sektör>/) medya yok; üretim bekliyor."
                    .to_string(),
            );
        } else {
            self.logs.push(satir);
        }
    }

    /// Her karede çalışan hafif sürüm: motor içerik parmak izini kendi tuttuğu
    /// için değişiklik yoksa diske yazılmaz.
    pub(crate) fn seo_motorunu_tazele(&mut self) {
        self.web_verisini_yaz();
    }

    /// SEO bağlamı: panelde seçili il/ilçe.
    fn seo_konumu(&self) -> (String, String) {
        let il = self
            .cities
            .get(self.selected_city_idx)
            .map(|sehir| sehir.0.to_string())
            .unwrap_or_default();
        let ilce = self
            .cities
            .get(self.selected_city_idx)
            .and_then(|sehir| sehir.1.get(self.selected_district_idx).copied())
            .map(|ilce| ilce.to_string())
            .unwrap_or_default();
        (il, ilce)
    }

'''


def main() -> None:
    print("1) Eski impl Katman blogu siliniyor")
    blok_sil("src/main.rs", "impl Katman {\n    const HEPSI: [Katman; 5]")

    print("2) Demo pipeline kalintilari siliniyor")
    for ad in ["pipeline_demoyu_hazirla", "medya_ureticileri", "yerel_ai_paneli"]:
        fn_sil("src/katmanlar.rs", ad)

    print("3) Ust bar dugmesi SEO motoruna baglaniyor")
    degistir(
        "src/katmanlar.rs",
        '                            "TEK TUSLA URET",',
        '                            "SEO MOTORU",',
        "dugme etiketi",
    )
    degistir(
        "src/katmanlar.rs",
        '"TEK TUŞLA ÜRET",',
        '"SEO MOTORU",',
        "dugme etiketi (utf8)",
    )
    degistir(
        "src/katmanlar.rs",
        "                            self.pipeline_baslat();",
        "                            self.seo_calistir();",
        "dugme cagrisi",
    )

    print("4) SEO motoru yontemleri ekleniyor")
    degistir(
        "src/katmanlar.rs",
        "    pub(crate) fn web_verisini_yaz(&self) {",
        SEO_YONTEMLERI + "    pub(crate) fn web_verisini_yaz(&self) {",
        "seo yontemleri",
    )
    print("bitti")


if __name__ == "__main__":
    main()
