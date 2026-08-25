<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Eidos üzerinden Fallout 4

Fallout 4'ün özel bir başlatma seçeneğine, adı değiştirilmiş bir çalıştırılabilir
dosyaya ya da sarmalayıcı betiğe gereksinimi yoktur. Bunu açıkça söylemeye değer,
çünkü F4SE için yazılmış diğer bütün Linux kılavuzları size aksini söyler - ve
öğütleri bir sonraki Steam güncellemesinde kırılır.

## Başlatma seçeneği

```
~/.local/bin/eidos-gui %command%
```

Steam'in Fallout 4 için başlatma hedefi `Fallout4Launcher.exe`'dir, asla
`Fallout4.exe` değil; dolayısıyla script extender'ı çalıştırmak aslında "Steam'e
nasıl başka bir program başlattırırım" sorusudur. Alışılmış yanıtlar `%command%`
ifadesini bash içinde yeniden yazar:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

ya da `f4se_loader.exe` dosyasını `Fallout4Launcher.exe` üzerine kopyalar; Steam bunu
her oyun güncellemesinde sessizce geri getirir - ondan sonra F4SE olmadan oynarsınız
ve hiçbir şey bunu söylemez.

Eidos değiştirmeyi oyun tanımlayıcısından yola çıkarak kendi yapar: kurulu bir
`f4se_loader.exe` varsa başlatıcıyı onunla değiştirir, yoksa `Fallout4.exe`'ye geri
düşer ve geri düşmek zorunda kaldığında **size söyler**. Bütün F4SE modları ölü
hâlde açılan bir oyun, hiç açılmayan bir oyundan kötüdür.

Başlatıcıyı asla çalıştırmamak için ikinci bir neden daha var: `Data`'yı yeniden
tarayıp `plugins.txt` dosyasını yeniden yazar ve az önce yerleştirilen yükleme
sırasını bozar. Eidos onu hiçbir zaman çalıştırmaz.

## Eidos'un sizin adınıza üstlendikleri

| | |
|---|---|
| Arşiv geçersizleştirme | `Fallout4Custom.ini` dosyasına `[Archive]` `bInvalidateOlderFiles=1` ve boş bir `sResourceDataDirsFinal=` yazılır; `Data` dışındaki serbest dosyaların görülebilmesini sağlayan iki anahtar. Oyun klasörüne değil, profile yazılır. |
| Yükleme sırası | Fallout 4'ün kullandığı yıldız biçiminde `plugins.txt` (`*` etkin demek), örtük Creation Club eklentileri için `Fallout4.ccc` gözetilerek |
| LOOT | Sıralama Skyrim'deki gibi çalışır - `eidos sort <instance>` `fallout4` masterlist'ini alır |
| Kayıtlar | `.fos` kayıtları ve `.f4se` yan kayıtları listelenir, kopyalanır ve profil başına tutulur; ayrıntı bölmesi kaydın kendi eklenti tablosunu okuduğu için, devre dışı bıraktığınız bir eklentiye gereksinen kayıt bunu siz yüklemeden önce söyler |
| Root modları | Bir modun çalıştırılabilir dosyanın yanına koyduğu her şey (F4SE'nin kendisi, ENB, bir `dxvk.conf`) Skyrim'dekiyle aynı `Root/` düzeneğiyle oraya iner |

## Sürüm meselesi

Fallout 4 artık 2019-2024 arasındaki donmuş oyun değil. Ağustos 2026 itibarıyla üç
canlı dal var ve biri için derlenmiş bir mod DLL'i ötekinde yüklenmez:

| Dal | Sürüm | F4SE |
|---|---|---|
| Klasik ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Bir mod listesi kurmadan önce bilmeye değer iki sonuç:

- **Gerçekte neye sahip olduğunuzu denetleyin.** Oyun kökünde `Creations/` ve
  `Mods/` klasörleri varsa 1.11.x çizgisindesiniz. Eidos'ta bir kaydın ayrıntı
  bölmesi onu yazan yapıyı da gösterir - Fallout bunu kaydın içine yazar, Eidos da
  "Game build" olarak yüzeye çıkarır.
- **Yeni çıkmış bir yama başlamak için iyi bir gün değildir.** F4SE genellikle bir
  Bethesda güncellemesinden bir iki gün sonra gelir, ama çoğu DLL modunun kaymalarını
  çözdüğü *Address Library for F4SE Plugins* kendi takvimiyle yürür. İkisinin
  arasında ekosistemin DLL yarısı yerdedir. DLL'siz modlar (dokular, ağlar,
  eklentiler) etkilenmez.

Düzeneğiniz çalışır çalışmaz Fallout 4 için Steam'in otomatik güncellemelerini
kapatın (Özellikler → Güncellemeler → "Bu oyunu yalnızca başlattığımda güncelle");
yoksa bir sonraki yama kurduğunuz her DLL'i kırar.

## Donanım notu: NVIDIA'da silah kalıntısı çökmeleri

Fallout 4'ün silah kalıntısı efekti, NVIDIA'nın Pascal kuşağından sonra desteklemeyi
bıraktığı bir PhysX türevi olan NVIDIA FleX üzerinde çalışır. Turing ve sonrası her
kartta - GTX 16, RTX 20'den RTX 50'ye - oyunu çökertir. Bu oyunun kendi hatasıdır;
Linux, Proton ya da Eidos ile ilgisi yoktur.

İki çözüm, hangisi olursa: oyunun ayarlarından "Weapon Debris" seçeneğini kapatın ya
da *Weapon Debris Crash Fix* (Nexus 48078) kurun; sonuncusu efekti değil parçacıkların
çarpışmasını devre dışı bırakır.

## Bir şey yanlış görünüyorsa

Genel denetim listesi [troubleshooting.tr.md](troubleshooting.md) içinde;
Fallout'a özgü ilk soru ise her zaman *gerçekte hangi çalıştırılabilir dosyanın
başladığı*. Eidos tam başlatma komutunu örneğin çalışma günlüğüne yazar, yani:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

`f4se_loader.exe` yazıyorsa değiştirme gerçekleşmiştir. `Fallout4Launcher.exe`
yazıyorsa F4SE, Eidos'un bulabileceği yere kurulmamıştır - yeri oyunun
çalıştırılabilir dosyasının yanıdır; modla yönetilen bir kurulumda bu, bir modun
`Root/` dizini (ya da elle kurulmuşsa oyun klasörünün kendisi) demektir.
