"""Test silme sırasında kalan gövde parçalarını temizler."""

from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")

KALINTI = '''
        )
        .expect("json");
        assert_eq!(
            kayit_dolulugu(&dolu),
            10,
            "puanlanan alanlar (telefon/adres) sayılmaz, diğer 5 etiket tavanı doldurur"
        );
    }
'''

if KALINTI not in metin:
    raise SystemExit("kalinti bulunamadi")

YOL.write_text(metin.replace(KALINTI, "", 1), encoding="utf-8")
print("kalinti silindi")
