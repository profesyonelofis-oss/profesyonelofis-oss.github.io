(function(){
  'use strict';
  if('scrollRestoration' in history) history.scrollRestoration = 'manual';
  var azalt = matchMedia('(prefers-reduced-motion: reduce)').matches;

  var slaytlar = Array.prototype.slice.call(document.querySelectorAll('.slayt'));
  var A = document.getElementById('cvA'), B = document.getElementById('cvB');
  var aktif = A, onde = 0;
  var bar = document.getElementById('bar');
  var noEl = document.getElementById('sahneNo'), adEl = document.getElementById('sahneAd');
  var sahneler = [], idx = -1, mesgul = false, oynatIstek = 0, sonIm = null;

  function katHazir(c){
    var d = Math.min(devicePixelRatio || 1, 2);
    var gw = Math.round(c.clientWidth * d), gh = Math.round(c.clientHeight * d);
    if(gw && gh && (c.width !== gw || c.height !== gh)){ c.width = gw; c.height = gh; }
    var k = c.getContext('2d');
    if(k) k.setTransform(d,0,0,d,0,0);
    return k;
  }
  function katBoyut(){ katHazir(A); katHazir(B); }

  function resimCiz(c, im){
    var w = c.clientWidth, h = c.clientHeight;
    if(!w || !h || !im || !im.naturalWidth) return;
    var k = katHazir(c);
    if(!k) return;
    var sc = Math.max(w/im.naturalWidth, h/im.naturalHeight);
    var dw = im.naturalWidth*sc, dh = im.naturalHeight*sc;
    k.clearRect(0,0,w,h);
    k.drawImage(im, (w-dw)/2, (h-dh)/2, dw, dh);
  }

  function kareYolu(s, i){ return 'sahneler/' + s.klasor + '/kare_' + String(i).padStart(4,'0') + '.jpg'; }

  function yukle(s){
    if(!s || s._imgs) return;
    s._imgs = [];
    for(var i=0;i<s.kare;i++){
      var im = new Image();
      im.decoding = 'async';
      im.src = kareYolu(s, i);
      s._imgs.push(im);
    }
  }

  function oynat(i, c){
    if(oynatIstek) cancelAnimationFrame(oynatIstek);
    var s = sahneler[i];
    if(!s || !s._imgs || azalt) return;
    var sure = Math.min(4600, Math.max(1500, s.kare*54));
    var t0 = performance.now();
    (function adim(t){
      var p = Math.min(1, (t - t0)/sure);
      var im = s._imgs[Math.min(s.kare-1, Math.round(p*(s.kare-1)))];
      if(im && im.complete && im.naturalWidth){ resimCiz(c, im); sonIm = im; }
      if(p < 1) oynatIstek = requestAnimationFrame(adim);
    })(t0);
  }

  function etiket(i){
    if(noEl) noEl.textContent = String(i+1).padStart(2,'0');
    if(adEl && sahneler[i]) adEl.textContent = sahneler[i].ad;
    if(bar) bar.style.width = (((i+1)/Math.max(1,sahneler.length))*100) + '%';
    document.querySelectorAll('.nav a').forEach(function(a){
      a.classList.toggle('on', +a.dataset.git === i);
    });
  }

  function git(i, zorla){
    if(!sahneler.length) return;
    if(i < 0 || i >= sahneler.length) return;
    if(mesgul && !zorla) return;
    if(i === idx) return;
    mesgul = true;

    var s = sahneler[i];
    yukle(s);
    var gelen = onde ? A : B, giden = onde ? B : A;
    var im = s._imgs[0];

    /* içerik aynı anda değişir */
    slaytlar.forEach(function(el, k){ el.classList.toggle('on', k === i); });
    etiket(i);

    function bitir(){
      resimCiz(gelen, im); sonIm = im;
      gelen.classList.add('on');
      giden.classList.remove('on');
      onde = 1 - onde; aktif = gelen; idx = i;
      oynat(i, gelen);
      yukle(sahneler[i+1]);           /* sonraki sahneyi önden hazırla */
      setTimeout(function(){ mesgul = false; }, 820);
    }
    if(im && im.complete && im.naturalWidth){ bitir(); }
    else if(im){
      var bitti = false;
      var tek = function(){ if(bitti) return; bitti = true; bitir(); };
      im.addEventListener('load', tek);
      im.addEventListener('error', tek);
      setTimeout(tek, 900);
    }
    else { bitir(); }
  }

  /* ---------- sahne listesi ---------- */
  fetch('sahneler/manifest.json', {cache:'no-store'})
    .then(function(r){ return r.ok ? r.json() : null; })
    .then(function(m){
      if(!m || !m.sahneler || !m.sahneler.length) throw new Error('sahne yok');
      sahneler = m.sahneler;
      katBoyut();
      git(0, true);
      yukle(sahneler[1]);
    })
    .catch(function(){
      /* video yoksa da içerik görünsün */
      slaytlar[0].classList.add('on');
      if(bar) bar.style.width = '14%';
    });

  addEventListener('resize', function(){
    katBoyut();
    if(sonIm) resimCiz(aktif, sonIm);
  });

  /* ---------- tekerlek: her tik = yeni sahne ---------- */
  var birikim = 0;
  addEventListener('wheel', function(e){
    e.preventDefault();
    if(mesgul || e.ctrlKey) return;
    birikim += e.deltaY;
    if(Math.abs(birikim) < 34) return;
    var yon = birikim > 0 ? 1 : -1;
    birikim = 0;
    git(idx + yon);
  }, {passive:false});

  /* ---------- dokunma ---------- */
  var basY = 0;
  addEventListener('touchstart', function(e){ basY = e.touches[0].clientY; }, {passive:true});
  addEventListener('touchmove', function(e){ e.preventDefault(); }, {passive:false});
  addEventListener('touchend', function(e){
    if(!e.changedTouches || !e.changedTouches[0]) return;
    var fark = basY - e.changedTouches[0].clientY;
    if(Math.abs(fark) < 46) return;
    git(idx + (fark > 0 ? 1 : -1));
  }, {passive:true});

  /* ---------- klavye ---------- */
  addEventListener('keydown', function(e){
    if(e.key === 'ArrowDown' || e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' '){
      e.preventDefault(); git(idx + 1);
    } else if(e.key === 'ArrowUp' || e.key === 'ArrowLeft' || e.key === 'PageUp'){
      e.preventDefault(); git(idx - 1);
    } else if(e.key === 'Home'){ e.preventDefault(); git(0); }
    else if(e.key === 'End'){ e.preventDefault(); git(sahneler.length - 1); }
  });

  /* ---------- nav tıklamaları ---------- */
  document.querySelectorAll('[data-git]').forEach(function(a){
    a.addEventListener('click', function(e){
      var i = +a.dataset.git;
      if(isNaN(i) || !sahneler.length) return;
      e.preventDefault();
      git(i);
    });
  });

  /* ---------- mıknatıslı düğmeler ---------- */
  if(!azalt){
    document.querySelectorAll('[data-mag]').forEach(function(b){
      b.addEventListener('pointermove', function(e){
        if(e.pointerType === 'touch') return;
        var r = b.getBoundingClientRect();
        var dx = (e.clientX - (r.left + r.width/2)) * .16;
        var dy = (e.clientY - (r.top + r.height/2)) * .26;
        b.style.transform = 'translate(' + dx + 'px,' + dy + 'px)';
      });
      b.addEventListener('pointerleave', function(){ b.style.transform = ''; });
    });
  }
})();