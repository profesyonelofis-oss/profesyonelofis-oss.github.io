(function(){
  var azalt = matchMedia('(prefers-reduced-motion: reduce)').matches;
  var lerp = function(a,b,t){ return a + (b-a)*t; };

  /* ---------- yükleyici ---------- */
  var lc = document.getElementById('lc'), load = document.getElementById('load'), n = 0;
  (function say(){
    n = Math.min(100, n + Math.ceil(Math.random()*9));
    lc.textContent = n;
    if(n < 100) setTimeout(say, 40); else setTimeout(function(){ load.classList.add('gone'); }, 260);
  })();

  /* ---------- ilerleme ---------- */
  var prog = document.getElementById('prog'), hdr = document.querySelector('header');
  function cubuk(){
    var h = document.documentElement;
    prog.style.width = (h.scrollTop / (h.scrollHeight - h.clientHeight || 1) * 100) + '%';
  }
  addEventListener('scroll', cubuk, {passive:true}); cubuk();

  /* ---------- split metin ---------- */
  document.querySelectorAll('[data-split]').forEach(function(el){
    var t = el.textContent.trim(); el.textContent = '';
    t.split(' ').forEach(function(kelime, i){
      var s = document.createElement('span');
      s.textContent = kelime;
      s.style.transitionDelay = (i * 70) + 'ms';
      el.appendChild(s);
      if(i < t.split(' ').length - 1) el.appendChild(document.createTextNode(' '));
    });
  });

  /* ---------- görünür olunca aç ---------- */
  var io = new IntersectionObserver(function(gs){
    gs.forEach(function(g){ if(g.isIntersecting){ g.target.classList.add('in'); io.unobserve(g.target); } });
  }, {threshold:.2});
  document.querySelectorAll('.split,.reveal-lines').forEach(function(e){ io.observe(e); });

  /* ---------- bölüm rayı + tema ---------- */
  var bölümler = ['s1','s2','s3','s4','s5','s6'].map(function(id){ return document.getElementById(id); }).filter(Boolean);
  var railnav = document.getElementById('railnav');
  bölümler.forEach(function(b, i){
    var d = document.createElement('button');
    d.setAttribute('aria-label', 'bölüm ' + (i+1));
    d.onclick = function(){ b.scrollIntoView({behavior:'smooth'}); };
    railnav.appendChild(d);
  });
  var noktalar = railnav.querySelectorAll('button');
  var temaGozlem = new IntersectionObserver(function(gs){
    gs.forEach(function(g){
      if(!g.isIntersecting) return;
      var i = bölümler.indexOf(g.target);
      noktalar.forEach(function(n2, k){ n2.classList.toggle('on', k === i); });
      var renk = g.target.dataset.accent || '#c8f169';
      document.documentElement.style.setProperty('--accent', renk);
      document.body.classList.toggle('t2', i === 1);
      document.body.classList.toggle('t3', i === 3);
      document.body.classList.toggle('t4', i === 5);
    });
  }, {threshold:.45});
  bölümler.forEach(function(b){ temaGozlem.observe(b); });

  /* ---------- imleç ---------- */
  var imlec = document.getElementById('cur'), halka = document.getElementById('curRing');
  if(matchMedia('(pointer:fine)').matches && !azalt){
    document.body.classList.add('cursor');
    var mx = innerWidth/2, my = innerHeight/2, hx = mx, hy = my;
    addEventListener('mousemove', function(e){ mx = e.clientX; my = e.clientY;
      imlec.style.left = mx + 'px'; imlec.style.top = my + 'px';
    });
    (function takip(){
      hx = lerp(hx, mx, .16); hy = lerp(hy, my, .16);
      halka.style.left = hx + 'px'; halka.style.top = hy + 'px';
      requestAnimationFrame(takip);
    })();
    document.querySelectorAll('[data-cursor="link"]').forEach(function(el){
      el.addEventListener('mouseenter', function(){ halka.style.width = '58px'; halka.style.height = '58px'; });
      el.addEventListener('mouseleave', function(){ halka.style.width = '34px'; halka.style.height = '34px'; });
    });
  }

  /* ---------- mıknatıs ---------- */
  if(!azalt){
    document.querySelectorAll('[data-mag]').forEach(function(b){
      b.addEventListener('mousemove', function(e){
        var r = b.getBoundingClientRect();
        var dx = (e.clientX - (r.left + r.width/2)) * .28;
        var dy = (e.clientY - (r.top + r.height/2)) * .38;
        b.style.transform = 'translate(' + dx + 'px,' + dy + 'px)';
      });
      b.addEventListener('mouseleave', function(){ b.style.transform = 'translate(0,0)'; });
    });
  }

  /* ---------- 3D tilt ---------- */
  document.querySelectorAll('.tilt').forEach(function(k){
    k.addEventListener('mousemove', function(e){
      if(azalt) return;
      var r = k.getBoundingClientRect();
      var px = (e.clientX - r.left) / r.width - .5;
      var py = (e.clientY - r.top) / r.height - .5;
      k.style.transform = 'rotateY(' + (px*14) + 'deg) rotateX(' + (-py*14) + 'deg) translateZ(6px)';
      k.style.background = 'linear-gradient(' + (135 + px*40) + 'deg,#161d19,#090c0a)';
    });
    k.addEventListener('mouseleave', function(){ k.style.transform = 'none'; });
  });

  /* ---------- hero parçacık ---------- */
  var hc = document.getElementById('heroCv'), hx2 = hc.getContext('2d'), parcalar = [];
  function heroBoyut(){
    var d = Math.min(devicePixelRatio || 1, 2);
    hc.width = hc.clientWidth * d; hc.height = hc.clientHeight * d;
    hx2.setTransform(d,0,0,d,0,0);
  }
  if(!azalt){
    heroBoyut(); addEventListener('resize', heroBoyut);
    for(var i=0;i<70;i++) parcalar.push({x:Math.random(),y:Math.random(),s:Math.random()*1.8+.4,v:Math.random()*.0004+.0001});
    (function heroDongu(){
      var w = hc.clientWidth, h = hc.clientHeight;
      hx2.clearRect(0,0,w,h);
      parcalar.forEach(function(p){
        p.y -= p.v * 60;
        if(p.y < 0) p.y = 1;
        hx2.beginPath();
        hx2.arc(p.x*w, p.y*h, p.s, 0, Math.PI*2);
        hx2.fillStyle = 'rgba(200,241,105,' + (.10 + p.s*.12) + ')';
        hx2.fill();
        hx2.beginPath();
        hx2.moveTo(p.x*w, p.y*h);
        hx2.lineTo(p.x*w, p.y*h + 26);
        hx2.strokeStyle = 'rgba(200,241,105,.06)';
        hx2.stroke();
      });
      requestAnimationFrame(heroDongu);
    })();
  }

  /* ---------- scroll-scrub sahne ---------- */
  var sc = document.getElementById('scrubCv'), sx = sc.getContext('2d');
  var frameNo = document.getElementById('frameNo'), meter = document.getElementById('meter');
  var hedefP = 0, p = 0;
  function scrubBoyut(){
    var d = Math.min(devicePixelRatio || 1, 2);
    sc.width = sc.clientWidth * d; sc.height = sc.clientHeight * d;
    sx.setTransform(d,0,0,d,0,0);
  }
  scrubBoyut(); addEventListener('resize', scrubBoyut);

  function ciz(p2){
    var w = sc.clientWidth, h = sc.clientHeight;
    sx.clearRect(0,0,w,h);
    var g = sx.createLinearGradient(0,0,0,h);
    g.addColorStop(0,'#070d0a'); g.addColorStop(.62,'#0a1510'); g.addColorStop(1,'#0c1a13');
    sx.fillStyle = g; sx.fillRect(0,0,w,h);

    // ufuk ve dağlar
    sx.fillStyle = 'rgba(200,241,105,.045)';
    sx.beginPath(); sx.moveTo(0,h*.55);
    for(var i=0;i<=10;i++){ sx.lineTo(w*i/10, h*.55 - Math.sin(i*1.3 + p2*2)*26 - 10); }
    sx.lineTo(w,h); sx.lineTo(0,h); sx.fill();

    // ikinci tepe katmanı (parallax hissi)
    sx.fillStyle = 'rgba(200,241,105,.07)';
    sx.beginPath(); sx.moveTo(0,h*.62);
    for(var j=0;j<=12;j++){ sx.lineTo(w*j/12, h*.62 - Math.cos(j*.9 + p2*3)*32 - 8); }
    sx.lineTo(w,h); sx.lineTo(0,h); sx.fill();

    var yol = h*.74;
    // asfalt
    sx.fillStyle = '#0b0f0c'; sx.fillRect(0, yol, w, h-yol);
    // şerit çizgileri (scroll ile akar)
    sx.strokeStyle = 'rgba(244,241,232,.30)'; sx.lineWidth = 2; sx.setLineDash([34, 26]);
    sx.lineDashOffset = -p2 * 900;
    sx.beginPath(); sx.moveTo(0, yol + (h-yol)*.52); sx.lineTo(w, yol + (h-yol)*.52); sx.stroke();
    sx.setLineDash([]);

    // hız çizgileri
    sx.strokeStyle = 'rgba(200,241,105,' + (.05 + Math.sin(p2*Math.PI)*.22) + ')';
    for(var k=0;k<7;k++){
      var yy = yol - 12 - k*13;
      var len = 30 + ((k*37 + p2*500) % 120);
      sx.beginPath(); sx.moveTo(w*.02 + ((p2*700 + k*140) % w), yy); sx.lineTo(w*.02 + ((p2*700 + k*140) % w) + len, yy); sx.stroke();
    }

    // çekici
    var tx = w*.06 + p2 * (w*.62);
    var ty = yol - 34 + Math.sin(p2*Math.PI*8)*1.6;
    var ol = Math.max(54, Math.min(124, w * (w < 600 ? .17 : .085)));
    // platform
    sx.fillStyle = '#16241c'; sx.fillRect(tx, ty + ol*.55, ol*2.15, ol*.34);
    // kabin
    sx.fillStyle = '#c8f169';
    sx.beginPath();
    sx.moveTo(tx + ol*1.28, ty + ol*.55);
    sx.lineTo(tx + ol*1.28, ty + ol*.22);
    sx.lineTo(tx + ol*1.55, ty + ol*.22);
    sx.lineTo(tx + ol*1.78, ty + ol*.55);
    sx.closePath(); sx.fill();
    // cam
    sx.fillStyle = '#0a1410'; sx.fillRect(tx + ol*1.34, ty + ol*.27, ol*.34, ol*.22);
    // far ışığı
    var fg = sx.createRadialGradient(tx + ol*1.7, ty + ol*.62, 4, tx + ol*1.7, ty + ol*.62, ol*1.5);
    fg.addColorStop(0,'rgba(200,241,105,.20)'); fg.addColorStop(1,'rgba(200,241,105,0)');
    sx.fillStyle = fg; sx.beginPath(); sx.arc(tx + ol*1.7, ty + ol*.62, ol*1.5, 0, Math.PI*2); sx.fill();

    // O-5 tabelası
    sx.fillStyle = '#123024'; sx.fillRect(w - 96, yol - 78, 74, 30);
    sx.strokeStyle = 'rgba(200,241,105,.75)'; sx.lineWidth = 1.5; sx.strokeRect(w - 96, yol - 78, 74, 30);
    sx.fillStyle = '#c8f169'; sx.font = 'bold 14px ui-sans-serif,system-ui';
    sx.fillText('O-5', w - 78, yol - 57);
    sx.fillStyle = '#2b3a31'; sx.fillRect(w - 62, yol - 48, 6, 48);

    // tekerler
    sx.fillStyle = '#060a08';
    [tx + ol*.36, tx + ol*1.52].forEach(function(wx){
      sx.beginPath(); sx.arc(wx, ty + ol*.88, ol*.19, 0, Math.PI*2); sx.fill();
      sx.strokeStyle = '#3a4a40'; sx.lineWidth = 2;
      for(var a=0;a<6;a++){
        var an = a*Math.PI/3 + p2*Math.PI*7;
        sx.beginPath(); sx.moveTo(wx, ty + ol*.88); sx.lineTo(wx + Math.cos(an)*ol*.15, ty + ol*.88 + Math.sin(an)*ol*.15); sx.stroke();
      }
    });
    // tepe lambası
    if(Math.sin(p2*Math.PI*16) > 0){
      sx.fillStyle = 'rgba(255,157,60,.9)'; sx.fillRect(tx + ol*1.45, ty + ol*.14, ol*.12, ol*.07);
      sx.fillStyle = 'rgba(255,157,60,.14)';
      sx.beginPath(); sx.arc(tx + ol*1.51, ty + ol*.17, ol*.30, 0, Math.PI*2); sx.fill();
    }

    // okuma etiketi
    sx.fillStyle = 'rgba(244,241,232,.55)';
    sx.font = '10px ui-monospace,monospace';
    sx.fillText('SCROLL → ZAMAN', 18, h - 14);
  }

  var kutu = document.querySelector('.scrub');
  function scrubOku(){
    var r = kutu.getBoundingClientRect();
    hedefP = Math.min(1, Math.max(0, -r.top / (r.height - innerHeight || 1)));
  }
  addEventListener('scroll', scrubOku, {passive:true}); scrubOku();

  function dongu(){
    if(azalt){ ciz(hedefP); }
    else {
      p = lerp(p, hedefP, .10);
      ciz(p);
      frameNo.textContent = String(Math.round(p*89)).padStart(2,'0');
      meter.style.width = (p*100) + '%';
      requestAnimationFrame(dongu);
    }
  }
  if(azalt){ ciz(0); meter.style.width = '0%'; } else { requestAnimationFrame(dongu); }

  /* ---------- yatay sticky + sürükle ---------- */
  var horiz = document.getElementById('horiz'), track = document.getElementById('htrack');
  var hHedef = 0, hSimdi = 0, suruk = 0, basladi = false, baslaX = 0, sonX = 0;
  function hOlc(){ hHedef = Math.max(0, track.scrollWidth - innerWidth + 24); }
  addEventListener('resize', hOlc); hOlc();
  function hOku(){
    var r = horiz.getBoundingClientRect();
    var oran = Math.min(1, Math.max(0, -r.top / (horiz.offsetHeight - innerHeight || 1)));
    return -oran * hHedef;
  }
  function hDongu(){
    var hedef = hOku() + suruk;
    hSimdi = azalt ? hedef : lerp(hSimdi, hedef, .11);
    track.style.transform = 'translateX(' + hSimdi + 'px)';
    if(!azalt) requestAnimationFrame(hDongu);
  }
  track.addEventListener('pointerdown', function(e){
    basladi = true; baslaX = e.clientX; sonX = suruk;
    track.setPointerCapture(e.pointerId); track.style.transition = 'none';
  });
  track.addEventListener('pointermove', function(e){
    if(!basladi) return;
    suruk = sonX + (e.clientX - baslaX) * -1;
    suruk = Math.min(0, Math.max(-hHedef, suruk));
  });
  track.addEventListener('pointerup', function(){ basladi = false; });
  track.addEventListener('pointercancel', function(){ basladi = false; });
  if(azalt){ track.style.transform = 'none'; } else { requestAnimationFrame(hDongu); }
})();