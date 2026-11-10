<!-- eidos-i18n: source=docs/guide/usage.md sha=15b7f6e81560a7045c8b62dd59dde35c8e45d32c -->

# Eidos kullanımı

Eidos'un nasıl kullanılacağı: örnekler ve oyun kopyaları, komut satırı, test
çalıştırmaları, pencere, Nexus koleksiyonları ve güncellemeler, bir örneği başka
bir makineye taşımak, araçlar ve eklentiler. Sorunlar için bkz.
[troubleshooting.md](troubleshooting.md); kaynaktan derleme ve depo düzeni için
bkz. [../internals/contributing.md](../../../../internals/contributing.md).

## Örnekler: global ve taşınabilir

Bir örnek alan komutlar bir oyun kimliğini ya da taşınabilir bir örneğin klasörünü
kabul eder. `skyrimse` global örneği adlandırır; Eidos onu
`~/.local/share/Colony/Eidos/instances/skyrimse` içinde tutar. Taşınabilir bir
örnek, istediğiniz yerde (ikinci bir disk, bir oyun bölümü) kendi kendine yeten
bir klasördür ve MO2'nin taşınabilir örnekleri gibi onu taşıyabilirsiniz:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # orada taşınabilir bir örnek oluştur
eidos install /mnt/games/EidosSkyrim mod.7z  # her komut klasörü kabul eder
eidos play /mnt/games/EidosSkyrim -- %command%
```

Klasörün `eidos-instance.ini` dosyası oyunu adlandırır, yani klasörün başka bir
şeye ihtiyacı yoktur. Ortamdaki `EIDOS_INSTANCE=<folder>` bir oyun kimliğini o
klasöre yönlendirir; örneğin bir Steam başlatma seçeneğinde. Oluşturduğunuz ya da
açtığınız taşınabilir örnekler `~/.config/Colony/Eidos/instances.ini` içinde
hatırlanır, en son kullanılan başta: pencerenin karşılama ekranı onları listeler,
Steam başlatması en son oynadığınızın üzerine iner ve `nxm://` işleyicisi onun
içine indirir.

Taşınabilir bir klasörü elle taşımak, eski konuma mutlak yollarla kaydettiğiniz
araç girdileri dışında her şeyi korur. `eidos pack` ve `eidos unpack`
([aşağıda](#bir-örneği-başka-bir-makineye-taşımak)) onları düzeltir; düz bir `mv`
düzeltmez. `~/.local/share/Colony/Eidos/runtimes/` içindeki çalışma zamanı önbelleği
makinedeki her örnek tarafından paylaşılır; bu yüzden 78 MB'lık bir .NET host'u
her birine kopyalanmaz.

Bir yer reddedilir: bir oyunun kurulum klasörünün içi, MO2 kullanıcılarının
örneklerini sık sık koyduğu yer. O ağaç Steam'e aittir ve bir güncelleme, bir
"verify integrity" ya da bir kaldırma onu kurulumunuzla birlikte yeniden yazabilir
ya da silebilir. Eidos ayrıca oyun kökünün üzerine bağlar, yani oradaki bir örnek
kendi bağlama hedefinin içinde otururdu. Sihirbaz, `eidos init`, `eidos play` ve
`eidos shell` onu reddeder. Klasörü bunun yerine oyunun yanına koyun; aynı
diskteki bir kardeş klasör aynı kolaylığı verir.

### Eidos dosyalarını nerede tutar

Eidos kendi dosyalarını `Colony/Eidos` altında tutar; bu, Colony ailesindeki her
programın kullandığı düzendir: seçtikleriniz için `~/.config/Colony/Eidos/`
(tercihler, Nexus oturumunuz, örnek listeniz, yazdığınız oyun ve eklenti
tanımları), global örnekler (`instances/`), indirilen çalışma zamanları ve
oturum günlükleri (`logs/`) için `~/.local/share/Colony/Eidos/`, eksik
olduklarında yeniden indirilen LOOT ana listeleri için de
`~/.cache/Colony/Eidos/`.

Eski bir Eidos'un düzeninde dosyalar bulduğunda (`~/.config/eidos/`,
`~/.local/share/eidos/` ve `~/.local/state/`), ilk başlatma onları karşıya
kopyalar ve bunu günlükte söyler. Global bir örnek bayt bayt kopyalanmak yerine
sabit bağlantılarla taşınır; böylece modları fazladan yer kaplamaz ve araç
girdileri yeni klasörü gösterecek şekilde güncellenir. Eski dizinlerde hiçbir şey
değiştirilmez ya da silinmez; yeni düzen çalışınca onları kendiniz kaldırın. Kopya
yapılamazsa (dolu ya da salt okunur bir disk, başka bir sürücüdeki eski bir örnek
klasörü, hâlâ oradan çalışan bir oyun), Eidos eski konumu kullanmaya devam eder ve
bir sonraki başlatmada yeniden dener. Hâlâ eski örnek klasörünü gösteren bir Steam
başlatma seçeneği, taşınmış olanı açar.

Sabit bağlantılar yalnızca başta yer kazandırır. Sonradan kaldırdığınız ya da
yeniden kurduğunuz bir mod veya indirme, eski örnek klasörü üzerinden eski
kopyasını diskte tutar; `~/.local/share/eidos/<game>/` klasörünü silmek o yeri
geri verir.

Taşınabilir bir örnek asla taşınmaz: onu koyduğunuz yerde kalır.

## Komut satırı

```sh
eidos games                                # burada kurulu desteklenen oyunlar (MO2'nin listesi gibi)
eidos init skyrimse [folder]               # bir örnek oluştur; klasör verirseniz taşınabilir olur
eidos init skyrimse --game-path <folder>   # birkaç kopya kuruluyken <folder> içindeki kopyayı kullan
eidos install skyrimse mod.7z [name]       # bir arşivi kur (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # modun üzerine yeniden kur (--merge dosyalarının üzerine kurar)
eidos import skyrimse <mo2-profile>        # bir MO2 profilinin mod sırasını ve eklenti durumunu devral
eidos sort skyrimse [--dry-run]            # yükleme sırasını LOOT ile sırala (--update-masterlist önce onu yeniler)
eidos play skyrimse                        # neyin bağlanacağını göster
eidos play skyrimse -- <command>           # <command>'ı modlar oyunun üzerine bağlanmış halde çalıştır
eidos shell skyrimse                       # birleşik görünümde bir kabuk; yazılanlar Overwrite'a gider
eidos check skyrimse [--json]              # pencere olmadan Health denetimleri
eidos sessions skyrimse                    # kaydedilmiş oyun ve araç çalıştırmaları, en yeni başta
eidos sessions skyrimse diff [<id>]        # çıkış kodu 0 ile biten son çalıştırmadan beri neyin değiştiği
eidos stock-game skyrimse                  # örneğin kullandığı oyun dosyaları; bir kopyayı dondur ya da değiştir
eidos tool skyrimse                        # araçları listele (add, rm ve run: aşağıdaki Araçlar'a bakın)
eidos tool skyrimse run --exe <path>       # bir exe dosyasını birleşik görünüm üzerinden bir kez çalıştır
eidos tool skyrimse import-mo2 [--write]   # bir ModOrganizer.ini içindeki çalıştırılabilir dosyalar; --write ekler
eidos prereqs skyrimse [--install]         # araçların neye ihtiyaç duyduğu; --install eksik olanı getirir
eidos collection skyrimse <link>           # bir Nexus koleksiyonunu kur (--dry-run, --no-optional)
eidos nexus status                         # kayıtlı Nexus oturumunu denetle
eidos nexus update skyrimse [--apply]      # mod güncellemelerini denetle; --apply onları kurar
eidos nxm <link>                           # bir Mod Manager Download bağlantısını indir
eidos nxm --register                       # sitenin Mod Manager Download düğmesinin Eidos'u açmasını sağla
eidos export skyrimse -o mods.csv --active # MO2'nin CSV biçimi; --active yalnızca etkin modları listeler
eidos pack skyrimse backup.eidos           # bütün örnek tek bir dosyada
eidos unpack backup.eidos [folder]         # klasör olmadan global bir örnek merkezî yerine döner
eidos --version                            # sürümü yazdır
```

Argümansız `eidos` komutların listesini yazdırır. Çoğu komut bir argüman
eksikken kendi seçeneklerini yazdırır (`eidos init` ve `eidos import` bunun yerine
tam listeyi yazdırır) ve `eidos tool --help` araç alt komutlarını listeler.
`eidos pack` `--dry-run`, `--level`, `--no-downloads`, `--no-recovery` ve
`--force` alır, `eidos unpack` ise `--info` ve `--force` alır; ikisi de
[aşağıda](#bir-örneği-başka-bir-makineye-taşımak) anlatılır. `eidos stock-game`,
[Oyun dosyaları](#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya) altında
anlatılır.

### Bir oyunu birleşik görünüm üzerinden çalıştırmak

`eidos play`, örneğin modlarını özel bir ad alanının içinde oyunun kendi `Data`
dizininin üzerine bağlar, sonra komutu o görünüm üzerinden çalıştırır. Oyunun kendi
dosyaları önce bind-stash ile kenara alınır, böylece daemon onları yine
değiştirilmemiş okur. Yazmalar (kayıtlar, yeniden üretilen yapılandırmalar)
örneğin `overwrite/` katmanına iner; oyun kurulumu ve her mod klasörü bayt bayt
olduğu gibi kalır.

Eidos root gerektirmez. Özel bir kullanıcı ve bağlama ad alanında bağlar; setuid
yardımcısı, sistem hizmeti ve verilecek bir yetki yoktur. `eidos` ikili dosyasındaki
isteğe bağlı bir yetenek FUSE passthrough'a izin verir; `EIDOS_FUSE_PASSTHROUGH=1`
ayarlanmadıkça o kapalı kalır, bkz. [install.md](install.md#seçmeli-fuse-passthrough).
Yetenekle Eidos, bir kullanıcı ad alanı yerine düz bir bağlama ad alanı kullanır ve
modlar ikisinde de aynı şekilde yerleşir.

### Birleşik görünümde bir kabuk

`eidos shell <instance>`, `$SHELL` kabuğunuzu (yoksa `/bin/sh`) oyunun göreceği haliyle oyun klasöründe başlatır: etkin her mod `Data` üzerine bağlanır, Root modları oyunun kök dizinine bağlanır ve oraya yazılan her şey Overwrite'a düşer. `eidos shell <instance> -- <command>` bunun yerine tek bir komut çalıştırır, örneğin `eidos shell skyrimse -- ls Data/meshes` ya da NifSkope gibi yerel bir araç. Çalıştırma, bir oyun oturumu gibi örnek kilidini tutar; bu yüzden siz çıkana kadar pencere değişiklik kaydedemez ve `eidos sessions` içinde `shell` adıyla görünür, hiçbir zaman oyun başlatması olarak değil. Bir komut sırasında Ctrl+C çalıştırmayı Stop gibi durdurur: Eidos komutu bitirir, oturumu yakalar ve bağlantıyı keser. Etkileşimli kabukta Ctrl+C her zamanki gibi kabuğa aittir; çalıştırmayı bitirmek için `exit` yazın.

### Terminalden sağlık denetimleri

`eidos check <instance>`, pencere gerektirmeyen Health denetimlerini çalıştırır;
böylece bunlar SSH üzerinden ya da Game Mode'da çalışabilir ve bir hata raporuna
eklenebilir:

- Proton öneki ve kayıt defteri;
- eklenti tanılamaları ve eksik master'lar;
- script extender ve eklentileri;
- son hatasız başlatmadan beri bir oyun güncellemesi;
- Steam başlatma seçeneği;
- grafik ayarları (NVAPI, tam ekran, ENB);
- son çalıştırmanın çökme günlüğü;
- profil INI'lerindeki `sTestFile` satırları;
- Starfield `.esp` ve blueprint eklentileri;
- `vm.max_map_count` ve açık dosya sınırı;
- örneğin çalıştırdığı oyun kopyası ve bir listenin `ModOrganizer.ini` dosyasındaki,
  başka bir oyun klasörünü adlandıran `gamePath`;
- artık geçerli olmayan koleksiyon sıralama kuralları;
- NTFS, exFAT ya da FAT üzerindeki oyun dosyaları;
- BodySlide veri yolları;
- `Overwrite/Root` içinde bırakılmış `Data` klasörleri;
- `Interface/fontconfig.txt` dosyasının adlandırdığı ve hiçbir modun sağlamadığı
  yazı tipi kitaplıkları;
- Fallout 4 ve Fallout 4 VR modlarının BA2 arşivlerindeki sıkıştırılmış ses.

Bazı kartları yalnızca pencere hesaplar, Diagnostics sekmesinde: yama sırası, bir
modun değiştirdiği extender betikleri, hiçbir eklentinin yüklemediği arşivler,
Nexus gereksinimleri, LOOT'un kirli işaretlediği eklentiler, oyunu çalıştıran
Proton sürümü, Display kartı ve girdilerinden eski araç çıktısı.

Komut, **Copy report** ile aynı sistem satırlarını, ardından her bulgu için bir
satır yazdırır; ev klasörünüz `~`, oturum adınız `<user>` olarak gösterilir. Çıkış
kodu, hiçbir sorun bulunmadığında 0 (yalnızca öneriler yine 0 verir), en az bir
sorun bulunduğunda 1, Eidos denetleyemediğinde 2'dir: yanlış argümanlar, açamadığı
bir örnek ya da algılamadığı bir oyun.

`eidos check <instance> --json` bunun yerine tek bir JSON nesnesi yazdırır. Üst düzey alanları `eidos` (sürüm), `game`, `instance`, `system` (`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`, `max_map_count`, `open_files_hard`) ve `findings`'dir. `max_map_count` ve `open_files_hard` birer sayıdır; Eidos bunları okuyamadığında `null` olur. Her bulgu bir `code`, bir `level` (`problem` ya da `advice`) ve kendi `args` alanından oluşur; Diagnostics sekmesi her kodun cümlesini gösterir.

### Son iyi çalıştırmadan beri neyin değiştiği

`eidos play`, `eidos shell` veya `eidos tool run` ile yapılan her çalıştırma, pencereden, terminalden ya da Steam'den başlasın, `<instance>/logs/sessions/` altına bir kayıt yazar: profil, etkin her mod ile sürümü ve Nexus dosya kimlikleri, eklenti sırası, her profil INI dosyasının sağlama toplamı, oyun çalıştırılabilir dosyasının sürümü, Eidos sürümü, çıkış kodu ve çalıştırmanın oluşturduğu kayıtlar. En yeni 50 kayıt tutulur ve `logs/` paketlenen bir örneğe girmez. `eidos sessions <instance> diff`, çıkış kodu 0 ile biten en yeni oyun çalıştırmasını mevcut durumla karşılaştırır; `diff <id>` listeden başka bir kaydı seçer. Liste bir [test çalıştırmasını](#test-çalıştırmaları) `test` olarak işaretler ve test çalıştırması hiçbir zaman son iyi çalıştırma olarak seçilmez. Durdurduğunuz bir çalıştırma 143 çıkış kodu kaydeder, yani o da seçilmez. Eidos burada henüz çökmeleri algılamaz ve Proton altında çöken bir oyun yine de 0 koduyla çıkabilir: seçilen çalıştırma böyleyse `diff <id>` ile daha eskisini belirtin.

## Mod listesi

MO2'de olduğu gibi her profilin kendi `modlist.txt` dosyası vardır (hangi
modların açık olduğu, hangi öncelikte), mod klasörleri ise örneğin paylaşılan
`mods/` dizininde yaşar. `mods/` içine elle bıraktığınız bir klasör henüz
listelenmemiştir: en yüksek öncelikte ve kapalı olarak görünür, pencerenin
listesinin en altında, MO2'de olduğu gibi; böylece onu açtığınızda
çakışmalarını kazanır.

Klasörler paylaşıldığı için bir modu kaldırmak ya da yeniden adlandırmak her
profili değiştirir: satırı her profilin `modlist.txt` dosyasından çıkarılır ya
da orada yeniden adlandırılır ve her profil geri kalanında kendi durumunu ve
sırasını korur. Hiç modu kalmayan bir profilde hiç `modlist.txt` olmaz.

Pencere, mod listesinin ve yükleme sırasının kendi kopyası üzerinden çalışır.
Pencere açıkken başka bir Eidos süreci bunlardan birini yazarsa (bir terminalde
`eidos install` ya da `eidos sort`, bir koleksiyon, Steam'in başlattığı bir
oturum, ikinci bir pencere), pencerenin sonraki düzenlemesi o işin üzerine
yazmak yerine reddedilir. Pencere diskten yeniden yükler ve *Not saved: another
Eidos process changed the mod list. It has been reloaded - redo the change.* der
(ya da yükleme sırası için aynısını); yalnızca o tek tıklama yitirilir, yani onu
yeniden yapın. Rename, New mod, Add separator, Remove ve Overwrite'ı bir moda
dönüştürmek, `mods/` dizinine dokunmadan önce denetim yapar; böylece reddedilen
bir düzenleme diskte hiçbir şeyi değiştirmez. INI düzenleyicisi de aynı biçimde,
açtığından beri değişmiş bir dosyanın üzerine kaydetmeyi reddeder.

## Kayıtlar profile aittir

Her profil kendi kayıtlarını `profiles/<name>/saves/` içinde tutar ve bir
çalıştırma boyunca o klasör, Proton önekindeki oyunun kayıt klasörünün üzerine
bağlanır. Bir profilin ilk başlatılması, önekte zaten bulunan kayıtları onun
içine bir kez kopyalar; böylece sonradan profilden sildiğiniz bir kayıt silinmiş
kalır. Her çalıştırmadan sonra profilin en yeni kayıtları Steam Cloud için öneke
geri kopyalanır; önekte Eidos'un oraya koymadığı bir kayıt önce profilde
`orphan-<time>-<name>` olarak saklanır, asla üzerine yazılmaz.

O ilk kopyadan sonra Eidos'tan geçmeden öneke inen bir kayıt (bir Steam
Deck'ten gelen bir Steam Cloud indirmesi, Eidos olmadan yapılan bir başlatma),
profilin kayıtları bağlıyken gizli kalır. Hiçbir şey onu silmez: başlatma, o
dosya kümesi her değiştiğinde bir kez, onu çalıştırmanın günlüğünde adlandırır
(pencereden bir başlatma için örneğin `logs/run-*.log` dosyası, Steam'den bir
başlatma için Steam'in konsolu) ve Saves sekmesi bu kayıtları başlığının altındaki
satırda sayar. Birini oynamak için Saves sekmesinde Import'a basın. Import her
birini co-save dosyasıyla birlikte profile kopyalar ve profildeki hiçbir kaydın
yerine geçmez: profilin adını zaten kullandığı bir kayıt `orphan-<time>-<name>`
olarak içe aktarılır.

## Test çalıştırmaları

Run'ın yanındaki Test run düğmesi oyunu Run gibi başlatır, ama oyunun yazdığı her şey profiliniz, Overwrite ve önek yerine `<instance>/test-run/` içine gider: yeni ve değişen kayıtlar, INI değişiklikleri, `plugins.txt`, Data'ya ya da oyun klasörüne yazılan dosyalar ve `Documents/My Games/<game>` içindeki günlükler. Profilin modlarıyla, hiç mod olmadan ya da yalnızca seçili ve açık olan modlar ve bunların eklentilerinin master olarak gerektirdiği modlarla test edebilirsiniz.

Oyun kapandığında Eidos bu dosyaları beş grupta listeler (kayıtlar, INI değişiklikleri, eklenti durumu, Overwrite çıktısı, günlükler) ve her birini tutmak mı yoksa atmak mı istediğinizi sorar. Apply, dosya içeren her grup yanıt alana kadar bekler. Çalıştırma profilinkinden başka bir mod kümesi kullandıysa eklenti durumu sunulmaz. Kayıtları tutmak profilden hiçbir kaydı silmez, oyunun çalıştırma sırasında sildiği bir kaydı bile. Overwrite çıktısını tutmak, oyunun sildiği dosyaları da Overwrite'tan kaldırır ve liste önce onları sayar. Herhangi bir şeyi atmak ikinci bir basış ister. Decide later listeyi kapatır ve dosyaları `test-run/` içinde bırakır; liste, örneği bir sonraki açışınızda ya da F5'e bastığınızda yeniden açılır. Eidos ya da bilgisayar bir test çalıştırması sırasında durursa dosyalar yine orada kalır ve liste aynı şekilde açılır.

Pencere oyunun mağazası tarafından başlatılmadıysa Test run, Run'ın yaptığı gibi oyunu o mağazaya (Steam, Heroic ya da Legendary) devreder ve mağazanın başlattığı çalıştırma test çalıştırması olur. Mağaza oyunu başlatmazsa istek bekler: o profilin Run'dan ya da mağazadan gelen bir sonraki oyun çalıştırması test çalıştırması olur ve Cancel the waiting test run onu bırakır. Bir test çalıştırması yalnızca yukarıda sayılan klasörleri kapsar: oyunun Wine kayıt defteri gibi başka yerlere yazdıkları her zamanki yerlerine gider. Bir test çalıştırmasından gelen kayıtlar Steam Cloud'a ancak onları tuttuktan ve normal bir çalıştırma oynadıktan sonra ulaşır.

### Bir sorunun arkasındaki modu bulmak

Diagnostics'teki Find the mod behind a problem düğmesi, yeniden üretebildiğiniz bir sorunun, örneğin belirli bir yerdeki bir çökmenin nedeni olan modu arar. Her adım, kalan şüphelilerin yarısı, bunların gerektirdiği master'lar ve her zaman açık tuttuğunuz modlarla yapılan bir test çalıştırmasıdır: başlarken listede seçili olan açık modlar ve script extender'ı sağlayan mod. Her çalıştırmadan sonra sorunun olup olmadığını yanıtlarsınız. Not sure şüphelileri başka türlü böler, Run it again adımı yineler. Yanıtlamak ya da Run it again'e basmak, ikinci bir basıştan sonra adımın yazdığı dosyaları atar. Bazılarını tutmak için önce See the files'a basın: bir test çalıştırmasından sonraki listenin aynısını açar. Oyun çöktüyse ve bir crash logger günlük yazdıysa ya da oyun bir hata koduyla çıktıysa Eidos, Yes'i olası yanıt olarak işaretler. 200 şüpheli yaklaşık 8 çalıştırma sürer.

Arama `<instance>/bisect.json` içinde saklanır, bu yüzden adımlar arasında Eidos'u kapatabilirsiniz ve arama başladığı profile aittir. Stop the search ikinci bir basıştan sonra aramayı bırakır. Sonuç, modu ya da birbirinin master'larına ihtiyaç duyan mod kümesini adlandırır ve `<instance>/logs/bisect-<time>.txt` içinde saklanır. Üretilmiş modlar (bir Nemesis ya da DynDOLOD çıktısı, bir Bashed Patch) tam liste için oluşturulur ve listenin yarısıyla bozulabilir: onları her zaman açık tutun ya da tüm arama boyunca kapalı bırakın.

## Wabbajack listeleri

Eidos'ta Wabbajack yükleyicisi yoktur. Listeyi Eidos dışında CLF3 veya Jackify ile kurun, ardından sihirbazda kurulum klasörünü taşınabilir örnek olarak seçin. Eidos klasörü MO2'nin düzeniyle okur: `mods/`, `profiles/<ad>/modlist.txt` ve eklenti listeleri. Yükleyicinin yazdığı `ModOrganizer.ini` dosyasını korur ve hiçbir zaman üzerine yazmaz, böylece listenin kendi çalıştırılabilir dosyaları kayıtlı kalır.

Birçok liste, oyunun yamalı bir kopyasını içeren bir `Stock Game` veya `Game Root` klasörü ekler ve MO2 oyunu oradan başlatır. Sihirbaz veya `eidos init` bir liste klasörünü ilk kez benimsediğinde ve o klasör oyunun çalıştırılabilir dosyasını içerdiğinde örnek de oyunu bu klasörden başlatır. Oyun yine Steam veya oyunun sahibi olan mağaza üzerinden başlar, bu yüzden hiçbir şeyi taşımak gerekmez. Eidos klasörü listenin `ModOrganizer.ini` dosyasındaki `gamePath` değerinden ya da bu iki addan bulur. Eidos'un daha eski bir sürümüyle benimsenmiş bir örnek, siz değiştirene kadar mağaza kurulumunu kullanmaya devam eder (bir sonraki bölüme bakın). Bu `gamePath`, örneğin çalıştırmadığı bir oyun klasörünü adlandırdığında Diagnostics bunu söyler, çünkü PGPatcher gibi Mod Organizer 2 kipindeki araçlar oyunu oradan okur. CLF3, BSA ve DDS dosyalarını kendi araçlarıyla yeniden oluşturur; bu yüzden sonuç bir Wabbajack kurulumundan farklı olabilir ve liste yazarları bunu desteklemez.

## Oyun dosyaları: mağaza kurulumu veya bir kopya

Varsayılan olarak oyun mağaza kurulumundan, yani Steam, Heroic veya Legendary'nin onu tuttuğu klasörden çalışır. Bunun yerine bir örnek, bu klasörün örnek içindeki bir kopyasından çalışabilir. Modlar kopyanın üstüne oturur ve mağaza kurulumu yalnızca Eidos'un sonucu bağladığı yer olur. Oyunu yine mağaza başlatır, bu yüzden Steam katmanını (overlay), gölgelendirici önbelleğini ve bulut kayıtlarını korur.

Diagnostics sekmesinin en üstündeki Game files kartı, örneğin hangi dosyaları çalıştırdığını, kopyanın ve mağaza kurulumunun oyun sürümüyle birlikte gösterir ve bunlar arasında geçiş yapar. Bir terminalde `eidos stock-game <instance>` aynı bilgileri yazdırır; `use <folder>`, `store`, `freeze` ve `delete --yes` kartın düğmelerinin yaptığını yapar. `freeze` boyutu ve kopyanın türünü yazdırır, `freeze --yes` kopyalar.

**Freeze this version** mağaza kurulumunu `<instance>/Stock Game` içine kopyalar ve oyunu oradan çalıştırır; böylece bir mağaza güncellemesi artık script extender eklentilerini bozmaz: güncelleme kopyayı değil, mağaza kurulumunu değiştirir. İlk basış boyutu ve dosya sisteminin bununla ne yapacağını gösterir, ikinci basış kopyalar. Blok paylaşabilen bir dosya sisteminde (reflink), örneğin btrfs ya da XFS üzerinde, kopya mağaza bir dosyayı değiştirene kadar neredeyse hiç yer kaplamaz. ext4 üzerinde ya da oyun ile örnek farklı dosya sistemlerindeyken, oyun klasörü kadar büyük tam bir kopyadır. Eidos şu durumlarda reddeder: Steam oyunu meşgul ya da tam kurulmamış olarak bildiriyorsa, örneğin bir oyunu ya da aracı çalışıyorsa, örnek zaten bir listenin oyun kopyasını, dondurulmuş bir kopyayı ya da bitmemiş bir kopyayı barındırıyorsa ve dosya sisteminde tam bir kopyanın gerektirdiğinden az boş yer varsa. Heroic ve Legendary kopyaları sürmekte olan bir güncelleme için denetlenmez. Stop, bir hata ya da kopyalama sırasında oyuna gelen bir Steam güncellemesi `Stock Game.eidos-tmp` klasörünü bırakır; kart onu silmeyi önerir ve bu klasör hiçbir zaman kullanılmaz.

Bir kopya, kopya alındıktan sonra mağazanın oyun klasörüne eklediği, değiştirdiği veya kaldırdığı her dosyayı gizler: yeni Creation Club dosyaları, yeni bir çalıştırılabilir dosya, yeni DLL'ler. Oyun, eklenti listesi, LOOT ve Health denetimleri hep kopyayı okur. Oyun klasörünün dışında olanları gizlemez: bir güncelleme bir sonraki başlatmada Proton önekine yine çalışma zamanları kurabilir ve Steam oyunu başka bir Proton'a geçirebilir.

**Return to the store version** ikinci basıştan sonra kopyayı bir sonraki başlatmadan çıkarır. Kopya diskte kalır ve **Delete the copy** onu yine ikinci basıştan sonra siler. Örnek kopyayı hâlâ çalıştırıyorsa Delete reddedilir. Eidos bir listenin kendi `Stock Game` veya `Game Root` klasörünü asla silmez. Kopya klasörü kaybolursa ya da artık oyunun çalıştırılabilir dosyasını içermiyorsa başlatmalar reddedilir ve Diagnostics klasörün adını verir; böylece mağaza sürümü hiçbir zaman habersizce çalışmaz.

Morrowind ile `bUseMyGamesDirectory=0` olan Oblivion, eklenti listelerini oyun klasöründe tutar ve Eidos onları henüz bir kopyadan çalıştırmaz.

## Bir Nexus koleksiyonu kurmak

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # onu oku ve ne olacağını söyle
eidos collection skyrimse rqhcxy --no-optional  # isteğe bağlı işaretli üyeleri atla
```

Pencerede, bağlantıyı **File -> Open a Nexus collection...** içine yapıştırın ya
da sitede birine tıklayın. Eidos revizyonun üyelerini, her biri bu örnekle
eşleştirilmiş olarak listeler: kurulu, indirilmiş ya da eksik. *Install this
collection* tüm tarifi bir çalışan iş parçacığında, paketleme ve açma işlerinin
kullandığı ilerleme çubuğuyla çalıştırır ve çubuk bittiğinde rapora dönüşür.
Pencere ve `eidos collection` aynı kurulumu çalıştırır.

Bağlantı verilmezse **File -> Open a Nexus collection...** **Browse** sekmesinde
açılır. Açık örneğin oyunu için yayımlanmış koleksiyonları, 20'şer 20'şer, en çok
indirilene, yakın zamanda güncellenene ya da en yeniye göre sıralı listeler ve
arama kutusu listeyi adı ya da metni eşleşen koleksiyonlara daraltır. Her satır
yazarı, son revizyonu, mod sayısını, indirme sayısını ve bir tarihi verir.
**View**, o koleksiyonun son revizyonunu **Link** sekmesinde açar; bağlantısını
yapıştırmakla aynıdır. Yetişkin işaretli koleksiyonlar, Nexus hesabınız yetişkin
içerik göstermedikçe dışarıda bırakılır, derecelendirmesi olmayan koleksiyonlar
her zaman dışarıda bırakılır ve listenin altındaki satır ikisini de sayar.
Göz atmak, panelin geri kalanı gibi, oturum açmış bir Nexus hesabı ister.

Bir koleksiyon yalnızca kendi oyununun örneğine karşı açılır. Bir Fallout 4 örneği
yüklüyken açılan bir Skyrim koleksiyonu, üyelerini yanlış mod listesiyle
eşleştirmek yerine adıyla reddedilir.

### Eidos bir koleksiyondan ne okur

Bir koleksiyonun neredeyse hiçbir yanı Nexus API'sinden görünmez. Kurulum sırası,
yazarın her modun betikli kurucusuna verdiği yanıtlar, bir dosya çakışmasını hangi
modun kazandığı, hangi eklentilerin nereye yükleneceği ve ikili yamalar,
koleksiyonun kendi arşivinin içindeki bir `collection.json` dosyasındadır. Aynı
modları indirip öntanımlı seçenekleriyle kurmak başka bir oyun üretir.

Eidos arşivi indirir, `collection.json` dosyasını okur ve onu uygular:

| Koleksiyon şunu der | Eidos şunu yapar |
| --- | --- |
| `phase` | o sırayla kurar, isteğe bağlı üyeler en sona |
| `choices` | yazarın her FOMOD'a verdiği yanıtları yeniden oynatır, oynatamadığında da bunu söyler |
| `modRules` | doğru modun her dosyayı kazanması için mod listesini yeniden sıralar |
| `plugins` / `pluginRules` | LOOT kurallarını userlist'inize katar ve sıralar |
| `INI Tweaks/` | onları kendi başına bir mod olarak kurar |
| `tools` | hangi araçları beklediğini söyler; hiçbir şey oluşturmaz |

Üyeler kurulurken, mod listesinin sonunda kurulum sırasıyla, `modRules` onları
kımıldatmadan önce açılır. Mod listesinin dışında kalan bir üye yüklenmezdi ve
rapor bunu göstermezdi.

Durum her üyeden sonra `<instance>/collections/<slug>-<revision>.state.json`
dosyasına yazılır. O revizyonun klasörünün yanında durur, asla içinde değil; bu
yüzden bir koleksiyon kendi durum dosyasını getiremez ve 200 üyenin 147'sinde
yarıda kesilen bir kurulum 147'den devam eder. Geçici bir adla yazılıp yerine
yeniden adlandırılır; yani yarıda kesilme geriye ya eski kaydı ya da yenisini
bırakır, asla ikisinden birinin yarısını değil. Bir gün okunamaz hale gelirse
Eidos bütün koleksiyonu baştan başlatmak yerine durur ve bunu söyler. Aynı komutu
yeniden çalıştırın.

Zaten sahip olduğunuz bir koleksiyonun daha yeni bir revizyonunu kurmak, önceki
revizyonun klasörlerini devralır: değişmemiş bir üye yerinde doğrulanır,
dosyasını yazarın güncellediği bir üye eski klasörünün içinde değiştirilir, yeni
revizyonun çıkardığı bir üye ise kurulu bırakılır ve devre dışı bırakmanız ya da
kaldırmanız için raporda adı geçer. Başka bir profilin açık tuttuğu bir klasör
asla devralınmaz, çünkü mods klasörü paylaşılır ve o profil sizden habersiz
değişirdi: yeni revizyon onun yerine kendi kopyasını onun yanına kurar ve rapor
sizden eskisini bu profilde devre dışı bırakmanızı ister, asla kaldırmanızı
değil. Daha eski bir revizyona dönmek, onun klasörlerini aynı biçimde geri alır
ya da alamadıklarının taze kopyalarını kurar.
Bir üye bu şekilde değiştirildiğinde ve siz onun klasöründe dosyaları
değiştirmiş ya da eklemişseniz, eski klasör önce `<name>_backup` adıyla yanında
saklanır; bu klasör oyuna asla yüklenmez ve raporda adı geçer.

Daha eski bir revizyondan kalıp açık kalan bir artık, kurulumun yazarın
oluşturduğu kurulum sayılmasını engeller; onu devre dışı bıraktığınızda not
kaybolur. `--no-optional` ile atladığınız bir üye devralınmaz ve aynı biçimde adı
geçer. Devralmanın kendisi başarısız olursa çalıştırma *previous revision: its
folders could not be taken over* iletisi ve nedeniyle durur; onu düzeltin ve
komutu yeniden çalıştırın.

### Ücretsiz hesaplar ve kendiniz getirdiğiniz üyeler

Ücretsiz bir Nexus hesabı üye modları getiremez. Nexus, API üzerinden ücretsiz
hesaplara indirme bağlantısı vermez: her dosya sitenin kendi "Mod Manager
Download" düğmesini ister. Koleksiyon arşivi ücretsiz bir hesapta iner, yani
bütün tarifi okuyabilirsiniz, ama her üye, kendiniz getirmeniz için tam sayfa
URL'siyle birlikte raporlanır ve yeniden denemek bunu değiştirmez. Onları sitenin
düğmesiyle getirin ve komutu yeniden çalıştırın: o üyeler her çalıştırmada
`downloads/` içinde aranır, yani koleksiyon siz sağladıkça tamamlanır.

Pencerede ücretsiz bir hesap, koleksiyon panelinde toplu getirme yerine
*Fetch the N missing required files, one by one* düğmesini görür. Bu düğme
eksik zorunlu dosyaların ilkinin Nexus sayfasını açar, o dosyanın `downloads/`
içine gelmesini bekler, sonra bir sonrakini açar. Her dosya için sayfanın
indirme düğmesine yine siz basarsınız. Panel ne kadar ilerlendiğini (*3 of 12*)
*Skip* ve *Stop* ile birlikte gösterir. Onun yerine aynı modun başka bir
dosyası gelirse panel iki dosya kimliğini de yazar ve sabitlenmiş dosyayı
beklemeyi sürdürür. İsteğe bağlı üyeler buna dahil değildir ve kendi
*Open* düğmelerini korur.

Eidos ayrıca ikili yamaları (her biri kayıtlı CRC ile denetlenir), dosya
geçersiz kılmalarını (adı geçen dosyalar MO2'nin yaptığı gibi `.mohidden` ile
gizlenir) ve dosyaları koleksiyon arşivinin içinde gelen üyeleri (`bundle`)
uygular. Koleksiyonun elle getirmenizi beklediği üyeler (`browse`, `manual`),
yazarın kendi yönergelerini rapora taşır. Adı zaten sizin bir modunuzun adı olan
bir üye, onun üzerine değil, yanına, boşta bir adla kurulur ve rapor hangisi
olduğunu söyler.

### Rapor

Kurucu yanıtlarının hepsi yeniden oynatılamayan bir üye, yazarın oluşturduğu gibi
kurulan üyelerden ayrı, kendi satırında *installed, but not the way the
collection asks* olarak görünür, çünkü diskteki dosyaları yazarınkilerden
farklıdır. Rapor ayrıca koleksiyonda hiçbir şeye karşılık gelmeyen sıralama
kurallarını, birbiriyle çelişen kuralların döngüsüne yakalanan modları, var
olmayan LOOT gruplarını ve manifest'in bu Eidos sürümünün anlamadığı her parçasını
listeler.

### Koleksiyon sırasını korumak

Eidos, kurulu her koleksiyonun kurallarını kurulumdan sonra da saklar. Sonradan
onun iki modunu bir önce ya da sonra kuralına aykırı şekilde taşırsanız
Diagnostics sekmesi o koleksiyon için tek bir kart gösterir; ilk böyle çifti
adlandırır ve artık geçerli olmayan kuralları sayar. Yalnızca geçerli profilde
açık modlar sayılır ve bir döngüye yakalanan kurallar atlanır, çünkü hiçbir sıra
onları koruyamaz. **Restore collection order** koleksiyonun modlarını her kuralı
koruyan bir sıraya geri taşır, kuralların izin verdiği her yerde kendi sıranızı
korur ve koleksiyonda olmayan modları oldukları yerde bırakır. Ctrl+Z geri alır.
`eidos check` aynı bulguyu `collection_order_broken` olarak listeler. Denetim,
`collections/` içindeki revizyon tarifini okur; bu yüzden önbelleğe alınmış tarifi
kaldırılmış bir koleksiyon denetlenmez.

## Bir örneği başka bir makineye taşımak

`eidos pack`, bütün bir örneği (her modu, yükleme sırasını, bütün profilleri,
içindeki kayıtlarınızla birlikte Overwrite'ı ve her şeyin kurulduğu arşivleri)
tek bir dosyaya yazar. `eidos unpack` onu geri koyar:

```sh
eidos pack skyrimse ~/backup.eidos                  # ya da bir klasör verin: oyunun ve tarihin adını alır
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # öteki makinede
```

`eidos pack --dry-run`, hiçbir şey yazmadan neyin gireceğini, neyin girmeyeceğini
ve nedenini, bir de ne kadar yer gerektiğini tam olarak listeler.
`eidos unpack --info`, elinizde zaten olan bir dosya için aynısını yapar.

7-Zip bir şeyi okuyamadıysa ya da klasör taraması bir yedekte bulunmasını
bekleyeceğiniz bir şeyi atlamak zorunda kaldıysa (bir sembolik bağ, okuyamadığı
bir klasör, 7-Zip'e güvenle verilemeyecek bir ad), arşiv yine de seçtiğiniz adla
yazılır, ama `eidos pack` 1 ile çıkar ve neyin eksik olduğunu söyler; böylece
`eidos pack ... && ...` çalıştıran bir betik orada durur.

Bir `.eidos` dosyası, kendine ait bir uzantısı olan bir 7-Zip arşividir. Bir modu
kurmak için zaten 7-Zip gerekiyor, yani bu hiçbir bağımlılık eklemez ve yedek,
7-Zip okuyan herhangi bir arşivleyicide Eidos olmadan açılır. LZMA2 ile `-mx1`
düzeyinde sıkıştırılır; doku ağırlıklı bir listede özgün boyutun yaklaşık %43'ü.
`-mx9` çok daha uzun sürer ve birkaç puan daha kazandırır. Dokular ve mesh'ler zaten
sıkıştırılmış biçimlerdir; daha büyük bir sözlüğün bulabileceği pek bir şey kalmaz.
`--level` (0, 1, 3, 5, 7 ya da 9) başka bir düzey seçer, `--no-downloads` ise
`downloads/` dizinini dışarıda bırakır.

Varsayılan düzey 1'de dosyalar en fazla 16 MB'lık solid bloklara yerleştirilir.
70 GB'lık bir yedekten tek bir dosyayı çıkarmak en fazla o dosyanın bulunduğu
bloğu çözer, asla önündeki her şeyi değil; oyun içeriğini paketlemek de her
dosyanın ayrı ayrı sıkıştırıldığı duruma göre 2 ila 5 kat daha hızlıdır (açma 2
ila 4 kat). Bedeli hasar durumunda ödenir: tek bir bozuk bayt, bloğundaki bütün
dosyaları, yani 16 MB'a kadar komşu dosyayı yanında götürebilir. Aşağıda
anlatılan kurtarma verisi bunu karşılar. Diğer bütün düzeyler non-solid kalır:
her dosya kendi akışıdır ve bozuk bir bayt yalnızca o dosyaya mal olur. Düzey 0
dosyaları sıkıştırmadan saklar. Düzey 3 ve üstünde LZMA2 zaten 16 MB ve daha
büyük parçalarla çalışır; orada solid bloklar ölçümlerde daha yavaş çıktı: düzey
5'te mesh'leri paketlemek 1,4 kat, dokuları açmak 6,8 kat daha uzun sürdü.

Paketleme örnek kilidini alır, yani oyun ya da başka bir Eidos o örneğe yazarken
çalışamaz ve okunurken hiçbir şey örneği değiştirmez. Yedek bir `.part` adıyla
yazılır, diske boşaltılır ve seçtiğiniz adı almadan önce `7z t` ile baştan sona
geri okunur. Bu denetim her bloğu çözer ve her sağlama toplamını karşılaştırır;
diğer makinedeki bir açma işleminin yapacağı işin aynısı. Diskin yazmadan hemen
sonra geri verdiğini gösterir, sürücünün onu saklayacağını değil; bunun için
aşağıdaki kurtarma verisi vardır. Başarısız olursa `.part` silinir, sizin
verdiğiniz adla hiçbir şey yazılmaz ve `eidos pack`, 7-Zip'in hasarı bulduğu
satırı aktarır. Yarıda kesilen ya da başarısız olan bir paketleme, bir `.part`
dosyası ya da hiçbir şey bırakır; tam görünen ama olmayan bir `.eidos` dosyası
asla bırakmaz.

par2cmdline kuruluysa (Arch'ta `sudo pacman -S par2cmdline`, Debian ve Ubuntu'da
`sudo apt install par2`, Fedora'da `sudo dnf install par2cmdline`), `eidos pack`
arşivin yanına PAR2 kurtarma verisi de yazar: `backup.eidos.par2` ve birkaç
`backup.eidos.volNN+MM.par2` dosyası, toplamda arşivin %5'i artı küçük bir
dizin. Bunları `.eidos` dosyasıyla aynı klasörde, yazıldıkları adlarla tutun ve
onunla birlikte kopyalayın: onlar olmadan taşınan bir yedeğin onarılacağı hiçbir
kaynak yoktur ve kurtarma verisi arşivin adını kaydeder, bu yüzden adı
değiştirilmiş bir yedek artık onlarla eşleşmez. `eidos unpack` onları adlarından
bulur ve herhangi bir şeyi çıkarmadan önce arşivi onlarla denetler; sonu
7-Zip'in açamayacağı kadar kesilmiş bir yedek de buna dahildir. Birkaç bayt
bozulmuşsa par2 önce arşivi onarır ve hasarlı kopyayı `backup.eidos.1` olarak
saklar, bunun için arşivin klasöründe arşiv boyutu kadar boş alan gerekir; %5'in
karşılayabileceğinden büyük bir hasar, tek bir dosya yazılmadan açma işlemini
durdurur. Arşivle eşleşmeyen kurtarma dosyaları, örneğin aynı adı taşıyan daha
eski bir yedeğinkiler, 7-Zip arşivi okuyup sağlam bulduğunda kullanılmaz ve açma
işlemi bunu bildirir. par2 olmayan bir makinede açma yine de çalışır ve kurtarma
verisinin kullanılmadığına dair bir uyarı verir. `--no-recovery` onlarsız
paketler ve `eidos pack --dry-run` yazılıp yazılmayacağını söyler. `--force`
aynı adlı mevcut bir dosyanın yerine yazar.

### İçinde ne yok ve neden

| Dışarıda kalan | Neden |
| --- | --- |
| Proton öneki | Makineye göre biçimlenmiştir (sürücü eşlemeleri, *bu* dosya sisteminin bir `Z:` görünümü) ve Eidos onu `eidos prereqs <instance> --install` ile bir dakikada yeniden kurar. Onu taşımak, öteki makinenin nasılsa yeniden üretmesi gereken bir şeyi göndermek uğruna arşivi kabaca ikiye katlardı. |
| Oyun | Bir örnek, içinde barındırmadığı bir oyunu modlar. Onu orada Steam'den kurun. |
| Örneğin dışındaki araçlar | xEdit, DynDOLOD ve diğer araçlar, kendi kurucuları olan üçüncü taraf ikili dosyalarıdır. Arşivde adları geçer, böylece açma işlemi yeni makinede hangilerinin eksik olduğunu tam olarak söyler. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Yedeği alan makinenin kayıtları. Özellikle `prereqs.done`, çalışma zamanı kitaplıklarının orada olmayan bir önekte zaten kurulu olduğunu iddia ederdi. |
| `loot/`, `userlist.yaml` dışında | Eidos'un gerektiğinde yeniden getirdiği bir masterlist önbelleği. `userlist.yaml` içindeki kendi LOOT kurallarınız yeniden getirilemez, bu yüzden o tek dosya birlikte gelir. |
| `.base/`, `.base-root/` | Oyunun kendi dosyalarının bir oturum boyunca saklandığı boş bağlama noktaları. |
| `test-run/`, `bisect.json` | İncelenmeyi bekleyen bir [test çalıştırması](#test-çalıştırmaları) ve iki adım arasındaki bir sorunun arkasındaki modu arama. İkisi de bu makinedeki çalıştırmalara aittir. |
| Yarım yazılmış dosyalar | Duraklatılmış bir indirme (`*.unfinished`), yolda olan atomik bir yazma (`*.eidos-tmp*`). |
| Sembolik bağlar | 7-Zip birini izler ve neyi gösteriyorsa onu kopyalardı; mutlak bir bağ için bu, yedeğinizin içine yabancı bir ağaç çekmek demektir. Onun yerine bildirilirler ve yedek eksik sayılır: başka bir sürücüye bağlı bir `mods/` ya da `downloads/` paketlenmez. Bağlı bir `downloads/` olmadan bilerek yedek almak için `--no-downloads` verin (ya da pencerede *Include downloads/* işaretini kaldırın); kasıtlı bir dışlama eksik sayılmaz. |

Örnekteki bir oyun kopyası (`Stock Game`) diğer klasörler gibi paketlenir; bu yüzden dondurulmuş bir örneğin yedeği yaklaşık oyunun boyutu kadar büyük olur. Bitmemiş bir kopya (`Stock Game.eidos-tmp`) dışarıda bırakılır.

Bunların her biri, nedeniyle birlikte arşivin kökündeki `eidos-backup.ini`
dosyasına girer; yani o dosyayı okumak nelerin dışarıda bırakıldığını gösterir.
Aynı dosya, örneğin eskiden nerede olduğunu, hangi profilin etkin olduğunu, açıldığında
ne kadar yer tuttuğunu ve yeni makinenin hangi araçlara gereksinim duyacağını
kaydeder.

İndirme kayıtları (`downloads/*.meta`), `url=` satırları boşaltılmış olarak
girer. O URL imzalı bir Nexus bağlantısıdır: saatler içinde çalışmayı bırakır ve
dosyayı indiren hesabın `user_id` değerini taşır; bir yedek de başkasına
verilebilir. Mod kimliği, dosya kimliği ve sürüm kalır, böylece indirme yeniden
bulunabilir.

### Pencerede

İkisi de **File** menüsündedir: *Pack this instance...* ve *Unpack a backup...*.
Açma işlemi ayrıca karşılama ekranında, örneklerin listesinin altında da
bulunur; böylece henüz hiç örneği olmayan bir makine de bir tane açabilir.

Pack iletişim kutusu, bir şey yazılmadan önce kaç dosya, ne kadar büyük, bunun ne
kadarı `downloads/`, arşivin kabaca ne tutacağı ve hangi araçların adı geçtiği
halde taşınmadığı gösterir. *Write recovery data* kutusu par2 kuruluysa
işaretlidir, kurulu değilse kurulacak paketin adıyla birlikte grileşir. Unpack
iletişim kutusu, dosyayı seçtiğiniz anda yedeğin manifest'ini okur; böylece
klasör seçmeden önce hangi oyunu barındırdığını, ne zaman alındığını, eskiden
nerede olduğunu ve araçlarından hangilerinin burada eksik olduğunu söyleyebilir.

İkisi de bir ilerleme çubuğuyla bir çalışan iş parçacığında koşar ve bittiklerinde
çubuk raporun kendisi olur.

### Açma işleminin onardıkları

Klasörün düz bir kopyası, her araç düğmesini eski makinedeki bir yolu gösterir
halde bırakırdı. `eidos unpack`, eski örnek kökünü adlandıran değerleri ve
yalnızca onları yeniden yazar: `tools.ini` içindeki `exe`, `workdir` ve numaralı
`arg` anahtarları ile her modun `meta.ini` dosyasındaki `installationFile`.
Hiçbir zaman örneğin içinde olmamış bir araç (`/opt/xedit/SSEEdit.exe`) tam
olarak olduğu gibi bırakılır ve yeniden kurulacaklar arasında listelenir;
Eidos o aracın yeni makinede nerede olduğunu tahmin etmez. Ayrıca örneğin kendini
merkezî mi taşınabilir mi saydığını da düzeltir, böylece sonraki komutlar onu artık
bulunduğu yerde arar.

Açma işlemi şunları reddeder: boş olmayan bir klasör (üzerine yazmak
istiyorsanız `--force` verin), manifest'in kendi rakamı için fazla küçük bir
disk, sizinkinden yeni bir Eidos'un yaptığı bir arşiv ve seçtiğiniz klasörün
dışına yazılacak bir girdi barındıran her arşiv. Klasör verilmezse, global bir
örneğin yedeği oyunu için merkezî yere geri döner; taşınabilir olan bir klasör ister.

Sonrasında:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # Proton önekini yeniden kur
eidos play /mnt/games/EidosSkyrim
```

## Pencere

`eidos-gui` ya da Colony'nin kurduğu `eidos-linux` dosyasını başlatın.

İlk başlatma MO2'nin sihirbazı gibi bir sihirbaz açar: karşılama, örnek türü
(taşınabilir ya da global), oyun, ad ve konum, özet, sonra ana pencere. Karşılama
ekranı ayrıca bilinen her örneği, global ve taşınabilir, en son kullanılan başta
olmak üzere tek tıkla açmak için listeler; yani örnek değiştirici işi de görür.
Sihirbazı bir örnek içeren bir klasöre yöneltmek, üzerine yeni bir tane
oluşturmak yerine o örneği olduğu gibi açar ve başka bir oyuna ait bir klasör
reddedilir.

Ana pencerenin iki bölmesi vardır: mod listesi ve Data, Plugins, Conflicts,
Overwrite, Archives, Saves, Downloads ve Diagnostics sekmeleri; ayrıca bir Run
düğmesi, bir Test run düğmesi ([Test çalıştırmaları](#test-çalıştırmaları)) ve bir
çalıştırma hedefi seçici. Mod listesini süzer, seçer, yeniden sıralar, ayırıcılarla
gruplar, kategoriye göre daraltır ve eylemler için sağ tıklarsınız. En üste ve en
alta göndermenin yanında mod menüsü MO2'nin hedefli taşımalarına sahiptir: ilk
çakışan modun üstüne, sonuncunun altına, verilen bir önceliğe ya da bir ayırıcının
grubuna göndermek.

Profil seçici profilleri değiştirir. Bir profile sağ tıklayın: Rename,
Copy to new..., Desktop shortcut, Import from MO2... ve silme; silme iki tıklama
ister ve etkin ya da son profili reddeder. Import from MO2... bir MO2 profilinin mod
sırasını ve yükleme sırasını devralır. Desktop shortcut,
`eidos tool <instance> run "<game>" --profile <name>` komutunu çalıştıran bir
masaüstü girdisi yazar: o profili etkin yapar ve mağazadan oyunu başlatmasını ister
(ayrıntılar [tools.md](tools.md#bir-araca-ulaşmak-gizlemek-sabitlemek-ve-bir-masaüstü-kısayolu)
içinde).

**File** menüsü ayrıca oyunun önekinde winecfg, regedit, Wine dosya gezgini ve
kaldırıcıyı çalıştırır ve Prefix drive_c önekin `C:` sürücüsünü açar; bkz.
[tools.md](tools.md#wineın-kendi-programları-oyunun-önekinde).
*Instances...* Eidos'un bildiği örnekleri listeler; Forget birini o listeden
çıkarır ve diskte hiçbir şeye dokunmaz. *Export the mod list...* listeyi,
`eidos export` gibi, MO2'nin CSV biçiminde yazar.

### Tercihler

İki bölmenin yerine Tercihler'i açmak için sol üstteki **Eidos**'a tıklayın;
geri dönmek için ona yeniden (ya da **Close**'a) tıklayın. Her Colony
programında olduğu gibi önce General, Appearance ve Accessibility gelir, ardından
Mod list, Nexus ve About. Her değişiklik anında uygulanır ve kaydedilir; Save
düğmesi yoktur.

- General: Eidos'un açtığı oyun, pencere boyutu, arayüz dili, araçlar klasörü ve
  bir oyunun nasıl çalıştırıldığı. Lock the window while a game runs öntanımlı
  olarak açıktır. Start the game at once when a store opens Eidos ve pencere
  arka plandayken uzun bir iş bittiğinde bunu söyleyen masaüstü bildirimi
  öntanımlı olarak kapalıdır. Pencere İngilizce ve Fransızca konuşur; System
  `LC_ALL`, `LC_MESSAGES` ya da `LANG` değerini izler ve değişiklik anında
  uygulanır. `eidos` komut satırı her zaman İngilizce yanıt verir.
- Appearance: tema (26 aile, 59 palet, öntanımlı olarak Eidos'un parşömeni),
  vurgu rengi, metin boyutu, efektler ve canlı bir önizleme.
- Accessibility: yüksek karşıtlık, disleksi dostu bir yazı tipi, azaltılmış
  hareket ve Appearance'taki boyutla çarpılan bir okuma metni boyutu.
- Mod list: bir mod listenin üst ya da alt kenarına sürüklenirken otomatik
  kaydırma hızı.
- Nexus: oturum açma, indirme klasörü, `nxm://` işleyicisi ve Register düğmesi,
  tercih edilen sunucular, Install downloads when they finish ve Offline mode.
- About: dosyaların nerede durduğu, klavye kısayolları, sürüm ve sürüm denetimi.

Görünüm ve arayüz dili `~/.config/Colony/Eidos/preferences.json` içinde durur.
Komut satırının da okuduğu diğer ayarlar (öntanımlı oyun, araçlar klasörü, Nexus
sunucuları, Offline mode ve gerisi) yanındaki `settings.ini` içinde, Nexus oturumu
ise `nexus.ini` içinde durur. Daha eski bir Eidos'un `settings.ini` içinde tuttuğu
tema ilk seferde aktarılır.

**About -> Version**, öntanımlı olarak açık olan sürüm denetimini içerir. Eidos
başlarken, günde en fazla bir kez, GitHub'a `Project-Colony/Eidos` deposunun en son
sürümünü sorar. Bu sürüm çalışan kopyadan yeniyse menü çubuğunda Help'in ardından
`Eidos <version> is available: release notes` görünür ve bağlantı o sürümün
değişiklik günlüğünü içeren sayfasını açar. Eidos kendini asla indirmez ya da
değiştirmez. Çevrimdışı kipte denetim atlanır. Bir paket yöneticisinin `/usr`
altına kurduğu kopyada kapalıdır, çünkü onu paket yöneticisi günceller;
`install.sh --system` komutunun Eidos'u koyduğu `/usr/local` sizin kendi
kurulumunuz sayılır.

### Sütunlar, sıralama ve gruplama

Liste öntanımlı olarak dört sütun gösterir (MO2'nin gösterdiği Category, Content,
Version ve Flags) ve sekiz sunar: Category, Content, Version, Author, Installed,
Nexus id, Game, Flags. Onları View menüsünden işaretleyin. Sekizi de açıkken ad
sütununa çok az yer kalır.

Ona göre sıralamak için bir başlığa tıklayın, ters çevirmek için yeniden, yükleme
sırasına dönmek için üçüncü kez. Liste yalnızca yükleme sırasındayken
sürüklenebilir: bir bırakma konumu gerçek listeye seslenir ve bir sıralama
altında bir satırın ekrandaki yeri, o listedeki yeri değildir. Bir sıralama açıkken
ekleme şeritleri çizilmez ve bir sürükleme reddedilir; MO2'de de böyledir. View
menüsü bunu söyler ve geri dönüş yolunu sunar.

View menüsü ayrıca bütün listeyi kategoriye ya da kaynağa göre (Nexus'tan ya da
elle kurulmuş) gruplayabilir. Grup başlıkları ayraç değildir: yeniden
adlandırılamaz, renklendirilemez ya da taşınamaz. Katlanırlar ve katlanmış bir
başlık sayısını korur. Bir sıralama ya da gruplama açıkken ayraçlar gizlenir, çünkü
bir ayraç yükleme sırasında kendisinden sonra gelen satırların başındadır ve ikisi
de o sırayı ekranda değiştirir.

### Fare ve klavye

Information için bir moda çift tıklayın, klasörü için Ctrl+çift tıklayın, Nexus
sayfası için Shift+çift tıklayın. Ctrl+F imleci süzgeç kutusuna koyar. Bir harfe
basmak onunla başlayan sonraki moda atlar, yeniden basmak ise ilkine takılıp
kalmak yerine geri kalanını dolaşır. Hiçbiri, süzgecin, katlanmış bir ayracın ya
da katlanmış bir grubun gizlediği bir satıra inmez; böylece sonraki Space,
göremediğiniz bir modu açıp kapatmaz.

Bir ayracın menüsündeki "Collapse others", o grup dışındaki her grubu katlar. Bir
sürükleme sırasında işaretçiyi katlanmış bir grubun üzerinde sabit tutmak onu açar,
böylece mod içine bırakılabilir; üzerinden geçmek açmaz.

Ctrl+Z mod listesindeki ya da yükleme sırasındaki son değişikliği geri alır: bir
işaret, bir taşıma, toplu bir devre dışı bırakma, bir LOOT sıralaması.
Ctrl+Shift+Z onu yeniden yapar. Pencere 50 adım tutar ve bunları F5'te, bir oyun
ya da araç çalıştırmasının sonunda, profil değişiminde ve başka bir Eidos süreci
listeyi değiştirdiğinde unutur. Bir modu kurmak, yeniden adlandırmak ya da
kaldırmak diskteki klasörleri değiştirir, bu yüzden bunlardan birinin ötesine
geçen bir geri alma reddedilir.

### Listeyi süzmek

Filters düğmesi her ölçüt için bir satır açar: etkin, çakışmalı, güncelleme var,
kategorisiz, endorsed, tracked, Nexus id'si olan, gizli dosyası olan, geçerli oyun
verisi olmayan, başka bir oyun için, notu olan ve her içerik türü için bir satır
(eklentiler, BSA veya BA2 arşivleri, dokular, mesh'ler, betikler, script extender
eklentileri, arayüz dosyaları, ses, FaceGen). Her tıklama satırı kapalı, only ve
except arasında döndürür. En üstteki satır, açık ölçütlerin hepsini karşılayan
modlarla en az birini karşılayan modlar arasında geçiş yapar. Ad kutusu ve kategori
listesi sonucu her zaman daha da daraltır. Gizli dosyalar satırı yalnızca etkin
modlara bakar.

### Listenin bir mod hakkında söyledikleri

İki uyarı bayrağı; ikisi de üzerine gelince açıklaması çıkan birer simge. No valid
game data, modun tepesinde bu oyunun yüklediği bir şeye benzeyen hiçbir şey
olmadığı anlamına gelir; klasörlerinin bir düzey yukarı taşınması gerekiyor olabilir
ya da bu oyun için bir mod olmayabilir. Another game, modun kendi `meta.ini`
dosyasının başka bir oyunu adlandırdığı anlamına gelir. İkisi de hiçbir şeyi
engellemez: mod yine de yerleşir. Satır menüsündeki "Mark as valid" her ikisini de
MO2'nin kendi `validated=` anahtarı üzerinden susturur, böylece bir yöneticide
geçerli işaretlenmiş bir mod diğerinde işaretlenmez.

Yerleşim denetimi bir modu yalnızca emin olduğunda işaretler: bir `Root/` ağacı,
okunamayan bir klasör ve boş bir klasör geçerli oyun verisi sayılır.

### Kayıtlarınızın veya başka modların kullandığı bir modu kapatmak

Bir mod kapatılmadan ya da kaldırılmadan önce Eidos, profilin en yeni üç
kaydının eklenti listelerini okur. Bunlardan biri o moddan bir eklenti
kullanıyorsa, listenin üstündeki bir çubuk eklentilerin adını verir ve
Disable anyway ya da Cancel bekler (Esc de iptal eder). Remove, yeniden
tıklamayı isteyen iletisine aynı adları ekler. Yalnızca Skyrim ve Fallout 4
kayıtları okunur; diğer oyunlarda mod sormadan kapatılır.

Bir modun Information bölümündeki General sekmesi, o modun ihtiyaç duyduğu modları ve ona ihtiyaç duyan modları, etkin eklentilerin master'larından okuyarak listeler. Başkalarının ihtiyaç duyduğu bir modu kapatmak aynı çubuğu gösterir; Disable them too, ona ihtiyaç duyan her modu ve onlara ihtiyaç duyan modları da kapatır, Keep them enabled ise yalnızca seçileni kapatır.
Diagnostics sekmesindeki eksik master kartında Enable what provides them düğmesi vardır: eksik her master'ı içeren modu etkinleştirir ve mevcut ama işaretsiz bir master eklentisini işaretler.

### Bir moda dokunmadan önce onu yedeklemek

"Back up this mod", klasörünü `<name>_backup` olarak (sonra `_backup2` ve böyle
sürer; bir yedek asla bir öncekinin yerine geçmez) bir yana kopyalar. Kopya bir mod
değildir: onay kutusu hiçbir şey yapmaz ve birleşik görünüme hiçbir şey katmaz,
çünkü onu işaretlemek tek bir modun iki kopyasını üst üste yerleştirirdi. "Restore
this backup over the mod" onu iki tıkla geri koyar; geçerli içerik önce bir yana
taşınır ve ancak kopyalama başarılı olduğunda atılır.

Araç çubuğundaki **Backups**, profilin mod listesinin, yükleme sırasının
(`plugins.txt` ve `loadorder.txt` birlikte) ve oyun INI'lerinin geri yükleme
noktalarını, her birinden 10 tane, `<profile>/backups/` içinde tutar. Back up now
bir tane ekler ve Restore önce geçerli durumu yedekler, böylece yanlış bir seçim
geri alınabilir. Eidos ayrıca her BethINI çalıştırmasından önce bir INI geri
yükleme noktası tutar (bkz. [tools.md](tools.md#bethini-pie)).

### Sekmeler

**Data**, birleşik görünümün bir ağacıdır ve her seferinde bir düzey açılır: bir
klasörü açmak o klasöre sahip her katmandaki klasörü okur, etkin her modu bir
anda değil. Bağlamanın sunduğu aynı katman yığınından kurulur, yani whiteout'lar ve
gizli dosyalar gözetilir ve sekme oyunun göreceğini gösterir. Ada göre süzün,
yalnızca çekişmeli dosyalara daraltın, Size ve Modified sütunlarıyla neyin nerede
olduğunu ayıklayın ve herhangi bir satırı bir dosya yöneticisinde Reveal ile
gösterin. Bir dosyanın üzerine gelmek, onu getiren diğer modları listeler. Bir
modun en üst düzeydeki `meta.ini`, `readme.txt`, `fomod/` ve `Root/` öğeleri Data'ya
girmez, ne bu sekmede ne de bağlamada: ilk üçü mod yöneticisi içindir, `Root/` ise
oyun klasörüne gider. Oyunun ya da bir aracın bu adlardan biriyle yazdığı dosya
Overwrite'a düşer ve görünür kalır.

Data sekmesindeki bir `.exe` satırında Add tool ve Run bulunur. Add tool, Executables düzenleyicisini o dosya doldurulmuş olarak açar (Save'e basana kadar hiçbir şey kaydedilmez). Run, dosyayı kayıtlı bir araç gibi birleşik görünüm üzerinden bir kez çalıştırır ([Araçlar](#araçlar)). Save tree, birleşik Data ağacının tamamını bir metin dosyasına yazar; her dosya için bir satır ve dosyayı sağlayan mod yer alır, ev klasörünüz ve kullanıcı adınız çıkarılır, hata raporu içindir.

**Plugins**, ESP/ESM/ESL yükleme sırasıdır: açıp kapatın, elle yeniden sıralayın ya
da LOOT ile sıralayıp sıralama sonrası raporu okuyun; oradaki öneri bağlantıları
tarayıcınızda açılır. Başlığı, etkin eklentileri her dizin alanında oyunun sınırına
karşı sayar, örneğin `full 231/254, light 512/4096`, ve biri dolunca turuncuya döner.
Filtre kutusu listeyi ada göre daraltır. Space, Ctrl+A, Shift aralıkları ve satır
menüsündeki Activate all shown ile Deactivate all shown yalnızca gösterilen satırlara
etki eder ve filtre temizlenene kadar bir sürükleme reddedilir. Bir eklentiyi
seçmek, master'larını eklenti listesinde ve onu getiren modu mod listesinde
renklendirir. LOOT'un kirli işaretlediği eklentiler, onları xEdit ile
temizleyebilen bir Diagnostics kartı alır
([tools.md](tools.md#lootun-kirli-olarak-işaretlediği-bir-eklentiyi-temizlemek)).

Bir LOOT sıralamasından sonra, yeni kayıtlarının hepsi light aralığına sığan tam bir
eklenti bunu fareyle üzerine gelince söyler ve satır menüsünde Flag as light bulunur.
Eidos, light bayrağı açık bir kopyayı mod listesinin en altında (en yüksek öncelik)
`<plugin> (light)` adlı yeni bir moda yazar ve özgün dosyaya dokunmaz; o modu
kapatmak işlemi geri alır. Profilin en yeni üç kaydından biri eklentiyi tam eklenti
olarak yüklüyorsa Eidos reddeder. Yalnızca Skyrim ve Fallout 4 kayıtları okunur;
Starfield'da hiçbir kayıt denetlenmez. LOOT yalnızca kayıt aralıklarını denetler:
hücre ya da worldspace ekleyen bir eklentiyi işaretlemeden önce xEdit'te inceleyin.

Bir eklentinin üzerine gelince kaynağı, yazarı, başlık ve form sürümü, açıklamasının ilk 300 karakteri, master dosyaları ve LOOT sıralamasından sonra LOOT iletileri görünür. Skyrim, Skyrim SE, Skyrim VR ve Enderal SE'de oyun, yüklenen bir eklentinin adını taşıyan bir INI dosyasını da okur; INI sütunu bu eklentileri işaretler ve tıklamak dosyayı salt okunur açar.

**Conflicts**, dosya başına kazananları ve kaybedenleri açıklar. **Overwrite**,
oyunun yazdığını tek adımda gerçek bir moda dönüştürür.

**Archives**, etkin modların getirdiği her BSA ya da BA2'yi, geldiği modu ve neden
yüklendiğini listeler: aynı adlı etkin bir eklenti ya da INI onu kaydeder. Hiçbir
şeyin yüklemediği bir arşiv ölü ağırlık olarak işaretlenir. Başka bir modun aynı
adlı ve çakışmayı kazanan bir arşivi varsa satır o modu adlandırır. Extract bir
arşivi, uzantısı olmadan adını taşıyan yeni bir moda açar (`Foo.ba2`, `Foo
(extracted)` olur) ve arşivi kendi modunda bırakır; serbest dosyalar, mod sırası ne
olursa olsun, herhangi bir arşivdeki dosyaları yener.

**Saves**, her kaydın başlığını okur (karakter, seviye, konum, oynanma süresi),
içindeki eklenti listesini geçerli listeyle karşılaştırır ve gerektirdiği modları
etkinleştiren bir düğmeye sahiptir. Eidos olmadan öneke ulaşan kayıtlar
[Kayıtlar profile aittir](#kayıtlar-profile-aittir) altında anlatılır.

**Downloads** ve **Diagnostics**'in aşağıda kendi bölümleri vardır.

### Mod bilgisi

"Information..." mod başına bir iletişim kutusu açar: General, Conflicts,
Filetree, INI Tweaks, Notes, Nexus files ve Nexus page sekmeleri. Filetree'den (ve
Data ağacından) herhangi bir dosya gizlenebilir: `<name>.mohidden` olarak yeniden
adlandırılır, bu da onu silmeden birleşik görünümün dışına çıkarır; böylece bir
modun birkaç başıboş dosyası önceliklere dokunmadan bastırılabilir. Filetree
sıradan dosya işlemlerini de yapar: yeni klasör, yeniden adlandır, sil, aç. Hepsi,
o modun içindeki düz bir yol olmayan her şeyi reddeder: `..` yok, mutlak yol yok ve
yol boyunca sembolik bağ yok, çünkü birini izlemek mod klasörünün dışındaki bir
dosyayı silebilirdi. Rename yalnızca yolun son parçasını değiştirir, yani bir
dosyayı asla taşımaz ve zaten alınmış bir adı reddeder. Delete iki tık ister ve geri
alınamaz.

Filetree'deki ya da Data ağacındaki herhangi bir satırda **View** dosyayı önizler:
görseller, metin, DDS dokuları (mip, katman, yüz ve kanalı seçin) ve difüz
dokularıyla birlikte statik NIF modelleri. NIF önizlemesi Eidos'un yanında ya da
`PATH` üzerinde `eidos-nif-preview` ister; sürüm paketi, Arch paketi ve kaynaktan
derlemelerde vardır, `eidos-linux` içinde yoktur. Metin 64 KB'a kadar okunur ve
nerede durduğunu söyler.

**INI Tweaks**, bir modun `INI Tweaks/` klasöründe getirdiği parçaları listeler;
etkin olanlar başlatmada, öncelik sırasıyla profilin oyun INI'sine katılır ve
çalıştırmanın INI'leri yakalandığında geri çıkarılır, yani bir ayarlamayı devre dışı
bırakmak ayarlarını kaldırır.

**Nexus files** sekmesi modun Nexus'taki dosyalarını, en yenisi başta, listeler ve
burada kurulu olanı ve onun yerini alanı işaretler. O ikinci satırda bir **Update**
düğmesi vardır; bkz. [Mod güncellemeleri](#mod-güncellemeleri).

**Nexus page** sekmesi modun açıklamasını ve sayfasındaki her dosya sürümünün
değişiklik günlüğünü, en yenisi başta, gösterir. Eidos ikisini de sekmeyi ilk
açtığınızda, saatlik istek bütçesinden sayılmayan Nexus GraphQL API'sinden okur.
Eidos kapanana ya da **Refresh**'e basana kadar onları tutar. Düz metin olarak
gösterilir: BBCode ve HTML biçimlendirmesi kaldırılır, bir bağlantının adresi
metninden sonra yazılır, görseller ve videolar dışarıda bırakılır. Metnin üstündeki
**Endorse** ve **Track**, modun sağ tık menüsünün yaptığını yapar. Yetişkin bir
modun sayfası, Nexus hesabınız yetişkin içerik göstermedikçe gizli kalır ve
derecelendirmesi olmayan bir sayfa her durumda gizli kalır.

### İndirmeler

Bir indirme, o öncelikte kurulması için Downloads listesinden mod listesindeki bir
konuma sürüklenebilir ve bir dosya yöneticisinden pencereye bırakılan arşivler ya
da klasörler de kurulur. Dosya bırakma bir X11 ya da XWayland oturumu ister: winit
bunu yalnızca X11 için gerçekler. İndirmeler duraklatılıp sürdürülebilir:
duraklatmak aktarımı durdurur ve yarım dosyayı saklar, Resume ise Nexus'tan taze bir
bağlantı ister ve durduğu yerden devam eder.

Downloads sekmesi indirdiğiniz arşivleri tutar. Ada göre süzün (dost mod adına göre
de, yani "skyui", `SkyUI_5_2_SE-12604-5-2SE.7z` dosyasını bulur), en yeniye, ada,
boyuta ya da duruma göre sıralayın ve işiniz bittiği bir arşivi gizleyin: dosya
kalır, yalnızca satır gider. "Show hidden" onları geri getirir, aynı düğme
gizlemeyi de kaldırır. "Remove N installed" zaten kurduğunuz modların arşivlerini
iki tıkla siler, hem de yalnızca süzgecin gösterdiklerini.

**Preferences -> Nexus -> Install downloads when they finish** öntanımlı olarak
kapalıdır. Açıkken, pencere açıkken biten bir Nexus indirmesi Downloads sekmesine
uğramadan kurulur. Bu olağan kurulumdur: bir FOMOD kendi kurucusunu açar, mod
listesinde zaten bulunan bir ad ne yapılacağını sorar. Pencere açılmadan önce ya da
seçenek açılmadan önce `downloads/` içinde duran arşivlere dokunulmaz, kurulumunu
iptal ettiğiniz indirmeye de. Koleksiyon panelinde bir koleksiyon açıkken hiçbir
indirme kendiliğinden kurulmaz ve bu sırada biten bir indirme sonradan da
kendiliğinden kurulmaz; koleksiyon üyelerini kendisi kurar. Bekleyen bir Update'in
beklediği dosyayı Update kurar. Bu şekilde yalnızca Nexus indirmeleri kurulur ve
Downloads süzgecinin gizlediği biri süzgeç temizlenene kadar bekler. Bir sürükleme
sürerken ya da oyun ya da başka bir Eidos örneği tutarken kurulum bekler.

### Mod güncellemeleri

Araç çubuğunun sağ ucuna yakın güncelleme düğmesi, Nexus id'si olan her modu
denetler. Mod sayfası kurulu sürümü gösteriyorsa başka bir şey sorulmaz. Aksi
halde Eidos modun dosya listesini okur (bir istek daha, denetimin geri kalanı için
saklanır) ve modun `meta.ini` dosyasının kaydettiği dosya kimliğinden başlayarak
güncelleme zincirini izler: yazarlar yeni bir yüklemeyi daha eskisinin ardılı olarak
işaretler ve MO2 aynı zinciri okur. O zincirdeki daha yeni bir dosya güncellemedir.
Zincir hiçbir ardıl göstermiyorsa ve kurulu dosya hâlâ sayfada listeleniyorsa, sayfa
başka bir sürüm gösterse bile mod güncel sayılır. Sayfanın sürümü yalnızca hiçbir
dosya kimliği kaydedilmemişse, kurulu dosya kaldırılmış ya da arşivlenmişse ya da
Nexus dosya listesini göndermediyse belirleyicidir.

Her mod, örneğin değil, kendi `meta.ini` dosyasının adlandırdığı oyunun
(`gameName`) Nexus alan adını kullanır: güncelleme denetimi, Endorse, Track ve Visit
on Nexus hepsi oraya gider. Enderal SE ayrıca `skyrimspecialedition` modlarını kabul
eder; bu yüzden bir Enderal SE örneğine kurulan bir Skyrim SE modu Skyrim SE
sayfasında denetlenir. Bir Mod Manager Download bağlantısı, oyunu bağlantının alan
adını kabul ediyorsa, en son kullandığınız örneğe gider. Aksi halde alan adı
kendisine ait olan oyunun bir örneğine gider; onu yalnızca kabul eden bir oyunun
örneğinden önce. `EIDOS_INSTANCE=<folder>` ikisini de geçersiz kılar.

Bir modun sağ tık menüsündeki Track ve Untrack, modu Nexus'ta takip ettiğiniz
modlara ekler ya da oradan çıkarır; böylece sitenin bildirimleri burada takip
ettiklerinizle örtüşür. Aynı menüdeki Endorse ve Abstain, modu Nexus'ta onaylar ya
da onayı geri çeker; güncelleme düğmesinin yanındaki endorse düğmesi seçili modu
onaylar. Her güncelleme denetimi ayrıca Nexus'tan onaylarınızı (endorsement) ve
takip ettiğiniz modları okur (iki istek) ve her modun Endorsed ve Tracked
işaretlerini, modun kendi Nexus alan adında, bunlara göre ayarlar. İkisi
uyuşmadığında Nexus kazanır. Listelerden biri okunamazsa denetim bunu bildirir ve o
işaretleri olduğu gibi bırakır.

"Information..." içindeki Nexus files sekmesindeki **Update** düğmesi daha yeni
dosyayı kurar. Premium bir hesapla Eidos dosyayı kendisi indirir. Ücretsiz bir
hesapla dosyanın sayfasını açar; orada Mod Manager Download dosyayı geri gönderir ve
arşiv `downloads/` klasörüne vardığında kurulum başlar. Oradan sonrası alışılmış
kurulumdur: bir FOMOD, modun kurulduğu cevaplarla açılır, ardından soru penceresi,
son pencerede ne seçmiş olursanız olun, "Keep a backup of the existing mod" işaretli
olarak Replace seçeneğini sunar. Replace, modun önceliğini ve etkin olup olmadığını
korur.

Bir Replace modun klasörünü değiştirmeden önce Eidos eski klasörü yenisiyle
dosya adı ve boyutuna göre karşılaştırır ve ikisindeki eklentilerin başlıklarını
okur. Bu, Update, bir koleksiyon güncellemesi ve kurulum sorusunda seçilen
Replace için geçerlidir. Durum satırı ardından eklenen, silinen ve boyutu
değişen dosyaları sayar, yeni sürümün gerek duyduğu ama eskisinin duymadığı her
master'ı adıyla anar. "Information..." penceresindeki **General** sekmesi bir
**Last update** bölümüyle biter; bölümde sürümler, eklenen veya silinen
eklentiler, eklenen veya bırakılan master'lar, ESM, ESL ya da ESH bayrağı
değişen eklentiler, bu modun artık dosya paylaştığı veya paylaşmadığı modlar ve
dosyalar listelenir. Aynı boyutla yeniden yazılan dosya listelenmez. Rapor,
örnek içindeki `logs/updates/<mod>.json` dosyasında tutulur; `eidos pack` bunu
dışarıda bırakır. Yeni bir kurulum ya da klasöre Merge raporu siler. `eidos
install --replace` aynı sayıları `changes:` satırında yazdırır.

Birkaç Nexus dosyası barındıran bir klasör, örneğin içine isteğe bağlı bir dosya
karıştırılmış bir ana dosya, reddedilir: onu yeni ana dosyayla değiştirmek
ötekileri kaybettirir. Bunları elle güncelleyin.

`eidos nexus update skyrimse --apply` aynı denetimi yapar, ardından premium bir
hesapta her yeni dosyayı indirip `--replace --backup` ile klasörünün üzerine
kurar. Ücretsiz bir hesapta bunun yerine her dosya sayfası için `OPEN <url>`
yazdırır. Birkaç dosyayı birleştiren klasörler gerekçesiyle atlanır. Kayıtlı
cevabı olmayan bir FOMOD modu, örneğin Eidos cevapları saklamaya başlamadan önce
kurulmuş olan, yükleyicinin varsayılanlarıyla yeniden kurulmaz: o güncelleme,
modu bir kez pencereden güncellemenizi isteyen bir mesajla başarısız olur;
pencere cevapları kaydeder. Yükleyicisinde seçilecek bir şey olmayan bir FOMOD
modu her zamanki gibi kurulur. Bir güncelleme başarısız olduysa komut 1 ile
çıkar.

### Koleksiyon güncellemeleri

Check for updates çalıştırdığınızda Eidos, Nexus'a bu örnekte kurulu her koleksiyonun en son revizyonunu da sorar. Mod listesinin üstündeki bir satır, daha yeni revizyonu olan her koleksiyonu adıyla verir ve eklenen (+), güncellenen (~) ve çıkarılan (-) mod sayısını gösterir; adlar için üzerine gelin. Kurulu revizyonun tarifi artık önbellekte değilse sayılar görünmez. **Update**, yeni revizyonu koleksiyon panelinde açar; orada *Install this collection* onu çalıştırır. Liste her denetimde yeniden kurulur ve kaydedilmez.

### Nexus ayarları

**Preferences -> Nexus -> Offline**, Eidos'un Nexus'la hiç bağlantı kurmasını
durdurur ve GitHub'daki günlük sürüm denetimini atlar. Güncelleme denetimleri,
oturum açma, indirmeler ve koleksiyonlar bir bağlantı hatasıyla başarısız olmak
yerine bunu söyler. Öntanımlı olarak kapalıdır.

**Preferred servers**, bir indirmenin yeğlediği CDN düğümlerini en iyisi başta
olacak biçimde sıralar. Aralarından seçim yapılacak birden çok yansı yalnızca
premium bir hesaba verilir, dolayısıyla diğer herkes için seçimi Nexus yapar ve
bu hiçbir şeyi değiştirmez. Adını verdiğiniz sunuculardan hiçbiri sunulmuyorsa
indirme yine de olur, Nexus'un ilk sunduğu düğümden.

### Kategoriler

Kategoriler tek bir moda ya da bütün bir seçime atanabilir, katalog aynı iletişim
kutusundan düzenlenebilir ve oyunun resmî kategori listesi Nexus'tan çekilebilir.
İki katalog dosyası da MO2'nin kendisidir (`categories.dat` ve `nexuscatmap.dat`),
yani paylaşılan bir örnek tek bir katalog tutar. Nexus'tan indirilen bir mod,
Fetch from Nexus kataloğu eşleştirdikten sonra kurulum sırasında kategorilenir.

### INI düzenleyici, günlük ve eklentiler

**View -> INI editor**, profilin oyun INI'lerini düzenler: Eidos'un tuttuğu kopyayı;
Proton önekindeki kopya her başlatmada üzerine yazılır. Son sekmesi profilin
`launch-env.ini` dosyasıdır
([Profil başına değişkenler ve wrapper'lar](#profil-başına-değişkenler-ve-wrapperlar)).
**View -> Log**, oturum günlüklerini okur. **View -> Extensions**, kendi
eklentilerinizi listeler; bkz. [extensions.md](extensions.md).

Bir başlatma oyunun ana INI dosyasını (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...) ne profilde ne de önekte bulamazsa, Eidos önce oyun klasöründeki varsayılan dosyayı (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) MO2'nin yaptığı gibi profile kopyalar. Örnek bir oyun kopyası çalıştırıyorsa varsayılan kopyadan gelir. Önek ve oyun klasörü değiştirilmez.

### Mod kurmak

Kurucu Simple arşivleri, FOMOD'u, Wrye Bash BAIN paketlerini (sırayla katılan alt
paketleri işaretleyin) ve Oblivion'un OMOD paketlerini işler. Bir OMOD betiği,
Eidos'un desteklediği OBMM komutlarını kullandığında ya da Eidos'un bildiği üç C#
kurucusundan biri olduğunda çalışır; Eidos başka hiçbir C# ya da Visual Basic kodu
çalıştırmaz ve bunu söyler. Hiçbir sezgisel yöntem yerleşimi tanımadığında, arşiv
ağacını gösteren ve veri kökünü işaret etmenizi sağlayan elle bir seçici açılır.

Bir FOMOD modunu yeniden kurmak ya da güncellemek, sihirbazı son kurulumun
cevapları işaretli olarak, o zamandan beri seçenekleri değişen ilk adımda ve
değişikliklerin listesiyle açar. **Reinstall with new choices** bunun yerine
yükleyicinin varsayılanlarından başlar. `eidos install --replace` ya da
`--merge` kayıtlı cevapları yeniden kullanır; cevaplar artık uymuyorsa
değişiklikleri listeleyip kurmayı reddeder.

### Diagnostics

Diagnostics sekmesi canlı sağlık denetimleri çalıştırır. Eksik master'ları, LOOT'un
kirli işaretlediği eklentileri, hiçbir etkin eklentinin yüklemeyeceği arşivleri, mod
listesinin hâlâ mods klasörüyle eşleşip eşleşmediğini, FUSE passthrough'un açık olup
olmadığını ve istenmişse onun gerektirdiği başlatma yeteneğinin orada olup olmadığını
bildirir. Bir çalıştırmadan sonra, script extender'ın kendi günlüğünün eklenti
DLL'lerinin her biri hakkında ne söylediğini gösterir; böylece SKSE eklentilerinizin
yüklenip yüklenmediğini görebilirsiniz. Ayrıca son temiz başlatmadan bu yana bir oyun
güncellemesini, eksik ya da oyun sürümüyle uyuşmayan bir script extender'ı
(extender'ın indirme sayfası ve indirdiğiniz arşivi kuran bir düğmeyle), Eidos'u
`%command%` ile başlatmayan bir Steam başlatma seçeneğini, DLSS için kapalı bırakılmış
NVAPI'yi, kare üretimini durduran ayrıcalıklı tam ekranı, `LinuxVersion=true`
içermeyen bir ENB `enblocal.ini` dosyasını, F4SE ve SFSE eklentilerinin eklenti
bildirimlerini ve son oturumun çökme günlüğünü, adını verdiği modlarla birlikte
bildirir.

Profil INI'leri ve eklentiler için denetimler, Fallout 4, Fallout 4 VR ve Starfield
özel INI'lerinin `[General]` bölümündeki, oyunun `plugins.txt` dosyasını yok saymasına yol açan
`sTestFile` satırlarına bakar (Fix bunları profilin kopyasından kaldırır ve önceki
dosyayı `<ini>.pre-fix` olarak tutar); sTestFile denetimi yalnızca Fallout 4,
Fallout 4 VR ve Starfield için geçerlidir. Ayrıca Starfield'daki etkin `.esp`
eklentilerine ve blueprint bayrağı, `BlueprintShips-` öneki ya da `.esm` uzantısı
eksik olan veya ana eklentisi etkin olmayan blueprint eklentilerine bakar. Sistem
denetimleri, Proton'un gerek duyduğundan düşük bir `vm.max_map_count` ya da sabit açık
dosya sınırını, ve NTFS, exFAT ya da FAT üzerindeki bir oyunu, öneki ya da örneği
işaretler. Mod denetimleri, BodySlide'ın veri yolu oyunun `Data` klasörünü
göstermiyorsa onu, bir aracın `Overwrite/Root` içine yazdığı `Data` klasörlerini,
serbest dosyaları yamaladığı modun karşısında kaybeden bir yamayı ve script
extender'ın Papyrus betiklerini değiştiren bir modu adlandırır.

Güncelleme denetimi her modun Nexus gereksinimlerini de okur. Diagnostics, eksik veya devre dışı bir gerekli modu listeler; başka yoldan karşılanan bir gereksinim için Ignore vardır.

Sekme ayrıca oyunu hangi Proton sürümünün çalıştırdığını gösterir ve daha yeni bir Proton'un hazırladığı bir öneki daha eski bir Proton çalıştırmadan önce uyarır; Rebuild bozuk bir öneki bir yana taşır, böylece Steam yeni bir tane oluşturur ([troubleshooting.md](troubleshooting.md#oyunu-hangi-proton-çalıştırıyor-ve-bozuk-bir-önek)). Diğer kartlar hiç kurulmamış ya da kayıt defteri eskimiş bir öneki, Mod Manager Download bağlantılarını Eidos dışında bir programın almasını (Register düzeltir), girdilerinden eski araç çıktısını (aracı yeniden çalıştıran bir düğmeyle) ve orijinali sonradan değişmiş bir eklentinin temizlenmiş ya da light kopyasını işaretler. Her kart [troubleshooting.md](troubleshooting.md#health-kartları) içinde açıklanır. **Copy report**, sistemin ve her bulgunun bir raporunu, ev klasörünüz ve oturum adınız gizlenmiş olarak, bir hata raporu için panoya koyar.

Skyrim SE, Enderal SE ve Fallout 4 için Diagnostics'te bir de Display kartı vardır. Profilin `SkyrimPrefs.ini` dosyasında (`EnderalPrefs.ini`, `Fallout4Prefs.ini`) ayarlı pencere boyutunu ve kipini, profilde bu dosya henüz yoksa önekteki kopyadakini gösterir. İkisi de bir boyut belirtmiyorsa alanlar `/sys/class/drm` içinde listelenen en büyük bağlı ekranla dolar, yani Steam Deck 1280x800 gösterir. Genişlik ve yükseklik 320 ile 16384 arasında tam sayılardır. Apply `iSize W`, `iSize H`, `bFull Screen` ve `bBorderless` değerlerini profilin kopyasına yazar; profilde kopya yoksa önce önekteki kopyayı alır. Pencereyi SSE Display Tweaks belirliyorsa Apply aynı boyutu ve kipi onun Overwrite'taki kopyasına da yazar; modun kendi dosyası değiştirilmez.

Sekmenin en üstündeki Game files kartı [Oyun dosyaları](#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya) altında, koleksiyon sırası kartı [Koleksiyon sırasını korumak](#koleksiyon-sırasını-korumak) altında, Find the mod behind a problem ise [Test çalıştırmaları](#bir-sorunun-arkasındaki-modu-bulmak) altında anlatılır.

### Oyunu başlatmak

Oyunu pencere üzerinden başlatmak için oyunun Steam başlatma seçeneğini programın
mutlak yoluna ayarlayın, çünkü Steam kabuğunuzun `PATH`'ini okumaz
([install.md](install.md#sonra-steami-ona-yöneltin)):

```
~/.local/bin/eidos-gui %command%
```

Colony'den kurulduysa yol `~/.local/share/Colony/apps/Eidos/eidos-linux` olur.
Eidos o oyunun örneğinde açılır, yani en son kullandığınızda; böylece taşınabilir
bir örnek de global olan gibi yeniden bulunur. Birleşik görünüm üzerinden
başlatmak için Run'a tıklayın.

**Start the game at once when a store opens Eidos** açıkken (Preferences, General, Running a game; öntanımlı olarak kapalı) Eidos bu tıklamayı atlar: Health'i denetler ve oyunu penceresini göstermeden başlatır. Health bir sorun listelediğinde, mod listesi ilgi gerektirdiğinde, oynamadan önce çalışacak şekilde ayarlanmış bir aracın çıktısı eskidiğinde ya da Skyrim SE, Enderal SE ve Fallout 4'te profilin henüz bir pencere boyutu olmadığında pencere her zamanki gibi açılır (son durumda Diagnostics sekmesinde açılır). Pencereyi tek bir başlatma için isterseniz komutun önüne `EIDOS_WINDOW=1` ekleyin: `EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

Bir mağazanın açmadığı bir pencerede Run oyunu kendisi başlatmaz ve Eidos hiçbir
oyunu doğrudan Proton ya da Wine ile başlatmaz. Run, kopyanın sahibi olan
mağazadan onu başlatmasını ister ve mağaza Eidos'u başlatır:

- Steam, yukarıdaki başlatma seçeneği ayarlıysa ve oyunun, Steam'in oyunu ilk
  başlattığında oluşturduğu bir Proton öneki varsa `steam -applaunch <appid>`
  çalıştırır. Steam kapalıysa önce Steam açılır.
- Heroic (GOG ya da Epic kopyası, yerel ya da Flatpak), Eidos oyunun Heroic'teki
  ayarlarında, çalıştırılabilir dosya olarak `eidos-gui`'nin (ya da
  `eidos-linux`'un) mutlak yolu ve hiç argüman olmadan bir wrapper olduğunda bir
  `heroic://launch` bağlantısı alır. Heroic kapalıysa önce Heroic açılır.
- Tek başına Legendary, `~/.config/legendary/config.ini` dosyasında oyunun
  `[<app>]` bölümünde ya da `[default]` altında `wrapper =` ve `eidos-gui`'nin (ya da
  `eidos-linux`'un) mutlak yolu varsa `legendary launch <app>` çalıştırır.

Script extender'ın yükleyicisi, başlatıcı ve oyunun çalıştırılabilir dosyası Run
listesinde bu şekilde mağazaya devredilir; bir terminalde bunlardan biri için
`eidos tool <instance> run <entry>` de öyle. Eidos bazı kopyaları mağazalarına
devredemez, örneğin Heroic dışındaki bir GOG kopyasını: Run bu durumda oyunu o
mağazadan başlatmanızı söyler.

Devretme, bu örneği en son kullandığınız örnek olarak kaydeder ve mağaza sonra onun
üzerinde ikinci bir Eidos penceresi açar. Start the game at once kapalıyken orada Run'a
tıklarsınız; açıkken o pencere, yukarıdaki durumlardan biri onu açmadıkça, kendini
göstermeden oyunu başlatır. İlk pencere o çalıştırmayı izlemez; listelerini yeniden
yüklemek için oynadıktan sonra orada F5'e basın. Mağazadaki ayar eksikse ya da script
extender'ın yükleyicisi oyunun çalıştırılabilir dosyasının yanında değilse, Run hiçbir
şey başlatmaz ve neyin nereye ayarlanacağını, çalışan ikili dosyanın gerçek yoluyla
gösterir.

Bir oyun ya da araç çalışırken Run, Stop olur. İki basış, çalıştırmanın başlattığı
her programı sonlandırır ve Eidos sonra, her çıkıştan sonra olduğu gibi, oturumu
yakalar ve bağlantıyı keser; durdurulmuş bir çalıştırma hiçbir zaman temiz başlatma
sayılmaz
([troubleshooting.md](troubleshooting.md#kapanmayan-bir-oyun-ya-da-araç)).

Bethesda oyunları için Steam'in `%command%` değeri genellikle
`<Game>Launcher.exe`'yi gösterir. Eidos onu asla çalıştırmaz: başlatıcı, `Data`'yı
yeniden tarayan ve `plugins.txt`'yi yeniden yazan ayrı bir ayar uygulamasıdır;
böylece az önce yerleştirilen yükleme sırasını geri alır. Eidos onun yerine,
kuruluysa script extender'ın yükleyicisini, değilse oyun ikili dosyasını çalıştırır
ve oyun ikili dosyasına geri düşmek zorunda kaldığında bunu söyler.

Eidos etkin modların DLL'lerinde bir `d3dcompiler_47.dll` içe aktarımı arar,
Microsoft'un DLL'ini öneke kopyalar ve ancak ondan sonra
`WINEDLLOVERRIDES=d3dcompiler_47=n,b` ayarlar; bunu kendiniz ayarlamanız gerekmez.

### Profil başına değişkenler ve wrapper'lar

Bir profil, kendi ortam değişkenlerini ve wrapper komutlarını `profiles/<ad>/launch-env.ini` içinde, satır başına bir girdi olacak şekilde taşıyabilir:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Bu profilin her `eidos play` çalıştırmasında (Steam'in `%command%` değeriyle ya da bir mağazanın açtığı pencerede Run ile başlatılan oyun) ve her araç çalıştırmasında geçerlidir. `eidos shell` değişkenleri alır, wrapper almaz. İlk `wrapper=` satırı en dıştakidir. `%command%` içeren bir `wrapper=` satırında komut o yere konur, Steam'in başlatma seçeneklerindeki gibi; içermeyen satırda komut sona eklenir. Wrapper'lar Eidos'un özel ad alanında çalışır, bu yüzden modları görür.

Buradaki bir değişken, Eidos'un aynı ad için ayarladığı değerin yerine geçer; bir aracın `tools.ini` içindeki kendi `env=` satırı ise ikisinin de yerine geçer. `WINEDLLOVERRIDES` ise Eidos'un ENB, ReShade ve birlikte gelen DLL'ler için ayarladığı override'lardan sonra eklenir: ikisinde de adı geçen bir DLL profilin ayarını alır, diğerleri kendi ayarını korur.

View -> INI editor dosyayı her oyun için son sekme olarak açar. Boş satırlar ve `#` ya da `;` ile başlayan satırlar atlanır. Kaydederken Eidos'un yok saydığı satır numaralarını listeler: ne `env=AD=değer` ne de `wrapper=komut` olan satırlar ve ilk sözcüğü, Steam'in başlatma seçeneklerinin izin verdiği gibi, `AD=değer` olan bir `wrapper=` satırı; o değişkeni bunun yerine bir `env=` satırına koyun. Komutsuz `eidos play <instance>`, bir çalıştırmanın alacağı değişkenleri ve wrapper'ları listeler. Diagnostics içindeki Copy report değişkenleri ve wrapper programlarını değerleri olmadan adlandırır.

## Araçlar

xEdit, BodySlide, DynDOLOD ve diğer araçlar, birleşik görünüm üzerinden oyunun Wine
önekinin içinde çalışır (Steam'in Proton öneki ya da bir GOG veya Epic kopyası için
Heroic'in kurduğu önek):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # bir exe dosyasını kaydetmeden bir kez çalıştır
eidos prereqs skyrimse            # kayıtlı araçların neye ihtiyaç duyduğu ve durumu
eidos prereqs skyrimse --install  # eksik olan ne varsa getir
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen ve Synthesis
kaydedilmeden bulunur. Pandora, Nemesis, FNIS, BodySlide ve Synthesis yakalama modu
hazır ayarlı gelir; bu yüzden çıktıları kendi modlarına gider (ayrıntılar
[tools.md](tools.md#bir-aracın-çıktısını-kendi-moduna-göndermek) içinde).

Bir araca verdiğiniz başlık, Eidos'un onun için hangi çalışma zamanlarını ve DLL'leri
sağlayacağına karar verir: `BodySlide` DirectX kitaplıklarını alır, `BS` hiçbir şey
almaz. Pencerede Executables iletişim kutusu her ön koşulun gerçek durumunu alanın
altında gösterir ve eksik olanlar birer düğmedir.

Sürüm paketi, `runtimes/` içinde vcrun2022 kurucularını (x86 ve x64), .NET 8 Desktop
Runtime 8.0.12'yi ve .NET 10 çalışma zamanı 10.0.10'u taşır ve `install.sh` onları
`~/.local/share/Colony/Eidos/runtimes/` içine kopyalar; böylece
`eidos prereqs <instance> --install` onları indirmeden kurar. Eidos Microsoft'un
vcrun2022 ve .NET 8 kurucularını oyunun önekinde winetricks olmadan sessizce
çalıştırır. `eidos-linux` ve kaynaktan derlemeler `runtimes/` klasörü taşımaz; bu
yüzden aynı sabitlenmiş dosyaları Microsoft'tan indirir. Synthesis'in istediği .NET
10 SDK (yaklaşık 300 MB) her zaman indirilir.

Tablo, üç ön koşul kademesi, DynDOLOD'un neden winetricks'in kuramadığı bir .NET
çalışma zamanı istediği ve bir mod olarak kurulan bir aracın neden kendi
klasöründen değil birleşik yoldan başlatıldığı [tools.md](tools.md) içinde.

## Eklentiler

Eidos yeniden derlenmeden genişletilebilir: `~/.config/Colony/Eidos/addons/`
içindeki bir TOML bildirimi, Extensions listesine bir araç ya da Diagnostics
sekmesine bir denetim ekler. Eidos'un içine hiçbir şey yüklenmez: bir eklenti, onun
çalıştırdığı bir programdır. Bkz. [extensions.md](extensions.md).

## Kavram kanıtını deneyin

Betik oyun gerektirmez. Birleşik görünümü, copy-on-write'ı, dokunulmamış bir oyun
klasörünü ve yalnızca kendi ad alanının içinde var olan bir birleşimi, bir kullanıcı
ad alanındaki yalnızca ayrıcalıksız OverlayFS'i kullanarak gösterir (Linux 5.11 ya da
daha yenisi):

```sh
./scripts/poc-overlay.sh
```
