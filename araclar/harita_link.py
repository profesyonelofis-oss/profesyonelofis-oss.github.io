"""Demo sayfası ve uygulamadaki harita bağlantılarını Google'a çevirir."""

from pathlib import Path

demo = Path("src/demo_site.rs")
d = demo.read_text(encoding="utf-8")

eski_bbox = '''    let bbox = format!(
        "{:.4},{:.4},{:.4},{:.4}",
        boylam - 0.004,
        enlem - 0.003,
        boylam + 0.004,
        enlem + 0.003
    );
'''
if eski_bbox not in d:
    raise SystemExit("demo_site: bbox bulunamadi")
d = d.replace(eski_bbox, "", 1)

d = d.replace(
    '"https://www.openstreetmap.org/export/embed.html?bbox={bbox}&layer=mapnik&marker={enlem:.5},{boylam:.5}"',
    '"https://maps.google.com/maps?q={enlem:.5},{boylam:.5}&z=17&output=embed"',
)
d = d.replace(
    '"https://www.openstreetmap.org/?mlat={enlem:.5}&mlon={boylam:.5}#map=18/{enlem:.5}/{boylam:.5}"',
    '"https://www.google.com/maps/search/?api=1&query={enlem:.5},{boylam:.5}"',
)
demo.write_text(d, encoding="utf-8")

ana = Path("src/main.rs")
m = ana.read_text(encoding="utf-8")
m = m.replace(
    '''                            let harita = format!(
                                "https://www.openstreetmap.org/?mlat={enlem}&mlon={boylam}#map=18/{enlem}/{boylam}"
                            );''',
    '''                            let harita = format!(
                                "https://www.google.com/maps/search/?api=1&query={enlem},{boylam}"
                            );''',
)
ana.write_text(m, encoding="utf-8")
print("harita baglantilari google'a cevrildi")
