"""Demo harita testlerini Google embed'e uyarla, artık doc satırını sil."""

from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")


def degistir(eski: str, yeni: str) -> None:
    global metin
    if eski not in metin:
        raise SystemExit("bulunamadi:\n" + eski[:200])
    metin = metin.replace(eski, yeni, 1)


degistir(
    '''        assert_eq!(
            html.matches("src=\\"http").count(),
            html.matches("src=\\"https://www.openstreetmap.org").count(),
            "dış kaynak yalnızca harita olmalı"
        );''',
    '''        assert_eq!(
            html.matches("src=\\"http").count(),
            html.matches("src=\\"https://maps.google.com").count(),
            "dış kaynak yalnızca Google haritası olmalı"
        );''',
)

degistir(
    '''        assert!(
            html.contains("export/embed.html") && html.contains("marker=41.04285,29.00753"),
            "konum varsa gömülü harita olmalı"
        );''',
    '''        assert!(
            html.contains("https://maps.google.com/maps?q=41.04285,29.00753"),
            "konum varsa gömülü Google haritası olmalı"
        );''',
)

degistir(
    '''        assert!(acik.contains("openstreetmap.org/export/embed"));''',
    '''        assert!(acik.contains("maps.google.com/maps?q="));''',
)

degistir(
    """/// Anahtarı `.env` dosyasına yazar ve çalışan süreçte etkinleştirir.

impl LeadApp {""",
    """impl LeadApp {""",
)

YOL.write_text(metin, encoding="utf-8", newline="")
print("test beklentileri guncellendi")
