<!-- eidos-i18n: source=docs/guide/graphics.md sha=1955cd62075925e7e97c47c7866e9ce2eceba1c2 -->

# Community Shaders, DLSS ve kare üretimi

Community Shaders 1.4+ kendi ölçekleyicisini (ayrı "Upscaling - Community Shaders"
paketi üzerinden DLSS 4 / FSR 3.1 / XeSS) ve FSR 3.1 kare üretimini getirir.
Hepsi Linux'ta Eidos üzerinden çalışır - CS ve paketleri sıradan modlar gibi kurulur
ve birleşim onların DLL'lerini her şey gibi sunar - ama üç şey oyunun içinden
**anlaşılamaz** ve her biri özelliğin sessizce hiçbir şey yapmamasına yol açar. Bu
sayfa onların listesi; gerçek bir kurulumda zor yoldan öğrenildi.

Sağlık sekmesi bunlardan üçünü sizin yerinize denetler: bir NVIDIA sürücüsü ve Upscaling paketi varken ve oyunun Proton sürümü NVAPI'yi varsayılan olarak kapalı bırakırken NVAPI değişkenini (GE-Proton11-9 ve Proton 11.0 onu açar), profilin `SkyrimPrefs.ini` dosyasındaki özel tam ekranı ve aşağıda anlatılan ENB satırını. Son ikisinin Düzelt düğmesi vardır; Diagnostics'teki Ekran kartı da kenarlıksız pencereyi bir boyutla birlikte ayarlar.

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

## Karisik doku modlari: PGPatcher

Community Shaders parallax, complex material ve PBR'yi isleyebilir, ama yalnizca
bir mesh bunun icin hazirlanmissa. Eskiden bu, onceden yamalanmis mesh'ler
kurmak ve ardindan dokularin eksik oldugu her yerde etkiyi yeniden kapatan bir
mod kurmak demekti: kapsam elinizdeki mesh'lerle sinirli kalir ve calisma
zamaninda, hic verilmemesi gereken bir kararı geri almak icin islemci zamani
harcanirdi.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) bunu
tersine cevirir. Istediginiz dokulari, herhangi bir shader turunde kurarsiniz;
o da her yuzeyin dogrusunu kullanmasi icin mesh'leri ve eklentileri yeniden
yazar - eski yontemin kapsamadigi, eklentilerdeki alternatif doku kayitlari
dahil. Bu, gorunen bir doku paketi ile yalnizca yuklenen bir paket arasindaki
farktir ve en cok karisik bir kurulumda onem tasir, ki her gercek yukleme sirasi
zaten oyledir.

Linux'ta calistirmadan once iki sey:

- **`--ignore-mo2vfscheck`** argumani gerekir, yoksa MO2'den yakinarak aninda
  cikar. Eidos bunu saglar - bayragin ne oldugu ve denetimin burada neden
  basarisiz oldugu icin [Araclar](tools.md#pgpatcher-neden---ignore-mo2vfscheck-ister) sayfasina bakin;
- mesh'leri degistirir, bu yuzden tum doku modlari kurulduktan **sonra** ve
  nihai mesh'leri gormesi gereken DynDOLOD'dan **once** calisir. Dokularinizi
  sonradan degistirmek, ikisini de bu sirayla yeniden calistirmak demektir.

Çıktısı Overwrite'a gitmez. PGPatcher yalnızca başlatıcısındaki Output location
alanında ayarlı klasöre yazar ve `Data` içindeki bir klasörü kabul etmez. Bu
alana kendine ait bir klasör verin, mesela örneğe ait `mods/` altında yeni bir
`PGPatcher Output` klasörü. PGPatcher çalışırken o modu kapalı tutun, çünkü
PGPatcher klasörü yeniden yazmadan önce boşaltır ve Eidos, görünüm bağlıyken
etkin modların değişmediğini varsayar. Ardından yüksek öncelikle etkinleştirin:
kazanmak için yapılmıştır.

**Capture output into** ile seçilen bir modu Output location olarak kullanmayın.
Eidos her çalıştırmadan sonra PGPatcher'ın kendi `cfg/` ve `log/` klasörlerini o
moda taşır ve PGPatcher ardından
"Output directory has non-PGPatcher related files" hatasıyla durur.

## Proton altında ENB: LinuxVersion

Jackify, Proton altında kurduğu her ENB için `enblocal.ini` dosyasının `[GLOBAL]` bölümüne `LinuxVersion=true` yazar. ENB'nin kendi belgeleri bu anahtarı anlatmaz. Sağlık sekmesi bu satırı olmayan bir `enblocal.ini` dosyasını bildirir; Düzelt düğmesi kazanan `enblocal.ini` dosyasının satır eklenmiş bir kopyasını `overwrite/Root/` içine yazar, böylece ENB modu kendi dosyasını korur.

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
