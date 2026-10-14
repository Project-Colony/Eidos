<!-- eidos-i18n: source=docs/guide/install.md sha=c47b7d58351d0b72f6e700bf9acb7e3d78af6a5c -->

# Eidos kurulumu

Üç giriş yolu. Hepsi `eidos` (komut satırı), `eidos-gui` ve statik NIF önizleme yardımcısını, ayrıca Nexus'taki "Mod Manager Download" düğmesinin indirilen dosyayı örneğinize ulaştırmasını sağlayan `nxm://` işleyicisini sunar.

## Önce gerekenler

| | |
|---|---|
| **FUSE'lu Linux** | PATH'inizde `fusermount3`. Güncel her dağıtım onu getirir. |
| **Bir kez başlatılmış bir Proton oyunu** | Steam oyunun Wine önekini yalnızca ilk açılışta oluşturur ve Eidos onun içinde çalışır. |
| **`7z`** | Mod arşivlerini kurmak için. Çoğu dağıtımda `p7zip`. |

root yok, artalan süreci yok, `/etc/fuse.conf` düzenlemesi yok, gruplarınıza
eklenecek bir şey yok. Eidos, oyun sürecine ait özel bir ad alanının içinde
bağlar.

## Arch

```bash
cd packaging && makepkg -si
```

## Bir sürüm arşivi

```bash
./install.sh
```

Öntanımlı olarak `~/.local/bin` içine kurar. `--system` onu `/usr/local/bin`
içine, `--bindir DIR` başka herhangi bir yere koyar. Yeniden çalıştırmak,
yükseltmenin desteklenen yoludur.

## Kaynaktan

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Kaynaktan derleme için Rust, `just`, CMake 3.20+, bir C++17 derleyicisi ve yerel yardımcının testleri için Python 3 gerekir. `just` olmadan kullanılacak komutlar için [yerel yardımcı yönergelerine](../../../../../native/eidos-nif-preview/README.md) bakın; Rust'ı `cargo build --release --locked` ile derleyin, yardımcıyı `target/release` içine kopyalayın ve ardından `packaging/install.sh --from target/release` çalıştırın.

## Sonra: Steam'i ona yöneltin

Eidos, oyununuzun başlatma komutu *olarak* çalışır; oyun başlamadan önce
bağlamayı böyle başarır. Steam'de oyuna sağ tıklayın -> Özellikler -> Başlatma
Seçenekleri:

```
~/.local/bin/eidos-gui %command%
```

Oyna'ya basın. Eidos o oyunun örneğinde açılır; mod kurun, LOOT ile sıralayın,
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
bkz. [usage.tr.md](usage.md). Tam gezinti de oradadır.

## İsteğe bağlı: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` isteğe bağlı yetkiyi verir, ancak passthrough özelliğini etkinleştirmez.
`EIDOS_FUSE_PASSTHROUGH=1`, çalışma zamanında kullanılan ayrı anahtardır. **Öntanımlı olarak kapalıdır ve neredeyse kesinlikle öyle
kalmasını istersiniz**: Skyrim SE üzerinde ölçüldüğünde oyunun kendi arşivlerini
ve eklentilerini açmasını engelliyor, böylece modlar sessizce yüklenmiyor. Bu
anahtar, düzeneği yeniden sınamak için var; önerildiği için değil.

Ayrıntılar ve o kararın arkasındaki ölçümler
[troubleshooting.tr.md](troubleshooting.md) içinde.

## Şimdiden bir sorun mu var?

[troubleshooting.tr.md](troubleshooting.md) ortam anahtarlarını, işlem
sayaçlarının nasıl okunacağını ve şimdiye dek birini ısırmış her sorunu anlatır.
