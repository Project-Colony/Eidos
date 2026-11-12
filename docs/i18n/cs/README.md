<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Nativní správce módů pro Linux, který se nikdy nedotkne vaší hry.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalace)

Eidos dává hrám od Bethesdy na Linuxu to, co jim na Windows dává Mod Organizer 2:
sloučený pohled na vaše módy, vytvořený při každém spuštění. Místo hookování
Windows API používá linuxové jmenné prostory připojení a FUSE a správce běží bez
Wine. Nic se nekopíruje do složky hry, takže po sobě není co uklízet. Eidos má
také definice pro Stellar Blade, Stardew Valley, 7 Days to Die a Crimson Desert.

Kopie ze Steamu spouštějí Eidos z parametru spuštění hry. Kopie z GOG a Epicu v
Heroicu nebo Legendary ho spouštějí jako obal. Hra se vždy spouští přes obchod,
který kopii vlastní; Eidos ji nikdy nespouští přes Proton sám.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Stav:** Skyrim SE se přes Eidos hraje denně, se SKSE, preloadery script
> extenderu, Creation Clubem, pořadím načítání řazeným LOOTem a uloženými
> pozicemi po profilech. Zatím je skutečným hraním ověřená jedna rodina her.
> Dalších deset her od Bethesdy a čtyři další jsou zapojeny a čekají na testery.

## Proč Eidos

Sloučený pohled je připojen v soukromém jmenném prostoru připojení, který vidí jen
strom procesů hry. Ostatní programy, i druhá hra, ho nikdy nevidí a nepotřebují pro
něj žádné oprávnění. Když hra skončí nebo je zabita, nebo stroji vypadne napájení,
jmenný prostor zmizí spolu se svými procesy a instalace je taková, jaká byla.

Každý profil vlastní svůj seznam módů, pořadí pluginů, INI soubory a uložené pozice.
Při spuštění se soubory pluginů a složka uložených pozic připojí vazbou přes vlastní
cesty hry, takže to, co tam hra zapíše, skončí v profilu. Přepnutí profilu přepne
všechno.

Eidos nepotřebuje žádného pomocníka setuid, žádný systémový démon, žádné
`sudo setcap` a žádnou úpravu `/etc/fuse.conf`. Démon FUSE běží jako váš uživatel po
dobu jednoho spuštění a skončí s hrou. Instalujete jeden program a nastavíte jeden
parametr spuštění ve Steamu.

Před každým během Eidos uchová snímek seznamu pluginů. Když pád seznam poškodí,
Diagnostics ho se snímkem porovná a Restore the pre-session order vrátí staré pořadí.
Zachycení, které by smazalo vaše pořadí načítání, se odmítne, s uvedením důvodu.

Test run spustí hru s módy profilu, bez módů, nebo jen s vybranými módy, které jsou zapnuté, a jejich
mastery. Uložené pozice, změny INI, stav pluginů, výstup do Overwrite a další soubory
My Games z toho běhu, včetně logů, jdou do `<instance>/test-run/`. Když hra skončí,
Eidos vypíše těchto pět skupin a každou ponecháte nebo zahodíte. Ostatní zápisy,
například registr Wine, jdou tam, kam vždy. Find the mod behind a problem v Diagnostics
spouští testovací běhy vždy s polovinou podezřelých módů plus mastery, které potřebují.
Po každém běhu odpovíte, zda se problém objevil, a Eidos zužuje podezřelé, dokud
nepojmenuje mód, nebo módy, které potřebují mastery jedna druhé; 200 podezřelých
zabere asi 8 běhů. Zapnuté módy vybrané v seznamu před začátkem zůstávají zapnuté v každém
kroku. Podrobnosti v [Testovací běhy](docs/guide/usage.md#testovací-běhy) a
[Hledání módu za problémem](docs/guide/usage.md#hledání-módu-za-problémem).

### Jak si stojí ve srovnání

| | Eidos | MO2 přes Wine | Fluorine-Manager | Limo / nasazení přes odkazy |
|---|---|---|---|---|
| Správce běží nativně | ano | ne, aplikace Windows ve Wine | ano (port do Qt) | ano |
| Složka hry nedotčená | ano, vždy | ano | ano | ne, zapisují se do ní odkazy |
| Připojení viditelné pro | jen hru | jen hru | celý systém | neuplatňuje se |
| Nutný úklid po pádu | žádný | žádný | obnova po mrtvém připojení | ruční zrušení nasazení |
| Root módy (ENB, preloadery) | ano, nativně | vyžaduje plugin | vyžaduje plugin | částečně |
| Vyžadovaná oprávnění | žádná | žádná | úprava `/etc/fuse.conf` | žádná |

### Jak je rychlý

| | dříve | nyní |
|---|---|---|
| načtení uložené pozice | ~20 sekund | 6-7 sekund |
| čtení adresářů v jednom sezení | 5,6 milionu | 335 tisíc |

Přechody mezi buňkami jsou okamžité. Hledání jednoho souboru se dřív ptalo každého z
padesáti módů po řadě a výpis jedné složky to dělal padesátkrát. Vrstvy jen pro čtení
se teď indexují jednou při připojení, takže nalezení souboru je jedno hledání v hash
tabulce a sloučený výpis složky se také počítá jednou při připojení. Čísla pocházejí ze
skutečné instance hrané normálně, ne z benchmarku:
[internals/performance.md](../../internals/performance.md).

### Předchozí práce a poděkování

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) a
  [usvfs](https://github.com/ModOrganizer2/usvfs): sémantika, kterou Eidos
  reprodukuje, a kódová základna, proti které se studovala jeho parita
- [LOOT](https://loot.github.io/): řadicí engine, přes libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) a ostatní linuxoví správci, které ukazují,
  že existuje komunita, která chce tohle vyřešit

## Co umí

### Módy

Eidos instaluje jednoduché archivy, průvodce FOMOD, balíčky BAIN z Wrye Bash,
kontejnery OMOD s podporovanými skripty OBMM a vše ostatní přes ruční výběr. Root
módy (preloadery script extenderu, ENB, Engine Fixes) fungují nativně, bez pluginu
Root Builder a bez čehokoli kopírovaného do vaší instalace. Můžete skrývat jednotlivé
soubory, seskupovat módy oddělovači, přesouvat výběr na cíl, držet poznámky a
kategorie u jednotlivých módů a importovat profil z MO2.

Seznam funguje jako ten z MO2: osm volitelných sloupců a řazení podle kteréhokoli z
nich, seskupení podle kategorie nebo podle zdroje, gesta dvojklikem, skok psaním,
zálohy jednotlivých módů, které nic nedělají, dokud je neobnovíte, a příznaky u módu,
jehož rozvržení tato hra nenačte nebo který byl stažen pro jinou hru. Filtry vybírají
módy podle stavu na Nexusu (schválený, sledovaný, dostupná aktualizace), poznámek,
skrytých souborů a druhu obsahu (pluginy, archivy, textury, meshe, skripty, zvuk a
další). Strom souborů vytváří složky, přejmenovává, maže a otevírá soubory a zobrazuje
náhled obrázků a textu, aniž by cokoli spouštěl.

Ctrl+Z a Ctrl+Shift+Z vracejí a opakují změny seznamu módů a pořadí načítání. Vypnutí
nebo odebrání módu, který používá jedna ze tří nejnovějších pozic profilu, se nejdřív
zeptá a jmenuje pluginy (pozice Skyrimu a Falloutu 4). Přeinstalace nebo aktualizace
módu FOMOD otevře průvodce s odpověďmi z jeho poslední instalace. Informace o módu
vypisují módy, které vyžaduje, a módy, které vyžadují jeho, čtené z masterů zapnutých
pluginů.

### Pluginy

Pořadí načítání má vestavěné řazení LOOTem, indexy módů tak, jak je počítá hra,
varování na chybějící mastery a váš obsah z DLC a Creation Clubu zobrazený jako
nespravované řádky. Záhlaví počítá použité sloty v každém indexovém prostoru (full,
medium, light). Najetí na plugin ukáže jeho autora, popis a verzi hlavičky. U enginů
Skyrimu je označen plugin, jehož INI stejného názvu hra načítá.

Pluginy, které LOOT označí jako špinavé, čistí QuickAutoClean z xEditu do módu
`Cleaned plugins`, nikdy na místě. Flag as light, do nového módu, zapíše kopii pluginu
s příznakem light a odmítne, když jedna z nejnovějších pozic načítá ten plugin jako
plný (pozice Skyrimu a Falloutu 4).

### Instance

Instance je globální, držená pod `~/.local/share/Colony/Eidos/instances/`, nebo
přenosná: samostatná složka kdekoli chcete (druhý disk, herní oddíl), přesunutelná a
izolovaná jako v MO2. Přenosné instance si Eidos pamatuje mezi sezeními. GUI, spuštění
ze Steamu i každý příkaz z příkazové řádky se drží té, kterou jste použili naposledy, a
každý příkaz bere složku všude tam, kde bere identifikátor hry. Podrobnosti v
[usage.md](docs/guide/usage.md#instance-globální-a-přenosné).

Instance může také spouštět hru z kopie složky hry uvnitř instance. Když Eidos poprvé
otevře seznam Wabbajack, spustí hru z vlastní kopie seznamu: složky, kterou jmenuje jeho
`ModOrganizer.ini`, jinak složky `Stock Game` nebo `Game Root`. Freeze this version na
kartě Game files v Diagnostics zkopíruje instalaci z obchodu do `<instance>/Stock Game`,
takže aktualizace obchodu už nemění soubory, ze kterých hra běží. Když obě složky leží na
jednom souborovém systému, který umí sdílet bloky, například btrfs nebo XFS, kopie je s
instalací z obchodu sdílí (reflink). Na ext4, nebo přes dva souborové systémy, je to plná
kopie a velikost a volné místo se ukážou předem. Hru stále spouští obchod. Return to the
store version a Delete the copy to vrátí a
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
dělá totéž v terminálu. Morrowind a Oblivion s `bUseMyGamesDirectory=0` z kopie běžet
nemohou. Podrobnosti v
[usage.md](docs/guide/usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie).

### Profily

Každý profil má vlastní pořadí módů, stav pluginů, INI soubory a uložené pozice. Uložené
pozice se rozeberou a porovnají s vašimi současnými pluginy, s tlačítkem, které zapne,
co daná pozice potřebuje, a po každém sezení se zkopírují zpět do prefixu pro Steam
Cloud. Pozice, které Steam Cloud přinesl z jiného zařízení nebo které po sobě nechalo
spuštění bez Eidosu v prefixu, vypisuje záložka Saves. Import je zkopíruje do profilu a
pozice, jejíž název už profil používá, přijde pod novým názvem `orphan-` místo toho, aby
ji nahradila.

Profil může nést proměnné spuštění a obaly (`env=NAME=value` a `wrapper=command`, po
jednom na řádek, v editoru INI) pro běhy své hry a nástrojů. U Skyrimu SE, Enderalu SE a
Falloutu 4 karta Display v Diagnostics nastaví velikost a režim okna v kopiích INI
profilu. Backups drží body obnovy INI profilu a Eidos jeden pořídí před každým během
BethINI Pie. Zástupce profilu na ploše spustí hru přes její obchod. `eidos sessions`
vypíše zaznamenané běhy a porovná jeden se současným stavem.

### Nexus

Připojte účet pro přijímání odkazů „Mod Manager Download", kontrolu aktualizací a
zobrazení autorů a jejich profilů. Každý mód se kontroluje, schvaluje a odkazuje na
doméně Nexusu, ze které pochází jeho soubor, takže mód pro Skyrim SE v instanci Enderalu
SE zůstane na webu Skyrimu SE. Aktualizace sleduje řetězec nástupců, který autor nastavil
na Nexusu, nahradí složku módu novým souborem (záloha starého se nabídne), ponechá jeho
prioritu a stav zapnutí a přehraje odpovědi FOMOD uložené při instalaci.
`eidos nexus update <instance> --apply` dělá totéž v terminálu na prémiovém účtu a na
bezplatném vypíše stránky souborů. Track a Endorse jsou v nabídce módu a každá kontrola
aktualizací je synchronizuje s vaším účtem na Nexusu. Informace o módu mají záložku Nexus
page s popisem a changelogem jako prostým textem a jeho záložka General končí tím, co
poslední aktualizace změnila: soubory přidané, odebrané a se změněnou velikostí a mastery
přidané nebo odebrané. Install downloads when they finish (Preferences, Nexus, ve výchozím
stavu vypnuto) nainstaluje stažení, jakmile skončí, zatímco je okno otevřené.

Kolekce se instalují podle svých receptů, s obnovitelným průběhem a výslovnými zprávami
o nepodporovaných krocích. Porovnání ověřuje přesné zdrojové soubory; neúplná starší
metadata zůstávají neověřená. Panel kolekcí prochází a prohledává kolekce publikované pro
hru, ukazuje, kdy má nainstalovaná kolekce novější revizi, a bezplatný účet provádí
chybějícími soubory po jedné stránce. Když přetažení poruší pravidla pořadí kolekce,
Diagnostics to řekne a Restore collection order vrátí její módy zpět. Záložka Downloads
filtruje, řadí, skrývá a odstraňuje archivy. Režim Offline (Preferences, Nexus) zastaví
přístup k Nexusu a denní kontrolu vydání.

### Nástroje

xEdit, BodySlide, DynDOLOD a další modderské nástroje běží skrz sloučený pohled uvnitř
prefixu Wine hry (prefix Proton ze Steamu, nebo ten, který nastavil Heroic). Vidí vaše
módy, jejich výstup přistane v Overwrite a jedno kliknutí z něj udělá skutečný mód. U
Skyrimu SE Eidos najde SSEEdit, SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS,
BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis a BethINI Pie ve složce hry, uvnitř módu
nebo ve složce s nástroji, kterou držíte vedle svých her, každý s runtimy, které potřebuje.
Ostatní hry mají vlastní seznam. Pandora, Nemesis, FNIS, BodySlide a Synthesis zapisují
výstup do pojmenovaného módu (`Nemesis Output` a podobně).

Balík vydání nese instalátory Visual C++ 2015-2022 a .NET 8 Desktop Runtime od Microsoftu
a runtime .NET 10. Eidos instaluje `vcrun2022` a .NET 8 těmito instalátory, bez
winetricks, a bez balíku stáhne tytéž soubory od Microsoftu, každý ověřený proti SHA-256
zabudovanému v Eidosu. Synthesis dostane SDK .NET 10, stahování 300 MB, které balík
nenese. Ostatní prerekvizity (.NET Framework 4.8, XACT a starší verze .NET) stále jdou
přes winetricks. Nic se nestahuje, dokud nepožádáte o instalaci prerekvizit nástroje.

Připněte nástroje, které používáte, skryjte ty, které ne, dejte nástroji vlastní Steam
AppID, když je sám o sobě aplikací na Steamu, a zapište zástupce `.desktop`, který ho
spustí skrz sloučený pohled, aniž by se Eidos otevřel. `eidos tool <game> import-mo2`
vypíše spustitelné soubory z `ModOrganizer.ini` ve složce seznamu a `--write` je přidá.
Exe v záložce Data lze spustit jednou nebo přidat jako nástroj a
`eidos tool <instance> run --exe <path>` spustí jedno z terminálu. S Ask before playing se
Run nejdřív zeptá, když je výstup generátoru zastaralý.

Stop ukončí běžící hru nebo nástroj. Nabídka File spouští winecfg, regedit, průzkumník
souborů Wine a odinstalátor Windows v prefixu hry. Rebuild prefix v Diagnostics přejmenuje
prefix, aby Steam vytvořil nový, a nic nemaže. `eidos shell <instance>` otevře shell ve
sloučeném pohledu, nebo spustí příkaz zadaný za `--`, a co zapíše, skončí v Overwrite.

Když je okno na pozadí, Eidos může poslat oznámení na plochu, když skončí běh nástroje,
instalace kolekce, pack nebo unpack nebo poslední běžící stahování, a když selže řazení
LOOTem. Notify the desktop when a long job ends v Preferences to zapne; ve výchozím stavu
je vypnuto.

### Diagnostika

Záložka Diagnostics vypisuje chybějící mastery, osiřelé archivy, rozejití seznamu módů a
poškozené sady pluginů a po běhu to, co podle vlastního logu script extenderu se načetlo.
Hlásí také:

- script extender, který chybí nebo neodpovídá verzi hry
- hru, která se změnila od posledního čistého spuštění
- parametr spuštění Steamu, který nespouští Eidos nebo za ním nemá `%command%`
- Proton starší než ten, který naposledy připravil prefix, nebo Proton ze Steamu z
  Flatpaku
- chybějící `PROTON_ENABLE_NVAPI` pro DLSS a exkluzivní celou obrazovku, která zastaví
  generování snímků v Community Shaders
- `enblocal.ini` bez `LinuxVersion=true`
- log pádu z poslední relace a módy, které jmenuje
- požadavky z Nexusu, které chybí nebo jsou vypnuté
- obsluhu `nxm://`, kterou je jiný program nebo chybějící soubor
- řádky `sTestFile`, kvůli kterým hra ignoruje `plugins.txt`
- zapnuté pluginy `.esp` ve Starfieldu a neplatné nebo nepárové pluginy blueprint
- patch, jehož volné soubory prohrávají s módem, který záplatuje
- mód, jehož kopie skriptů Papyrus script extenderu vyhrávají nad vlastními skripty
  extenderu
- nízký `vm.max_map_count` nebo limit otevřených souborů
- soubory hry na souborovém systému Windows
- datové cesty BodySlidu, které nejsou složkou Data hry, a složky v `Overwrite/Root`,
  které patří do Data
- `ModOrganizer.ini` seznamu, který míří nástroje na jinou složku hry
- kopii hry, která chybí nebo nemá spustitelný soubor hry

`eidos check <instance> [--json]` spustí kontroly, které nepotřebují okno, a při nalezení
problému skončí kódem 1. Copy report zkopíruje zprávu s vaší domovskou složkou zapsanou
jako `~` a uživatelským jménem jako `<user>` a Save tree v záložce Data zapíše sloučený
strom Data se zdrojem každého souboru, zakrytý stejným způsobem.

[Kontroly integrity a vygenerované soubory](../../../docs/internals/integrity.md) (anglicky)
popisují ochranu při nahrazování, konflikty souborů v BSA/BA2, statickou diagnostiku SKSE
a záznamy generování včetně jejich omezení.

### Kde si drží vlastní soubory

Eidos používá rozvržení, které používá každý program z rodiny Colony:
`~/.config/Colony/Eidos/` pro to, co jste zvolili (předvolby, vaše relace na Nexusu, váš
seznam instancí, definice her a doplňků, které jste napsali),
`~/.local/share/Colony/Eidos/` pro globální instance, runtimy nástrojů (stažené nebo
zkopírované z balíku přes `install.sh`) a logy relací a `~/.cache/Colony/Eidos/` pro
masterlisty LOOTu. Uvnitř instance drží `Stock Game` zmrazenou kopii hry a `test-run/`
drží testovací běh čekající na kontrolu.

Starší Eidos držel své soubory v `~/.config/eidos/`, `~/.local/share/eidos/` a
`~/.local/state/`. První spuštění po aktualizaci je přenese, řekne to v logu a staré
adresáře nechá, jak byly. Dokud přesun není dokončen, Eidos dál používá staré soubory.
Podrobnosti v [internals/paths.md](../../internals/paths.md) (anglicky).

### Podporované hry

Skyrim SE/AE je ověřený skutečným hraním. Fallout 4 je zapojený od začátku do konce také
(F4SE se podsune automaticky, invalidace archivů, pořadí načítání s hvězdičkami, LOOT,
uložené pozice `.fos`); viz [guide/fallout4.md](docs/guide/fallout4.md). Zapojené a
hledající testery: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV,
Starfield, Oblivion a Morrowind. Fallout 3, Fallout NV, Oblivion a Morrowind řadí pluginy
podle dat souborů: Eidos drží jedno pořadí na profil a data ukazuje uvnitř připojení,
soubory na disku nemění.

Čtyři další hry jsou zapojené a také hledají testery: Stellar Blade (módy Unreal `.pak` v
`~mods`), Stardew Valley (módy SMAPI, se SMAPI nainstalovaným zvlášť), 7 Days to Die
(složky `Mods/` vedle hry) a Crimson Desert. Crimson Desert bere jen módy s kódem: pluginy
ASI, ReShade a loader DLL v `bin64`. Jeho datové módy, které žijí ve skupinách PAZ, se
nepodporují.

Hra je jeden soubor TOML a ten váš může jít do `~/.config/Colony/Eidos/games/` bez
nového sestavení: [internals/adding-games.md](../../internals/adding-games.md).

## Instalace

### Přes Colony (doporučeno)

Vyhledejte **Eidos** v [Colony](https://github.com/Project-Colony/Colony) a nainstalujte jej.
Aktualizace přicházejí přes spouštěč. Colony nainstaluje jediný soubor, `eidos-linux`:
okno, a zároveň příkazovou řádku `eidos`, když je zavolán přes odkaz pojmenovaný `eidos`.
Ten soubor nenese ani pomocný program pro náhled modelů NIF, ani runtimy Microsoftu z
balíku níže. Bez pomocného programu to náhled řekne a všechno ostatní funguje a Eidos
stáhne runtime od Microsoftu, když instalujete prerekvizity nástroje. Co dělat po
instalaci a jak na něj nasměrovat Steam:
[docs/guide/install.md](docs/guide/install.md#přes-colony).

### Přímé stažení binárních souborů

Stáhněte si soubor z [posledního vydání](https://github.com/Project-Colony/Eidos/releases/latest):

| Soubor | Co to je |
|---|---|
| `eidos-linux` | celý program v jednom souboru, tak jak jej instaluje Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, pomocný program pro náhled NIF, instalátory Visual C++ 2015-2022 a .NET 8 Desktop Runtime a runtime .NET 10, ikony, oznámení o licencích a instalátor, který zaregistruje položku na ploše a obsluhu stahování z Nexusu |

Nejdřív stažený soubor ověřte, jak ukazují
[zásady podepisování kódu](#zásady-podepisování-kódu), potom:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Instalátor umístí binárky do `~/.local/bin` bez sudo a zkopíruje `runtimes/` z balíku do
`~/.local/share/Colony/Eidos/runtimes`. `--system` a `--bindir DIR` umístí binárky jinam a
`--from DIR` je vezme z jiné složky. `--cap` požádá sudo o `CAP_SYS_ADMIN`, které potřebuje
jen experimentální FUSE passthrough, a passthrough zůstává vypnutý; `--no-cap` je výchozí.
Oba soubory potřebují Linux x86_64 s glibc 2.39 nebo novějším, FUSE (`fusermount3`) a 7-Zip
pro archivy s módy.

Eidos se neaktualizuje sám. Jednou denně při startu se okno zeptá GitHubu na nejnovější
vydání a když je venku novější, ukáže odkaz na jeho poznámky k vydání za Help v řádku
nabídek. Preferences to vypnou a kopie pod `/usr` mimo `/usr/local`, kterou aktualizuje
správce balíčků, se neptá nikdy.

### Sestavení ze zdrojových kódů

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Vyžaduje Rust 1.92 nebo novější (`rust-version`, které kontroluje CI; CI a vydání sestavují
s 1.99.1), plus `just`, CMake 3.20+ a překladač C++17 pro pomocný program náhledu NIF.
`just build` používá profil release: jednu codegen jednotku a fat LTO. Čisté sestavení
`eidos` a `eidos-gui` se 3 úlohami trvalo tak 334 sekund a s thin LTO 243 sekund a fat LTO
zmenší `eidos` o 1,2 MB a `eidos-gui` o 3,8 MB (měření v `Cargo.toml`).
`just profile=dev build` se kompiluje rychleji. Balíček pro Arch se sestaví z
`packaging/PKGBUILD`. Prerekvizity a příkazy bez `just`:
[docs/guide/install.md](docs/guide/install.md).

### Parametry spuštění ve Steamu

Eidos běží jako spouštěcí příkaz hry, a právě tak stihne připojit dřív, než se hra rozběhne.
Základní řádek je všechno, co většina sestav potřebuje, s absolutní cestou k vaší instalaci
(Steam nečte váš `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Z Colony je ta cesta `~/.local/share/Colony/apps/Eidos/eidos-linux`. Potom stiskněte Hrát:
Eidos se otevře na instanci té hry a Run spustí hru skrz sloučený pohled.

V okně, které jste otevřeli sami, Run požádá Steam o spuštění hry
(`steam -applaunch <appid>`), takže parametr spuštění stále platí a v okně, které Steam
otevře, stisknete Run. Heroic dostane odkaz `heroic://launch` a Legendary spustí
`legendary launch <app>`, jakmile je tam Eidos nastaven jako obal hry. Když nastavení
obchodu chybí, Run nespustí nic a řekne, co nastavit:
[guide/usage.md](docs/guide/usage.md#spuštění-hry). Se zapnutým Start the game at once
when a store opens Eidos (Preferences, ve výchozím stavu vypnuto) Eidos spustí kontroly
Diagnostics a spustí hru bez zobrazení okna, když nic nepotřebuje pozornost.

Všechno ostatní jsou proměnné prostředí naskládané před něj a volně se kombinují:

| Chcete... | Dejte dopředu |
|---|---|
| DLSS s Community Shaders | `PROTON_ENABLE_NVAPI=1`. Bez ní se DLSS nikdy neinicializuje a nic neřekne; kompletní seznam je [guide/graphics.md](docs/guide/graphics.md) |
| počítadlo FPS na obrazovce | `DXVK_HUD=fps` |
| interpolaci snímků na úrovni ovladače, nula módů (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, nikdy zároveň s vlastní generací snímků z Community Shaders |
| podrobné logy pro hlášení chyby | `EIDOS_LOG=debug` (logy relací přistanou v `~/.local/share/Colony/Eidos/logs/`) |
| zprávu o I/O z připojení za jednu relaci | `EIDOS_FUSE_STATS=1` |
| jiný počet pracovních vláken FUSE | `EIDOS_FUSE_THREADS=8` (výchozí 4; `1` je první věc, kterou zkusit při honu na souběhovou chybu) |
| toto spuštění připnuté k jedné přenosné instanci | `EIDOS_INSTANCE=/path/to/folder`. Bez ní Eidos otevře instanci, kterou jste použili naposledy |
| okno pro jedno spuštění, když je zapnuté Start the game at once | `EIDOS_WINDOW=1` |

Řádek, který si nechat pro moderní moddovanou sestavu (Community Shaders, DLSS, generace
snímků), je finální příkaz, ne příklad:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Přidejte dopředu `DXVK_HUD=fps`, dokud ověřujete, že sestava funguje, a jakmile funguje,
zase ho odeberte.

Tytéž proměnné mohou jít místo řádku ve Steamu do proměnných spuštění profilu, což je drží
po profilech a uplatní je i u běhů nástrojů:
[guide/usage.md](docs/guide/usage.md#proměnné-a-obaly-pro-profil).

Hlubší diagnostické přepínače (`EIDOS_FUSE_TRACE`, přepínače pro bisekci cache a indexu,
proč je `EIDOS_FUSE_PASSTHROUGH` ve výchozím stavu vypnutý) jsou v
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Dokumentace

Všechno je v [docs/](docs/README.md), seřazené podle toho, kdo čte, v
[docs/README.md](docs/README.md): uživatelská příručka (instalace, příkazová řádka a GUI,
nástroje jako xEdit a DynDOLOD, grafika, řešení problémů), jak Eidos funguje uvnitř a proč
existuje. Bezpečnostní politika a jak nahlásit zranitelnost:
[SECURITY.md](../../../SECURITY.md).

## Jazyk

Stránky, které hráč potřebuje, jsou přeložené. Kanonická je angličtina: když s ní překlad
nesouhlasí, pravdu má anglický soubor.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Všechno ostatní je anglicky. `docs/internals/` a `docs/project/` čtou lidé, kteří zároveň
čtou Rust, `CHANGELOG.md` se generuje a `SECURITY.md` se také nepřekládá. Překlad prvních
dvou by přidal víc než 28 000 slov, které je třeba udržovat, pro čtenáře, kteří je
nepotřebují.

Každý překlad nese hash anglického souboru, ze kterého vznikl, a CI selže, dokud tento hash
angličtině neodpovídá; viz [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Překlad
se pak aktualizuje a znovu označí přes `scripts/i18n-check.sh --fix`. Zastaralý překlad se
aktualizuje, nemaže se.

Jazyk je jeden adresář. `docs/i18n/<lang>/` zrcadlí kořen repozitáře, takže
`docs/i18n/de/docs/guide/install.md` je německý `docs/guide/install.md`. Právě díky tomu je
odkaz mezi dvěma přeloženými stránkami tentýž řetězec jako odkaz mezi jejich anglickými
originály a vyřazení jazyka je jedno `rm -r`.

## Zásady podepisování kódu

Každý soubor vydání podepisuje organizace Project Colony v CI, nikdy na stroji vývojáře.
Vedle každého souboru na stránce vydání:

| Soubor | Co to je |
|---|---|
| `<asset>.sig` | podpis ed25519 nad souborem, vytvořený klíčem vydání organizace |
| `<asset>.meta` | tři řádky, které soubor svazují s jeho názvem, jeho sha256 a verzí vydání |
| `<asset>.meta.sig` | podpis ed25519 nad `.meta` |

Soukromý klíč je tajemství organizace, které používá jen sdílený
[workflow sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
v úloze, která nic nesestavuje; úlohy, které kompilují Eidos, jej nikdy nevidí. Colony před
instalací nebo aktualizací Eidosu kontroluje všechny tři soubory a odmítne vydání starší než
to nainstalované. Stažený soubor si ověříte sami pomocí OpenSSL 3 (tytéž příkazy fungují pro
každý soubor):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Jak se vydání sestavují, podepisují a zveřejňují:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)
(anglicky).

## Soukromí

Eidos neposílá žádnou telemetrii, žádnou analytiku a žádná hlášení o pádech. Připojuje se
jen k serverům v této tabulce, a to jen kvůli akci, která je u nich uvedena. Režim Offline
(Preferences, Nexus) zastaví každé spojení s Nexusem a denní kontrolu vydání.

| Data | Uložena, nebo odeslána | Kam a proč |
|---|---|---|
| Předvolby, seznam instancí, definice her a doplňků | uložena | `~/.config/Colony/Eidos/` |
| Instance, runtimy nástrojů, logy relací | uložena | `~/.local/share/Colony/Eidos/`, nebo složka, kterou jste zvolili pro přenosnou instanci |
| Masterlisty LOOT | uložena | `~/.cache/Colony/Eidos/loot/` |
| Kontrola vydání | odeslána | Při startu, nejvýš jednou denně, jeden požadavek na `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos čte jen značku vydání a nic nestahuje. Je ve výchozím stavu zapnutá a v Preferences se vypíná, v režimu Offline se přeskočí a přeskočí se i u kopie nainstalované pod `/usr` mimo `/usr/local`, kterou aktualizuje správce balíčků. [Prohlášení GitHubu o ochraně soukromí](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Přihlášení k Nexusu | odeslána | Přihlášení otevře ve vašem prohlížeči `users.nexusmods.com`, který výsledek předá zpět naslouchači, jejž Eidos otevře na `127.0.0.1` jen pro toto přihlášení. Eidos ho pak vymění za tokeny na `users.nexusmods.com/oauth/token` a stáhne veřejné klíče, kterými je Nexus podepisuje. [Zásady ochrany soukromí Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Přihlašovací údaje k Nexusu | uložena | Přístupový a obnovovací token, v `~/.config/Colony/Eidos/nexus.ini` s režimem 0600. Řádek `api_key=`, který tam zapsal starší Eidos, se zachová, ale nikdy se nečte; klidně ho smažte. |
| Požadavky na Nexus | odeslána | `api.nexusmods.com`, když použijete funkci Nexusu: jméno vašeho účtu a stav premium, podrobnosti o módech a souborech, kontroly aktualizací, kolekce a odkazy ke stažení. Vaše schválení a sledované módy se čtou při každé kontrole aktualizací a zapisují, když stisknete Endorse nebo Track. Požadavky módů, popisy a changelogy a hledání kolekcí jdou na `api.nexusmods.com/v2/graphql`. Každý požadavek nese váš přístupový token a uvádí Eidos a jeho verzi. |
| Stahování z Nexusu | odeslána | Soubory, o které požádáte, z hostitele CDN Nexusu, kterého uvádí odkaz ke stažení, uložené do složky stahování instance s `.meta` ve formátu MO2 vedle každého z nich. |
| Odkazy `nxm://` | uložena | `eidos nxm --register`, který za vás spustí `install.sh`, zapíše `~/.local/share/applications/eidos-nxm.desktop` a přes `xdg-mime` z něj udělá obsluhu `nxm://`, takže tlačítko Mod Manager Download na webu předá svůj odkaz Eidosu. Balíček pro Arch instaluje stejnou položku pro celý systém. |
| Masterlisty LOOT | odeslána | `raw.githubusercontent.com/loot/...`, při prvním řazení LOOTem pro danou hru a kdykoli požádáte o aktualizaci. Nic vašeho se neodesílá. [Prohlášení GitHubu o ochraně soukromí](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimy nástrojů | odeslána | Jen když instalujete prerekvizity nástroje, a jen pro to, co balík vydání nepřibalil (samostatný soubor `eidos-linux` a sestavení ze zdrojů nepřibalují nic): runtime .NET 10 z `builds.dotnet.microsoft.com` a instalátory Visual C++ a .NET 8 z `download.visualstudio.microsoft.com`, každý odmítnutý, pokud neodpovídá SHA-256 zabudovanému v Eidosu; SDK .NET 10, vždy z `builds.dotnet.microsoft.com` a ověřené stejně; a `winetricks` pro ostatní prerekvizity (.NET Framework 4.8, XACT, .NET 6 a 7), který to, co instaluje, stahuje od Microsoftu a z dalších hostitelů, které winetricks používá. [Prohlášení Microsoftu o ochraně osobních údajů](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Logy relací | uložena | `~/.local/share/Colony/Eidos/logs/`, režim 0600: deset relací na instanci, smazané po 30 dnech. Vaše domovská složka se zapisuje jako `~` a vaše uživatelské jméno jako `<user>`, takže log lze vložit do issue tak, jak je. Váš stroj nikdy neopustí, pokud je sami neodešlete. |
| Záznamy běhů | uložena | `<instance>/logs/sessions/`: pro každý běh hry nebo nástroje profil, zapnuté módy s jejich verzemi a id souborů z Nexusu, pořadí pluginů, kontrolní součet každého INI profilu, verze hry a Eidosu, kód ukončení a uložené pozice, které běh vytvořil. Uchovává se 50 nejnovějších, čte je `eidos sessions` a `eidos pack` je vynechává. Zůstávají na vašem stroji. |
| Zprávy o aktualizacích | uložena | `<instance>/logs/updates/<mod>.json`: co poslední aktualizace nebo Replace změnily ve složce toho módu. `eidos pack` je vynechává. |
| Zástupci | uložena | `install.sh` zapíše `eidos.desktop` a jeho ikony pod `~/.local/share`; zástupce nástroje nebo profilu, který vytvoříte, jde do `~/.local/share/applications`. |
| Oznámení na plochu | ani jedno | Ve výchozím stavu vypnuto. Po zapnutí jdou do služby oznámení vaší plochy přes D-Bus (`org.freedesktop.Notifications`) a jmenují úlohu, která skončila. |
| Odkazy, které otevřete | ani jedno | Stránky módů a další odkazy se otevírají ve vašem vlastním prohlížeči přes `xdg-open`. |
| Doplňky | ani jedno | Rozšíření, která nainstalujete, běží na vašem stroji jako samostatné programy; co dělají, je jejich věc. |

## Licence

Původní kód Eidosu je pod licencí GPL-3.0-or-later; viz [LICENSE](../../../LICENSE). Tyto
části jsou pod GPL-3.0 bez možnosti „or later":

- překlady skriptů OMOD v `eidos-install`; viz jejich
  [poděkování původním autorům](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- pomocný program `eidos-nif-preview`, postavený na přibalené knihovně nifly; viz jeho
  [licence součástí a poděkování autorům](../../../crates/eidos-nif-preview/UPSTREAM.md);
- crate `esplugin` a `libloadorder`, které linkuje kód pořadí načítání a které deklarují
  `GPL-3.0`.

Ostatní crate jazyka Rust v binárních souborech si ponechávají své vlastní licence (MIT,
Apache-2.0, BSD a podobné). Balík vydání nese jejich oznámení v `licenses/crates/` spolu s
přehledem každého crate a jeho licence.

Binární soubory také obsahují osm pravých redistribuovatelných DLL Microsoft DirectX
(`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` a `d3dx11_43`, 32- a 64bitové), uložených
komprimovaně gzipem (7,9 MB místo 17,9 MB), které Eidos rozbalí a nasadí do prefixu hry,
když je potřebuje mód nebo nástroj. Zůstávají pod redistribučními podmínkami Microsoftu;
viz jejich [původ](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Balík vydání také nese pod `runtimes/` instalátory Visual C++ 2015-2022 od Microsoftu,
instalátor .NET 8 Desktop Runtime a zip runtimu .NET 10, tak jak je Microsoft publikuje.
Jejich podmínky jsou v `licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
