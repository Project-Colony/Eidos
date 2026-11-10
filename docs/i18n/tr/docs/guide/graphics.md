<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS ve kare üretimi

Community Shaders 1.4+ kendi ölçekleyicisini (ayrı "Upscaling - Community Shaders"
paketi üzerinden DLSS 4 / FSR 3.1 / XeSS) ve FSR 3.1 kare üretimini getirir.
Hepsi Linux'ta Eidos üzerinden çalışır: CS ve paketleri sıradan modlar gibi kurulur
ve birleşim onların DLL'lerini her dosya gibi sunar. Ama üç ayar oyunun içinden
görülemez ve her biri özelliğin hata vermeden hiçbir şey yapmamasına yol açar:
NVAPI değişkeni, ayrıcalıklı tam ekran ve Reflex FPS tavanı.

Diagnostics sekmesi ilk ikisini ve aşağıdaki ENB satırını, bir Wine önekine
(Steam'in Proton öneki ya da Heroic'in kurduğu) sahip oyun için denetler. Reflex
tavanı denetlenmez. Örnek oyunu `Stock Game` gibi bir kopyadan çalıştırıyorsa
denetimler mağaza kurulumunu değil, kopyanın dosyalarını okur.
`eidos check <instance>` aynı kartları bir terminalde, Fix düğmeleri olmadan
yazdırır.

- NVAPI kartı, Skyrim SE ya da Enderal SE'nin bir Steam kopyası için şu koşullarda
  görünür: bir NVIDIA sürücüsü yüklü (`/proc/driver/nvidia/version` var), birleşik
  `Data` içinde `Shaders/Upscaling` altında bir şey var, oyunun Proton sürümü NVAPI'yi
  varsayılan olarak kapalı bırakıyor ve ne Steam'in başlatma seçeneği, ne Eidos'un
  kendi ortamı, ne de profilin `launch-env.ini` dosyası `PROTON_ENABLE_NVAPI=1`
  ayarlıyor.
- Ayrıcalıklı tam ekran kartı yalnızca Upscaling paketi kuruluyken, profilin
  `SkyrimPrefs.ini` dosyasında `bFull Screen=1` varsa ve
  `SKSE/Plugins/SSEDisplayTweaks.ini` hem `Fullscreen=false` hem
  `Borderless=true` ayarlamıyorsa görünür. Fix düğmesi profilin `SkyrimPrefs.ini`
  dosyasına `bFull Screen=0` ve `bBorderless=1` yazar. Denetim Enderal SE için de
  `SkyrimPrefs.ini` okur, oysa bir Enderal SE profili `EnderalPrefs.ini` taşır;
  bu yüzden Enderal SE'de bunun yerine aşağıdaki Display kartını kullanın.
- ENB kartı, kök görünümde kazanan `enblocal.ini` dosyasının `[GLOBAL]` bölümünde
  `LinuxVersion=true` olmayan her oyun için görünür. Onun da bir Fix düğmesi var.

## DLSS'in gereksindiği başlatma seçeneği

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Kendi `eidos-gui` ya da `eidos-linux` dosyanızın mutlak yolunu kullanın,
[install.md](install.md) içindeki gibi. Kart ayarlanacak satırı verir: oyunun
mevcut başlatma seçeneğinin önüne `PROTON_ENABLE_NVAPI=1` konmuş hâli, hiç yoksa
çalışan Eidos'un yolundan kurulan satır.

NVIDIA NVAPI katmanı (dxvk-nvapi) olmadan CS, DLSS'i başlatamaz ve ekranda nedenini
söyleyen hiçbir şey olmadan FSR ölçeklemesine geri düşer. Bir Proton sürümünün
katmanı kendiliğinden açıp açmadığı kendi `proton` betiğinde yazar ve Eidos oradaki
`use_nvapi` satırını okur. Bir vazgeçme anahtarı (`disablenvapi`) varsayılan olarak
açık demektir: GE-Proton11-9 ve Proton 11.0 bu biçimi taşır (2026-11-09'da
denetlendi). Bir katılma anahtarı (`enablenvapi`) kapalı demektir ve yalnızca o
sürüm değişkene ihtiyaç duyar. Değişkeni ayarlamak NVIDIA olmayan makinelerde hiçbir
şeye mal olmaz. Kare üretiminin kendisi FSR 3.1'dir ve NVAPI istemez; onu yalnızca
DLSS ölçekleyici ister.

Bunu yalnızca tek bir profil için ayarlamak istiyorsanız, `env=PROTON_ENABLE_NVAPI=1`
satırını o profilin `launch-env.ini` dosyasına yazın (View -> INI editor, son sekme).

## Kare üretimi kenarlıksız pencere ister

CS'nin kare üretimi bir D3D12 sunum vekili üzerinde çalışır ve ayrıcalıklı tam ekranı
düpedüz reddeder. `SkyrimPrefs.ini` içinde `bFull Screen=1` varken hiç devreye
girmez: hata yok, ileti yok, oyun taban kare hızında kalır. SSE Display Tweaks,
INI'ler ne derse desin kipi motor düzeyinde ayarlar:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Pencere birebir aynı görünür (doğal çözünürlükte kenarlıksız). Değişen yalnızca
motorun bildirdiği kiptir ve CS'nin denetlediği şey o kiptir.

Diagnostics'teki Display kartı boyutu ve kipi tek adımda ayarlar; Skyrim SE,
Enderal SE ve Fallout 4 için. Bağlı en büyük ekranın boyutunu gösterir. Borderless
window seçiliyken Apply, profilin `SkyrimPrefs.ini` dosyasına (Enderal SE için
`EnderalPrefs.ini`, Fallout 4 için `Fallout4Prefs.ini`) `iSize W`, `iSize H`,
`bFull Screen=0` ve `bBorderless=1` yazar. Pencereyi SSE Display Tweaks
ayarlıyorsa Apply ayrıca `Resolution`, `Fullscreen=false` ve `Borderless=true`
değerlerini onun INI'sinin Overwrite içindeki bir kopyasına
(`SKSE/Plugins/SSEDisplayTweaks.ini`) yazar; modun kendi dosyası olduğu gibi kalır.
Start the game at once when a store opens Eidos açıkken (Preferences, General,
Running a game), profilde henüz bir pencere boyutu yoksa ve SSE Display Tweaks
pencereyi ayarlamıyorsa pencere yine de açılır; böylece boyut ilk başlatmadan önce
ayarlanabilir.

Aynı sessiz başarısızlıkla iki etkinleştirme koşulu daha:

- Ekran tazeleme hızı 120 Hz ya da üstü, ya da CS'nin ölçekleme ayarlarında
  `frameGenerationForceEnable` açık. Kare üretimi sunulan hızı ikiye katlar; bu
  yüzden CS, sonucu gösteremeyecek ekranlarda onu kurmayı reddeder.
- Upscaling paketi kurulu (onun `Data/Shaders/Upscaling/` ağacında Streamline ve
  FidelityFX DLL'leri bulunur). Onsuz CS menü girdilerini gösterir ama hiçbir şeyi
  etkinleştiremez.

## Reflex'in kare hızı sınırı çıktıyı boğabilir

CS'nin Reflex ayarları kendi FPS tavanını taşır (`reflexFPSLimit`, yanında
`reflexUseFPSLimit`). Eski bir değerde kalmış bir tavan (bizimki eski bir ayar
turundan kalma 79'du) kare üretiminin akış aşağısında oturur ve tam da onun ürettiği
kareleri kırpar: taban 60 ikiye katlanıp 120 olur, sonra 79'a geri kırpılır ve "kare
üretimi hiçbir şey yapmıyor" diye okunur. 144 Hz'lik bir ekranda alışılmış Reflex
tavanı ~138'dir. Üretilmiş kareler eksik göründüğünde ona bakın.

## Bilinen etkileşim: SSE Display Tweaks ile siyah ekran

FG + Display Tweaks + DXVK bileşiminin bilinen bir siyah ekran arızası var. Sırayla
çözüm:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Yetmezse, oyunun çalıştırılabilir dosyasının yanına bir `dxvk.conf` (bir modun
   `Root/` dizini oraya bir tane koyar) ve içine
   `dxvk.enableGraphicsPipelineLibrary = False`

## Karışık doku modları: PGPatcher

Community Shaders parallax, complex material ve PBR'yi işleyebilir, ama yalnızca
bir mesh bunun için hazırlanmışsa. Eskiden bu, önceden yamalanmış mesh'ler
kurmak ve ardından dokuların eksik olduğu her yerde etkiyi yeniden kapatan bir
mod kurmak demekti: kapsam elinizdeki mesh'lerle sınırlı kalır ve çalışma
zamanında, kurulum sırasında verilmiş bir kararı geri almak için işlemci zamanı
harcanırdı.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) bunu
tersine çevirir. İstediğiniz dokuları, herhangi bir shader türünde kurarsınız;
o da her yüzeyin doğrusunu kullanması için mesh'leri ve eklentileri yeniden
yazar; eski yöntemin kapsamadığı, eklentilerdeki alternatif doku kayıtları da
buna dahildir. Onsuz, mesh'inin istemediği bir shader'a ihtiyaç duyan bir doku
yüklenir ama istendiği gibi işlenmez; bu en çok doku türlerini karıştıran bir
kurulumda olur.

Linux'ta çalıştırmadan önce iki şey:

- `--ignore-mo2vfscheck` argümanı gerekir, yoksa MO2'den yakınarak anında
  çıkar. Eidos bunu sağlar; bayrağın ne olduğu ve denetimin burada neden
  başarısız olduğu için [Araçlar](tools.md#pgpatcher-neden---ignore-mo2vfscheck-ister)
  sayfasına bakın;
- mesh'leri değiştirir, bu yüzden tüm doku modları kurulduktan sonra ve
  nihai mesh'leri görmesi gereken DynDOLOD'dan önce çalışır. Dokularınızı
  sonradan değiştirmek, ikisini de bu sırayla yeniden çalıştırmak demektir.

Çıktısı Overwrite'a gitmez. PGPatcher yalnızca başlatıcısındaki Output location
alanında ayarlı klasöre yazar ve `Data` içindeki bir klasörü kabul etmez. Bu
alana kendine ait bir klasör verin, mesela örneğe ait `mods/` altında yeni bir
`PGPatcher Output` klasörü. PGPatcher çalışırken o modu kapalı tutun, çünkü
PGPatcher klasörü yeniden yazmadan önce boşaltır ve Eidos, görünüm bağlıyken
etkin modların değişmediğini varsayar. Ardından yüksek öncelikle etkinleştirin:
kazanmak için yapılmıştır.

PGPatcher'ın girdisine bir Capture output into modu vermeyin ve sonra o modu
Output location olarak kullanmayın. Eidos her çalıştırmadan sonra PGPatcher'ın
kendi `cfg/` ve `log/` klasörlerini o moda taşır ve PGPatcher ardından
"Output directory has non-PGPatcher related files" hatasıyla durur.

## Proton altında ENB: LinuxVersion

Jackify, Proton altında kurduğu her ENB için `enblocal.ini` dosyasının `[GLOBAL]`
bölümüne `LinuxVersion=true` yazar. ENB'nin kendi belgeleri bu anahtarı anlatmaz.
Diagnostics sekmesindeki bir Health kartı bu satırı olmayan bir `enblocal.ini`
dosyasını bildirir; Fix düğmesi kazanan `enblocal.ini` dosyasının satır eklenmiş bir
kopyasını `overwrite/Root/` içine yazar, böylece ENB modu kendi dosyasını korur.

Başka bir kart, oyun klasöründe bir ENB bulunurken (bir `enblocal.ini`, bir
`enbseries.ini` ya da bir `enbseries/` klasörü) etkin bir modun Community Shaders
getirdiğini (`SKSE/Plugins/CommunityShaders.dll`) söyler. İkisi birlikte
çalışabilir. Görüntü yanlış görünüyorsa birini kendi INI'sinden kapatın.

## Sayıları sonradan okumak

Üretilen kareler yalnızca sunum tarafındadır: motor hâlâ taban hızda benzetim yapar,
Havok hâlâ taban hızda tıklar ve *motor* karelerini sayan her şey (CS'nin kendi
sayaçları dahil) ekran ~120 gösterirken ~60 bildirmeyi sürdürür. Sayaç doğrudur ve
motorun kendi kare hızını yükseltmek güvenli değilken kare üretiminin fizik açısından
güvenli olmasının nedeni budur. Ekranda bir sayaç isterseniz başlatma seçeneklerindeki
`DXVK_HUD=fps` bunu gösterir.

Tek kural: sürücü düzeyinde ara kare üretimi (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) ile CS'nin kare üretimi rakip teknolojilerdir.
Birini ya da ötekini kullanın, asla ikisini birden değil.
