<!-- eidos-i18n: source=docs/guide/install.md sha=bc05db1de00bb7c80224f065c3f730f3adc542dc -->

# Instalace Eidosu

Čtyři cesty dovnitř. Colony nainstaluje jediný soubor, `eidos-linux`, který je
celým programem; ostatní dají vedle sebe `eidos` (příkazovou řádku), `eidos-gui`
a pomocný program pro statický náhled NIF, plus obsluhu `nxm://`, díky které
tlačítko „Mod Manager Download“ na Nexusu přistane ve vaší instanci.

## Co potřebujete nejdřív

| | |
|---|---|
| **Linux s FUSE** | `fusermount3` v PATH. Dodává jej každá současná distribuce. |
| **Hra pod Protonem, jednou spuštěná** | Steam vytvoří Wine prefix hry až při prvním spuštění a Eidos pracuje uvnitř něj. |
| **`7z`** | Pro instalaci archivů s módy. Ve většině distribucí `p7zip`. |
| **`par2`** (volitelné) | Data pro obnovu PAR2 pro zálohy z `eidos pack`. `par2cmdline` na Archu a Fedoře, `par2` na Debianu a Ubuntu. |

Žádný root, žádný démon, žádná úprava `/etc/fuse.conf` a nic, co byste museli
přidávat do svých skupin. Eidos připojuje uvnitř soukromého jmenného prostoru,
který patří procesu hry.

## Přes Colony

Vyhledejte **Eidos** v [Colony](https://github.com/Project-Colony/Colony) a
nainstalujte jej; aktualizace přicházejí přes spouštěč. Colony drží jediný
soubor, `~/.local/share/Colony/apps/Eidos/eidos-linux`. Spusťte jej a otevře se
okno. Zavolaný přes odkaz pojmenovaný `eidos` je tentýž soubor příkazovou
řádkou. Dejte si jeden takový do `PATH` a zaregistrujte obsluhu stahování
z Nexusu:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

Okno žádný odkaz nepotřebuje: kdykoli spouští hru nebo nástroj, spustí svůj
vlastní soubor jako CLI. Náhled modelů NIF potřebuje `eidos-nif-preview`, který
obsahuje jen balík vydání a sestavení ze zdrojů; bez něj to náhled řekne a nic
jiného se nemění. Steam nasměrujte přímo na ten soubor, jak je popsáno níže.

## Balík vydání

Stáhněte `eidos-linux-bundle.tar.gz` a k němu `.sig`, `.meta` a `.meta.sig`
z [posledního vydání](https://github.com/Project-Colony/Eidos/releases/latest)
a ověřte je tak, jak ukazují
[zásady podepisování kódu](../../README.md#zásady-podepisování-kódu) v README.
Potom:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Ve výchozím stavu instaluje do `~/.local/bin`. `--system` jej dá do
`/usr/local/bin`, `--bindir DIR` kamkoli jinam. Opětovné spuštění je zamýšlený
způsob aktualizace. Balík i `eidos-linux` potřebují Linux x86_64 s glibc 2.39
nebo novějším.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD sestaví aktuální `main` ze zdrojových kódů.

## Ze zdrojových kódů

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Sestavení ze zdrojů vyžaduje Rust 1.89 nebo novější, `just`, CMake 3.20+, překladač C++17 a Python 3 pro testy nativního pomocného programu. Příkazy bez `just` najdete v [pokynech k nativnímu pomocnému programu](../../../../../crates/eidos-nif-preview/README.md). Rust sestavte pomocí `cargo build --release --locked`, pomocný program zkopírujte do `target/release` a poté spusťte `packaging/install.sh --from target/release`.

## Potom: nasměrovat na něj Steam

Eidos běží *jako* spouštěcí příkaz vaší hry - právě tak stihne připojit pohled
dřív, než se hra rozběhne. Ve Steamu pravým tlačítkem na hru -> Vlastnosti ->
Parametry spuštění:

```
~/.local/bin/eidos-gui %command%
```

Z Colony je cesta `~/.local/share/Colony/apps/Eidos/eidos-linux`. Stiskněte
Hrát. Eidos se otevře na instanci té hry; instalujte módy, seřaďte
LOOTem, klikněte na Run. Po ukončení připojení zmizí s hrou a vaše instalace je
přesně taková, jaká byla.

Použijte absolutní cestu - Steam nečte `PATH` vašeho shellu.

### Pokud dáváte přednost terminálu

```sh
eidos init skyrimse               # vytvořit instanci (zadejte složku a bude přenosná)
eidos install skyrimse mod.7z     # módy Simple / FOMOD / BAIN / root
eidos sort skyrimse               # seřadit pořadí načítání LOOTem
eidos play skyrimse -- %command%  # spustit cokoli skrz sloučený pohled
```

Každý příkaz, který bere identifikátor hry, bere i složku přenosné instance -
viz [usage.cs.md](usage.md). Kompletní prohlídka je tamtéž.

## Volitelně: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` uděluje volitelné oprávnění, ale nezapíná passthrough.
Patří na samostatný binární soubor `eidos` z balíku nebo ze sestavení ze zdrojů,
nikdy na `eidos-linux`, který je zároveň oknem ([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` je samostatný přepínač při spuštění. Je **ve výchozím stavu vypnutý a téměř jistě to tak chcete
nechat**: měřeno na Skyrim SE brání hře otevřít vlastní archivy a pluginy, takže
se módy tiše nenačtou. Přepínač existuje proto, aby šlo mechanismus znovu
otestovat, ne proto, že by byl doporučen.

Podrobnosti a měření za tímto rozhodnutím v
[troubleshooting.cs.md](troubleshooting.md).

## Už je něco špatně?

[troubleshooting.cs.md](troubleshooting.md) pokrývá přepínače prostředí,
čtení čítačů operací a každý problém, který dosud někoho kousl.
