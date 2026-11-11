import io

p = "src/demo_site.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

ek = r'''
/// Acil sektör katmanı: Ken Burns, cam rozetler, konum butonu, mobil bar,
/// yorum kaydırıcısı ve yol yardım asistanı. Yalnız bu katman eklenir; klasik
/// düzen ve "gece"/"ışık" temaları olduğu gibi kalır.
const EK_CSS: &str = r#"
/* --- Ken Burns: görseller yavaşça yakınlaşır, parallax ile birlikte --- */
@keyframes kenburns{0%{transform:scale(1.02) translate3d(0,0,0)}55%{transform:scale(1.1) translate3d(-1.2%,-1%,0)}100%{transform:scale(1.16) translate3d(1.4%,-1.8%,0)}}
.sahne.foto .sahne-foto{animation:kenburns 24s ease-in-out infinite alternate;will-change:transform}
.galeri-kare img{animation:kenburns 28s ease-in-out infinite alternate;will-change:transform}
.hikaye-gorsel img{animation:kenburns 30s ease-in-out infinite alternate;will-change:transform}
.perde-kare img{animation:kenburns 34s ease-in-out infinite alternate;will-change:transform}
.sahne.foto{transform:translateY(calc(var(--kaydir,0px) * -1))}
.galeri-kare{transform:translateY(calc(var(--kaydir,0px) * .35))}
@media (prefers-reduced-motion:reduce){
  .sahne.foto .sahne-foto,.galeri-kare img,.hikaye-gorsel img,.perde-kare img{animation:none}
  .sahne.foto,.galeri-kare{transform:none}
}
/* --- cam rozetler --- */
.hero-rozetler{display:flex;flex-wrap:wrap;gap:8px;margin-top:18px}
.hcip{display:inline-flex;align-items:center;gap:8px;padding:8px 13px;border-radius:999px;font-size:11.5px;font-weight:600;letter-spacing:.03em;color:color-mix(in srgb,var(--ink) 84%,transparent);background:color-mix(in srgb,var(--bg) 58%,transparent);border:1px solid color-mix(in srgb,var(--accent) 26%,transparent);backdrop-filter:blur(14px) saturate(1.15);-webkit-backdrop-filter:blur(14px) saturate(1.15);box-shadow:0 10px 26px -20px color-mix(in srgb,var(--ink) 60%,transparent)}
.hcip.canli b{width:7px;height:7px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.55);animation:nabiz 2.2s infinite}
.hcip.canli{color:color-mix(in srgb,#2f9e63 78%,var(--ink));border-color:color-mix(in srgb,#4ade80 42%,transparent)}
/* --- canlı konum butonu --- */
.buton-konum{position:relative;overflow:hidden;background:linear-gradient(120deg,var(--accent),color-mix(in srgb,var(--accent) 55%,#12b3a6));color:var(--accent-karsi);box-shadow:0 18px 44px -18pm var(--accent)}
.buton-konum::after{content:"";position:absolute;inset:0;background:linear-gradient(110deg,transparent 20%,rgba(255,255,255,.38) 42%,transparent 62%);transform:translateX(-120%);animation:parilti 4.2s ease-in-out infinite}
@keyframes parilti{0%,55%{transform:translateX(-120%)}85%{transform:translateX(120%)}100%{transform:translateX(120%)}}
.buton-konum svg{width:16px;height:16px}
/* --- sabit mobil acil barı --- */
.mobil-bar{display:none;position:fixed;left:10px;right:10px;bottom:10px;z-index:80;gap:8px;padding:8px;border-radius:16px;background:color-mix(in srgb,var(--bg) 72%,transparent);border:1px solid var(--cizgi);backdrop-filter:blur(18px) saturate(1.2);-webkit-backdrop-filter:blur(18px) saturate(1.2);box-shadow:0 20px 50px -20px rgba(0,0,0,.75)}
.mobil-bar a{flex:1;display:flex;align-items:center;justify-content:center;gap:8px;padding:13px 10px;border-radius:11px;font-weight:750;font-size:13px;letter-spacing:.01em}
.mobil-bar svg{width:16px;height:16px}
.mobil-ara{background:color-mix(in srgb,var(--ink) 8%,transparent);color:var(--ink);border:1px solid var(--cizgi)}
.mobil-konum{background:linear-gradient(120deg,var(--accent),color-mix(in srgb,var(--accent) 58%,#12b3a6));color:var(--accent-karsi);box-shadow:0 12px 30px -16px var(--accent)}
@media (max-width:1000px){
  .mobil-bar{display:flex}
  .mobil-cta{display:none}
  body.acil-bar{padding-bottom:96px}
}
/* --- yorum kaydırıcısı --- */
.yorum-kaydirici{position:relative;margin-top:4px}
.yorum-iz{display:flex;gap:18px;overflow-x:auto;scroll-snap-type:x mandatory;scrollbar-width:none;padding:2px 2px 6px}
.yorum-iz::-webkit-scrollbar{display:none}
.yorum-iz .alinti{flex:0 0 clamp(272px,46%,520px);scroll-snap-align:start}
.yorum-kontroller{display:flex;align-items:center;gap:9px;margin-top:14px}
.yk-btn{width:34px;height:34px;border-radius:50%;border:1px solid var(--cizgi);background:var(--kart);color:var(--ink);font-size:18px;line-height:1;cursor:pointer;transition:border-color .3s,background .3s}
.yk-btn:hover{border-color:color-mix(in srgb,var(--accent) 55%,transparent);background:color-mix(in srgb,var(--accent) 10%,transparent)}
.yorum-noktalar{display:flex;align-items:center;gap:6px;margin-left:auto}
.yorum-noktalar button{width:7px;height:7px;border-radius:50%;border:0;padding:0;background:color-mix(in srgb,var(--ink) 22%,transparent);cursor:pointer;transition:width .3s,background .3s}
.yorum-noktalar button.etkin{width:20px;border-radius:99px;background:var(--accent)}
@media (max-width:1000px){.yorum-iz .alinti{flex-basis:86%}}
/* --- yol yardım asistanı --- */
.asistan{position:fixed;right:18px;bottom:18px;z-index:90;display:flex;flex-direction:column;align-items:flex-end;gap:10px}
.asistan-dugme{display:inline-flex;align-items:center;gap:9px;padding:12px 16px;border-radius:999px;border:1px solid color-mix(in srgb,var(--accent) 34%,transparent);background:color-mix(in srgb,var(--bg) 74%,transparent);color:var(--ink);font-weight:700;font-size:12.5px;cursor:pointer;backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);box-shadow:0 18px 40px -22px rgba(0,0,0,.8);transition:transform .3s,border-color .3s}
.asistan-dugme:hover{transform:translateY(-2px);border-color:var(--accent)}
.asistan-nokta{width:8px;height:8px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.6);animation:nabiz 2.4s infinite}
.asistan-panel{width:min(344px,calc(100vw - 28px));padding:16px;border-radius:18px;background:color-mix(in srgb,var(--bg) 92%,transparent);border:1px solid var(--cizgi);backdrop-filter:blur(20px) saturate(1.2);-webkit-backdrop-filter:blur(20px) saturate(1.2);box-shadow:0 30px 70px -30px rgba(0,0,0,.85);animation:asistanAc .28s ease both}
@keyframes asistanAc{from{opacity:0;transform:translateY(12px) scale(.98)}to{opacity:1;transform:none}}
.asistan-panel[hidden]{display:none}
.asistan-ust{display:flex;align-items:center;justify-content:space-between;gap:10px}
.asistan-ust b{font-family:var(--f-baslik);font-size:15px;letter-spacing:-.01em}
.asistan-kapat{width:26px;height:26px;border-radius:50%;border:1px solid var(--cizgi);background:transparent;color:var(--dim);font-size:15px;line-height:1;cursor:pointer}
.asistan-soru{margin:8px 0 12px;font-size:12.5px;line-height:1.55;color:var(--dim)}
.asistan-secim{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.asistan-secim button{padding:11px 8px;border-radius:11px;border:1px solid var(--cizgi);background:var(--kart);color:var(--ink);font-weight:650;font-size:12.5px;cursor:pointer;transition:border-color .25s,background .25s,transform .25s}
.asistan-secim button:hover{border-color:color-mix(in srgb,var(--accent) 52%,transparent);transform:translateY(-1px)}
.asistan-secim button.secili{background:color-mix(in srgb,var(--accent) 16%,transparent);border-color:var(--accent);color:color-mix(in srgb,var(--accent) 72%,var(--ink))}
.asistan-alan{display:block;margin-top:10px}
.asistan-alan span{display:block;font-size:10.5px;letter-spacing:.08em;text-transform:uppercase;color:var(--dim);margin-bottom:5px}
.asistan-alan input{width:100%;padding:10px 12px;border-radius:10px;border:1px solid var(--cizgi);background:color-mix(in srgb,var(--bg) 80%,transparent);color:var(--ink);font:inherit;font-size:12.5px;outline:none}
.asistan-alan input:focus{border-color:var(--accent)}
.asistan-gonder{width:100%;margin-top:11px;padding:13px;border-radius:11px;border:0;background:var(--accent);color:var(--accent-karsi);font-weight:750;font-size:13px;cursor:pointer}
.asistan-gonder:hover{filter:brightness(1.05)}
.asistan-not{min-height:14px;margin:9px 0 0;font-size:11px;line-height:1.5;color:var(--dim)}
@media (max-width:1000px){.asistan{right:12px;bottom:96px}.asistan-dugme{padding:11px 14px;font-size:12px}}
"#;

/// Sayfa davranışının acil katmanı: kaydırıcı, parallax, mobil bar ve asistan.
const EK_JS: &str = r##"<script>
(function () {
  var azalt = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // 1) Yorum kaydırıcısı: oklar, noktalar ve otomatik geçiş.
  var iz = document.getElementById("yorum-iz");
  if (iz && iz.children.length) {
    var kartlar = [].slice.call(iz.children);
    var noktalar = document.getElementById("yorum-noktalar");
    var adim = function () { return kartlar[0].offsetWidth + 18; };
    var sira = function () { return Math.round(iz.scrollLeft / Math.max(1, adim())); };
    var kaydir = function (i) {
      var hedef = kartlar[((i % kartlar.length) + kartlar.length) % kartlar.length];
      if (hedef) iz.scrollTo({ left: hedef.offsetLeft - iz.offsetLeft, behavior: azalt ? "auto" : "smooth" });
    };
    var isaretle = function () {
      if (!noktalar) return;
      var aktif = sira();
      [].forEach.call(noktalar.children, function (b, i) { b.className = i === aktif ? "etkin" : ""; });
    };
    if (noktalar) {
      kartlar.forEach(function (_, i) {
        var b = document.createElement("button");
        b.type = "button";
        b.setAttribute("aria-label", (i + 1) + ". yorum");
        b.addEventListener("click", function () { kaydir(i); });
        noktalar.appendChild(b);
      });
    }
    [].forEach.call(document.querySelectorAll("[data-yon]"), function (b) {
      b.addEventListener("click", function () { kaydir(sira() + Number(b.getAttribute("data-yon"))); });
    });
    iz.addEventListener("scroll", function () { window.requestAnimationFrame(isaretle); }, { passive: true });
    isaretle();
    if (!azalt) {
      var zaman = setInterval(function () {
        if (document.hidden) return;
        kaydir(sira() + 1);
      }, 6500);
      iz.addEventListener("mouseenter", function () { clearInterval(zaman); });
      iz.addEventListener("touchstart", function () { clearInterval(zaman); }, { passive: true });
    }
  }

  // 2) Mobil acil barı varsa gövdeye ek pay bırakılır.
  if (document.querySelector(".mobil-bar")) document.body.classList.add("acil-bar");

  // 3) Parallax: fotoğraf ve galeri kareleri kaydırmayla hafifçe kayar.
  if (!azalt) {
    var kayanlar = [].slice.call(document.querySelectorAll(".sahne.foto, .galeri-kare"));
    if (kayanlar.length) {
      var bekliyor = false;
      window.addEventListener("scroll", function () {
        if (bekliyor) return;
        bekliyor = true;
        window.requestAnimationFrame(function () {
          var d = window.scrollY;
          kayanlar.forEach(function (k) { k.style.setProperty("--kaydir", (d * .055).toFixed(2) + "px"); });
          bekliyor = false;
        });
      }, { passive: true });
    }
  }

  // 4) Yol yardım asistanı: durum + konum → WhatsApp.
  var asistan = document.getElementById("asistan");
  if (asistan) {
    var panel = document.getElementById("asistan-panel");
    var dugme = document.getElementById("asistan-dugme");
    var kapat = document.getElementById("asistan-kapat");
    var gonder = document.getElementById("asistan-gonder");
    var girdi = document.getElementById("asistan-not-girdi");
    var not = document.getElementById("asistan-not");
    var durum = "";
    var acKapa = function () {
      if (!panel) return;
      panel.hidden = !panel.hidden;
      if (dugme) dugme.setAttribute("aria-expanded", String(!panel.hidden));
      if (!panel.hidden && not) not.textContent = "";
    };
    if (dugme) dugme.addEventListener("click", acKapa);
    if (kapat) kapat.addEventListener("click", acKapa);
    [].forEach.call(asistan.querySelectorAll("[data-durum]"), function (b) {
      b.addEventListener("click", function () {
        durum = b.getAttribute("data-durum") || "";
        [].forEach.call(asistan.querySelectorAll("[data-durum]"), function (x) { x.classList.toggle("secili", x === b); });
      });
    });
    var yolla = function (konum) {
      var wa = asistan.getAttribute("data-wa");
      if (!wa) return;
      var metin = "Merhaba, yolda kaldım. Konumum: ";
      if (konum) metin += konum;
      if (durum) metin += " · " + durum;
      var ek = girdi && girdi.value.trim();
      if (ek) metin += " · " + ek;
      window.open("https://wa.me/" + wa + "?text=" + encodeURIComponent(metin), "_blank", "noopener");
      if (not) not.textContent = "WhatsApp açılıyor… Konumu ataç → Konum ile de ekleyebilirsiniz.";
    };
    if (gonder) {
      gonder.addEventListener("click", function () {
        if (not) not.textContent = "Konum alınıyor…";
        if (!navigator.geolocation) { yolla(null); return; }
        var bitti = false;
        navigator.geolocation.getCurrentPosition(
          function (k) {
            if (bitti) return;
            bitti = true;
            yolla("https://maps.google.com/?q=" + k.coords.latitude.toFixed(5) + "," + k.coords.longitude.toFixed(5));
          },
          function () { if (!bitti) { bitti = true; yolla(null); } },
          { timeout: 3500, maximumAge: 60000 }
        );
      });
    }
  }
})();
</script>
"##;

'''
s = s.rstrip() + "\n" + ek
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("ek katman yazildi")
