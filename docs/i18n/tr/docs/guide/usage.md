<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Eidos kullanımı

Uygulamalı el kitabı: komut satırı, GUI, Steam başlatma seçeneği, kaynaktan
derleme ve kavram kanıtı betiği. Bir şey ters göründüğünde ne yapılacağı için
bkz. [troubleshooting.tr.md](troubleshooting.md).

## Kullanın (CLI)

```sh
eidos games                       # burada kurulu desteklenen oyunlar (MO2'nin listesi gibi)
eidos init skyrimse               # bir modlama örneği oluştur
# ...her modu bir klasör olarak <instance>/mods/ içine bırakın (global örnek
#    ~/.local/share/eidos/skyrimse konumundadır; `eidos init` sizinkini yazar)...
eidos install skyrimse mod.7z     # ya da indirilmiş bir arşivi kur (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # var olan bir MO2 profilinin sırasını + eklenti durumunu devral
eidos sort skyrimse               # eklenti yükleme sırasını LOOT ile sırala
eidos play skyrimse               # neyin bağlanacağını göster
eidos play skyrimse -- <command>  # <command>'ı modlar oyunun üzerine bağlanmış halde çalıştır
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` ve `eidos export`
takımı tamamlar; tam liste için `eidos`'u argümansız çalıştırın.

### Örnekler: global ve taşınabilir

Yukarıdaki her komut bir örneğe seslenir. `skyrimse`, **global** olanı
adlandırır - merkezî olarak `~/.local/share/eidos/skyrimse` içinde saklanır,
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
indirir. Bilmeye değer iki uyarı: taşınabilir bir klasörü taşımak, eski konuma
mutlak yollarla kaydettiğiniz araç girdileri dışında her şeyi korur (onları
yeniden ekleyin) ve paylaşılan çalışma zamanı önbelleği
(`~/.local/share/Colony/Eidos/runtimes/`) bilerek makine genelinde kalır - 78
MB'lık bir .NET host'u örnek başına olmaz.

Eidos kendi dosyalarını `Colony/Eidos` altında tutar; bu, Colony ailesindeki her
programın kullandığı düzendir: seçtikleriniz için `~/.config/Colony/Eidos/`
(tercihler, Nexus oturumunuz, örnek listeniz, yazdığınız oyun ve eklenti
tanımları), oturum günlükleri için `~/.local/state/Colony/Eidos/logs/` ve
Eidos'un indirdikleri için `~/.local/share/Colony/Eidos/`. Daha eski bir Eidos
bunları `~/.config/eidos/` ve `~/.local/state/eidos/` içinde tutuyordu;
yükseltmeden sonraki ilk başlatma onları karşıya **kopyalar** ve bunu günlükte
söyler. Eski dizinler tam olarak oldukları gibi bırakılır - hiçbir şey silinmez,
yani kötü bir yükseltme size bir oturum açmaya mal olamaz - ve içiniz rahat
ettiğinde onları kendiniz kaldırabilirsiniz.

Modlarınız buna dahil değildir. Global bir örnek hâlâ
`~/.local/share/eidos/<game>/` konumundadır, taşınabilir olan da onu koyduğunuz
yerde, çünkü bu yollar örnek listenize ve muhtemelen bir Steam başlatma
seçeneğine yazılmıştır: onları taşımak, Eidos'un iki ucuna birden sahip olmadığı
bir bağı koparırdı.

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

### Ayrıcalıklı adım gerekmez

Eidos tamamen rootsuz çalışır. Özel bir kullanıcı + bağlama ad alanı içinde
bağlar; yani setuid yardımcısı yok, artalan süreci yok ve verilecek bir şey yok.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` **isteğe bağlıdır** ve tam
olarak tek bir şeyi açar: çekirdek FUSE passthrough'u; o da oyunu bozduğu için
öntanımlı olarak kapalıdır (aşağıda). Bu yetenekle Eidos, bir kullanıcı ad alanı
yerine düz bir bağlama ad alanı alır; modlar iki durumda da aynı biçimde yerleşir.


Eski `setcap` önerisinin neden ortadan kalktığı - ve FUSE passthrough'un neden
kapalı geldiği - [troubleshooting.tr.md](troubleshooting.md#passthrough-neden-öntanımlı-olarak-kapalı)
içinde anlatılıyor.

## GUI

```sh
cargo run -p eidos-gui
```

Colony'nin parşömen / bordo görünümünde, MO2 tarzı bir ilk başlatma sihirbazı:
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
ve herhangi bir satırı Reveal ile bir dosya yöneticisinde gösterin. **Plugins**,
ESP/ESM/ESL yükleme sırasıdır (açıp kapatın, elle yeniden sıralayın ya da LOOT
ile sıralayıp sıralama sonrası raporu okuyun; oradaki öneri bağlantıları
tarayıcınızda açılır). **Conflicts**, dosya başına kazananları ve kaybedenleri
açıklar. **Overwrite**, oyunun yazdığını tek adımda gerçek bir moda dönüştürür.
**Saves**, her kaydın başlığını ayrıştırır - karakter, seviye, konum, oynanma
süresi - ve içine gömülü eklenti listesini geçerli listenizle karşılaştırır;
gerektirdiği modları etkinleştiren bir düğmeyle birlikte, çünkü onları adlandırıp
gerisini size bırakmak işin sıkıcı yarısıdır.

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

### Nexus koleksiyonları

Bir koleksiyon bağlantısı yapıştırın - ya da sitede birine tıklayın - Eidos o
revizyonun üyelerini listeler; her biri bu örnekle eşleştirilmiş olarak: kurulu,
indirilmiş ya da eksik. Bir koleksiyonu **okur**; kurmaz ve bölme de bunu söyler.
Burada bir kurucuyu yalnızca zor değil, dürüstlükten uzak kılan dört şey var:
üyeler, sitenin kendi düğmesi dışında yalnızca premium bir hesabın
üretebileceği, dosya başına bir anahtar isteyen sıradan Nexus dosyalarıdır; tam
bir kurulum, bu istemcinin aşmayı reddettiği bir bütçeye karşı üye başına üç API
çağrısıdır; manifest'in aşamaları, kuralları ve yeniden oynatılan FOMOD yanıtları
gerçek, yayımlanmış bir Bethesda koleksiyonuna karşı doğrulanamadı ve tahmin
etmek, doğru görünen ama olmayan bir yükleme sırası üretir. Okumak bir istek eder
ve kesindir.

Bir koleksiyon yalnızca **kendi oyununa** karşı okunabilir. Yüklü bir Fallout 4
örneğiyle bir Skyrim koleksiyonu açın; üyeleri yanlış mod listesiyle
eşleştirmektense adını vererek reddeder - o listede her "kurulu" ve her "eksik",
yanıt kılığına girmiş gürültü olurdu.

### Çevrimdışı kip

**Settings -> Nexus -> Offline**, Eidos'un Nexus'la hiç bağlantı kurmasını
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
örnek tek bir katalog tutar.

**View -> INI editor**, profilin oyun INI'lerini düzenler - her başlatmada
üzerine yazılan, Proton önekine gömülü olanı değil, kalıcı olan kopyayı.
**View -> Log**, oturum günlüklerini okur. **View -> Extensions**, kendi
eklentilerinizi listeler; bkz. [extensions.tr.md](extensions.md).

Kurulum her şeyi kabul eder: Simple ve FOMOD yolları, artı Wrye Bash **BAIN**
paketleri (sırayla katılan alt paketleri işaretleyin) ve hiçbir sezgisel yöntem
yerleşimi tanımadığında arşiv ağacını gösterip veri kökünü işaret etmenizi
sağlayan **elle** bir seçici. Hiçbir arşiv reddedilmez.

**Diagnostics** canlı sağlık denetimleri çalıştırır: her şeyden önce başlatma
yeteneği, eksik master'lar (tek başına en güvenilir çökme belirtisi), etkin
hiçbir eklentinin yüklemeyeceği arşivler, mod listesinin hâlâ mods klasörüyle
eşleşip eşleşmediği ve - bir çalıştırmadan sonra - script extender'ın kendi
günlüğünün, eklenti DLL'lerinin her biri hakkında ne söylediği; bu da "SKSE
eklentilerim yüklendi mi?" sorusunu bir çıkarımdan bir kanıta çevirir.

Oyunu GUI üzerinden başlatmak için oyunun Steam başlatma seçeneğini ikili
dosyanın mutlak yoluna ayarlayın (Steam, PATH'te `~/.cargo/bin`'i görmez):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos o oyunun örneğinde açılır - en son kullandığınızda, yani taşınabilir bir
örnek de global olan gibi yeniden bulunur; birleşik görünüm üzerinden başlatmak
için Run'a tıklayın. (Run düğmesine Steam dışında basarsanız, düğme çalışan ikili
dosyanın gerçek yoluyla tam olarak bu satırı gösterir.)

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
