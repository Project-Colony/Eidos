<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# Sorun giderme ve tanılama

Ortam anahtarları, işlem sayaçlarının nasıl okunacağı, Diagnostics sekmesindeki
Health kartları, bir çökmenin arkasındaki modun nasıl bulunacağı, bir hata
raporuna neyin ekleneceği ve bilinen sorunlar ile geçmişleri.

## VFS'i tanılamak

Oyunun, dosya sisteminin katılmadığı bir şey gördüğü durumların çoğunu iki ortam
değişkeni kapsar:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

Bilinen sorunlar altındaki `FOPEN_KEEP_CACHE` çökmesini bulan, ayrıntılı biçim
oldu: dördünü birden kapatmak "önbellekleme mi?" sorusunu yanıtlar, yalnızca
adlarını vermek "hangisi" sorusunu yanıtlar. Sayaçlar diğer yarısını yanıtlar.
Passthrough açıkken (öntanımlı olarak kapalıdır) `read 0` gösteren bir yükleme,
`FUSE_PASSTHROUGH`'un her baytı çekirdekte sunduğu bir yüklemedir; yani okuma
yolunda ayarlamak üzere olduğunuz her şey zaten bedavadır.

Birkaç anahtar daha bir arızayı daha da daraltır:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

Bir Steam başlatmasında bunları başlatma seçeneğinde komutun önüne yazın, örneğin
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. Çıktı çalıştırmanın
günlüğüne gider: pencereden başlatmada örneğin `logs/run-*.log` dosyasına,
Steam'den başlatmada Steam konsoluna.

Birleşik görünümün kendisine bakmak için `eidos shell <instance>`, oyunun gördüğü
hâliyle oyun klasöründe bir kabuk açar ve
`eidos shell <instance> -- ls Data/meshes` orada tek bir komut çalıştırır; yazılan
her şey Overwrite'a iner ([usage.md](usage.md#birleşik-görünümde-bir-kabuk)). Data
sekmesindeki Save tree düğmesi, birleşik `Data` ağacının tamamını, satır başına bir
dosya ve geldiği modla, ev klasörünüz gizlenmiş olarak bir metin dosyasına yazar.

## Bir birleşimi elle bağlamak

`eidos-fuse` Arch paketiyle ve kaynaktan derlemelerle gelir: `just build` onu
derler, `just install` ya da `install.sh` orada olduğunda kopyalar. Sürüm paketi
ve `eidos-linux` onu taşımaz. Kullanım satırı:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

Çakışmada ilk `--layer` kazanır; sonuncusu dokunulmamış oyun verinizdir. Bağlama
yalnızca `/dev/fuse` ve `fusermount3` ister (overlayfs yok, Wine yok):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Yazmalar `--overwrite <dir>` içine düşer (verilmediğinde geçici bir dizin),
böylece burada bile katmanların kendisi dokunulmamış kalır.

## Passthrough neden öntanımlı olarak kapalı

Passthrough, çekirdeğe gerçek dayanak dosyasını verir, böylece okumalar bu
daemon'u büsbütün atlar. Aşağıdaki A/B, daemon'un önceki bir sürümüyle, ileride
"FUSE passthrough, oyunun hiçbir mod içeriğini yüklemesine engel oluyor" bilinen
sorununda anlatılan değişiklikten önce ölçüldü. Bu değişiklik en olası nedenden
kaçınıyor, ama henüz bir oyunda yeniden ölçülmedi, bu yüzden passthrough o
ölçülene kadar öntanımlı olarak kapalı kalıyor. Skyrim SE 1.6.1170,
proton-cachyos 11.0, çekirdek 7.1.4, aynı 82 eklentilik yükleme sırası üzerinde
A/B ölçüldü; tek değişken, ikili dosyanın yeteneği taşıyıp taşımadığıydı:

| passthrough | `STATUS_ACCESS_VIOLATION` ile `NtCreateFile` başarısızlıkları |
|-------------|--------------------------------------------------------------|
| açık        | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`              |
| kapalı      | 0                                                            |

Açıkken oyun kendi arşivlerinin ya da eklentilerinin hiçbirini açmıyordu; bu,
oyun içinde orada olmayan modlar olarak görünüyordu - hata yok, günlük satırı
yok. Kapalıyken aynı yükleme sırası, eklentileri, arşivleri ve Papyrus betikleri
çalışır durumdayken oyuna ulaşıyordu.

Bu hata daemon'un içinden görünmüyordu: daemon'un kendi `open`'ı her seferinde
başarılı oluyordu ve çekirdek bir dayanak dosyasını hiçbir zaman geri çevirmiyordu
(`EIDOS_FUSE_TRACE=open` ile, başarısız olan tam bir oturum boyunca doğrulandı:
sıfır `open FAILED`, sıfır `passthrough refused`). Hata, daemon `opened_passthrough`
yanıtını verdikten sonra üretiliyordu, dolayısıyla daemon tarafındaki hiçbir
günlükleme onu göremiyordu. Uzantıya özgü de değildi: arşivleri de eklentileri de,
yani oyunun çalışması boyunca açık tuttuğu dosyaları vuruyordu.

`EIDOS_FUSE_PASSTHROUGH=1` onu geri açar; ne kazandırdığını ölçmek ya da düzeneği
yeniden sınamak için. Başlatıcıdaki ve Diagnostics sekmesindeki yetenek uyarıları
yalnızca siz istediğinizde belirir.

## Katman indeksi gerçekten kullanılıyor mu?

`LayerStack::new` salt okunur katmanların ya eksiksiz bir haritasını kurar
ya da hiç kurmaz. Harita yoksa her sorgu katmanları eskisi gibi dolaşır: yanıtlar
aynıdır, bir kaydın yüklenmesi yaklaşık üç kat uzun sürer ve başka hiçbir şey
farklı görünmez.

İndeks kapalıyken bağlama, başlangıçta bir kez nedenini yazar:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

Neden; okunamayan bir dizini, 64 dizinden derin bir katmanı, 4.000.000'dan fazla
girdiyi ya da `EIDOS_NO_INDEX`'i adlandırır. Satır çalıştırmanın günlüğüne gider:
pencereden başlatmada örneğin `logs/run-*.log` dosyasına, Steam'den başlatmada
Steam konsoluna. `EIDOS_FUSE_STATS=1` ile ayırma sırasındaki dökümde bir
`layer index:` satırı da bulunur; normal bir bağlamada `unindexed` sayacının sıfır
olmaması, indeksin kapalı olduğu anlamına gelir.

Aşağıdaki örnekler aynı şeyi çevrimdışı, bir örneğin klasörleri üzerinde,
bağlama yapmadan denetler:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` gerçek yolları indeksle ve indekssiz çözer, sonra dizin
taramalarını karşılaştırır. `index_agrees` ikisinin AYNI yanıtı verdiğini,
gerçek bir örneğin her yolunda ve her listelemesinde denetler. `listing_cost`
birleştirilmiş çocuk haritasının `readdir` üzerinde ne kazandırdığını ölçer.

`EIDOS_NO_INDEX=1` dolaşmayı zorlar; iki yanıt arasındaki farkın kendisi hata
ayıklanan şey olduğunda işe yarar.

## Oyunu CLI ile başlatmak

Oyunu pencere yerine `eidos` CLI'ı ile başlatmak için Steam başlatma seçeneğini
`eidos` dosyasının mutlak yoluna ayarlayın, örneğin:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Başına bir `WINEDLLOVERRIDES` eklemek gerekmez. Etkin bir mod `d3dcompiler_47`
içe aktardığında Eidos kendi paketlediği kopyayı öneke yerleştirir, onu native
olmaya zorlar ve bunu bir modun getirdiği DLL geçersiz kılmalarıyla birleştirir
(ENB/ReShade/`.asi` yükleyicileri).

Eidos oyunu hiçbir zaman kendisi Proton üzerinden başlatmaz. Oyunu başlatan bir
girdiyle (script extender'ın yükleyicisi, başlatıcı ya da oyun ikili dosyası)
`eidos tool <instance> run <title>`, kopyanın sahibi mağazadan onu başlatmasını
ister; bunu bir mağazanın açmadığı bir pencerede Run'ın yaptığı gibi yapar ve
mağazanın ayarı Eidos'u başlatmıyorsa neyin ayarlanacağını yazdırır. `--print`
komutu çalıştırmak yerine mağaza komutunu yazdırır.

## Kapanmayan bir oyun ya da araç

Bir oyun ya da araç çalışırken Run düğmesi Stop olur, kilit kartında da bir
Stop düğmesi vardır. Ona iki kez basın. İkinci basış, çalıştırmanın başlattığı
her programı sonlandırır: önce SIGTERM, 10 saniye sonra da geriye kalanlar için
SIGKILL. Oyunun kaydetmediği her şey kaybolur. Ardından Eidos her çıkıştan sonra
yaptığını yapar: INI ve eklenti değişiklikleri profile geri döner, araç çıktısı
kaydedilir ve modların bağlantısı kaldırılır. Çalıştırma, program ne döndürürse
döndürsün 143 çıkış koduyla kaydedilir; bu yüzden `eidos sessions <instance> diff`
onu hiçbir zaman son iyi çalıştırma olarak almaz. Unlock farklıdır: pencereyi
geri verir ve programı çalışır durumda bırakır. Bir terminalden, `eidos`
sürecine `kill <pid>` (pid kilit kartında yazar) Stop ile aynı işi yapar.

## Neyin bozulduğunu bulmak

Oyunu hangi değişikliğin bozduğunu bulmak için:

- `eidos sessions <instance> diff`, 0 koduyla çıkan son oyun çalıştırmasından beri
  neyin değiştiğini listeler: modlar, sürümler, eklenti sırası, INI'ler ve oyunun
  çalıştırılabilir dosyası
  ([usage.md](usage.md#son-iyi-çalıştırmadan-beri-neyin-değiştiği)).
- Run'ın yanındaki Test run, profilin modlarıyla, modsuz ya da seçili modlarla ve
  onların gereksindiği master'larla oynar ve oyunun yazdığı her şeyi
  `<instance>/test-run/` içine gönderir. Çalıştırmadan sonra her dosya grubunu
  tutar ya da atarsınız: kayıtlar, INI değişiklikleri, eklenti durumu, Overwrite
  çıktısı ve günlükler ([usage.md](usage.md#test-çalıştırmaları)).
- Diagnostics sekmesindeki Find the mod behind a problem, her seferinde kalan
  şüphelilerin yarısıyla test çalıştırmaları yapar ve sorunun oluşup oluşmadığını
  sorar. 200 şüpheli yaklaşık 8 çalıştırma sürer ve arama Eidos kapatılsa da
  sürer ([usage.md](usage.md#bir-sorunun-arkasındaki-modu-bulmak)).

Durdurulmuş bir çalıştırma o aramada hiçbir zaman önceden Yes doldurmaz, çünkü 143
çıkış koduyla kaydedilir (yukarıya bakın).

## Health kartları

Diagnostics sekmesi, bulduğu her sorun için nedeniyle birlikte, bazıları için de
onu düzelten bir düğmeyle bir Health kartı gösterir. Başlatma seçeneği kartları
[install.md](install.md) içinde, NVAPI, tam ekran ve ENB kartları
[graphics.md](graphics.md) içinde, Proton ve önek kartları bir sonraki bölümde
anlatılır. `eidos check <instance>` pencere gerektirmeyen bulguları yazdırır
([usage.md](usage.md#terminalden-sağlık-denetimleri) onları listeler); önek,
kayıt defteri ve `S:` sürücüsü kartları bunların arasındadır. Yalnızca pencere,
Flatpak, bilinmeyen sürüm ve daha eski sürüm varyantlarıyla Proton kartını, yama
sırasını, extender betiklerini, hiçbir eklentinin yüklemediği arşivleri, Nexus
gereksinimlerini, eski eklenti kopyalarını ve Display kartını gösterir.

### Sistem

- "vm.max_map_count is <value>", Arch Linux'un, Fedora 39 ve sonrasının, Ubuntu
  24.04 ve sonrasının öntanımlı değeri olan 1048576'nın altında görünür. Proton
  altındaki bazı oyunlar daha fazla bellek bölgesi eşler ve sonra hatasız çöker.
  `/etc/sysctl.d` altındaki bir dosyaya, örneğin `80-gamecompatibility.conf`,
  `vm.max_map_count = 1048576` yazın, sonra `sudo sysctl --system` çalıştırın.
- "The open files limit is <value>", sabit sınır Proton'un esync'inin gereksindiği
  524288'in altındayken görünür; bunun altında oyunlar başlamayabilir ya da
  çökebilir. systemd 240 ve sonrası bunu ayarlar. Daha eski bir düzende
  `/etc/systemd/system.conf` ve `/etc/systemd/user.conf` içinde
  `DefaultLimitNOFILE=524288` ayarlayın, sonra yeniden oturum açın.
- "Game files on a Windows file system", oyun klasörü, önek ya da örnek kökü
  `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse), `exfat` ya da `vfat` üzerinde
  olduğunda onu adlandırır. Proton sembolik bağlara, Unix izinlerine ve harf
  duyarlı adlara ihtiyaç duyar. Oyunu, önekini ve örneği ext4, btrfs ya da xfs
  gibi bir Linux dosya sistemine taşıyın.

### Oyun dosyaları

Sekmenin üstündeki Game files kartı, örneğin mağaza kurulumunu mu yoksa örneğin
içindeki bir oyun kopyasını mı çalıştırdığını gösterir
([usage.md](usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya)). Başlatmalar
üç durumda, nedenini söyleyen bir kartla reddedilir:

- "The game copy '<folder>' is missing": örnek, artık olmayan bir kopyayı
  çalıştırıyor. Onu geri koyun ya da mağaza sürümüne dönün.
- "'<folder>' has no <binary>": örneğin çalıştırdığı klasörde oyunun çalıştırılabilir
  dosyası yok, yani oyunun bir kopyası değil.
- "<game> cannot run from a copy yet": Morrowind ve `bUseMyGamesDirectory=0` ile
  Oblivion eklenti listesini oyun klasöründe tutar.

"ModOrganizer.ini points tools at another game folder", Eidos'un yazmadığı, bir
listenin kendi `ModOrganizer.ini` dosyasındaki `gamePath` örneğin çalıştırdığı
oyun dosyalarından başka bir yeri gösterdiğinde görünür. PGPatcher gibi Mod
Organizer 2 kipindeki araçlar oyunu oradan okur. O dosyada `gamePath` değerini
değiştirin ya da dosyayı kaldırın, böylece Eidos kendisininkini yazar.

### Yükleme sırası

- "sTestFile lines make the game ignore plugins.txt", profilin `Fallout4Custom.ini`
  ya da `StarfieldCustom.ini` dosyasında `[General]` altında `sTestFile1` ile
  `sTestFile10` arası satırlar olduğunda Fallout 4, Fallout 4 VR ve Starfield için
  görünür. Biri tanımlıyken oyun o eklentileri yükler ve Eidos'un yazdığı yükleme
  sırasının etkisi olmaz. Fix satırları profilin kopyasından kaldırır ve önceki
  dosyayı `<ini>.pre-fix` olarak tutar.
- Starfield: master bayrağı olmayan etkin `.esp` eklentileri bir kart alır, çünkü
  Starfield onları `.esm` ve master bayraklı eklentiler arasına sıralayamaz ve
  kayıtlarının bir kısmını oturum boyunca yüklü tutar. Yazardan bir `.esm` sürümü
  ikisini de önler.
- Starfield: "Invalid blueprint plugins", blueprint bayrağının bir kısmına sahip
  olup tamamına sahip olmayan, adı `BlueprintShips-` ile başlayan ve `.esm`
  uzantılı eklentileri listeler. "Blueprint plugins without their main plugin",
  ana eklentisi etkin olmayanları listeler: `BlueprintShips-Example.esm` için
  `Example.esm`, `.esp` ya da `.esl` gerekir.
- "<patch> loses <n> file(s) to <master>", bir moddaki eklenti başka bir moddan bir
  master gerektirdiğinde ve o öteki mod daha yüksek önceliğe sahip olduğunda,
  yani ikisinin de getirdiği dosyaların kendi kopyaları kazandığında görünür. Bir
  yama genellikle yamaladığı modun altına gider ve kartın Move düğmesi onu oraya
  koyar. Yama o dosyaları kaybetmek için yapılmışsa sırayı olduğu gibi bırakın.
- "Out-of-date copy of <plugin>", bir mod, orijinali sonradan değişmiş (bir mod ya da
  oyun güncellemesi) bir eklentinin temizlenmiş ya da light bayraklı bir kopyasını
  tuttuğunda görünür. Oyun eski kopyayı yükler. Onu gizleyin, sonra yeni eklentiyi
  yeniden temizleyin ya da bayraklayın.

### Mod listesi

- "Order rules of the collection <name> that no longer hold", kurulu bir koleksiyonun
  modları artık kurallarının istediği sırada durmadığında görünür. Restore
  collection order yalnızca o koleksiyonun modlarını geri taşır ve Ctrl+Z geri alır.
- "<mod> requires <mod>, which is not installed" (ya da "which is disabled"),
  Check for updates'in okuduğu Nexus gereksinimlerinden gelir. Open on Nexus
  gereken modun sayfasına gider, Download main file premium hesapta onu indirir ve
  Ignore, başka bir modun zaten karşıladığı bir gereksinimi gizler. Bir DLC
  gereksinimi yalnızca tavsiyedir: Eidos DLC'nin adını taşıyan bir eklenti arar.
- "fontconfig.txt names a font library that is missing" (Skyrim motorları), hiçbir
  katmanın ve hiçbir arşivin tutmadığı yazı tiplerini ve `Interface/fontconfig.txt`
  dosyası kazanan modu adlandırır. Yazı tipi kurulana ya da satır kaldırılana kadar
  menü metni kutular olarak görünebilir.
- "BA2 archive stores audio compressed" (Fallout 4 ve Fallout 4 VR), bir modun BA2
  arşivinin sıkıştırılmış sakladığı `.wav` ve `.xwm` dosyalarını sayar. Oyun onları
  çalarken çökebilir. Arşivi sesler sıkıştırılmadan yeniden paketleyin.

### Script extender ve oyun güncellemeleri

- "<loader> is not installed", script extender eklentileri kurulu olduğunda
  (`SKSE/Plugins`, `F4SE/Plugins` vb. içinde) ama yükleyici etkin hiçbir modda ve
  oyun klasöründe olmadığında görünür. Oyun onlarsız başlar.
- "The script extender does not match game version <version>", yükleyici kurulu
  ama çalışan oyun sürümünün çalışma zamanı DLL'si (örneğin
  `skse64_1_6_1170.dll`) kurulu olmadığında görünür. Yükleyici bir hatayla durur.
  Bu genellikle bir oyun güncellemesinin ardından olur. Bu kartın ve yukarıdakinin
  extender'ın sitesi için bir Open the download page düğmesi ve yükleyiciyi modun
  `Root/` klasörüne koyan bir Install the downloaded archive düğmesi vardır.
- "<game> changed since the last clean launch", oyunun çalıştırılabilir dosyası,
  Steam kopyasında ise yapı kimliği, Eidos üzerinden hatasız çıkan son başlatmadan
  beri değiştiğinde görünür. Çalıştırılabilir dosyayı olduğu gibi bırakan bir Steam
  güncellemesi onu doğurmaz. Eski sürüm için derlenmiş script extender eklentileri
  yüklenmeyebilir ve aşağıdaki kartlar bunu bildirenleri adlandırır. Bir listenin
  üzerine kurulduğu oyun sürümünü korumak için onu Game files kartındaki Freeze
  this version ile ya da boyutu `freeze` tek başına gösterdikten sonra
  `eidos stock-game <instance> freeze --yes` ile örneğe dondurun: sonraki mağaza
  güncellemeleri mağaza kurulumunu değiştirir, kopyayı değil.
- "<mod> replaces <n> <extender> script(s)", script extender'dan yüksek öncelikli bir
  mod, extender'ın kendi betiklerinin serbest `.pex` kopyalarını taşıdığında
  görünür. Eski bir kopya bildirdiği işlevleri bozar ve oyun hata göstermez.
  Extender'ı o modun altına taşıyın (ikisi de mod olduğunda kartın Move düğmesi
  bunu yapar) ya da moddaki o dosyaları gizleyin. Extender'ın betikleri oyunun
  `Data` klasöründe serbest duruyorsa, sıralanabilmesi için extender'ı mod olarak
  kurun. Arşivler denetlenmez, yalnızca serbest dosyalar.

### Araç çıktısı

- "Overwrite/Root holds meshes" (ya da textures), veri yolu oyun klasörünü gösteren
  bir aracın o klasörleri oyunun hiç okumadığı oraya yazdığı anlamına gelir. Aracın
  yolunu düzeltin, klasörleri silin ve aracı yeniden çalıştırın. Open folder
  düğmesi `Overwrite/Root` klasörünü açar.
- "BodySlide's GameDataPath is not the game's Data folder" (ya da
  `OutputDataPath`): bkz. [Araçlar](tools.md#bir-aracın-kendi-ayarları-yine-kendisinindir).

### Son oturumdaki bir çökme

"The game crashed in the last session", son başlatmada bir çökme kaydedicisi bir
günlük yazdığında görünür. Kart, günlükteki istisnayı ve modlarınızın ya da
Overwrite'ın kurduğu DLL'leri ve eklentileri adlandırır. Bunları ipucu sayın:
çökme günlüğünde adı geçen bir mod çoğu zaman çökmenin göründüğü yerdir, nedeni
değil. Günlük modlarınızdan hiçbir dosyayı adlandırmıyorsa kart bunu söyler. Open
the log onu açar, Copy the log (home path hidden) ev klasörünüz gizlenmiş olarak
kopyalar.

### Pencere boyutu

Skyrim SE, Enderal SE ve Fallout 4 için bir Display kartı, profilin oyunu hangi
pencere boyutu ve kipiyle başlattığını gösterir; ne profilin Prefs INI'si ne de
önekinki bir boyut ayarlıyorsa "No window size set for this profile" der. Apply
boyutu ve kipi profilin kopyasına, pencereyi SSE Display Tweaks ayarlıyorsa
Overwrite içindeki INI kopyasına yazar ([usage.md](usage.md#diagnostics)).

## Oyunu hangi Proton çalıştırıyor ve bozuk bir önek

Diagnostics sekmesinde bir Proton kartı vardır: Eidos ile Steam'in oyun için
kullandığı sürüm, Steam'in kendi ayarı ve öneki en son çalıştıran sürüm. Öneki
en son seçilenden daha yeni bir Proton hazırladıysa kart bir öneriye dönüşür,
çünkü daha yeni bir önekte daha eski bir Proton, önekin Wine dosyalarını
sıfırlayabilir ya da başlamayabilir. Steam'de oyunun Özellikler, Uyumluluk
bölümünden en az o kadar yeni bir sürüm seçin ya da öneki yeniden kurun.

Diğer Proton ve önek kartları:

- "Proton comes from the Flatpak Steam install": o Proton çalışma zamanını ve
  steamclient kitaplıklarını Flatpak kum havuzunun içinde tutar, bu yüzden
  ana makineden çalıştırmak başarısız olabilir. Eidos onu `flatpak run` ile
  başlatmaz; bu oyunu kum havuzuna, birleşik görünümün dışına koyardı.
  `~/.steam/root/compatibilitytools.d/` içine bir Proton kurun ve oyun için onu seçin.
- "Eidos cannot tell which Proton runs this game": Steam'in `config.vdf` dosyası
  onun için hiçbir uyumluluk aracı adlandırmıyor ve önek kurulu bir sürüm
  kaydetmiyor. Steam'de (oyunun Özellikler, Uyumluluk bölümü) biri seçilene kadar
  araçlar başlamaz.
- "<game> has no Proton prefix yet": oyunu bir kez Steam üzerinden başlatın, sonra
  F5'e basın. Steam öneki oluşturmuyorsa `steamapps/compatdata` altındaki oyun
  klasörünün yazılabilir olduğunu denetleyin; root'a ait boş bir klasör bunu
  iletisiz engeller.
- "Windows tools cannot find <game>": önekin kurulum yolu kaydı yanlış ya da eksik
  ve xEdit, DynDOLOD, Wrye Bash ve Creation Kit onu okur. Eidos bir sonraki araç
  çalıştırışınızda doğru yolu yazar.
- "The prefix's S: drive points outside the Steam library": `S:`, Eidos'un bir araç
  için gösterdiği kütüphane köküyle de, Steam'in oyun için gösterdiği
  `<library>/steamapps` ile de eşleşmiyor. Kütüphane muhtemelen taşındı ya da önek
  başka bir makineden geldi. Önekte saklanan `S:` göreli her yol, Eidos ya da Steam
  üzerinden bir sonraki başlatma onu yeniden yöneltene kadar hiçbir yere çözülmez.
- "<store> installation: started through its store", bir GOG ya da Epic kopyasının
  kartıdır. Eidos oradaki wrapper olduğunda Run, Heroic ya da Legendary'den oyunu
  başlatmasını ister ve araçlar, Heroic'in kopya için kaydettiği Wine ya da Proton
  sürümüyle çalışır.

File menüsü, oyunun önekinde winecfg, regedit, Wine dosya gezgini ve kaldırıcıyı,
oyunun araçlarını çalıştıran Proton ile çalıştırır
([tools.md](tools.md#wineın-kendi-programları-oyunun-önekinde)). Önekte o programlar
olana kadar satırları soluk kalır.

Proton kartındaki Rebuild prefix düğmesi artık çalışmayan bir önek içindir. Ona iki
kez basın. Eidos hiçbir şey silmez: `steamapps/compatdata/<appid>` klasörünü aynı
klasör içinde `<appid>.eidos-backup-<date>-<time>` olarak yeniden adlandırır ve
oyunu Steam'den bir sonraki başlatışınızda Steam yeni bir önek oluşturur. Öneki
bir oyun, bir araç ya da herhangi bir Wine programı kullanırken bunu reddeder.
Yeni önek oluşana kadar penceredeki Run oyunu Steam'e devredemez ve bunun yerine
başlatma seçeneğini gösterir; bu yüzden Steam'de Play'e basın. Yalnızca eski önekte
duran kayıtlar ve ayarlar yeniden adlandırılan klasörde kalır: ihtiyacınız olanı
geri kopyalayın ve artık istemediğinizde o klasörü kendiniz silin.

## Hata raporuna neyi eklemeli

- Diagnostics sekmesindeki Copy report; Eidos sürümünü, oyunu, Proton sürümünü,
  Steam'in Flatpak olup olmadığını, passthrough durumunu, oturum türünü, dağıtımı,
  çekirdeği, `vm.max_map_count` değerini, açık dosya sınırını, `/dev/fuse` ve
  `fusermount3` dosyalarının orada olup olmadığını, profilin başlatma
  değişkenlerinin ve wrapper programlarının adlarını ve her Health kartını
  kopyalar. Ev klasörünüz `~`, oturum adınız `<user>` olarak görünür.
  `eidos check <instance>` aynı başlığı bir terminalde yazdırır, `--json` onu JSON
  olarak verir.
- Çalıştırmanın günlüğü `<instance>/logs/run-*.log`, Eidos'un çalıştırdığı tam
  komutla başlar. File, Eidos logs, Eidos'un kendi günlüklerinin klasörünü açar:
  `~/.local/share/Colony/Eidos/logs`.
- Çökme kartındaki Copy the log (home path hidden) çökme günlüğünü kopyalar.
- Data sekmesindeki Save tree, birleşik `Data` ağacını her dosyanın kaynağıyla
  kaydeder.

## Bilinen sorunlar

### DLSS ya da frame generation sessizce hiçbir şey yapmıyor

Üç ayrı neden, her biri hiçbir hata iletisi vermeden: başlatma seçeneklerinde
NVAPI'nin açık olmaması, exclusive fullscreen ya da bayatlamış bir Reflex FPS
sınırı. Denetim listesinin tamamı [graphics.md](graphics.md) içinde.

### Run, Flatpak Steam ile hiçbir şey yapmıyor

Bir mağazanın açmadığı bir pencerede Run, `steam -applaunch <appid>` çalıştırarak
Steam'den oyunu başlatmasını ister. Flatpak Steam ana makinenin `PATH`'ine bir
`steam` koymaz; bu yüzden durum satırı hatayla birlikte "Could not start Steam"
der. Bunun yerine Steam'de Play'e basın.

### Başka bir cihazdan gelen bir kayıt görünmüyor

Eidos üzerinden geçmeden önekine ulaşan bir kayıt, örneğin bir Steam Deck'ten
Steam Cloud indirmesi, profilin kayıtları bağlıyken gizli kalır. Hiçbir şey onu
silmez. Saves sekmesi bu kayıtları başlığının altında sayar ve Import onları bir
profil kaydının yerine geçmeden profile kopyalar
([usage.md](usage.md#kayıtlar-profile-aittir)).

### Bir dizini iki farklı yazan bir mod, ikincisinin altındaki her şeyi yitirdi

Düzeltildi. ext4 `meshes/` ile `Meshes/` dizinlerini ayrı tutar; birleşik görünüm
tutmamalıdır ve gerçek modlar ikisini birden getirir - XP32 Maximum Skeleton'ın
animasyonları ve FNIS davranış dosyası büyük harfli olanın, `character assets`
dizini ise ötekinin altındadır.

Çözümleyici her yol bileşeni için tam harf eşleşmesini alıp ona bağlanıyordu:
`meshes/` içine giriyor, yolun geri kalanını orada bulamıyor ve TÜM KATMANI
bırakıyordu. Öteki yazımın altındaki her dosya oyun için görünmezdi - hata yok,
günlük yok, hiçbir tanılamada hiçbir şey yok. Gerçek, 50 katmanlı bir örnekte bu
74 dosya ediyordu.

Eşleşen bir bileşen artık bir karar değil, bir aday; tam harf yazımı yine önce
denenir ve yalnızca geri kalan onun altında başarısız olduğunda tarama, harf
katlamasında eşit kardeşleri arar. Listelemelerde aynı kusur bir dizin
yukarıdaydı; artık katman başına harf katlamasında eşit her dizini okuyorlar.

Yol indeksinde bu hata hiç olmadı, çünkü bulduğu her dizini dolaşır. Yani bir süre
indeks, fallback'in döndüremediği dosyaları döndürdü; oysa hiçbir zaman yanılmaması
gereken yanıt fallback'tir.

### DynDOLOD'un LODGen'i boş bir günlük bırakarak ölüyor

`dotnet10` ile düzeltildi; bkz. [tools.md](tools.md). Her dünyanın
`LODGen_SSE_<world>_log.txt` dosyasında bir sürüm başlığı, bir `.NET Version:`
satırı ve başka hiçbir şey yok, bir iletişim kutusu da yalnızca "failed to
generate object LOD for one or more worlds" diyor. Neden, .NET Framework yerine
Wine'ın Mono'sunun yanıt vermesidir ve ne kadar .NET Framework kurarsanız kurun
bunu düzeltmez: Proton her önek güncellemesinde `mscoree.dll`'i kendi ağacına
giden bir sembolik bağla değiştirir. Sürüm paketi .NET 10 çalışma zamanını getirir;
diğer kurulumlar onu bir kez indirir.

### Wine, bağlamanın harf katlaması yaptığını anlayamıyordu

Düzeltildi.

Wine'ın `get_dir_case_sensitivity` işlevi önce dizine `FS_IOC_GETFLAGS` ile
sorar, sonra CIOPFS'in sunduğu dizinlerde bıraktığı imi arar. İkisi de yoksa
Wine harf DUYARLI varsayar ve yazımı bayt bayt eşleşmeyen her arama, harf
duyarsız bir eşleşme bulmak için TÜM dizini okumaya geri düşer. Bethesda
oyunları, dosya `ccBGSSSE001-Fish.bsa` iken `data/ccbgssse001-fish.bsa` ister,
dolayısıyla bu neredeyse her varlıkta tetiklendi: sekiz saniyede 4471 im
yoklaması ve 2236 tam dizin yeniden okuması, doksan saniyede de `Data` için
195796 sayım. Skyrim SE ana menüsüne hiç ulaşamadı: daemon bir çekirdeğin
%92'sini yakarken 240 MB yerleşik bellekte öylece durdu.

Eidos en baştan beri `resolve_read` içinde harf katlaması yapıyordu. Bütün
bedel, bunu hiç söylememesiydi. `lookup` artık `.ciopfs` yanıtı veriyor;
`readdir` onu hâlâ listelemiyor. `ioctl` ayrıca dizinler için
`FS_IOC_GETFLAGS`'a casefold bayrağıyla yanıt verir; güncel Wine imi aramadan
önce bunu sorar.

İki şey bunu yalnızca yavaş değil, öldürücü kıldı. Bedel dizin boyutuyla
ölçeklenir, bu yüzden Anniversary içeriğini kurmak (`Data` 37 dosyadan 177'ye)
dengeyi bozdu. Bir de `opendir` birleştirilmiş listelemeyi hevesle kuruyordu;
Wine bir dizini yalnızca içindeki o imi `stat` etmek için açtığında bu tam bir
israftır: anlık görüntü artık ilk `readdir` üzerinde alınıyor.

Sonrasında: ana menü, 2,1 GB yerleşik bellek, daemon %0 CPU.

Onu bulan `EIDOS_FUSE_TRACE=opendir` oldu ve ürünle birlikte geliyor. İşlem
sayaçları kaç tane olduğunu söyler; tek bir dizinin 195796 kez sayılması bir
toplam içinde görünmez.

### Oyun `plugins.txt` dosyasını boş olarak yeniden yazdı

Büyük olasılıkla yukarıdaki harf katlamasıyla aynı şeydi: makul bir sürede
sayamadığı bir `Data`, bu yüzden orada hiçbir şey olmadığı sonucuna varıp bunu
kaydetti. Kanıtlanmadı ve yeniden bakmaya değer. Her hâlükârda yakalama koruması
(etkin kümeyi büsbütün temizleyen bir yakalama, boyutu ne olursa olsun geri
çevrilir) artık profile zarar veremeyeceği anlamına geliyor.

### `FOPEN_KEEP_CACHE` kapalı

Düzeltildi. Hiç mod kurulu değilken, ana menüden saniyeler sonra Skyrim SE'yi bir
null başvurusuyla, her seferinde çökertiyordu; çekirdek tarafındaki diğer üç
önbellek tek tek elenerek ayıklandı ve yalnızca bu önemliydi. Bu eleme
`FUSE_PASSTHROUGH` açıkken yapıldı ve çekirdek, `FOPEN_KEEP_CACHE` taşıyan bir
passthrough açmasını EIO ile başarısız kılar; yani bayrak açıkken her passthrough
açması başarısız oluyordu. Oyunu çökerten belki de bu başarısız açmalardı ve
bayrağın kendisi, passthrough kapalıyken, hiç sınanmadı. Passthrough yanıtları
artık onu taşımıyor. Onu yitirmek o sırada bedelsiz ölçülmüştü, ama passthrough
açıkken daemon *sıfır* okuma sunar (`EIDOS_FUSE_STATS` tam bir yükleme için
`read 0` bildirdi), dolayısıyla passthrough öntanımlı olarak kapalıyken gerçek
bedel ölçülmedi. Tek başına sınamak için `EIDOS_FUSE_KEEP_CACHE=1` ile yeniden açın.

### FUSE passthrough, oyunun hiçbir mod içeriğini yüklemesine engel oluyor

Daemon artık en olası nedenden kaçınıyor (aşağıda), ama bu henüz bir oyunda
yeniden ölçülmedi, bu yüzden passthrough öntanımlı olarak kapalı kalıyor;
`EIDOS_FUSE_PASSTHROUGH=1` onu açar. Bu değişiklikten önce, passthrough açıkken
Skyrim SE kendi dosyalarının 152'sini `STATUS_ACCESS_VIOLATION` ile açamıyordu;
yani hiçbir mod içeriği yüklenmiyordu, sessizce. Ölçüm ve daemon'un kendi
günlüklerinin neden temiz bir çalışma gösterdiği, yukarıdaki "Passthrough neden
öntanımlı olarak kapalı" bölümünde.

Çekirdek her inode'a tek bir dayanak dosya bağlar (`fs/fuse/iomode.c`). Zaten
bir dayanak dosyası olan bir inode için ikinci bir dayanak kimliği getiren açma
EIO ile başarısız olur; dayanak dosyası olan bir inode'un normal açması
(`FOPEN_PASSTHROUGH` olmadan) da öyle. `FOPEN_KEEP_CACHE` taşıyan bir
passthrough açması da EIO ile başarısız olur. Daemon eskiden her açmada yeni bir
kimlik kaydediyordu, bu yüzden oyunun zaten açık tuttuğu bir dosyanın ikinci
açması başarısız oluyordu. Wine, EIO'yu `STATUS_ACCESS_VIOLATION` olarak
bildirir; 152 başarısız açmanın ardındaki hata budur. Bu kural çekirdeğin kaynak
kodundan okundu ve o 152 başarısızlıkla örtüşüyor, bu da onu en olası neden
yapıyor.

Daemon artık inode başına tek bir dayanak dosya kaydediyor; bu dosyayı inode'un
tüm passthrough açmaları paylaşıyor ve bir passthrough açmasında
`FOPEN_KEEP_CACHE` hiçbir zaman ayarlanmıyor. Passthrough yalnızca oyun ve mod
katmanlarındaki, 1 MiB ya da daha büyük olan veya `.dll`, `.exe` ya da `.asi`
ile biten dosyaların salt okunur açmalarını kapsar. O dayanak dosya
kaydedildikten sonra dosyanın her salt okunur açması onu kullanır; dosyanın
normal açmaları sürdükçe yeni okuyucular da normal açma alır. Overwrite'taki bir
dosya hiçbir zaman passthrough'dan geçmez ve eklenti zaman damgalarını yansıtan
bir bağlama onu hiç kullanmaz. Passthrough okuyucularının tuttuğu bir dosyayı
yazmak için yapılan bir açma, herhangi bir copy-up'tan önce ETXTBSY alır
(Wine'da: `STATUS_SHARING_VIOLATION`); böylece `O_TRUNC` ile yapılan bir açma,
Overwrite'ta modun dosyasını gizleyen boş bir kopya bırakamaz.

### Mod Manager Download düğmesi hiçbir şey yapmıyor

Preferences, Nexus, Downloads, `nxm://` bağlantılarını hangi programın aldığını
gösterir. Bu program Eidos değilse, yoksa ya da hiçbiri ayarlanmamışsa bir Health
kartı görünür. Register düğmesi `eidos-nxm.desktop` dosyasını yazar ve onu
`x-scheme-handler/nxm` için varsayılan işleyici yapar. Terminalde aynı işi
`eidos nxm --register` görür.

Durum zaten Eidos diyorsa ve düğme yine de bir şey yapmıyorsa tarayıcı önceki bir
seçimi kullanıyor olabilir. Tarayıcının ayarlarından nxm bağlantıları için kayıtlı
seçimi silin, böylece yeniden sorar. Flatpak tarayıcı ana makinenin programlarını
başlatamaz ve bağlantıyı, ana makinede kayıtlı işleyiciyi okuyan masaüstü portalı
üzerinden iletir.
