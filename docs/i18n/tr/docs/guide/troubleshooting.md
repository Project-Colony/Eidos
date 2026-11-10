<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=55e68d8b6ee830b2d336ce1e96db32155a304c57 -->

# Sorun giderme ve tanılama

Oyunun, dosya sisteminin katılmadığı bir şey gördüğü gün için gereken her şey:
ortam anahtarları, işlem sayaçlarının nasıl okunacağı, bilinen sorunlar ve
geçmişleri, bir de passthrough hikâyesi.

### VFS'i tanılamak

Oyun, dosya sisteminin katılmadığı bir şey gördüğünde diye iki ortam değişkeni
var:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

troubleshooting.md içinde anlatılan çökmeyi bulan, ayrıntılı biçim oldu:
dördünü birden kapatmak "önbellekleme mi?" sorusunu yanıtlar, yalnızca adlarını
vermek "hangisi" sorusunu yanıtlar. Sayaçlar diğer yarısını yanıtlar - `read 0`
gösteren bir yükleme, `FUSE_PASSTHROUGH`'un her baytı çekirdekte sunduğu bir
yüklemedir, yani okuma yolunda ayarlamak üzere olduğunuz her şey zaten
bedavadır.

## Bir birleşimi elle bağlamak

Çakışmada ilk `--layer` kazanır; sonuncusu dokunulmamış oyun verinizdir. Bağlama
yalnızca `/dev/fuse` ve `fusermount3` ister (overlayfs yok, Wine yok):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Yazmalar `--overwrite <dir>` içine düşer (verilmediğinde geçici bir dizin),
böylece burada bile katmanların kendisi dokunulmamış kalır.


#### Passthrough neden öntanımlı olarak kapalı

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

Bu hata daemon'un içinden görünmüyordu; bulunmasını pahalı kılan da buydu:
daemon'un kendi `open`'ı her seferinde başarılı oluyordu ve çekirdek bir dayanak
dosyasını hiçbir zaman geri çevirmiyordu (`EIDOS_FUSE_TRACE=open` ile, başarısız
olan tam bir oturum boyunca doğrulandı: sıfır `open FAILED`, sıfır
`passthrough refused`). Hata, daemon `opened_passthrough` yanıtını verdikten
sonra üretiliyordu, dolayısıyla daemon tarafındaki hiçbir günlükleme onu
göremiyordu. Uzantıya özgü de değildi - arşivleri de eklentileri de, yani oyunun
çalışması boyunca açık tuttuğu dosyaları vuruyordu.

`EIDOS_FUSE_PASSTHROUGH=1` onu geri açar; ne kazandırdığını ölçmek ya da düzeneği
yeniden sınamak için. Başlatıcıdaki ve Diagnostics sekmesindeki yetenek uyarıları
yalnızca siz istediğinizde belirir.

Oyunun kendisini Eidos üzerinden başlatmak için Steam başlatma seçeneğini şuna
ayarlayın:

```
eidos play skyrimse -- %command%
```

Başına bir `WINEDLLOVERRIDES` eklemek gerekmez. Etkin bir mod `d3dcompiler_47`
içe aktardığında Eidos kendi paketlediği kopyayı öneke yerleştirir, onu native
olmaya zorlar ve bunu bir modun getirdiği DLL geçersiz kılmalarıyla birleştirir
(ENB/ReShade/`.asi` yükleyicileri).


### Katman indeksi gerçekten kullanılıyor mu?

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

## Kapanmayan bir oyun ya da araç

Bir oyun ya da araç çalışırken Run düğmesi **Stop** olur, kilit kartında da bir
Stop düğmesi vardır. Ona iki kez basın. İkinci basış, çalıştırmanın başlattığı
her programı sonlandırır: önce SIGTERM, 10 saniye sonra da geriye kalanlar için
SIGKILL. Oyunun kaydetmediği her şey kaybolur. Ardından Eidos her çıkıştan sonra
yaptığını yapar: INI ve eklenti değişiklikleri profile geri döner, araç çıktısı
kaydedilir ve modların bağlantısı kaldırılır. **Unlock** farklıdır: pencereyi
geri verir ve programı çalışır durumda bırakır. Bir terminalden, `eidos`
sürecine `kill <pid>` (pid kilit kartında yazar) Stop ile aynı işi yapar.

## Oyunu hangi Proton çalıştırıyor, ve bozuk bir önek

Diagnostics sekmesinde bir **Proton** kartı vardır: Eidos ile Steam'in oyun için
kullandığı sürüm, Steam'in kendi ayarı ve öneki en son çalıştıran sürüm. Öneki
en son seçilenden daha yeni bir Proton hazırladıysa kart bir öneriye dönüşür,
çünkü daha yeni bir önekte daha eski bir Proton, önekin Wine dosyalarını
sıfırlayabilir ya da başlamayabilir. Steam'de oyunun Özellikler, Uyumluluk
bölümünden en az o kadar yeni bir sürüm seçin.

O karttaki **Rebuild prefix**, artık çalışmayan bir önek içindir. Ona iki kez
basın. Eidos hiçbir şey silmez: `steamapps/compatdata/<appid>` klasörünü aynı
klasör içinde `<appid>.eidos-backup-<date>-<time>` olarak yeniden adlandırır ve
oyunu Steam'den bir sonraki başlatışınızda Steam yeni bir önek oluşturur. Öneki
bir oyun, bir araç ya da herhangi bir Wine programı kullanırken bunu reddeder.
Yalnızca eski önekte duran kayıtlar ve ayarlar yeniden adlandırılan klasörde
kalır: ihtiyacınız olanı geri kopyalayın ve artık istemediğinizde o klasörü
kendiniz silin.

## Bilinen sorunlar

### DLSS ya da frame generation sessizce hiçbir şey yapmıyor

Üç ayrı neden, her biri hiçbir hata iletisi vermeden: başlatma seçeneklerinde
NVAPI'nin açık olmaması, exclusive fullscreen ya da bayatlamış bir Reflex FPS
sınırı. Denetim listesinin tamamı [graphics.tr.md](graphics.md) içinde.

**Bir dizini iki farklı yazan bir mod, ikincisinin altındaki her şeyi yitirdi.**
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

Biçimi açısından bilmeye değer: yol indeksinde bu hata hiç olmadı, çünkü bulduğu
her dizini dolaşır. Sessizce, fallback'in döndüremediği dosyaları döndürüyordu;
bu ters yönde bir durum - hiçbir zaman yanılmaması gereken yanıt, fallback'tir.

**DynDOLOD'un LODGen'i boş bir günlük bırakarak ölüyor.** `dotnet10` ile
düzeltildi; bkz. [tools.tr.md](tools.md). Belirti apaçıktır: her dünya için
bir sürüm başlığı, bir `.NET Version:` satırı ve başka hiçbir şey içeren
`LODGen_SSE_<world>_log.txt`, bir de yalnızca "failed to generate object LOD for
one or more worlds" diyen bir iletişim kutusu. Neden, .NET Framework yerine
Wine'ın Mono'sunun yanıt vermesidir ve ne kadar .NET Framework kurarsanız kurun
bunu düzeltmez - Proton her önek güncellemesinde `mscoree.dll`'i kendi ağacına
giden bir sembolik bağla değiştirir.

**Wine, bağlamanın harf katlaması yaptığını anlayamıyordu.** Düzeltildi ve
önemli olan buydu.

Wine'ın `get_dir_case_sensitivity` işlevi önce dizine `FS_IOC_GETFLAGS` ile
sorar, sonra CIOPFS'in sunduğu dizinlerde bıraktığı imi arar. İkisi de yoksa
Wine harf DUYARLI varsayar ve yazımı bayt bayt eşleşmeyen her arama, harf
duyarsız bir eşleşme bulmak için TÜM dizini okumaya geri düşer. Bethesda
oyunları, dosya `ccBGSSSE001-Fish.bsa` iken `data/ccbgssse001-fish.bsa` ister,
dolayısıyla bu neredeyse her varlıkta tetiklendi: sekiz saniyede 4471 im
yoklaması ve 2236 tam dizin yeniden okuması, doksan saniyede de `Data` için
195796 sayım. Skyrim SE ana menüsüne hiç ulaşamadı - daemon bir çekirdeğin
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
israftır - anlık görüntü artık ilk `readdir` üzerinde alınıyor.

Sonrasında: ana menü, 2,1 GB yerleşik bellek, daemon %0 CPU.

Onu bulan `EIDOS_FUSE_TRACE=opendir` oldu ve ürünle birlikte geliyor. İşlem
sayaçları kaç tane olduğunu söyler; tek bir dizinin 195796 kez sayılması bir
toplam içinde görünmezdir.

**Oyunun `plugins.txt` dosyasını boş olarak yeniden yazması** büyük olasılıkla
aynı şeydi - makul bir sürede sayamadığı bir `Data`, bu yüzden orada hiçbir şey
olmadığı sonucuna varıp bunu kaydetti. Kanıtlanmadı ve yeniden bakmaya değer.
Her hâlükârda yakalama koruması (etkin kümeyi büsbütün temizleyen bir yakalama,
boyutu ne olursa olsun geri çevrilir) artık profile zarar veremeyeceği anlamına
geliyor.

**`FOPEN_KEEP_CACHE` kapalı.** Düzeltildi ve nedenini bilmeye değer. Hiç mod
kurulu değilken, ana menüden saniyeler sonra Skyrim SE'yi bir null başvurusuyla,
her seferinde çökertiyordu; çekirdek tarafındaki diğer üç önbellek tek tek
elenerek ayıklandı ve yalnızca bu önemliydi. Bu eleme `FUSE_PASSTHROUGH` açıkken
yapıldı ve çekirdek, `FOPEN_KEEP_CACHE` taşıyan bir passthrough açmasını EIO ile
başarısız kılar; yani bayrak açıkken her passthrough açması başarısız oluyordu.
Oyunu çökerten belki de bu başarısız açmalardı ve bayrağın kendisi, passthrough
kapalıyken, hiç sınanmadı. Passthrough yanıtları artık onu taşımıyor. Onu
yitirmek o sırada bedelsiz ölçülmüştü, ama passthrough açıkken daemon *sıfır*
okuma sunar (`EIDOS_FUSE_STATS` tam bir yükleme için `read 0` bildirdi),
dolayısıyla passthrough öntanımlı olarak kapalıyken (aşağıda) gerçek bedel
ölçülmedi. Tek başına sınamak için `EIDOS_FUSE_KEEP_CACHE=1` ile yeniden açın.

### FUSE passthrough, oyunun hiçbir mod içeriğini yüklemesine engel oluyor

Daemon artık en olası nedenden kaçınıyor (aşağıda), ama bu henüz bir oyunda
yeniden ölçülmedi, bu yüzden passthrough öntanımlı olarak kapalı kalıyor;
`EIDOS_FUSE_PASSTHROUGH=1` onu açar. Bu değişiklikten önce, passthrough açıkken
Skyrim SE, çekirdek 7.1.4 üzerinde kendi dosyalarının 152'sini (75 `.bsa`, 65
`.esl`, 10 `.esm`, 2 `.esp`) `STATUS_ACCESS_VIOLATION` ile açamıyordu;
kapalıyken bu sayı 0'dı - yani hiçbir mod içeriği yüklenmiyordu, sessizce.
Çekirdek hatayı, daemon `opened_passthrough` yanıtını verdikten sonra
üretiyordu, bu yüzden daemon'un kendi günlükleri temiz bir çalışma gösteriyordu
(sıfır başarısız açma, sıfır geri çevrilmiş dayanak dosya).

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

Tercihler > İndirmeler, `nxm://` bağlantılarını hangi programın aldığını gösterir. Bu program Eidos değilse, yoksa ya da hiçbiri ayarlanmamışsa Sağlık bir kart ekler. Kaydet düğmesi `eidos-nxm.desktop` dosyasını yazar ve onu `x-scheme-handler/nxm` için varsayılan işleyici yapar. Terminalde aynı işi `eidos nxm --register` görür.

Durum zaten Eidos diyorsa ve düğme yine de bir şey yapmıyorsa tarayıcı önceki bir seçimi kullanıyor olabilir. Tarayıcının ayarlarından nxm bağlantıları için kayıtlı seçimi silin, böylece yeniden sorar. Flatpak tarayıcı ana makinenin programlarını başlatamaz ve bağlantıyı, ana makinede kayıtlı işleyiciyi okuyan masaüstü portalı üzerinden iletir.
