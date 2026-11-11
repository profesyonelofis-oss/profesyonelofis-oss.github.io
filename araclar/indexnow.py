"""IndexNow ile URL'leri Bing ve Yandex'e bildirir (hesap gerekmez).

Kullanım: python araclar/indexnow.py [https://alan-adresi]
Anahtar dosyası yayında olmalı: <alan>/<anahtar>.txt
"""
import json
import os
import sys
import urllib.request

KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUBLIC = os.path.join(KOK, "yayin", "public")
SAYFALAR = [
    "",
    "izmir-kemalpasa",
    "manisa-turgutlu",
    "manisa-saruhanli",
    "manisa-akhisar",
    "manisa-kirkagac",
    "manisa-soma",
    "balikesir-savastepe",
    "anasayfa",
    "cekici",
]


def anahtar() -> str:
    for ad in os.listdir(PUBLIC):
        if ad.endswith(".txt") and len(ad) == 36 and ad[:32].isalnum():
            return ad[:-4]
    raise SystemExit("anahtar dosyası yok: önce araclar/yayin_hazirla.py çalıştır")


def main() -> int:
    if len(sys.argv) < 2:
        print("kullanım: python araclar/indexnow.py <https://alan-adresi>")
        return 2
    url = sys.argv[1].rstrip("/")
    host = url.split("://", 1)[-1].split("/", 1)[0]
    kod = anahtar()
    govde = {
        "host": host,
        "key": kod,
        "keyLocation": f"{url}/{kod}.txt",
        "urlList": [f"{url}/{s}" if s else f"{url}/" for s in SAYFALAR],
    }
    istek = urllib.request.Request(
        "https://api.indexnow.org/indexnow",
        data=json.dumps(govde).encode("utf-8"),
        headers={"Content-Type": "application/json; charset=utf-8"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(istek, timeout=20) as yanit:
            print(f"IndexNow yanıtı: {yanit.status} {yanit.reason}")
        print(f"bildirilen URL: {len(govde['urlList'])} · anahtar: {kod}.txt")
        return 0
    except Exception as hata:  # noqa: BLE001 - ağ hatası kullanıcıya gösterilir
        print(f"IndexNow hatası: {hata}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
