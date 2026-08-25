<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Eklentiler

Bir eklenti, Eidos'un parçası olmadan Eidos'a bir girdi ekler. Bir programı
adlandıran bir TOML bildirimi ve olsa olsa o programdan ibarettir.

Bildirimler `~/.config/Colony/Eidos/addons/` içinde durur, eklenti başına bir
`.toml`. Klasörü **View -> Extensions -> Open folder** ile açın, sonra
**Reload**'a basın - yeniden başlatma yok.

## Eidos'un içine neden hiçbir şey yüklenmiyor

Mod Organizer 2 eklentileri paylaşılan kitaplık olarak yükler, Python olanları da
Qt üzerinden barındırır. İkisi de buraya taşınmıyor. Rust'ın kararlı bir ABI'si
yok; dolayısıyla başka bir derleyiciyle - ya da başka bir eniyileme bayrağıyla,
ortak bir bağımlılığın başka bir özellik kümesiyle - derlenmiş paylaşılan bir
kitaplık, sürüm uyuşmazlığı değil tanımsız davranıştır. Üstelik Eidos'un
görsel bileşenleri derleme zamanında geneldir, yani ABI kararlı olsaydı bile bir
kitaplık geri verecek bir bileşen kuramazdı.

Bu yüzden eklenti, Eidos'un *çalıştırdığı* bir programdır. Pencereyi çökertemez,
bir mod listesini bozamaz ve Eidos güncellemeleri boyunca çalışmayı sürdürür.

## Bir araç

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # her oyun için atlayın
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

**View -> Extensions** altında bir Run düğmesiyle görünür ve ayrık başlar - Eidos
onu beklemez.

## Bir denetim

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Her tazelemede çalışır ve satır başına bir bulgu yazar:

```
level<TAB>title<TAB>detail
```

burada `level` `problem`, `advice` ya da `ok` olur. Ayrıntı isteğe bağlıdır.
Bilinen bir düzeyle başlamayan her şey yok sayılır; böylece ilerleme çıktısı ve
başıboş uyarılar, Eidos'un kendi denetimlerinden biri gibi görünen bir satır
üretemez. Bulgular **Health** sekmesinde, eklentinin adı önlerine eklenerek
görünür.

Bir denetimin üç saniyesi vardır. Aşan durdurulur ve kendine karşı bir sorun
olarak bildirilir - her tıklamanın ardından gelen aynı tazelemede çalıştığı için,
takılan bir denetim pencereyi dondururdu.

## Yer tutucular

Hem `args` hem `workdir` şunları açar:

| Yer tutucu      | Nedir                                        |
| --------------- | -------------------------------------------- |
| `{instance}`    | örneğin kökü                                 |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | etkin profilin adı                           |
| `{profile_dir}` | etkin profilin dizini                        |
| `{game}`        | oyun kimliği, örn. `skyrimse`                |
| `{game_name}`   | oyunun görünen adı                           |
| `{install}`     | oyunun kurulum dizini                        |
| `{data}`        | oyunun `Data` dizini                         |

Bilinmeyen bir yer tutucu boşaltılmak yerine tam yazıldığı gibi bırakılır; böylece
bir hata görünür biçimde başarısız olur, `--out {typo}` ifadesini
`--out --next-flag` haline getirmez. Yer tutucularının tümü çözülemeyen bir aracı
çalıştırmak reddedilir ve Eidos hangilerinin eksik olduğunu söyler.

## Bir eklentinin yapamayacakları

Değerleri alır ve çalışır; Eidos'u geri çağıramaz, mod listesini değiştiremez,
pencereye hiçbir şey çizemez. Bu bilinçlidir. MO2'nin eklentilerle karşıladığı ve
gerçekten içeriye uzanması GEREKEN şeyler - oyun desteği, kurucular, çakışma
motoru - burada sonradan takılmış değil, gömülüdür: bir oyun tanımı
`~/.config/Colony/Eidos/games/` içindeki kendi TOML'udur ve FOMOD ile BAIN
kurucuları yerlidir.
