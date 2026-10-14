<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Eidos kurulumu

Dört giriş yolu. Colony, programın tamamı olan tek bir dosya kurar: `eidos-linux`. Diğerleri `eidos` (komut satırı), `eidos-gui` ve statik NIF önizleme yardımcısını yan yana, ayrıca Nexus'taki "Mod Manager Download" düğmesinin indirilen dosyayı örneğinize ulaştırmasını sağlayan `nxm://` işleyicisini sunar.

## Önce gerekenler

| | |
|---|---|
| **FUSE'lu Linux** | PATH'inizde `fusermount3`. Güncel her dağıtım onu getirir. |
| **Bir kez başlatılmış bir Proton oyunu** | Steam oyunun Wine önekini yalnızca ilk açılışta oluşturur ve Eidos onun içinde çalışır. |
| **`7z`** | Mod arşivlerini kurmak için. Çoğu dağıtımda `p7zip`. |

root yok, artalan süreci yok, `/etc/fuse.conf` düzenlemesi yok, gruplarınıza
eklenecek bir şey yok. Eidos, oyun sürecine ait özel bir ad alanının içinde
bağlar.

## Colony üzerinden

[Colony](https://github.com/Project-Colony/Colony) içinde **Eidos**'u arayın ve
kurun; güncellemeler başlatıcı üzerinden gelir. Colony tek bir dosya tutar:
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Onu çalıştırın, pencere açılır.
`eidos` adlı bir bağlantı üzerinden çağrıldığında aynı dosya komut satırıdır.
`PATH`'inize bir tane koyun ve Nexus indirme işleyicisini kaydedin:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # sitenin Mod Manager Download düğmesi artık Eidos'a ulaşır
```

Pencerenin bağlantıya ihtiyacı yoktur: bir oyunu ya da aracı her başlattığında
kendi dosyasını komut satırı olarak çalıştırır. NIF model önizlemesi
`eidos-nif-preview` gerektirir; onu yalnızca sürüm paketi ve kaynaktan
derlemeler içerir. O olmadan önizleme bunu söyler ve başka hiçbir şey
etkilenmez. Steam'i aşağıda olduğu gibi dosyanın kendisine yöneltin.

## Bir sürüm paketi

[Son sürümden](https://github.com/Project-Colony/Eidos/releases/latest)
`eidos-linux-bundle.tar.gz` dosyasını ve onun `.sig`, `.meta` ve `.meta.sig`
dosyalarını indirin ve README'deki
[Kod imzalama politikası](../../README.md#kod-imzalama-politikası) bölümünün
gösterdiği gibi denetleyin. Ardından:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Öntanımlı olarak `~/.local/bin` içine kurar. `--system` onu `/usr/local/bin`
içine, `--bindir DIR` başka herhangi bir yere koyar. Yeniden çalıştırmak,
yükseltmenin desteklenen yoludur. Paket ve `eidos-linux`, glibc 2.39 ya da daha
yenisine sahip x86_64 Linux gerektirir.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD, güncel `main` dalını kaynaktan derler.

## Kaynaktan

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Kaynaktan derleme için Rust 1.89 ya da daha yenisi, `just`, CMake 3.20+, bir C++17 derleyicisi ve yerel yardımcının testleri için Python 3 gerekir. `just` olmadan kullanılacak komutlar için [yerel yardımcı yönergelerine](../../../../../crates/eidos-nif-preview/README.md) bakın; Rust'ı `cargo build --release --locked` ile derleyin, yardımcıyı `target/release` içine kopyalayın ve ardından `packaging/install.sh --from target/release` çalıştırın.

## Sonra: Steam'i ona yöneltin

Eidos, oyununuzun başlatma komutu *olarak* çalışır; oyun başlamadan önce
bağlamayı böyle başarır. Steam'de oyuna sağ tıklayın -> Özellikler -> Başlatma
Seçenekleri:

```
~/.local/bin/eidos-gui %command%
```

Colony'den kurduysanız yol `~/.local/share/Colony/apps/Eidos/eidos-linux`
olur. Oyna'ya basın. Eidos o oyunun örneğinde açılır; mod kurun, LOOT ile sıralayın,
Run'a tıklayın. Çıktığınızda bağlama da onunla gider ve kurulumunuz tam olarak
eskisi gibidir.

Mutlak yolu kullanın - Steam kabuğunuzun `PATH`'ini okumaz.

### Terminali yeğliyorsanız

```sh
eidos init skyrimse               # bir örnek oluştur (klasör verirseniz taşınabilir olur)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root modları
eidos sort skyrimse               # yükleme sırasını LOOT ile sırala
eidos play skyrimse -- %command%  # herhangi bir şeyi birleşik görünüm üzerinden çalıştır
```

Oyun kimliği alan her komut, taşınabilir bir örneğin klasörünü de alır -
bkz. [usage.md](usage.md#örnekler-global-ve-taşınabilir). Tam gezinti
[usage.md](usage.md) içindedir.

## İsteğe bağlı: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` isteğe bağlı yetkiyi verir, ancak passthrough özelliğini etkinleştirmez.
Bu yetki paketin ya da bir kaynak derlemesinin ayrı `eidos` ikili dosyasına
verilir, hiçbir zaman aynı zamanda pencere de olan `eidos-linux` dosyasına
verilmez ([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1`, çalışma zamanında kullanılan ayrı anahtardır. **Öntanımlı olarak kapalıdır ve neredeyse kesinlikle öyle
kalmasını istersiniz**: Skyrim SE üzerinde ölçüldüğünde oyunun kendi arşivlerini
ve eklentilerini açmasını engelliyor, böylece modlar sessizce yüklenmiyor. Bu
anahtar, düzeneği yeniden sınamak için var; önerildiği için değil.

Ayrıntılar ve o kararın arkasındaki ölçümler
[troubleshooting.md](troubleshooting.md) içinde.

## Şimdiden bir sorun mu var?

[troubleshooting.md](troubleshooting.md) ortam anahtarlarını, işlem
sayaçlarının nasıl okunacağını ve şimdiye dek birini ısırmış her sorunu anlatır.
