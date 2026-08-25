<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Araçlar: xEdit, BodySlide, DynDOLOD, FNIS

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
- örnekler arasında paylaşılan dizin için Ayarlar'da belirlediğiniz **araçlar
  klasörü** (Tools -> Tools folder) - `/mnt/Games/Tools` ve benzerleri.

Liste oyun başınadır, yani bir Skyrim örneğine Fallout'un düzenleyicisi asla
önerilmez. Arama dört düzey aşağıda durur, çünkü bir mod havuzu yüz binlerce
dosyadır ve bu, araç listesi her kurulduğunda çalışır; sembolik bağları da
izlemez. Böyle bulunan bir araç, tam olarak sizin elle yazdığınız gibi
yapılandırılır: çalışma zamanları adından gelir, aşağıdaki her şeyle aynı kurala
göre.

Bir araç başka bir yerdeyse ya da farklı argümanlar istiyorsanız, elle ekleyin -
aynı başlığa sahip bir kullanıcı girdisi, otomatik bulunan her şeyi geçersiz
kılar.

## Bir araç eklemek

GUI'de: **Tools -> Executables**, ardından Add. Komut satırından:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # kayıtlı olanları listele
eidos tool skyrimse run BodySlide         # birleşik görünüm üzerinden çalıştır
eidos tool skyrimse run BodySlide --print # komutu çalıştırmadan göster
```

Script extender, oyun ikili dosyası ve başlatıcı kendiliğinden algılanır;
yalnızca ek araçların kaydedilmesi gerekir.

### Nerede olursa olsun gerçek dosyayı gösterin

Çalıştırılabilir dosyayı gerçekte bulunduğu yerde kaydedin. Araç bir mod olarak
kurulduysa, orası mod klasörünün içidir:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Üç tür ön gereksinim

**Kademe 1 - paketle gelen DLL'ler** (`d3dx9_43`, `d3dcompiler_47`,
`d3dx11_43`). Eidos onları birlikte getirir ve başlatmada öneke kopyalar.
Yapılacak bir şey yok, ağ yok.

**Kademe 2 - winetricks fiilleri** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Bunlar registry anahtarlarını, GAC'yi ve CLR
barındırıcılarını yazar, dolayısıyla dosya kopyalanarak halledilemezler.
**Microsoft'tan indirirler**.

**Kademe 3 - çalışma zamanları** (`dotnet10`). Modern bir .NET çalışma zamanı,
kendi dizininde duran ve `DOTNET_ROOT` üzerinden bulunan 193 dosyadır: hiç
kaydedilmez, öneke hiçbir biçimde kurulmaz, dolayısıyla diğer iki kademeden
hiçbiri onu taşıyamaz. Eidos onu kendi indirir, ikili dosyaya gömülü bir sağlama
toplamına karşı denetler ve `~/.local/share/Colony/Eidos/runtimes/` içinde
önbelleğe alır - **herhangi bir örneğin dışında**, çünkü 78 MB ne oyun başınadır
ne de profil başına.

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
