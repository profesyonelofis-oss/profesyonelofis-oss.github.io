# Windows açılışında otomatik başlatma (Nexus Enterprise)

Nexus Enterprise bir **masaüstü GUI uygulamasıdır** (Rust + eframe). Arka planda
ayrı bir servis süreci gerektirmez: uygulama açıldığında kendi bağımlılıklarını
kendisi hazırlar.

| Bileşen | Kim başlatır | Başlatılamazsa |
|---|---|---|
| Yerel model (`ollama serve`, 127.0.0.1:11434) | Uygulama açılışında otomatik (`ai::otomatik_baslat`) | Uygulama anahtar sözcük/manuel modda devam eder |
| WhatsApp köprüsü (127.0.0.1:8777) | Gelen Kutusu katmanına girildiğinde + kopmada yeniden | Elle mesaj yapıştırma yolu kullanılır |
| Harita döşemeleri (OSM) | Harita görünür olduğunda, 10 eşzamanlı istek | Çevrimdışı ızgara + pinler |

## 1. Başlangıç klasörüne kısayol (önerilen, en basit)

Uygulamanın Windows oturumu açıldığında sessizce başlaması için:

```powershell
# Ekle
powershell -ExecutionPolicy Bypass -File araclar\baslangica_ekle.ps1

# Kaldır
powershell -ExecutionPolicy Bypass -File araclar\baslangica_ekle.ps1 -Kaldir
```

Betik, `shell:startup` klasörüne `Nexus Enterprise.lnk` kısayolu yazar; hedef
`target\release\ollama_proje.exe`, çalışma dizini exe'nin klasörüdür (`.env`,
`ayarlar.json` ve defter dosyaları oradan okunur). Uygulama `windows_subsystem =
"windows"` ile derlendiği için konsol penceresi açılmaz.

## 2. Görev Zamanlayıcı (gecikmeli / yönetici olarak)

Başlangıçta 30 saniye gecikmeli, yönetici haklarıyla:

```powershell
$exe = (Resolve-Path "target\release\ollama_proje.exe").Path
schtasks /Create /TN "Nexus Enterprise" /SC ONLOGON /DELAY 0000:30 `
  /TR "\"$exe\"" /RL HIGHEST /F

# Kaldır
schtasks /Delete /TN "Nexus Enterprise" /F
```

Görev, oturum açılışında tetiklenir; `ollama serve` zaten uygulama tarafından
başlatıldığı için ayrı bir görev gerekmez.

## 3. Windows Servisi (yalnız headless senaryolar)

Bu uygulama ekran çizen bir masaüstü programıdır; **Windows Servisi olarak
çalıştırmak önerilmez** (servis oturumunda GUI yoktur, ekran çizilemez).
Sunucusuz/başsız bir arka plan süreci gerekirse:

- `windows-service` crate'i ile kendi servis giriş noktanızı yazın
  (<https://docs.rs/windows-service>) veya
- hazır sarmalayıcı kullanın: [NSSM](https://nssm.cc/usage) · [WinSW](https://github.com/winsw/winsw)

Servis olarak yalnız **köprü** çalıştırılacaksa, köprü ayrı bir süreçtir
(`POST /gelen`, `POST /gonder`, `GET /durum`) ve uygulamadan bağımsız
başlatılabilir; ancak mesajları görmek için yine GUI gerekir.

## 4. Sağlık kontrolü ve sorun giderme

| Belirti | Kontrol | Çözüm |
|---|---|---|
| "yerel model yok (ollama serve)" | `curl http://127.0.0.1:11434/api/tags` | `ollama` PATH'te mi? `ollama serve` elle çalıştırın |
| Model yok | `ollama list` | `ollama pull llama3.2` |
| Köprü "dinleyici açılamadı" | `netstat -ano \| findstr 8777` | Portu tutan süreci kapatın; uygulama sıradaki boş portu kendisi seçer |
| Köprü "yeniden bağlandı" mesajı | — | Beklenen davranış: kopma sonrası otomatik toparlanma |
| Açılışta hata kutusu | `%EXE_KLASÖRÜ%\nexus_hata.log` | Günlükteki yığın izi ve konum satırına bakın |

## 5. Kaldırma

1. `araclar\baslangica_ekle.ps1 -Kaldir` (kısayol)
2. `schtasks /Delete /TN "Nexus Enterprise" /F` (görev zamanlayıcı)
3. Uygulamayı kapatın; `ollama serve` süreci arka planda kalırsa
   `Get-Process ollama | Stop-Process` ile sonlandırın.
