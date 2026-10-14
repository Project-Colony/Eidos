<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Oyununuza asla dokunmayan yerel Linux mod yöneticisi.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#kurulum)

Eidos, Linux'taki Bethesda oyunlarına Mod Organizer 2'nin Windows'ta verdiğini
verir - modlarınızın sanal, her başlatmada kurulan birleşik görünümünü - Windows
API kancalamasıyla değil, Linux ilkelleriyle kurulmuş olarak. Yönetici için Wine
yok. Oyun klasörüne kopyalanan dosya yok. Temizlik yolu da yok, çünkü
temizlenecek bir şey yok.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Durum:** Skyrim SE her gün Eidos üzerinden oynanıyor - SKSE, script extender
> ön yükleyicileri, Creation Club, LOOT ile sıralanmış yükleme sıraları, profil
> başına kayıtlar, hepsi. Şimdiye dek gerçek oyunda kanıtlanmış tek bir oyun
> ailesi; on tanesi daha bağlanmış, test edenleri bekliyor.

## Neden Eidos

- 🔒 **Yalnızca oyununuzun görebildiği bir bağlama.** Birleşik görünüm özel bir
  bağlama ad alanında yaşar: dosya yöneticiniz, yedekleme işiniz, ikinci bir
  oyun - hiçbiri onu görmez, hiçbirinin onun için izne ihtiyacı yoktur. Oyunu
  öldürün, fişi çekin: ad alanı süreç ağacıyla birlikte ölür ve kurulumunuz tam
  olarak eskisi gibidir. *Yapısı gereği* hiçbir kalıntı yoktur.
- 🧾 **Gerçeğin tek bir kopyası.** Profiliniz kendi mod listesine, eklenti
  sırasına, INI'lerine ve kayıtlarına sahiptir. Eklenti dosyaları ve kayıt
  klasörü başlatmada oyunun kendi yollarının üzerine bind-mount edilir, böylece
  oyunun kendi yazdıkları bile profilinize iner. Profil değiştirmek her şeyi
  değiştirir.
- 🐧 **Tamamen root'suz.** setuid yardımcısı yok, artalan süreci yok, `sudo
  setcap` yok, `/etc/fuse.conf` düzenlemesi yok. Tek ikili, tek Steam başlatma
  seçeneği.
- 🛡️ **Kanıtını gösteren korumalar.** Eklenti listenizi bozan bir çökme, oturum
  öncesi alınmış bir anlık görüntüye karşı işaretlenir ve tek tıkla geri yükleme
  sunulur. Yükleme sıranızı silecek bir yakalama reddedilir ve nedenini söyler.

### Nasıl kıyaslanıyor

| | Eidos | Wine üzerinden MO2 | Fluorine-Manager | Limo / bağlantı dağıtıcıları |
|---|---|---|---|---|
| Yönetici doğal çalışır | ✅ | ❌ Wine içinde Windows uygulaması | ✅ (Qt portu) | ✅ |
| Oyun klasörüne dokunulmaz | ✅ her zaman | ✅ | ✅ | ❌ içine bağlar yazılır |
| Bağlamayı gören | yalnızca oyun | yalnızca oyun | **tüm sistem** | yok |
| Çökme sonrası temizlik | tasarım gereği yok | yok | eskimiş bağlamayı kurtarma | elle geri alma |
| Root modları (ENB, ön yükleyiciler) | ✅ doğal | eklenti gerekir | eklenti gerekir | kısmi |
| Gereken yetkiler | yok | yok | `/etc/fuse.conf` düzenlemesi | yok |

### Ne kadar hızlı

| | önce | şimdi |
|---|---|---|
| bir kaydı yüklemek | ~20 saniye | **6-7 saniye** |
| bir oturumdaki klasör okumaları | 5,6 milyon | 465 bin |

Hücre geçişleri anında. Kazanç, modlarınıza daha az soru sormaktan geldi: tek
bir dosyayı bulmak eskiden ellisini de sırayla sorgulardı, tek bir klasörü
listelemek de bunu elli kez yapardı. Artık ikisi de yapmıyor. Bir kıyaslama
üzerinde değil, normal oynanan gerçek bir örnek üzerinde ölçüldü.

### Önceki çalışmalar ve teşekkürler

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) ve
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos'un yeniden ürettiği
  anlambilim ve denkliğinin karşısında incelendiği kod tabanı
- [LOOT](https://loot.github.io/) - libloot üzerinden sıralama motoru
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) ve diğer Linux yöneticileri - bunun
  çözülmesini isteyen bir topluluk olduğunun kanıtı

## Ne yapar

**Modlar.** Basit arşivler, FOMOD sihirbazları, Wrye Bash BAIN paketleri, gerisi
için elle seçim - bir de **root modları doğal olarak** (script extender ön
yükleyicileri, ENB, Engine Fixes), Root Builder eklentisi olmadan ve
kurulumunuza hiçbir şey kopyalanmadan. Tek tek dosyaları gizleyin, ayraçlarla
gruplayın, hedefli taşımalar, mod başına notlar ve kategoriler, bir de MO2
profil içe aktarıcısı.

Liste MO2'nin listesidir, alışkanlıklarıyla birlikte: sekiz isteğe bağlı sütun
ve herhangi biri üzerinde sıralama, kategoriye ya da kaynağa göre gruplama, çift
tıklama hareketleri, yazarak atlama, siz geri yükleyene dek etkisiz duran mod
başına yedekler, bir de bu oyunun yükleyemeyeceği bir düzene sahip ya da başka
bir oyun için indirilmiş modlar için uyarı bayrakları. Dosya ağacı sıradan
işlemleri yapar - yeni klasör, yeniden adlandır, sil, aç - ve hiçbir şey
başlatmadan görselleri ve metinleri önizler.

**Eklentiler.** LOOT sıralaması gömülü yükleme sırası, oyunun hesapladığı
biçimde mod indeksleri, eksik master uyarıları ve DLC'lerinizle Creation Club
içeriğinizin oldukları gibi, yönetilmeyen satırlar olarak gösterilmesi.

**Örnekler.** Genel - `~/.local/share/Colony/Eidos/instances/` altında merkezî olarak yönetilir -
ya da taşınabilir: istediğiniz her yerde kendi kendine yeten bir klasör (ikinci
bir disk, bir oyun bölümü), taşınabilir ve yalıtılmış, MO2'ninkiler gibi.
Taşınabilir örnekler oturumlar arasında hatırlanır; GUI, Steam başlatması ve her
CLI komutu en son kullandığınızı izler ve her komut, oyun kimliği aldığı her
yerde klasörü de alır. Ayrıntılar
[usage.md](docs/guide/usage.md#örnekler-global-ve-taşınabilir) içinde.

**Profiller.** Profil başına mod sırası, eklenti durumu, INI'ler ve kayıtlar.
Kayıtlar ayrıştırılır, geçerli eklentilerinizle karşılaştırılır - bir kaydın
gerektirdiğini etkinleştiren bir düğmeyle birlikte - ve her oturumdan sonra
Steam Cloud için geri eşitlenir.

**Nexus.** Mod Manager Download bağlantılarını almak, güncellemeleri denetlemek ve yapımcılarla profillerini görmek için bir hesap bağlayın. Koleksiyonlar kendi tariflerinden, kaldığı yerden devam edebilen ilerleme ve desteklenmeyen adımlara ilişkin açık raporlarla kurulur. Eşleştirme tam kaynak dosyalarını denetler; eksik eski meta veriler doğrulanmamış kalır. İndirmeler sekmesi arşivleri filtreler, sıralar, gizler ve kaldırır. **Çevrimdışı** anahtarı Nexus erişimini durdurur.

**Araçlar.** xEdit, BodySlide, DynDOLOD ve arkadaşları oyunun Proton önekinin
içinde *birleşik görünüm üzerinden* çalışır - modlarınızı görürler, çıktıları
Overwrite'a iner ve tek tık onu gerçek bir moda dönüştürür. Her birinin
gerektirdiği çalışma zamanı istendiğinde getirilir, böylece eksik bir DLL bir
öğleden sonra değil, bir düğmedir. xEdit ve onun QuickAutoClean ikizi sizin için
bulunur - oyun klasöründe, bir modun içinde ya da oyunlarınızın yanında
tuttuğunuz araç klasöründe - doğru çalışma zamanları çoktan seçilmiş olarak.
Kullandıklarınızı sabitleyin, kullanmadıklarınızı gizleyin, kendi Steam
uygulaması olan bir araca kendi
Steam AppID'sini verin ve Eidos'u hiç açmadan onu birleşik görünüm üzerinden
başlatan bir `.desktop` kısayolu yazın.

**Tanılama.** Eksik master'lar, öksüz arşivler, mod listesi kayması, hasarlı
eklenti kümeleri - bir de bir çalıştırmadan sonra, script extender'ın kendi
günlüğünün gerçekte neyin yüklendiğine dair söyledikleri.

**Kendi dosyalarını nerede tutar.** Colony ailesindeki her programın kullandığı
düzen: seçtikleriniz için `~/.config/Colony/Eidos/` - tercihler, Nexus
oturumunuz, örnek listeniz, yazdığınız oyun ve eklenti tanımları - global
örnekler, indirilen çalışma zamanları ve oturum günlükleri için
`~/.local/share/Colony/Eidos/`, LOOT ana listeleri için
`~/.cache/Colony/Eidos/`. Daha eski bir Eidos bunları `~/.config/eidos/`,
`~/.local/share/eidos/` ve `~/.local/state/` içinde tutuyordu; yükseltmeden
sonraki ilk başlatma onları taşır, bunu günlükte söyler ve eski klasörleri tam
olarak eskisi gibi bırakır. Bir taşıma tamamlanana kadar Eidos eski dosyaları
kullanmaya devam eder. Ayrıntılar için
[internals/paths.md](../../internals/paths.md) sayfasına bakın (İngilizce).

Değiştirme koruması, BSA/BA2 içindeki dosya çakışmaları, statik SKSE tanılamaları ve üretim kayıtları ile bunların sınırları için [Bütünlük denetimleri ve üretilen dosyalar](../../../docs/internals/integrity.md) sayfasına bakın (İngilizce).

### Desteklenen oyunlar

**Skyrim SE/AE** - gerçek oyunda kanıtlanmış. **Fallout 4** de baştan sona
bağlanmış durumda (F4SE otomatik olarak devreye alınır, arşiv geçersizleme,
yıldızlı yükleme sırası, LOOT, `.fos` kayıtları) - bkz.
[guide/fallout4.md](docs/guide/fallout4.md). Ortak oyun tanımlayıcısına göre
bağlanmış ve test edenleri arayanlar: Skyrim LE, Skyrim VR, Enderal SE, Fallout
3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion ve Morrowind (son ikisi
modları bağlar ve yönetir; zaman damgasına göre sıralanan eklenti listeleri
henüz yönetilmiyor).

Bir aile eklemek tek bir tanımlayıcı satırıdır:
[internals/adding-games.md](../../internals/adding-games.md).

## Kurulum

### Colony üzerinden (önerilen)

[Colony](https://github.com/Project-Colony/Colony) içinde **Eidos**'u arayın ve
kurun. Güncellemeler başlatıcı üzerinden gelir. Colony tek bir dosya kurar,
`eidos-linux`: pencere, ayrıca `eidos` adlı bir bağlantı üzerinden
çağrıldığında `eidos` komut satırı. NIF model önizlemesi, yalnızca aşağıdaki
paketin içerdiği bir yardımcı gerektirir; o olmadan önizleme bunu söyler ve geri
kalan her şey çalışır. Kurduktan sonra yapılacaklar ve Steam'i ona nasıl
yönelteceğiniz:
[docs/guide/install.md](docs/guide/install.md#colony-üzerinden).

### Doğrudan ikili indirme

[Son sürümden](https://github.com/Project-Colony/Eidos/releases/latest) bir
dosya alın:

| Dosya | Nedir |
|---|---|
| `eidos-linux` | Colony'nin kurduğu biçimde, tek dosyada programın tamamı |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, NIF önizleme yardımcısı, simgeler ve masaüstü girdisini ve Nexus indirme işleyicisini kaydeden bir kurucu |

Önce indirdiğinizi [Kod imzalama politikası](#kod-imzalama-politikası)
bölümünün gösterdiği gibi denetleyin, ardından:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Kurucu her şeyi sudo olmadan `~/.local/bin` içine koyar; `--system` ve
`--bindir DIR` başka bir yer seçer. İki dosya da glibc 2.39 ya da daha yenisine
sahip x86_64 Linux, FUSE (`fusermount3`) ve mod arşivleri için 7-Zip gerektirir.

### Kaynaktan derleme

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.89 ya da daha yenisi (CI'nin denetlediği `rust-version`; CI ve sürümler
1.99.0 ile derlenir), ayrıca NIF önizleme yardımcısı için `just`, CMake 3.20+ ve
bir C++17 derleyicisi gerekir. Bir Arch paketi `packaging/PKGBUILD` üzerinden
derlenir. Ön gereksinimler ve `just` olmadan kullanılacak komutlar:
[docs/guide/install.md](docs/guide/install.md).

### Steam başlatma seçenekleri

Eidos oyunun başlatma komutu *olarak* çalışır; oyun başlamadan önce bağlamayı
böyle başarır. Temel satır, kurulumunuzun mutlak yoluyla birlikte, çoğu
kurulumun ihtiyaç duyduğu her şeydir (Steam `PATH`'inizi okumaz):

```
~/.local/bin/eidos-gui %command%
```

Colony'den kurduysanız bu yol `~/.local/share/Colony/apps/Eidos/eidos-linux`
olur. Ardından Oyna'ya basın: Eidos o oyunun örneğinde açılır ve Run oyunu
birleşik görünüm üzerinden başlatır.

Geri kalan her şey onun önüne dizilen ortam değişkenleridir ve serbestçe
birleşirler:

| Şunu isterseniz... | Önüne koyun |
|---|---|
| Community Shaders ile DLSS | `PROTON_ENABLE_NVAPI=1` - onsuz DLSS sessizce hiç başlatılmaz; tam denetim listesi [guide/graphics.md](docs/guide/graphics.md) |
| ekranda bir FPS sayacı | `DXVK_HUD=fps` |
| sürücü düzeyinde kare aradeğerleme, sıfır mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - asla Community Shaders'ın kendi kare üretimiyle birlikte değil |
| bir hata bildirimi için ayrıntılı günlükler | `EIDOS_LOG=debug` (oturum günlükleri `~/.local/share/Colony/Eidos/logs/` içine iner) |
| bağlamadan oturum başına bir G/Ç raporu | `EIDOS_FUSE_STATS=1` |
| farklı bir FUSE işçi sayısı | `EIDOS_FUSE_THREADS=8` (öntanımlı 4; bir eşzamanlılık hatası kovalarken ilk denenecek şey `1`) |
| bu başlatmanın tek bir taşınabilir örneğe sabitlenmesi | `EIDOS_INSTANCE=/path/to/folder` - onsuz Eidos en son kullandığınız örneği açar, ki genelde istediğiniz de budur |

Modern bir modlu kurulum için (Community Shaders, DLSS, kare üretimi) saklanacak
satır - bu son komuttur, bir örnek değil:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Kurulumun çalıştığını doğrularken önüne `DXVK_HUD=fps` ekleyin, çalıştığında
kaldırın.

Daha derin tanılama anahtarları (`EIDOS_FUSE_TRACE`, önbellek ve indeks ikiye
bölme düğmeleri, `EIDOS_FUSE_PASSTHROUGH`'un neden öntanımlı olarak kapalı
olduğu) [guide/troubleshooting.md](docs/guide/troubleshooting.md) içinde.

## Belgeler

Her şey [docs/](docs/) içindedir ve kimin okuduğuna göre
[docs/README.md](docs/README.md) sayfasında dizinlenmiştir: kullanıcı kılavuzu
(kurulum, CLI ve GUI, xEdit ve DynDOLOD gibi araçlar, grafik, sorun giderme),
Eidos'un içeride nasıl çalıştığı ve neden var olduğu. Güvenlik politikası ve bir
güvenlik açığının nasıl bildirileceği: [SECURITY.md](../../../SECURITY.md).

## Dil

Bir oyuncunun ihtiyaç duyduğu sayfalar çevrilmiştir. **İngilizce asıldır**: bir
çeviri onunla uyuşmadığında, doğru olan İngilizce dosyadır.

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

**Geri kalan her şey ihmalden değil, bilerek İngilizcedir.** `docs/internals/`
ve `docs/project/`, aynı zamanda Rust'ı da okuyan kişilerce okunuyor,
`CHANGELOG.md` ise üretiliyor. Onları çevirmek, ihtiyacı olmayan bir kitle için
dürüst tutulacak 18.536 kelime daha demek olurdu.

Her çeviri, yapıldığı İngilizce dosyanın hash'ini taşır ve İngilizce ilerlediğinde
CI başarısız olur - bkz. [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Güncelliğine kavuşturulamayan bir çeviri yerinde bırakılmaz, **silinir**:
eskimiş bir sayfa hâlâ yetkili görünür ve geçen ayın komutlarını dağıtır, ki bu
okur için İngilizceye yönlendirilmekten daha kötüdür.

Bir dil tek bir dizindir. `docs/i18n/<lang>/` deponun kökünü yansıtır, yani
`docs/i18n/de/docs/guide/install.md` Almanca `docs/guide/install.md` dosyasıdır -
iki çevrilmiş sayfa arasındaki bağlantıyı İngilizce asıllarının arasındaki
bağlantıyla AYNI dize yapan da, bir dili emekliye ayırmayı tek bir `rm -r` yapan
da budur.


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

Eidos telemetri, analiz verisi ya da çökme raporu göndermez. Bu tablodakiler
dışında hiçbir sunucuya bağlanmaz ve onlara da yalnızca orada belirtilen eylem
için bağlanır. **Offline** kipi (Preferences -> Nexus), Nexus'a giden her
bağlantıyı durdurur.

| Veri | Saklanır ya da gönderilir | Nereye ve neden |
|---|---|---|
| Tercihler, örnek listesi, oyun ve eklenti tanımları | saklanır | `~/.config/Colony/Eidos/` |
| Örnekler, indirilen çalışma zamanları, oturum günlükleri | saklanır | `~/.local/share/Colony/Eidos/` ya da taşınabilir bir örnek için seçtiğiniz klasör |
| LOOT ana listeleri | saklanır | `~/.cache/Colony/Eidos/loot/` |
| Nexus oturum açma | gönderilir | Oturum açmak tarayıcınızda `users.nexusmods.com` adresini açar; site sonucu, Eidos'un yalnızca o oturum açma için `127.0.0.1` üzerinde açtığı bir dinleyiciye geri verir. Eidos ardından bunu `users.nexusmods.com/oauth/token` adresinde jetonlarla takas eder ve Nexus'un onları imzaladığı açık anahtarları getirir. [Nexus Mods gizlilik politikası](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus kimlik bilgileri | saklanır | Erişim ve yenileme jetonları, 0600 kipinde `~/.config/Colony/Eidos/nexus.ini` içinde. Daha eski bir Eidos'un oraya yazdığı bir `api_key=` satırı korunur ama asla okunmaz; dilerseniz silin. |
| Nexus istekleri | gönderilir | Bir Nexus özelliğini kullandığınızda `api.nexusmods.com`: hesap adınız ve premium durumunuz, mod ve dosya ayrıntıları, güncelleme denetimleri, koleksiyonlar ve indirme bağlantıları. Her istek erişim jetonunuzu taşır ve Eidos'u ve sürümünü belirtir. |
| Nexus indirmeleri | gönderilir | İstediğiniz dosyalar, indirme bağlantısının belirttiği Nexus CDN sunucusundan; örneğin indirmeler klasörüne, her birinin yanında MO2 biçiminde bir `.meta` ile kaydedilir. |
| `nxm://` bağlantıları | saklanır | `install.sh`'nin sizin için çalıştırdığı `eidos nxm --register`, `~/.local/share/applications/eidos-nxm.desktop` dosyasını yazar ve onu `xdg-mime` üzerinden `nxm://` işleyicisi yapar; böylece sitenin Mod Manager Download düğmesi bağlantısını Eidos'a verir. Arch paketi aynı girdiyi sistem genelinde kurar. |
| LOOT ana listeleri | gönderilir | Bir oyunun ilk LOOT sıralamasında ve her güncelleme istediğinizde `raw.githubusercontent.com/loot/...`. Size ait hiçbir şey gönderilmez. [GitHub gizlilik bildirimi](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Araç çalışma zamanları | gönderilir | Yalnızca bir araç ön gereksinimi kurduğunuzda ya da bir mod buna ihtiyaç duyduğunda: `builds.dotnet.microsoft.com` adresinden, Eidos'a gömülü SHA-256 ile eşleşmedikçe reddedilen .NET çalışma zamanı ve oyunun Proton önekine kurduğunu (Visual C++ ve .NET yeniden dağıtılabilir paketleri) Microsoft'tan ve winetricks'in kullandığı diğer sunuculardan indiren `winetricks`. [Microsoft gizlilik bildirimi](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Oturum günlükleri | saklanır | `~/.local/share/Colony/Eidos/logs/`, 0600 kipinde: örnek başına on oturum, 30 gün sonra temizlenir. Ev klasörünüz `~`, kullanıcı adınız `<user>` olarak yazılır, böylece bir günlük bir hata kaydına olduğu gibi yapıştırılabilir. Siz göndermedikçe makinenizden asla çıkmazlar. |
| Kısayollar | saklanır | `install.sh`, `eidos.desktop` dosyasını ve simgelerini `~/.local/share` altına yazar; oluşturduğunuz bir araç kısayolu `~/.local/share/applications` içine gider. |
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
bit); Eidos bunları bir mod ya da araç ihtiyaç duyduğunda oyunun önekine
yerleştirir. Microsoft'un yeniden dağıtım koşullarına tabi kalırlar; bkz.
[kaynak bilgileri](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Mod yönetimi herkesindir.
