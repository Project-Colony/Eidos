<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Instalace Eidosu

Tři cesty dovnitř. Všechny dají tytéž dva spustitelné soubory - `eidos`
(příkazová řádka) a `eidos-gui` - plus obsluhu `nxm://`, díky které tlačítko
„Mod Manager Download" na Nexusu přistane ve vaší instanci.

## Co potřebujete nejdřív

| | |
|---|---|
| **Linux s FUSE** | `fusermount3` v PATH. Dodává jej každá současná distribuce. |
| **Hra pod Protonem, jednou spuštěná** | Steam vytvoří Wine prefix hry až při prvním spuštění a Eidos pracuje uvnitř něj. |
| **`7z`** | Pro instalaci archivů s módy. Ve většině distribucí `p7zip`. |

Žádný root, žádný démon, žádná úprava `/etc/fuse.conf` a nic, co byste museli
přidávat do svých skupin. Eidos připojuje uvnitř soukromého jmenného prostoru,
který patří procesu hry.

## Arch

```bash
cd packaging && makepkg -si
```

## Archiv vydání

```bash
./install.sh
```

Ve výchozím stavu instaluje do `~/.local/bin`. `--system` jej dá do
`/usr/local/bin`, `--bindir DIR` kamkoli jinam. Opětovné spuštění je zamýšlený
způsob aktualizace.

## Ze zdrojových kódů

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Potom: nasměrovat na něj Steam

Eidos běží *jako* spouštěcí příkaz vaší hry - právě tak stihne připojit pohled
dřív, než se hra rozběhne. Ve Steamu pravým tlačítkem na hru -> Vlastnosti ->
Parametry spuštění:

```
~/.local/bin/eidos-gui %command%
```

Stiskněte Hrát. Eidos se otevře na instanci té hry; instalujte módy, seřaďte
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
viz [usage.cs.md](usage.cs.md). Kompletní prohlídka je tamtéž.

## Volitelně: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` zapne jaderný FUSE
passthrough. Je **ve výchozím stavu vypnutý a téměř jistě to tak chcete
nechat**: měřeno na Skyrim SE brání hře otevřít vlastní archivy a pluginy, takže
se módy tiše nenačtou. Přepínač existuje proto, aby šlo mechanismus znovu
otestovat, ne proto, že by byl doporučen.

Podrobnosti a měření za tímto rozhodnutím v
[troubleshooting.cs.md](troubleshooting.cs.md).

## Už je něco špatně?

[troubleshooting.cs.md](troubleshooting.cs.md) pokrývá přepínače prostředí,
čtení čítačů operací a každý problém, který dosud někoho kousl.
