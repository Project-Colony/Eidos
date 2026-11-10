<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Instalace Eidosu

Čtyři cesty dovnitř. Colony nainstaluje jediný soubor, `eidos-linux`, který je
celým programem. Balík vydání, balíček pro Arch a sestavení ze zdrojů dají vedle
sebe `eidos` (příkazovou řádku), `eidos-gui` a statický pomocný program pro náhled
NIF `eidos-nif-preview`. Balíček pro Arch a sestavení ze zdrojů instalují také
`eidos-fuse` a `eidos-launch`, dvě nízkoúrovňová CLI. Každá cesta umí zaregistrovat
obsluhu `nxm://`, díky které tlačítko „Mod Manager Download" na Nexusu přistane ve
vaší instanci.

## Co potřebujete nejdřív

| Požadavek | Proč |
|---|---|
| Linux s FUSE | `fusermount3` v PATH. Dodává jej každá současná distribuce. |
| Hra pod Protonem, jednou spuštěná | Steam vytvoří Wine prefix hry až při prvním spuštění a Eidos pracuje uvnitř něj. Eidos také rozpozná kopie Skyrimu Special Edition z GOG a Epicu, kopie Falloutu 3 a Fallout: New Vegas z Epicu a kopii Stardew Valley z GOG. Ty běží přes Heroic nebo Legendary s Eidosem jako obalem hry: viz [usage.md](usage.md#spuštění-hry). |
| Binárka 7-Zip: `7z`, `7zz` nebo `7za` | Pro instalaci archivů s módy. `7zip` na Archu, `p7zip` ve většině ostatních distribucí. |
| `par2` (volitelné) | Data pro obnovu PAR2 pro zálohy z `eidos pack`. `par2cmdline` na Archu a Fedoře, `par2` na Debianu a Ubuntu. |

Žádný root, žádný démon, žádná úprava `/etc/fuse.conf` a nic, co byste museli
přidávat do svých skupin. Eidos připojuje uvnitř soukromého jmenného prostoru,
který patří procesu hry.

Některá nastavení systému se projeví jen pády. Záložka Diagnostics kontroluje
`vm.max_map_count` (1048576 nebo víc), pevný limit otevřených souborů (524288 nebo
víc) a to, zda hra, její prefix nebo instance leží na NTFS, exFAT nebo FAT;
nápravu každého případu popisuje
[troubleshooting.md](troubleshooting.md#systém).

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
obsahuje jen balík vydání, balíček pro Arch a sestavení ze zdrojů; bez něj to
náhled řekne a nic jiného se nemění. `eidos-linux` také nenese žádný z
runtimů Microsoftu, které balík dodává
([níže](#runtimy-microsoftu-v-balíku)): když nástroj nějaký potřebuje,
Eidos ho stáhne od Microsoftu, jakmile spustíte instalaci prerekvizit. Steam
nasměrujte přímo na ten soubor, jak je popsáno níže.

## Balík vydání

Stáhněte `eidos-linux-bundle.tar.gz` a k němu `.sig`, `.meta` a `.meta.sig`
z [posledního vydání](https://github.com/Project-Colony/Eidos/releases/latest)
a ověřte je tak, jak ukazují
[zásady podepisování kódu](../../README.md#zásady-podepisování-kódu) v README.
Balík obsahuje `eidos`, `eidos-gui` a `eidos-nif-preview` na nejvyšší úrovni, dále
`install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, ikony v `assets/brand/png`,
oznámení třetích stran v `licenses/` (nifly, omod, microsoft-directx,
microsoft-runtimes a crates Rustu) a složku `runtimes/` popsanou níže. Nenese
`eidos-fuse` ani `eidos-launch`. Potom:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` zkopíruje binárky do `~/.local/bin`, nebo do `$XDG_BIN_HOME`, je-li
nastaven. `--system` je dá do `/usr/local/bin` přes sudo, `--bindir DIR` kamkoli
jinam a `--from DIR` vezme binárky z jiného adresáře. Skript také:

- zaregistruje obsluhu `nxm://` spuštěním `eidos nxm --register`, takže krok
  registrace v části o Colony není potřeba;
- zapíše `~/.local/share/applications/eidos.desktop` a ikony Eidosu do
  `~/.local/share/icons/hicolor`;
- zkopíruje oznámení o licencích do `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` s `--system`);
- zkopíruje `runtimes/` do `~/.local/share/Colony/Eidos/runtimes`, i s `--system`,
  takže runtimy patří uživateli, který skript spouští;
- na konci vypíše parametr spuštění pro Steam s absolutní cestou k
  `eidos-gui`, který nainstaloval.

`--cap` požádá sudo o udělení `CAP_SYS_ADMIN` nainstalovanému `eidos` (viz FUSE
passthrough níže). `--no-cap` je výchozí a nechá instalaci bez rootu. Opětovné
spuštění skriptu je zamýšlený způsob aktualizace. Balík i `eidos-linux` potřebují
Linux x86_64 s glibc 2.39 nebo novějším.

### Runtimy Microsoftu v balíku

Složka `runtimes/` v balíku obsahuje soubory Microsoftu, které nástroje žádají:
`VC_redist.x86.exe` a `VC_redist.x64.exe` (Visual C++ 2015-2022, prerekvizita
`vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (.NET 8 Desktop Runtime,
pro `dotnet8` a `dotnetdesktop8`) a `dotnet-runtime-10.0.10-win-x64.zip` (runtime
.NET 10, pro `dotnet10`). Instalátory Visual C++ a .NET 8 spouští Eidos sám v
prefixu hry, bez winetricks, a zip .NET 10 rozbalí jednou do
`~/.local/share/Colony/Eidos/runtimes`, kde ho sdílí každá instance (78 MB
rozbaleno). Každý soubor se použije jen tehdy, když jeho SHA-256 odpovídá hodnotě
připnuté v Eidosu. .NET 10 SDK, který potřebuje Synthesis (`dotnetsdk10`, asi
300 MB), v balíku není a vždy se stahuje.

Rozbalený balík běží i na místě: Eidos hledá `runtimes/` vedle vlastní binárky,
až po výše uvedené složce s daty.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD sestaví aktuální `main` ze zdrojových kódů. Do `/usr/bin` nainstaluje
`eidos`, `eidos-gui`, `eidos-nif-preview`, `eidos-fuse` a `eidos-launch`, spolu se
systémovou obsluhou `nxm://`, položkou desktopu a ikonami. Závisí na `fuse3`,
`7zip` a `xdg-utils`. Složku `runtimes/` nedodává, takže se runtimy Microsoftu
stahují, když je nástroj potřebuje. `winetricks` je volitelný: Eidos ho stále
používá pro prerekvizity, které sám neinstaluje, například `dotnet48` a `xact`.

## Ze zdrojových kódů

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Sestavení ze zdrojů vyžaduje Rust 1.92 nebo novější, `just`, CMake 3.20+, překladač
C++17 a Python 3 pro testy nativního pomocného programu. `just build` sestaví celý
workspace, takže se sestaví i `eidos-fuse` a `eidos-launch`, a `just install` spustí
`packaging/install.sh` na výstupu sestavení, který je zkopíruje s ostatními.
Příkazy bez `just` najdete v
[pokynech k nativnímu pomocnému programu](../../../../../crates/eidos-nif-preview/README.md).
Rust sestavte pomocí `cargo build --release --locked`, pomocný program zkopírujte do
`target/release` a poté spusťte `packaging/install.sh --from target/release`.
Sestavení ze zdrojů také nemá složku `runtimes/`, takže se runtimy Microsoftu
stahují, když je nástroj potřebuje.

Profil release používá jednu codegen jednotku, fat LTO a odstraněné symboly. S
rustc 1.99.1 a 3 úlohami sestavení trvalo čisté sestavení `eidos` a `eidos-gui` 334 s,
oproti 243 s s thin LTO, a dalo `eidos` o velikosti 20,9 MB a `eidos-gui` o velikosti
49,7 MB. Každá binárka vkládá osm DLL DirectX uložených komprimovaně gzipem: 7,9 MB
místo 17,9 MB. `just profile=dev build` sestaví bez optimalizace.

## Potom: nasměrovat na něj Steam

Eidos běží *jako* spouštěcí příkaz vaší hry - právě tak stihne připojit pohled
dřív, než se hra rozběhne. Ve Steamu pravým tlačítkem na hru -> Vlastnosti ->
Parametry spuštění a zadejte absolutní cestu k oknu Eidosu následovanou
`%command%`:

```
~/.local/bin/eidos-gui %command%
```

Použijte absolutní cestu - Steam nečte `PATH` vašeho shellu. Cesta závisí na tom,
jak jste Eidos nainstalovali, a `install.sh` vypíše řádek pro adresář, který
použil:

| Instalace | Parametr spuštění |
|---|---|
| Balík vydání nebo `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Balíček pro Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Stiskněte Hrát. Eidos se otevře na instanci té hry; instalujte módy, seřaďte
LOOTem, klikněte na Run. Po ukončení připojení zmizí s hrou a vaše instalace je
přesně taková, jaká byla.

Hra se vždy spouští přes svůj obchod. V okně, které jste otevřeli sami, Run
spustí `steam -applaunch <appid>`, Steam spustí Eidos přes parametr spuštění a
otevře se druhé okno Eidosu, ve kterém stisknete Run. Dokud parametr spuštění
Eidos nespustí, Run nespustí nic a stavový řádek ukáže řádek, který nastavit.
Se zapnutým Start the game at once when a store opens Eidos (Preferences,
General, Running a game; ve výchozím stavu vypnuto) Hrát ve Steamu spustí hru bez
okna, pokud Health neukazuje žádný problém. Heroic a Legendary fungují stejně přes
své nastavení obalu; podrobnosti jsou v
[usage.md](usage.md#spuštění-hry).

Záložka Diagnostics řádek kontroluje. Karta Health ukáže řádek, který nastavit,
když je parametr spuštění prázdný, nejmenuje žádnou binárku Eidosu, ukazuje na
neexistující soubor, nemá za Eidosem `%command%`, nebo spouští jinou kopii Eidosu
než tu, která běží.

### Pokud dáváte přednost terminálu

```sh
eidos init skyrimse               # vytvořit instanci (zadejte složku a bude přenosná)
eidos install skyrimse mod.7z     # módy Simple / FOMOD / BAIN / root
eidos sort skyrimse               # seřadit pořadí načítání LOOTem
eidos check skyrimse              # kontroly Health, které nepotřebují okno
eidos play skyrimse -- %command%  # spustit cokoli skrz sloučený pohled
eidos shell skyrimse              # shell ve sloučeném pohledu, zápisy skončí v Overwrite
```

Každý příkaz, který bere identifikátor hry, bere i složku přenosné instance -
viz [usage.md](usage.md#instance-globální-a-přenosné). Kompletní prohlídka je v
[usage.md](usage.md).

## Aktualizace

Colony aktualizuje `eidos-linux` přes svůj spouštěč. U balíku spusťte
`install.sh` nového balíku; u balíčku pro Arch znovu `makepkg -si`; u sestavení ze
zdrojů stáhněte změny a spusťte `just install`. Okno se při startu ptá GitHubu na
nejnovější vydání, nejvýš jednou denně, a když je venku novější, ukáže odkaz na
jeho poznámky k vydání za Help v řádku nabídek. Nikdy nic nestahuje ani se
nenahrazuje. Preferences, About, Version kontrolu vypne, režim offline ji přeskočí
a kopie nainstalovaná pod `/usr` (balíček pro Arch) se neptá nikdy, protože ji
aktualizuje správce balíčků.

## Volitelně: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` uděluje volitelné
oprávnění, ale nezapíná passthrough. `./install.sh --cap` a `just setcap` ho udělí
`eidos`, který nainstalují nebo sestaví. Na přípojném bodu `nosuid` jádro
oprávnění souborů ignoruje, takže `install.sh` udělení přeskočí s varováním a
`just setcap` skončí chybou. Oprávnění patří na samostatnou binárku `eidos` z
balíku, balíčku pro Arch nebo sestavení ze zdrojů, nikdy na `eidos-linux`, který je
zároveň oknem ([SECURITY.md](../../../../../SECURITY.md)). Znovu sestavená nebo
přeinstalovaná binárka je nový soubor bez oprávnění, takže ho po každé aktualizaci
udělte znovu, pokud ho používáte.

`EIDOS_FUSE_PASSTHROUGH=1` je samostatný přepínač při spuštění a ve výchozím stavu
je vypnutý. Se zapnutým passthroughem dřívější verze démona bránila Skyrimu SE
načíst jakýkoli obsah módů. Nejpravděpodobnější příčinou je, že při každém otevření
registrovala nový podkladový soubor, zatímco jádro přijímá jediný na inode. Démon
teď jeden sdílí mezi všemi otevřeními souboru přes passthrough, ale ve hře to zatím
nebylo znovu změřeno. Do té doby passthrough zapínejte jen kvůli testování.
Měření a příčina jsou v [troubleshooting.md](troubleshooting.md).

## Už je něco špatně?

[troubleshooting.md](troubleshooting.md) pokrývá přepínače prostředí, čtení čítačů
operací, karty Health a každý problém, který dosud někoho kousl.
