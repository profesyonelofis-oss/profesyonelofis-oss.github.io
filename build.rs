//! Windows kaynak derlemesi: `.exe` dosya ikonu ve sürüm bilgisi.
//!
//! `cargo build --release` çıktısındaki `ollama_proje.exe`, Explorer'da ve
//! görev çubuğunda `varliklar/nexus.ico` ikonunu taşır; "Özellikler →
//! Ayrıntılar" sekmesinde ürün adı ve sürüm görünür.
//!
//! Diğer platformlarda (Linux/macOS) bu adım hiçbir şey yapmaz: ikon orada
//! pencere ikonu olarak çalışır. Kaynak derleyici (rc.exe) yoksa derleme
//! kırılmaz; uyarı basılır ve devam edilir.

fn main() {
    println!("cargo:rerun-if-changed=varliklar/nexus.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut kaynak = winresource::WindowsResource::new();
        kaynak.set_icon("varliklar/nexus.ico");
        kaynak.set("ProductName", "Nexus Enterprise");
        // Sürüm kaynağı rc.exe tarafından ANSI kod sayfasıyla yazılır; Türkçe
        // karakterler Özellikler penceresinde bozuk görünür, bu yüzden ASCII.
        kaynak.set(
            "FileDescription",
            "Nexus Enterprise - Isletme Kesif ve Demo Otomasyonu",
        );
        kaynak.set("CompanyName", "Nexus Enterprise");
        kaynak.set("LegalCopyright", "(c) 2026 Nexus Enterprise");
        kaynak.set("OriginalFilename", "NexusEnterprise.exe");
        kaynak.set("FileVersion", "0.1.0.0");
        kaynak.set("ProductVersion", "0.1.0.0");
        kaynak.set_manifest(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="0.1.0.0" processorArchitecture="*" name="NexusEnterprise" type="win32"/>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>"#,
        );
        if let Err(hata) = kaynak.compile() {
            println!("cargo:warning=Windows ikon kaynagi derlenemedi: {hata}");
        }
    }
}
