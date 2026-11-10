<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Eklentiler

Bir eklenti, Eidos'un parçası olmadan Eidos'a bir girdi ekler. Bir programı
adlandıran bir TOML bildirimi ve olsa olsa o programdan ibarettir.

Bildirimler `~/.config/Colony/Eidos/addons/` içinde durur, eklenti başına bir
`.toml`. Klasörü View -> Extensions -> Open folder ile açın, sonra Reload'a
basın; yeniden başlatmak gerekmez.

`id`, `kind` ve `exec` zorunludur; `name` verilmezse `id` kullanılır. `exec`
programı adlandırır. İçinde `/` olmayan bir ad `PATH` içinde aranır. Program
yoksa Eidos eklentiyi atlar ve View -> Extensions altındaki satırı nedenini
söyler (`<exec> is not on PATH` ya da `<exec> is not there`). `games`
eklentiyi verilen oyun kimlikleriyle sınırlar; verilmezse eklenti her oyunda
geçerlidir, başka bir oyunda satırında "not for this game" yazar.

Eidos'un reddettiği bir bildirim, nedeniyle birlikte listenin başında gösterilir:
okunamayan bir dosya, geçerli TOML olmayan veya zorunlu alanı eksik bir dosya,
boş ya da boşluk içeren bir `id`, boş bir `exec`, önceki bir bildirimin zaten
aldığı bir `id` (dosyalar ad sırasıyla okunur), Eidos'un tanımadığı bir `kind`,
ya da `protocol = 1` ve `extensions` dosya adı olmayan bir kurucu, önizleme veya
kayıt bilgisi türü.

## Eidos'un içine neden hiçbir şey yüklenmiyor

Mod Organizer 2 eklentileri paylaşılan kitaplık olarak yükler, Python olanları da
Qt üzerinden barındırır. İkisi de buraya taşınmıyor. Rust'ın kararlı bir ABI'si
yok; dolayısıyla başka bir derleyiciyle, ya da başka bir eniyileme bayrağıyla,
ortak bir bağımlılığın başka bir özellik kümesiyle derlenmiş paylaşılan bir
kitaplık tanımsız davranıştır ve hiçbir şey bunu sürüm uyuşmazlığı olarak
bildirmez. Üstelik Eidos'un görsel bileşenleri derleme zamanında geneldir, yani
ABI kararlı olsaydı bile bir kitaplık geri verecek bir bileşen kuramazdı.

Eklenti, Eidos'un hesabınızın dosya sistemi erişim haklarıyla çalıştırdığı bir programdır. Yalnızca güvendiğiniz yardımcıları kurun. Süreç yalıtımı ve denetlenen yanıtlar, işletim sistemi düzeyinde bir korumalı alan sağlamaz.

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

View -> Extensions altında bir Run düğmesiyle görünür ve ayrık başlar; Eidos onu
beklemez, durum satırı pid'ini verir. Çıktısı atılır. `workdir` yoksa Eidos'un
kendisinin başlatıldığı klasörde başlar.

## Bir denetim

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Her tazelemede, `workdir` başka bir şey söylemedikçe örnek kökünde çalışır ve
satır başına bir bulgu yazar:

```
level<TAB>title<TAB>detail
```

burada `level` `problem`, `advice` ya da `ok` olur. Ayrıntı isteğe bağlıdır;
başlığı olmayan bir satır atılır. Bilinen bir düzeyle başlamayan her şey yok
sayılır; böylece ilerleme çıktısı ve başıboş uyarılar, Eidos'un kendi
denetimlerinden biri gibi görünen bir satır üretemez. Bulgular Diagnostics
sekmesinde `<name> - <title>` olarak, önek ve Nexus kartlarından sonra, diğer
Health kartlarından önce görünür.

Bir denetimin üç saniyesi vardır. Aşan durdurulur ve kendine karşı bir sorun
olarak bildirilir; her tıklamanın ardından gelen aynı tazelemede çalıştığı için,
takılan bir denetim pencereyi dondururdu. Saat, denetim çıkıp çıktı boruları
kapanana kadar işler; bu yüzden onları hâlâ tutan bir arka plan alt süreci üç
saniyeden sayılır. Hata ile çıkıp hiçbir şey yazmayan denetim, hata çıktısının
ilk satırıyla aynı biçimde bildirilir; iki akıştan birine 1 MiB'den fazla yazan
da öyle. Her iki durumda da Eidos, siz Reload'a basana kadar onu yeniden
çalıştırmaz ve satırında "It will not be retried until you press Reload."
yazar. Bir denetim standart girdisinden hiçbir şey okumaz.

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
| `{install}`     | örneğin çalıştırdığı oyun klasörü            |
| `{data}`        | o klasörün `Data` dizini                     |

`{install}` mağaza kurulumudur; örnek bir oyun kopyası çalıştırıyorsa (donmuş bir
`Stock Game` ya da bir listenin kendi oyun klasörü, bkz.
[usage.md](usage.md#oyun-dosyaları-mağaza-kurulumu-veya-bir-kopya)) o kopyadır.
`{data}` onu izler.

Bir denetim ayrıca, bildirimini tutan klasör olan `{addon_dir}` değerini alır;
aşağıdaki protokol 1 eklentileri de öyle. Bir araç almaz.

Bilinmeyen bir yer tutucu boşaltılmak yerine tam yazıldığı gibi bırakılır; böylece
bir hata görünür biçimde başarısız olur, `--out {typo}` ifadesini
`--out --next-flag` haline getirmez. Yer tutucularının tümü çözülemeyen bir aracı
ya da denetimi çalıştırmak reddedilir ve Eidos hangilerinin eksik olduğunu söyler:
araçta durum satırında, denetimde Diagnostics sekmesindeki satırında.

## Bir eklentinin yapamayacakları

Bir eklenti Eidos'a geri çağrı yapamaz, kendisini bir arşivden kaydedemez veya kendi arayüz bileşenlerini çizemez. Yapılandırılmış protokol 1 kurucuları denetlenen dosya planları ve seçim soruları döndürebilir; inceleme, geçici hazırlık ve yayımlama Eidos'un kontrolündedir. FOMOD ve BAIN yerel kurucular olarak kalır. Önizleme ve kayıt bilgisi eklentileri aynı sınırlı yanıt sunucusunu kullanır. Eşleştirme önceliği, kaydedilmiş CLI yanıtları, sınırlar ve yeniden yürütmede kimlik denetimleri için [protokol sözleşmesine ve çalıştırılabilir örneklere](../../../../guide/examples/extensions/README.md) bakın.
