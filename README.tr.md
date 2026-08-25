<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Oyununuza asla dokunmayan yerel Linux mod yöneticisi.**

</div>

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

**Örnekler.** Genel - `~/.local/share/eidos` altında merkezî olarak yönetilir -
ya da taşınabilir: istediğiniz her yerde kendi kendine yeten bir klasör (ikinci
bir disk, bir oyun bölümü), taşınabilir ve yalıtılmış, MO2'ninkiler gibi.
Taşınabilir örnekler oturumlar arasında hatırlanır; GUI, Steam başlatması ve her
CLI komutu en son kullandığınızı izler ve her komut, oyun kimliği aldığı her
yerde klasörü de alır. Ayrıntılar
[usage.tr.md](docs/guide/usage.tr.md#örnekler-global-ve-taşınabilir) içinde.

**Profiller.** Profil başına mod sırası, eklenti durumu, INI'ler ve kayıtlar.
Kayıtlar ayrıştırılır, geçerli eklentilerinizle karşılaştırılır - bir kaydın
gerektirdiğini etkinleştiren bir düğmeyle birlikte - ve her oturumdan sonra
Steam Cloud için geri eşitlenir.

**Nexus.** Bir hesap bağlayın; sitenin "Mod Manager Download" düğmesi doğrudan
örneğinize iner, kurduklarınıza karşı güncelleme denetimleri, her modu kimin
yaptığı ve profiline bir bağlantı ile birlikte. Bir **collection** bağlantısı,
üyelerini örneğinizle eşleştirerek listeler - kurulu, indirilmiş, eksik - ki bu
bir collection'ı kurmak değil okumaktır ve bölme bunun nedenini söyler.
Downloads sekmesi bir arşiv kitaplığıdır: süzün, sıralayın, silmeden gizleyin ve
zaten kurulmuş olanları temizleyin. Bir **çevrimdışı** anahtarı bunların tümünü
durdurur.

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

**Kendi dosyalarını nerede tutar.** Seçtikleriniz için
`~/.config/Colony/Eidos/` - tercihler, Nexus oturumunuz, örnek listeniz,
yazdığınız oyun ve eklenti tanımları - günlükler ise
`~/.local/state/Colony/Eidos/` altında. Colony ailesindeki her programın
kullandığı düzen. Daha eski bir Eidos bunları `~/.config/eidos/` içinde
tutuyordu; yükseltmeden sonraki ilk başlatma onları kopyalar, bunu günlükte
söyler ve eski klasörü tam olarak eskisi gibi bırakır.

## Nasıl kıyaslanıyor

| | Eidos | Wine üzerinden MO2 | Fluorine-Manager | Limo / bağlantı dağıtıcıları |
|---|---|---|---|---|
| Yönetici doğal çalışır | ✅ | ❌ Wine içinde Windows uygulaması | ✅ (Qt portu) | ✅ |
| Oyun klasörüne dokunulmaz | ✅ her zaman | ✅ | ✅ | ❌ içine bağlar yazılır |
| Bağlamayı gören | yalnızca oyun | yalnızca oyun | **tüm sistem** | yok |
| Çökme sonrası temizlik | tasarım gereği yok | yok | eskimiş bağlamayı kurtarma | elle geri alma |
| Root modları (ENB, ön yükleyiciler) | ✅ doğal | eklenti gerekir | eklenti gerekir | kısmi |
| Gereken yetkiler | yok | yok | `/etc/fuse.conf` düzenlemesi | yok |

## Ne kadar hızlı

| | önce | şimdi |
|---|---|---|
| bir kaydı yüklemek | ~20 saniye | **6-7 saniye** |
| bir oturumdaki klasör okumaları | 5,6 milyon | 465 bin |

Hücre geçişleri anında. Kazanç, modlarınıza daha az soru sormaktan geldi: tek
bir dosyayı bulmak eskiden ellisini de sırayla sorgulardı, tek bir klasörü
listelemek de bunu elli kez yapardı. Artık ikisi de yapmıyor. Bir kıyaslama
üzerinde değil, normal oynanan gerçek bir örnek üzerinde ölçüldü.

## Başlarken

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Sonra oyununuzun Steam başlatma seçeneğini `~/.local/bin/eidos-gui %command%`
olarak ayarlayın ve Oyna'ya basın.

Arch paketleri ve sürüm arşivleri, önce neyin kurulu olması gerektiği ve CLI
yolu: **[docs/guide/install.tr.md](docs/guide/install.tr.md)**.

## Steam başlatma seçenekleri

Temel satır, çoğu kurulumun ihtiyaç duyduğu her şeydir:

```
~/.local/bin/eidos-gui %command%
```

Geri kalan her şey onun önüne dizilen ortam değişkenleridir ve serbestçe
birleşirler:

| Şunu isterseniz... | Önüne koyun |
|---|---|
| Community Shaders ile DLSS | `PROTON_ENABLE_NVAPI=1` - onsuz DLSS sessizce hiç başlatılmaz; tam denetim listesi [guide/graphics.tr.md](docs/guide/graphics.tr.md) |
| ekranda bir FPS sayacı | `DXVK_HUD=fps` |
| sürücü düzeyinde kare aradeğerleme, sıfır mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - asla Community Shaders'ın kendi kare üretimiyle birlikte değil |
| bir hata bildirimi için ayrıntılı günlükler | `EIDOS_LOG=debug` (oturum günlükleri `~/.local/state/Colony/Eidos/logs/` içine iner) |
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
olduğu) [guide/troubleshooting.tr.md](docs/guide/troubleshooting.tr.md) içinde.

## Sonra nereye

| Şunu isterseniz... | |
|---|---|
| kurmak | [guide/install.tr.md](docs/guide/install.tr.md) |
| CLI'yi ve GUI'yi öğrenmek | [guide/usage.tr.md](docs/guide/usage.tr.md) |
| xEdit, BodySlide ya da DynDOLOD'u kurmak | [guide/tools.tr.md](docs/guide/tools.tr.md) |
| Fallout 4 oynamak (F4SE, sürümler, NVIDIA debris çökmesi) | [guide/fallout4.tr.md](docs/guide/fallout4.tr.md) |
| DLSS / kare üretimini çalıştırmak (Community Shaders) | [guide/graphics.tr.md](docs/guide/graphics.tr.md) |
| yanlış görünen bir şeyi düzeltmek | [guide/troubleshooting.tr.md](docs/guide/troubleshooting.tr.md) |
| neden hızlı olduğunu bilmek ve kendiniz denetlemek | [internals/performance.md](docs/internals/performance.md) |
| içeride nasıl çalıştığını anlamak | [internals/architecture.md](docs/internals/architecture.md) |
| derlemek, sınamak, katkıda bulunmak | [internals/contributing.md](docs/internals/contributing.md) |
| neden var olduğunu bilmek | [project/landscape.md](docs/project/landscape.md) |

Tam dizin [docs/README.tr.md](docs/README.tr.md) içinde; güvenlik ilkesi ve bir
açığın nasıl bildirileceği [SECURITY.md](SECURITY.md) içinde.

## Dil

Bir oyuncunun ihtiyaç duyduğu sayfalar çevrilmiştir. **İngilizce asıldır**: bir
çeviri onunla uyuşmadığında, doğru olan İngilizce dosyadır.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Geri kalan her şey ihmalden değil, bilerek İngilizcedir.** `docs/internals/`
ve `docs/project/`, aynı zamanda Rust'ı da okuyan kişilerce okunuyor,
`CHANGELOG.md` ise üretiliyor. Onları çevirmek, ihtiyacı olmayan bir kitle için
dürüst tutulacak 17.678 kelime daha demek olurdu.

Her çeviri, yapıldığı İngilizce dosyanın hash'ini taşır ve İngilizce ilerlediğinde
CI başarısız olur - bkz. [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Güncelliğine kavuşturulamayan bir çeviri yerinde bırakılmaz, **silinir**:
eskimiş bir sayfa hâlâ yetkili görünür ve geçen ayın komutlarını dağıtır, ki bu
okur için İngilizceye yönlendirilmekten daha kötüdür.

Bir dil eklemek dört dosya ve bu tabloda bir satır demektir;
[`docs/internals/contributing.md`](docs/internals/contributing.md) adımları içerir.

## Desteklenen oyunlar

**Skyrim SE/AE** - gerçek oyunda kanıtlanmış. **Fallout 4** de baştan sona
bağlanmış durumda (F4SE otomatik olarak devreye alınır, arşiv geçersizleme,
yıldızlı yükleme sırası, LOOT, `.fos` kayıtları) - bkz.
[guide/fallout4.tr.md](docs/guide/fallout4.tr.md). Ortak oyun tanımlayıcısına göre
bağlanmış ve test edenleri arayanlar: Skyrim LE, Skyrim VR, Enderal SE, Fallout
3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion ve Morrowind (son ikisi
modları bağlar ve yönetir; zaman damgasına göre sıralanan eklenti listeleri
henüz yönetilmiyor).

Bir aile eklemek tek bir tanımlayıcı satırıdır:
[internals/adding-games.md](docs/internals/adding-games.md).

## Önceki çalışmalar ve teşekkürler

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) ve
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos'un yeniden ürettiği
  anlambilim ve denkliğinin karşısında incelendiği kod tabanı
- [LOOT](https://loot.github.io/) - libloot üzerinden sıralama motoru
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) ve diğer Linux yöneticileri - bunun
  çözülmesini isteyen bir topluluk olduğunun kanıtı

## Lisans

GPL-3.0-or-later. Mod yönetimi herkesindir.
