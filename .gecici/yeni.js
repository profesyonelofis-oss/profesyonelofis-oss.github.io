(function(){
  'use strict';
  var azalt = matchMedia('(prefers-reduced-motion: reduce)').matches;
  var mobil = matchMedia('(max-width: 760px)').matches;
  var $ = function(s, r){ return (r || document).querySelector(s); };
  var $$ = function(s, r){ return Array.prototype.slice.call((r || document).querySelectorAll(s)); };
  var sinir = function(v, a, b){ return Math.min(b, Math.max(a, v)); };
  var yumusak = function(a, b, x){
    var t = sinir((x - a) / (b - a || 1), 0, 1);
    return t * t * (3 - 2 * t);
  };
  var iki = function(n){ return String(Math.max(0, Math.floor(n))).padStart(2, '0'); };
  function zaman(s){ return iki(s / 60) + ':' + iki(s % 60); }

  /* ---------- video ---------- */
  var film = $('#film'), v = $('#v');
  var dur = 2.0;
  v.src = mobil ? 'video/film-768.mp4?v=3' : 'video/film-1024.mp4?v=3';
  v.load();

  var hazir = false;
  function videoHazir(){ hazir = true; film.classList.add('ready'); oku(true); }
  v.addEventListener('loadeddata', videoHazir);
  v.addEventListener('canplay', function(){ if(!hazir) videoHazir(); });
  v.addEventListener('loadedmetadata', function(){ if(v.duration && isFinite(v.duration)) dur = v.duration; });
  v.addEventListener('error', function(){ film.classList.add('ready'); });

  /* iOS: ilk dokunuşta kilidi aç, sonra durdur (scrub bizden) */
  var kilitAcildi = false;
  function kilitAc(){
    if(kilitAcildi) return; kilitAcildi = true;
    var p = v.play();
    if(p && p.then) p.then(function(){ v.pause(); }).catch(function(){});
    else { try { v.pause(); } catch(e){} }
  }
  addEventListener('touchstart', kilitAc, {once:true, passive:true});
  addEventListener('click', kilitAc, {once:true});

  /* ---------- film ilerlemesi ---------- */
  var chapters = $$('.ch'), rail = $$('#rail span');
  var bar = $('#filmBar'), tcNow = $('#tcNow'), tcAll = $('#tcAll');
  var hedef = 0, simdi = 0, raf = 0, gorunur = true;

  if(tcAll) tcAll.textContent = zaman(dur);

  function ilerleme(){
    var r = film.getBoundingClientRect();
    var toplam = film.offsetHeight - innerHeight;
    if(toplam <= 0) return 0;
    return sinir(-r.top / toplam, 0, 1);
  }

  function metinleriGuncelle(t){
    for(var i = 0; i < chapters.length; i++){
      var el = chapters[i];
      var a = parseFloat(el.dataset['in']), b = parseFloat(el.dataset['out']);
      var yerel = (t - a) / (b - a);
      var giris = yumusak(0.0, 0.10, yerel);
      var cikis = 1 - yumusak(0.90, 0.99, yerel);
      var gor = Math.min(giris, cikis);
      if(yerel < -0.12 || yerel > 1.12) gor = 0;
      el.style.opacity = gor.toFixed(3);
      el.style.transform = 'translate3d(0,' + ((1 - giris) * 24).toFixed(1) + 'px,0)';
      el.style.pointerEvents = gor > 0.55 ? 'auto' : 'none';
      el.setAttribute('aria-hidden', gor > 0.3 ? 'false' : 'true');

      var satirlar = el.querySelectorAll('.ln > span');
      for(var j = 0; j < satirlar.length; j++){
        var k = yumusak(0.0 + j * 0.014, 0.07 + j * 0.014, yerel);
        satirlar[j].style.transform = 'translateY(' + ((1 - k) * 112).toFixed(1) + '%)';
      }
    }
    var aktif = 0;
    for(var c = 0; c < chapters.length; c++){
      if(t >= parseFloat(chapters[c].dataset['in']) - 0.15) aktif = c;
    }
    for(var r = 0; r < rail.length; r++) rail[r].classList.toggle('on', r === aktif);
  }

  function ciz(p){
    if(hazir && Math.abs(v.currentTime - simdi) > 0.03){
      try { v.currentTime = simdi; } catch(e){}
    }
    v.style.transform = 'scale(' + (1.06 - 0.06 * p).toFixed(4) + ')';
    if(bar) bar.style.transform = 'scaleX(' + p.toFixed(4) + ')';
    if(tcNow) tcNow.textContent = zaman(simdi);
    metinleriGuncelle(simdi);
  }

  function adim(){
    raf = 0;
    var p = ilerleme();
    hedef = p * Math.max(0.1, dur - 0.05);
    simdi += (hedef - simdi) * 0.16;
    if(Math.abs(hedef - simdi) < 0.004) simdi = hedef;
    ciz(p);
    if(gorunur) raf = requestAnimationFrame(adim);
  }

  function basla(){ if(!raf && gorunur) raf = requestAnimationFrame(adim); }

  /* rAF'ten bağımsız okuma: gizli sekmede ve hızlı kaydırmada da senkron kalır */
  function oku(hemen){
    var p = ilerleme();
    hedef = p * Math.max(0.1, dur - 0.05);
    if(hemen) simdi = hedef;
    ciz(p);
  }

  addEventListener('scroll', function(){ oku(document.hidden || azalt); }, {passive:true});
  addEventListener('resize', function(){ oku(true); }, {passive:true});
  document.addEventListener('visibilitychange', function(){ if(!document.hidden) oku(true); });

  if(!azalt){
    new IntersectionObserver(function(g){
      gorunur = g[0].isIntersecting;
      if(gorunur) basla();
    }, {threshold: 0}).observe(film);
    oku(true);
    basla();
  } else {
    v.style.transform = 'scale(1)';
    metinleriGuncelle(0);
    if(tcNow) tcNow.textContent = '00:00';
    if(bar) bar.style.transform = 'scaleX(0)';
  }

  /* ---------- reveal ---------- */
  if('IntersectionObserver' in window && !azalt){
    var io = new IntersectionObserver(function(girisler){
      girisler.forEach(function(g){
        if(g.isIntersecting){ g.target.classList.add('in'); io.unobserve(g.target); }
      });
    }, {threshold: 0.16, rootMargin: '0px 0px -8% 0px'});
    $$('.rv').forEach(function(el){ io.observe(el); });
  } else {
    $$('.rv').forEach(function(el){ el.classList.add('in'); });
  }

  /* ---------- header teması ---------- */
  var hd = $('#hd'), temali = $$('[data-hd]');
  function hdTema(){
    var sec = null;
    for(var i = 0; i < temali.length; i++){
      var r = temali[i].getBoundingClientRect();
      if(r.top <= 42 && r.bottom > 42) sec = temali[i];
    }
    hd.classList.toggle('light', !!(sec && sec.dataset.hd === 'light'));
  }
  addEventListener('scroll', hdTema, {passive:true});
  hdTema();

  /* ---------- hat çizgisi ---------- */
  var rota = $('#hat'), dolgu = $('#routeFill');
  if(rota && dolgu && !azalt){
    var rotaRaf = 0;
    function rotaCiz(){
      rotaRaf = 0;
      var r = rota.getBoundingClientRect();
      var p = sinir((innerHeight * 0.82 - r.top) / (r.height * 0.8), 0, 1);
      dolgu.style.setProperty('--f', p.toFixed(3));
    }
    addEventListener('scroll', rotaCiz, {passive:true});
    addEventListener('resize', rotaCiz, {passive:true});
    rotaCiz();
  } else if(dolgu){ dolgu.style.setProperty('--f', '1'); }
})();