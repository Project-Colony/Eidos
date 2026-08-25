<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS ve kare üretimi

Community Shaders 1.4+ kendi ölçekleyicisini (ayrı "Upscaling - Community Shaders"
paketi üzerinden DLSS 4 / FSR 3.1 / XeSS) ve FSR 3.1 kare üretimini getirir.
Hepsi Linux'ta Eidos üzerinden çalışır - CS ve paketleri sıradan modlar gibi kurulur
ve birleşim onların DLL'lerini her şey gibi sunar - ama üç şey oyunun içinden
**anlaşılamaz** ve her biri özelliğin sessizce hiçbir şey yapmamasına yol açar. Bu
sayfa onların listesi; gerçek bir kurulumda zor yoldan öğrenildi.

## DLSS'in gereksindiği başlatma seçeneği

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton, oyun Valve'ın izin listesinde değilse NVIDIA NVAPI katmanını (dxvk-nvapi)
devre dışı bırakır ve Skyrim o listede değildir. O olmadan CS, DLSS'i başlatamaz ve
sessizce FSR ölçeklemesine geri düşer; ekranda nedenini söyleyen hiçbir şey olmaz.
Değişkeni ayarlamak NVIDIA olmayan makinelerde hiçbir şeye mal olmaz, bu yüzden
güvenli başlatma seçeneği yukarıdaki satırdır. Kare üretiminin kendisi FSR 3.1'dir
ve NVAPI istemez; onu yalnızca DLSS ölçekleyici ister.

## Kare üretimi kenarlıksız pencere ister

CS'nin kare üretimi bir D3D12 sunum vekili üzerinde çalışır ve tam ekran ayrıcalıklı
kipi düpedüz reddeder. `SkyrimPrefs.ini` içindeki `bFull Screen=1`, hiç devreye
girmeyeceği anlamına gelir - hata yok, ileti yok, yalnızca taban kare hızı. Sağlam
çözüm, INI'ler ne derse desin kipi motor düzeyinde dayatan SSE Display Tweaks'tir:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Pencere birebir aynı görünür (doğal çözünürlükte kenarlıksız); değişen yalnızca
motorun inandığı şeydir - ve motorun inandığı şey, CS'nin denetlediği şeydir.

Aynı sessiz başarısızlıkla iki etkinleştirme koşulu daha:

- **Ekran tazeleme hızı 120 Hz ya da üstü**, ya da CS'nin ölçekleme ayarlarında
  `frameGenerationForceEnable` seçeneğini açın. Kare üretimi sunulan hızı ikiye
  katlar; bu yüzden CS, sonucu gösteremeyecek ekranlarda onu kurmayı reddeder.
- **Upscaling paketi kurulu** olmalı (onun `Data/Shaders/Upscaling/` ağacında
  Streamline ve FidelityFX DLL'leri bulunur). Onsuz CS menü girdilerini gösterir ama
  hiçbir şeyi etkinleştiremez.

## Reflex'in kare hızı sınırı çıktıyı boğabilir

CS'nin Reflex ayarları kendi FPS tavanını taşır (`reflexFPSLimit`, yanında
`reflexUseFPSLimit`). Eski bir değerde kalmış bir tavan - bizimki eski bir ayar
turundan kalma 79'du - kare üretiminin akış aşağısında oturur ve tam da onun ürettiği
kareleri kırpar: taban 60 ikiye katlanıp 120 olur, sonra 79'a geri kırpılır ve "kare
üretimi hiçbir şey yapmıyor" diye okunur. 144 Hz'lik bir ekranda alışılmış Reflex
tavanı ~138'dir. Üretilmiş çıktı eksik göründüğünde her seferinde bakın; tam ekran
ayrıcalıklı kipten sonraki ikinci sessiz katildir.

## Bilinen etkileşim: SSE Display Tweaks ile siyah ekran

FG + Display Tweaks + DXVK bileşiminin bilinen bir siyah ekran arızası var. Sırayla
çözüm:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Yetmezse, oyunun çalıştırılabilir dosyasının yanına bir `dxvk.conf` (bir modun
   `Root/` dizini oraya bir tane koyar) ve içine
   `dxvk.enableGraphicsPipelineLibrary = False`

## Sayıları sonradan okumak

Üretilen kareler yalnızca sunum tarafındadır: motor hâlâ taban hızda benzetim yapar,
Havok hâlâ taban hızda tıklar ve *motor* karelerini sayan her şey (CS'nin kendi
sayaçları dahil) ekran ~120 gösterirken ~60 bildirmeyi sürdürür. Bu bozuk bir sayaç
değil, doğru davranıştır - ve motorun kendi kare hızını yükseltmek güvenli değilken
kare üretiminin fizik açısından güvenli olmasının nedeni tam da budur. Ekranda bir
sayaç isterseniz başlatma seçeneklerindeki `DXVK_HUD=fps` bunu gösterir.

Tek kural: sürücü düzeyinde ara kare üretimi (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) ile CS'nin kare üretimi rakip teknolojilerdir.
Birini ya da ötekini kullanın, asla ikisini birden değil.
