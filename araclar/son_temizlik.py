"""Kalan OSM testlerini siler, ölü alanları ve kullanılmayan fn'i kaldırır."""

import re
from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")
satirlar = metin.splitlines(keepends=True)


def blok_sil(desen: str) -> bool:
    """desen ile eşleşen satırdan, aynı girintideki kapanış satırına kadar siler."""
    kalip = re.compile(desen)
    for i, satir in enumerate(satirlar):
        if not kalip.match(satir):
            continue
        girinti = len(satir) - len(satir.lstrip())
        bas = i
        while bas > 0 and (
            satirlar[bas - 1].strip().startswith("#[")
            or satirlar[bas - 1].strip().startswith("///")
        ):
            bas -= 1
        son = None
        for j in range(i + 1, len(satirlar)):
            if satirlar[j].rstrip("\r\n") == " " * girinti + "}":
                son = j
                break
        if son is None:
            raise SystemExit(f"kapanis yok: {desen}")
        del satirlar[bas : son + 1]
        return True
    return False


for ad in (
    "web_sitesi_olan_isletmeler_atlanir",
    "telefonsuz_kayit_musteri_adayi_degildir",
    "sosyal_medyali_kayit_listeye_girer",
    "etiket",
):
    if not blok_sil(rf"^    fn {ad}\b|^fn {ad}\b"):
        print("bulunamadi:", ad)

metin = "".join(satirlar)

# SonucOnbellekKaydi: kullanılmayan siteli alanı.
metin = metin.replace(
    """struct SonucOnbellekKaydi {
    leads: Vec<BusinessLead>,
    siteli: usize,
    gun: i64,
}""",
    """struct SonucOnbellekKaydi {
    leads: Vec<BusinessLead>,
    gun: i64,
}""",
)
metin = metin.replace(
    """        Some(SonucOnbellekKaydi {
            leads: kayit.leads.clone(),
            siteli: kayit.siteli,
            gun,
        })""",
    """        Some(SonucOnbellekKaydi {
            leads: kayit.leads.clone(),
            gun,
        })""",
)

# KotaDurumu.son_kullanim: ipucunda göster.
metin = metin.replace(
    """        format!(
            "Kalan Google Places hakkı: bugün {}/{}, bu ay {}/{}. Anahtarı değiştirmek için tıklayın.",
            durum.gunluk_kalan, durum.gunluk_limit, durum.aylik_kalan, durum.aylik_limit
        )""",
    """        format!(
            "Kalan Google Places hakkı: bugün {}/{}, bu ay {}/{}.{} Anahtarı değiştirmek için tıklayın.",
            durum.gunluk_kalan,
            durum.gunluk_limit,
            durum.aylik_kalan,
            durum.aylik_limit,
            durum
                .son_kullanim
                .as_ref()
                .map(|tarih| format!(" Son kullanım {tarih}."))
                .unwrap_or_default()
        )""",
)

YOL.write_text(metin, encoding="utf-8", newline="")
print("son temizlik tamam, satir:", metin.count("\n"))
