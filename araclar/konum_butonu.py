"""“WhatsApp'tan konum gönder” düğmesini anlık konumla çalıştırır.

Düğmeye basınca tarayıcı GPS'ten konumu alır (önce hassas, olmazsa kaba konum),
Google Haritalar bağlantısını hazır WhatsApp mesajına yazar. İzin verilmezse ya da
12 saniye içinde konum gelmezse ekranda ne yapılacağı yazar; sessizce eski mesaja
düşmez.

Kullanım: python araclar/konum_butonu.py [--kuru]
"""
import os
import re
import sys

KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TEL = "905071299439"
IM = "konum-gonder-js"
DURUM = {0: "zaten var", 1: "eklendi", 2: "güncellendi"}

ILCE = {
    "izmir-kemalpasa": "Kemalpaşa",
    "manisa-turgutlu": "Turgutlu",
    "manisa-saruhanli": "Saruhanlı",
    "manisa-akhisar": "Akhisar",
    "manisa-kirkagac": "Kırkağaç",
    "manisa-soma": "Soma",
    "balikesir-savastepe": "Savaştepe",
}

ANA = ["site/anasayfa.html", "site/index.html", "site/cekici.html", "site/tanitim.html"]

BETIK = """<script id="konum-gonder-js">
(function () {
  var TEL = '905071299439';
  var BEKCI_MS = 12000;

  function gonder(metin) {
    window.location.href = 'https://wa.me/' + TEL + '?text=' + encodeURIComponent(metin);
  }

  function mesaj(lat, lon, dogruluk, bolge) {
    var yer = 'https://www.google.com/maps?q=' + lat.toFixed(6) + ',' + lon.toFixed(6);
    var s = 'Merhaba, aracım yolda kaldı. Anlık konumum: ' + yer;
    if (dogruluk) s += ' (±' + Math.round(dogruluk) + ' m)';
    if (bolge) s += ' · Bölge: ' + bolge;
    return s + ' · Lütfen en yakın ekibi yönlendirin. 0507 129 94 39';
  }

  function bag(l, href) {
    l.href = href;
    l.style.cssText = 'color:#7a5b06;font-weight:600';
    return l;
  }

  function notEkle(a, yedek, metin) {
    var eski = a.parentNode.querySelector('.konum-not');
    if (eski) eski.remove();
    var n = document.createElement('div');
    n.className = 'konum-not';
    n.setAttribute('role', 'status');
    n.style.cssText = 'flex:1 0 100%;margin-top:10px;font:500 12.5px/1.55 system-ui,' +
      '-apple-system,Segoe UI,Roboto,Arial;max-width:38ch;padding:10px 12px;border-radius:10px;' +
      'background:#fff7e6;color:#5c4a16;border:1px solid #e8d9ae';
    n.appendChild(document.createTextNode(metin));

    var wa = document.createElement('a');
    wa.textContent = 'konum olmadan yaz';
    n.appendChild(bag(wa, yedek));

    n.appendChild(document.createTextNode(' ya da '));

    var tel = document.createElement('a');
    tel.textContent = '0507 129 94 39';
    n.appendChild(bag(tel, 'tel:05071299439'));

    n.appendChild(document.createTextNode('.'));
    a.parentNode.insertBefore(n, a.nextSibling);
  }

  function bagla(a) {
    if (a.getAttribute('data-konum-hazir')) return;
    a.setAttribute('data-konum-hazir', '1');
    var bolge = a.getAttribute('data-konum') || '';
    var yedek = a.getAttribute('href') || '';
    var eski = a.textContent;
    var kosuyor = false;
    var bekci = null;

    function dur() {
      if (bekci) { clearTimeout(bekci); bekci = null; }
      kosuyor = false;
      a.textContent = eski;
      a.removeAttribute('aria-busy');
    }

    function hata() {
      dur();
      a.textContent = 'Konum alınamadı · tekrar dene';
      notEkle(a, yedek, 'Konum izni verilmedi. Adres çubuğundaki kilit simgesi → Konum → ' +
        'İzin ver, sonra düğmeye tekrar bas. Hemen ulaşmak istersen ');
    }

    function ok(p) {
      if (!kosuyor) return;
      dur();
      gonder(mesaj(p.coords.latitude, p.coords.longitude, p.coords.accuracy, bolge));
    }

    // Masaüstünde hassas konum çoğu zaman düşer; kaba (ağ tabanlı) konumu dene.
    function kaba() {
      if (!kosuyor) return;
      navigator.geolocation.getCurrentPosition(ok, function () {
        if (!kosuyor) return;
        hata();
      }, { enableHighAccuracy: false, timeout: 6000, maximumAge: 300000 });
    }

    a.addEventListener('click', function (e) {
      if (!navigator.geolocation) return;              // eski davranış: bağlantı açılır
      e.preventDefault();
      if (kosuyor) return;
      kosuyor = true;
      var eskiNot = document.querySelectorAll('.konum-not');
      for (var i = 0; i < eskiNot.length; i++) eskiNot[i].remove();
      a.setAttribute('aria-busy', 'true');
      a.textContent = 'Konum izni verin…';
      // Tarayıcı izin penceresi cevaplanmazsa istek asılı kalır: kendi sayacımızı kur.
      bekci = setTimeout(function () {
        if (!kosuyor) return;
        hata();
      }, BEKCI_MS);
      navigator.geolocation.getCurrentPosition(ok, kaba,
        { enableHighAccuracy: true, timeout: 4000, maximumAge: 0 });
    });
  }

  var liste = document.querySelectorAll('a[data-konum]');
  for (var i = 0; i < liste.length; i++) bagla(liste[i]);
})();
</script>
"""

SIRA_KONUM = re.compile(
    r'<a class="[^"]*" href="https://wa\.me/' + TEL + r'[^"]*"[^>]*>[^<]*[Kk]onum gönder</a>'
)


def etiketle(html: str, desen: re.Pattern, bolge: str):
    """Eşleşen <a> etiketlerine data-konum ekler (idempotent)."""
    sayi = 0

    def degistir(m):
        nonlocal sayi
        parca = m.group(0)
        if "data-konum" in parca:
            return parca
        sayi += 1
        return '<a data-konum="%s" %s' % (bolge, parca[3:])

    return desen.sub(degistir, html), sayi


def betik_ekle(html: str):
    """Betiği ekler; daha önce eklenmişse güncel sürümle değiştirir."""
    if IM in html:
        yeni = re.sub(
            r'<script id="konum-gonder-js">.*?</script>', lambda _: BETIK.strip(), html, flags=re.S
        )
        return yeni, (2 if yeni != html else 0)
    if "</body>" not in html:
        return html, 0
    return html.replace("</body>", BETIK + "</body>", 1), 1


def main() -> int:
    kuru = "--kuru" in sys.argv
    toplam_dugme = toplam_betik = 0
    for ad in ANA:
        yol = os.path.join(KOK, ad)
        html = open(yol, encoding="utf-8").read()
        html, dugme = etiketle(html, SIRA_KONUM, "Saruhanlı / Manisa")
        html, betik = betik_ekle(html)
        toplam_dugme += dugme
        toplam_betik += 1 if betik else 0
        print(f"{ad:48} düğme +{dugme} · betik {DURUM[betik]}")
        if not kuru:
            open(yol, "w", encoding="utf-8").write(html)

    for anahtar, bolge in ILCE.items():
        ad = f"bolgesel/saruhanliotokurtarma/{anahtar}.html"
        yol = os.path.join(KOK, ad)
        html = open(yol, encoding="utf-8").read()
        html, d1 = etiketle(
            html, re.compile(r'<a class="dugme wa" href="https://wa\.me/' + TEL + r'[^"]*"'), bolge
        )
        html, d2 = etiketle(
            html, re.compile(r'<a class="arama-cubugu__wa" href="https://wa\.me/' + TEL + r'[^"]*"'), bolge
        )
        html, betik = betik_ekle(html)
        toplam_dugme += d1 + d2
        toplam_betik += 1 if betik else 0
        print(f"{ad:48} düğme +{d1 + d2} · betik {DURUM[betik]}")
        if not kuru:
            open(yol, "w", encoding="utf-8").write(html)

    print(f"\ntoplam: {toplam_dugme} yeni düğme, {toplam_betik} sayfada betik güncel"
          + (" (kuru)" if kuru else ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
