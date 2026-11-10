<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Eidos kurulumu

Dört giriş yolu. Colony, programın tamamı olan tek bir dosya kurar: `eidos-linux`.
Sürüm paketi, Arch paketi ve kaynaktan derleme `eidos` (komut satırı), `eidos-gui`
ve statik NIF önizleme yardımcısı `eidos-nif-preview` dosyalarını yan yana verir.
Arch paketi ve kaynaktan derleme ayrıca iki alt düzey komut satırı olan
`eidos-fuse` ve `eidos-launch` dosyalarını da kurar. Her yol, Nexus'taki "Mod
Manager Download" düğmesinin indirilen dosyayı örneğinize ulaştırmasını sağlayan
`nxm://` işleyicisini kaydedebilir.

## Önce gerekenler

| Gereksinim | Neden |
|---|---|
| FUSE'lu Linux | PATH'inizde `fusermount3`. Güncel her dağıtım onu getirir. |
| Bir kez başlatılmış bir Proton oyunu | Steam oyunun Wine önekini yalnızca ilk açılışta oluşturur ve Eidos onun içinde çalışır. Eidos ayrıca Skyrim Special Edition'ın GOG ve Epic kopyalarını, Fallout 3 ve Fallout: New Vegas'ın Epic kopyalarını ve Stardew Valley'in GOG kopyasını da algılar. Bunlar Heroic ya da Legendary üzerinden, Eidos oyunun wrapper'ı olarak çalışır: bkz. [usage.md](usage.md#oyunu-başlatmak). |
| Bir 7-Zip ikili dosyası: `7z`, `7zz` ya da `7za` | Mod arşivlerini kurmak için. Arch'ta `7zip`, çoğu başka dağıtımda `p7zip`. |
| `par2` (isteğe bağlı) | `eidos pack` yedekleri için PAR2 kurtarma verisi. Arch ve Fedora'da `par2cmdline`, Debian ve Ubuntu'da `par2`. |

root yok, artalan süreci yok, `/etc/fuse.conf` düzenlemesi yok, gruplarınıza
eklenecek bir şey yok. Eidos, oyun sürecine ait özel bir ad alanının içinde
bağlar.

Bazı sistem ayarları yalnızca çökme olarak ortaya çıkar. Diagnostics sekmesi
`vm.max_map_count` (1048576 ya da daha fazla), sabit açık dosya sınırı (524288 ya
da daha fazla) ve oyunun, önekinin ya da örneğin NTFS, exFAT veya FAT üzerinde
durup durmadığını denetler; her birinin çözümü
[troubleshooting.md](troubleshooting.md#sistem) içinde.

## Colony üzerinden

[Colony](https://github.com/Project-Colony/Colony) içinde Eidos'u arayın ve
kurun; güncellemeler başlatıcı üzerinden gelir. Colony tek bir dosya tutar:
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Onu çalıştırın, pencere açılır.
`eidos` adlı bir bağlantı üzerinden çağrıldığında aynı dosya komut satırıdır.
`PATH`'inize bir tane koyun ve Nexus indirme işleyicisini kaydedin:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # sitenin Mod Manager Download düğmesi artık Eidos'a ulaşır
```

Pencerenin bağlantıya ihtiyacı yoktur: bir oyunu ya da aracı her başlattığında
kendi dosyasını komut satırı olarak çalıştırır. NIF model önizlemesi
`eidos-nif-preview` gerektirir; onu yalnızca sürüm paketi, Arch paketi ve
kaynaktan derlemeler içerir. O olmadan önizleme bunu söyler ve başka hiçbir şey
etkilenmez. `eidos-linux` ayrıca paketin getirdiği Microsoft çalışma zamanlarından
hiçbirini taşımaz ([aşağıda](#paketteki-microsoft-çalışma-zamanları)): bir araç
birine ihtiyaç duyduğunda Eidos, ön koşul kurulumunu başlattığınızda onu
Microsoft'tan indirir. Steam'i aşağıda olduğu gibi dosyanın kendisine yöneltin.

## Bir sürüm paketi

[Son sürümden](https://github.com/Project-Colony/Eidos/releases/latest)
`eidos-linux-bundle.tar.gz` dosyasını ve onun `.sig`, `.meta` ve `.meta.sig`
dosyalarını indirin ve README'deki
[Kod imzalama politikası](../../README.md#kod-imzalama-politikası) bölümünün
gösterdiği gibi denetleyin. Paketin üst düzeyinde `eidos`, `eidos-gui` ve
`eidos-nif-preview` bulunur; yanlarında `install.sh`, `README.md`, `LICENSE`,
`SECURITY.md`, `assets/brand/png` içindeki simgeler, `licenses/` içindeki üçüncü
taraf bildirimleri (nifly, omod, microsoft-directx, microsoft-runtimes ve Rust
crate'leri) ve aşağıdaki `runtimes/` klasörü vardır. `eidos-fuse` ya da
`eidos-launch` taşımaz. Ardından:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` ikili dosyaları `~/.local/bin` içine, `$XDG_BIN_HOME` ayarlıysa oraya
kopyalar. `--system` onları sudo ile `/usr/local/bin` içine, `--bindir DIR` başka
herhangi bir yere koyar, `--from DIR` ikili dosyaları başka bir dizinden alır.
Betik ayrıca şunları yapar:

- `eidos nxm --register` çalıştırarak `nxm://` işleyicisini kaydeder; bu yüzden
  Colony bölümündeki kayıt adımı gerekmez;
- `~/.local/share/applications/eidos.desktop` dosyasını ve Eidos simgelerini
  `~/.local/share/icons/hicolor` altına yazar;
- lisans bildirimlerini `~/.local/share/licenses/eidos` içine kopyalar
  (`--system` ile `/usr/local/share/licenses/eidos`);
- `runtimes/` klasörünü `~/.local/share/Colony/Eidos/runtimes` içine kopyalar,
  `--system` ile de; böylece çalışma zamanları betiği çalıştıran kullanıcıya ait
  olur;
- sonunda, kurduğu `eidos-gui` dosyasının mutlak yoluyla Steam başlatma
  seçeneğini yazdırır.

`--cap`, sudo'dan kurulan `eidos` dosyasına `CAP_SYS_ADMIN` vermesini ister
(aşağıdaki FUSE passthrough bölümüne bakın). `--no-cap` öntanımlıdır ve root
gerektirmez. Betiği yeniden çalıştırmak, yükseltmenin desteklenen yoludur. Paket
ve `eidos-linux`, glibc 2.39 ya da daha yenisine sahip x86_64 Linux gerektirir.

### Paketteki Microsoft çalışma zamanları

Paketin `runtimes/` klasörü, araçların istediği Microsoft dosyalarını taşır:
`VC_redist.x86.exe` ve `VC_redist.x64.exe` (Visual C++ 2015-2022, `vcrun2022`
ön koşulu), `windowsdesktop-runtime-8.0.12-win-x64.exe` (`dotnet8` ve
`dotnetdesktop8` için .NET 8 Desktop Runtime) ve
`dotnet-runtime-10.0.10-win-x64.zip` (`dotnet10` için .NET 10 çalışma zamanı).
Eidos Visual C++ ve .NET 8 kurucularını oyunun önekinde winetricks olmadan kendisi
çalıştırır, .NET 10 zip dosyasını da bir kez `~/.local/share/Colony/Eidos/runtimes`
içine açar; her örnek orayı paylaşır (açılmış hâli 78 MB). Her dosya yalnızca
SHA-256 değeri Eidos'a sabitlenmiş değerle eşleşirse kullanılır. Synthesis'in
istediği .NET 10 SDK (`dotnetsdk10`, yaklaşık 300 MB) pakette yoktur ve her zaman
indirilir.

Açılmış bir paket olduğu yerde de çalışır: Eidos yukarıdaki veri klasöründen sonra
kendi ikili dosyasının yanında `runtimes/` arar.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD, güncel `main` dalını kaynaktan derler. `eidos`, `eidos-gui`,
`eidos-nif-preview`, `eidos-fuse` ve `eidos-launch` dosyalarını `/usr/bin` içine,
sistem genelinde bir `nxm://` işleyicisi, bir masaüstü girdisi ve simgelerle
birlikte kurar. `fuse3`, `7zip` ve `xdg-utils` paketlerine bağlıdır. `runtimes/`
klasörü getirmez; bu yüzden Microsoft çalışma zamanları bir araç gerek duyduğunda
indirilir. `winetricks` isteğe bağlıdır: Eidos kendi kurmadığı ön koşullar için,
örneğin `dotnet48` ve `xact`, onu hâlâ kullanır.

## Kaynaktan

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Kaynaktan derleme için Rust 1.92 ya da daha yenisi, `just`, CMake 3.20+, bir C++17
derleyicisi ve yerel yardımcının testleri için Python 3 gerekir. `just build` tüm
çalışma alanını derler, yani `eidos-fuse` ve `eidos-launch` de derlenir; `just
install` derleme çıktısı üzerinde `packaging/install.sh` çalıştırır ve onları
diğerleriyle birlikte kopyalar. `just` olmadan kullanılacak komutlar için
[yerel yardımcı yönergelerine](../../../../../crates/eidos-nif-preview/README.md)
bakın; Rust'ı `cargo build --release --locked` ile derleyin, yardımcıyı
`target/release` içine kopyalayın ve ardından
`packaging/install.sh --from target/release` çalıştırın. Kaynaktan derlemede de
`runtimes/` klasörü yoktur; Microsoft çalışma zamanları bir araç gerek duyduğunda
indirilir.

Release profili tek bir codegen unit, fat LTO ve soyulmuş semboller kullanır.
rustc 1.99.1 ve 3 derleme işiyle `eidos` ile `eidos-gui` için soğuk bir derleme
334 s sürdü; thin LTO ile 243 s sürüyordu. Sonuç 20,9 MB'lık bir `eidos` ve 49,7
MB'lık bir `eidos-gui` oldu. Her ikili dosya, gzip ile sıkıştırılmış sekiz DirectX
DLL'si gömer: 17,9 MB yerine 7,9 MB. `just profile=dev build` eniyileme olmadan
derler.

## Sonra: Steam'i ona yöneltin

Eidos, oyununuzun başlatma komutu *olarak* çalışır; oyun başlamadan önce
bağlamayı böyle başarır. Steam'de oyuna sağ tıklayın -> Özellikler -> Başlatma
Seçenekleri ve Eidos penceresinin mutlak yolunu, ardından `%command%` yazın:

```
~/.local/bin/eidos-gui %command%
```

Mutlak yolu kullanın - Steam kabuğunuzun `PATH`'ini okumaz. Yol Eidos'u nasıl
kurduğunuza bağlıdır ve `install.sh` kullandığı dizin için satırı yazdırır:

| Kurulum | Başlatma seçeneği |
|---|---|
| Sürüm paketi ya da `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch paketi | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Oyna'ya basın. Eidos o oyunun örneğinde açılır; mod kurun, LOOT ile sıralayın,
Run'a tıklayın. Çıktığınızda bağlama da onunla gider ve kurulumunuz tam olarak
eskisi gibidir.

Oyun her zaman mağazası üzerinden başlar. Kendiniz açtığınız bir pencerede Run,
`steam -applaunch <appid>` çalıştırır, Steam Eidos'u başlatma seçeneğiyle başlatır
ve Run'a bastığınız ikinci bir Eidos penceresi açılır. Başlatma seçeneği Eidos'u
başlatana kadar Run hiçbir şey başlatmaz ve durum satırı ayarlanacak satırı
gösterir. Start the game at once when a store opens Eidos açıkken (Preferences,
General, Running a game; öntanımlı olarak kapalı), Health hiçbir sorun
listelemiyorsa Steam'de Play oyunu pencere olmadan başlatır. Heroic ve Legendary
wrapper ayarları üzerinden aynı şekilde çalışır; ayrıntılar
[usage.md](usage.md#oyunu-başlatmak) içinde.

Diagnostics sekmesi satırı denetler. Başlatma seçeneği boşsa, hiçbir Eidos ikili
dosyası adlandırmıyorsa, var olmayan bir dosyayı gösteriyorsa, Eidos'tan sonra
`%command%` yoksa ya da çalışandan başka bir Eidos kopyasını başlatıyorsa bir
Health kartı ayarlanacak satırı gösterir.

### Terminali yeğliyorsanız

```sh
eidos init skyrimse               # bir örnek oluştur (klasör verirseniz taşınabilir olur)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root modları
eidos sort skyrimse               # yükleme sırasını LOOT ile sırala
eidos check skyrimse              # pencere gerektirmeyen Health denetimleri
eidos play skyrimse -- %command%  # herhangi bir şeyi birleşik görünüm üzerinden çalıştır
eidos shell skyrimse              # birleşik görünümde bir kabuk, yazmalar Overwrite'a iner
```

Oyun kimliği alan her komut, taşınabilir bir örneğin klasörünü de alır -
bkz. [usage.md](usage.md#örnekler-global-ve-taşınabilir). Tam gezinti
[usage.md](usage.md) içindedir.

## Güncelleme

Colony `eidos-linux` dosyasını başlatıcısı üzerinden günceller. Paket için yeni
paketin `install.sh` dosyasını çalıştırın; Arch paketi için yeniden `makepkg -si`;
kaynaktan derleme için çekin ve `just install` çalıştırın. Pencere, başlangıçta
GitHub'dan son sürümü, günde en fazla bir kez sorar ve daha yeni bir sürüm
çıktıysa menü çubuğunda Help'ten sonra sürüm notlarına bir bağlantı gösterir.
Kendini hiçbir zaman indirmez ya da değiştirmez. Preferences, About, Version
denetimi kapatır, çevrimdışı kip atlar ve `/usr` altına kurulmuş bir kopya (Arch
paketi) hiç sormaz, çünkü onu paket yöneticisi günceller.

## Seçmeli: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` isteğe bağlı yetkiyi verir,
ancak passthrough özelliğini etkinleştirmez. `./install.sh --cap` ve `just setcap`
onu kurdukları ya da derledikleri `eidos` dosyasına verir. `nosuid` bir bağlama
noktasında çekirdek dosya yetkilerini yok sayar; bu yüzden `install.sh` yetki
vermeyi bir uyarıyla atlar, `just setcap` bir hatayla durur. Yetki paketin, Arch
paketinin ya da bir kaynak derlemesinin ayrı `eidos` ikili dosyasına verilir,
hiçbir zaman aynı zamanda pencere de olan `eidos-linux` dosyasına verilmez
([SECURITY.md](../../../../../SECURITY.md)). Yeniden derlenmiş ya da yeniden
kurulmuş bir ikili dosya yetkisiz yeni bir dosyadır; kullanıyorsanız her
yükseltmeden sonra yeniden verin.

`EIDOS_FUSE_PASSTHROUGH=1` çalışma zamanında kullanılan ayrı anahtardır ve
öntanımlı olarak kapalıdır. Passthrough açıkken daemon'un önceki bir sürümü
Skyrim SE'nin hiçbir mod içeriğini yüklemesine izin vermiyordu. En olası neden,
her açmada yeni bir dayanak dosya kaydetmesiydi; çekirdek ise inode başına yalnızca
bir tane kabul eder. Daemon artık bir dosyanın tüm passthrough açmaları arasında
bir tane paylaştırıyor, ama bu henüz bir oyunda yeniden ölçülmedi. O zamana kadar
passthrough'u yalnızca sınamak için açın. Ölçümler ve neden
[troubleshooting.md](troubleshooting.md) içinde.

## Şimdiden bir sorun mu var?

[troubleshooting.md](troubleshooting.md) ortam anahtarlarını, işlem sayaçlarının
nasıl okunacağını, Health kartlarını ve şimdiye dek birini ısırmış her sorunu
anlatır.
