<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# Řešení potíží a diagnostika

Přepínače prostředí, jak číst čítače operací, karty Health v záložce Diagnostics,
jak najít mód za pádem, co přiložit k hlášení chyby a známé problémy s jejich
historií.

## Diagnostika VFS

Dvě proměnné prostředí pokryjí většinu případů, kdy hra vidí něco, s čím souborový
systém nesouhlasí:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

Právě podrobná podoba našla pád `FOPEN_KEEP_CACHE` popsaný ve Známých problémech:
vypnutí všech čtyř odpoví na „je to cachování?", a teprve jejich pojmenování
odpoví „které". Čítače odpovídají na druhou půlku. Se zapnutým passthroughem (ve
výchozím stavu je vypnutý) je načtení, které ukazuje `read 0`, takové, kde
`FUSE_PASSTHROUGH` obsloužil každý bajt v jádře, takže cokoli, co jste se chystali
ladit na čtecí cestě, je už zadarmo.

Další přepínače zúží chybu ještě víc:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

Při spuštění ze Steamu je dejte před příkaz v parametru spuštění, například
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. Výstup jde do logu běhu:
do `logs/run-*.log` instance při spuštění z okna, do konzole Steamu při spuštění
přes Steam.

Samotný sloučený pohled si prohlédnete přes `eidos shell <instance>`, který otevře
shell ve složce hry tak, jak ji vidí hra, a
`eidos shell <instance> -- ls Data/meshes` tam spustí jeden příkaz; cokoli se zapíše,
skončí v Overwrite ([usage.md](usage.md#shell-ve-sloučeném-pohledu)). Tlačítko Save
tree v záložce Data zapíše celý sloučený strom `Data` do textového souboru, jeden
soubor na řádek s módem, ze kterého pochází, s domovskou složkou skrytou.

## Připojit sjednocení ručně

`eidos-fuse` je v balíčku pro Arch a v sestaveních ze zdrojů: `just build` ho
sestaví a `just install` nebo `install.sh` ho zkopíruje, když tam je. Balík vydání
a `eidos-linux` ho nenesou. Jeho řádek použití:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

První `--layer` vyhrává při konfliktu; poslední jsou vaše nedotčená data hry.
Připojení potřebuje jen `/dev/fuse` a `fusermount3` (žádný overlayfs, žádný
Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Zápisy přistávají v `--overwrite <dir>` (dočasný adresář, když jej vynecháte),
takže samotné vrstvy zůstávají nedotčené i tady.

## Proč je passthrough ve výchozím stavu vypnutý

Passthrough předá jádru skutečný podkladový soubor, takže čtení tohoto démona
úplně obcházejí. Měření A/B níže proběhlo s dřívější verzí démona, před změnou
popsanou dál ve známém problému „FUSE passthrough stops the game loading any mod
content". Tato změna se vyhýbá nejpravděpodobnější příčině, ale ve hře zatím
nebyla znovu změřena, takže passthrough zůstává ve výchozím stavu vypnutý, dokud
se to nestane. Měřeno A/B na Skyrim SE 1.6.1170, proton-cachyos 11.0, jádro
7.1.4, stejné pořadí načítání s 82 pluginy, jediná proměnná byla, zda binárka
nesla tu capability:

| passthrough | selhání `NtCreateFile` se `STATUS_ACCESS_VIOLATION` |
|-------------|--------------------------------------------------------|
| zapnutý     | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| vypnutý     | 0                                                      |

Se zapnutým hra neotevřela žádný ze svých archivů ani pluginů, což se ve hře
projevilo jako módy, které nejsou - žádná chyba, žádný řádek v logu. S vypnutým
totéž pořadí načítání došlo ke hraní se živými pluginy, archivy a Papyrus
skripty.

Selhání bylo zevnitř démona neviditelné: jeho vlastní `open` uspělo pokaždé a
jádro nikdy neodmítlo podkladový soubor (ověřeno napříč celou selhávající relací
s `EIDOS_FUSE_TRACE=open`: nula `open FAILED`, nula `passthrough refused`). Chyba
vznikala až poté, co démon odpověděl `opened_passthrough`, takže ji žádné logování
na straně démona nemohlo vidět. Nebyla ani vázaná na příponu - trefovala archivy
stejně jako pluginy, tedy soubory, které hra drží otevřené po celý svůj běh.

`EIDOS_FUSE_PASSTHROUGH=1` jej zase zapne, pro měření toho, co přináší, nebo pro
opětovné testování mechanismu. Varování o capability ve spouštěči a v záložce
Diagnostics se objeví, jen když jste si o něj řekli.

## Používá se vlastně index vrstev?

`LayerStack::new` buď postaví úplnou mapu vrstev jen pro čtení, nebo žádnou.
Bez mapy každý dotaz prochází vrstvy jako dřív: odpovědi jsou stejné, načtení
uložené hry trvá zhruba třikrát déle a nic jiného nevypadá jinak.

Když je index vypnutý, připojení řekne proč, jednou, při startu:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

Důvod jmenuje nečitelný adresář, vrstvu hlubší než 64 adresářů, více než
4 000 000 položek, nebo `EIDOS_NO_INDEX`. Řádek jde do logu běhu: do
`logs/run-*.log` instance při spuštění z okna, do konzole Steamu při spuštění přes
Steam. S `EIDOS_FUSE_STATS=1` má výpis při odpojení také řádek `layer index:` a
nenulový počet `unindexed` na běžném připojení znamená, že index byl vypnutý.

Příklady níže kontrolují totéž offline, nad složkami instance, bez připojení:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` rozřeší skutečné cesty s indexem i bez něj a porovná průchody
adresáři. `index_agrees` kontroluje, že oba odpovídají TOTÉŽ, na každé cestě
a každém výpisu skutečné instance. `listing_cost` měří, co mapa sloučených
potomků ušetří na `readdir`.

`EIDOS_NO_INDEX=1` vynutí průchod, pro chvíle, kdy je laděným předmětem právě
rozdíl mezi oběma odpověďmi.

## Spuštění hry přes CLI

Chcete-li hru spustit přes CLI `eidos` místo okna, nastavte její parametr spuštění
ve Steamu na absolutní cestu k `eidos`, například:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Žádná předpona `WINEDLLOVERRIDES` není potřeba. Když zapnutý mód importuje
`d3dcompiler_47`, Eidos nasadí do prefixu svou přibalenou kopii, vynutí ji jako
nativní a sloučí to s jakýmikoli DLL overrides, které přináší mód
(ENB/ReShade/`.asi` loadery).

Eidos nikdy nespouští hru přes Proton sám. `eidos tool <instance> run <title>` se
záznamem, který hru spouští (loader script extenderu, launcher nebo binárka hry),
požádá o spuštění obchod, který kopii vlastní, stejně jako Run v okně, které
neotevřel žádný obchod, a vypíše, co nastavit, když nastavení obchodu Eidos
nespouští. `--print` vypíše příkaz obchodu místo jeho spuštění.

## Hra nebo nástroj, který se nezavře

Zatímco hra nebo nástroj běží, tlačítko Run se změní na Stop a karta zámku má také
tlačítko Stop. Stiskněte ho dvakrát. Druhé stisknutí ukončí každý program, který
spuštění zahájilo: nejprve SIGTERM, po 10 sekundách SIGKILL pro to, co zbylo. Co
hra neuložila, je ztraceno. Eidos pak udělá to, co po každém ukončení: změny INI a
pluginů se vrátí do profilu, výstup nástrojů se zaznamená a módy se odpojí.
Běh se zaznamená s kódem ukončení 143, ať program vrátil cokoli, takže
`eidos sessions <instance> diff` ho nikdy nevezme jako poslední dobrý běh. Unlock
je něco jiného: vrátí okno a nechá program běžet. Z terminálu udělá `kill <pid>`
na proces `eidos` (pid je na kartě zámku) totéž co Stop.

## Hledání toho, co se rozbilo

Chcete-li zjistit, která změna hru rozbila:

- `eidos sessions <instance> diff` vypíše, co se změnilo od posledního běhu hry,
  který skončil kódem 0: módy, verze, pořadí pluginů, INI a spustitelný soubor hry
  ([usage.md](usage.md#co-se-změnilo-od-posledního-dobrého-běhu)).
- Test run vedle Run hraje s módy profilu, bez módů, nebo s vybranými módy a
  mastery, které potřebují, a vše, co hra zapíše, posílá do `<instance>/test-run/`.
  Po běhu si každou skupinu souborů ponecháte nebo zahodíte: uložené pozice, změny
  INI, stav pluginů, výstup do Overwrite a logy ([usage.md](usage.md#testovací-běhy)).
- Find the mod behind a problem v záložce Diagnostics spouští testovací běhy vždy
  s polovinou zbývajících podezřelých a ptá se, zda se problém objevil. 200
  podezřelých zabere asi 8 běhů a hledání přežije zavření Eidosu
  ([usage.md](usage.md#hledání-módu-za-problémem)).

Zastavený běh v tomto hledání nikdy předem nevyplní Yes, protože se zaznamená s
kódem ukončení 143 (viz výše).

## Karty Health

Záložka Diagnostics ukazuje kartu Health pro každý nalezený problém, s příčinou a
u některých s tlačítkem, které ho opraví. Karty parametru spuštění popisuje
[install.md](install.md), karty NVAPI, celé obrazovky a ENB
[graphics.md](graphics.md) a karty Protonu a prefixu následující část.
`eidos check <instance>` vypíše zjištění, která okno nepotřebují
([usage.md](usage.md#kontroly-health-z-terminálu) je uvádí), mezi nimi karty
prefixu, registru a jednotky `S:`. Samotné okno ukazuje kartu Proton s jejími
variantami pro Flatpak, neznámé sestavení a starší sestavení, pořadí patchů,
skripty extenderu, archivy, které žádný plugin nenačítá, požadavky z Nexusu,
zastaralé kopie pluginů a kartu Display.

### Systém

- „vm.max_map_count is <value>" se objeví pod 1048576, což je výchozí hodnota na
  Arch Linuxu, Fedoře 39 a novější a Ubuntu 24.04 a novějším. Některé hry pod
  Protonem mapují víc oblastí paměti a pak bez chyby spadnou. Dejte
  `vm.max_map_count = 1048576` do souboru v `/etc/sysctl.d`, například
  `80-gamecompatibility.conf`, a spusťte `sudo sysctl --system`.
- „The open files limit is <value>" se objeví, když je pevný limit pod 524288, což
  potřebuje esync Protonu; pod ním hry mohou nejít spustit nebo padat. systemd 240
  a novější ho nastavuje. Na starším systému nastavte `DefaultLimitNOFILE=524288`
  v `/etc/systemd/system.conf` a `/etc/systemd/user.conf` a přihlaste se znovu.
- „Game files on a Windows file system" jmenuje složku hry, prefix nebo kořen
  instance, když leží na `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse), `exfat`
  nebo `vfat`. Proton potřebuje symlinky, unixová oprávnění a názvy rozlišující
  velikost písmen. Přesuňte hru, její prefix a instanci na linuxový souborový
  systém, například ext4, btrfs nebo xfs.

### Soubory hry

Karta Game files nahoře v záložce ukazuje, zda instance spouští instalaci z obchodu,
nebo kopii hry uvnitř instance
([usage.md](usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie)). Spuštění se odmítne,
s kartou, která řekne proč, ve třech případech:

- „The game copy '<folder>' is missing": instance spouští kopii, která zmizela.
  Vraťte ji, nebo se vraťte k verzi z obchodu.
- „'<folder>' has no <binary>": složka, kterou instance spouští, neobsahuje
  spustitelný soubor hry, takže to není kopie hry.
- „<game> cannot run from a copy yet": Morrowind a Oblivion s
  `bUseMyGamesDirectory=0` drží seznam pluginů ve složce hry.

„ModOrganizer.ini points tools at another game folder" se objeví, když vlastní
`ModOrganizer.ini` seznamu, který nezapsal Eidos, má `gamePath` jiný než soubory
hry, které instance spouští. Nástroje v režimu Mod Organizer 2, například
PGPatcher, čtou hru odtud. Změňte `gamePath` v tom souboru, nebo soubor odstraňte,
aby Eidos zapsal vlastní.

### Pořadí načítání

- „sTestFile lines make the game ignore plugins.txt" se objeví u Falloutu 4,
  Falloutu 4 VR a Starfieldu, když `Fallout4Custom.ini` nebo `StarfieldCustom.ini`
  profilu má v `[General]` řádky `sTestFile1` až `sTestFile10`. Dokud je některý
  nastaven, hra načítá tyto pluginy a pořadí načítání, které Eidos zapisuje, nemá
  žádný účinek. Fix řádky z kopie profilu odstraní a předchozí soubor zachová jako
  `<ini>.pre-fix`.
- Starfield: zapnuté pluginy `.esp` bez příznaku master dostanou kartu, protože
  Starfield je nedokáže seřadit mezi `.esm` a pluginy s příznakem master a drží
  některé jejich záznamy načtené po celou relaci. Vydání autora jako `.esm` se
  vyhne oběma.
- Starfield: „Invalid blueprint plugins" vypíše pluginy, které mají některé, ale ne
  všechny z následujících: příznak blueprint, název začínající `BlueprintShips-` a
  příponu `.esm`. „Blueprint plugins without their main plugin" vypíše ty, jejichž
  hlavní plugin není zapnutý: `BlueprintShips-Example.esm` potřebuje `Example.esm`,
  `.esp` nebo `.esl`.
- „<patch> loses <n> file(s) to <master>" se objeví, když plugin v jednom módu
  potřebuje master z jiného módu a ten druhý mód má vyšší prioritu, takže vyhrávají
  jeho kopie souborů, které dodávají oba. Patch obvykle patří pod mód, který
  záplatuje, a tlačítko Move na kartě ho tam dá. Pořadí ponechte, pokud má patch
  tyto soubory prohrát.
- „Out-of-date copy of <plugin>" se objeví, když mód drží vyčištěnou kopii pluginu
  nebo kopii s příznakem light, jejíž originál se od té doby změnil (aktualizace
  módu nebo hry). Hra načte starou kopii. Skryjte ji a pak nový plugin vyčistěte
  nebo označte znovu.

### Seznam módů

- „Order rules of the collection <name> that no longer hold" se objeví, když módy
  nainstalované z kolekce už neleží v pořadí, které její pravidla žádají. Restore
  collection order přesune zpět jen módy té kolekce a Ctrl+Z to vrátí.
- „<mod> requires <mod>, which is not installed" (nebo „which is disabled")
  pochází z požadavků z Nexusu, které čte Check for updates. Open on Nexus otevře
  stránku požadovaného módu, Download main file ho stáhne na prémiovém účtu a
  Ignore skryje požadavek, který už splňuje jiný mód. Požadavek na DLC je jen rada:
  Eidos hledá plugin s názvem DLC.
- „fontconfig.txt names a font library that is missing" (enginy Skyrimu) jmenuje
  fonty, které nemá žádná vrstva ani archiv, a mód, jehož
  `Interface/fontconfig.txt` vyhrává. Text v nabídkách se může zobrazovat jako
  čtverečky, dokud se font nenainstaluje nebo se řádek neodstraní.
- „BA2 archive stores audio compressed" (Fallout 4 a Fallout 4 VR) spočítá soubory
  `.wav` a `.xwm`, které BA2 módu ukládá komprimované. Hra může při jejich přehrání
  spadnout. Přebalte archiv se zvukem bez komprese.

### Script extender a aktualizace hry

- „<loader> is not installed" se objeví, když jsou nainstalované pluginy script
  extenderu (v `SKSE/Plugins`, `F4SE/Plugins` a podobně), ale loader není v žádném
  zapnutém módu ani ve složce hry. Hra se spustí bez nich.
- „The script extender does not match game version <version>" se objeví, když je
  loader nainstalovaný, ale runtime DLL pro verzi běžící hry ne, například
  `skse64_1_6_1170.dll`. Loader skončí s chybou. Obvykle to následuje po aktualizaci
  hry. Tato karta i ta výše mají tlačítko Open the download page pro web extenderu
  a tlačítko Install the downloaded archive, které vloží loader do složky `Root/`
  módu.
- „<game> changed since the last clean launch" se objeví, když se spustitelný
  soubor hry, nebo u kopie ze Steamu její build id, změnil od posledního spuštění
  přes Eidos, které skončilo bez chyby. Aktualizace Steamu, která spustitelný
  soubor nechá, jak byl, ji nevyvolá. Pluginy script extenderu postavené pro starou
  verzi se mohou přestat načítat a karty pod ní jmenují ty, které to deklarují.
  Chcete-li zachovat verzi hry, na které byl seznam postaven, zmrazte ji do
  instance tlačítkem Freeze this version na kartě Game files, nebo
  `eidos stock-game <instance> freeze --yes` poté, co samotné `freeze` ukázalo
  velikost: pozdější aktualizace z obchodu změní instalaci z obchodu, ne kopii.
- „<mod> replaces <n> <extender> script(s)" se objeví, když mód s vyšší prioritou
  než script extender má volné kopie `.pex` vlastních skriptů extenderu. Stará
  kopie rozbije funkce, které deklaruje, a hra nezobrazí žádnou chybu. Přesuňte
  extender pod ten mód (tlačítko Move na kartě to udělá, jsou-li oba módy), nebo ty
  soubory v módu skryjte. Když skripty extenderu leží volně v `Data` hry,
  nainstalujte extender jako mód, aby šel řadit. Kontrolují se jen volné soubory,
  ne archivy.

### Výstup nástrojů

- „Overwrite/Root holds meshes" (nebo textures) znamená, že nástroj, jehož cesta k
  datům mířila na složku hry, tam zapsal tyto složky, kde je hra nikdy nečte.
  Opravte cestu nástroje, složky smažte a nástroj spusťte znovu. Tlačítko Open
  folder otevře `Overwrite/Root`.
- „BodySlide's GameDataPath is not the game's Data folder" (nebo
  `OutputDataPath`): viz
  [Tools](tools.md#vlastní-nastavení-nástroje-zůstává-jeho).

### Pád v poslední relaci

„The game crashed in the last session" se objeví, když záznamník pádů zapsal log
během posledního spuštění. Karta jmenuje výjimku a ty DLL a pluginy z logu, které
instalují vaše módy nebo Overwrite. Berte je jako stopy: mód jmenovaný v logu pádu
je často tím, kde se pád projevil, ne jeho příčinou. Když log nejmenuje žádný soubor
z vašich módů, karta to řekne. Open the log ho otevře a Copy the log (home path
hidden) ho zkopíruje se skrytou domovskou složkou.

### Velikost okna

U Skyrimu SE, Enderalu SE a Falloutu 4 karta Display ukazuje velikost a režim okna,
s nimiž profil hru spouští, a řekne „No window size set for this profile", když ji
nenastavuje ani INI Prefs profilu, ani prefixu. Apply zapíše velikost a režim do
kopie profilu a do kopie INI módu SSE Display Tweaks v Overwrite, když okno
nastavuje tento mód ([usage.md](usage.md#diagnostika)).

## Který Proton hru spouští, a rozbitý prefix

Záložka Diagnostics má kartu Proton: sestavení, které pro hru používají Eidos a
Steam, vlastní nastavení Steamu a sestavení, které prefix naposledy spustilo. Když
prefix naposledy připravil novější Proton než ten vybraný, karta se změní na radu,
protože starší Proton na novějším prefixu může resetovat jeho soubory Wine nebo se
nespustit. Ve Steamu vyberte ve Vlastnostech hry, Kompatibilitě sestavení alespoň
stejně nové, nebo prefix vytvořte znovu.

Další karty Protonu a prefixu:

- „Proton comes from the Flatpak Steam install": tento Proton drží svůj runtime a
  knihovny steamclient uvnitř sandboxu Flatpaku, takže jeho spuštění z hostitele
  může selhat. Eidos ho nespouští přes `flatpak run`, což by hru dalo do sandboxu,
  mimo sloučený pohled. Nainstalujte Proton do
  `~/.steam/root/compatibilitytools.d/` a vyberte ho pro hru.
- „Eidos cannot tell which Proton runs this game": `config.vdf` Steamu pro ni
  nejmenuje žádný kompatibilitní nástroj a prefix nezaznamenává žádné nainstalované
  sestavení. Nástroje se nespustí, dokud ve Steamu nějaký nevyberete (Vlastnosti
  hry, Kompatibilita).
- „<game> has no Proton prefix yet": spusťte hru jednou přes Steam a stiskněte F5.
  Když Steam prefix nevytvoří, zkontrolujte, že složka hry v
  `steamapps/compatdata` je zapisovatelná; prázdná, kterou vlastní root, ho
  zablokuje bez hlášky.
- „Windows tools cannot find <game>": záznam prefixu o cestě instalace je špatný
  nebo chybí a čtou ho xEdit, DynDOLOD, Wrye Bash a Creation Kit. Eidos zapíše
  správnou cestu při příštím spuštění nástroje.
- „The prefix's S: drive points outside the Steam library": `S:` neodpovídá ani
  kořeni knihovny, na který ho Eidos nasměruje pro nástroj, ani
  `<library>/steamapps`, na který ho Steam nasměruje pro hru. Knihovna se
  pravděpodobně přesunula, nebo prefix pochází z jiného stroje. Každá cesta
  vztažená k `S:` uložená v prefixu se nikam nerozřeší, dokud ji příští spuštění
  přes Eidos nebo Steam nenasměruje znovu.
- „<store> installation: started through its store" je karta kopie z GOG nebo
  Epicu. Run požádá Heroic nebo Legendary o spuštění hry, když je tam Eidos obalem
  hry, a nástroje běží se sestavením Wine nebo Protonu, které Heroic pro kopii
  zaznamenal.

Nabídka File spouští winecfg, regedit, průzkumník souborů Wine a odinstalátor v
prefixu hry, s Protonem, který spouští nástroje hry
([tools.md](tools.md#vlastní-programy-wine-v-prefixu-hry)). Jejich řádky zůstávají
šedé, dokud prefix tyto programy nemá.

Tlačítko Rebuild prefix na kartě Proton je pro prefix, který už nefunguje. Stiskněte
ho dvakrát. Eidos nic nemaže: přejmenuje `steamapps/compatdata/<appid>` na
`<appid>.eidos-backup-<date>-<time>` ve stejné složce a Steam vytvoří nový prefix,
až příště spustíte hru ze Steamu. Odmítne to, dokud prefix používá hra, nástroj
nebo jakýkoli program Wine. Dokud nový prefix neexistuje, Run v okně nemůže hru
předat Steamu a ukáže místo toho parametr spuštění, takže stiskněte Hrát ve Steamu.
Uložené pozice a nastavení, které jsou jen ve starém prefixu, zůstanou v
přejmenované složce: zkopírujte zpět, co potřebujete, a tu složku smažte sami, až ji
už nebudete chtít.

## Co přiložit k hlášení chyby

- Copy report v záložce Diagnostics zkopíruje verzi Eidosu, hru, sestavení Protonu,
  zda je Steam z Flatpaku, stav passthroughu, typ relace, distribuci, jádro,
  `vm.max_map_count`, limit otevřených souborů, zda je přítomný `/dev/fuse` a
  `fusermount3`, názvy spouštěcích proměnných a obalových programů profilu a každou
  kartu Health. Vaše domovská složka se zobrazí jako `~` a přihlašovací jméno jako
  `<user>`. `eidos check <instance>` vypíše stejnou hlavičku v terminálu a `--json`
  ji dá jako JSON.
- Log běhu, `<instance>/logs/run-*.log`, začíná přesným příkazem, který Eidos spustil.
  File, Eidos logs otevře složku vlastních logů Eidosu,
  `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden) na kartě pádu zkopíruje log pádu.
- Save tree v záložce Data uloží sloučený strom `Data` se zdrojem každého souboru.

## Známé problémy

### DLSS nebo generování snímků tiše nedělá nic

Tři samostatné příčiny, každá bez jakékoli chybové hlášky: nezapnuté NVAPI
v parametrech spuštění, exkluzivní režim celé obrazovky, nebo zastaralý strop FPS
v Reflexu. Celý kontrolní seznam je v [graphics.md](graphics.md).

### Run nic nedělá se Steamem z Flatpaku

V okně, které neotevřel žádný obchod, Run požádá Steam o spuštění hry příkazem
`steam -applaunch <appid>`. Steam z Flatpaku nedává na `PATH` hostitele žádný
`steam`, takže stavový řádek hlásí „Could not start Steam" s chybou. Stiskněte
místo toho Hrát ve Steamu.

### Uložená pozice z jiného zařízení se nezobrazuje

Uložená pozice, která se do prefixu dostane mimo Eidos, například stažení ze Steam
Cloudu ze Steam Decku, zůstane skrytá, dokud jsou uložené pozice profilu připojené.
Nic ji nemaže. Záložka Saves tyto pozice počítá pod svým nadpisem a Import je
zkopíruje do profilu, aniž by nahradil pozici profilu
([usage.md](usage.md#uložené-pozice-patří-profilu)).

### Mód, který píše jeden adresář dvěma způsoby, přišel o všechno pod tím druhým

Opraveno. ext4 drží `meshes/` a `Meshes/` odděleně; sloučený pohled nesmí, a
skutečné módy dodávají obojí - XP32 Maximum Skeleton má své animace a svůj FNIS
behaviour soubor pod tím s velkým písmenem, své `character assets` pod tím druhým.

Resolver vzal pro každou složku cesty shodu s přesnou velikostí písmen a držel se
jí: vstoupil do `meshes/`, nenašel tam zbytek cesty a zahodil CELOU VRSTVU. Každý
soubor pod druhým zápisem byl pro hru neviditelný - žádná chyba, žádný log, nic v
žádné diagnostice. Na skutečné instanci s 50 vrstvami to bylo 74 souborů.

Složka, která se shoduje, je teď kandidát, ne rozhodnutí; přesná velikost písmen se
stále zkouší první a teprve když pod ní selže zbytek, hledá průchod sourozence
shodné po sjednocení velikosti písmen. Výpisy měly tutéž chybu o adresář výš a teď
čtou každý takto shodný adresář na vrstvu.

Index cest tuhle chybu nikdy neměl, protože prochází každý adresář, který najde.
Po nějakou dobu tedy index vracel soubory, které záložní cesta vrátit nedokázala,
ačkoli záložní cesta je ta odpověď, která nemá být nikdy špatně.

### LODGen z DynDOLODu umírá a nechává prázdný log

Opraveno pomocí `dotnet10`; viz [tools.md](tools.md). `LODGen_SSE_<world>_log.txt`
každého světa obsahuje hlavičku s verzí, řádek `.NET Version:` a nic víc a dialog
říká pouze „failed to generate object LOD for one or more worlds". Příčinou je Wine
Mono odpovídající za .NET Framework a žádné množství instalací .NET Frameworku to
nespraví - Proton nahrazuje `mscoree.dll` symlinkem do svého vlastního stromu při
každé aktualizaci prefixu. Balík vydání nese runtime .NET 10; ostatní instalace ho
stáhnou jednou.

### Wine nedokázal poznat, že připojení sjednocuje velikost písmen

Opraveno.

Wine se ve své `get_dir_case_sensitivity` nejprve zeptá adresáře přes
`FS_IOC_GETFLAGS` a potom čichá po značce, kterou CIOPFS nechává v adresářích, jež
obsluhuje. Když chybí obojí, Wine předpokládá case-SENSITIVE a každé vyhledání,
jehož zápis se neshoduje bajt po bajtu, spadne zpátky ke čtení CELÉHO adresáře,
aby našlo shodu bez ohledu na velikost písmen. Hry od Bethesdy se ptají na
`data/ccbgssse001-fish.bsa`, zatímco soubor je `ccBGSSSE001-Fish.bsa`, takže to
vystřelilo skoro u každého assetu: 4471 dotazů na značku a 2236 úplných opětovných
čtení adresáře za osm sekund a 195796 výčtů `Data` za devadesát. Skyrim SE se nikdy
nedostal do hlavní nabídky - seděl na 240 MB rezidentní paměti, zatímco démon
spaloval 92 % jádra.

Eidos sjednocoval velikost písmen v `resolve_read` od začátku. Celá ta cena byla za
to, že to nikdy neřekl. `lookup` teď odpovídá `.ciopfs`; `readdir` jej stále
nevypisuje. `ioctl` navíc na adresářích odpovídá na `FS_IOC_GETFLAGS` příznakem
casefold, na který se současný Wine ptá dřív, než začne hledat značku.

Fatálním, ne jen pomalým, to udělaly dvě věci. Cena roste s velikostí adresáře,
takže instalace obsahu Anniversary (`Data` ze 37 souborů na 177) to překlopila. A
`opendir` dychtivě stavěl sloučený výpis, což je čirá ztráta, když Wine otevírá
adresář jen proto, aby uvnitř udělal `stat` na tu značku - snímek se teď bere až při
prvním `readdir`.

Potom: hlavní nabídka, 2,1 GB rezidentní paměti, démon na 0 % CPU.

`EIDOS_FUSE_TRACE=opendir` je to, co to našlo, a dodává se. Čítače operací říkají
kolik; 195796 výčtů jednoho adresáře je v součtu neviditelných.

### Hra přepsala `plugins.txt` naprázdno

Velmi pravděpodobně totéž co sjednocování velikosti písmen výše - `Data`, které
nedokázala v rozumném čase vyjmenovat, takže usoudila, že tam nic není, a uložila to.
Neprokázáno a stojí za opětovné ověření. Tak či tak pojistka zachytávání (zachycení,
které úplně vyprázdní aktivní sadu, je odmítnuto při jakékoli velikosti) znamená, že
už profil poškodit nemůže.

### `FOPEN_KEEP_CACHE` je vypnutý

Opraveno. Shazoval Skyrim SE na null dereferenci pár sekund po hlavní nabídce,
deterministicky, s nula nainstalovanými módy; ostatní tři cache na straně jádra byly
vybisektovány jednotlivě a záleželo jen na tomto. Ta bisekce běžela se zapnutým
`FUSE_PASSTHROUGH` a jádro nechá otevření přes passthrough, které nese
`FOPEN_KEEP_CACHE`, selhat s EIO, takže se zapnutým příznakem selhalo každé otevření
přes passthrough. Možná právě tato neúspěšná otevření shazovala hru, a samotný
příznak s vypnutým passthroughem nebyl nikdy otestován. Odpovědi přes passthrough jej
už nenesou. Jeho ztráta byla tehdy naměřena jako zadarmo, jenže se zapnutým
passthroughem démon obsluhuje *nula* čtení (`EIDOS_FUSE_STATS` hlásil `read 0` za
celé načtení), takže s passthroughem ve výchozím stavu vypnutým je skutečná cena
nezměřená. Znovu jej zapnete přes `EIDOS_FUSE_KEEP_CACHE=1`, chcete-li jej otestovat
samostatně.

### FUSE passthrough brání hře načíst jakýkoli obsah módů

Démon se teď vyhýbá nejpravděpodobnější příčině (níže), ale ve hře to zatím nebylo
znovu změřeno, takže passthrough zůstává ve výchozím stavu vypnutý;
`EIDOS_FUSE_PASSTHROUGH=1` jej zapne. Před touto změnou Skyrim SE se zapnutým
passthroughem neotevřel 152 vlastních souborů se `STATUS_ACCESS_VIOLATION`, takže se
tiše nenačetl žádný obsah módů. Měření a to, proč vlastní logy démona ukazovaly čistý
běh, jsou výše v části „Proč je passthrough ve výchozím stavu vypnutý".

Jádro váže na každý inode jediný podkladový soubor (`fs/fuse/iomode.c`). Otevření,
které pro inode, jenž už jeden má, přinese druhé ID podkladového souboru, selže s
EIO, stejně jako normální otevření (bez `FOPEN_PASSTHROUGH`) inodu, který podkladový
soubor má. Otevření přes passthrough, které nese `FOPEN_KEEP_CACHE`, selže s EIO
také. Démon dřív registroval při každém otevření nové ID, takže selhalo druhé
otevření souboru, který hra už držela otevřený. Wine hlásí EIO jako
`STATUS_ACCESS_VIOLATION`, což je chyba za 152 neúspěšnými otevřeními. Toto pravidlo
pochází ze zdrojového kódu jádra a odpovídá těm 152 selháním, a proto je
nejpravděpodobnější příčinou.

Démon teď registruje jediný podkladový soubor na inode, sdílený všemi jeho otevřeními
přes passthrough, a otevření přes passthrough nikdy nenastaví `FOPEN_KEEP_CACHE`.
Passthrough se týká jen otevření pouze pro čtení u souborů z vrstev hry a módů, které
mají 1 MiB nebo víc nebo končí na `.dll`, `.exe` či `.asi`. Jakmile má soubor
podkladový soubor, používá jej každé otevření pouze pro čtení, a dokud má soubor
normální otevření, dostanou normální otevření i noví čtenáři. Soubor v Overwrite přes
passthrough nikdy nejde a připojení, které promítá časová razítka pluginů, jej
nepoužívá vůbec. Zapisovatel, který otevře soubor, zatímco ho drží čtenáři přes
passthrough, dostane ETXTBSY (ve Wine: `STATUS_SHARING_VIOLATION`) dřív, než proběhne
jakýkoli copy-up, takže otevření s `O_TRUNC` nemůže v Overwrite nechat prázdnou kopii,
která by skryla soubor módu.

### Tlačítko Mod Manager Download nic nedělá

Preferences, Nexus, Downloads ukazuje, který program přijímá odkazy `nxm://`. Karta
Health se objeví, když tím programem není Eidos, když chybí nebo když není nastaven
žádný. Tlačítko Register zapíše `eidos-nxm.desktop` a nastaví ho jako výchozí obsluhu
`x-scheme-handler/nxm`. Z terminálu totéž udělá `eidos nxm --register`.

Pokud stav už uvádí Eidos a tlačítko přesto nic nedělá, prohlížeč možná používá
dřívější volbu. V nastavení prohlížeče smažte uloženou volbu pro odkazy nxm, aby se
zeptal znovu. Prohlížeč z Flatpaku nemůže spouštět programy hostitele sám a předá
odkaz přes desktopový portál, který čte obsluhu registrovanou na hostiteli.
