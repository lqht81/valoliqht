# ValoLiqht

VALORANT'ta ajan seçimine girince takımının, maç başlayınca da herkesin silah skinlerini gösteren küçük bir Windows uygulaması.

## Kurulum

[Releases](../../releases) sayfasından birini indir:

- **ValoLiqht-setup.exe**: kurulum dosyası, başlat menüsüne ekler.
- **ValoLiqht-portable.exe**: kurulum yok, çift tıkla çalıştır.

VALORANT'ı oynadığın bilgisayarda çalıştır. Oyun açıkken uygulama kendiliğinden bağlanır.

Windows "bilinmeyen yayıncı" uyarısı verirse **Ek bilgi → Yine de çalıştır** de. Uygulama imzalı değil, o yüzden bu uyarı normal.

## Neler yapar, neler yapmaz

- Ajan seçiminde takım arkadaşlarının, maçta herkesin skinlerini gösterir. Silah seçerek ya da "Tümü" ile bakabilirsin.
- Adını gizleyen oyuncuların adı gösterilmez. Riot'un gizlilik ayarına uyulur.
- Ajan kilitleme gibi oyunu etkileyen hiçbir otomasyon yoktur.
- Telemetri, sayaç ya da otomatik güncelleme yoktur. Uygulama yalnızca şunlara bağlanır:
  - bu bilgisayardaki Riot Client (127.0.0.1)
  - Riot'un oyun sunucuları (`*.a.pvp.net`)
  - skin isimleri ve görselleri için `valorant-api.com`

Riot'un resmi uygulaması değildir. Riot Games bu uygulamayı onaylamamıştır.

## Yeni sürüm çıkarmak

`v0.2.0` gibi bir etiket gönder; GitHub Actions exe dosyalarını derleyip Releases'a ekler.
