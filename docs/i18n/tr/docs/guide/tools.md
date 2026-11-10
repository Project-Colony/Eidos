<!-- eidos-i18n: source=docs/guide/tools.md sha=631027d4686fac9eb78728b23a8024d0c5000909 -->

# Araçlar: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Eidos üzerinden çalıştırılan bir araç, oyunun kendi Proton öneki içinde
**birleşik görünümü** görür. Oyunun okuyacağını okur - etkin her mod, öncelik
sırasıyla - ve yazdığı ne varsa Overwrite'a iner; orada tek tıkla gerçek bir mod
olur.

## Eidos'un kendiliğinden bulduğu araçlar

Bazı araçlar bildirilmek yerine bulunacak kadar ayırt edici adlara sahiptir ve
xEdit bunun bariz örneğidir: Fallout 4 için `FO4Edit.exe`, Skyrim SE için
`SSEEdit.exe`, orijinali için `TES5Edit.exe` ve böyle sürer - her birinin
**QuickAutoClean** ikiziyle birlikte; o da LOOT'un uyarıp durduğu kirli
düzenlemelerin düğmesidir. Eidos onları dosya adına göre şuralarda arar:

- oyunun kurulum klasörü ve etkin modların `Root/` ağaçları;
- MO2 kullanıcılarının araçları kurduğu yer olan **bu örneğin `mods/`** dizini;
- örnekler arasında paylaşılan dizin için Tercihler'de belirlediğiniz **araçlar
  klasörü** (General -> Tools) - `/mnt/Games/Tools` ve benzerleri.

`PGPatcher.exe` Creation Engine oyunlarinda ayni sekilde bulunur ve onsuz calisamayacagi bir argumanla gelir - bkz. [asagisi](#pgpatcher-neden---ignore-mo2vfscheck-ister).

Liste oyun başınadır, yani bir Skyrim örneğine Fallout'un düzenleyicisi asla
önerilmez. Arama dört düzey aşağıda durur, çünkü bir mod havuzu yüz binlerce
dosyadır ve bu, araç listesi her kurulduğunda çalışır; sembolik bağları da
izlemez. Böyle bulunan bir araç, tam olarak sizin elle yazdığınız gibi
yapılandırılır: çalışma zamanları adından gelir, aşağıdaki her şeyle aynı kurala
göre.

Bir araç başka bir yerdeyse ya da farklı argümanlar istiyorsanız, elle ekleyin -
aynı başlığa sahip bir kullanıcı girdisi, otomatik bulunan her şeyi geçersiz
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

xEdit'in yedekleri, önbelleği ve günlüğü birleşik görünümün dışında,
`<instance>/logs/qac-<time>/` içine gider ve durum satırı o günlükteki sayıları
bildirir. Eidos bu klasörü başarılı bir temizlikten sonra siler, başarısız bir
temizlikten sonra tutar. Eidos her kopyanın yapıldığı eklentinin boyutunu ve
tarihini `Cleaned plugins/meta.ini` içine kaydeder. Bu eklenti değiştiğinde Health sekmesi kopyayı eski olarak bildirir ve gizlemeyi önerir.

## Bir araç eklemek

GUI'de: **Tools -> Executables**, ardından Add. Komut satırından:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # kayıtlı olanları listele
eidos tool skyrimse run BodySlide         # birleşik görünüm üzerinden çalıştır
eidos tool skyrimse run BodySlide --print # komutu çalıştırmadan göster
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` bu çalıştırmanın çıktısını, aracın kendi ayarı ne olursa olsun, `mods/<mod>` içine alır.

Script extender, oyun ikili dosyası ve başlatıcı kendiliğinden algılanır;
yalnızca ek araçların kaydedilmesi gerekir.

### Nerede olursa olsun gerçek dosyayı gösterin

Çalıştırılabilir dosyayı gerçekte bulunduğu yerde kaydedin. Araç bir mod olarak
kurulduysa, orası mod klasörünün içidir:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(bu, global örneğin yoludur - taşınabilir bir örnekte aynı kural kendi klasörü
altında geçerlidir, `<instance>/mods/...`; şunu da not edin: böyle mutlak bir
yol, taşınabilir bir klasörü sonradan TAŞIMAKTAN sağ çıkmayan tek şeydir).

Eidos, başlatmadan önce o yolu birleşik olanla değiştirir, böylece araç
`<game>/Data/CalienteTools/BodySlide/` içinden çalışır ve oradaki diğer tüm
modların dosyalarını da görür. Bu, kulağa geldiğinden daha önemlidir: BodySlide
**boş** bir `SliderSets` dizini ile gelir ve inşa edebileceği her beden CBBE'den
ve kıyafet modlarından gelir. Kendi mod klasöründen başlatıldığında hiçbir şey
bulamaz ve bozuk görünür.

MO2 da aynı nedenle aynı değiştirmeyi yapar - kendi yorum satırı FNIS'i anar.

**Devre dışı** bir moddaki araç için bu değiştirme yapılamaz, çünkü dosyaları
görünümde de değildir. Eidos bunu söyler ve numara yapmak yerine aracı kendi
klasöründen çalıştırır.

## Mod Organizer 2 listesinden araçlar

Eidos örneği olarak açılan bir Wabbajack listesi ya da başka bir MO2 örneği `ModOrganizer.ini` dosyasını korur ve orada tanımlı çalıştırılabilir dosyalar `tools.ini` içine kopyalanabilir:

```sh
eidos tool /path/to/list import-mo2           # nelerin ekleneceğini gösterir
eidos tool /path/to/list import-mo2 --write   # ekler
```

Pencerede Çalıştırılabilir Dosyalar iletişim kutusunda **ModOrganizer.ini'den içe aktar** düğmesi vardır; her girdi için işareti kaldırılabilen bir kutu bulunur. Kaydet'e basana kadar hiçbir şey kaydedilmez.

`Z:` üzerindeki yollar Linux yollarıdır ve olduğu gibi kalır. Başka bir sürücü harfindeki yol, listenin klasörünün adını içeriyorsa o klasörün altına taşınır (`D:\Lists\MyList\tools\SSEEdit.exe`, `<list>/tools/SSEEdit.exe` olur); bu çalıştırılabilir dosyada, çalışma dizininde ve bağımsız değişkenlerde geçerlidir. Betik genişleticinin yükleyicisi ile oyunun ikili dosyası düz ada dönüşür, böylece yerleşik girdiler gibi Eidos'un görünümü bağladığı oyun klasöründen çalışırlar. Başka yere işaret eden girdiler (bir Windows sistem programı, liste dışındaki bir araç) nedeniyle birlikte atlanmış olarak listelenir, `tools.ini` içinde zaten bulunan bir başlık ise hiçbir zaman değiştirilmez.

## Bir aracın çıktısını kendi moduna göndermek

Bir üreteç - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - yüzlerce dosya
yazar. Öntanımlı olarak bunlar diğer her şeyle birlikte Overwrite'a iner.
Executables düzenleyicisinde **Capture output into** ayarını verin; bu
çalıştırmanın çıktısı onun yerine o moda gider:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Mod yoksa oluşturulur. Yalnızca BU çalıştırmanın ürettiği dosyalar taşınır;
Overwrite'ta zaten bulunan ne varsa orada kalır, böylece yakalama hedefi olan
iki araç birbirinin çıktısını çalmaz. Hiçbir şey yazmayan bir çalıştırma,
ardında boş bir mod bırakmaz.

Bu iş, MO2'nin yaptığı gibi yazma katmanını moda yönelterek değil, çalıştırmadan
sonra yapılır. Yazma katmanını bir moda yöneltmek, o modu çalıştırma boyunca en
yüksek önceliğe çıkarır - içinde bulunduğu her çatışmayı ters çevirip sonra geri
çevirir - ve modun kendi dosyalarının üzerine copy-up olmadan doğrudan yazar.
Yakalama, ikisi de olmadan aynı son duruma varır.

Hedef mod devre dışıysa çıktı yine yazılır ama oyun onu görmez, dolayısıyla araç
bir sonraki çalıştırmada aynı dosyaları yeniden üretir. Eidos durum böyleyken
uyarır.

### Çıktısı güncelliğini yitirdiğinde

Eidos, bir aracın çıktısının hangi modlardan, hangi eklenti sırasından, hangi profilden ve hangi çalıştırılabilir dosyadan üretildiğini kaydeder. Bunlardan biri değişirse Diagnostics sekmesi bunu bildirir; kartta aracı şimdiki ayarlarıyla çalıştıran bir **Run <tool> again** düğmesi bulunur.

Güncel olmayan çıktısı oyunu bozan bir üretici için, örneğin Pandora veya Nemesis, `tools.ini` içindeki girdisine `check_before_play=1` ekleyin ya da Executables penceresinde **Ask before playing when its output is out of date** seçeneğini açın. Run bu durumda önce sorar: aracı çalıştır ya da yine de oyna. Yine de oyna seçimi, modlar, eklentiler veya profil yeniden değişene ya da pencere kapanana kadar hatırlanır. Eidos aracı kendiliğinden hiç başlatmaz.

## Bir aracın gereksindiği DLL'ler ADINA göre seçilir

Şaşırtıcı olan kısım budur, o yüzden düz düz söylemeye değer: **bir araca
verdiğiniz başlık, Eidos'un onun için hangi çalışma zamanı ön gereksinimlerini
sağlayacağını belirler.** Eşleşme, başlığın büyük/küçük harf duyarsız bir alt
dizesidir.

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

Yani **`BodySlide`** olarak kaydedilen bir araç DirectX DLL'lerini alır; aynı
çalıştırılabilir dosya **`BS`** olarak kaydedilirse hiçbir şey almaz ve
DLL'lerden hiç söz etmeyen bir hatayla başlamayabilir. Araçları programın adıyla
adlandırın.

Liste `default_prereqs` içindedir (`crates/eidos-instance/src/tools.rs`) ve
Executables iletişim kutusundaki `Prereqs` alanı düzenlenebilir - algılama bir
öntanımdır, kural değil.

Baslik **argumanlara** da ayni sekilde karar verir; kimsenin tahmin
edemeyecegi bir arguman isteyen tek arac icin: `pgpatcher` (ya da eski adi
`parallaxgen`) iceren bir baslik `--ignore-mo2vfscheck` alir. Nedeni ve neden
istege bagli olmadigi [asagisi](#pgpatcher-neden---ignore-mo2vfscheck-ister).

### Üç tür ön gereksinim

**Kademe 1 - paketle gelen DLL'ler** (`d3dx9_43`, `d3dcompiler_47`,
`d3dcompiler_43`, `d3dx11_43`). Eidos onları birlikte getirir ve başlatmada
öneke kopyalar. Yapılacak bir şey yok, ağ yok.

**Kademe 2 - winetricks fiilleri** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Bunlar registry anahtarlarını, GAC'yi ve CLR
barındırıcılarını yazar, dolayısıyla dosya kopyalanarak halledilemezler.
**Microsoft'tan indirirler**.

**Kademe 3 - çalışma zamanları** (`dotnet10`, `dotnetsdk10`). Modern bir .NET çalışma zamanı,
kendi dizininde duran ve `DOTNET_ROOT` üzerinden bulunan 193 dosyadır: hiç
kaydedilmez, öneke hiçbir biçimde kurulmaz, dolayısıyla diğer iki kademeden
hiçbiri onu taşıyamaz. Eidos onu kendi indirir, ikili dosyaya gömülü bir sağlama
toplamına karşı denetler ve `~/.local/share/Colony/Eidos/runtimes/` içinde
önbelleğe alır - **herhangi bir örneğin dışında**, çünkü 78 MB ne oyun başınadır
ne de profil başına.

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
`tools.ini` içinde hâlâ
eski varsayılanı, `dotnet8`, `vcrun2022` taşıyan bir Synthesis girdisi, Eidos
onu okurken `dotnetsdk10`, `vcrun2022` olur.

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

Kademe 2 ya da 3'te hiçbir şey sessizce çalışmaz:

```sh
eidos prereqs skyrimse            # kayıtlı araçların neye gereksindiğini ve durumlarını göster
eidos prereqs skyrimse --install  # eksik olanı getir (indirir)
```

GUI'de aynı durumlar Prereqs alanının altında durur ve eksik olanlar birer
düğmedir. Ne paketle gelen, ne bir çalışma zamanı, ne de bilinen bir winetricks
fiili olan bir fiil, indirme olarak sunulmak yerine olası bir yazım hatası diye
bildirilir.

### DynDOLOD neden `dotnet10` gereksinir

DynDOLOD nesne LOD'unu kendi inşa etmez: işi LODGen'e havale eder ve üç tanesini
birlikte getirir. `LODGenx64.exe`, Proton altında Wine'ın Mono'suna yönlendirilen
.NET Framework 4.8'i hedefler - ki onun `System.Uri` başlatıcısı Mono'nun
uygulamadığı bir yöntemi çağırır. İlk iş satırından önce ölür; geriye yalnızca
bir sürüm başlığı tutan bir günlük ve yalnızca "failed for one or more worlds"
diyen bir DynDOLOD iletişim kutusu kalır.

Gerçek .NET Framework'ü kurmak bunu düzeltmez: Proton, onu bulacak yükleyici olan
`mscoree.dll`'i kendi ağacına giden bir sembolik bağla değiştirir ve bunu her
önek güncellemesinde yeniden yapar.

İşe yarayan yapı `LODGenx64Win10.exe`; modern .NET'i hedefler ve `mscoree`'ye hiç
dokunmaz. `DOTNET_ROOT`'u bir .NET 10 çalışma zamanına yöneltin, çalışır.
`dotnet10`'un sağladığı budur ve Eidos, onu bildiren herhangi bir aracı
başlatırken değişkeni ayarlar.

Eidos, sistemdeki `winetricks`'i Proton'un kendi `wine`'ı ve oyun öneki üzerinde
çalıştırır; bu da Steam'in pressure-vessel kabını ve protontricks + Proton-GE
uyuşmazlığını atlar. Kurulmamış bir Kademe 2 fiili bildiren bir araç yine de
başlar; fiili ve onu düzeltecek komutu anan bir uyarıyla - kullanıcıda başka bir
yerden gelmiş olabilir.

### PGPatcher neden `--ignore-mo2vfscheck` ister

PGPatcher, sahip oldugunuz doku modlarinin - vanilla, parallax, complex
material, PBR - her yuzeyde dogru shader'i kullanmasi icin mesh'leri ve
eklentileri yeniden yazar. Bir mod yoneticisi disinda calismayi hakli olarak
reddeder: ciplak bir oyun klasorunu yamamak yikici olurdu.

Sorun bunu NASIL denetledigidir: Windows'ta MO2'nin oyuna enjekte ettigi DLL
kanca katmani olan **usvfs**'i arar. Eidos'ta boyle bir sey yok ve hicbir zaman
olmayacak - birlesik gorunum, Wine'in varligindan habersiz icine girdigi bir
FUSE baglamasidir - dolayisiyla denetim hicbir sey bulamaz ve arac hemen su
mesajla cikar

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

kullanicinin calistirmadigi bir programi adlandirarak, herhangi bir is
yapmadan once. Yazarin iki Linux bildirimine ([#716][pg716] ve [#725][pg725],
ikisi de ayni nedenle FUSE tabanli olan MO2'nin Linux catallanmasi Fluorine'den)
kendi yaniti, denetimi atlayan bir bayraktir.

Bu yuzden Eidos onu onceden koyar. Bir mod havuzunda bulunan PGPatcher argumani
ayarlanmis olarak gelir ve Executables penceresinde baslik olarak `PGPatcher`
yazmak Arguments alanini ayni sekilde doldurur. Yazdiginiz her zaman kazanir:
yalnizca bos biraktiginiz bir alan doldurulur, tipki yukaridaki on kosullar
gibi.

Liste `default_args` icinde (`crates/eidos-instance/src/tools.rs`),
`default_prereqs` yaninda. Tek girdisi var ve bir tanesi bir islevi hakli
cikarmaya yeter: bir bayragi kimse tahmin edemez ve onledigi hata yanlis
programi adlandirir.

Calisirken hicbir sey istemez, bir .NET araci icin alisilmadik bicimde: kendi
calisma zamanini tasir (`dotnetlib/`, `includedFrameworks` bildiren bir
`runtimeconfig.json` ile) ve Microsoft CRT DLL'lerini yurutulebilir dosyanin
yanina koyar. Prereqs alanini bos birakin.

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
3. `--ignore-mo2vfscheck` bağımsız değişkenini aracın bağımsız değişkenlerinde
   tutun. Bu modda PGPatcher usvfs'yi arar ve bulamazsa durur.

PGPatcher klasörü yalnızca içinde bir `modorganizer.ini` varsa kabul eder; bu
yüzden Eidos, oyunu ya da bir aracı her başlattığında, başlatılan profil için
örneğin kök klasörüne `ModOrganizer.ini` yazar. Dosya yalnızca içeriği
değiştiğinde yeniden yazılır. İçinde şunlar bulunur:

- `[General]` içinde: `gameName` (oyunun MO2'deki adı, yani Fallout: New Vegas için `New Vegas`), `gamePath` (oyunun kurulum klasörü, `Z:` yolu
  olarak) ve `selected_profile`;
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

Eidos kendi dosyasını yalnızca bir şey başlatırken yazar, bu yüzden iki başlatma
arasında dosya sonuncunun profilini gösterir. Eidos'un başlattığı bir araç orada
kendi profilini bulur.

PGPatcher, profilde etkin hiçbir mod olmadığında ve çıktı klasörü etkin bir
modun klasörü olduğunda, MO2 altında olduğu gibi bir hatayla durur.

## Önekteki oyun yolu

Windows araçları oyunlarını `HKLM\Software\Bethesda Softworks\<game>`
`installed path` okuyarak bulur; bu anahtarı oyunun kendi kurucusu yazar - ve
Proton altındaki Steam o kurucuyu hiç çalıştırmaz. O olmadan xEdit, Wrye Bash ve
DynDOLOD boş bir yolla açılır. Eidos onu bir aracı çalıştırmadan önce yazar:
idempotent, ekleyici ve önek ilklendirilmemişse ya da kullanımdaysa atlanır.

## Bir araca ulaşmak: gizlemek, sabitlemek ve bir masaüstü kısayolu

Bir oyunun öntanımlıları hiç kullanmayacağınız araçları içerir ve ikinciye
ulaşmak için sekiz girdi sıralayan bir seçici, kimsenin okumadığı bir seçicidir.
Executables iletişim kutusunda:

- **Pin to top** bir girdiyi Run listesinin başına koyar.
- **Hide from picker** bir girdiyi silmeden listeden çıkarır.
- **Desktop shortcut** `~/.local/share/applications` içine bir `.desktop` yazar -
  bir freedesktop sisteminde bir başlatıcının ait olduğu yer orasıdır, dolayısıyla
  masaüstünde değil uygulama menünüzde ve aramada belirir. Doğrudan
  `eidos tool <instance> run <title>` çalıştırır; bu da aracın, Eidos penceresi
  hiç açık olmadan **birleşik görünüm üzerinden ve bu örneğin profiliyle**
  geldiği anlamına gelir.

Gizleme ve sabitleme, bir aracın ne çalıştırdığıyla değil ona nasıl *ulaşıldığıyla*
ilgilidir, dolayısıyla kendi girdilerinize olduğu gibi oyun başına
öntanımlılara da uygulanır.

Script extender'ın loader'ı, başlatıcı ve oyun ikili dosyasında Desktop
shortcut düğmesi yoktur. Bunlar oyunu başlatır ve oyun her zaman kendi mağazası
üzerinden başlar; bunu mağazanın kendi kısayolu zaten yapar. Eidos 1.22 veya
daha eski bir sürümle bunlardan biri için oluşturduğunuz kısayol çalışmaya
devam eder: bir terminale yazılan `eidos tool <instance> run <title>` gibi,
mağazadan oyunu başlatmasını ister; hiçbir mağazanın açmadığı bir pencerede
Run'ın yaptığı gibi. Mağazadaki ayar eksikse komut hiçbir şey başlatmaz ve
neyin ayarlanacağını yazdırır. Kısayol hiçbir ileti göstermez, bu yüzden okumak
için aynı komutu bir terminalde çalıştırın.

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

Windows'ta bu, farklı bir başlatıcı demektir. Burada ise zaten kurulmakta olan
çalıştırmaya iki ortam değişkeni eklemek demektir - `SteamAppId` ve
`SteamGameId`, ikisi birden, çünkü birini Proton, ötekini Steam'in kendi
kitaplıkları okur ve ikisini uyuşmaz gören bir araç, açıkça değil tuhaf biçimde
başarısız olur. `eidos tool ... --print` gerçek çalıştırmanın tam olarak neyi
alacağını gösterir.

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

Eidos bu satırları kendi ayarladığı her şeyden sonra uygular; bu yüzden burada
verilen bir değer Eidos'un kendi değerinin yerini alır. Buna, Eidos'un
modlarınızın getirdiği DLL'lerden kurduğu `WINEDLLOVERRIDES` de dahildir: bunu
ayarlayan bir araç, ihtiyaç duyduğu her DLL'i içinde listelemelidir. Bir ad
harflerden, rakamlardan ve `_` karakterinden oluşur ve rakamla başlamaz. Başka
bir ada sahip satır yok sayılır.

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
winetricks'e verir. Yalnızca Legendary ile kurulmuş bir kopyanın Heroic
ayarları yoktur.

`slr=1`, aracı ve ondan önceki kayıt defteri aktarımını, aracın Proton sürümünün
belirttiği Steam Linux Runtime kapsayıcısı içinde başlatır (`toolmanifest.vdf`
dosyasındaki `require_tool_appid`); Steam oyunu da böyle başlatır. Bu çalışma
zamanı bir Steam kitaplığında kurulu olmalıdır; değilse çalıştırma durur ve
kurulması gereken Steam uygulamasını söyler. `eidos prereqs` winetricks'i yine
ana makinede çalıştırır ve `runner=native` olan bir araç bu anahtarı yok sayar.
Kapsayıcı ev klasörünüzü ve oyunun Steam kitaplığını görür. Başka bir yerdeki,
örneğin başka bir sürücüdeki bir araç, mod ya da çalışma zamanı için
`env=STEAM_COMPAT_MOUNTS=/o/yol` gerekir (birden çok yol `:` ile ayrılır).
`/usr/share/steam/compatibilitytools.d` altına kurulmuş bir Proton, kapsayıcı
içindeki `/usr` çalışma zamanına ait olduğundan, büyük olasılıkla `slr=1` ile
kullanılamaz. Kapsayıcı içinde bir araç çalıştırması henüz denenmedi.

`eidos tool <instance> run <title> --print`, son komutu ve çalıştırmanın
devralınan ortamın üzerine ayarladığı her değişkeni, bu satırlar dahil, gösterir.

## Bir aracın kendi ayarları yine kendisinindir

Eidos bir aracı doğru DLL'lerle doğru yere koyar. Aracın sonrasında kendi
yapılandırmasıyla ne yaptığı sizinle o araç arasındadır ve başarısızlık genellikle
sessizdir.

İşlenmiş örnek, aksi halde bir saate mal olduğu için: BodySlide'ın **Game Data
Path** ayarı (Settings), üstündeki oyun klasörünü değil oyunun `Data` dizinini
göstermelidir. Bir düzey fazla yukarı ayarlayın; toplu inşa "All sets processed
successfully" bildirir ve 1439 mesh'i oyunun asla bakmayacağı bir yere yazar.
Eidos onları yakalar - kurulumunuza değil `Overwrite/Root/` içine inerler - ama
oyunun bakış açısından, bedenlerinizin inşa edilmemiş olması dışında yanlış
hiçbir şey yoktur.

Araç çıktısı Overwrite'a aittir. Bir çalıştırma saklamaya değer bir şey
ürettiğinde, **Overwrite -> Create mod...** onu, başka herhangi biri gibi
sıralanabilen, devre dışı bırakılabilen ve kaldırılabilen sıradan bir moda
dönüştürür.
