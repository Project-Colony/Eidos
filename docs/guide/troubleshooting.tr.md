<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

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
daemon'u büsbütün atlar. Burada doğruluğa mal olan bir verim kazancıdır. Skyrim
SE 1.6.1170, proton-cachyos 11.0, çekirdek 7.1.4, aynı 82 eklentilik yükleme
sırası üzerinde A/B ölçüldü; tek değişken, ikili dosyanın yeteneği taşıyıp
taşımadığıydı:

| passthrough | `STATUS_ACCESS_VIOLATION` ile `NtCreateFile` başarısızlıkları |
|-------------|--------------------------------------------------------------|
| açık        | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`              |
| kapalı      | 0                                                            |

Açıkken oyun kendi arşivlerinin ya da eklentilerinin hiçbirini açmaz; bu, oyun
içinde sadece orada olmayan modlar olarak görünür - hata yok, günlük satırı yok.
Kapalıyken aynı yükleme sırası, eklentileri, arşivleri ve Papyrus betikleri
çalışır durumdayken oyuna ulaşır.

Bu hata daemon'un içinden görünmez; bulunmasını pahalı kılan da budur: kendi
`open`'ımız her seferinde başarılı olur ve çekirdek bir dayanak dosyasını hiçbir
zaman geri çevirmez (`EIDOS_FUSE_TRACE=open` ile, başarısız olan tam bir oturum
boyunca doğrulandı: sıfır `open FAILED`, sıfır `passthrough refused`). Hata,
daemon `opened_passthrough` yanıtını verdikten sonra üretilir, dolayısıyla
daemon tarafındaki hiçbir günlükleme onu göremez. Uzantıya özgü de değildir -
arşivleri de eklentileri de, yani oyunun çalışması boyunca açık tuttuğu
dosyaları vurur.

`EIDOS_FUSE_PASSTHROUGH=1` onu geri açar; ne kazandırdığını ölçmek ya da düzeneği
yeniden sınamak için. Başlatıcıdaki ve Diagnostics sekmesindeki yetenek uyarıları
yalnızca siz istediğinizde belirir.

Oyunun kendisini Eidos üzerinden başlatmak için Steam başlatma seçeneğini şuna
ayarlayın:

```
eidos play skyrimse -- %command%
```

Proton'un shader derlemesi için native d3dcompiler'a ihtiyacı varsa başına
`WINEDLLOVERRIDES="d3dcompiler_47=n"` ekleyin; Eidos bunu bir modun getirdiği
DLL geçersiz kılmalarıyla birleştirir (ENB/ReShade/`.asi` yükleyicileri).


### Katman indeksi gerçekten kullanılıyor mu?

İndeks ya hep ya hiçtir ve sessizce kurulur: `LayerStack::new` ya salt okunur
katmanların eksiksiz bir haritasını alır ya da `None`; ardından her sorgu onları
tam eskisi gibi dolaşır. Oturum günlüğünde ikisini ayırt eden hiçbir şey yoktur,
dolayısıyla sessizce geri düşmüş bir yığın, çalışan biriyle birebir aynı görünür
- üstelik eski bedeli öderken.

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

## Bilinen sorunlar

### DLSS ya da frame generation sessizce hiçbir şey yapmıyor

Üç ayrı neden, her biri hiçbir hata iletisi vermeden: başlatma seçeneklerinde
NVAPI'nin açık olmaması, exclusive fullscreen ya da bayatlamış bir Reflex FPS
sınırı. Denetim listesinin tamamı [graphics.tr.md](graphics.tr.md) içinde.

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
düzeltildi; bkz. [tools.tr.md](tools.tr.md). Belirti apaçıktır: her dünya için
bir sürüm başlığı, bir `.NET Version:` satırı ve başka hiçbir şey içeren
`LODGen_SSE_<world>_log.txt`, bir de yalnızca "failed to generate object LOD for
one or more worlds" diyen bir iletişim kutusu. Neden, .NET Framework yerine
Wine'ın Mono'sunun yanıt vermesidir ve ne kadar .NET Framework kurarsanız kurun
bunu düzeltmez - Proton her önek güncellemesinde `mscoree.dll`'i kendi ağacına
giden bir sembolik bağla değiştirir.

**Wine, bağlamanın harf katlaması yaptığını anlayamıyordu.** Düzeltildi ve
önemli olan buydu.

"bu dosya sistemi harf duyarsız mı" diye soran bir API yok, bu yüzden Wine'ın
`get_dir_case_sensitivity` işlevi, CIOPFS'in sunduğu dizinlerde bıraktığı imi
arar. İm yoksa Wine harf DUYARLI varsayar ve yazımı bayt bayt eşleşmeyen her
arama, harf duyarsız bir eşleşme bulmak için TÜM dizini okumaya geri düşer.
Bethesda oyunları, dosya `ccBGSSSE001-Fish.bsa` iken `data/ccbgssse001-fish.bsa`
ister, dolayısıyla bu neredeyse her varlıkta tetiklendi: sekiz saniyede 4471 im
yoklaması ve 2236 tam dizin yeniden okuması, doksan saniyede de `Data` için
195796 sayım. Skyrim SE ana menüsüne hiç ulaşamadı - daemon bir çekirdeğin
%92'sini yakarken 240 MB yerleşik bellekte öylece durdu.

Eidos en baştan beri `resolve_read` içinde harf katlaması yapıyordu. Bütün bedel,
bunu hiç söylememesiydi. `lookup` artık `.ciopfs` yanıtı veriyor; `readdir` onu
hâlâ listelemiyor.

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
elenerek ayıklandı ve yalnızca bu önemliydi. Onu yitirmek o sırada bedelsiz
ölçülmüştü, ama o ölçüm `FUSE_PASSTHROUGH` etkinken alınmıştı; orada daemon
*sıfır* okuma sunar (`EIDOS_FUSE_STATS` tam bir yükleme için `read 0` bildirdi)
ve çekirdek o sayfaları zaten dayanak dosyaya karşı önbelleğe alıyordu.
Passthrough artık öntanımlı olarak kapalı (aşağıda), dolayısıyla o savın
geçerliliği kalmadı ve gerçek bedel ölçülmedi - çökme, her durumda onu kapalı
bırakmak için yeterli bir gerekçe. İncelemek için `EIDOS_FUSE_KEEP_CACHE=1` ile
yeniden açın; iki bayrak artık birbirine dolanmış değil, bu yüzden tek başına
sınanabiliyor.

### FUSE passthrough, oyunun hiçbir mod içeriğini yüklemesine engel oluyor

Kapatılarak düzeltildi; `EIDOS_FUSE_PASSTHROUGH=1` onu geri getirir. Passthrough
açıkken Skyrim SE, çekirdek 7.1.4 üzerinde kendi dosyalarının 152'sini (75
`.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`) `STATUS_ACCESS_VIOLATION` ile açamıyor;
kapalıyken bu sayı 0 - yani hiçbir mod içeriği yüklenmiyor, sessizce. Çekirdek
hatayı, daemon `opened_passthrough` yanıtını verdikten sonra üretir, bu yüzden
daemon'un kendi günlükleri temiz bir çalışma gösterir (sıfır başarısız açma,
sıfır geri çevrilmiş dayanak dosya). Çekirdek yolundaki kök neden saptanmadı;
anahtar, yeniden sınanabilsin diye ve image-mapping'in buna ihtiyaç duyduğu
ortaya çıkarsa passthrough yalnızca DLL'lere daraltılabilsin diye tutuluyor.
