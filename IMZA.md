# İmzalama ve Dağıtım Hazırlığı

Bu belge `target\release\ollama_proje.exe` dosyasının **Windows kod imzalama sertifikası**
ile imzalanmasını ve dağıtıma hazır hâle getirilmesini anlatır. İmza, dosyanın
değiştirilmediğini kanıtlar ve SmartScreen uyarılarını azaltır.

## 0. Mevcut durum

| Konu | Durum |
|---|---|
| Uygulama ikonu (`.exe` kaynağı) | **Gömülü** — `varliklar/nexus.ico` (16/24/32/48/64/128/256), `build.rs` → `winresource` |
| Pencere / görev çubuğu ikonu | **Gömülü** — `varliklar/nexus_256.png`, `main.rs` → `ViewportBuilder::with_icon` |
| Sürüm bilgisi | `ProductName=Nexus Enterprise`, `FileVersion=0.1.0.0`, `ProductVersion=0.1.0.0` |
| DPI manifesti | `permonitorv2` (build.rs manifesti) |
| Kod imzası | **Yok** (`signtool` sertifikası gerekir) |

Doğrulama komutları:

```powershell
(Get-Item target\release\ollama_proje.exe).VersionInfo | Format-List ProductName,FileDescription,FileVersion
[System.Drawing.Icon]::ExtractAssociatedIcon("$PWD\target\release\ollama_proje.exe")
Get-AuthenticodeSignature target\release\ollama_proje.exe | Format-List Status,SignerCertificate
```

## 1. Sertifika seçimi

| Tür | Kullanım | Not |
|---|---|---|
| **OV** (Organization Validation) | Genel dağıtım | SmartScreen itibarı zamanla oluşur (indirme sayısına bağlı) |
| **EV** (Extended Validation) | Hızlı itibar | Donanım token (HSM/USB) zorunlu, yıllık maliyet yüksek |
| **Azure Trusted Signing** | Bulut tabanlı | Sertifika donanımı yok; `signtool` + `metadata.json` ile imzalanır |

Sertifika sağlayıcıları: DigiCert, Sectigo, SSL.com. Azure Trusted Signing için
Azure aboneliği + kimlik doğrulaması gerekir (bkz. bölüm 4).

## 2. PFX ile imzalama (OV/EV sertifikası)

Sertifika `.pfx` olarak geldiyse:

```powershell
$exe = "target\release\ollama_proje.exe"

# 1) İmzala (SHA-256 + RFC 3161 zaman damgası)
signtool sign /fd SHA256 /f sertifika.pfx /p "$env:SERTIFIKA_SIFRESI" `
  /tr http://timestamp.digicert.com /td SHA256 $exe

# 2) Doğrula
signtool verify /pa /v $exe

# 3) Sonucu gör
Get-AuthenticodeSignature $exe | Format-List Status,StatusMessage,SignerCertificate
```

Beklenen çıktı: `Status = Valid`, `SignerCertificate.Subject` sertifika sahibi.

- `/p` yerine şifreyi komuta gömmemek için `$env:` üzerinden geçirin.
- Zaman damgası (`/tr` + `/td`) olmadan sertifika süresi dolunca imza geçersizleşir.
- EV sertifika donanım tokensa `signtool sign /sha1 <THUMBPRINT> /tr ... $exe` kullanın.

## 3. Trusted Signing (bulut) ile imzalama

```powershell
signtool sign /v /fd SHA256 `
  /tr "http://timestamp.acs.microsoft.com" /td SHA256 `
  /dlib "$env:TRUSTED_SIGNING_DLIB" `
  /dmdf metadata.json $exe
```

`metadata.json` içinde `Endpoint`, `CodeSigningAccountName` ve
`CertificateProfileName` alanları bulunur (Azure portalından alınır).

## 4. Dağıtım öncesi kontrol listesi

1. `cargo fmt --all -- --check` → 0
2. `cargo clippy --all-targets -- -D warnings` → 0
3. `cargo test --all-targets` → tüm testler geçer
4. `cargo build --release` → `target\release\ollama_proje.exe`
5. İmzala + `signtool verify /pa /v`
6. Sürüm numarasını `Cargo.toml` ve `build.rs` içinde **birlikte** artır
7. Dağıtım paketi: `.exe` + `nexus.ico` gerekmez (gömülü); `.env`, `ayarlar.json`,
   `musteri_kayitlari.json` gibi çalışma dosyaları pakete **konmaz**
8. SHA-256 özeti yayınla: `Get-FileHash target\release\ollama_proje.exe -Algorithm SHA256`

## 5. SmartScreen hakkında

- İmza, "bilinmeyen yayıncı" uyarısını **kaldırır**; itibar puanı yine de zamanla oluşur.
- Yeni sertifika + düşük indirme sayısı ilk günlerde "yine de çalıştır" uyarısı gösterebilir.
- Uyarı, imzayı kaldırmakla değil **doğrulanabilir yayıncı + tutarlı dosya adı/sürüm**
  ile azalır. `.exe` adını sürümden sürüme değiştirmeyin.
- Kurumsal dağıtımda imzalı `.exe` + SHA-256 özeti birlikte verilmelidir.

## 6. Sık karşılaşılan hatalar

| Hata | Neden / çözüm |
|---|---|
| `SignTool Error: No certificates were found` | `/f` yolu yanlış ya da sertifika depoya kurulmamış |
| `The specified timestamp server either could not be found` | Farklı TSA dene: `http://timestamp.sectigo.com`, `http://timestamp.acs.microsoft.com` |
| `Access is denied` | `.exe` çalışıyor; imzalamadan önce kapat |
| `Status = NotSigned` | Komut başarısız olmuş; `$LASTEXITCODE` kontrol edilmeli |
