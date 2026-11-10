<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Araçlar: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Eidos üzerinden çalıştırılan bir araç, oyunun Wine önekinin (Steam'in Proton öneki
ya da Heroic'in kurduğu önek) içinde birleşik görünümü görür. Oyunun okuyacağını
okur (etkin her mod, öncelik sırasıyla) ve yazdığı ne varsa Overwrite'a iner;
orada tek tıkla gerçek bir mod olur. `runner=native` bir araç Proton olmadan
çalışır; bkz.
[aşağısı](#ortam-değişkenleri-ve-protonsuz-çalışan-araçlar).

## Eidos'un kendiliğinden bulduğu araçlar

Bazı araçlar bildirilmek yerine bulunacak kadar ayırt edici adlara sahiptir ve
xEdit bunun bariz örneğidir: Fallout 4 için `FO4Edit.exe`, Skyrim SE için
`SSEEdit.exe`, orijinali için `TES5Edit.exe` ve böyle sürer. Her birinin bir
QuickAutoClean girdisi de vardır; o, LOOT'un uyardığı kirli düzenlemeleri
temizler. `PGPatcher.exe`, Morrowind dışındaki her Bethesda oyununda aynı yolla
bulunur ve onsuz çalışamayacağı bir argümanla gelir (bkz.
[aşağısı](#pgpatcher-neden---ignore-mo2vfscheck-ister)). Eidos onları dosya adına
göre şuralarda arar:

- `Overwrite/Root/`, örneğin çalıştığı oyun klasörü (mağaza kurulumu ya da oyun
  kopyası) ve etkin modların `Root/` ağaçları;
- MO2 kullanıcılarının araçları kurduğu yer olan bu örneğin `mods/` dizini;
- örnekler arasında paylaşılan dizin için Tercihler'de belirlediğiniz araçlar
  klasörü (General -> Tools), örneğin `/mnt/Games/Tools`.

Yaygın üreticiler de aynı yolla bulunur, her biri desteklediği oyunlarda: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen ve Synthesis. BethINI Pie (`Bethini.exe`) de öyle, PGPatcher ile aynı oyunlarda; bkz. [BethINI Pie](#bethini-pie). Stardew Valley'de SMAPI (`StardewModdingAPI.exe`) aynı yolla bulunur. O aynı zamanda o oyunun script extender yükleyicisidir; bu yüzden oyunu, [Bir araca ulaşmak](#bir-araca-ulaşmak-gizlemek-sabitlemek-ve-bir-masaüstü-kısayolu) bölümünde anlatıldığı gibi mağaza üzerinden başlatır.

Liste oyun başınadır, yani bir Skyrim örneğine Fallout'un düzenleyicisi asla
önerilmez. Arama dört düzey aşağıda durur, çünkü bir mod havuzu yüz binlerce
dosyadır ve bu, araç listesi her kurulduğunda çalışır; sembolik bağları da
izlemez. Böyle bulunan bir araç, elle yazdığınız gibi yapılandırılır: ön
gereksinimleri, argümanları (PGPatcher'ın `--ignore-mo2vfscheck` değeri) ve
yakalama modu, aşağıdaki kurallara göre başlığından gelir.

Bir araç başka bir yerdeyse ya da farklı argümanlar istiyorsanız elle ekleyin.
Aynı başlığa sahip bir kullanıcı girdisi, otomatik bulunan her şeyi geçersiz
kılar.

## LOOT'un kirli olarak işaretlediği bir eklentiyi temizlemek

Bir LOOT sıralamasından sonra Diagnostics sekmesi, LOOT'un kirli (dirty) olarak
işaretlediği eklentileri listeler. Bu, hem bir xEdit'i hem de LOOT temizlik
verisi olan oyunlar içindir (Skyrim LE, SE ve VR, Enderal SE, Fallout 3, New
Vegas, Fallout 4 ve 4 VR, Oblivion, Starfield). Eidos xEdit'i bulduğunda
(Executables'a kayıtlı ya da örnekte araç aradığı herhangi bir yerde), kartta
Clean all bulunur ve her kirli eklentinin satır menüsünde Clean with
QuickAutoClean yer alır; bulamazsa kart, xEdit'i Executables'a eklemenizi
söyler. Eidos xEdit'i indirmez. xEdit'in QuickAutoClean modunu birleşik görünüm
üzerinden, `-qac -autoexit -autoload` ile, her seferinde tek bir eklenti
üzerinde çalıştırır. xEdit temizlenmiş eklentiyi yüklediğinin üzerine kaydeder;
birleşik görünümde bu, Overwrite'ta yeni bir dosyadır. Eidos bu dosyayı `Cleaned
plugins` adlı bir moda taşır; bu mod etkin olarak listenin en altında
oluşturulur ve orada her çakışmayı kazanır. Kendi modundaki ya da oyunun Data
klasöründeki eklentiye asla yazılmaz. Bir temizliği geri almak için `Cleaned
plugins` modunu devre dışı bırakın ya da içindeki bir dosyayı gizleyin.
Overwrite içinde duran bir eklenti temizlenmez, çünkü xEdit o dosyanın kendisini
yeniden yazardı; önce Overwrite'ı bir moda dönüştürün.

xEdit'in yedekleri, önbelleği ve günlüğü birleşik görünümün dışında,
`<instance>/logs/qac-<time>/` içine gider ve durum satırı o günlükteki sayıları
bildirir. Eidos bu klasörü başarılı bir temizlikten sonra siler, başarısız bir
temizlikten sonra tutar. Eidos her kopyanın yapıldığı eklentinin boyutunu ve
tarihini `Cleaned plugins/meta.ini` içine kaydeder. Bu eklenti değiştiğinde
Diagnostics sekmesi **Out-of-date copy of <plugin>** kartını gösterir; kartta eski
kopyayı gizleyen bir düğme vardır.

## Bir araç eklemek

GUI'de: menü çubuğunda **Tools** ya da araç çubuğunda **Executables**, ardından
Add. Komut satırından:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # kayıtlı olanları listele
eidos tool skyrimse run BodySlide         # birleşik görünüm üzerinden çalıştır
eidos tool skyrimse run BodySlide --print # komutu çalıştırmadan göster
eidos tool skyrimse rm BodySlide          # eklediğiniz bir girdiyi kaldır
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # bir exe dosyasını kaydetmeden bir kez çalıştır
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` bu çalıştırmanın çıktısını, aracın kendi ayarı ne olursa olsun, `mods/<mod>` içine alır. `rm` yalnızca sizin eklediğiniz girdileri kaldırır; oyun başına bir öntanımlı gizlenebilir, kaldırılamaz.

`--exe` ile çalıştırılan ve dosya adı, büyük/küçük harf yok sayılarak, Eidos'un oyun için tanıdığı bir ad olan bir exe, o aracın başlığını alır ve onunla birlikte aşağıdaki ön gereksinimleri, argümanları ve yakalama modunu da. `GenerateFNISforUsers.exe` FNIS'tir; bu yüzden yukarıdaki `--exe` satırı `dotnet48` ister ve `FNIS Output` içine yakalar. Başka her exe dosya adıyla adlandırılır. Data sekmesinde bir `.exe` satırında, girdisi doldurulmuş Executables penceresini açan **Add tool** ve dosyayı `--exe` gibi bir kez çalıştıran **Run** vardır. İkisi de dosyayı aynı şekilde adlandırır.

Bir Windows aracı olmayan bir komut için `eidos shell skyrimse`, `$SHELL`
kabuğunuzu (ayarlı değilse `/bin/sh`) birleşik görünüm bağlıyken oyun klasöründe
açar. Kabuğun yazdığı her şey Overwrite'a iner ve kabuktan çıkmak görünümün
bağlantısını kaldırır. `eidos shell skyrimse -- <command>` bunun yerine tek bir
komut çalıştırır. Etkileşimli kabukta Ctrl+C, yazdığınız komutu durdurur ve
kabuğu açık bırakır. Kabuk Proton olmadan çalışır; bir Windows programı
`eidos tool` üzerinden gider.

Pencerede **Stop**, takılan bir araç çalıştırmasını sonlandırır: ona iki kez
basın, Eidos çalıştırmanın programlarını sonlandırır, sonra dosyalarını her
çıkıştan sonra olduğu gibi işler. **Notify the desktop when a long job ends**
açıkken (Preferences, General, Running a game), pencere arka plandayken biten bir
araç çalıştırması masaüstü bildirimi gönderir.

Script extender, oyun ikili dosyası ve başlatıcı kendiliğinden algılanır;
yalnızca ek araçların kaydedilmesi gerekir. Bir oyun kopyasından çalışan bir
örnekte bunlar kopyada algılanır; bkz.
[Bir oyun kopyası üzerindeki araçlar](#bir-oyun-kopyası-üzerindeki-araçlar).

### Nerede olursa olsun gerçek dosyayı gösterin

Çalıştırılabilir dosyayı gerçekte bulunduğu yerde kaydedin. Araç bir mod olarak
kurulduysa, orası mod klasörünün içidir:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Bu, global örneğin yoludur. Taşınabilir bir örnekte aynı kural kendi klasörü
altında geçerlidir, `<instance>/mods/...`. Böyle mutlak bir yol, taşınabilir bir
klasörü sonradan taşımaktan sağ çıkmayan tek şeydir.

Eidos, başlatmadan önce o yolu birleşik olanla değiştirir, böylece araç
`<game>/Data/CalienteTools/BodySlide/` içinden çalışır ve oradaki diğer tüm
modların dosyalarını da görür. BodySlide boş bir `SliderSets` dizini ile gelir ve
inşa edebileceği her beden CBBE'den ve kıyafet modlarından gelir. Kendi mod
klasöründen başlatıldığında hiçbir şey bulamaz ve bozuk görünür.

MO2 da aynı nedenle aynı değiştirmeyi yapar: kendi yorum satırı FNIS'i anar.

Devre dışı bir moddaki araç için bu değiştirme yapılamaz, çünkü dosyaları
görünümde de değildir. Eidos bunu söyler ve aracı kendi klasöründen çalıştırır.

## Mod Organizer 2 listesinden araçlar

Eidos örneği olarak açılan bir Wabbajack listesi ya da başka bir MO2 örneği `ModOrganizer.ini` dosyasını korur ve orada tanımlı çalıştırılabilir dosyalar `tools.ini` içine kopyalanabilir:

```sh
eidos tool /path/to/list import-mo2           # nelerin ekleneceğini gösterir
eidos tool /path/to/list import-mo2 --write   # ekler
```

Pencerede Executables iletişim kutusunda **Import from ModOrganizer.ini** düğmesi vardır; her girdi için işareti kaldırılabilen bir kutu bulunur. **Add selected** işaretli girdileri listeye koyar ve Save'e basana kadar hiçbir şey kaydedilmez. Yalnızca Eidos'un yazmadığı bir `ModOrganizer.ini` sayılır; Eidos'un yazdığı için bkz. [PGPatcher ve PGLite](#mod-organizer-2-modunda-pgpatcher-ve-pglite).

`Z:` üzerindeki yollar Linux yollarıdır ve olduğu gibi kalır. Başka bir sürücü harfindeki yol, listenin klasörünün adını içeriyorsa o klasörün altına taşınır (`D:\Lists\MyList\tools\SSEEdit.exe`, `<list>/tools/SSEEdit.exe` olur); bu çalıştırılabilir dosyada, çalışma dizininde ve argümanlarda geçerlidir. Script extender'ın yükleyicisi, başlatıcı ve oyun ikili dosyası düz ada dönüşür, böylece yerleşik girdiler gibi Eidos'un görünümü bağladığı oyun klasöründen çalışırlar. MO2'nin `hide` değeri **Hide from picker**, `steamAppID` değeri **Steam AppID** olur. Başka yere işaret eden girdiler (bir Windows sistem programı, liste dışındaki bir araç) nedeniyle birlikte atlanmış olarak listelenir, `tools.ini` içinde zaten bulunan bir başlık ise hiçbir zaman değiştirilmez.

İçe aktarılan bir girdi MO2'nin başlığını ve argümanlarını korur ve Eidos başlığından hiçbir şey doldurmaz: ön gereksinim yok, ek argüman yok, yakalama modu yok. Bunları Executables penceresinde ayarlayın. Windows'ta yapılmış bir listeden gelen bir PGPatcher girdisinde genellikle `--ignore-mo2vfscheck` eksiktir; ekleyin (bkz. [aşağısı](#pgpatcher-neden---ignore-mo2vfscheck-ister)).

Listenin `Stock Game` klasörünü gösteren içe aktarılmış bir yol, yalnızca örnek o kopyadan çalışırken ve yalnızca çalıştırma sırasında mağaza kurulumuna taşınır; bkz. sonraki bölüm.

## Bir oyun kopyası üzerindeki araçlar

Örnek bir oyun kopyasından çalıştığında (bir listenin `Stock Game` ya da `Game Root`
klasörü, ya da dondurulmuş bir kopya; bkz.
[Oyun dosyaları](usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya)), bir araç
mağaza kurulumunun değil kopyanın dosyalarını görür. Birleşik görünüm yine mağaza
kurulumunun üzerine bağlanır: kök görünümün en altında kopya durur ve Data
görünümü kopyanın `Data` klasörünü okur. Eidos script extender'ı, başlatıcıyı, oyun
ikili dosyasını ve bilinen araçları mağaza kurulumunda değil kopyada arar.

Kopyayı gösteren bir araç yolu, argüman ya da çalışma dizini çalıştırmadan önce
mağaza kurulumuna taşınır. Linux yolu ya da Wine `Z:` yolu olarak yazılabilir:
`-D:"Z:\...\Stock Game\Data"`, mağaza kurulumu altındaki aynı yola dönüşür. Bir
listenin `ModOrganizer.ini` dosyasından içe aktarılan girdilerin buna ihtiyacı var.
Örnek mağaza kurulumundan çalışırken hiçbir şey taşınmaz, bu yüzden böyle bir girdi
modlarınız olmadan çıplak kopyayı okur.

Önekin kayıt defterindeki oyunun `installed path` değeri ve Eidos'un yazdığı
`ModOrganizer.ini` içindeki `gamePath`, birleşik görünümün bulunduğu mağaza
kurulumunu adlandırmayı sürdürür. Kopya klasörü yoksa ya da hiçbir oyun ikili
dosyası içermiyorsa, bir oyun başlatması gibi bir araç çalıştırması da durur ve
ileti mağaza kurulumuna dönmek için `eidos stock-game <instance> store` komutunu
adlandırır.

`eidos stock-game <instance>`, örneğin hangi oyun dosyalarını çalıştırdığını ve
hangi kopyalara geçebileceğini gösterir. `use <folder>` ve `store` geçiş yapar,
`freeze --yes` mağaza kurulumunu örneğin `Stock Game` klasörüne kopyalar ve oradan
çalışır. Diagnostics sekmesindeki Game files kartı aynısını yapar; bkz.
[Oyun dosyaları](usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya).

## Bir aracın çıktısını kendi moduna göndermek

Bir üreteç (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) yüzlerce dosya
yazar. Öntanımlı olarak bunlar diğer her şeyle birlikte Overwrite'a iner.
Executables düzenleyicisinde **Capture output into** ayarını verin; bu
çalıştırmanın çıktısı onun yerine o moda gider:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide ve Synthesis, yakalama modu hazır ayarlı gelir: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` ve `Synthesis Output`. DynDOLOD, TexGen ve xLODGen ise vermeniz gereken çıktı klasörüne yazdıkları için bu ayar olmadan gelir.

Mod yoksa oluşturulur. Yalnızca bu çalıştırmanın ürettiği dosyalar taşınır;
Overwrite'ta zaten bulunan ne varsa orada kalır, böylece yakalama hedefi olan
iki araç birbirinin çıktısını almaz. Hiçbir şey yazmayan bir çalıştırma,
ardında boş bir mod bırakmaz.

Eidos dosyaları çalıştırmadan sonra taşır. MO2 ise yazma katmanını moda yöneltir;
bu, o modu çalıştırma boyunca en yüksek önceliğe çıkarır: içinde bulunduğu her
çakışma ters döner ve sonra geri döner. Yazmalar da copy-up olmadan doğrudan modun
kendi dosyalarının üzerine gider. Yakalama, ikisi de olmadan aynı son duruma varır.

Hedef mod devre dışıysa çıktı yine yazılır ama oyun onu görmez, dolayısıyla araç
bir sonraki çalıştırmada aynı dosyaları yeniden üretir. Eidos durum böyleyken
uyarır.

### Çıktısı güncelliğini yitirdiğinde

Eidos, bir aracın çıktısının hangi modlardan, hangi eklenti sırasından ve hangi profilden üretildiğini, aracın çalıştırılabilir dosyasının boyutunu ve tarihini, ayrıca girdinin `tools.ini` içindeki yolunu ve argümanlarını kaydeder. Bunlardan biri değişirse Diagnostics sekmesi **Generated output inputs changed** kartını gösterir; kartta aracı şimdiki ayarlarıyla çalıştıran bir **Run <tool> again** düğmesi bulunur.

Güncel olmayan çıktısı oyunu bozan bir üretici için, örneğin Pandora veya Nemesis, `tools.ini` içindeki girdisine `check_before_play=1` ekleyin ya da Executables penceresinde onun için **Ask before playing when its output is out of date** seçeneğini açın. Run bu durumda önce sorar: aracı çalıştır ya da yine de oyna. Play anyway seçimi, modlar, eklentiler veya profil yeniden değişene ya da pencere kapanana kadar hatırlanır. Eidos aracı kendiliğinden hiç başlatmaz. **Start the game at once when a store opens Eidos** açıkken böyle bir soru, oyunu başlatmak yerine pencereyi açık tutar.

## Ön gereksinimler ve argümanlar başlıktan gelir

Bir araca verdiğiniz başlık, Eidos'un onun için hangi çalışma zamanı ön
gereksinimlerini sağlayacağını belirler. Eşleşme, başlığın büyük/küçük harf
duyarsız bir alt dizesidir.

| Başlık şunu içeriyorsa | Eidos şunu ister |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| başka herhangi bir şey | hiçbir şey |

Yani `BodySlide` olarak kaydedilen bir araç DirectX DLL'lerini alır; aynı
çalıştırılabilir dosya `BS` olarak kaydedilirse hiçbir şey almaz ve DLL'lerden
hiç söz etmeyen bir hatayla başlamayabilir. Araçları programın adıyla
adlandırın.

Liste `default_prereqs` içindedir (`crates/eidos-instance/src/tools.rs`) ve
Executables iletişim kutusundaki `Prereqs` alanı düzenlenebilir: başlık yalnızca
öntanımlıyı doldurur.

Başlık argümanlara da aynı şekilde karar verir, kimsenin tahmin edemeyeceği bir
argüman isteyen tek araç için: `pgpatcher` (ya da eski adı `parallaxgen`) içeren
bir başlığa `--ignore-mo2vfscheck` verilir. Nedeni ve neden isteğe bağlı olmadığı
[aşağıda](#pgpatcher-neden---ignore-mo2vfscheck-ister). Yakalama modu da başlıktan
gelir (bkz. [yukarısı](#bir-aracın-çıktısını-kendi-moduna-göndermek)).

### Üç tür ön gereksinim

Kademe 1, paketle gelen DLL'lerdir: `d3dx9_43`, `d3dcompiler_47`,
`d3dcompiler_43` ve `d3dx11_43`. Eidos onları ikili dosyalarının içinde taşır ve
başlatmada öneke kopyalar. 64 bit bir önekte, Proton'un öntanımlısında, 64 bit
sürüm `system32` içine, 32 bit olan `syswow64` içine gider. Yapılacak bir şey yok,
ağ yok. Her sürüm gzip ile sıkıştırılmış saklanır; bu yüzden sekiz dosya her ikili
dosyada 17,9 MB yerine 7,9 MB yer tutar ve yalnızca kopyalanan açılır (en çok 4,7
MB).

Kademe 2, kurucularıdır: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` ve `xact_x64`. Bunlar
registry anahtarlarını, GAC'yi ve CLR barındırıcılarını yazar, dolayısıyla dosya
kopyalanarak halledilemezler. Eidos bunlardan üçünü winetricks olmadan kurar:
`vcrun2022`, Microsoft'un `VC_redist.x86.exe` ve `VC_redist.x64.exe` dosyalarını
çalıştırır; `dotnet8` ve `dotnetdesktop8` ise .NET çalışma zamanını da içeren .NET
8.0.12 Desktop Runtime kurucusunu çalıştırır. Sürüm paketi
(`eidos-linux-bundle.tar.gz`) bu kurucuları `runtimes/` klasöründe getirir ve
`install.sh` onları, çalıştıran kullanıcı için `--system` ile de,
`~/.local/share/Colony/Eidos/runtimes/` içine kopyalar. Eidos her dosyayı önce
orada, sonra `eidos` ikili dosyasının yanındaki bir `runtimes/` klasöründe arar
(yerinde çalıştırılan açılmış bir paket). İkisinde de kopya yoksa (tek dosya
`eidos-linux`, kaynaktan derleme ya da bir sistem kurulumunun başka bir kullanıcısı)
Eidos onları Microsoft'tan (`download.visualstudio.microsoft.com`) indirir ve
`~/.local/share/Colony/Eidos/runtimes/` içinde tutar. Her iki durumda da SHA-256'sı
Eidos'a gömülü olanla eşleşmeyen bir dosya reddedilir; dosya, oyunun araçlarını
çalıştıran Proton ya da Wine sürümüyle oyunun önekinde pencere olmadan
(`/install /quiet /norestart`) çalışır. winetricks'in yaptığı gibi `vcrun2022` önce
önekin DLL geçersiz kılmalarında Visual C++ DLL'lerini `native,builtin` yapar.
Ardından, bir Proton sürümüyle, Eidos önekin C++ çalışma zamanının yarısının
Microsoft'tan, yarısının Proton'dan gelmediğini denetler. Öyleyse kurulum
başarısız olur: ileti farklı yapılardan gelen DLL çiftlerini adlandırır ve bunun
yerine `vcrun2022` için winetricks kullanmanızı söyler; o `msvcp140.dll` dosyasını
kendisi değiştirir. Yalnızca 64 bit .NET 8 kurulur; 32 bit bir .NET 8 aracı hâlâ
`winetricks dotnetdesktop8` ister.

Diğer altı fiil, `PATH` üzerinde olması gereken sistemdeki `winetricks` üzerinden
geçer. Eidos onu, oyunun araçlarını çalıştıran Proton ya da Wine sürümünün
`wine` dosyasıyla, oyun öneki üzerinde çalıştırır; bu da Steam'in pressure-vessel
kabını ve protontricks + Proton-GE uyuşmazlığını atlar. Bazı winetricks fiilleri
`cabextract` ister ve eksikse Eidos uyarır. Kurulmamış bir Kademe 2 fiili bildiren
bir araç yine de başlar; fiili ve onu düzeltecek komutu anan bir uyarıyla, çünkü
onu başka bir yoldan kurmuş olabilirsiniz.

Önekin kendi `winetricks.log` dosyası bir Kademe 2 fiilini listeliyorsa, winetricks
ya da protontricks'i kim çalıştırmış olursa olsun, ya da Eidos onu aynı önekte
kurduysa fiil tamam sayılır. Eidos kendi kurulumlarını örnekteki `prereqs.done`
dosyasına, önekin yoluna ve inode'una bağlı olarak kaydeder; bu yüzden yeniden
kurulmuş bir önek fiillerini yine bekliyor gösterir. Bir Steam kopyası için
Diagnostics eylemi olan **Rebuild prefix**, `compatdata/<appid>` klasörünü
`<appid>.eidos-backup-<time>` olarak yeniden adlandırır ve hiçbir şey silmez; Steam
oyunu bir sonraki başlatışında yeni bir önek oluşturur.

Kademe 3, çalışma zamanlarıdır: `dotnet10` ve `dotnetsdk10`. Modern bir .NET
çalışma zamanı, kendi dizininde duran ve `DOTNET_ROOT` üzerinden bulunan 193
dosyadır. Hiç kaydedilmez ve öneke kurulmaz, dolayısıyla diğer iki kademeden
hiçbiri onu taşıyamaz. Eidos onu hiçbir örneğin dışında tutar, çünkü 78 MB ne oyun
başınadır ne de profil başına: her çalışma zamanı
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` içine açılır
(`dotnet10-10.0.10`, `dotnetsdk10-10.0.403`). Eidos çalışma zamanının zip dosyasını,
indirme adıyla, Kademe 2 kurucularıyla aynı iki klasörde arar ve sürüm paketinin
getirdiği kopyayı (`dotnet-runtime-10.0.10-win-x64.zip`, yalnızca `dotnet10` için)
açar ya da indirir. Paket SDK'nın `dotnet-sdk-10.0.403-win-x64.zip` dosyasını
getirmez, ama oraya elle konan bir kopya kullanılır ve indirmeyi kazandırır. Eidos
SHA-256'sı Eidos'a gömülü olanla eşleşmeyen dosyayı reddeder. Zip'i, mod arşivleri
için kullandığı 7-Zip ile, `7z` ile açar.

`dotnetsdk10`, .NET 10 SDK'sıdır: indirmesi 301 MB, açıldığında 808 MB.
Synthesis patcher'larını derlemek için buna ihtiyaç duyar ve onları derleyip
çalıştırmak için yolsuz bir `dotnet` çağırır; bu yüzden `dotnetsdk10` bildiren
bir araç ayrıca `WINEPATH` üzerinde SDK klasörünü, bir de
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
ve `DOTNET_CLI_TELEMETRY_OPTOUT=1` değerlerini alır.
Bir patcher derlemesi MSBuild düğümlerini ve VBCSCompiler sunucusunu çalışır
halde bırakırdı ve Wine altında bunlar önekin wineserver'ını canlı tutar; bu
yüzden araç ayrıca `MSBUILDDISABLENODEREUSE=1`,
`DOTNET_CLI_USE_MSBUILD_SERVER=0` ve `UseSharedCompilation=false` değerlerini de
alır. `DOTNET_ROLL_FORWARD=Major`, .NET 8 için derlenmiş bir patcher'ın (0.54
öncesi Mutagen) hiçbir .NET 8 çalışma zamanı bulunamadığında SDK'nın .NET 10
çalışma zamanında çalışmasını sağlar.
`tools.ini` içinde hâlâ eski varsayılanı, `dotnet8`, `vcrun2022` taşıyan bir
Synthesis girdisi, Eidos onu okurken `dotnetsdk10`, `vcrun2022` olur.

NuGet geri yüklediği her paketin imzasını Windows kök sertifika deposuna göre
denetler ve bir Wine önekinde, SDK'nın
`sdk/10.0.403/trustedroots/codesignctl.pem` ve `timestampctl.pem` olarak
getirdiği kod imzalama ve zaman damgası kökleri yoktur. Bunlar olmadan
`Mutagen.Bethesda.Synthesis` 0.36.6 geri yüklemesi `NU3018` ve `NU3028`
hatalarıyla başarısız olur. Bu yüzden Eidos, `dotnetsdk10` bildiren bir aracı
çalıştırmadan önce ve SDK indirilmişse, iki dosyadaki 385 sertifikayı oyunun
önekindeki geçerli kullanıcının kök sertifika deposuna ekler; bunu oyun yolunu
yazan aynı `regedit` aktarımında yapar (aşağıya bakın). Biri kaybolursa onları
yeniden aktarır. Bundan sonra o önekteki her Windows programı bu köklere
güvenir. Bunlarla, yeni bir önekte ve boş bir NuGet önbelleğiyle, aynı geri
yükleme ve derleme sistemdeki Wine 11.21 ile de GE-Proton11-9 ile de başarılı
olur.

Eidos siz isteyene kadar Kademe 2 ya da 3'ten hiçbir şey kurmaz:

```sh
eidos prereqs skyrimse            # kayıtlı araçların neye gereksindiğini ve durumlarını göster
eidos prereqs skyrimse --install  # eksik olanı kur (önce paketin kopyaları, sonra indirmeler)
```

GUI'de aynı durumlar Prereqs alanının altında durur ve eksik olanlar birer
düğmedir. Ne paketle gelen, ne bir çalışma zamanı, ne de bir Kademe 2 fiili olan
bir fiil, indirme olarak sunulmak yerine olası bir yazım hatası diye bildirilir.

`eidos prereqs <instance> --install`, oyun, Steam ya da bir araç önekin hâlâ
tuttuğu sürece reddeder. Kademe 3 çalışma zamanları önek henüz yokken bile kurulur.
Eidos'un kendi çalıştırdığı kurucular var olan bir önek ister: mağaza oyunu bir kez
başlatmadan önce "start the game once through its store, then install again"
iletisiyle dururlar. Kademe 2 fiilleri tek tek kurulur ve her başarı kaydedilir;
bu yüzden bir başarısızlıktan sonra aynı komut kaldığı yerden sürer. Heroic
ayarı olmayan bir GOG ya da Epic kopyası için `--install` paketlenmiş DLL'leri
kopyalar ve sonra Kademe 2 fiillerinden önce durur: o çalıştırıcının kendi
winetricks'ini kullanmanızı söyler ve bir sonraki denetimde önekin
`winetricks.log` dosyasını okur.

### DynDOLOD neden `dotnet10` gereksinir

DynDOLOD nesne LOD'unu kendi inşa etmez: işi LODGen'e havale eder ve üç tanesini
birlikte getirir. `LODGenx64.exe`, Proton altında Wine'ın Mono'suna yönlendirilen
.NET Framework 4.8'i hedefler ve Mono'nun `System.Uri` başlatıcısı Mono'nun
uygulamadığı bir yöntemi çağırır. LODGen ilk iş satırından önce ölür; geriye
yalnızca bir sürüm başlığı tutan bir günlük ve yalnızca "failed for one or more
worlds" diyen bir DynDOLOD iletişim kutusu kalır.

Gerçek .NET Framework'ü kurmak bunu düzeltmez: Proton, onu bulacak yükleyici olan
`mscoree.dll`'i kendi ağacına giden bir sembolik bağla değiştirir ve bunu her
önek güncellemesinde yeniden yapar.

İşe yarayan yapı `LODGenx64Win10.exe`; modern .NET'i hedefler ve `mscoree`'ye hiç
dokunmaz. `DOTNET_ROOT`'u bir .NET 10 çalışma zamanına yöneltin, çalışır.
`dotnet10`'un sağladığı budur ve Eidos, onu bildiren herhangi bir aracı
başlatırken değişkeni ayarlar.

### Pandora neden `dotnet10` gereksinir

Pandora bir WPF değil, bir Avalonia uygulamasıdır ve Pandora 4.1.0'dan beri
`net10.0` hedefler; bu yüzden onu yalnızca .NET 10 çalışma zamanı çalıştırır.
Hiçbir çalışma zamanı bulamadığında .NET Desktop Runtime'ı kurmanızı söyler. Bu,
çalışma zamanı bulamayan her GUI programı için .NET başlatıcısının gösterdiği
genel metindir ve `dotnetdesktop8` onu düzeltmez.

Eidos 1.21.0 ve öncesi Pandora'ya `dotnetdesktop8` veriyordu ve bir girdi
`prereqs=` satırını korur. Eidos hâlâ yalnızca `dotnetdesktop8` listeleyen bir
Pandora girdisini okuduğunda ve çalıştırılabilir dosyasının yanındaki
`.runtimeconfig.json` `net10.0` adlandırıyorsa, girdi onun yerine `dotnet10`
alır. Pandora 2.x .NET 8 hedefler ve sahip olduğunu korur.

### PGPatcher neden `--ignore-mo2vfscheck` ister

PGPatcher, sahip olduğunuz doku modlarının (vanilla, parallax, complex material,
PBR) her yüzeyde doğru shader'ı kullanması için mesh'leri ve eklentileri yeniden
yazar. Bir mod yöneticisi dışında çalışmayı reddeder, çünkü çıplak bir oyun
klasörünü yamamak yıkıcı olurdu.

PGPatcher, MO2'nin oyuna enjekte ettiği Windows DLL kanca katmanı olan usvfs'i
arar. Eidos'ta böyle bir şey yok: birleşik görünüm, Wine'ın varlığından habersiz
içine girdiği bir FUSE bağlamasıdır. Bu yüzden denetim hiçbir şey bulamaz ve araç
hemen şu iletiyle çıkar

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

kullanıcının çalıştırmadığı bir programı adlandırarak, herhangi bir iş yapmadan
önce. Yazarın bu konudaki iki Linux bildirimine ([#716][pg716] ve [#725][pg725],
ikisi de aynı nedenle FUSE tabanlı olan, MO2'nin Linux çatalı Fluorine'den) kendi
yanıtı, denetimi atlayan bir bayraktır.

Bu yüzden Eidos onu önceden koyar. Bir mod havuzunda bulunan PGPatcher argümanı
ayarlanmış olarak gelir ve Executables penceresinde başlık olarak `PGPatcher`
yazmak Arguments alanını aynı şekilde doldurur. Yazdığınız her zaman kazanır:
yalnızca boş bıraktığınız bir alan doldurulur, tıpkı yukarıdaki ön gereksinimler
gibi.

Liste `default_args` içinde (`crates/eidos-instance/src/tools.rs`),
`default_prereqs` yanında.

Çalışma zamanı, bir .NET aracı için alışılmadık biçimde hiçbir şey istemez: kendi
çalışma zamanını taşır (`dotnetlib/`, `includedFrameworks` bildiren bir
`runtimeconfig.json` ile) ve Microsoft CRT DLL'lerini çalıştırılabilir dosyanın
yanına koyar. Prereqs alanını boş bırakın.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 modunda PGPatcher ve PGLite

Mod Manager `None` olarak ayarlıyken PGPatcher tek bir birleşik `Data` klasörü
görür ve bir mesh'in ya da dokunun hangi moddan geldiğini bilemez, bu yüzden
gösterecek çakışması ve izleyecek mod sırası yoktur. `Mod Organizer 2` modu
örneğin kendisini okur: profilin `modlist.txt` dosyasını, etkin modları öncelik
sırasıyla ve her modun klasöründeki dosyaları. Oyun içi sürüm olan PGLite de
örneği aynı şekilde okur. Bir Eidos örneği zaten MO2'nin düzenine sahiptir
(`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), bu yüzden bu mod onunla
çalışır.

PGPatcher'ın başlatıcısında:

1. **Mod Manager** ayarını `Mod Organizer 2` yapın.
2. **MO2 Options -> Instance Location** ayarını Eidos örneğinin klasörü yapın;
   bunu Wine'ın `/` olan `Z:` sürücüsünde bir Windows yolu olarak yazın.
   `/mnt/Games/Eidos-Skyrim` konumundaki taşınabilir bir örnek
   `Z:\mnt\Games\Eidos-Skyrim` olur; genel bir örnek
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>` olur.
3. `--ignore-mo2vfscheck` argümanını aracın argümanlarında tutun. Bu modda
   PGPatcher usvfs'yi arar ve bulamazsa durur.

PGPatcher klasörü yalnızca içinde bir `modorganizer.ini` varsa kabul eder; bu
yüzden Eidos, oyunu ya da bir aracı her başlattığında, başlatılan profil için
örneğin kök klasörüne `ModOrganizer.ini` yazar. Dosya yalnızca içeriği
değiştiğinde yeniden yazılır. İçinde şunlar bulunur:

- `[General]` içinde: `gameName` (oyunun MO2'deki adı, yani Fallout: New Vegas için
  `New Vegas`), `gamePath` (oyunun mağaza kurulum klasörü, `Z:` yolu olarak; bir
  oyun kopyasından çalışan örnekte de) ve `selected_profile`;
- `[Settings]` içinde: mutlak `Z:` yolları olarak `mod_directory`,
  `profiles_directory` ve `overwrite_directory`, örneğin bir `downloads/`
  klasörü varsa ayrıca `download_directory`.

İlk satırı, dosyayı Eidos'un yazdığını söyleyen bir yorumdur. Bu satırı
içermeyen bir `ModOrganizer.ini`, adı hangi büyük/küçük harfle yazılmış olursa
olsun, asla değiştirilmez ya da silinmez: Eidos'ta açtığınız bir MO2 örneği
kendi dosyasını korur ve PGPatcher o dosyayı olduğu gibi okur. MO2 ayarlarını
kaydederken yorumları atar, bu yüzden MO2'nin üzerine kaydettiği bir dosya o
andan itibaren MO2'nindir. Boş bir dosya Eidos'unki sayılır.

Böyle bir dosya MO2'nin en son seçtiği profili gösterir ve PGPatcher, Eidos'un
başlattığı profili değil bu profili kullanır. İkisi farklıysa Eidos başlatırken
günlüğe bir uyarı yazar. MO2'de aynı profili seçin ya da Eidos kendi dosyasını
yazabilsin diye dosyanın adını değiştirin veya dosyayı kaldırın.

Mod Organizer 2 kipindeki araçlar oyunu da o dosyanın `gamePath` değerinden okur.
Bu değer ne örneğin çalıştığı oyun klasörünü ne de mağaza kurulumunu adlandırıyorsa
Diagnostics sekmesi, o yolla ve bu örneğin çalıştırdığı oyun klasörüyle birlikte
**ModOrganizer.ini points tools at another game folder** kartını gösterir.
Dosyada `gamePath` değerini değiştirin ya da Eidos kendisininkini yazsın diye
dosyayı kaldırın.

Eidos kendi dosyasını yalnızca bir şey başlatırken yazar, bu yüzden iki başlatma
arasında dosya sonuncunun profilini gösterir. Eidos'un başlattığı bir araç orada
kendi profilini bulur.

PGPatcher, profilde etkin hiçbir mod olmadığında ve çıktı klasörü etkin bir
modun klasörü olduğunda, MO2 altında olduğu gibi bir hatayla durur.

## Wine'ın kendi programları oyunun önekinde

Dosya menüsü, oyunun önekinde Wine'ın dört programını çalıştırır: winecfg, regedit, Wine dosya gezgini ve kaldırıcı. Diğer araçlar gibi, oyunun araçlarını çalıştıran Proton veya Wine sürümüyle, modlar bağlanmış olarak başlarlar ve pencere onları bekler. **Prefix drive_c**, önekin `C:` sürücüsünü dosya yöneticinizde açar. Önekte o program olana kadar satır gri kalır; bu, mağaza oyunu ilk kez başlattığında olur. Bir terminalden `eidos tool <instance> run winecfg` aynısını yapar; listede bu başlığı taşıyan bir araç yoksa `regedit`, `explorer` ve `uninstaller` de öyle.

regedit'te oyunun `installed path` değerinde yaptığınız değişiklik bir sonraki araç çalıştırmasında geri konur; bkz. sonraki bölüm.

## Önekteki oyun yolu

Windows araçları oyunlarını `HKLM\Software\Bethesda Softworks\<game>`
`installed path` okuyarak bulur; bu anahtarı oyunun kendi kurucusu yazar. Proton
altındaki Steam o kurucuyu hiç çalıştırmaz. O anahtar olmadan xEdit, Wrye Bash ve
DynDOLOD boş bir yolla açılır.

Bir araç çalıştırmasından önce Eidos, daha önceki bir aktarım aynı yolu yazmadıysa
ve önek hâlâ onu tutmuyorsa, anahtarı oyunun önekinde bir `regedit` aktarımıyla
yazar. Anahtar, birleşik görünüm orada bağlı olduğu için, bir oyun kopyasından
çalışan örnekte de mağaza kurulumunu adlandırır. `dotnetsdk10` bildiren bir araç
için aynı aktarım NuGet imza köklerini ekler (bkz.
[yukarısı](#üç-tür-ön-gereksinim)). Bir `runner=native` araç bu adımı atlar.
Önek henüz yokken ya da başka bir program onu tutarken araç yine de çalışır ve
Eidos, oyunu bir kez başlatmak için mağazayı ya da kapatılacak programı adlandıran
bir uyarı yazdırır. Anahtar mağaza kurulumundan başka bir klasörü adlandırıyorsa
Diagnostics sekmesi, sonraki araç çalıştırması onu yeniden yazana kadar **Windows
tools cannot find <game>** kartını gösterir.

## Bir araca ulaşmak: gizlemek, sabitlemek ve bir masaüstü kısayolu

Bir oyunun öntanımlıları hiç kullanmayacağınız araçları içerir. Executables
iletişim kutusunda:

- **Pin to top** bir girdiyi Run listesinin başına koyar.
- **Hide from picker** bir girdiyi silmeden listeden çıkarır.
- **Desktop shortcut** `~/.local/share/applications` içine bir `.desktop` yazar;
  dolayısıyla uygulama menünüzde ve aramada belirir, masaüstünde değil. Doğrudan
  `eidos tool <instance> run <title>` çalıştırır; bu yüzden araç, Eidos penceresi
  açık olmadan, birleşik görünüm üzerinden ve bu örneğin profiliyle gelir.

Gizleme ve sabitleme yalnızca bir araca nasıl ulaşıldığını değiştirir, dolayısıyla
kendi girdilerinize olduğu gibi oyun başına öntanımlılara da uygulanır.

Script extender'ın yükleyicisi, başlatıcı ve oyun ikili dosyasında Desktop
shortcut düğmesi yoktur. Bunlar oyunu başlatır ve oyun her zaman kendi mağazası
üzerinden başlar; bunu mağazanın kendi kısayolu zaten yapar. Eidos 1.22 veya
daha eski bir sürümle bunlardan biri için oluşturduğunuz kısayol çalışmaya
devam eder: bir terminale yazılan `eidos tool <instance> run <title>` gibi,
mağazadan oyunu başlatmasını ister; hiçbir mağazanın açmadığı bir pencerede
Run'ın yaptığı gibi. Mağazadaki ayar eksikse komut hiçbir şey başlatmaz ve
neyin ayarlanacağını yazdırır. Kısayol hiçbir ileti göstermez, bu yüzden okumak
için aynı komutu bir terminalde çalıştırın.

Eidos, bir girdinin çalıştırılabilir dosyasının adı, büyük/küçük harf yok sayılarak,
script extender yükleyicisinin, başlatıcının ya da oyun ikili dosyasının adı
olduğunda, o girdiyi mağazaya bu şekilde devreder: öntanımlı bir girdi, sizin
eklediğiniz biri, MO2'den içe aktarılmış biri ya da bir `--exe` çalıştırması. Önce
bu örneği son kullanılan olarak işaretler, böylece mağazanın açtığı pencere bu
örneği gösterir; sonra `steam -applaunch <id>`, Heroic için
`xdg-open heroic://launch?appName=<app>&runner=<runner>` ya da
`legendary launch <app>` çalıştırır. `--print` ile bu mağaza komutunu yazdırır ve
hiçbir şey başlatmaz. `steam` komutu `PATH` üzerinde olmalıdır: Flatpak Steam onu
oraya koymaz, bu yüzden başlatma başarısız olur ve ileti oyunu Steam'den
başlatmanızı söyler.

Bir mağaza süreci başlattığında girdi, her araç gibi doğrudan birleşik görünüm
üzerinden çalışır: `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`,
`HEROIC_APP_RUNNER` ya da `EIDOS_STORE_STARTED` değişkenlerinden biri ayarlıdır.
Bunlardan biri olmadan Eidos oyunu kendisi Proton ile başlatmaz. Örnekte script
extender yükleyicisi yoksa (çalıştığı oyun klasöründe, etkin bir modun `Root/`
dizininde ya da `Overwrite/Root/` içinde), hiçbir şey başlamaz ve satır önce script
extender'ı kurmanızı söyler. Heroic dışındaki bir GOG kopyası ya da Eidos'un
aktaramadığı bir mağaza kimliği için oyunu o mağazadan başlatmanızı söyler.

Bir profilin oyun için kendi kısayolu olabilir. Profile sağ tıklayıp
**Desktop shortcut** seçeneğini seçin: kısayol
`eidos tool <instance> run "<game>" --profile <name>` komutunu çalıştırır; bu
komut, pencerede seçmekle aynı şekilde o profili etkin profil yapar ve ardından
mağazadan oyunu başlatmasını ister. Mağazanın açtığı Eidos penceresi o profili
gösterir. `--profile` her `eidos tool ... run` ile çalışır ve profili araç
başlamadan önce seçer; `--print` ile hiçbir şey seçmez.

## Kendi başına bir Steam uygulaması olan araç

Creation Kit ayrı bir Steam uygulamasıdır ve kendi AppID'sini ister; Steam
üzerinden dağıtılan birkaç modlama aracı daha aynıdır. Girdide **Steam AppID**
ayarlayın; Eidos onu oyununkinin yerine o kimlikle başlatır.

Windows'ta bu, farklı bir başlatıcı demektir. Burada ise aynı çalıştırmaya iki
ortam değişkeni, `SteamAppId` ve `SteamGameId` eklenir. Eidos ikisini de ayarlar,
çünkü birini Proton, ötekini Steam'in kendi kitaplıkları okur ve ikisini uyuşmaz
gören bir araç açık bir hata vermeden başarısız olur. `eidos tool ... --print`
gerçek çalıştırmanın tam olarak neyi alacağını gösterir.

## Ortam değişkenleri ve Proton'suz çalışan araçlar

`<instance>/tools.ini` içindeki üç anahtarın Executables penceresinde henüz bir
alanı yok; bu yüzden onları aracın bölümüne elle ekleyin. Her `env=NAME=value`
satırı, yalnızca o aracın çalıştırmaları için bir değişken ayarlar:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos bu satırları kendi ayarladığı her şeyden sonra ve profilin
`launch-env.ini` dosyasındaki değişkenlerden sonra uygular; bu yüzden burada verilen
bir değer ikisinin de yerini alır. Buna, Eidos'un modlarınızın getirdiği DLL'lerden
kurduğu `WINEDLLOVERRIDES` de dahildir: bunu ayarlayan bir araç, ihtiyaç duyduğu
her DLL'i içinde listelemelidir. Profilin wrapper'ları, oyunun komutunun etrafında
olduğu gibi bir aracın komutunun da etrafına girer; bkz.
[Profil başına değişkenler ve wrapper'lar](usage.md#profil-başına-değişkenler-ve-wrapperlar).
Bir ad harflerden, rakamlardan ve `_` karakterinden oluşur ve rakamla başlamaz.
Başka bir ada sahip satır yok sayılır.

`runner=native`, `exe`'yi Proton olmadan doğrudan çalıştırır. Birleşik görünüm
her araçta olduğu gibi bağlanır ve `arg0=`, `arg1=` satırları onun
argümanlarıdır. Bunu bir Linux programı için ya da Steam'in yönetmediği ve
Heroic'in çalıştırıcı kaydetmediği bir kopyanın aracı için kullanın: `exe` o
kopyanın Wine'ıdır, Windows programı ilk argümandır ve önek `env=` ile gelir.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` mutlak bir yol olmalıdır. Çalışma dizini varsayılan olarak `exe`'nin
bulunduğu klasördür, burada çalıştırıcınınki, bu yüzden `workdir=` ayarlayın.
Birleşik yoluna yalnızca `exe` yeniden yazılır: bir modun içindeki Windows
programı için, birleşik görünümün onu gösterdiği oyunun `Data` klasörü altındaki
yolunu verin. Yerel bir araç Proton adımlarını atlar: yukarıdaki kayıt defteri
anahtarı, Steam'in shader önbelleği, boş .NET kökleri ve Steam katmanının
`LD_PRELOAD`'dan çıkarılması.

Heroic ile kurulmuş bir GOG ya da Epic kopyasında, oyunun Heroic ayarları
(`GamesConfig/<appName>.json`) bir Wine ya da Proton sürümü belirtiyorsa
araçların böyle bir girdiye ihtiyacı yoktur: Eidos onları bu sürümle oyunun
Heroic önekinde çalıştırır ve `eidos prereqs <instance> --install` bu sürümü
kurucular ve winetricks için kullanır. Yalnızca Legendary ile kurulmuş bir
kopyanın Heroic ayarları yoktur.

`slr=1`, aracı ve ondan önceki kayıt defteri aktarımını, aracın Proton sürümünün
belirttiği Steam Linux Runtime kapsayıcısı içinde başlatır (`toolmanifest.vdf`
dosyasındaki `require_tool_appid`); Steam oyunu da böyle başlatır. Bu çalışma
zamanı bir Steam kitaplığında kurulu olmalıdır; değilse çalıştırma durur ve
kurulması gereken Steam uygulamasını söyler. `eidos prereqs` kurucularını ve
winetricks'i yine ana makinede çalıştırır ve `runner=native` olan bir araç bu
anahtarı yok sayar. Kapsayıcı ev klasörünüzü ve oyunun Steam kitaplığını görür.
Başka bir yerdeki, örneğin başka bir sürücüdeki bir araç, mod ya da çalışma
zamanı için `env=STEAM_COMPAT_MOUNTS=/o/yol` gerekir (birden çok yol `:` ile
ayrılır). `/usr/share/steam/compatibilitytools.d` altına kurulmuş bir Proton,
kapsayıcı içindeki `/usr` çalışma zamanına ait olduğundan, büyük olasılıkla
`slr=1` ile kullanılamaz. Kapsayıcı içinde bir araç çalıştırması henüz
denenmedi.

`eidos tool <instance> run <title> --print`, son komutu ve çalıştırmanın
devralınan ortamın üzerine ayarladığı her değişkeni, bu satırlar dahil, gösterir.
Oyunu başlatan bir girdi için bunun yerine mağaza komutunu yazdırır (bkz.
[Bir araca ulaşmak](#bir-araca-ulaşmak-gizlemek-sabitlemek-ve-bir-masaüstü-kısayolu)).

## BethINI Pie

Eidos BethINI Pie'ı birlikte getirmez. Nexus sayfasından indirdiğiniz sürümü bu örneğin `mods/` klasörüne ya da araç klasörünüze koyun; `Bethini.exe` araç listesinde görünür, PGPatcher ile aynı oyunlarda. Eidos üzerinden çalıştırıldığında profilin oyun INI kopyasını düzenler. Başlığında `bethini` geçen bir aracın her çalıştırmasından önce Eidos bu INI'lerin bir geri yükleme noktasını saklar; araç çubuğundaki **Backups**, sonra **INIs**, birini geri koyar. Kopya alındığında var olmayan bir INI olduğu gibi bırakılır. Pencere her türün en yeni 10 geri yükleme noktasını tutar ve birini geri yüklemek önce INI'leri o anki hâlleriyle yedekler; böylece yanlış bir seçim geri alınabilir.

## Bir aracın kendi ayarları yine kendisinindir

Eidos çoğu araç ayarını denetlemez; yanlış bir ayar genellikle hata vermeden
başarısız olur.

BodySlide'ın **Game Data Path** ayarı (Settings), üstündeki oyun klasörünü değil
oyunun `Data` dizinini göstermelidir. Bir düzey fazla yukarı ayarlanırsa toplu
inşa "All sets processed successfully" bildirir ve 1439 mesh'i oyunun asla
bakmayacağı bir yere yazar. Eidos onları `Overwrite/Root/` içinde yakalar; bu
yüzden oyun kurulumunuza dokunulmaz ve oyundaki tek belirti bedenlerinizin inşa
edilmemiş olmasıdır.

Bir BodySlide veya Outfit Studio çalıştırmasından önce, kazanan
`CalienteTools/BodySlide/Config.xml` içindeki `GameDataPath` veya `OutputDataPath`
çalıştırma sırasında oyunun `Data` klasörüne çıkmıyorsa durum satırı uyarır;
Diagnostics sekmesi de aynı uyarıyı ayarlanacak değerle bir kart olarak gösterir.
`eidos check <instance>` da bunu bildirir. Bir `S:` yolu, Eidos'un bir araç için
`S:` sürücüsünü eşlediği gibi, Steam kütüphanesi klasörü olarak okunur. Bir çıktı
modu için, örneğe ait `mods/` klasörünün içindeki bir `OutputDataPath` kabul edilir.
Diagnostics ayrıca `Overwrite/Root/` bir `meshes` veya `textures` klasörü
içerdiğinde de bir kart gösterir.

Araç çıktısı Overwrite'a aittir. Bir çalıştırma saklamaya değer bir şey
ürettiğinde, **Overwrite -> Create mod...** onu, başka herhangi biri gibi
sıralanabilen, devre dışı bırakılabilen ve kaldırılabilen sıradan bir moda
dönüştürür.
