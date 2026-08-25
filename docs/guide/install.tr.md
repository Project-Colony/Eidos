<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Eidos kurulumu

Üç giriş yolu. Hepsi aynı iki çalıştırılabilir dosyayı verir - `eidos` (komut
satırı) ve `eidos-gui` - artı Nexus'taki "Mod Manager Download" düğmesinin sizin
örneğinize inmesini sağlayan `nxm://` işleyicisi.

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
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

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
bkz. [usage.tr.md](usage.tr.md). Tam gezinti de oradadır.

## İsteğe bağlı: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` çekirdek FUSE
passthrough'unu açar. **Öntanımlı olarak kapalıdır ve neredeyse kesinlikle öyle
kalmasını istersiniz**: Skyrim SE üzerinde ölçüldüğünde oyunun kendi arşivlerini
ve eklentilerini açmasını engelliyor, böylece modlar sessizce yüklenmiyor. Bu
anahtar, düzeneği yeniden sınamak için var; önerildiği için değil.

Ayrıntılar ve o kararın arkasındaki ölçümler
[troubleshooting.tr.md](troubleshooting.tr.md) içinde.

## Şimdiden bir sorun mu var?

[troubleshooting.tr.md](troubleshooting.tr.md) ortam anahtarlarını, işlem
sayaçlarının nasıl okunacağını ve şimdiye dek birini ısırmış her sorunu anlatır.
