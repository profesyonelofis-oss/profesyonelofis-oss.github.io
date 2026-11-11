"""main.rs: ölü Google yardımcılarını sil, rozet + 3 sütun + hedef paneli düzelt."""

import re
from pathlib import Path

YOL = Path("src/main.rs")
s = YOL.read_text(encoding="utf-8")


def rep(eski: str, yeni: str, adet: int = 1, ad: str = "") -> None:
    global s
    c = s.count(eski)
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    s = s.replace(eski, yeni)


def fn_sil(ad: str, girinti: str = "") -> None:
    global s
    im = re.search(rf"(?m)^{re.escape(girinti)}(?:pub(?:\(crate\))? )?fn {re.escape(ad)}\b", s)
    assert im, f"fonksiyon yok: {ad}"
    bas = im.start()
    while True:
        onceki = s.rfind("\n", 0, bas - 1)
        if onceki < 0:
            break
        satir = s[onceki + 1 : bas].strip()
        if satir.startswith("///") or satir.startswith("#[") or satir.startswith("//"):
            bas = onceki + 1
        else:
            break
    derinlik = 0
    gordum = False
    i = im.end()
    while i < len(s):
        if s[i] == "{":
            derinlik += 1
            gordum = True
        elif s[i] == "}":
            derinlik -= 1
            if gordum and derinlik == 0:
                son = i + 1
                while son < len(s) and s[son] in "\r\n":
                    son += 1
                s = s[:bas] + s[son:]
                return
        i += 1
    raise AssertionError(f"kapanış yok: {ad}")


# --- testler: kaldırılan HTTP yardımcılarının testleri ---------------------
for test in [
    "gecici_hatalar_ayirt_edilir",
    "retry_after_basligi_beklemeye_cevrilir",
    "tekrar_dene_gecici_hatada_yeniden_dener",
    "tekrar_dene_kalici_hatada_durmaz",
]:
    fn_sil(test, "    ")

# --- ölü kod --------------------------------------------------------------
for ad in [
    "http_hata",
    "tekrar_dene_n",
    "tekrar_dene",
    "retry_after_sn",
    "gecici_hata",
    "ilce_kotasi",
    "ay_anahtari",
    "ayar_degeri",
]:
    fn_sil(ad)

rep("/// İstek başına en fazla deneme sayısı (Google Maps Platform).\nconst ISTEK_DENEMESI: usize = 3;\n\n", "", ad="istek denemesi sabiti")
rep(
    """/// Street View karelerinin önbelleği. Görüntü istekleri faturalanır; renk
/// değiştirmek veya önizlemeyi tazelemek aynı işletme için yeniden ücret
/// ödetmesin diye indirilen kare diskte tutulur.

/// İki zıt yön: dükkânın hangi tarafa baktığını bilmediğimiz için cephenin
/// görünme şansı artar. Her kare bir faturalı istektir.
const STREETVIEW_YONLER: [u16; 2] = [0, 180];

""",
    "",
    ad="street view sabitleri",
)
rep(
    "    fn duz_metin(html: &str) -> String {",
    "    #[cfg(test)]\n    fn duz_metin(html: &str) -> String {",
    ad="duz_metin test niteliği",
)

# --- test beklentisi: gerekçe metni artık "150 inceleme" -------------------
rep(
    '            potansiyel_gerekcesi(&incelemeli).contains("150 Google incelemesi"),',
    '            potansiyel_gerekcesi(&incelemeli).contains("150 inceleme"),',
    ad="gerekçe testi",
)

# --- kart rozeti ----------------------------------------------------------
rep(
    """                            pill(ui, &lead.category, kategori_rengi);
                            durum_pili(ui, lead.durum);""",
    """                            pill(ui, &lead.category, kategori_rengi);
                            // Bu ekranın tek hedefi sitesiz işletmeler: rozet
                            // her kartta görünür ve dijital durumu söyler.
                            pill(ui, "❌ Web Sitesi Yok - Hedef Müşteri", VURGU);
                            durum_pili(ui, lead.durum);""",
    ad="kart rozeti",
)

# --- 3 sütun: %20 - %45 - %35 ---------------------------------------------
rep(
    "            .exact_width(300.0)",
    "            .exact_width((ctx.screen_rect().width() * 0.20).clamp(260.0, 340.0))",
    ad="sol panel genişliği",
)
rep(
    "        let harita_genisligi = (ctx.screen_rect().width() * 0.42).clamp(420.0, 760.0);",
    "        let harita_genisligi = (ctx.screen_rect().width() * 0.35).clamp(360.0, 720.0);",
    ad="harita genişliği",
)

# --- günlük hedef paneli: haritanın üstünden çık, satırları düzelt ---------
i = s.index(".anchor(egui::Align2::RIGHT_TOP, [-22.0, 84.0])")
s = (
    s[:i]
    + ".anchor(egui::Align2::LEFT_TOP, [336.0, 96.0])"
    + s[i + len(".anchor(egui::Align2::RIGHT_TOP, [-22.0, 84.0])") :]
)
rep(
    """            .default_width(460.0)
            .anchor(egui::Align2::LEFT_TOP, [336.0, 96.0])""",
    """            .default_width(430.0)
            .max_height(440.0)
            .anchor(egui::Align2::LEFT_TOP, [336.0, 96.0])""",
    ad="hedef paneli boyutu",
)

rep(
    """                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:02}", sira_no + 1))
                                            .size(10.0)
                                            .color(DIM),
                                    );
                                    ui.vertical(|ui| {
                                        ui.add_space(1.0);
                                        ui.label(
                                            egui::RichText::new(&kayit.ad)
                                                .size(12.0)
                                                .color(METIN),
                                        );
                                        ui.label(
                                            egui::RichText::new(kayit.alt_satir())
                                                .size(9.0)
                                                .color(SOLUK),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {""",
    """                                // Metin sütunu düğmelere yer bırakır: uzun adres
                                // sarılır, "Hazırla"/"Tamamlandı" üstüne binmez.
                                ui.horizontal_top(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:02}", sira_no + 1))
                                            .size(10.0)
                                            .color(DIM),
                                    );
                                    let dugme_g = 92.0 + 6.0 + 70.0 + 10.0;
                                    let metin_g = (ui.available_width() - dugme_g).max(140.0);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(metin_g, 0.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.add_space(1.0);
                                            ui.label(
                                                egui::RichText::new(&kayit.ad)
                                                    .size(12.0)
                                                    .color(METIN),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(kayit.alt_satir())
                                                        .size(9.5)
                                                        .color(SOLUK),
                                                )
                                                .wrap(true),
                                            );
                                        },
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {""",
    ad="hedef satırı",
)
rep(
    """                                            if kayit.lead_idx.is_some()
                                                && ikincil_buton(
                                                    ui,
                                                    "Hazırla",
                                                    true,
                                                    Some(egui::vec2(66.0, 26.0)),
                                                )""",
    """                                            if kayit.lead_idx.is_some()
                                                && ikincil_buton(
                                                    ui,
                                                    "Hazırla",
                                                    true,
                                                    Some(egui::vec2(66.0, 26.0)),
                                                )""",
    ad="hazırla düğmesi",
)

# --- metin: Google kalıntısı kullanıcı yüzeyinde kalmasın -----------------
rep(
    "/// Google'daki ortalama yıldız puanı (0-5). Yalnızca Google kaynağında gelir;\n    /// Google Maps'te yorum/puan verisi yoktur. Karnede \"4,2 ★ · 37 yorum\"",
    "/// Ortalama yıldız puanı (0-5). Yalnızca harici kaynaklı eski kayıtlarda\n    /// bulunur; OSM kayıtlarında yoktur. Karnede \"4,2 ★ · 37 yorum\"",
    ad="yıldız belgesi",
)
rep(
    "    /// Google'daki inceleme sayısı: gerçek müşteri trafiği ve bütçe göstergesi.",
    "    /// İnceleme sayısı (harici kaynaklı eski kayıtlarda): müşteri trafiği sinyali.",
    ad="inceleme belgesi",
)

YOL.write_text(s, encoding="utf-8", newline="")
print("tamam")
