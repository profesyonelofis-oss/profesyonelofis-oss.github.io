from pathlib import Path

p = Path('src/demo_site.rs')
s = p.read_text(encoding='utf-8')

bas = s.index('  /* Scroll-scrub hero videosu:')
son = s.index('  var rozet=document.getElementById("sn-durum");')
yeni = '''  /* Hero videosu: sessiz, otomatik ve kesintisiz döngü. Poster ilk kare
     olarak durur; video hazır olunca başlar. Tarayıcı otomatik oynatmayı
     engellerse ilk dokunuşta devreye girer, sekme geri gelince devam eder.
     Döngü bitişinde (loop desteklenmezse) başa sarılır. */
  var video=document.getElementById("sn-video");
  if(video){
    var kaynak=video.querySelector("source");
    var adres=kaynak&&kaynak.getAttribute("src");
    if(azalt||!adres){K.dataset.video="poster";}
    else{
      var baslangic=parseFloat(getComputedStyle(K).getPropertyValue("--sn-v-bas"))||0;
      var oynat=function(){
        if(video.readyState<2)return;
        var p=video.play();
        if(p&&p.catch)p.catch(function(){});
      };
      video.addEventListener("loadedmetadata",function(){
        K.dataset.video="hazir";
        var sure=video.duration||8;
        if(baslangic>0&&baslangic<sure-0.4){
          try{video.currentTime=baslangic;}catch(e){}
        }
        oynat();
      });
      video.addEventListener("canplay",oynat);
      video.addEventListener("error",function(){K.dataset.video="yok";});
      video.addEventListener("ended",function(){
        try{video.currentTime=0;oynat();}catch(e){}
      });
      document.addEventListener("visibilitychange",function(){
        if(!document.hidden)oynat();
      });
      addEventListener("pointerdown",oynat,{passive:true,once:true});
      // Poster hemen görünür; video kısa süre sonra indirilip başlar.
      setTimeout(function(){video.preload="auto";video.load();},400);
    }
  }

'''
s = s[:bas] + yeni + s[son:]

# Sinematik video testi: autoplay/loop/playsinline ve spotlight oranları.
eski = '''    #[test]
    fn sinematik_iskeleti_donusum_ve_yedekleri_tasir() {'''
yeni_test = '''    #[test]
    fn sinematik_hero_video_kesintisiz_dongude_oynar() {
        let mut g = sinematik_girdi(4);
        g.video = Some(VideoGirdi {
            dosya: "media/hero.mp4".to_string(),
            poster: "media/poster.webp".to_string(),
            atif: None,
        });
        let sayfa = uret(&g);
        for parca in [
            "autoplay",
            "muted",
            "loop",
            "playsinline",
            "preload=\\"auto\\"",
            "disablepictureinpicture",
            "aspect-ratio:16/10",
            "calc(100% + clamp(40px,9vw,180px))",
            "oynat()",
        ] {
            assert!(sayfa.contains(parca), "eksik: {parca}");
        }
        assert!(
            !sayfa.contains("hesapla=function"),
            "kaydırma ile scrub video döngüsünü kesmemeli"
        );
    }

    #[test]
    fn sinematik_iskeleti_donusum_ve_yedekleri_tasir() {'''
assert eski in s
s = s.replace(eski, yeni_test, 1)

p.write_text(s, encoding='utf-8')
print('hero video dongusu guncellendi')
