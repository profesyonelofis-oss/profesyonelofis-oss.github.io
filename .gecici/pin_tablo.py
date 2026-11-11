from __future__ import annotations
"""Pinleme durum tablosunu tek PNG olarak çizer."""
from PIL import Image, ImageDraw, ImageFont

W = 1040
PAD = 40
BG_TOP = (13, 58, 41)
BG_BOT = (7, 17, 13)
CREAM = (244, 241, 232)
DIM = (152, 173, 160)
FAINT = (120, 143, 130)
LIME = (200, 241, 105)
AMBER = (255, 179, 71)
CARD = (9, 27, 20)
LINE = (255, 255, 255, 28)
HDRBG = (255, 255, 255, 10)

F = "C:/Windows/Fonts/segoeui.ttf"
FB = "C:/Windows/Fonts/segoeuib.ttf"


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FB if bold else F, size)


img = Image.new("RGB", (W, 2300), BG_BOT)
d = ImageDraw.Draw(img, "RGBA")

# arka plan: dikey degrade
for y in range(2300):
    t = min(1.0, y / 700)
    r = round(BG_TOP[0] + (BG_BOT[0] - BG_TOP[0]) * t)
    g = round(BG_TOP[1] + (BG_BOT[1] - BG_TOP[1]) * t)
    b = round(BG_TOP[2] + (BG_BOT[2] - BG_TOP[2]) * t)
    d.line([(0, y), (W, y)], fill=(r, g, b))

y = 34

# ---------------- baslik ----------------
d.rounded_rectangle([PAD, y, PAD + 54, y + 54], 15, fill=(10, 51, 38), outline=(200, 241, 105, 90), width=1)
d.text((PAD + 16, y + 12), "S", font=font(26, True), fill=LIME)

d.text((PAD + 70, y + 2), "Pinleme durumu", font=font(33, True), fill=CREAM)
d.text((PAD + 70, y + 44), "Saruhanlı Oto Kurtarma  ·  O-5 koridoru  ·  İzmir – Manisa – Balıkesir / Savaştepe  ·  0507 129 94 39",
       font=font(13), fill=DIM)

tarih = "10 KASIM 2026"
ft = font(11, True)
tw = d.textlength(tarih, font=ft)
d.rounded_rectangle([W - PAD - tw - 34, y + 4, W - PAD, y + 30], 13, fill=(255, 255, 255, 12), outline=LINE, width=1)
d.text((W - PAD - tw - 17, y + 10), tarih, font=ft, fill=(219, 231, 223))

y += 66
d.line([(PAD, y), (W - PAD, y)], fill=LINE)
y += 18

# ---------------- KPI ----------------
kpis = [("30", "PİN / BÖLGE"), ("8", "ANAHTAR KELİME"), ("30", "BÖLGE SAYFASI"),
        ("37", "ÜRETİM DOSYASI"), ("0", "YAYINDA SAYFA")]
kw = (W - 2 * PAD - 4 * 9) / 5
for i, (num, lab) in enumerate(kpis):
    x = PAD + i * (kw + 9)
    d.rounded_rectangle([x, y, x + kw, y + 84], 13, fill=(9, 27, 20, 235), outline=LINE, width=1)
    col = LIME if i < 4 else AMBER
    d.text((x + 16, y + 12), num, font=font(31, True), fill=col)
    d.text((x + 16, y + 57), lab, font=font(9.5, True), fill=FAINT)
y += 106


def bolum(baslik: str) -> None:
    global y
    d.text((PAD, y), baslik, font=font(12, True), fill=LIME)
    y += 22


def tablo(sutunlar, satirlar, genislikler, hizala=None, satir_h=40):
    """Tabloyu çizer. satirlar: list[list[str|tuple]]; tuple ise (metin, renk, dolgu)."""
    global y
    x0, x1 = PAD, W - PAD
    h = 34 + satir_h * len(satirlar)
    d.rounded_rectangle([x0, y, x1, y + h], 13, fill=(9, 27, 20, 235), outline=LINE, width=1)
    d.rectangle([x0 + 1, y + 1, x1 - 1, y + 34], fill=HDRBG)
    xs = []
    cx = x0 + 18
    for i, g in enumerate(genislikler):
        xs.append(cx)
        cx += g
    for i, s in enumerate(sutunlar):
        d.text((xs[i], y + 11), s, font=font(9.5, True), fill=FAINT)
    d.line([(x0 + 1, y + 34), (x1 - 1, y + 34)], fill=LINE)
    ry = y + 34
    for ri, satir in enumerate(satirlar):
        if ri:
            d.line([(x0 + 1, ry), (x1 - 1, ry)], fill=(255, 255, 255, 14))
        for ci, hucre in enumerate(satir):
            bok = isinstance(hucre, tuple)
            metin, renk = (hucre[0], hucre[1]) if bok else (hucre, CREAM)
            fs = font(12, True) if ci == 0 else font(12)
            if bok:
                col = renk
                tw = d.textlength(metin, font=font(10, True))
                d.rounded_rectangle([xs[ci] - 2, ry + 9, xs[ci] + tw + 22, ry + 31], 11,
                                    fill=(col[0], col[1], col[2], 26), outline=(col[0], col[1], col[2], 70), width=1)
                d.text((xs[ci] + 10, ry + 14), metin, font=font(10, True), fill=col)
            else:
                d.text((xs[ci], ry + 12), metin, font=fs, fill=renk)
        ry += satir_h
    y += h + 14


# ---------------- 1 ----------------
bolum("1 · PİNLEMEDE NE YAPILDI")
OK = (LIME[0], LIME[1], LIME[2])
NO = (AMBER[0], AMBER[1], AMBER[2])
tablo(
    ["İŞ", "DURUM", "NOT"],
    [
        ["O-5 koridor haritası, 30 numaralı pin", ("BİTTİ", OK), "İstanbul → İzmir sıralı, hat üzerinde"],
        ["8 hizmet kelimesi (ilçe adı eklenmeden)", ("BİTTİ", OK), "çekici, akü takviye, akücü…"],
        ["Her pin için Google Maps arama bağlantısı", ("BİTTİ", OK), "30 / 30 pin"],
        ["30 bölgeye özel açılış sayfası", ("BİTTİ", OK), "başlık + 8 hizmet + 5 S.S.S. + telefon"],
        ["AutoRepair + OfferCatalog şeması", ("BİTTİ", OK), "her sayfada gömülü"],
        ["sitemap.xml · robots.txt", ("BİTTİ", OK), "alan adı satırı bekliyor"],
        ["Plan · gönderi · yorum metinleri", ("BİTTİ", OK), "plan, siralama-plani, gonderiler, profil"],
        ["Tek işletme kuralı (kopya pin yok)", ("UYULDU", OK), "30 pin işaret; 30 ayrı profil değil"],
        ["Alan adı + yayına alma", ("BEKLİYOR", NO), "senin alacağın alan adı"],
        ["Profil doğrulama + 30 bölge hizmet alanı", ("BEKLİYOR", NO), "Google hesabında, elle"],
        ["Search Console + sitemap gönderimi", ("BEKLİYOR", NO), "alan adından sonra"],
        ["10 gerçek fotoğraf + haftada 3 yorum", ("BEKLİYOR", NO), "sıralamayı en hızlı artıran ikisi"],
    ],
    [450, 130, 330],
)

# ---------------- 2 ----------------
bolum("2 · KELİMELERDE DURUM")
kelimeler = ["çekici", "oto kurtarma", "yol yardım 7/24", "en yakın çekici",
             "benzin destek", "mazot takviye", "akü takviye", "akücü"]
GREY = (140, 163, 150)
tablo(
    ["KELİME", "PİN", "SAYFA", "PROFİL HİZMETİ", "GOOGLE SIRASI"],
    [[k, ("HAZIR", OK), ("HAZIR", OK), ("BEKLİYOR", NO), ("ÖLÇÜLMEDİ", GREY)] for k in kelimeler],
    [240, 130, 130, 190, 220],
    satir_h=38,
)

d.text((PAD, y), "Google sırası neden boş:", font=font(12, True), fill=CREAM)
y += 19
for satir in [
    "site yayında değil, alan adı alınmadı, işletme profili doğrulanmadı ve Search Console bağlı değil.",
    "Bu dört adım olmadan hiçbir kelimede ölçülecek sıra yok; uydurma numara yazmıyoruz.",
]:
    d.text((PAD, y), satir, font=font(12), fill=DIM)
    y += 18
y += 12

# ---------------- 3 ----------------
bolum("3 · NE KADAR ÖNDEYİZ")
barlar = [
    ("Teknik hazırlık (üretim)", 1.00, "%100 · bitti", LIME),
    ("Yayına alma", 0.02, "%0 · alan adı yok", AMBER),
    ("Profil + doğrulama", 0.02, "%0 · elle yapılacak", AMBER),
    ("Yorum (ilk 30 gün / 12)", 0.02, "0 / 12", AMBER),
    ("Ölçüm (4 hafta)", 0.02, "0 / 4 hafta", AMBER),
]
bh = 34 * len(barlar) + 22
d.rounded_rectangle([PAD, y, W - PAD, y + bh], 13, fill=(9, 27, 20, 235), outline=LINE, width=1)
by = y + 14
for lab, oran, val, col in barlar:
    d.text((PAD + 18, by + 4), lab, font=font(12), fill=(219, 231, 223))
    tx0, tx1 = PAD + 260, W - PAD - 150
    d.rounded_rectangle([tx0, by + 8, tx1, by + 15], 4, fill=(255, 255, 255, 26))
    ww = max(6, int((tx1 - tx0) * oran))
    d.rounded_rectangle([tx0, by + 8, tx0 + ww, by + 15], 4, fill=col)
    vt = val
    vt_w = d.textlength(vt, font=font(11))
    d.text((W - PAD - 18 - vt_w, by + 3), vt, font=font(11), fill=DIM)
    by += 34
y += bh + 16

d.text((PAD, y), "Rakip sıralamasını bilmiyoruz; “kaçıncı sıradayız” ancak yayın + doğrulama + 2–4 hafta dizinleme sonrası ölçülür.",
       font=font(12), fill=DIM)
y += 18
d.text((PAD, y), "Teknik tarafta 30 bölge sayfası, 8 kelime ve şema hazır: yayına geçince hızlı tırmanış için avantaj.",
       font=font(12), fill=DIM)
y += 26

# ---------------- alt bilgi ----------------
d.line([(PAD, y), (W - PAD, y)], fill=LINE)
y += 14
d.text((PAD, y), "Dosyalar:  bolgesel/saruhanliotokurtarma/  →  30 bölge sayfası · index.html · sitemap.xml · robots.txt · plan.md · siralama-plani.md · gonderiler.md · profil.md",
       font=font(10.5), fill=FAINT)
y += 16
d.text((PAD, y), "Kaynak:  src/otoyol_pin.rs (30 pin, 8 kelime)  ·  src/bolgesel.rs (30 bölge üretimi)  ·  Google’a hiçbir veri gönderilmedi.",
       font=font(10.5), fill=FAINT)
y += 34

img = img.crop((0, 0, W, y))
img.save("pin-durum.png")
print("yazildi: pin-durum.png", img.size)
