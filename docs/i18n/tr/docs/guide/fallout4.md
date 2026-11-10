<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Eidos üzerinden Fallout 4

Eidos ile Fallout 4'ün özel bir başlatma seçeneğine, adı değiştirilmiş bir
çalıştırılabilir dosyaya ya da sarmalayıcı betiğe gereksinimi yoktur. Her
diğer oyunla aynı başlatma seçeneğini kullanır.

## Başlatma seçeneği

```
~/.local/bin/eidos-gui %command%
```

Steam'in Fallout 4 için başlatma hedefi `Fallout4Launcher.exe`'dir, asla
`Fallout4.exe` değil; dolayısıyla script extender'ı çalıştırmak aslında "Steam'e
nasıl başka bir program başlattırırım" sorusudur. Alışılmış yanıtlar `%command%`
ifadesini bash içinde yeniden yazar:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

ya da `f4se_loader.exe` dosyasını `Fallout4Launcher.exe` üzerine kopyalar; Steam bunu
her oyun güncellemesinde geri getirir. Ondan sonra F4SE olmadan oynarsınız ve
hiçbir şey bunu söylemez.

Eidos değiştirmeyi oyun tanımlayıcısından yola çıkarak kendi yapar. Pencere
oyunu başlattığında, yükleyici oyun klasöründe, etkin bir modun `Root/` ya da
`Overwrite/Root/` dizinindeyse `Fallout4Launcher.exe` yerine `f4se_loader.exe`
çalıştırır. Yükleyici yoksa `Fallout4.exe` çalışır ve durum satırı şunu yazar:
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Orada iki dosya da yoksa başlatıcıyı değiştirmeden çalıştırır ve bunu da söyler.
Başlatma seçeneği bunun yerine `eidos play fallout4 -- %command%` ise CLI
yalnızca yükleyici varsa değiştirir, yoksa `Fallout4Launcher.exe` çalıştırır.

Başlatıcıdan kaçınmak için ikinci bir neden daha var: `Data`'yı yeniden tarayıp
`plugins.txt` dosyasını yeniden yazar ve az önce yerleştirilen yükleme sırasını
bozar.

Pencereyi kendiniz açıp Run'a bastığınızda Eidos, Steam'den oyunu başlatmasını
ister (`steam -applaunch 377160`) ve Steam, Eidos'u yukarıdaki başlatma
seçeneğiyle başlatır. `f4se_loader.exe` örnekteki oyunun çalıştırılabilir
dosyasının yanında değilse bunu yapmaz ve durum satırı önce script extender'ı
kurmanızı ister. Fallout 4 VR aynı şekilde çalışır; dosyalar
`Fallout4VRLauncher.exe` ve `f4sevr_loader.exe` olur.

## Eidos'un sizin adınıza üstlendikleri

| Alan | Eidos ne yapar |
|---|---|
| Arşiv geçersizleştirme | `Fallout4Custom.ini` dosyasına `[Archive]` `bInvalidateOlderFiles=1` ve boş bir `sResourceDataDirsFinal=` yazılır; `Data` dışındaki serbest dosyaların görülebilmesini sağlayan iki anahtar. Oyun klasörüne değil, profile yazılır. |
| INI'ler | `Fallout4.ini`, `Fallout4Prefs.ini` ve `Fallout4Custom.ini` profile aittir. Eidos üzerinden çalıştırılan BethINI Pie profilin kopyalarını düzenler; Backups -> INIs, Eidos'un her çalıştırmadan önce tuttuğu kopyayı geri koyar ([tools.md](tools.md#bethini-pie)). |
| Yükleme sırası | Fallout 4'ün kullandığı yıldız biçiminde `plugins.txt` (`*` etkin demek), örtük Creation Club eklentileri için `Fallout4.ccc` gözetilerek |
| LOOT | Sıralama Skyrim'deki gibi çalışır - `eidos sort <instance>` `fallout4` masterlist'ini alır |
| Kayıtlar | `.fos` kayıtları ve `.f4se` yan kayıtları listelenir, kopyalanır ve profil başına tutulur; ayrıntı bölmesi kaydın kendi eklenti tablosunu okuduğu için, devre dışı bıraktığınız bir eklentiye gereksinen kayıt bunu siz yüklemeden önce söyler. Eklentisini profilin en yeni üç kaydından biri kullanan bir modu devre dışı bırakmadan ya da kaldırmadan önce sorar. |
| Root modları | Bir modun çalıştırılabilir dosyanın yanına koyduğu her şey (F4SE'nin kendisi, ENB, bir `dxvk.conf`) Skyrim'dekiyle aynı `Root/` düzeneğiyle oraya iner |

## Sürüm meselesi

Fallout 4 artık 2019-2024 arasındaki donmuş oyun değil. Ağustos 2026 itibarıyla üç
canlı dal var ve biri için derlenmiş bir mod DLL'i ötekinde yüklenmez:

| Dal | Sürüm | F4SE |
|---|---|---|
| Klasik ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 ile 1.11.240 arası | 0.7.4 ile 0.7.9 arası |

Bir mod listesi kurmadan önce bilmeye değer iki sonuç:

- Oyun kökünde `Creations/` ve `Mods/` klasörleri varsa 1.11.x çizgisindesiniz.
  Eidos'ta bir kaydın ayrıntı bölmesi onu yazan yapıyı da gösterir: Fallout bunu
  kaydın içine yazar, Eidos da "Game build" olarak gösterir.
- F4SE genellikle bir Bethesda güncellemesinden bir iki gün sonra gelir, ama çoğu
  DLL modunun kaymalarını çözdüğü *Address Library for F4SE Plugins* kendi
  takvimiyle yürür. İkisinin arasında DLL modları yüklenmez. DLL'siz modlar
  (dokular, ağlar, eklentiler) etkilenmez; bu yüzden yeni çıkmış bir yama, bir
  mod listesine başlamak için kötü bir gündür.

Düzeneğiniz çalışınca oyun sürümünü örneğe dondurun: Diagnostics sekmesindeki
Game files kartında Freeze this version, ya da boyutu görmek için
`eidos stock-game fallout4 freeze`, kopyalamak için
`eidos stock-game fallout4 freeze --yes`. Eidos mağaza kurulumunu
`<instance>/Stock Game` içine kopyalar ve oyunu o kopyadan çalıştırır; yine
Steam başlatır. Sonraki bir yama mağaza kurulumunu değiştirir, kopyayı değil;
böylece kurduğunuz DLL'ler oyun sürümlerini korur. btrfs ya da XFS üzerinde
kopya mağazanın bloklarını paylaşır ve Steam bir dosyayı değiştirene kadar
neredeyse yer kaplamaz; ext4 üzerinde oyun klasörü kadar büyük tam bir kopyadır.
Steam oyunu meşgul ya da tam kurulmamış bildiriyorsa Eidos reddeder. Ayrıntılar
[usage.md](usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya) içinde.

Kopya yoksa Fallout 4 için Steam'in otomatik güncellemelerini kapatın (Özellikler,
Güncellemeler, "Bu oyunu yalnızca başlattığımda güncelle"); yoksa bir sonraki yama
kurduğunuz her DLL'i kırar.

## Donanım notu: NVIDIA'da silah kalıntısı çökmeleri

Fallout 4'ün silah kalıntısı efekti, NVIDIA'nın Pascal kuşağından sonra desteklemeyi
bıraktığı bir PhysX türevi olan NVIDIA FleX üzerinde çalışır. Turing ve sonrası her
kartta (GTX 16, RTX 20'den RTX 50'ye) oyunu çökertir. Bu oyunun kendi hatasıdır;
Linux, Proton ya da Eidos ile ilgisi yoktur.

İki çözüm, hangisi olursa: oyunun ayarlarından "Weapon Debris" seçeneğini kapatın ya
da *Weapon Debris Crash Fix* (Nexus 48078) kurun; sonuncusu efekti değil parçacıkların
çarpışmasını devre dışı bırakır.

## Fallout 4 için Health kartları

Diagnostics sekmesi şunların her biri için bir Health kartı gösterir:

- F4SE eklentileri `F4SE/Plugins` içine kurulu, ama `f4se_loader.exe` etkin hiçbir
  modda ve oyun klasöründe yok. Oyun onlarsız başlar.
- `f4se_loader.exe` kurulu, ama çalışan oyun sürümü için `f4se_<version>.dll`
  (1.10.984 için `f4se_1_10_984.dll`) yok. Yükleyici bir hatayla durur; bu genellikle
  bir oyun güncellemesinin ardından olur.
- Oyunun çalıştırılabilir dosyası ya da Steam yapısı, Eidos üzerinden hatasız
  çıkan son başlatmadan beri değişti. Eski sürüm için derlenmiş F4SE eklentileri
  yüklenmeyebilir.
- F4SE'den yüksek öncelikli bir mod, F4SE'nin kendi betiklerinin serbest `.pex`
  kopyalarını taşıyor. Eski bir kopya, bildirdiği F4SE işlevlerini bozar ve oyun
  hata göstermez.
- Profilin `Fallout4Custom.ini` dosyasında `[General]` altında `sTestFile1` ile
  `sTestFile10` arası satırlar var. Biri tanımlıyken oyun o eklentileri yükler ve
  `plugins.txt` dosyasını yok sayar. Fix bunları kaldırır ve önceki dosyayı
  `Fallout4Custom.ini.pre-fix` olarak tutar.
- Bir modun BA2 arşivi `.wav` ya da `.xwm` sesini sıkıştırılmış olarak saklıyor.
  Oyun o sesi çalarken çökebilir. Arşivi sesler sıkıştırılmadan yeniden paketleyin.
  Fallout 4 VR aynı kartı alır.
- Bir çökme kaydedicisi son başlatmada bir günlük yazdı: Buffout 4'ün
  `Documents/My Games/Fallout4/F4SE` içindeki `crash-*.log` dosyası ya da
  `F4SE/Crashlogs` içindeki bir `.log`. Kart, günlükteki istisnayı ve modlarınızın
  kurduğu DLL'leri ve eklentileri adlandırır.
- Display kartı, profilin `Fallout4Prefs.ini` dosyasında ayarlı pencere boyutunu ve
  kipini gösterir; Apply oraya `iSize W`, `iSize H`, `bFull Screen` ve
  `bBorderless` yazar.
- Bir Nexus gereksinimi bir DLC'yi adlandırıyor. Eidos DLC'nin adını taşıyan bir
  eklenti arar, Fallout 4'ün DLC eklentilerinin adları ise başkadır (Far Harbor
  için `DLCCoast.esm`); bu yüzden bu kart bir tavsiyedir: elle denetleyin, sonra
  Ignore'a basın.

İki yükleyici kartının <https://f4se.silverlock.org/> için bir Open the download
page düğmesi ve yükleyiciyi modun `Root/` dizinine koyan bir Install the
downloaded archive düğmesi vardır. `eidos check fallout4` aynı bulguları yazdırır;
yalnızca pencerenin hesapladığı `.pex` kartı, Display kartı ve Nexus gereksinim
kartları hariç.

## Bir şey yanlış görünüyorsa

Genel denetim listesi [troubleshooting.md](troubleshooting.md) içinde;
Fallout'a özgü ilk soru ise her zaman *gerçekte hangi çalıştırılabilir dosyanın
başladığı*. Pencere oyunu başlattığında tam başlatma komutunu örneğin çalışma
günlüğüne yazar, yani:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

`f4se_loader.exe` yazıyorsa değiştirme gerçekleşmiştir. `Fallout4.exe` yazıyorsa
F4SE, Eidos'un bulabileceği yere kurulmamıştır; yeri oyunun çalıştırılabilir
dosyasının yanıdır, modla yönetilen bir kurulumda bu bir modun `Root/` dizini
(ya da elle kurulmuşsa oyun klasörünün kendisi) demektir. `Fallout4Launcher.exe`
yazıyorsa Eidos kök görünümde ne yükleyiciyi ne de `Fallout4.exe` dosyasını
bulmuştur.

Doğru çalıştırılabilir dosya başlayıp oyun yine çöküyorsa Test run ve Find the mod
behind a problem, profilin kayıtlarına dokunmadan sorunu tek bir moda kadar
daraltır ([usage.md](usage.md#test-çalıştırmaları)).
