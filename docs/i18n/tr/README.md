<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Oyun klasörünüze asla dokunmayan yerel bir Linux mod yöneticisi.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#kurulum)

Eidos, Linux'taki Bethesda oyunlarına Mod Organizer 2'nin Windows'ta verdiğini
verir: modlarınızın, her başlatma için kurulan birleşik bir görünümünü. Windows
API kancalaması yerine Linux bağlama ad alanlarını ve FUSE'u kullanır ve yönetici
Wine olmadan çalışır. Oyun klasörüne hiçbir şey kopyalanmaz, dolayısıyla sonradan
temizlenecek bir şey yoktur. Eidos'un ayrıca Stellar Blade, Stardew Valley, 7 Days
to Die ve Crimson Desert için tanımları vardır.

Steam kopyaları Eidos'u oyunun başlatma seçeneğinden çalıştırır. Heroic ya da
Legendary içindeki GOG ve Epic kopyaları onu bir wrapper olarak çalıştırır. Oyun
her zaman kopyanın sahibi olan mağaza üzerinden başlar; Eidos onu hiçbir zaman
kendisi Proton ile başlatmaz.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Durum:** Skyrim SE her gün Eidos üzerinden oynanıyor: SKSE, script extender
> ön yükleyicileri, Creation Club, LOOT ile sıralanmış yükleme sıraları ve profil
> başına kayıtlarla. Şimdiye dek gerçek oyunda kanıtlanmış tek bir oyun ailesi
> var. On Bethesda oyunu daha ve dört oyun daha bağlanmış durumda ve test
> edenleri bekliyor.

## Neden Eidos

Birleşik görünüm, yalnızca oyunun süreç ağacının görebildiği özel bir bağlama ad
alanında bağlanır. Başka programlar, ikinci bir oyun dahil, onu hiç görmez ve onun
için izne ihtiyaç duymaz. Oyun çıktığında ya da öldürüldüğünde, ya da makine
elektriğini yitirdiğinde ad alanı süreçleriyle birlikte gider ve kurulum eskisi
gibi kalır.

Her profil kendi mod listesine, eklenti sırasına, INI'lerine ve kayıtlarına
sahiptir. Başlatmada eklenti dosyaları ve kayıt klasörü oyunun kendi yollarının
üzerine bind-mount edilir; böylece oyunun oraya yazdıkları profile iner. Profil
değiştirmek bunların hepsini değiştirir.

Eidos setuid yardımcısı, sistem artalan süreci, `sudo setcap` ve `/etc/fuse.conf`
düzenlemesi gerektirmez. FUSE daemon'u tek bir başlatma için kullanıcınız olarak
çalışır ve oyunla birlikte çıkar. Tek bir program kurar ve tek bir Steam başlatma
seçeneği ayarlarsınız.

Eidos her çalıştırmadan önce eklenti listesinin bir anlık görüntüsünü tutar. Bir
çökme listeyi bozarsa Diagnostics onu o anlık görüntüyle karşılaştırır ve Restore
the pre-session order eski sırayı geri koyar. Yükleme sıranızı silecek bir yakalama
nedeniyle birlikte reddedilir.

Test run oyunu profilin modlarıyla, modsuz ya da yalnızca seçili modlar ve onların
master'larıyla başlatır. O çalıştırmanın kayıtları, INI değişiklikleri, eklenti
durumu, Overwrite çıktısı ve günlükler dahil diğer My Games dosyaları
`<instance>/test-run/` içine gider. Wine kayıt defteri gibi diğer yazmalar her
zamanki yerlerine gider. Oyun kapandığında Eidos bu beş grubu listeler ve her birini
tutar ya da atarsınız. Diagnostics'teki Find the mod behind a problem, her seferinde
şüpheli modların yarısı ve onların gerektirdiği master'larla test çalıştırmaları
yapar. Her çalıştırmadan sonra sorunun olup olmadığını yanıtlarsınız ve Eidos modu
ya da birbirinin master'larına ihtiyaç duyan modları adlandırana kadar şüphelileri
daraltır; 200 şüpheli yaklaşık 8 çalıştırma sürer. Başlamadan önce listede seçili
modlar her adımda açık kalır. Ayrıntılar
[usage.md](docs/guide/usage.md#test-çalıştırmaları) ve
[usage.md](docs/guide/usage.md#bir-sorunun-arkasındaki-modu-bulmak) içinde.

### Nasıl kıyaslanıyor

| | Eidos | Wine üzerinden MO2 | Fluorine-Manager | Limo / bağlantı dağıtıcıları |
|---|---|---|---|---|
| Yönetici doğal çalışır | evet | hayır, Wine içinde bir Windows uygulaması | evet (Qt portu) | evet |
| Oyun klasörüne dokunulmaz | evet, her zaman | evet | evet | hayır, içine bağlantılar yazılır |
| Bağlamayı gören | yalnızca oyun | yalnızca oyun | tüm sistem | yok |
| Çökme sonrası temizlik | gerekmez | gerekmez | eskimiş bağlamayı kurtarma | elle geri alma |
| Root modları (ENB, ön yükleyiciler) | evet, doğal | eklenti gerekir | eklenti gerekir | kısmi |
| Gereken yetkiler | yok | yok | `/etc/fuse.conf` düzenlemesi | yok |

### Ne kadar hızlı

| | önce | şimdi |
|---|---|---|
| bir kaydı yüklemek | ~20 saniye | 6-7 saniye |
| bir oturumdaki klasör taramaları | 5,6 milyon | 335 bin |

Hücre geçişleri anında. Tek bir dosyayı bulmak eskiden elli modun hepsini sırayla
sorgulardı ve tek bir klasörü listelemek bunu elli kez yapardı. Salt okunur
katmanlar artık bağlama sırasında bir kez indekslenir; bu yüzden bir dosyayı bulmak
tek bir hash araması olur ve bir klasörün birleşik listelemesi de bağlama sırasında
bir kez hesaplanır. Rakamlar bir kıyaslamadan değil, normal oynanan gerçek bir
örnekten gelir: [internals/performance.md](../../internals/performance.md).

### Önceki çalışmalar ve teşekkürler

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) ve
  [usvfs](https://github.com/ModOrganizer2/usvfs): Eidos'un yeniden ürettiği
  anlambilim ve denkliğinin karşısında incelendiği kod tabanı
- [LOOT](https://loot.github.io/): libloot üzerinden sıralama motoru
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) ve diğer Linux yöneticileri; bunun
  çözülmesini isteyen bir topluluk olduğunu gösteriyorlar

## Ne yapar

### Modlar

Eidos basit arşivleri, FOMOD sihirbazlarını, Wrye Bash BAIN paketlerini, desteklenen
OBMM betiklerine sahip OMOD kapsayıcılarını ve geri kalan her şeyi elle bir seçici
üzerinden kurar. Root modları (script extender ön yükleyicileri, ENB, Engine Fixes)
Root Builder eklentisi olmadan ve kurulumunuza hiçbir şey kopyalanmadan doğal
çalışır. Tek tek dosyaları gizleyebilir, modları ayraçlarla gruplayabilir, bir
seçimi bir hedefe taşıyabilir, mod başına not ve kategori tutabilir ve bir MO2
profilini içe aktarabilirsiniz.

Liste MO2'nin listesi gibi çalışır: sekiz isteğe bağlı sütun ve herhangi biri
üzerinde sıralama, kategoriye ya da kaynağa göre gruplama, çift tıklama
hareketleri, yazarak atlama, siz geri yükleyene dek bir şey yapmayan mod başına
yedekler ve bu oyunun yükleyemeyeceği bir düzene sahip ya da başka bir oyun için
indirilmiş modlar için bayraklar. Süzgeçler modları Nexus durumuna (endorsed,
tracked, güncelleme var), notlara, gizli dosyalara ve içerik türüne (eklentiler,
arşivler, dokular, mesh'ler, betikler, ses ve daha fazlası) göre seçer. Dosya ağacı
klasör oluşturur, yeniden adlandırır, siler ve dosya açar; hiçbir şey başlatmadan
görselleri ve metinleri önizler.

Ctrl+Z ve Ctrl+Shift+Z, mod listesindeki ve yükleme sırasındaki değişiklikleri geri
alır ve yeniden yapar. Profilin en yeni üç kaydından birinin kullandığı bir modu
devre dışı bırakmak ya da kaldırmak önce sorar ve eklentileri adlandırır (Skyrim ve
Fallout 4 kayıtları). Bir FOMOD modunu yeniden kurmak ya da güncellemek, sihirbazı
son kurulumun cevaplarıyla açar. Bir modun bilgisi, gereksindiği modları ve onu
gereksinen modları, etkin eklentilerin master'larından okuyarak listeler.

### Eklentiler

Yükleme sırasında gömülü LOOT sıralaması, oyunun hesapladığı biçimde mod indeksleri,
eksik master uyarıları ve DLC'leriniz ile Creation Club içeriğiniz yönetilmeyen
satırlar olarak gösterilir. Başlık, her indeks alanında (full, medium, light)
kullanılan yuvaları sayar. Bir eklentinin üzerine gelmek yazarını, açıklamasını ve
başlık sürümünü gösterir. Skyrim motorlarında, oyunun aynı adlı INI dosyasını
yüklediği bir eklenti işaretlenir.

LOOT'un kirli işaretlediği eklentiler, xEdit'in QuickAutoClean'i ile asla yerinde
değil, `Cleaned plugins` adlı bir moda temizlenir. Flag as light, yeni bir moda bir
eklentinin light kopyasını yazar ve en yeni kayıtlardan biri o eklentiyi tam eklenti
olarak yüklüyorsa reddeder (Skyrim ve Fallout 4 kayıtları).

### Örnekler

Bir örnek ya global olup `~/.local/share/Colony/Eidos/instances/` altında tutulur
ya da taşınabilirdir: istediğiniz her yerde kendi kendine yeten bir klasör (ikinci
bir disk, bir oyun bölümü), MO2'ninkiler gibi taşınabilir ve yalıtılmış.
Taşınabilir örnekler oturumlar arasında hatırlanır. GUI, Steam başlatması ve her
CLI komutu en son kullandığınızı izler ve herhangi bir komut, oyun kimliği aldığı
her yerde klasörü de alır. Ayrıntılar
[usage.md](docs/guide/usage.md#örnekler-global-ve-taşınabilir) içinde.

Bir örnek ayrıca oyunu, örneğin içindeki oyun klasörü kopyasından çalıştırabilir.
Eidos bir Wabbajack listesini ilk açtığında listenin kendi kopyasını çalıştırır:
önce `ModOrganizer.ini` dosyasının adlandırdığı klasörü, o yoksa bir `Stock Game`
ya da `Game Root` klasörünü. Diagnostics'teki Game files kartında Freeze this
version, mağaza kurulumunu `<instance>/Stock Game` içine kopyalar; böylece bir
mağaza güncellemesi artık oyunun çalıştırdığı dosyaları değiştirmez. İki klasör de
blok paylaşabilen tek bir dosya sistemindeyse, örneğin btrfs ya da XFS, kopya
blokları mağaza kurulumuyla paylaşır (reflink). ext4 üzerinde ya da iki dosya
sistemi arasında tam bir kopyadır ve boyut ile boş yer önce gösterilir. Oyunu yine
mağaza başlatır. Return to the store version ve Delete the copy bunu geri alır;
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
bir terminalde aynısını yapar. Morrowind ve `bUseMyGamesDirectory=0` ile Oblivion
bir kopyadan çalışamaz. Ayrıntılar
[usage.md](docs/guide/usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya)
içinde.

### Profiller

Her profilin kendi mod sırası, eklenti durumu, INI'leri ve kayıtları vardır.
Kayıtlar ayrıştırılır ve geçerli eklentilerinizle karşılaştırılır, bir kaydın
gerektirdiğini etkinleştiren bir düğmeyle birlikte, ve her oturumdan sonra Steam
Cloud için öneke geri kopyalanır. Steam Cloud'un başka bir cihazdan getirdiği ya da
Eidos olmadan yapılan bir başlatmanın önekte bıraktığı kayıtlar Saves sekmesinde
listelenir. Import onları profile kopyalar ve adını profilin zaten kullandığı bir
kayıt, yerine geçmek yerine yeni bir `orphan-` adıyla gelir.

Bir profil, oyun ve araç çalıştırmaları için başlatma değişkenleri ve wrapper'lar
taşıyabilir (INI düzenleyicisinde satır başına bir `env=NAME=value` ve
`wrapper=command`). Skyrim SE, Enderal SE ve Fallout 4 için Diagnostics'teki bir
Display kartı pencere boyutunu ve kipini profilin INI kopyalarına yazar. Backups
profilin INI'lerinin geri yükleme noktalarını tutar ve Eidos her BethINI Pie
çalıştırmasından önce bir tane alır. Bir profilin masaüstü kısayolu oyunu mağazası
üzerinden başlatır. `eidos sessions` kaydedilmiş çalıştırmaları listeler ve birini
geçerli durumla karşılaştırır.

### Nexus

Mod Manager Download bağlantılarını almak, güncellemeleri denetlemek ve yapımcıları
ile profillerini görmek için bir hesap bağlayın. Her mod, dosyasının geldiği Nexus
alan adında denetlenir, onaylanır ve bağlanır; bu yüzden bir Enderal SE örneğindeki
Skyrim SE modu Skyrim SE sitesinde kalır. Bir güncelleme yazarın Nexus'ta belirlediği
ardıl zincirini izler, modun klasörünü yeni dosyayla değiştirir (eskisinin yedeği
önerilir), önceliğini ve etkin durumunu korur ve kurulumda saklanan FOMOD cevaplarını
yeniden oynatır. `eidos nexus update <instance> --apply` premium bir hesapta aynısını
bir terminalde yapar, ücretsiz bir hesapta dosya sayfalarını yazdırır. Track ve
Endorse mod menüsündedir ve her güncelleme denetimi onları Nexus hesabınızla eşitler.
Bir modun bilgisinde açıklamasını ve değişiklik günlüğünü düz metin olarak gösteren
bir Nexus page sekmesi vardır ve General sekmesi son güncellemenin neyi değiştirdiğiyle
biter: eklenen, silinen ve boyutu değişen dosyalar ile eklenen ya da kaldırılan
master'lar. Install downloads when they finish (Preferences, Nexus, öntanımlı olarak
kapalı), pencere açıkken biten bir indirmeyi biter bitmez kurar.

Koleksiyonlar kendi tariflerinden, kaldığı yerden devam edebilen ilerleme ve
desteklenmeyen adımlara ilişkin açık raporlarla kurulur. Eşleştirme tam kaynak
dosyalarını denetler; eksik eski meta veriler doğrulanmamış kalır. Koleksiyon paneli
bir oyun için yayımlanmış koleksiyonlara göz atar ve onlarda arar, kurulu bir
koleksiyonun daha yeni bir revizyonu olduğunda bunu gösterir ve ücretsiz bir hesabı
eksik dosyalarda sayfa sayfa gezdirir. Bir sürükleme bir koleksiyonun sıralama
kurallarını bozduğunda Diagnostics bunu söyler ve Restore collection order modlarını
geri koyar. Downloads sekmesi arşivleri süzer, sıralar, gizler ve kaldırır. Offline
mode (Preferences, Nexus) Nexus erişimini ve günlük sürüm denetimini durdurur.

### Araçlar

xEdit, BodySlide, DynDOLOD ve diğer modlama araçları, birleşik görünüm üzerinden
oyunun Wine önekinin (Steam'in Proton öneki ya da Heroic'in kurduğu) içinde çalışır.
Modlarınızı görürler, çıktıları Overwrite'a iner ve tek tık bu çıktıyı gerçek bir
moda dönüştürür. Skyrim SE için Eidos; SSEEdit, SSEEdit QuickAutoClean, PGPatcher,
Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis ve BethINI
Pie araçlarını oyun klasöründe, bir modun içinde ya da oyunlarınızın yanında
tuttuğunuz araçlar klasöründe bulur; her birini gereken çalışma zamanlarıyla
birlikte. Diğer oyunların kendi listeleri var. Pandora, Nemesis, FNIS, BodySlide ve
Synthesis çıktılarını adlandırılmış bir moda yazar (`Nemesis Output` vb.).

Sürüm paketi Microsoft'un Visual C++ 2015-2022 ve .NET 8 Desktop Runtime
kurucularını ve .NET 10 çalışma zamanını getirir. Eidos `vcrun2022` ve .NET 8'i bu
kurucularla, winetricks olmadan kurar; paket olmadan aynı dosyaları Microsoft'tan
indirir ve her birini Eidos'a gömülü SHA-256 ile denetler. Synthesis, paketin
getirmediği 300 MB'lık bir indirme olan .NET 10 SDK'sını alır. Diğer ön koşullar
(.NET Framework 4.8, XACT ve daha eski .NET sürümleri) hâlâ winetricks üzerinden
geçer. Bir aracın ön koşullarını kurmayı isteyene kadar hiçbir şey indirilmez.

Kullandığınız araçları sabitleyin, kullanmadıklarınızı gizleyin, kendi Steam
uygulaması olan bir araca kendi Steam AppID'sini verin ve Eidos'u açmadan onu
birleşik görünüm üzerinden başlatan bir `.desktop` kısayolu yazın.
`eidos tool <game> import-mo2`, bir listenin klasöründeki `ModOrganizer.ini`
içindeki çalıştırılabilir dosyaları listeler ve `--write` onları ekler. Data
sekmesindeki bir exe bir kez çalıştırılabilir ya da araç olarak eklenebilir ve
`eidos tool <instance> run --exe <path>` bir terminalden birini çalıştırır. Ask
before playing ile Run, bir üretecin çıktısı eskidiğinde önce sorar.

Stop çalışan bir oyunu ya da aracı sonlandırır. File menüsü oyunun önekinde winecfg,
regedit, Wine dosya gezgini ve Windows kaldırıcısını çalıştırır. Diagnostics'teki
Rebuild prefix öneki yeniden adlandırır, böylece Steam yeni bir tane oluşturur ve
hiçbir şey silmez. `eidos shell <instance>` birleşik görünümde bir kabuk açar ya da
`--` sonrasında verilen komutu çalıştırır ve yazdığı her şey Overwrite'a iner.

Pencere arka plandayken Eidos, bir araç çalıştırması, bir koleksiyon kurulumu, bir
paketleme ya da açma ya da çalışan son indirme bittiğinde ve bir LOOT sıralaması
başarısız olduğunda bir masaüstü bildirimi gönderebilir. Preferences'taki Notify the
desktop when a long job ends bunu açar; öntanımlı olarak kapalıdır.

### Tanılama

Diagnostics sekmesi eksik master'ları, öksüz arşivleri, mod listesi kaymasını ve
hasarlı eklenti kümelerini, bir çalıştırmadan sonra da script extender'ın kendi
günlüğünün neyin yüklendiğine dair söylediklerini listeler. Ayrıca şunları bildirir:

- eksik ya da oyun sürümüyle uyuşmayan bir script extender
- son hatasız başlatmadan beri değişmiş bir oyun
- Eidos'u başlatmayan ya da arkasında `%command%` olmayan bir Steam başlatma seçeneği
- öneki en son hazırlayandan daha eski bir Proton ya da Flatpak Steam'den gelen bir
  Proton
- DLSS için eksik `PROTON_ENABLE_NVAPI` ve Community Shaders'ın kare üretimini
  durduran ayrıcalıklı tam ekran
- `LinuxVersion=true` olmayan bir `enblocal.ini`
- son oturumun çökme günlüğü ve adını verdiği modlar
- eksik ya da devre dışı Nexus gereksinimleri
- başka bir program olan ya da eksik bir dosya olan `nxm://` işleyicisi
- oyunun `plugins.txt` dosyasını yok saymasına yol açan `sTestFile` satırları
- Starfield'daki etkin `.esp` eklentileri ve geçersiz ya da eşleşmeyen blueprint
  eklentileri
- serbest dosyaları yamaladığı modun karşısında kaybeden bir yama
- script extender'ın Papyrus betiklerinin kopyaları extender'ınkileri yenen bir mod
- düşük bir `vm.max_map_count` ya da açık dosya sınırı
- bir Windows dosya sistemindeki oyun dosyaları
- oyunun Data klasörü olmayan BodySlide veri yolları ve `Overwrite/Root` içindeki,
  Data'ya ait klasörler
- araçları başka bir oyun klasörüne yönelten bir listenin `ModOrganizer.ini` dosyası
- eksik olan ya da oyun çalıştırılabilir dosyası olmayan bir oyun kopyası

`eidos check <instance> [--json]` pencere gerektirmeyen denetimleri çalıştırır ve
bir sorun bulduğunda 1 ile çıkar. Copy report, ev klasörünüz `~`, kullanıcı adınız
`<user>` yazılmış bir rapor kopyalar ve Data sekmesindeki Save tree, birleşik Data
ağacını her dosyanın kaynağıyla, aynı şekilde gizlenmiş olarak yazar.

Değiştirme koruması, BSA/BA2 üye çakışmaları, statik SKSE tanılamaları ve üretici
makbuzları ile bunların sınırları için
[Bütünlük denetimleri ve üretilen dosyalar](../../internals/integrity.md) sayfasına
bakın (İngilizce).

### Kendi dosyalarını nerede tutar

Eidos, Colony ailesindeki her programın kullandığı düzeni kullanır: seçtikleriniz
için `~/.config/Colony/Eidos/` (tercihler, Nexus oturumunuz, örnek listeniz,
yazdığınız oyun ve eklenti tanımları), global örnekler, araç çalışma zamanları
(indirilen ya da `install.sh` tarafından paketten kopyalanan) ve oturum günlükleri
için `~/.local/share/Colony/Eidos/`, LOOT ana listeleri için
`~/.cache/Colony/Eidos/`. Bir örneğin içinde `Stock Game` oyunun dondurulmuş
kopyasını, `test-run/` ise inceleme bekleyen bir test çalıştırmasını tutar.

Daha eski bir Eidos dosyalarını `~/.config/eidos/`, `~/.local/share/eidos/` ve
`~/.local/state/` içinde tutuyordu. Yükseltmeden sonraki ilk başlatma onları karşıya
taşır, bunu günlükte söyler ve eski dizinleri oldukları gibi bırakır. Bir taşıma
tamamlanana kadar Eidos eski dosyaları kullanmaya devam eder. Ayrıntılar
[internals/paths.md](../../internals/paths.md) içinde (İngilizce).

### Desteklenen oyunlar

Skyrim SE/AE gerçek oyunda kanıtlanmıştır. Fallout 4 de baştan sona bağlanmıştır
(F4SE otomatik devreye alınır, arşiv geçersizleştirme, yıldızlı yükleme sırası,
LOOT, `.fos` kayıtları); bkz. [guide/fallout4.md](docs/guide/fallout4.md). Bağlanmış
ve test edenleri arayanlar: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout
3, Fallout NV, Starfield, Oblivion ve Morrowind. Fallout 3, Fallout NV, Oblivion ve
Morrowind eklentileri dosya tarihlerine göre sıralar: Eidos profil başına tek bir
sıra tutar ve bu tarihleri diskteki dosyaları değiştirmeden bağlama içinde gösterir.

Dört oyun daha bağlanmıştır ve onlar da test eden arıyor: Stellar Blade (`~mods`
içinde Unreal `.pak` modları), Stardew Valley (SMAPI modları, SMAPI ayrıca
kurulur), 7 Days to Die (oyunun yanındaki `Mods/` klasörleri) ve Crimson Desert.
Crimson Desert yalnızca kod modlarını alır: `bin64` içindeki ASI eklentileri,
ReShade ve yükleyici DLL'leri. PAZ gruplarında yaşayan veri modları desteklenmez.

Bir oyun tek bir TOML dosyasıdır ve sizinki yeniden derlemeden
`~/.config/Colony/Eidos/games/` içine konabilir:
[internals/adding-games.md](../../internals/adding-games.md).

## Kurulum

### Colony üzerinden (önerilen)

[Colony](https://github.com/Project-Colony/Colony) içinde Eidos'u arayın ve kurun.
Güncellemeler başlatıcı üzerinden gelir. Colony tek bir dosya kurar, `eidos-linux`:
pencere, ayrıca `eidos` adlı bir bağlantı üzerinden çağrıldığında `eidos` komut
satırı. Bu dosya ne NIF model önizleme yardımcısını ne de aşağıdaki paketin
Microsoft çalışma zamanlarını taşır. Yardımcı olmadan önizleme bunu söyler ve geri
kalan her şey çalışır; bir aracın ön koşullarını kurduğunuzda ise Eidos bir çalışma
zamanını Microsoft'tan indirir. Kurduktan sonra yapılacaklar ve Steam'i ona nasıl
yönelteceğiniz:
[docs/guide/install.md](docs/guide/install.md#colony-üzerinden).

### Doğrudan ikili indirme

[Son sürümden](../../../../../releases/latest) bir dosya alın:

| Dosya | Nedir |
|---|---|
| `eidos-linux` | Colony'nin kurduğu biçimde, tek dosyada programın tamamı |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, NIF önizleme yardımcısı, Visual C++ 2015-2022 ve .NET 8 Desktop Runtime kurucuları ile .NET 10 çalışma zamanı, simgeler, lisans bildirimleri ve masaüstü girdisini ve Nexus indirme işleyicisini kaydeden bir kurucu |

Önce indirdiğinizi [Kod imzalama politikası](#kod-imzalama-politikası) bölümünün
gösterdiği gibi denetleyin, ardından:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Kurucu ikili dosyaları sudo olmadan `~/.local/bin` içine koyar ve paketin
`runtimes/` klasörünü `~/.local/share/Colony/Eidos/runtimes` içine kopyalar.
`--system` ve `--bindir DIR` ikili dosyaları başka bir yere koyar, `--from DIR`
onları başka bir klasörden alır. `--cap`, yalnızca deneysel FUSE passthrough'un
ihtiyaç duyduğu `CAP_SYS_ADMIN` için sudo'dan yetki ister ve passthrough kapalı
kalır; `--no-cap` öntanımlıdır. İki dosya da glibc 2.39 ya da daha yenisine sahip
x86_64 Linux, FUSE (`fusermount3`) ve mod arşivleri için 7-Zip gerektirir.

Eidos kendini güncellemez. Pencere başlangıçta günde bir kez GitHub'a son sürümü
sorar ve daha yenisi çıktıysa menü çubuğunda Help'ten sonra sürüm notlarına bir
bağlantı gösterir. Preferences bunu kapatır ve `/usr/local` dışında `/usr` altındaki
bir kopya, ki onu paket yöneticisi günceller, hiç sormaz.

### Kaynaktan derleme

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.92 ya da daha yenisi (CI'nin denetlediği `rust-version`; CI ve sürümler
1.99.1 ile derlenir), ayrıca NIF önizleme yardımcısı için `just`, CMake 3.20+ ve
bir C++17 derleyicisi gerekir. `just build` release profilini kullanır: tek bir
codegen unit ve fat LTO. `eidos` ile `eidos-gui` için 3 derleme işiyle soğuk bir
derleme böyle 334 saniye, thin LTO ile 243 saniye sürdü ve fat LTO `eidos`'u 1,2
MB, `eidos-gui`'yi 3,8 MB küçültür (ölçümler `Cargo.toml` içinde).
`just profile=dev build` daha hızlı derler. Bir Arch paketi `packaging/PKGBUILD`
üzerinden derlenir. Ön gereksinimler ve `just` olmadan kullanılacak komutlar:
[docs/guide/install.md](docs/guide/install.md).

### Steam başlatma seçenekleri

Eidos oyunun başlatma komutu olarak çalışır; oyun başlamadan önce bağlamayı böyle
başarır. Temel satır, çoğu kurulumun ihtiyaç duyduğu her şeydir ve kurulumunuzun
mutlak yolunu taşır (Steam `PATH`'inizi okumaz):

```
~/.local/bin/eidos-gui %command%
```

Colony'den bu yol `~/.local/share/Colony/apps/Eidos/eidos-linux` olur. Ardından
Play'e basın: Eidos o oyunun örneğinde açılır ve Run oyunu birleşik görünüm
üzerinden başlatır.

Kendiniz açtığınız bir pencerede Run, Steam'den oyunu başlatmasını ister
(`steam -applaunch <appid>`); böylece başlatma seçeneği yine geçerlidir ve Steam'in
açtığı pencerede Run'a basarsınız. Eidos oyunun wrapper'ı olarak ayarlandığında
Heroic bir `heroic://launch` bağlantısı alır ve Legendary `legendary launch <app>`
çalıştırır. Mağazadaki ayar eksikse Run hiçbir şey başlatmaz ve neyin ayarlanacağını
söyler: [guide/usage.md](docs/guide/usage.md#oyunu-başlatmak). Start the game at once
when a store opens Eidos (Preferences, öntanımlı olarak kapalı) açıkken, hiçbir şey
ilgi gerektirmiyorsa Eidos Diagnostics denetimlerini çalıştırır ve oyunu penceresini
göstermeden başlatır.

Geri kalan her şey onun önüne dizilen ortam değişkenleridir ve serbestçe birleşirler:

| Şunu isterseniz... | Önüne koyun |
|---|---|
| Community Shaders ile DLSS | `PROTON_ENABLE_NVAPI=1`. Onsuz DLSS hiç başlatılmaz ve bir şey söylemez; tam denetim listesi [guide/graphics.md](docs/guide/graphics.md) içinde |
| ekranda bir FPS sayacı | `DXVK_HUD=fps` |
| sürücü düzeyinde kare aradeğerleme, sıfır mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, asla Community Shaders'ın kendi kare üretimiyle birlikte değil |
| bir hata bildirimi için ayrıntılı günlükler | `EIDOS_LOG=debug` (oturum günlükleri `~/.local/share/Colony/Eidos/logs/` içine iner) |
| bağlamadan oturum başına bir G/Ç raporu | `EIDOS_FUSE_STATS=1` |
| farklı bir FUSE işçi sayısı | `EIDOS_FUSE_THREADS=8` (öntanımlı 4; bir eşzamanlılık hatası kovalarken ilk denenecek şey `1`) |
| bu başlatmanın tek bir taşınabilir örneğe sabitlenmesi | `EIDOS_INSTANCE=/path/to/folder`. Onsuz Eidos en son kullandığınız örneği açar |
| Start the game at once açıkken tek bir başlatma için pencere | `EIDOS_WINDOW=1` |

Modern bir modlu kurulum (Community Shaders, DLSS, kare üretimi) için saklanacak
satır, bir örnek değil, son komuttur:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Kurulumun çalıştığını doğrularken önüne `DXVK_HUD=fps` ekleyin, çalıştığında
kaldırın.

Aynı değişkenler Steam satırı yerine profilin başlatma değişkenlerine de yazılabilir;
bu onları profil başına tutar ve araç çalıştırmalarına da uygular:
[guide/usage.md](docs/guide/usage.md#profil-başına-değişkenler-ve-wrapperlar).

Daha derin tanılama anahtarları (`EIDOS_FUSE_TRACE`, önbellek ve indeks ikiye
bölme düğmeleri, `EIDOS_FUSE_PASSTHROUGH`'un neden öntanımlı olarak kapalı olduğu)
[guide/troubleshooting.md](docs/guide/troubleshooting.md) içinde.

## Belgeler

Her şey [docs/](docs/) içindedir ve kimin okuduğuna göre
[docs/README.md](docs/README.md) sayfasında dizinlenmiştir: kullanıcı kılavuzu
(kurulum, CLI ve GUI, xEdit ve DynDOLOD gibi araçlar, grafik, sorun giderme),
Eidos'un içeride nasıl çalıştığı ve neden var olduğu. Güvenlik politikası ve bir
güvenlik açığının nasıl bildirileceği: [SECURITY.md](../../../SECURITY.md).

## Dil

Bir oyuncunun ihtiyaç duyduğu sayfalar çevrilmiştir. İngilizce asıldır: bir çeviri
onunla uyuşmadığında doğru olan İngilizce dosyadır.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Geri kalan her şey İngilizcedir. `docs/internals/` ve `docs/project/` aynı zamanda
Rust'ı da okuyan kişilerce okunur, `CHANGELOG.md` üretilir ve `SECURITY.md` de
çevrilmez. İlk ikisini çevirmek, buna ihtiyacı olmayan okurlar için eşgüdümlü
tutulacak 19.000'den fazla kelime daha demek olurdu.

Her çeviri, yapıldığı İngilizce dosyanın hash'ini taşır ve bu hash İngilizceyle
artık eşleşmediği sürece CI başarısız olur; bkz.
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Çeviri bu durumda
güncellenir ve `scripts/i18n-check.sh --fix` ile yeniden damgalanır. Güncelliğini
yitirmiş bir çeviri kaldırılmaz, güncellenir.

Bir dil tek bir dizindir. `docs/i18n/<lang>/` deponun kökünü yansıtır, yani
`docs/i18n/de/docs/guide/install.md` Almanca `docs/guide/install.md` dosyasıdır.
İki çevrilmiş sayfa arasındaki bağlantının, İngilizce asılları arasındaki
bağlantıyla aynı dize olmasının ve bir dili kaldırmanın tek bir `rm -r` olmasının
nedeni budur.

## Kod imzalama politikası

Her sürüm dosyası, hiçbir zaman bir geliştirici makinesinde değil, CI içinde
Project Colony kuruluşu tarafından imzalanır. Sürüm sayfasında her dosyanın
yanında:

| Dosya | Nedir |
|---|---|
| `<asset>.sig` | kuruluşun sürüm anahtarıyla yapılmış, dosya üzerinde bir ed25519 imzası |
| `<asset>.meta` | dosyayı adına, sha256 değerine ve sürüm numarasına bağlayan üç satır |
| `<asset>.meta.sig` | `.meta` üzerinde bir ed25519 imzası |

Özel anahtar bir kuruluş sırrıdır ve yalnızca hiçbir şey derlemeyen bir işte,
paylaşılan
[sign-and-publish iş akışı](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
tarafından kullanılır; Eidos'u derleyen işler onu asla görmez. Colony, Eidos'u
kurmadan ya da güncellemeden önce üç dosyanın hepsini denetler ve kurulu
olandan daha eski bir sürümü reddeder. İndirdiğiniz bir dosyayı OpenSSL 3 ile
kendiniz denetlemek için (aynı komutlar her dosya için çalışır):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Sürümlerin nasıl derlendiği, imzalandığı ve yayımlandığı:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).


## Gizlilik

Eidos telemetri, analiz verisi ya da çökme raporu göndermez. Yalnızca bu tablodaki
sunuculara bağlanır ve onlara da yalnızca orada belirtilen eylem için. Offline kipi
(Preferences, Nexus) Nexus'a giden her bağlantıyı ve günlük sürüm denetimini durdurur.

| Veri | Saklanır ya da gönderilir | Nereye ve neden |
|---|---|---|
| Tercihler, örnek listesi, oyun ve eklenti tanımları | saklanır | `~/.config/Colony/Eidos/` |
| Örnekler, araç çalışma zamanları, oturum günlükleri | saklanır | `~/.local/share/Colony/Eidos/` ya da taşınabilir bir örnek için seçtiğiniz klasör |
| LOOT ana listeleri | saklanır | `~/.cache/Colony/Eidos/loot/` |
| Sürüm denetimi | gönderilir | Başlangıçta, günde en fazla bir kez, `api.github.com/repos/Project-Colony/Eidos/releases/latest` adresine bir istek. Eidos yalnızca sürüm etiketini okur ve hiçbir şey indirmez. Öntanımlı olarak açıktır ve Preferences'ta kapatılır; Offline kipinde ve `/usr/local` dışında `/usr` altına kurulmuş, paket yöneticisinin güncellediği bir kopyada atlanır. [GitHub gizlilik bildirimi](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus oturum açma | gönderilir | Oturum açmak tarayıcınızda `users.nexusmods.com` adresini açar; site sonucu, Eidos'un yalnızca o oturum açma için `127.0.0.1` üzerinde açtığı bir dinleyiciye geri verir. Eidos ardından bunu `users.nexusmods.com/oauth/token` adresinde jetonlarla takas eder ve Nexus'un onları imzaladığı açık anahtarları getirir. [Nexus Mods gizlilik politikası](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus kimlik bilgileri | saklanır | Erişim ve yenileme jetonları, 0600 kipinde `~/.config/Colony/Eidos/nexus.ini` içinde. Daha eski bir Eidos'un oraya yazdığı bir `api_key=` satırı korunur ama asla okunmaz; dilerseniz silin. |
| Nexus istekleri | gönderilir | Bir Nexus özelliğini kullandığınızda `api.nexusmods.com`: hesap adınız ve premium durumunuz, mod ve dosya ayrıntıları, güncelleme denetimleri, koleksiyonlar ve indirme bağlantıları. Onaylarınız ve takip ettiğiniz modlar her güncelleme denetiminde okunur ve Endorse ya da Track'e bastığınızda yazılır. Mod gereksinimleri, açıklamalar ve değişiklik günlükleri ile koleksiyon araması `api.nexusmods.com/v2/graphql` adresine gider. Her istek erişim jetonunuzu taşır ve Eidos'u ve sürümünü belirtir. |
| Nexus indirmeleri | gönderilir | İstediğiniz dosyalar, indirme bağlantısının belirttiği Nexus CDN sunucusundan; örneğin indirmeler klasörüne, her birinin yanında MO2 biçiminde bir `.meta` ile kaydedilir. |
| `nxm://` bağlantıları | saklanır | `install.sh`'nin sizin için çalıştırdığı `eidos nxm --register`, `~/.local/share/applications/eidos-nxm.desktop` dosyasını yazar ve onu `xdg-mime` üzerinden `nxm://` işleyicisi yapar; böylece sitenin Mod Manager Download düğmesi bağlantısını Eidos'a verir. Arch paketi aynı girdiyi sistem genelinde kurar. |
| LOOT ana listeleri | gönderilir | Bir oyunun ilk LOOT sıralamasında ve her güncelleme istediğinizde `raw.githubusercontent.com/loot/...`. Size ait hiçbir şey gönderilmez. [GitHub gizlilik bildirimi](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Araç çalışma zamanları | gönderilir | Yalnızca bir aracın ön koşullarını kurduğunuzda ve yalnızca sürüm paketinin getirmediği şeyler için (tek dosya `eidos-linux` ve kaynaktan derleme hiçbir şey getirmez): `builds.dotnet.microsoft.com` adresinden .NET 10 çalışma zamanı ve `download.visualstudio.microsoft.com` adresinden Visual C++ ve .NET 8 kurucuları, her biri Eidos'a gömülü SHA-256 ile eşleşmedikçe reddedilir; her zaman `builds.dotnet.microsoft.com` adresinden .NET 10 SDK, aynı şekilde denetlenerek; ve diğer ön koşullar (.NET Framework 4.8, XACT, .NET 6 ve 7) için, kurduğunu Microsoft'tan ve winetricks'in kullandığı diğer sunuculardan indiren `winetricks`. [Microsoft gizlilik bildirimi](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Oturum günlükleri | saklanır | `~/.local/share/Colony/Eidos/logs/`, 0600 kipinde: örnek başına on oturum, 30 gün sonra temizlenir. Ev klasörünüz `~`, kullanıcı adınız `<user>` olarak yazılır, böylece bir günlük bir hata kaydına olduğu gibi yapıştırılabilir. Siz göndermedikçe makinenizden asla çıkmazlar. |
| Çalıştırma kayıtları | saklanır | `<instance>/logs/sessions/`: her oyun ya da araç çalıştırması için profil, sürümleri ve Nexus dosya kimlikleriyle etkin modlar, eklenti sırası, her profil INI dosyasının sağlama toplamı, oyun ve Eidos sürümleri, çıkış kodu ve çalıştırmanın oluşturduğu kayıtlar. En yeni 50 tanesi tutulur, `eidos sessions` onları okur ve `eidos pack` onları dışarıda bırakır. Makinenizde kalırlar. |
| Güncelleme raporları | saklanır | `<instance>/logs/updates/<mod>.json`: son güncellemenin ya da Replace'in o modun klasöründe neyi değiştirdiği. `eidos pack` onları dışarıda bırakır. |
| Kısayollar | saklanır | `install.sh`, `eidos.desktop` dosyasını ve simgelerini `~/.local/share` altına yazar; oluşturduğunuz bir araç ya da profil kısayolu `~/.local/share/applications` içine gider. |
| Masaüstü bildirimleri | hiçbiri | Öntanımlı olarak kapalıdır. Açıkken, D-Bus üzerinden (`org.freedesktop.Notifications`) masaüstünüzün bildirim hizmetine gider ve biten işi adlandırır. |
| Açtığınız bağlantılar | hiçbiri | Mod sayfaları ve diğer bağlantılar `xdg-open` üzerinden kendi tarayıcınızda açılır. |
| Eklentiler | hiçbiri | Kurduğunuz eklentiler makinenizde kendi programları olarak çalışır; ne yaptıkları kendilerine aittir. |

## Lisans

Eidos'un özgün kodu GPL-3.0-or-later lisanslıdır; bkz. [LICENSE](../../../LICENSE).
Şu bölümler "or later" seçeneği olmadan GPL-3.0 lisanslıdır:

- `eidos-install` içindeki OMOD betik çevirileri; bkz.
  [özgün geliştiricilere verilen atıflar](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- gömülü nifly kitaplığı üzerine kurulu `eidos-nif-preview` yardımcısı; bkz.
  [bileşen lisansları ve geliştirici atıfları](../../../crates/eidos-nif-preview/UPSTREAM.md);
- yükleme sırası kodunun bağladığı ve `GPL-3.0` beyan eden `esplugin` ve
  `libloadorder` crate'leri.

İkili dosyalardaki diğer Rust crate'leri kendi lisanslarını korur (MIT,
Apache-2.0, BSD ve benzerleri). Sürüm paketi, her crate'in ve lisansının bir
dizinini içeren `licenses/crates/` klasöründe onların bildirimlerini taşır.

İkili dosyalar ayrıca sekiz özgün Microsoft DirectX yeniden dağıtılabilir DLL'i
gömer (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` ve `d3dx11_43`, 32 ve 64
bit), gzip ile sıkıştırılmış olarak saklanır (17,9 MB yerine 7,9 MB); Eidos bunları
bir mod ya da araç ihtiyaç duyduğunda açar ve oyunun önekine yerleştirir.
Microsoft'un yeniden dağıtım koşullarına tabi kalırlar; bkz.
[kaynak bilgileri](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Sürüm paketi ayrıca `runtimes/` altında, Microsoft'un yayımladığı biçimde,
Microsoft'un Visual C++ 2015-2022 kurucularını, .NET 8 Desktop Runtime kurucusunu
ve .NET 10 çalışma zamanı zip dosyasını taşır. Koşulları
`licenses/microsoft-runtimes/` içindedir (`vc_redist.txt`, `dotnet.txt`).
