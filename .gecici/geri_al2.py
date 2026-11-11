from pathlib import Path
p=Path('src/main.rs')
s=p.read_text(encoding='utf-8')
s=s.replace('            demo_yaz_profille("demos", &lead, "+90 500 000 00 00", Some("sinematik"))\n                .expect("demo yazılmalı");', '            fs::write(\n                &demo_yolu,\n                demo_site::kucult(&demo_html(&lead, "+90 500 000 00 00")),\n            )\n            .expect("demo yazılmalı");', 1)
p.write_text(s, encoding='utf-8')
