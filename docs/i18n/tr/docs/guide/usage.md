<!-- eidos-i18n: source=docs/guide/usage.md sha=25a1ec0af89508cf1c8d9ee3a8851ab91cf0d6af -->

# Eidos kullanımı

Uygulamalı el kitabı: komut satırı, GUI, Steam başlatma seçeneği, kaynaktan
derleme ve kavram kanıtı betiği. Bir şey ters göründüğünde ne yapılacağı için
bkz. [troubleshooting.tr.md](troubleshooting.md).

## Kullanın (CLI)

```sh
eidos games                       # burada kurulu desteklenen oyunlar (MO2'nin listesi gibi)
eidos init skyrimse               # bir modlama örneği oluştur
# ...her modu bir klasör olarak <instance>/mods/ içine bırakın (global örnek
#    ~/.local/share/Colony/Eidos/instances/skyrimse konumundadır; `eidos init` sizinkini yazar)...
eidos install skyrimse mod.7z     # ya da indirilmiş bir arşivi kur (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # var olan bir MO2 profilinin sırasını + eklenti durumunu devral
eidos sort skyrimse               # eklenti yükleme sırasını LOOT ile sırala
eidos play skyrimse               # neyin bağlanacağını göster
eidos play skyrimse -- <command>  # <command>'ı modlar oyunun üzerine bağlanmış halde çalıştır
eidos check skyrimse              # pencere olmadan sağlık denetimleri (betikler için --json)
eidos sessions skyrimse           # kaydedilmiş oyun ve araç çalıştırmaları, en yeni başta
eidos sessions skyrimse diff      # çıkış kodu 0 ile biten son çalıştırmadan beri neyin değiştiği
eidos collection skyrimse <link>  # bir Nexus koleksiyonunu yazarının kurduğu gibi kur
eidos nexus update skyrimse --apply  # sizinkilerin yerini alan Nexus dosyalarını kur
eidos pack skyrimse backup.eidos  # bütün örnek tek bir dosyada, başka yere taşımak için
eidos unpack backup.eidos <folder>   # öteki makinede geri koy
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` ve `eidos export`
takımı tamamlar; tam liste için `eidos`'u argümansız çalıştırın.

### Terminalden sağlık denetimleri

`eidos check <instance>`, **Diagnostics** sekmesindeki prefix, eklenti, eksik master ve script extender denetimlerini pencereyi açmadan çalıştırır; böylece bunlar SSH üzerinden ya da Game Mode'da çalışabilir ve bir hata raporuna eklenebilir. **Copy report** ile aynı sistem satırlarını, ardından her bulgu için bir satır yazdırır; ev klasörünüz `~`, oturum adınız `<user>` olarak gösterilir. Çıkış kodu, hiçbir sorun bulunmadığında 0 (yalnızca öneriler yine 0 verir), en az bir sorun bulunduğunda 1, Eidos denetleyemediğinde 2'dir: yanlış argümanlar, açamadığı bir örnek ya da algılamadığı bir oyun.

`eidos check <instance> --json` bunun yerine tek bir JSON nesnesi yazdırır. Üst düzey alanları `eidos` (sürüm), `game`, `instance`, `system` (`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`) ve `findings`'dir. Her bulgu bir `code`, bir `level` (`problem` ya da `advice`) ve kendi `args` alanından oluşur; Diagnostics sekmesi her kodun cümlesini gösterir.

### Örnekler: global ve taşınabilir

Yukarıdaki her komut bir örneğe seslenir. `skyrimse`, **global** olanı
adlandırır - merkezî olarak `~/.local/share/Colony/Eidos/instances/skyrimse` içinde saklanır,
Eidos tarafından yönetilir. Diğer tür **taşınabilir**: istediğiniz yerde (ikinci
bir disk, bir oyun bölümü) kendi kendine yeten bir klasör; taşınabilir ve
yalıtılmış, tam olarak MO2'nin taşınabilir örnekleri gibi. Bir komut nerede bir
oyun kimliği alıyorsa, orada taşınabilir bir örneğin klasörünü de alır:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # orada taşınabilir bir örnek oluştur
eidos install /mnt/games/EidosSkyrim mod.7z  # her komut klasörü kabul eder
eidos play /mnt/games/EidosSkyrim -- %command%
```

Klasör kendini tanımlar (`eidos-instance.ini` dosyası oyunu adlandırır), yani
başka bir şey gerekmez - ve ortamdaki `EIDOS_INSTANCE=<folder>` bir oyun
kimliğini o klasöre yönlendirir; bu da Steam başlatma seçeneklerinde işe yarar.
Oluşturduğunuz ya da açtığınız taşınabilir örnekler
`~/.config/Colony/Eidos/instances.ini` içinde hatırlanır (en son kullanılan
başta); GUI'nin karşılama ekranı onları tek tıkla açmak üzere listeler, Steam
başlatması en son oynadığınızın üzerine iner ve `nxm://` işleyicisi onun içine
indirir. Bilmeye değer iki uyarı: taşınabilir bir klasörü elle taşımak, eski
konuma mutlak yollarla kaydettiğiniz araç girdileri dışında her şeyi korur
(aşağıdaki `eidos pack` / `eidos unpack` bunları sizin için düzeltir; düz bir
`mv` düzeltmez) ve paylaşılan çalışma zamanı önbelleği
(`~/.local/share/Colony/Eidos/runtimes/`) bilerek makine genelinde kalır - 78
MB'lık bir .NET host'u örnek başına olmaz.

Eidos kendi dosyalarını `Colony/Eidos` altında tutar; bu, Colony ailesindeki her
programın kullandığı düzendir: seçtikleriniz için `~/.config/Colony/Eidos/`
(tercihler, Nexus oturumunuz, örnek listeniz, yazdığınız oyun ve eklenti
tanımları), global örnekler (`instances/`), indirilen çalışma zamanları ve
oturum günlükleri (`logs/`) için `~/.local/share/Colony/Eidos/`, eksik
olduklarında yeniden indirilen LOOT ana listeleri için de
`~/.cache/Colony/Eidos/`.

Daha eski bir Eidos bunları `~/.config/eidos/`, `~/.local/share/eidos/` ve
`~/.local/state/` içinde tutuyordu. Yükseltmeden sonraki ilk başlatma onları
karşıya **kopyalar** ve bunu günlükte söyler. Global bir örnek bayt bayt
kopyalanmak yerine sabit bağlantılarla taşınır; böylece onlarca gigabaytlık
modlar bir anda, fazladan yer kaplamadan taşınır ve araç kayıtları yeni klasörü
gösterecek şekilde güncellenir. Eski dizinler tam olarak oldukları gibi
bırakılır - hiçbir şey silinmez, yani kötü bir yükseltme size ne bir oturum
açmaya ne de bir kayıt dosyasına mal olabilir - ve içiniz rahat ettiğinde onları
kendiniz kaldırabilirsiniz. Kopya yapılamazsa (dolu ya da salt okunur bir disk,
başka bir sürücüdeki eski bir örnek klasörü, hâlâ oradan çalışan bir oyun),
Eidos eski konumu kullanmaya devam eder ve bir sonraki başlatmada yeniden dener.
Hâlâ eski örnek klasörünü gösteren bir Steam başlatma seçeneği, taşınmış örneği
açar.
Taşıma yalnızca başta fazladan yer kaplamaz: sonradan kaldırdığınız ya da
yeniden kurduğunuz bir mod veya indirme, eski örnek klasörü üzerinden eski
kopyasını diskte tutar; o yeri geri almak için `~/.local/share/eidos/<game>/`
klasörünü silmeniz gerekir.

Taşınabilir bir örnek asla taşınmaz: onu koyduğunuz yerde kalır.

Bir yer düpedüz reddedilir: **bir oyunun kurulum klasörünün içi** (MO2
kıdemlisinin refleksi). O ağaç Steam'e aittir - bir güncelleme, bir "verify
integrity" ya da bir kaldırma onu yeniden yazabilir ya da silebilir, bütün
kurulumunuzu da beraberinde götürür - ve Eidos oyun kökünün üzerine bağlar, yani
oradaki bir örnek kendi bağlama hedefinin içinde otururdu. Sihirbaz, `eidos init`
ve `eidos play`, üçü de hayır der; klasörü onun yerine oyunun YANINA koyun (aynı
diskteki bir kardeş klasör size aynı kolaylığı verir).

`play`, örneğin modlarını özel bir ad alanının içinde oyunun kendi `Data`
dizininin üzerine bağlar (bir bind-stash aracılığıyla, böylece artalan süreci
hâlâ dokunulmamış dosyaları okur), sonra komutu o görünüm üzerinden çalıştırır.
Yazmalar (kayıtlar, yeniden üretilen yapılandırmalar) örneğin `overwrite/`
katmanına iner; oyun kurulumu ve her mod kaynağı bayt bayt dokunulmamış kalır.

### Wabbajack listeleri

Eidos'ta Wabbajack yükleyicisi yoktur. Listeyi Eidos dışında CLF3 veya Jackify ile kurun, ardından sihirbazda kurulum klasörünü taşınabilir örnek olarak seçin. Eidos klasörü MO2'nin düzeniyle okur: `mods/`, `profiles/<ad>/modlist.txt` ve eklenti listeleri. Yükleyicinin yazdığı `ModOrganizer.ini` dosyasını korur ve hiçbir zaman üzerine yazmaz, böylece listenin kendi çalıştırılabilir dosyaları kayıtlı kalır.

Birçok liste, oyunun değiştirilmiş bir kopyasını içeren bir `Stock Game` veya `Game Root` klasörü ekler ve MO2 oyunu oradan başlatır. Eidos Steam kurulumunu başlatır ve başka bir oyun klasörü için ayarı yoktur. Sihirbaz böyle bir klasör bulursa bunu özet ekranında bildirir ve başka bir şey yapmaz: listenin oraya koyduklarını kendiniz bir modun `Root/` klasörüne taşıyın. CLF3, BSA ve DDS dosyalarını kendi araçlarıyla yeniden oluşturur; bu yüzden sonuç bir Wabbajack kurulumundan farklı olabilir ve liste yazarları bunu desteklemez.

### Mod listesi

MO2'de olduğu gibi her profilin kendi `modlist.txt` dosyası vardır (hangi
modların açık olduğu, hangi öncelikte), mod klasörleri ise örneğin paylaşılan
`mods/` dizininde yaşar. `mods/` içine elle bıraktığınız bir klasör henüz
listelenmemiştir: **en yüksek öncelikte ve kapalı** olarak görünür - pencerenin
listesinin en altında, MO2'nin de onu koyduğu yerde - böylece onu açtığınızda
çakışmalarını kazanır. Daha eski sürümler onu en üste koyuyordu; orada diğer her
mod onu geçersiz kılıyordu.

Klasörler paylaşıldığı için bir modu kaldırmak ya da yeniden adlandırmak her
profili değiştirir: satırı her profilin `modlist.txt` dosyasından çıkarılır ya
da orada yeniden adlandırılır ve her profil geri kalanında kendi durumunu ve
sırasını korur. Hiç modu kalmayan bir profilde boş bir `modlist.txt` değil, hiç
`modlist.txt` olmaz; boş olan, hasarlı bir dosya gibi okunuyor ve sonraki
kurulumu engelliyordu.

Pencere, mod listesinin ve yükleme sırasının kendi kopyası üzerinden çalışır.
Pencere açıkken başka bir Eidos süreci bunlardan birini yazarsa - bir terminalde
`eidos install` ya da `eidos sort`, bir koleksiyon, Steam'in başlattığı bir
oturum, ikinci bir pencere - pencerenin sonraki düzenlemesi o işin üzerine
yazmak yerine reddedilir. Pencere diskten yeniden yükler ve *Not saved: another
Eidos process changed the mod list. It has been reloaded - redo the change.* der
(ya da yükleme sırası için aynısını); yalnızca o tek tıklama yitirilir, yani onu
yeniden yapın. Rename, New mod, Add separator, Remove ve Overwrite'ı bir moda
dönüştürmek, `mods/` dizinine dokunmadan önce denetim yapar; böylece reddedilen
bir düzenleme diskte hiçbir şeyi değiştirmez. INI düzenleyicisi de aynı biçimde,
açtığından beri değişmiş bir dosyanın üzerine kaydetmeyi reddeder.

### Kayıtlar profile aittir

Her profil kendi kayıtlarını `profiles/<name>/saves/` içinde tutar ve bir
çalıştırma boyunca o klasör, Proton önekindeki oyunun kayıt klasörünün üzerine
bağlanır. Bir profilin ilk başlatılması, önekte zaten bulunan kayıtları onun
içine bir kez kopyalar; böylece sonradan profilden sildiğiniz bir kayıt silinmiş
kalır. Her çalıştırmadan sonra profilin en yeni kayıtları Steam Cloud için öneke
geri kopyalanır; önekte Eidos'un oraya koymadığı bir kayıt önce profilde
`orphan-<time>-<name>` olarak saklanır, asla üzerine yazılmaz.

O ilk kopyadan sonra Eidos'tan geçmeden öneke inen bir kayıt - bir Steam
Deck'ten gelen bir Steam Cloud indirmesi, Eidos olmadan yapılan bir başlatma -
profilin kayıtları bağlıyken gizli kalır. Hiçbir şey onu silmez: başlatma, o
dosya kümesi her değiştiğinde bir kez, onu çalıştırmanın günlüğünde adlandırır
(pencereden bir başlatma için örneğin `logs/run-*.log` dosyası, Steam'den bir
başlatma için Steam'in konsolu). Onu oynamak için kaydı ve ona eşlik eden
co-save dosyasını profilin `saves/` klasörüne kopyalayın.

### Son iyi çalıştırmadan beri neyin değiştiği

`eidos play` veya `eidos tool run` ile yapılan her çalıştırma, pencereden, terminalden ya da Steam'den başlasın, `<instance>/logs/sessions/` altına bir kayıt yazar: profil, etkin her mod ile sürümü ve Nexus dosya kimlikleri, eklenti sırası, her profil INI dosyasının sağlama toplamı, oyun yürütülebilir dosyasının sürümü, Eidos sürümü, çıkış kodu ve çalıştırmanın oluşturduğu kayıtlar. En yeni 50 kayıt tutulur ve `logs/` paketlenen bir örneğe girmez. `eidos sessions <instance> diff`, çıkış kodu 0 ile biten en yeni oyun çalıştırmasını mevcut durumla karşılaştırır; `diff <id>` listeden başka bir kaydı seçer. Eidos burada henüz çökmeleri algılamaz ve Proton altında çöken bir oyun yine de 0 koduyla çıkabilir: seçilen çalıştırma böyleyse `diff <id>` ile daha eskisini belirtin.

### Ayrıcalıklı adım gerekmez

Eidos tamamen rootsuz çalışır. Özel bir kullanıcı + bağlama ad alanı içinde
bağlar; yani setuid yardımcısı yok, artalan süreci yok ve verilecek bir şey yok.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` **isteğe bağlıdır** ve tam
olarak tek bir şeyi açar: çekirdek FUSE passthrough'u; daemon'un önceki bir
sürümü onunla oyunu bozduğu ve en olası nedenden kaçınan değişiklik henüz bir
oyunda yeniden ölçülmediği için o, öntanımlı olarak kapalıdır (aşağıda). Bu
yetenekle Eidos, bir kullanıcı ad alanı yerine düz bir bağlama ad alanı alır;
modlar iki durumda da aynı biçimde yerleşir.


Eski `setcap` önerisinin neden ortadan kalktığı - ve FUSE passthrough'un neden
kapalı geldiği - [troubleshooting.tr.md](troubleshooting.md#passthrough-neden-öntanımlı-olarak-kapalı)
içinde anlatılıyor.

## Bir Nexus koleksiyonu kurmak

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run     # onu oku ve ne olacağını söyle
```

Pencerede: bağlantıyı **File -> Open a Nexus collection...** içine yapıştırın ve
*Install this collection*'a basın.

### Bu neden "şu modları indir" değil

Bir koleksiyon bir alışveriş listesi değil, bir TARİFTİR ve tarifin neredeyse
hiçbir yanı Nexus API'sinden görünmez. Kurulum sırası, yazarın her modun betikli
kurucusuna verdiği yanıtlar, bir dosya çakışmasını hangi modun kazandığı, hangi
eklentilerin nereye yükleneceği, ikili yamalar - hepsi koleksiyonun kendi
arşivinin içindeki bir `collection.json` dosyasında yaşar. Aynı modları indirip
öntanımlı seçenekleriyle kurmak başka bir oyun üretir.

Bu yüzden Eidos arşivi indirir, tarifi okur ve onu uygular:

| Koleksiyon şunu der | Eidos şunu yapar |
| --- | --- |
| `phase` | o sırayla kurar, isteğe bağlı üyeler en sona |
| `choices` | yazarın her FOMOD'a verdiği yanıtları yeniden oynatır, oynatamadığında da bunu söyler |
| `modRules` | doğru modun her dosyayı kazanması için mod listesini yeniden sıralar |
| `plugins` / `pluginRules` | LOOT kurallarını userlist'inize katar ve sıralar |
| `INI Tweaks/` | onları kendi başına bir mod olarak kurar |
| `tools` | hangi araçları beklediğini söyler; hiçbir şey oluşturmaz |

Üyeler kurulurken, mod listesinin sonunda kurulum sırasıyla, `modRules` onları
kımıldatmadan önce **açık** duruma getirilir - hiçbir şeyin listelemediği bir
mod, hiçbir şeyin yüklemediği bir moddur ve diskte atıl duran bir koleksiyon,
raporun göremeyeceği tek başarısızlıktır.

Durum **her** üyeden sonra
`<instance>/collections/<slug>-<revision>.state.json` dosyasına yazılır - o
revizyonun klasörünün yanına, asla içine değil, böylece bir koleksiyon kendi
kayıt defterini kendisi taşıyamaz - ve 200 üyenin 147'sinde yarıda kesilen bir
kurulum 147'den devam eder. Geçici bir adla yazılıp yerine yeniden adlandırılır,
yani yarıda kesilme geriye ya eski kaydı ya da yenisini bırakır, asla ikisinden
birinin yarısını değil; bir gün okunamaz hale gelirse Eidos bütün koleksiyonu
sessizce baştan başlatmaktansa durur ve bunu söyler. Aynı komutu yeniden
çalıştırın.

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

Daha eski bir revizyondan kalıp açık kalmayı sürdüren bir artık, kurulumun
yazarın oluşturduğu kurulum sayılmasını engeller; onu devre dışı bıraktığınızda
not kaybolur. `--no-optional` ile atladığınız bir üye devralınmaz ve aynı
biçimde adı geçer. Devralmanın kendisi başarısız olursa koşu *previous revision:
its folders could not be taken over* iletisi ve nedeniyle durur; onu düzeltin ve
komutu yeniden çalıştırın.

### Neyi varmış gibi göstermez

**Ücretsiz bir Nexus hesabı üye modları getiremez.** Nexus, API üzerinden
ücretsiz hesaplar için indirme bağlantısı üretmez; her dosya sitenin kendi "Mod
Manager Download" düğmesini ister. Koleksiyon ARŞİVİ ücretsiz bir hesapta
sorunsuz iner - yani bütün tarifi okuyabilirsiniz - ama her üye, kendiniz
getirmeniz için tam sayfa URL'siyle birlikte raporlanır. Bu bir Nexus iş
kuralıdır ve ne kadar yeniden denerseniz deneyin aşılmaz. Onları sitenin
düğmesiyle getirin ve komutu yeniden çalıştırın: o üyeler her koşuda
`downloads/` içinde aranır, yani koleksiyon siz sağladıkça tamamlanır.

Pencerede ücretsiz bir hesap, koleksiyon panelinde toplu getirme yerine
*Fetch the N missing required files, one by one* düğmesini görür. Bu düğme
eksik zorunlu dosyaların ilkinin Nexus sayfasını açar, o dosyanın `downloads/`
içine gelmesini bekler, sonra bir sonrakini açar. Her dosya için sayfanın
indirme düğmesine yine siz basarsınız. Panel ne kadar ilerlendiğini (*3 of 12*)
*Skip* ve *Stop* ile birlikte gösterir. Onun yerine aynı modun başka bir
dosyası gelirse panel iki dosya kimliğini de yazar ve koleksiyonun belirlediği
dosyayı beklemeyi sürdürür. İsteğe bağlı üyeler buna dahil değildir ve kendi
*Open* düğmelerini korur.

Koleksiyonun tarif ettiği gibi uygulanır: **ikili yamalar** (her biri kayıtlı CRC ile denetlenir), **dosya geçersiz kılmaları** (adı geçen dosyalar MO2'nin yaptığı gibi `.mohidden` ile gizlenir) ve dosyaları koleksiyon arşivinin içinde gelen üyeler (`bundle`). Koleksiyonun elle getirmenizi beklediği üyeler
(`browse`, `manual`), yazarın kendi yönergelerini rapora taşır. Adı zaten sizin
bir modunuzun adı olan bir üye, onun üzerine değil, yanına, boşta bir adla
kurulur ve rapor hangisi olduğunu söyler.

### Rapor

Raporun amacı "kuruldu"nun güvenilmeye değer olmasıdır. Kurucu yanıtlarının
hepsi yeniden oynatılamayan bir üye **kurulmuştur ama koleksiyonun istediği
biçimde değil** - yüzeysel olarak benzediği başarılardan ayrı, kendi satırında;
çünkü diskteki dosyalar yazarınkilerden farklıdır. Koleksiyonda hiçbir şeye
karşılık gelmeyen sıralama kuralları, birbiriyle çelişen kuralların döngüsüne
yakalanan modlar, var olmayan LOOT grupları ve manifest'in bu Eidos sürümünün
anlamadığı her parçası da listelenir.

Alternatifi - böyle şeyleri günlüğe yazıp kurulumu bitmiş saymak - bir
koleksiyonun, kimsenin bulamayacağı nedenlerle yanlış oynamaya başlamasının
yoludur.

## Bir örneği başka bir makineye taşımak

`eidos pack`, bütün bir örneği - her modu, yükleme sırasını, bütün profilleri,
içindeki kayıtlarınızla birlikte Overwrite'ı ve her şeyin kurulduğu arşivleri -
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
bir klasör, 7-Zip'e güvenle verilemeyecek bir ad), arşiv yine de yazılır ve
adlandırılır - elde olması iyidir - ama `eidos pack` **1** ile çıkar ve neyin
eksik olduğunu söyler. Eksik bir yedek bir başarı değildir ve bu, insanların
`&&` işaretinin önüne koyduğu bir komuttur.

Bir `.eidos` dosyası, kendine ait bir ad altındaki bir 7-Zip arşividir; bu bir
kılık değil, bir tercihtir: bir modu kurabilmek için zaten 7-Zip gerekiyor, yani
bu hiçbir bağımlılık eklemez ve Eidos'u bir gün yitirseniz de yedeğiniz
elinizdeki herhangi bir arşivleyicide yine açılır. LZMA2 ile `-mx1` düzeyinde
sıkıştırılır - doku ağırlıklı bir listede özgün boyutun yaklaşık %43'ü, hem de
`-mx9`'un birkaç puan daha kazanmak için harcadığı sürenin küçük bir bölümünde.
Dokular ve mesh'ler zaten sıkıştırılmış biçimlerdir; daha büyük bir sözlüğün
bulabileceği pek bir şey kalmaz. Kendi içeriğinizde başka düşünüyorsanız
`--level` (0, 1, 3, 5, 7, 9) bunu geçersiz kılar, `--no-downloads` ise
`downloads/` dizinini dışarıda bırakır.

Varsayılan düzey 1'de dosyalar **en fazla 16 MB'lık solid bloklara**
yerleştirilir. 70 GB'lık bir yedekten tek bir dosyayı çıkarmak en fazla o
dosyanın bulunduğu bloğu çözer, asla önündeki her şeyi değil; oyun içeriğini
paketlemek de her dosyanın ayrı ayrı sıkıştırıldığı duruma göre 2 ila 5 kat daha
hızlıdır (açma 2 ila 4 kat). Bunun bedeli hasar durumunda ödenir: tek bir bozuk
bayt, bloğundaki bütün dosyaları, yani 16 MB'a kadar komşu dosyayı yanında
götürebilir. Aşağıda anlatılan kurtarma verisi bu durumu karşılar. Diğer bütün
düzeyler non-solid kalır: her dosya kendi akışıdır ve bozuk bir bayt yalnızca o
dosyaya mal olur. Düzey 0 dosyaları sıkıştırmadan saklar. Düzey 3 ve üstünde
LZMA2 zaten 16 MB ve daha büyük parçalarla çalışır; orada solid bloklar
ölçümlerde daha yavaş çıktı: düzey 5'te mesh'leri paketlemek 1,4 kat, dokuları
açmak 6,8 kat daha uzun sürdü.

Paketleme örnek kilidini alır, yani oyun ya da başka bir Eidos o örneğe yazarken
çalışamaz - yedek bir anlık görüntüdür, hareket halinde alınmış bulanık bir kare
değil. Bir `.part` adı altında yazılır, diske boşaltılır ve seçtiğiniz adı
almadan önce `7z t` ile baştan sona geri okunur. Bu denetim her bloğu çözer ve
her sağlama toplamını karşılaştırır; diğer makinedeki bir açma işleminin
yapacağı işin aynısı. Diskin yazmadan hemen sonra geri verdiğini gösterir,
sürücünün onu saklayacağını değil; bunun için aşağıdaki kurtarma verisi vardır.
Başarısız olursa `.part` silinir, sizin verdiğiniz adla hiçbir şey yazılmaz ve
`eidos pack`, 7-Zip'in hasarı bulduğu satırı aktarır. Yarıda kesilen ya da
başarısız olan bir paketleme, tam görünen ama olmayan bir `.eidos` dosyası
yerine açıkça yarım kalmış bir şey bırakır ya da hiçbir şey bırakmaz.

par2cmdline kuruluysa (Arch'ta `sudo pacman -S par2cmdline`, Debian ve Ubuntu'da
`sudo apt install par2`, Fedora'da `sudo dnf install par2cmdline`), `eidos pack`
arşivin yanına PAR2 kurtarma verisi de yazar: `backup.eidos.par2` ve birkaç
`backup.eidos.volNN+MM.par2` dosyası, toplamda arşivin %5'i artı küçük bir
dizin. Bunları `.eidos` dosyasıyla aynı klasörde, yazıldıkları adlarla tutun ve
onunla birlikte kopyalayın: onlar olmadan taşınan bir yedeğin onarılacağı hiçbir
kaynak yoktur, ayrıca kurtarma verisi arşivin adını kaydeder, bu yüzden adı
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
paketler, `eidos pack --dry-run` ise yazılıp yazılmayacaklarını söyler.

### İçinde ne yok ve neden

| Dışarıda kalan | Neden |
| --- | --- |
| Proton öneki | Makineye göre biçimlenmiştir (sürücü eşlemeleri, *bu* dosya sisteminin bir `Z:` görünümü) ve Eidos onu `eidos prereqs <instance> --install` ile bir dakikada yeniden kurar. Onu taşımak, öteki makinenin nasılsa yeniden üretmesi gereken bir şeyi göndermek uğruna arşivi kabaca ikiye katlardı. |
| Oyun | Bir örnek, içinde barındırmadığı bir oyunu modlar. Onu orada Steam'den kurun. |
| Örneğin dışındaki araçlar | xEdit, DynDOLOD ve arkadaşları, kendi kurucuları olan üçüncü taraf ikili dosyalarıdır. Arşivde **adları geçer**, böylece açma işlemi yeni makinede hangilerinin eksik olduğunu tam olarak söyler. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Yedeği alan makinenin kayıtları. Özellikle `prereqs.done`, çalışma zamanı kitaplıklarının orada olmayan bir önekte zaten kurulu olduğunu iddia ederdi. |
| `loot/`, `userlist.yaml` dışında | Eidos'un gerektiğinde yeniden getirdiği bir masterlist önbelleği. Kendi LOOT kurallarınız bir önbellek değildir - onları kimse yeniden getirmez - bu yüzden o tek dosya birlikte gelir. |
| `.base/`, `.base-root/` | Oyunun kendi dosyalarının bir oturum boyunca saklandığı boş bağlama noktaları. |
| Yarım yazılmış dosyalar | Duraklatılmış bir indirme (`*.unfinished`), yolda olan atomik bir yazma (`*.eidos-tmp*`). |
| Sembolik bağlar | 7-Zip birini izler ve neyi gösteriyorsa onu kopyalardı; mutlak bir bağ için bu, yedeğinizin içine yabancı bir ağaç çekmek demektir. Onun yerine bildirilirler ve yedek eksik sayılır: başka bir sürücüyü gösteren sembolik bağ olan bir `mods/` ya da `downloads/` pakete GİRMEZ. Sembolik bağ olan bir `downloads/` dizinini bilerek dışarıda bırakıp yedek almak için `--no-downloads` verin (ya da pencerede *Include downloads/* işaretini kaldırın); kasıtlı bir dışlama eksik sayılmaz. |

Bunların her biri, nedeniyle birlikte arşivin kökündeki `eidos-backup.ini`
dosyasına girer - yani "burada ne yok" sorusunun yanıtı bir tahmin değil,
`cat`'tir. Aynı dosya, örneğin eskiden nerede olduğunu, hangi profilin etkin
olduğunu, açıldığında ne kadar yer tuttuğunu ve yeni makinenin hangi araçlara
gereksinim duyacağını kaydeder.

İndirme kayıtları (`downloads/*.meta`), `url=` satırları **boşaltılmış** olarak
girer. O URL imzalı bir Nexus bağlantısıdır: saatler içinde çalışmayı bırakır ve
dosyayı indiren hesabın `user_id`'sini taşır. Bir yedek başkasının eline
verilmek üzere alınır, bu yüzden onu taşımaz. İndirmeyi yeniden bulunabilir
kılan her şey - mod kimliği, dosya kimliği, sürüm - kalır.

### Pencerede

İkisi de **File** menüsündedir: *Pack this instance...* ve
*Unpack a backup...*. Açma işlemi ayrıca karşılama ekranında, örneklerin
listesinin altında da bulunur; çünkü ona en çok gereksinim duyan makine, henüz
hiç örneği olmayandır.

Pack iletişim kutusu, işe girişmeden önce önizler: kaç dosya, ne kadar büyük,
bunun ne kadarı `downloads/`, arşivin kabaca ne tutacağı ve hangi araçların adı
geçtiği halde taşınmadığı. *Write recovery data* kutusu par2 kuruluysa
işaretlidir, kurulu değilse kurulacak paketin adıyla birlikte grileşir. Unpack
iletişim kutusu, dosyayı seçtiğiniz anda yedeğin manifest'ini okur; böylece daha
ona bir klasör vermeden hangi oyunu barındırdığını, ne zaman alındığını, eskiden
nerede olduğunu ve araçlarından hangilerinin burada eksik olduğunu size
söyleyebilir.

İkisi de bir ilerleme çubuğuyla birlikte bir çalışan iş parçacığında koşar,
böylece onlar çalışırken pencere yanıt vermeyi sürdürür ve bitirdiklerinde
çubuk raporun kendisi olur. Paketleme, örnek kilidini bütün koşu boyunca
tutar: örnek okunurken başka hiçbir şey ona dokunamaz, yedeği hareket halinde
alınmış bulanık bir kare değil de bir anlık görüntü yapan da budur.

### Açma işleminin onardıkları

Klasörün düz bir kopyası, her araç düğmesini eski makinedeki bir yolu gösterir
halde bırakırdı. `eidos unpack`, eski örnek kökünü adlandıran değerleri ve
yalnızca onları yeniden yazar: `tools.ini` içindeki `exe`, `workdir` ve numaralı
`arg` anahtarları ile her modun `meta.ini` dosyasındaki `installationFile`.
Hiçbir zaman örneğin içinde olmamış bir araç (`/opt/xedit/SSEEdit.exe`) tam
olarak olduğu gibi bırakılır ve yeniden kurulacaklar arasında listelenir -
başkasının xEdit'inin nereye gittiğini tahmin etmek onarım değil, uydurmadır.
Ayrıca örneğin kendini merkezî mi taşınabilir mi saydığını da düzeltir, böylece
sonraki komutlar onu artık bulunduğu yerde arar.

Açma işlemi şunları reddeder: boş olmayan bir klasör (üzerine yazmak
istiyorsanız `--force` verin), manifest'in kendi rakamı için fazla küçük bir
disk, sizinkinden yeni bir Eidos'un yaptığı bir arşiv ve seçtiğiniz klasörün
*dışına* yazılacak bir girdi barındıran her arşiv.

Sonrasında:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # Proton önekini yeniden kur
eidos play /mnt/games/EidosSkyrim
```

## GUI

```sh
cargo run -p eidos-gui
```

Paylaşılan Colony temalarıyla çizilmiş, MO2 tarzı bir ilk başlatma sihirbazı:
karşılama -> örnek türü (taşınabilir / global) -> oyun -> ad ve konum -> özet ->
oluştur -> ana ekran. Karşılama ekranı ayrıca bilinen her mevcut örneği (global
ve taşınabilir, en son kullanılan başta) tek tıkla açmak üzere listeler - aynı
zamanda örnek değiştirici görevi de görür - ve sihirbazı zaten bir örnek
barındıran bir klasöre yöneltmek, onun üzerine oluşturmak yerine örneği olduğu
gibi DEVRALIR (klasör başka bir oyuna aitse düpedüz reddeder).

İki bölmeli ana pencere de yapıldı: bir profil seçici (değiştirin ya da geçerli
olanı kopyalayarak yeni bir tane oluşturun), süzdüğünüz, seçtiğiniz, yeniden
sıraladığınız, ayraçlarla grupladığınız, kategoriye göre daralttığınız ve
eylemler için sağ tıkladığınız bir mod listesi, artı Data / Plugins / Conflicts /
Overwrite / Saves / Downloads / Diagnostics sekmeleri ve çalıştırma hedefi
seçicisi olan bir Run düğmesi.

Yeniden sıralama yalnızca en üste/en alta göndermek değil: MO2'nin hedefli
taşımaları da burada - çakışan ilk modun üstüne, sonuncusunun altına, açıkça
belirtilmiş bir önceliğe ya da bir ayracın grubunun içine gönderin. Hepsi tek bir
paylaşılan taşıma yardımcısından geçer, böylece satırları yeniden eklemeden önce
kaldırmaktan doğan bir-fark hatası beş yerde değil tek yerde bulunur.

### Tercihler

İki bölmenin yerine Tercihler'i açmak için sol üstteki **Eidos**'a tıklayın;
geri dönmek için ona yeniden (ya da **Close**'a) tıklayın. Her Colony
programında olduğu gibi önce General, Appearance ve Accessibility gelir, ardından
Mod list, Nexus ve About. Her değişiklik anında uygulanır ve kaydedilir; Save
düğmesi yoktur.

- **General**: Eidos'un açtığı oyun, pencere boyutu, arayüz dili, araçlar
  klasörü ve bir oyunun nasıl çalıştırıldığı. Pencere İngilizce ve Fransızca
  konuşur; **System** `LC_ALL`, `LC_MESSAGES` ya da `LANG` değerini izler ve
  değişiklik anında uygulanır. `eidos` komut satırı her zaman İngilizce yanıt
  verir.
- **Appearance**: tema (25 aile, 57 palet, varsayılan olarak Eidos'un parşömeni),
  vurgu rengi, metin boyutu, efektler ve canlı bir önizleme.
- **Accessibility**: yüksek karşıtlık, disleksi dostu bir yazı tipi, azaltılmış
  hareket ve Appearance'taki boyutla çarpılan bir okuma metni boyutu.

Bunlar `~/.config/Colony/Eidos/preferences.json` içinde durur. Daha eski bir
Eidos'un `settings.ini` içinde tuttuğu tema ilk seferde aktarılır.

### Sütunlar, sıralama ve gruplama

Liste kutudan çıktığı gibi dört sütun çizer ve sekiz sunar: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Onları View menüsünden
işaretleyin. Öntanımlıda sekizinin birden açık olmaması kasıtlı - her sütunu
gösteren bir listede, asıl okumakta olduğunuz sütun olan AD'a yer kalmaz.

Ona göre sıralamak için herhangi bir başlığa tıklayın. Yeniden tıklamak sırayı
ters çevirir, üçüncü tıklama ise **yükleme sırasına** döner; bu da kulağa
geldiğinden önemlidir: liste yalnızca yükleme sırasındayken sürüklenebilir, çünkü
bir ekleme boşluğu gerçek listeye seslenirken sıralanmış bir satır bambaşka bir
yerdedir. Bir sıralama açıkken ekleme şeritleri çizilmez ve bir sürükleme,
kimsenin nişan almadığı bir yere inmektense reddedilir - MO2'nin yaptığının
aynısı, aynı nedenle. View menüsü bunu söyler ve geri dönüş yolunu sunar.

View menüsü ayrıca bütün listeyi kategoriye ya da kaynağa göre (Nexus'tan ya da
elle kurulmuş) **gruplayabilir**. Grup başlıkları ayraç değildir: arkalarında
yeniden adlandırılacak, renklendirilecek ya da taşınacak bir şey yoktur,
katlanırlar ve katlandıklarında sayı başlıkta kalır. Bir sıralama ya da gruplama
altında ayraçlar listeden çıkar - bir ayraç, yükleme sırasında kendisinden sonra
gelen satırların başındadır ve ikisi de o satırları oynatmıştır.

### Fare ve klavye

Information için bir moda çift tıklayın, klasörü için Ctrl+çift tıklayın, Nexus
sayfası için Shift+çift tıklayın. Ctrl+F imleci süzgeç kutusuna koyar. Bir harfe
basmak onunla başlayan sonraki moda atlar, yeniden basmak ise ilkine takılıp
kalmak yerine geri kalanını dolaşır. Hiçbiri, süzgecin, katlanmış bir ayracın ya
da katlanmış bir grubun gizlediği bir satıra inemez - göremediğiniz bir vurguyu
oynatmak, sonraki Space'in bakmadığınız bir modu açıp kapatmasının yoludur.

Bir ayracın menüsündeki "Collapse others", o grup dışındaki her grubu katlar. Bir
sürükleme sırasında katlanmış bir grubun üzerinde beklemek onu açar, böylece bir
mod önce sürüklemeyi bırakmadan içine bırakılabilir - beklemek, üzerinden geçip
gitmek değil.

Ctrl+Z mod listesindeki ya da yükleme sırasındaki son değişikliği geri alır: bir işaret,
bir taşıma, toplu bir devre dışı bırakma, bir LOOT sıralaması. Ctrl+Shift+Z onu yeniden yapar. Pencere 50 adım tutar ve
bunları F5'te, bir oyun ya da araç çalıştırmasının sonunda, profil değişiminde ve
başka bir Eidos süreci listeyi değiştirdiğinde unutur. Bir modu kurmak, yeniden
adlandırmak ya da kaldırmak diskteki klasörleri değiştirir, bu yüzden bunlardan
birinin ötesine geçen bir geri alma reddedilir.

### Listeyi süzmek

Filters düğmesi her ölçüt için bir satır açar: etkin, çakışmalı, güncelleme var,
kategorisiz, Nexus'ta beğenilmiş (endorsed), takip edilen (tracked), Nexus
id'si olan, gizli dosyası olan, geçerli oyun verisi olmayan, başka bir oyun
için, notu olan ve her içerik türü için bir satır (eklentiler, BSA veya BA2
arşivleri, dokular, mesh'ler, betikler, script extender eklentileri, arayüz
dosyaları, ses, FaceGen). Her tıklama satırı bir sonraki duruma geçirir: kapalı,
"only", "except" ve yeniden kapalı. En üstteki satır, açık ölçütlerin hepsini
karşılayan modlarla en az birini karşılayan modlar arasında geçiş yapar. Ad
kutusu ve kategori listesi sonucu her zaman daha da daraltır. Gizli dosyalar
satırı yalnızca etkin modlara bakar.

### Listenin bir mod hakkında söyledikleri

İki uyarı bayrağı; ikisi de üzerine gelince açıklaması çıkan birer simge. **No
valid game data**, modun tepesinde bu oyunun yüklediği bir şeye benzeyen hiçbir
şey olmadığı anlamına gelir; klasörlerinin bir düzey yukarı taşınması gerekiyor
olabilir ya da bu oyun için bir mod olmayabilir. **Another game**, modun kendi
`meta.ini` dosyasının başka bir oyunu adlandırdığı anlamına gelir. İkisi de
hiçbir şeyi engellemez - mod yine de yerleşir - ve satır menüsündeki "Mark as
valid" her ikisini de MO2'nin kendi `validated=` anahtarı üzerinden susturur,
böylece bir yöneticide kefil olduğunuz bir mod diğerine sessiz gelir.

Yerleşim denetimi bilerek cömerttir: bir `Root/` ağacı sayılır, okunamayan bir
klasör sayılır, boş olan sayılır. Beş yüz satırlık bir listede yanlış bir uyarı,
eksik olandan kötüdür.

### Kayıtlarınızın veya başka modların kullandığı bir modu kapatmak

Bir mod kapatılmadan ya da kaldırılmadan önce Eidos, profilin en yeni üç
kaydının eklenti listelerini okur. Bunlardan biri o moddan bir eklenti
kullanıyorsa, listenin üstündeki bir çubuk eklentilerin adını verir ve
"Disable anyway" ya da "Cancel" bekler (Esc de iptal eder). Remove, yeniden
tıklamayı isteyen iletisine aynı adları ekler. Yalnızca Skyrim ve Fallout 4
kayıtları okunur; diğer oyunlarda mod sormadan kapatılır.

Bir modun Information bölümündeki General sekmesi, o modun ihtiyaç duyduğu modları ve ona ihtiyaç duyan modları, etkin eklentilerin master'larından okuyarak listeler. Başkalarının ihtiyaç duyduğu bir modu kapatmak aynı çubuğu gösterir; "Disable them too" ona ihtiyaç duyan her modu ve onlara ihtiyaç duyan modları da kapatır, "Keep them enabled" ise yalnızca seçileni kapatır. Health sekmesindeki eksik master kartında "Enable what provides them" düğmesi vardır: eksik her master'ı içeren modu etkinleştirir ve mevcut ama işaretsiz bir master eklentisini işaretler.

### Bir moda dokunmadan önce onu yedeklemek

"Back up this mod", klasörünü `<name>_backup` olarak (sonra `_backup2` ve böyle
sürer - bir yedek asla bir öncekinin yerine geçmez) bir yana kopyalar. Kopya
**atıldır**: bir mod değildir, onay kutusu hiçbir şey yapmaz ve birleşik görünüme
hiçbir katkısı olmaz, çünkü onu işaretlemek tek bir modun iki kopyasını üst üste
yerleştirirdi. "Restore this backup over the mod" onu iki tıkla geri koyar;
geçerli içerik önce bir yana taşınır ve ancak kopyalama başarılı olduğunda
atılır.

**Data**, birleşik görünümün gerçek bir ağacıdır; her seferinde bir düzey açılır,
böylece bir düğümü açmak, etkin her modun özyinelemeli olarak taranması yerine, o
düğüme sahip katman başına bir dizin okuması eder. Bağlamanın hizmet ettiği AYNI
katman yığını tarafından yanıtlanır, yani whiteout'lar ve gizli dosyalar gözetilir
ve sekme, oyunun göreceğiyle çelişemez. Ada göre süzün, yalnızca çekişmeli
dosyalara daraltın, Size ve Modified sütunlarıyla neyin nerede olduğunu ayıklayın
ve herhangi bir satırı Reveal ile bir dosya yöneticisinde gösterin. Bir modun en
üst düzeydeki `meta.ini`, `readme.txt`, `fomod/` ve `Root/` öğeleri Data'ya
girmez, ne bu sekmede ne de bağlamada: ilk üçü mod yöneticisi içindir, `Root/`
ise oyun klasörüne gider. Oyunun ya da bir aracın bu adlardan biriyle yazdığı
dosya Overwrite'a düşer ve görünür kalır. **Plugins**,
ESP/ESM/ESL yükleme sırasıdır (açıp kapatın, elle yeniden sıralayın ya da LOOT
ile sıralayıp sıralama sonrası raporu okuyun; oradaki öneri bağlantıları
tarayıcınızda açılır). Başlığı, etkin eklentileri her dizin alanında oyunun sınırına karşı sayar, örneğin `full 231/254, light 512/4096`, ve biri dolunca turuncuya döner. Bir LOOT sıralamasından sonra, yeni kayıtlarının hepsi light aralığına sığan tam bir eklenti bunu fareyle üzerine gelince söyler ve satır menüsünde Flag as light bulunur. Eidos, light bayrağı açık bir kopyayı `<plugin> (light)` adlı yeni bir moda yazar, bu modu mod listesinin en altına (en yüksek öncelik) koyar ve özgün dosyaya dokunmaz; o modu kapatmak işlemi geri alır. Profilin en yeni üç kaydından biri eklentiyi tam eklenti olarak yüklüyorsa Eidos reddeder. Yalnızca Skyrim ve Fallout 4 kayıtları okunur; Starfield'da hiçbir kayıt denetlenmez. LOOT yalnızca kayıt aralıklarını denetler: hücre ya da worldspace ekleyen bir eklentiyi işaretlemeden önce xEdit'te inceleyin. **Conflicts**, dosya başına kazananları ve kaybedenleri
açıklar. **Overwrite**, oyunun yazdığını tek adımda gerçek bir moda dönüştürür.
**Saves**, her kaydın başlığını ayrıştırır - karakter, seviye, konum, oynanma
süresi - ve içine gömülü eklenti listesini geçerli listenizle karşılaştırır;
gerektirdiği modları etkinleştiren bir düğmeyle birlikte, çünkü onları adlandırıp
gerisini size bırakmak işin sıkıcı yarısıdır.

Bir eklentinin üzerine gelince kaynağı, yazarı, başlık ve form sürümü, açıklamasının ilk 300 karakteri, master dosyaları ve LOOT sıralamasından sonra LOOT iletileri görünür.

"Information...", mod başına bir iletişim kutusu açar: genel, çakışmalar, dosya
ağacı, INI ayarlamaları, notlar. Dosya ağacından (ve Data ağacından) herhangi bir
dosya **gizlenebilir** - `<name>.mohidden` olarak yeniden adlandırılır, bu da onu
silmeden sanal görünümün dışına düşürür, böylece bir modun üç başıboş mesh'i
önceliklere dokunmadan bastırılabilir. Dosya ağacı sıradan dosya işlemlerini de
yapar: yeni klasör, yeniden adlandır, sil, aç. Hepsi, o modun içindeki düz bir
yol olmayan her şeyi reddeden tek bir çözücüden geçer - `..` yok, mutlak yol yok
ve sembolik bağ olan bir bileşen yok, çünkü birini izlemek bir silmeyi tümüyle
mod klasörünün dışına çıkarırdı. Yeniden adlandırma yalnızca son bileşeni
değiştirir, yani asla bir taşımaya dönüşemez ve zaten alınmış bir adı, o dosyanın
üzerine sessizce yazmaktansa reddeder. Silme iki tık ister; burada yeniden
tıklamanın geri alamayacağı tek eylem odur.

Dosya ağacındaki ya da Data ağacındaki herhangi bir satırda **View** dosyayı
önizler: görseller ve metin. DDS ya da NIF değil - onlar bu ağacın sahip olmadığı
bir blok çözücü ve bir görüntüleyici ister - ama boş bir kutu göstermek yerine
bunu söyler ve Reveal'ı işaret ederler. Metin 64 KB'a kadar okunur ve nerede
durduğunu söyler, çünkü önizleme bir göz atmadır ve bir Papyrus günlüğü yüz
megabayt olabilir. **INI Tweaks**, bir modun `INI Tweaks/` klasöründe getirdiği
parçaları listeler; etkin olanlar başlatmada, öncelik sırasıyla profilin oyun
INI'sine katılır ve çalıştırmanın INI'leri yakalandığında geri çıkarılır - yoksa
bir ayarlama sessizce bir ayara dönüşür ve onu devre dışı bırakmak hiçbir şey
yapmazdı.

Bir indirme, o öncelikte kurulması için **Downloads listesinden mod listesindeki
bir konuma sürüklenebilir** ve bir dosya yöneticisinden pencereye bırakılan
arşivler ya da klasörler de kurulur (o yarısı bir X11 ya da XWayland oturumu
ister - winit dosya bırakmayı yalnızca X11 için gerçekler). İndirmelerin
kendileri duraklatılıp sürdürülebilir: duraklatmak aktarımı durdurur ve yarım
dosyayı saklar, Resume ise taze bir bağlantıyı yeniden çözer ve durduğu yerden
devam eder.

Downloads sekmesi bir aktarım kuyruğu değil, bir arşiv **kitaplığıdır**. Ada göre
süzün (dost mod adına göre de, yani "skyui", `SkyUI_5_2_SE-12604-5-2SE.7z`
dosyasını bulur), en yeniye, ada, boyuta ya da duruma göre sıralayın ve işiniz
bittiği bir arşivi **gizleyin** - bu, dosyayı korur ve yalnızca satırı düşürür,
yani bir kitabı kaldırıp koymak onu yakmak değildir. "Show hidden" onları geri
getirir, aynı düğme gizlemeyi de kaldırır. "Remove N installed", zaten kurduğunuz
modların arşivlerini iki tıkla siler, hem de yalnızca **ekranda olanları**:
süzgeç, hangilerini kastettiğinizi söyleme biçiminizdir.

### Mod güncellemeleri

Araç çubuğunun sağ ucundaki güncelleme düğmesi kurulu her dosyayı Nexus'ta onun
yerini alan dosyayla karşılaştırır. Yazarlar yeni bir yüklemeyi daha eskisinin
ardılı olarak işaretler; Eidos bu zinciri, modun `meta.ini` dosyasının
kaydettiği dosya kimliğinden başlayarak izler. Zincir hiçbir ardıl göstermiyorsa
bunun yerine mod sayfasındaki sürüm karşılaştırılır.

"Information..." içindeki **Nexus files** sekmesi modun Nexus'taki dosyalarını
en yeniden başlayarak listeler; burada kurulu olanı ve onun yerini alanı
işaretler. Bu ikinci satırda bir **Update** düğmesi vardır. Premium bir hesapla
Eidos dosyayı kendisi indirir. Ücretsiz bir hesapla dosyanın sayfasını açar;
orada Mod Manager Download dosyayı geri gönderir ve arşiv `downloads/` klasörüne
vardığında kurulum başlar. Oradan sonrası alışılmış kurulumdur: bir FOMOD, modun
kurulduğu cevaplarla açılır, ardından soru penceresi "Keep a backup of the
existing mod" işaretli olarak Replace seçeneğini sunar; bir önceki pencerede ne
seçmiş olursanız olun. Replace, modun önceliğini ve etkin olup olmadığını korur.

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

### Nexus koleksiyonları

Bir koleksiyon bağlantısı yapıştırın - ya da sitede birine tıklayın - Eidos o
revizyonun üyelerini listeler; her biri bu örnekle eşleştirilmiş olarak: kurulu,
indirilmiş ya da eksik. *Install this collection* ise bütün tarifi, paketleme ve
açma işlerinin kullandığı ilerleme çubuğunun aynısıyla, bir çalışan iş
parçacığında koşturur; bitirdiğinde çubuk raporun kendisi olur. Yukarıda neyin
uygulandığı ve neyin yalnızca adının geçtiği üzerine söylenen her şey burada da
kelimesi kelimesine geçerlidir: pencere ile `eidos collection`, iki ön yüzü olan
tek bir motordur.

Bir koleksiyon yalnızca **kendi oyununa** karşı okunabilir. Yüklü bir Fallout 4
örneğiyle bir Skyrim koleksiyonu açın; üyeleri yanlış mod listesiyle
eşleştirmektense adını vererek reddeder - o listede her "kurulu" ve her "eksik",
yanıt kılığına girmiş gürültü olurdu.

Güncellemeleri denetlediğinizde Eidos, Nexus'a bu örnekte kurulu her koleksiyonun en son revizyonunu da sorar. Mod listesinin üstündeki bir satır, daha yeni revizyonu olan her koleksiyonu adıyla verir ve eklenen (+), güncellenen (~) ve çıkarılan (-) mod sayısını gösterir; adlar için üzerine gelin. Kurulu revizyonun tarifi artık önbellekte değilse sayılar görünmez. **Güncelle**, yeni revizyonu koleksiyon panelinde açar; orada *Install this collection* onu çalıştırır. Liste her denetimde yeniden kurulur ve kaydedilmez.

### Çevrimdışı kip

**Preferences -> Nexus -> Offline**, Eidos'un Nexus'la hiç bağlantı kurmasını
durdurur. Güncelleme denetimleri, oturum açma, indirmeler ve koleksiyonlar bir
bağlantı hatasıyla başarısız olmak yerine bunu söyler. Siz açmadıkça kapalıdır -
daha eski bir Eidos'un yazdığı bir ayar dosyasında böyle bir anahtar yoktur ve
eksik olanı "açık" diye okumak, yükseltme yapan herkesin ağını keserdi.

**Preferred servers**, bir indirmenin yeğlediği CDN düğümlerini en iyisi başta
olacak biçimde sıralar. Aralarından seçim yapılacak birden çok yansı yalnızca
premium bir hesaba verilir, dolayısıyla diğer herkes için seçimi Nexus yapar ve
bu hiçbir şeyi değiştirmez. Bu bir süzgeç değil, bir sıralamadır: adını
verdiğiniz hiçbiri bugün sunulmuyorsa indirme yine de olur, Nexus'un ilk sunduğu
düğümden.

**Categories** yalnızca gösterilmez, düzenlenebilir de: onları tek bir moda ya da
bütün bir seçime atayın, katalogun kendisini aynı iletişim kutusundan düzenleyin
ve oyunun resmî kategori listesini Nexus'tan çekin. İki katalog dosyası da
MO2'nin kendisidir (`categories.dat` ve `nexuscatmap.dat`), yani paylaşılan bir
örnek tek bir katalog tutar. Nexus'tan indirilen bir mod, Nexus'tan getir kataloğu eşleştirdikten sonra kurulum sırasında kategorilenir.

**View -> INI editor**, profilin oyun INI'lerini düzenler - her başlatmada
üzerine yazılan, Proton önekine gömülü olanı değil, kalıcı olan kopyayı.
**View -> Log**, oturum günlüklerini okur. **View -> Extensions**, kendi
eklentilerinizi listeler; bkz. [extensions.tr.md](extensions.md).

Kurulum her şeyi kabul eder: Simple ve FOMOD yolları, artı Wrye Bash **BAIN**
paketleri (sırayla katılan alt paketleri işaretleyin) ve hiçbir sezgisel yöntem
yerleşimi tanımadığında arşiv ağacını gösterip veri kökünü işaret etmenizi
sağlayan **elle** bir seçici. Hiçbir arşiv reddedilmez.

Bir FOMOD modunu yeniden kurmak ya da güncellemek, sihirbazı son kurulumun
cevapları işaretli olarak, o zamandan beri seçenekleri değişen ilk adımda ve
değişikliklerin listesiyle açar. **Reinstall with new choices** bunun yerine
yükleyicinin varsayılanlarından başlar. `eidos install --replace` ya da
`--merge` kayıtlı cevapları yeniden kullanır; cevaplar artık uymuyorsa
değişiklikleri listeleyip kurmayı reddeder.

**Diagnostics** canlı sağlık denetimleri çalıştırır: her şeyden önce başlatma
yeteneği, eksik master'lar (tek başına en güvenilir çökme belirtisi), etkin
hiçbir eklentinin yüklemeyeceği arşivler, mod listesinin hâlâ mods klasörüyle
eşleşip eşleşmediği ve - bir çalıştırmadan sonra - script extender'ın kendi
günlüğünün, eklenti DLL'lerinin her biri hakkında ne söylediği; bu da "SKSE
eklentilerim yüklendi mi?" sorusunu bir çıkarımdan bir kanıta çevirir.
Ayrıca son temiz başlatmadan bu yana bir oyun güncellemesini, eksik ya da oyun sürümüyle uyuşmayan bir script extender'ı (extender'ın indirme sayfası ve indirdiğiniz arşivi kuran bir düğmeyle), Eidos'u `%command%` ile başlatmayan bir Steam başlatma seçeneğini, DLSS için kapalı bırakılmış NVAPI'yi, kare üretimini durduran özel tam ekranı, `LinuxVersion=true` içermeyen bir ENB `enblocal.ini` dosyasını, F4SE ve SFSE eklenti bildirimlerini ve son oturumun çökme günlüğünü, adını verdiği modlarla birlikte bildirir.
Güncelleme denetimi her modun Nexus gereksinimlerini de okur. Diagnostics, eksik veya devre dışı bir gerekli modu listeler; başka yoldan karşılanan bir gereksinim için Ignore vardır.

Oyunu GUI üzerinden başlatmak için oyunun Steam başlatma seçeneğini ikili
dosyanın mutlak yoluna ayarlayın (Steam, PATH'te `~/.cargo/bin`'i görmez):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos o oyunun örneğinde açılır - en son kullandığınızda, yani taşınabilir bir
örnek de global olan gibi yeniden bulunur; birleşik görünüm üzerinden başlatmak
için Run'a tıklayın.

Steam dışında Run oyunu kendisi Proton ya da Wine ile başlatmaz. Kopyanın sahibi
olan mağazadan onu başlatmasını ister, mağaza da Eidos'u başlatır:

- Steam, yukarıdaki başlatma seçeneği ayarlıysa `steam -applaunch <appid>`
  çalıştırır. Steam kapalıysa önce Steam açılır.
- Heroic (GOG ya da Epic kopyası, yerel ya da Flatpak), Eidos oyunun Heroic'teki
  ayarlarında, çalıştırılabilir dosya olarak `eidos-gui`'nin mutlak yolu ve hiç
  argüman olmadan bir wrapper olduğunda bir `heroic://launch` bağlantısı alır.
  Heroic kapalıysa önce Heroic açılır.
- Tek başına Legendary, `~/.config/legendary/config.ini` dosyasında oyunun
  `[<app>]` bölümünde ya da `[default]` altında `wrapper =` ve `eidos-gui`'nin
  mutlak yolu varsa `legendary launch <app>` çalıştırır.

Eidos ardından en son kullandığınız örnekte ikinci bir pencere açar ve Run'a
orada tıklarsınız. İlk pencere o çalıştırmayı izlemez; listelerini yeniden
yüklemek için oynadıktan sonra orada F5'e basın. Mağazadaki ayar eksikse ya da
script extender'ın loader'ı oyunun çalıştırılabilir dosyasının yanında değilse,
Run hiçbir şey başlatmaz ve neyin nereye ayarlanacağını, çalışan ikili dosyanın
gerçek yoluyla gösterir.

Bethesda oyunları için Steam'in `%command%` değeri genellikle
`<Game>Launcher.exe`'yi gösterir. Eidos onu asla çalıştırmaz: başlatıcı, `Data`'yı
yeniden tarayan ve `plugins.txt`'yi yeniden yazan ayrı bir ayar uygulamasıdır;
böylece az önce yerleştirilen yükleme sırasını geri alır. Kuruluysa script
extender'ın yükleyicisini, değilse oyun ikili dosyasını yerine koyar ve geri
düşmek zorunda kaldığında bunu söyler - her SKSE modu atıl halde başlayan bir
oyun, hiç başlamayandan kötüdür.

Buradaki eski yönergeler `WINEDLLOVERRIDES="d3dcompiler_47=n"` dayatıyordu. Bu
artık gerekmiyor ve zaten hiç doğru değildi: *native*'e yapılan bir geçersiz
kılma, ancak önekte gerçek bir `d3dcompiler_47.dll` zaten varsa işe yarar. Eidos
artık etkin modların DLL içe aktarımlarını tarar, gerçek Microsoft DLL'ini
kendisi yerleştirir ve geçersiz kılmayı ancak ondan sonra ayarlar.

## Kavram kanıtını deneyin

Oyun gerekmez. Yalnızca bir kullanıcı ad alanındaki ayrıcalıksız OverlayFS'i
kullanarak birleşim + copy-on-write + sıfır dokunuş + ad alanı başına kapsamı
kanıtlar (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Araçlar

xEdit, BodySlide, DynDOLOD ve arkadaşları, oyunun Proton öneki içinde birleşik
görünüm üzerinden çalışır:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # kayıtlı araçların istedikleri ve durumu
eidos prereqs skyrimse --install  # eksik olan ne varsa getir
```

Bir aracı adlandırmadan önce bilinmesi gereken bir şey: **başlık, Eidos'un onun
için hangi çalışma zamanı DLL'lerini sağlayacağına karar verir** - `BodySlide`
DirectX kitaplıklarını alır, `BS` hiçbir şey almaz. GUI'de Executables iletişim
kutusu her ön koşulun gerçek durumunu alanın altında gösterir ve eksik olanlar
birer düğmedir.

Tablo, üç ön koşul katmanı, DynDOLOD'un neden winetricks'in kuramadığı bir .NET
çalışma zamanı istediği ve bir mod olarak kurulan bir aracın neden kendi
klasöründen değil birleşik yoldan başlatıldığı [tools.tr.md](tools.md) içinde.

Kaynaktan derleme ve depo düzeni
[../internals/contributing.md](../../../../internals/contributing.md) içinde.

## Eklentiler

Eidos yeniden derlenmeden genişletilebilir: `~/.config/Colony/Eidos/addons/`
içindeki bir TOML bildirimi, Extensions listesine bir araç ya da Health sekmesine
bir denetim ekler. Eidos'un içine hiçbir şey yüklenmez - bir eklenti, onun
çalıştırdığı bir programdır. Bkz. [extensions.tr.md](extensions.md).
