# Sinematik Hero Videosu Üretim Rehberi

Her sektör grubu için üç şey var: **video prompt'u** (metinden videoya), **poster
prompt'u** (tek kare, video için yedek ve ilk yükleme görseli) ve **ücretsiz stok
arama kelimeleri**. Videolar 5-8 saniye, tek çekim, kamera çok yavaş yaklaşan
(push-in) ya da çok yavaş kayan (slider) olmalı; hızlı kesme, zoom-out ve el
kamera titremesi kullanılmaz. Prompt'ların sonunda **"no text, no logos, no
people looking at camera"** ifadesi mutlaka durur.

Teknik beklenti (programın içe aktarma adımı bunu kendisi uygular):
720p, H.264, sessiz, en fazla 8 sn, her kare anahtar kare (all-intra).
Kaynak dosyayı 1080p veya üstü, yatay, 5-20 sn olarak indir.

---

## 1 · Kuaför / berber (`media/sektor/kuafor/`)

**Video prompt**
> Cinematic close-up inside an upscale barber shop at dusk, slow dolly push-in
> along a leather chair toward a mirror, warm tungsten pools of light, soft
> haze in the air, shallow depth of field 50mm f/1.4, subtle film grain,
> reflections of amber bulbs on chrome, quiet luxurious atmosphere, 24fps,
> 6 seconds, no text, no logos, no people looking at camera.

**Poster prompt**
> Single cinematic still: vintage barber chair beside a large mirror in a dark
> walnut-panelled salon, warm amber practical lights, deep shadows, shallow
> depth of field, muted film palette, 35mm photograph, no text, no logos.

**Stok arama kelimeleri:** `barber shop cinematic`, `hair salon slow motion`,
`hairdresser close up`

---

## 2 · Restoran / kafe (`media/sektor/restoran/`)

**Video prompt**
> Cinematic slow push-in across a candle-lit restaurant table at night, steam
> rising from a plated dish, warm brass highlights, dark green velvet
> background bokeh, hands of a chef placing garnish in the far background,
> 85mm lens, shallow depth of field, rich contrast, film grain, 6 seconds,
> no text, no logos, no faces to camera.

**Poster prompt**
> Moody still life: a single plated dish on a dark stone table, one candle,
> brass cutlery, deep shadows, warm rim light, editorial food photography,
> 50mm, no text, no logos.

**Stok arama kelimeleri:** `restaurant cinematic night`, `chef plating slow
motion`, `coffee shop atmosphere`

---

## 3 · Oto servis (`media/sektor/oto/`)

**Video prompt**
> Cinematic slow slider move through a dark premium auto workshop, a car hood
> opening under cool overhead strip lights, fine dust in the light beam, teal
> and warm orange contrast, reflective concrete floor, 35mm anamorphic look,
> shallow focus, 6 seconds, no text, no logos, no people looking at camera.

**Poster prompt**
> Cinematic still: a polished car on a lift inside a dark garage, cool strip
> lights overhead, wet concrete reflections, teal-orange grade, wide 35mm
> frame, no text, no logos.

**Stok arama kelimeleri:** `car workshop cinematic`, `mechanic working close
up`, `garage lights slow motion`

---

## 4 · Klinik / diş (`media/sektor/klinik/`)

**Video prompt**
> Cinematic slow push-in down a calm modern dental clinic corridor, soft
> daylight from a side window, pale oak and matte white surfaces, one plant
> silhouette, very shallow depth of field, clean minimal atmosphere, 50mm,
> gentle film grain, 6 seconds, no text, no logos, no people looking at camera.

**Poster prompt**
> Serene still: modern dental treatment room with pale oak cabinetry, soft
> morning light, one green plant, clean minimal composition, architectural
> photography, no text, no logos.

**Stok arama kelimeleri:** `dental clinic cinematic`, `dentist close up light`,
`medical interior slow motion`

---

## 5 · Otel (`media/sektor/otel/`)

**Video prompt**
> Cinematic slow push-in through a boutique hotel room at sunrise, linen
> curtains breathing in the breeze, warm light raking across a stone wall,
> dust motes in the air, 35mm lens, soft contrast, luxurious quiet mood,
> 7 seconds, no text, no logos, no people.

**Poster prompt**
> Editorial still: boutique hotel room at golden hour, linen bed, stone wall,
> sheer curtain glowing, warm shadows, architectural digest style, no text,
> no logos.

**Stok arama kelimeleri:** `boutique hotel cinematic`, `hotel lobby slow
motion`, `morning light hotel room`

---

## 6 · Spor salonu (`media/sektor/spor/`)

**Video prompt**
> Cinematic low-angle slow push-in across dark gym equipment, chalk dust
> floating in a single hard spotlight, cold blue shadows with warm rim light,
> matte black plates, 24mm lens, high contrast, film grain, powerful quiet
> atmosphere, 6 seconds, no text, no logos, no people looking at camera.

**Poster prompt**
> Dramatic still: barbell loaded with black plates on a rubber floor, single
> overhead spotlight, chalk dust in the beam, dark background, high contrast
> editorial photography, no text, no logos.

**Stok arama kelimeleri:** `gym cinematic dark`, `fitness training slow
motion`, `weights close up light`

---

## Çekim kuralları (hepsi için)

- Tek çekim, kesme yok; kamera yavaş yaklaşır veya yavaş kayar.
- 5-8 saniye; ilk kare ile son kare arasında büyük kompozisyon değişimi yok
  (kaydırmayla ileri-geri oynatılacağı için).
- Hareket bulanıklığı düşük olsun: 24fps, 1/50 shutter hissi.
- Metin, logo, marka, okunabilir tabela yok.
- İnsan varsa yalnız el/silüet; kameraya bakan yüz yok.
- Renk: koyu zemin + tek sıcak vurgu (altın/amber) veya tek soğuk vurgu.
- Dikey kırpım için ortada güvenli alan bırak (mobilde `object-fit: cover`).

## İçe aktarma adımları

1. Videoyu indir (1080p veya üstü, yatay, 5-20 sn).
2. Panelde **Demo → Üretim motoru: Sinematik → Sinematik varlıklar**.
3. Sektör grubunu seç, arama kelimesini yaz, **Adayları getir**.
4. Aday listesinden **Bu videoyu hazırla**: program indirir, 720p all-intra
   sessiz videoya çevirir, `poster.webp` çıkarır, `KAYNAK.json` yazar.
5. **Demo üret**: sayfa `demos/sinematik/<işletme>/index.html` olarak,
   `media/hero.mp4` ile birlikte yazılır.

Kaynak, çekim sahibi, bağlantı ve lisans bilgisi her zaman
`media/sektor/<grup>/KAYNAK.json` içinde durur; sayfaya yalnız lisans atıf
istiyorsa altbilgi satırı eklenir.
