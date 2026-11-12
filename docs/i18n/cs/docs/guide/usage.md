<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Používání Eidosu

Jak používat Eidos: instance a kopie hry, příkazová řádka, testovací běhy, okno,
kolekce z Nexusu a aktualizace, přesun instance na jiný stroj, nástroje a
rozšíření. Problémy řeší [troubleshooting.md](troubleshooting.md); sestavení ze
zdrojů a rozložení repozitáře popisuje
[../internals/contributing.md](../../../../internals/contributing.md).

## Instance: globální a přenosné

Příkazy, které berou instanci, přijímají identifikátor hry nebo složku přenosné
instance. `skyrimse` jmenuje globální instanci, kterou Eidos drží v
`~/.local/share/Colony/Eidos/instances/skyrimse`. Přenosná instance je samostatná
složka kdekoli chcete (druhý disk, oddíl pro hry) a můžete ji přesouvat, stejně
jako přenosné instance MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # vytvořit tam přenosnou instanci
eidos install /mnt/games/EidosSkyrim mod.7z  # každý příkaz přijímá složku
eidos play /mnt/games/EidosSkyrim -- %command%
```

Soubor `eidos-instance.ini` ve složce jmenuje hru, takže složka nepotřebuje nic
dalšího. `EIDOS_INSTANCE=<folder>` v prostředí přesměruje identifikátor hry na tu
složku, například v parametru spuštění Steamu. Přenosné instance, které jste
vytvořili nebo otevřeli, si Eidos pamatuje, nejnověji použité první, v
`~/.config/Colony/Eidos/instances.ini`: uvítací obrazovka okna je vypisuje, spuštění
ze Steamu skončí na té, kterou jste hráli naposledy, a obsluha `nxm://` stahuje do ní.

Ruční přesun přenosné složky zachová všechno kromě záznamů nástrojů, které jste
zaregistrovali s absolutními cestami do starého umístění. `eidos pack` a
`eidos unpack` ([níže](#přesun-instance-na-jiný-stroj)) je opraví; prosté
`mv` ne. Cache runtimů v `~/.local/share/Colony/Eidos/runtimes/` sdílí každá
instance na stroji, takže se 78 MB hostitel .NET nekopíruje do každé z nich.

Jedno místo je odmítnuto: uvnitř instalační složky hry, kam uživatelé MO2 často
dávají svou instanci. Ten strom vlastní Steam a aktualizace, „ověření integrity" nebo
odinstalace ho mohou přepsat nebo smazat i s vaším nastavením. Eidos také připojuje
přes kořen hry, takže instance uvnitř by ležela uvnitř vlastního cíle připojení.
Průvodce, `eidos init`, `eidos play` a `eidos shell` to odmítnou. Dejte složku vedle
hry; sourozenecká složka na stejném disku je stejně pohodlná.

### Kde Eidos drží své soubory

Eidos drží své soubory pod `Colony/Eidos`, v rozložení, které používá každý program
rodiny Colony: `~/.config/Colony/Eidos/` pro to, co jste zvolili (předvolby, vaše
relace na Nexusu, seznam instancí, definice her a doplňků, které jste napsali),
`~/.local/share/Colony/Eidos/` pro globální instance (`instances/`), stažené runtimy a
logy relací (`logs/`) a `~/.cache/Colony/Eidos/` pro masterlisty LOOTu, které se
stáhnou znovu, kdykoli chybí.

Když najde soubory v rozložení staršího Eidosu (`~/.config/eidos/`,
`~/.local/share/eidos/` a `~/.local/state/`), první spuštění je zkopíruje a řekne to v
logu. Globální instance se přesouvá pevnými odkazy místo kopie bajtů, takže její módy
nezabírají další místo, a její záznamy nástrojů se přesměrují na novou složku. Nic ve
starých adresářích se nemění ani nemaže; odstraňte je sami, až nové rozložení
funguje. Když kopii nelze vytvořit (plný nebo jen pro čtení disk, stará složka
instance na jiném disku, hra, která z ní ještě běží), Eidos dál používá staré umístění
a zkusí to znovu při příštím spuštění. Parametr spuštění Steamu, který pořád jmenuje
starou složku instance, otevře přesunutou.

Pevné odkazy šetří místo jen zpočátku. Mód nebo stažený soubor, který potom odeberete
nebo přeinstalujete, si přes starou složku instance ponechá svou starou kopii na disku
a smazáním `~/.local/share/eidos/<game>/` se to místo uvolní.

Přenosná instance se nikdy nepřesouvá: zůstává tam, kam jste ji dali.

## Příkazová řádka

```sh
eidos games                                # podporované hry nainstalované tady, jako seznam MO2
eidos init skyrimse [folder]               # vytvořit instanci; se složkou je přenosná
eidos init skyrimse --game-path <folder>   # použít kopii nainstalovanou v <folder>, když je nainstalováno víc
eidos install skyrimse mod.7z [name]       # nainstalovat archiv (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # přeinstalovat přes mód (--merge instaluje přes jeho soubory)
eidos import skyrimse <mo2-profile>        # převzít pořadí módů a stav pluginů profilu MO2
eidos sort skyrimse [--dry-run]            # seřadit pořadí načítání LOOTem (--update-masterlist ho nejdřív obnoví)
eidos play skyrimse                        # ukázat, co by se připojilo
eidos play skyrimse -- <command>           # spustit <command> s módy připojenými přes hru
eidos shell skyrimse                       # shell ve sloučeném pohledu; zápisy skončí v Overwrite
eidos check skyrimse [--json]              # kontroly Health bez okna
eidos sessions skyrimse                    # zaznamenané běhy her a nástrojů, nejnovější první
eidos sessions skyrimse diff [<id>]        # co se změnilo od posledního běhu s kódem ukončení 0
eidos stock-game skyrimse                  # které soubory hry instance spouští; zmrazit kopii nebo přepnout
eidos tool skyrimse                        # vypsat nástroje (add, rm a run: viz Nástroje níže)
eidos tool skyrimse run --exe <path>       # spustit exe jednou přes sloučený pohled
eidos tool skyrimse import-mo2 [--write]   # spustitelné soubory z ModOrganizer.ini; --write je přidá
eidos prereqs skyrimse [--install]         # co nástroje potřebují; --install stáhne, co chybí
eidos collection skyrimse <link>           # nainstalovat kolekci z Nexusu (--dry-run, --no-optional)
eidos nexus status                         # zkontrolovat uložené přihlášení k Nexusu
eidos nexus update skyrimse [--apply]      # zkontrolovat aktualizace módů; --apply je nainstaluje
eidos nxm <link>                           # stáhnout odkaz Mod Manager Download
eidos nxm --register                       # nechat tlačítko Mod Manager Download na webu otevřít Eidos
eidos export skyrimse -o mods.csv --active # formát CSV z MO2; --active vypíše jen zapnuté módy
eidos pack skyrimse backup.eidos           # celá instance v jednom souboru
eidos unpack backup.eidos [folder]         # bez složky se globální instance vrátí na své centrální místo
eidos --version                            # vypsat verzi
```

`eidos` bez argumentů vypíše seznam příkazů. Většina příkazů vypíše své vlastní
volby, když chybí argument (`eidos init` a `eidos import` vypíšou místo toho celý
seznam), a `eidos tool --help` vypíše podpříkazy nástrojů. `eidos pack` bere
`--dry-run`, `--level`, `--no-downloads`, `--no-recovery` a `--force` a `eidos unpack`
bere `--info` a `--force`; obojí je popsáno
[níže](#přesun-instance-na-jiný-stroj). `eidos stock-game` je popsán v části
[Soubory hry](#soubory-hry-instalace-z-obchodu-nebo-kopie).

### Spuštění hry přes sloučený pohled

`eidos play` připojí módy instance přes vlastní adresář `Data` hry uvnitř soukromého
jmenného prostoru a pak spustí příkaz skrz tento pohled. Vlastní soubory hry se
nejdřív odloží přes bind, takže je démon stále čte beze změny. Zápisy (uložené pozice,
znovu vygenerované konfigurace) přistávají ve vrstvě `overwrite/` instance; instalace
hry i každá složka módu zůstanou bajt po bajtu takové, jaké byly.

Eidos nepotřebuje root. Připojuje v soukromém jmenném prostoru uživatele a připojení,
bez pomocníka setuid, bez systémové služby a bez čehokoli k udělování. Volitelné
oprávnění na binárce `eidos` umožňuje FUSE passthrough, který zůstává vypnutý, dokud
se nenastaví `EIDOS_FUSE_PASSTHROUGH=1`; viz
[install.md](install.md#volitelně-fuse-passthrough). S oprávněním Eidos používá
obyčejný jmenný prostor připojení místo uživatelského a módy se nasazují v obou
případech stejně.

### Shell ve sloučeném pohledu

`eidos shell <instance>` spustí váš `$SHELL` (nebo `/bin/sh`) ve složce hry tak, jak
by ji viděla hra: každý zapnutý mód připojený přes `Data`, módy Root přes kořen hry a
vše, co se tam zapíše, skončí v Overwrite. `eidos shell <instance> -- <command>`
místo toho spustí jeden příkaz, například `eidos shell skyrimse -- ls Data/meshes`
nebo nativní nástroj jako NifSkope. Běh drží zámek instance jako herní relace, takže
okno nemůže ukládat změny, dokud neskončíte, a v `eidos sessions` se zaznamená pod
názvem `shell`, nikdy jako spuštění hry. Ctrl+C na příkazu zastaví běh tak, jak to
dělá Stop: Eidos příkaz ukončí, zachytí relaci a odpojí. V interaktivním shellu patří
Ctrl+C shellu, jak je obvyklé; běh ukončíte příkazem `exit`.

### Kontroly Health z terminálu

`eidos check <instance>` spustí kontroly Health, které nepotřebují okno, takže mohou
běžet přes SSH nebo v Game Mode a jít do hlášení chyby:

- prefix Proton a jeho registr;
- diagnostika pluginů a chybějící mastery;
- script extender a jeho pluginy;
- aktualizace hry od posledního čistého spuštění;
- parametr spuštění Steamu;
- nastavení grafiky (NVAPI, celá obrazovka, ENB);
- log pádu z posledního běhu;
- řádky `sTestFile` v INI profilu;
- pluginy `.esp` a blueprint ve Starfieldu;
- `vm.max_map_count` a limit otevřených souborů;
- kopie hry, kterou instance spouští, a `gamePath` v `ModOrganizer.ini` seznamu, který
  jmenuje jinou složku hry;
- pravidla pořadí kolekce, která už neplatí;
- soubory hry na NTFS, exFAT nebo FAT;
- datové cesty BodySlidu;
- složky `Data` zbylé v `Overwrite/Root`;
- knihovny fontů, které jmenuje `Interface/fontconfig.txt` a žádný mód je nedodává;
- komprimovaný zvuk v archivech BA2 módů pro Fallout 4 a Fallout 4 VR.

Některé karty počítá jen okno, v záložce Diagnostics: pořadí patchů, skripty
extenderu, které mód nahrazuje, archivy, které nic nenačítá, požadavky z Nexusu,
pluginy, které LOOT označuje jako špinavé, sestavení Protonu, které spouští hru,
karta Display a výstup nástroje starší než jeho vstupy.

Příkaz vypíše stejné systémové řádky jako **Copy report** a pak jeden řádek na
zjištění, s vaší domovskou složkou zobrazenou jako `~` a přihlašovacím jménem jako
`<user>`. Kód ukončení je 0, když se nenašel žádný problém (samotná rada dává také 0),
1, když se našel aspoň jeden problém, a 2, když Eidos nemohl kontrolovat: špatné
argumenty, instance, kterou nelze otevřít, nebo hra, kterou nedetekuje.

`eidos check <instance> --json` vypíše místo toho jeden objekt JSON. Jeho pole nejvyšší
úrovně jsou `eidos` (verze), `game`, `instance`, `system` (`proton`, `flatpak_steam`,
`passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`,
`max_map_count`, `open_files_hard`) a `findings`. `max_map_count` a `open_files_hard`
jsou čísla, nebo `null`, když je Eidos nemohl přečíst. Každé zjištění je `code`, `level`
(`problem` nebo `advice`) a jeho `args`; záložka Diagnostics ukazuje větu pro každý kód.

### Co se změnilo od posledního dobrého běhu

Každý běh přes `eidos play`, `eidos shell` nebo `eidos tool run`, spuštěný z okna,
terminálu nebo Steamu, zapíše záznam do `<instance>/logs/sessions/`: profil, každý
zapnutý mód s jeho verzí a id souborů z Nexusu, pořadí pluginů, kontrolní součet
každého INI profilu, verzi spustitelného souboru hry, verzi Eidosu, kód ukončení a
uložené pozice, které běh vytvořil. Uchovává se 50 nejnovějších a `logs/` zůstává mimo
zabalenou instanci. `eidos sessions <instance> diff` porovná nejnovější běh hry, který
skončil kódem 0, se současným stavem; `diff <id>` vybere jiný záznam ze seznamu.
Seznam označí [testovací běh](#testovací-běhy) jako `test` a testovací běh se nikdy nevybere
jako poslední dobrý běh. Běh, který jste zastavili, zaznamená kód ukončení 143, takže
se také nikdy nevybere. Eidos tu pády zatím nedetekuje a hra, která spadla pod
Protonem, může stále skončit kódem 0: pokud byl takový běh ten, který vybere, pojmenujte
dřívější přes `diff <id>`.

## Seznam módů

Stejně jako v MO2 má každý profil vlastní `modlist.txt` (které módy jsou zapnuté a v
jaké prioritě), zatímco složky módů leží ve sdíleném `mods/` instance. Složka, kterou
vložíte do `mods/` ručně, zatím není uvedena: objeví se vypnutá s nejvyšší prioritou,
na konci seznamu v okně jako v MO2, takže po zapnutí vyhrává své konflikty.

Protože jsou složky sdílené, odebrání nebo přejmenování módu mění každý profil: jeho
řádek se odstraní z `modlist.txt` každého profilu, nebo se v něm přejmenuje, a každý
profil si ponechá vlastní stav a pořadí pro ostatní. Profil bez módů nemá `modlist.txt`
vůbec.

Okno pracuje s vlastní kopií seznamu módů a pořadí načítání. Pokud jiný proces Eidosu
zapíše jedno nebo druhé, zatímco je otevřené (`eidos install` nebo `eidos sort` v
terminálu, kolekce, relace spuštěná Steamem, druhé okno), další úprava okna se
odmítne místo přepsání té práce. Okno se znovu načte z disku a řekne *Not saved:
another Eidos process changed the mod list. It has been reloaded - redo the change.*
(nebo totéž pro pořadí načítání); ztratí se jen to jedno kliknutí, takže ho udělejte
znovu. Rename, New mod, Add separator, Remove a přeměna Overwrite na mód kontrolují
před sáhnutím na `mods/`, takže odmítnutá úprava na disku nic nezmění. Editor INI
stejně odmítne uložit přes soubor, který se od otevření změnil.

## Uložené pozice patří profilu

Každý profil drží své uložené pozice v `profiles/<name>/saves/` a po dobu běhu je ta
složka připojena přes složku uložených pozic hry v prefixu Proton. První spuštění
profilu zkopíruje pozice, které už v prefixu jsou, do něj, jednou, takže pozice, kterou
později z profilu smažete, zůstane smazaná. Po každém běhu se nejnovější pozice
profilu zkopírují zpět do prefixu pro Steam Cloud; pozice v prefixu, kterou tam Eidos
nedal, se nejdřív uchová v profilu jako `orphan-<time>-<name>`, nikdy se nepřepíše.

Pozice, která se do prefixu dostane po tom prvním kopírování mimo Eidos (stažení ze
Steam Cloudu ze Steam Decku, spuštění bez Eidosu), zůstane skrytá, dokud jsou pozice
profilu připojené. Nic ji nemaže: spuštění ji jmenuje v logu běhu (`logs/run-*.log`
instance při spuštění z okna, konzole Steamu při spuštění přes Steam), jednou pokaždé,
když se ta sada souborů změní, a záložka Saves tyto pozice počítá na řádku pod svým
nadpisem. Chcete-li jednu hrát, stiskněte Import v záložce Saves. Import zkopíruje
každou i s jejím co-save do profilu a nikdy nenahradí pozici profilu: pozice, jejíž
název už profil používá, se naimportuje jako `orphan-<time>-<name>`.

## Testovací běhy

Tlačítko Test run vedle Run spustí hru tak jako Run, ale vše, co hra zapíše, jde do
`<instance>/test-run/` místo do vašeho profilu, Overwrite a prefixu: nové a změněné
uložené pozice, změny INI, `plugins.txt`, soubory zapsané do Data nebo složky hry a
logy v `Documents/My Games/<game>`. Můžete testovat s módy profilu, bez jakéhokoli
módu, nebo jen s vybranými módy, které jsou zapnuté, plus módy, jejichž pluginy
potřebují jako mastery.

Když hra skončí, Eidos vypíše tyto soubory v pěti skupinách (uložené pozice, změny
INI, stav pluginů, výstup do Overwrite, logy) a zeptá se, zda každou ponechat, nebo
zahodit. Apply čeká, dokud nemá odpověď každá skupina, která obsahuje soubory. Stav
pluginů se nenabízí, když běh použil jinou sadu módů než profil. Ponechání pozic nikdy
nesmaže pozici z profilu, ani takovou, kterou hra během běhu smazala. Ponechání výstupu
do Overwrite také odstraní z Overwrite soubory, které hra smazala, a seznam je nejdřív
spočítá. Zahození čehokoli chce druhé stisknutí. Decide later zavře seznam a nechá
soubory v `test-run/` a seznam se otevře znovu při příštím otevření instance nebo
stisku F5. Když Eidos nebo počítač během testovacího běhu přestane, soubory tam také
zůstanou a seznam se otevře stejně.

Když okno nespustil obchod hry, Test run předá hru tomu obchodu (Steam, Heroic nebo
Legendary), jako Run, a běh, který obchod spustí, je testovací běh. Pokud obchod hru
nespustí, požadavek čeká: další běh hry toho profilu, z Run nebo z obchodu, je
testovací běh, a Cancel the waiting test run ho zahodí. Testovací běh pokrývá jen výše
uvedené složky: ostatní zápisy hry, například registr Wine, jdou tam, kam vždy.
Uložené pozice z testovacího běhu se dostanou do Steam Cloudu, až když je ponecháte a
zahrajete normální běh.

### Hledání módu za problémem

Find the mod behind a problem v záložce Diagnostics hledá mód, který způsobuje problém,
který umíte zopakovat, například pád na daném místě. Každý krok je testovací běh s
polovinou zbývajících podezřelých, jejich mastery a módy, které ponecháte stále
zapnuté: zapnuté módy vybrané v seznamu při zahájení a mód, který dodává script
extender. Po každém běhu odpovíte, zda se problém objevil. Not sure rozdělí podezřelé
jinak a Run it again krok zopakuje. Odpověď nebo Run it again zahodí soubory, které krok
zapsal, po druhém stisknutí. Chcete-li některé ponechat, stiskněte před odpovědí See the files:
otevře stejný seznam jako po testovacím běhu. Když hra spadla a záznamník pádů zapsal
log, nebo hra skončila s chybovým kódem, Eidos označí Yes jako pravděpodobnou
odpověď. 200 podezřelých zabere asi 8 běhů.

Hledání se ukládá do `<instance>/bisect.json`, takže můžete mezi kroky zavřít Eidos, a
patří profilu, ve kterém začalo. Stop the search ho zahodí po druhém stisknutí.
Výsledek jmenuje mód, nebo sadu módů, které potřebují mastery jedna druhé, a ukládá se
do `<instance>/logs/bisect-<time>.txt`. Generované módy (výstup Nemesis nebo DynDOLOD,
Bashed Patch) jsou postavené pro celý seznam a s polovinou mohou selhat: ponechte je
stále zapnuté, nebo je po celé hledání nechte vypnuté.

## Seznamy Wabbajack

Eidos nemá instalátor Wabbajack. Nainstalujte seznam přes CLF3 nebo Jackify, mimo
Eidos, a pak v průvodci vyberte instalační složku jako přenosnou instanci. Eidos čte
složku tak, jak ji rozloží MO2: `mods/`, `profiles/<name>/modlist.txt` a seznamy
pluginů. Ponechá si `ModOrganizer.ini`, který zapsal instalátor, a nikdy ho nepřepíše,
takže spustitelné soubory samotného seznamu zůstanou zaregistrované.

Mnoho seznamů přidává složku `Stock Game` nebo `Game Root`, která drží záplatovanou
kopii hry, a MO2 spouští hru odtamtud. Když průvodce nebo `eidos init` poprvé převezme
složku seznamu a ta složka obsahuje spustitelný soubor hry, instance spouští hru také
z ní. Hra se stále spouští přes Steam nebo obchod, který ji vlastní, takže se nemusí nic
přesouvat. Eidos najde složku podle `gamePath` v `ModOrganizer.ini` seznamu, nebo podle
těch dvou názvů. Instance převzatá dřívějším Eidosem si ponechá instalaci z obchodu,
dokud ji nepřepnete (viz další část). Když tento `gamePath` jmenuje složku hry, kterou
instance nespouští, Diagnostics to řekne, protože nástroje v režimu Mod Organizer 2,
například PGPatcher, čtou hru odtud. CLF3 přestavuje soubory BSA a DDS vlastními
nástroji, takže se výsledek může lišit od instalace Wabbajackem a autoři seznamů ho
nepodporují.

## Soubory hry: instalace z obchodu, nebo kopie

Ve výchozím stavu hra běží z instalace z obchodu, ze složky, ve které ji drží Steam,
Heroic nebo Legendary. Instance může místo toho běžet z kopie té složky uvnitř
instance. Módy leží na kopii a instalace z obchodu je jen místo, kde Eidos připojuje
výsledek. Hru stále spouští obchod, takže Steam si ponechá overlay, shader cache a
cloudové uložené pozice.

Karta Game files nahoře v záložce Diagnostics ukazuje, které soubory instance spouští,
s verzí hry kopie a instalace z obchodu, a mezi nimi přepíná. V terminálu
`eidos stock-game <instance>` vypíše tytéž údaje a `use <folder>`, `store`, `freeze` a
`delete --yes` dělají to, co tlačítka karty. `freeze` vypíše velikost a druh kopie a
`freeze --yes` kopíruje.

**Freeze this version** zkopíruje instalaci z obchodu do `<instance>/Stock Game` a
spouští hru z ní, takže aktualizace obchodu už nerozbije pluginy script extenderu:
aktualizace změní instalaci z obchodu, ne kopii. První stisknutí ukáže velikost a co s
ní souborový systém udělá a druhé zkopíruje. Na souborovém systému, který umí sdílet
bloky (reflink), například btrfs nebo XFS, kopie zabere skoro žádné místo, dokud obchod
nezmění soubor. Na ext4, nebo když hra a instance leží na různých souborových
systémech, je to plná kopie velká jako složka hry. Eidos odmítne, dokud Steam hlásí hru
jako zaneprázdněnou nebo ne zcela nainstalovanou, dokud běží hra nebo nástroj instance,
když instance už drží kopii hry ze seznamu, zmrazenou kopii nebo nedokončenou, a když
má souborový systém méně volného místa, než plná kopie potřebuje. Kopie z Heroicu a
Legendary se nekontrolují na probíhající aktualizaci. Stop, selhání nebo aktualizace
hry ze Steamu během kopírování nechá `Stock Game.eidos-tmp`, které karta nabídne
smazat; nikdy se nepoužije.

Kopie skryje každý soubor, který obchod po pořízení kopie ve složce hry přidá, změní
nebo odebere: nové soubory Creation Clubu, nový spustitelný soubor, nové DLL. Hra, seznam
pluginů, LOOT i kontroly Health čtou kopii. Neskryje to, co se děje mimo složku hry:
aktualizace může při dalším spuštění stále nainstalovat runtimy do prefixu Proton a
Steam může hru přepnout na jiný Proton.

**Return to the store version** vyřadí kopii z dalšího spuštění po druhém stisknutí.
Kopie zůstane na disku a **Delete the copy** ji odstraní, opět po druhém stisknutí.
Delete se odmítne, dokud instance kopii stále spouští. Eidos nikdy nemaže vlastní složku
`Stock Game` nebo `Game Root` seznamu. Když složka kopie zmizí nebo už neobsahuje
spustitelný soubor hry, spuštění se odmítnou a Diagnostics jmenuje složku, takže
verze z obchodu nikdy nepoběží z překvapení.

Morrowind a Oblivion s `bUseMyGamesDirectory=0` drží seznam pluginů ve složce hry a
Eidos je z kopie zatím nespouští.

## Instalace kolekce z Nexusu

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # přečíst ji a říct, co by se stalo
eidos collection skyrimse rqhcxy --no-optional  # přeskočit členy, které označuje jako volitelné
```

V okně vložte odkaz do **File -> Open a Nexus collection...**, nebo klikněte na kolekci
na webu. Eidos vypíše členy revize, každý porovnaný s touto instancí: nainstalovaný,
stažený nebo chybějící. *Install this collection* spustí celý recept ve vlákně workeru,
s ukazatelem průběhu, který používají úlohy pack a unpack, a po dokončení se ukazatel
změní na zprávu. Okno a `eidos collection` spouštějí stejnou instalaci.

Bez odkazu se **File -> Open a Nexus collection...** otevře na záložce **Browse**.
Vypisuje kolekce publikované pro hru otevřené instance, po 20, seřazené podle nejvíc
stažených, nedávno aktualizovaných nebo nejnovějších, a vyhledávací pole zúží seznam na
kolekce, jejichž název nebo text odpovídá. Každý řádek uvádí autora, nejnovější revizi,
počet módů, stažení a datum. **View** otevře nejnovější revizi té kolekce na záložce
**Link**, stejně jako vložení jejího odkazu. Kolekce označené jako pro dospělé se
vynechají, pokud váš účet na Nexusu neukazuje obsah pro dospělé, kolekce bez hodnocení se
vynechají vždy a řádek pod seznamem počítá obojí. Procházení potřebuje přihlášený účet na
Nexusu, jako zbytek panelu.

Kolekce se otevře jen proti instanci vlastní hry. Kolekce pro Skyrim otevřená při
načtené instanci Falloutu 4 se odmítne jménem, místo aby se její členové porovnávali se
špatným seznamem módů.

### Co Eidos z kolekce čte

Téměř nic z kolekce není vidět přes API Nexusu. Pořadí instalace, odpovědi, které autor
dal skriptovanému instalátoru každého módu, který mód vyhrává konflikt souboru, které
pluginy se načítají kam a binární patche jsou všechny v `collection.json` uvnitř
vlastního archivu kolekce. Stažení týchž módů a jejich instalace s výchozími volbami dá
jinou hru.

Eidos stáhne archiv, přečte `collection.json` a použije ho:

| Kolekce říká | Eidos dělá |
| --- | --- |
| `phase` | instaluje v tomto pořadí, volitelné členy nakonec |
| `choices` | přehraje autorovy odpovědi na každý FOMOD a řekne to, když nemůže |
| `modRules` | přeřadí seznam módů tak, aby správný mód vyhrál každý soubor |
| `plugins` / `pluginRules` | sloučí pravidla LOOTu do vašeho userlistu a seřadí |
| `INI Tweaks/` | nainstaluje je jako samostatný mód |
| `tools` | řekne, které nástroje očekává; nic nevytváří |

Členové se zapínají při instalaci, na konci seznamu módů v pořadí instalace, než je
`modRules` přesune. Člen, který by v seznamu módů chyběl, by se nenačetl a zpráva by ho
neukázala.

Stav se po každém členu zapisuje do `<instance>/collections/<slug>-<revision>.state.json`.
Leží vedle složky té revize, nikdy v ní, takže kolekce nemůže dodat vlastní soubor se
stavem, a instalace přerušená u člena 147 z 200 pokračuje od 147. Zapisuje se pod
dočasným názvem a přejmenuje na místo, takže přerušení nechá starý záznam nebo nový a
nikdy půlku z obou. Kdyby se nedal přečíst, Eidos se zastaví a řekne to, místo aby celou
kolekci začal znovu. Spusťte stejný příkaz znovu.

Instalace novější revize kolekce, kterou už máte, převezme složky předchozí revize:
nezměněný člen se ověří na místě, člen, jehož soubor autor aktualizoval, se nahradí
uvnitř své staré složky a člen, kterého nová revize vypustila, zůstane nainstalovaný a
zpráva ho jmenuje, abyste ho vypnuli nebo odebrali. Složka, kterou zapíná jiný profil, se
nikdy nepřevezme, protože složka módů je sdílená a ten profil by se vám pod rukama
změnil: nová revize místo toho nainstaluje vlastní kopii vedle ní a zpráva vás požádá,
abyste starou v tomto profilu vypnuli, nikdy abyste ji odebrali. Návrat ke starší revizi
převezme její složky stejným způsobem, nebo nainstaluje čerstvé kopie těch, které
převzít nemůže.
Kdykoli se člen takto nahradí a vy jste v jeho složce změnili nebo přidali soubory, stará
složka se nejdřív uchová vedle ní jako `<name>_backup`, která se do hry nikdy nenačte, a
zpráva ji jmenuje.

Pozůstatek z dřívější revize, který zůstane zapnutý, brání tomu, aby se instalace
počítala za tu, kterou autor postavil; poznámka zmizí, až ho vypnete. Člen, kterého jste
přeskočili přes `--no-optional`, se nepřebírá a jmenuje se stejně. Když selže samotné
převzetí, běh se zastaví s *previous revision: its folders could not be taken over* a
důvodem; opravte to a spusťte příkaz znovu.

### Bezplatné účty a členové, které stahujete sami

Bezplatný účet na Nexusu nemůže stáhnout členské módy. Nexus bezplatným účtům nedává přes
API žádné odkazy ke stažení: každý soubor potřebuje vlastní tlačítko „Mod Manager
Download" na webu. Archiv kolekce se na bezplatném účtu stáhne, takže můžete přečíst
celý recept, ale každý člen se hlásí s přesnou adresou své stránky, abyste ho stáhli sami,
a opakování to nezmění. Stáhněte je přes tlačítko webu a spusťte příkaz znovu: tito
členové se hledají v `downloads/` při každém běhu, takže kolekce skončí, jak je dodáváte.

V okně dostane bezplatný účet v panelu kolekce místo dávkového stažení
*Fetch the N missing required files, one by one*. Otevře na Nexusu stránku prvního
chybějícího povinného souboru, počká, až soubor přistane v `downloads/`, a pak otevře
další. Tlačítko stažení na stránce pro každý soubor stále mačkáte vy. Panel ukazuje, jak
daleko je (*3 of 12*), s *Skip* a *Stop*. Když místo toho přistane jiný soubor téhož
módu, panel jmenuje obě id souborů a dál čeká na připnutý. Volitelní členové do toho
nepatří a mají své vlastní tlačítko *Open*.

Eidos také používá binární patche (každý ověřený proti zaznamenanému CRC), přepisy
souborů (jmenované soubory se skryjí přes `.mohidden`, jako to dělá MO2) a členy, jejichž
soubory cestují uvnitř archivu kolekce (`bundle`). Členové, které kolekce očekává, že
stáhnete ručně (`browse`, `manual`), přenesou autorovy vlastní pokyny do zprávy. Člen,
jehož název už je mód u vás, se nainstaluje vedle něj pod volným názvem, nikdy přes něj, a
zpráva řekne, který.

### Zpráva

Člen, jehož odpovědi instalátoru nešlo všechny přehrát, dostane vlastní řádek,
*installed, but not the way the collection asks*, odděleně od členů, kteří se nainstalovali
tak, jak je autor postavil, protože jeho soubory na disku se od autorových liší. Zpráva
také vypisuje pravidla řazení, která nejmenovala nic v kolekci, módy zachycené ve smyčce
protichůdných pravidel, skupiny LOOTu, které neexistují, a každou část manifestu, které
tato verze Eidosu nerozumí.

### Zachování pořadí kolekce

Eidos si po instalaci ponechává pravidla každé nainstalované kolekce. Když později
přesunete dva její módy proti pravidlu before nebo after, záložka Diagnostics ukáže pro
tu kolekci jednu kartu, která jmenuje první takový pár a počítá pravidla, která už
neplatí. Počítají se jen módy zapnuté v současném profilu a pravidla zachycená ve smyčce
se přeskakují, protože je žádné pořadí nesplní. **Restore collection order** přesune módy
kolekce zpět do pořadí, které splňuje každé pravidlo, ponechá vaše vlastní pořadí všude,
kde to pravidla dovolí, a módy, které nejsou v kolekci, nechá, kde jsou. Ctrl+Z to vrátí.
`eidos check` uvádí totéž zjištění jako `collection_order_broken`. Kontrola čte recept
revize v `collections/`, takže kolekce, jejíž recept v cache byl odstraněn, se nekontroluje.

## Přesun instance na jiný stroj

`eidos pack` zapíše celou instanci (každý mód, pořadí načítání, všechny profily,
Overwrite s vašimi uloženými pozicemi a archivy, ze kterých se vše instalovalo) do jednoho
souboru. `eidos unpack` ji vrátí:

```sh
eidos pack skyrimse ~/backup.eidos                  # nebo zadejte složku: pojmenuje se podle hry a data
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # na druhém stroji
```

`eidos pack --dry-run` vypíše přesně, co by se vložilo, co ne a proč, a kolik místa je
potřeba, bez zápisu čehokoli. `eidos unpack --info` dělá totéž pro soubor, který už máte.

Pokud 7-Zip něco nedokázal přečíst, nebo si průchod musel přeskočit něco, co byste v záloze
čekali (symbolický odkaz, složku, kterou nešlo přečíst, název, který 7-Zip nelze bezpečně
zadat), archiv se přesto zapíše pod zvoleným názvem, ale `eidos pack` skončí s kódem 1 a
řekne, co chybí, takže skript, který spouští `eidos pack ... && ...`, se tam zastaví.

Soubor `.eidos` je archiv 7-Zip s vlastní příponou. 7-Zip je už potřeba k instalaci módu,
takže to nepřidává žádnou závislost a záloha se otevře i bez Eidosu v každém archivátoru,
který čte 7-Zip. Je komprimovaná LZMA2 s `-mx1`, asi na 43 % originálu u seznamu s mnoha
texturami; `-mx9` trvá mnohem déle a ušetří o pár bodů víc. Textury a meshe jsou už
komprimované formáty, takže větší slovník najde k zkomprimování velmi málo. `--level` (0,
1, 3, 5, 7 nebo 9) vybere jinou úroveň a `--no-downloads` vynechá `downloads/`.

Na výchozí úrovni 1 jdou soubory do solidních bloků nejvýš 16 MB. Vytažení jednoho souboru
ze zálohy o 70 GB dekóduje nejvýš blok, ve kterém leží, nikdy vše před ním, a balení
herního obsahu běží 2 až 5krát rychleji než s každým souborem komprimovaným zvlášť
(rozbalení 2 až 4krát). Cenou je poškození: jeden poškozený bajt může vzít s sebou každý
soubor ve svém bloku, až 16 MB sousedních souborů. Data pro obnovu popsaná níže to
pokrývají. Každá další úroveň zůstává nesolidní: každý soubor je vlastní proud a poškozený
bajt stojí ten jeden soubor. Úroveň 0 soubory jen uloží. Od úrovně 3 výš LZMA2 už pracuje v
blocích 16 MB nebo víc a solidní bloky se tam změřily jako pomalejší: na úrovni 5 trvalo
balení meshů 1,4krát déle a rozbalení textur 6,8krát déle.

Balení vezme zámek instance, takže nemůže běžet, zatímco do té instance zapisuje hra nebo
jiný Eidos, a nic instanci nemění, dokud se čte. Záloha se zapisuje pod názvem `.part`,
vyprázdní se na disk a celá se přečte zpět přes `7z t`, než dostane zvolený název. Ta
kontrola dekóduje každý blok a porovná každý kontrolní součet, tedy stejnou práci jako
rozbalení na druhém stroji. Ukazuje, co disk vrátil hned po zápisu, ne že disk si to
udrží; na to jsou data pro obnovu níže. Když selže, `.part` se smaže, pod vaším názvem se
nezapíše nic a `eidos pack` ocituje řádek, kde 7-Zip poškození našel. Přerušené nebo
selhavší balení nechá soubor `.part` nebo nic, nikdy soubor `.eidos`, který vypadá hotový a
není.

Když je nainstalován par2cmdline (`sudo pacman -S par2cmdline` na Archu,
`sudo apt install par2` na Debianu a Ubuntu, `sudo dnf install par2cmdline` na Fedoře),
`eidos pack` zapíše vedle archivu také data pro obnovu PAR2: `backup.eidos.par2` a několik
souborů `backup.eidos.volNN+MM.par2`, 5 % archivu plus malý index. Držte je ve stejné složce
jako soubor `.eidos`, pod názvy, se kterými byly zapsány, a kopírujte je s ním: záloha
přesunutá bez nich nemá z čeho být opravena a data pro obnovu zaznamenávají název archivu,
takže přejmenovaná záloha jim už neodpovídá. `eidos unpack` je najde podle názvu a archiv
proti nim zkontroluje před extrakcí čehokoli, včetně zálohy, jejíž konec byl uříznut, takže
ji 7-Zip nemůže otevřít. Když se pár bajtů pokazilo, par2 archiv nejdřív opraví a poškozenou
kopii ponechá jako `backup.eidos.1`, což potřebuje volné místo o velikosti archivu v jeho
složce; poškození nad rámec toho, co pokrývá těch 5 %, zastaví rozbalení dřív, než se zapíše
jediný soubor. Soubory pro obnovu, které archivu neodpovídají, například ty ze starší zálohy
se stejným názvem, se nepoužijí, když je 7-Zip archiv přečte a najde ho nepoškozený, a
rozbalení to řekne. Na stroji bez par2 rozbalení stále běží, s varováním, že data pro obnovu
nebyla použita. `--no-recovery` balí bez nich a `eidos pack --dry-run` řekne, zda se
zapíšou. `--force` nahradí existující soubor stejného názvu.

### Co v ní není a proč

| Vynecháno | Proč |
| --- | --- |
| Prefix Proton | Má tvar stroje (mapování jednotek, pohled `Z:` na *tento* souborový systém) a Eidos ho za minutu postaví znovu přes `eidos prereqs <instance> --install`. Nést ho by zhruba zdvojnásobilo archiv kvůli něčemu, co druhý stroj stejně musí vygenerovat znovu. |
| Hra | Instance módy upravuje hru, kterou neobsahuje. Nainstalujte ji tam ze Steamu. |
| Nástroje mimo instanci | xEdit, DynDOLOD a další nástroje jsou binárky třetích stran s vlastními instalátory. V archivu jsou pojmenovány, takže rozbalení řekne přesně, které na novém stroji chybí. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Záznamy stroje, který zálohu vytvořil. Zejména `prereqs.done` by tvrdil, že runtime knihovny jsou už nainstalované v prefixu, který tam není. |
| `loot/`, kromě `userlist.yaml` | Cache masterlistu, kterou Eidos na požádání stáhne znovu. Vaše vlastní pravidla LOOTu v `userlist.yaml` stáhnout znovu nelze, takže ten jeden soubor se vloží. |
| `.base/`, `.base-root/` | Prázdné přípojné body, kam se během relace odkládají vlastní soubory hry. |
| `test-run/`, `bisect.json` | [Testovací běh](#testovací-běhy) čekající na kontrolu a hledání módu za problémem mezi dvěma kroky. Obojí patří k během na tomto stroji. |
| Napůl zapsané soubory | Pozastavené stahování (`*.unfinished`), probíhající atomický zápis (`*.eidos-tmp*`). |
| Symbolické odkazy | 7-Zip by jeden následoval a zkopíroval, na co ukazuje, což u absolutního odkazu znamená vtáhnout cizí strom do vaší zálohy. Místo toho se hlásí a záloha se počítá za neúplnou: `mods/` nebo `downloads/` odkazované na jiný disk se nebalí. Chcete-li zálohovat bez odkazovaného `downloads/` záměrně, zadejte `--no-downloads` (nebo v okně odškrtněte *Include downloads/*); záměrné vynechání se za chybějící nepočítá. |

Kopie hry v instanci (`Stock Game`) se balí jako každá jiná složka, takže záloha zmrazené
instance je o velikost hry větší. Nedokončená kopie (`Stock Game.eidos-tmp`) se vynechá.

Každé z těchto vynechání je uvedeno s důvodem v `eidos-backup.ini` v kořeni archivu, takže
čtení toho souboru ukáže, co bylo vynecháno. Tentýž soubor zaznamenává, kde instance dříve
žila, který profil byl aktivní, jak velká je po rozbalení a které nástroje bude nový stroj
potřebovat.

Záznamy stahování (`downloads/*.meta`) se vkládají s vyprázdněným řádkem `url=`. Ta adresa
je podepsaný odkaz z Nexusu, který přestane fungovat během hodin a nese `user_id` účtu,
který soubor stáhl, a zálohu může někdo předat dál. Id módu, id souboru a verze zůstávají,
takže stahování jde najít znovu.

### V okně

Obojí je v nabídce **File**: *Pack this instance...* a *Unpack a backup...*. Rozbalení je
také na uvítací obrazovce, pod seznamem instancí, takže stroj, který ještě nemá žádnou
instanci, může nějakou rozbalit.

Než se cokoli zapíše, dialog Pack ukáže, kolik souborů, jak velké, kolik z toho je
`downloads/`, zhruba na kolik se archiv vyšplhá a které nástroje jsou jmenovány, ale
nenesou se. Jeho políčko *Write recovery data* je zaškrtnuté, když je par2 nainstalován, a
zašedlé s balíčkem k instalaci, když není. Dialog Unpack přečte manifest zálohy, jakmile
soubor vyberete, takže vám dřív, než zvolíte složku, může říct, jakou hru obsahuje, kdy
vznikla, kde dříve žila a které z jejích nástrojů tady chybí.

Oba běží ve vlákně workeru s ukazatelem průběhu a po dokončení se ukazatel změní na zprávu.

### Co rozbalení opraví

Prostá kopie složky by nechala každé tlačítko nástroje ukazovat na cestu na starém stroji.
`eidos unpack` přepíše hodnoty, které jmenují starý kořen instance, a jen ty: `exe`,
`workdir` a číslované klíče `arg` v `tools.ini` a `installationFile` v `meta.ini` každého
módu. Nástroj, který nikdy nebyl uvnitř instance (`/opt/xedit/SSEEdit.exe`), zůstane přesně
tak, jak byl, a vypíše se jako něco k přeinstalaci; Eidos neodhaduje, kde ten nástroj na
novém stroji je. Opraví také to, zda se instance nazývá centrální nebo přenosná, takže
pozdější příkazy ji hledají tam, kde teď je.

Rozbalení odmítne složku, která není prázdná (zadejte `--force`, pokud chcete přepsat),
disk příliš malý podle údaje manifestu, archiv vytvořený novějším Eidosem, než je váš, a
každý archiv obsahující položku, která by se zapsala *mimo* zvolenou složku. Bez složky se
záloha globální instance vrátí na centrální místo její hry; přenosná složku potřebuje.

Potom:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # znovu postavit prefix Proton
eidos play /mnt/games/EidosSkyrim
```

## Okno

Spusťte `eidos-gui`, nebo soubor `eidos-linux`, který instaluje Colony.

První spuštění otevře průvodce jako v MO2: uvítání, typ instance (přenosná nebo
globální), hra, název a umístění, souhrn, pak hlavní okno. Uvítací obrazovka také
vypisuje každou známou instanci, globální i přenosné, naposledy použité první, k otevření
jedním kliknutím, takže slouží i jako přepínač instancí. Nasměrování průvodce na složku,
která už instanci obsahuje, tuto instanci otevře, jak je, místo vytvoření nové přes ni, a
složka patřící jiné hře se odmítne.

Hlavní okno má dva panely: seznam módů a záložky Data, Plugins, Conflicts, Overwrite,
Archives, Saves, Downloads a Diagnostics, s tlačítkem Run, tlačítkem Test run
([Testovací běhy](#testovací-běhy)) a výběrem cíle spuštění. Seznam módů filtrujete, vybíráte,
přeřazujete, seskupujete oddělovači, zužujete podle kategorie a pravým tlačítkem otevíráte
akce. Kromě odeslání nahoru a dolů má nabídka módu cílené přesuny z MO2: nad první
konfliktní mód, pod poslední, na zadanou prioritu nebo do skupiny oddělovače.

Výběr profilu přepíná profily. Pravým tlačítkem na profil se zobrazí Rename,
Copy to new..., Desktop shortcut, Import from MO2... a smazání, které chce dvě kliknutí a
odmítne aktivní nebo poslední profil. Import from MO2... převezme pořadí módů a pořadí
načítání profilu MO2. Desktop shortcut zapíše položku desktopu, která spustí
`eidos tool <instance> run "<game>" --profile <name>`: udělá z toho profilu aktivní a
požádá obchod, aby hru spustil (podrobnosti v
[tools.md](tools.md#jak-se-k-nástroji-dostat-skrýt-připnout-a-zástupce-na-ploše)).

Nabídka **File** také spouští winecfg, regedit, průzkumník souborů Wine a odinstalátor v
prefixu hry a Prefix drive_c otevře disk `C:` prefixu; viz
[tools.md](tools.md#vlastní-programy-wine-v-prefixu-hry). *Instances...* vypíše
instance, které Eidos zná; Forget jednu z toho seznamu vyřadí a na disku se nedotkne
ničeho. *Export the mod list...* zapíše seznam ve formátu CSV z MO2, jako `eidos export`.

### Předvolby

Klikněte na **Eidos** vlevo nahoře a Preferences se otevřou místo obou panelů; dalším
kliknutím (nebo **Close**) se vrátíte. General, Appearance a Accessibility jsou první, jako
v každém programu Colony, pak Mod list, Nexus a About. Každá změna se použije a uloží
hned; není tu tlačítko Save.

- General: hra, kterou Eidos otevře, velikost okna, jazyk rozhraní, složka s nástroji a
  jak se hra spouští. Lock the window while a game runs je ve výchozím stavu zapnuto. Start
  the game at once when a store opens Eidos a oznámení na plochu, které říká, že dlouhá
  úloha skončila, když je okno na pozadí, jsou ve výchozím stavu vypnuty. Okno mluví
  anglicky a francouzsky; System sleduje `LC_ALL`, `LC_MESSAGES` nebo `LANG` a změna se
  použije hned. Příkazová řádka `eidos` odpovídá vždy anglicky.
- Appearance: motiv (26 rodin, 59 palet, ve výchozím stavu pergamen Eidosu), barva
  zvýraznění, velikost textu, efekty a živý náhled.
- Accessibility: vysoký kontrast, font přívětivý k dyslexii, omezený pohyb a velikost
  textu pro čtení, která se násobí s tou z Appearance.
- Mod list: rychlost automatického posouvání, když se mód táhne k hornímu nebo
  dolnímu okraji seznamu.
- Nexus: přihlášení, složka stahování, obsluha `nxm://` a její tlačítko Register,
  preferované servery, Install downloads when they finish a Offline mode.
- About: kde žijí soubory, klávesové zkratky, verze a kontrola vydání.

Vzhled a jazyk rozhraní žijí v `~/.config/Colony/Eidos/preferences.json`. Ostatní
nastavení, která čte i příkazová řádka (výchozí hra, složka s nástroji, servery Nexusu,
Offline mode a zbytek), žijí vedle v `settings.ini` a relace na Nexusu v `nexus.ini`.
Motiv, který starší Eidos držel v `settings.ini`, se při prvním spuštění převezme.

**About -> Version** drží kontrolu vydání, ve výchozím stavu zapnutou. Při startu, nejvýš
jednou denně, se Eidos zeptá GitHubu na nejnovější vydání `Project-Colony/Eidos`. Když je
novější než běžící kopie, řádek nabídek ukáže za Help `Eidos <version> is available:
release notes` a odkaz otevře stránku toho vydání s changelogem. Eidos nikdy nic nestahuje
ani se nenahrazuje. Kontrola se v režimu offline přeskočí. U kopie nainstalované pod `/usr`
správcem balíčků, který ji aktualizuje, je vypnutá; `/usr/local`, kam `install.sh --system`
Eidos dává, se počítá jako vaše vlastní instalace.

### Sloupce, řazení a seskupování

Seznam ve výchozím stavu ukazuje čtyři sloupce (Category, Content, Version a Flags, ty,
které ukazuje MO2) a nabízí osm: Category, Content, Version, Author, Installed, Nexus id,
Game, Flags. Zaškrtněte je v nabídce View. Se všemi osmi zapnutými zbývá sloupci s názvem
málo místa.

Kliknutím na záhlaví seřadíte podle něj, dalším obrátíte a třetím se vrátíte k pořadí
načítání. Seznam lze přetahovat jen v pořadí načítání: pozice upuštění adresuje skutečný
seznam a při řazení není místo řádku na obrazovce jeho místem v tom seznamu. Při zapnutém
řazení se vkládací pruhy nekreslí a přetažení se odmítne, jako v MO2. Nabídka View to
říká a nabízí cestu zpět.

Nabídka View může také seskupit celý seznam, podle kategorie nebo zdroje (z Nexusu, nebo
nainstalované ručně). Záhlaví skupin nejsou oddělovače: nelze je přejmenovat, obarvit ani
přesunout. Skládají se a složené záhlaví drží svůj počet. Oddělovače se při řazení nebo
seskupení skryjí, protože oddělovač uvozuje řádky, které po něm v pořadí načítání
následují, a obojí toto pořadí na obrazovce mění.

### Myš a klávesnice

Dvojklik na módu otevře Information, Ctrl+dvojklik jeho složku, Shift+dvojklik jeho
stránku na Nexusu. Ctrl+F dá kurzor do pole filtru. Napsání písmene skočí na další mód,
který jím začíná, a opětovné stisknutí projde zbytek, místo aby zůstalo na prvním. Žádné z
nich nepřistane na řádku, který skrývá filtr, složený oddělovač nebo složená skupina, takže
další mezerník nikdy nepřepne mód, který nevidíte.

„Collapse others" v nabídce oddělovače složí každou skupinu kromě té. Při přetahování
podržení ukazatele nad složenou skupinou ji otevře, takže mód lze upustit dovnitř;
přejetí přes ni ne.

Ctrl+Z vrátí poslední změnu seznamu módů nebo pořadí načítání: zaškrtnutí, přesun,
hromadné vypnutí, řazení LOOTem. Ctrl+Shift+Z ji zopakuje. Okno drží 50 kroků a zapomene
je při F5, na konci běhu hry nebo nástroje, při přepnutí profilu a když jiný proces Eidosu
změní seznam. Instalace, přejmenování nebo odebrání módu mění složky na disku, takže vrácení
přes ně se odmítne.

### Filtrování seznamu

Tlačítko Filters otevře jeden řádek na kritérium: aktivní, v konfliktu, dostupná aktualizace,
bez kategorie, schválený (endorsed), sledovaný (tracked), má id z Nexusu, má skryté soubory,
bez platných herních dat, pro jinou hru, má poznámky a jeden řádek na druh obsahu (pluginy,
archivy BSA nebo BA2, textury, meshe, skripty, pluginy script extenderu, soubory rozhraní,
zvuk, FaceGen). Kliknutí přepíná řádek mezi off, only a except. Řádek nahoře přepíná mezi
shodou se všemi zapnutými kritérii a shodou s kterýmkoli z nich. Pole názvu a seznam
kategorií výsledek vždy dál zužují. Řádek skrytých souborů se dívá jen na zapnuté módy.

### Co vám seznam o módu řekne

Dva poradní příznaky, oba jako glyf s vysvětlením při najetí. No valid game data znamená,
že nic nahoře v módu nevypadá jako něco, co tato hra načítá; může potřebovat přesun složek
o úroveň výš, nebo to nemusí být mód pro tuto hru. Another game znamená, že vlastní
`meta.ini` módu jmenuje jinou. Ani jeden nic neblokuje: mód se stále nasazuje. „Mark as
valid" v nabídce řádku ztiší kterýkoli přes vlastní klíč `validated=` z MO2, takže mód
označený jako platný v jednom správci není v druhém označen.

Kontrola rozložení označí mód, jen když si je jistá: strom `Root/`, nečitelná složka a
prázdná složka se všechny počítají jako platná herní data.

### Vypnutí módu, který vaše uložené pozice nebo jiné módy potřebují

Než se mód vypne nebo odebere, Eidos přečte seznamy pluginů tří nejnovějších pozic profilu.
Když jedna z nich používá plugin z toho módu, lišta nad seznamem jmenuje pluginy a čeká na
Disable anyway nebo Cancel (zruší i Escape). Remove přidá stejná jména do své zprávy s
výzvou ke kliknutí znovu. Čtou se jen pozice Skyrimu a Falloutu 4; ostatní hry se vypínají
bez ptaní.

Information módu, záložka General, vypisuje módy, které vyžaduje, a módy, které vyžadují
jeho, čtené z masterů zapnutých pluginů. Vypnutí módu, který jiné vyžadují, ukáže stejnou
lištu s Disable them too, což vypne i každý mód, který ho potřebuje, a módy, které
potřebují ty, nebo Keep them enabled. Karta chybějících masterů v záložce Diagnostics má
Enable what provides them: zapne mód, který drží každý chybějící master, a zaškrtne master
plugin, který je přítomný, ale nezaškrtnutý.

### Záloha módu, než na něj sáhnete

„Back up this mod" zkopíruje jeho složku stranou jako `<name>_backup` (pak `_backup2` a tak
dále; záloha nikdy nenahradí předchozí). Kopie není mód: její zaškrtávátko nic nedělá a nic
nepřidává do sloučeného pohledu, protože zaškrtnutí by nasadilo dvě kopie jednoho módu
přes sebe. „Restore this backup over the mod" ji vrátí, dvěma kliknutími; současný obsah se
nejdřív odloží a zahodí se, až když kopie uspěje.

**Backups** na liště nástrojů drží body obnovy seznamu módů profilu, jeho pořadí načítání
(`plugins.txt` a `loadorder.txt` společně) a jeho herních INI, po 10 od každého, v
`<profile>/backups/`. Back up now jeden přidá a Restore nejdřív zálohuje současný stav,
takže chybný výběr jde vrátit. Eidos také uchovává bod obnovy INI před každým během
BethINI (viz [tools.md](tools.md#bethini-pie)).

### Záložky

**Data** je strom sloučeného pohledu, rozbalovaný po jedné úrovni: otevření složky přečte tu
složku v každé vrstvě, která ji má, nikdy každý zapnutý mód najednou. Staví se ze stejného
zásobníku vrstev, který připojení obsluhuje, takže whiteouty a skryté soubory se
respektují a záložka ukazuje, co hra uvidí. Filtrujte ji podle názvu, zužte na jen
sporné soubory, zorientujte se, co kde je, sloupci Size a Modified a Reveal libovolný řádek
ve správci souborů. Najetí na soubor vypíše další módy, které ho dodávají. Soubory
`meta.ini`, `readme.txt`, `fomod/` a `Root/` na nejvyšší úrovni módu se z Data vynechávají,
v této záložce i v připojení: první tři jsou pro správce módů a `Root/` jde místo toho do
složky hry. Soubor, který hra nebo nástroj zapíše pod jedním z těchto názvů, skončí v
Overwrite a zůstane viditelný.

Řádek `.exe` v Data má Add tool, které otevře editor Executables s vyplněným tím souborem
(nic se neuloží, dokud nestisknete Save), a Run, které ho jednou spustí přes sloučený pohled
jako zaregistrovaný nástroj ([Nástroje](#nástroje)). Save tree zapíše celý sloučený strom Data do
textového souboru, jeden řádek na soubor s módem, který ho dodává, s odstraněnou domovskou
složkou a uživatelským jménem, pro hlášení chyby.

**Plugins** je pořadí načítání ESP/ESM/ESL: přepínejte, přeřazujte ručně, nebo řaďte LOOTem
a čtěte zprávu po řazení, jejíž odkazy s radami se otevírají v prohlížeči. Jeho záhlaví
počítá zapnuté pluginy v každém indexovém prostoru proti limitu hry, například
`full 231/254, light 512/4096`, a zoranžoví, když je jeden plný. Pole filtru zužuje seznam
podle názvu. Mezerník, Ctrl+A, rozsahy se Shiftem a položky nabídky řádku Activate all shown
a Deactivate all shown pak působí jen na zobrazené řádky a přetažení se odmítne, dokud se
filtr nevymaže. Výběr pluginu obarví jeho mastery v seznamu pluginů a mód, který ho dodává,
v seznamu módů. Pluginy, které LOOT označí jako špinavé, dostanou kartu v Diagnostics, která
je umí vyčistit přes xEdit ([tools.md](tools.md#čištění-pluginu-který-loot-označí-jako-špinavý)).

Po řazení LOOTem to plný plugin, jehož nové záznamy se všechny vejdou do rozsahu light, řekne
při najetí a nabídka jeho řádku má Flag as light. Eidos zapíše kopii s příznakem light do
nového módu pojmenovaného `<plugin> (light)` na konci seznamu módů (nejvyšší priorita) a
originál nechá, takže vypnutí toho módu to vrátí. Odmítne, když jedna ze tří nejnovějších
pozic profilu načítá plugin jako plný. Čtou se jen pozice Skyrimu a Falloutu 4, takže u
Starfieldu se nekontroluje žádná. LOOT kontroluje jen rozsahy záznamů: plugin, který přidává
cely nebo worldspacy, si před označením prohlédněte v xEditu.

Najetí na plugin ukáže, odkud pochází, jeho autora, jeho hlavičku a verzi formulářů,
prvních 300 znaků jeho popisu, jeho mastery a po řazení LOOTem zprávy LOOTu. U Skyrimu,
Skyrimu SE, Skyrimu VR a Enderalu SE hra čte také INI pojmenované podle načteného pluginu;
sloupec INI tyto pluginy označí a kliknutí otevře ten soubor jen pro čtení.

**Conflicts** vysvětluje vítěze a poražené po souborech. **Overwrite** promění to, co hra
zapsala, jedním krokem ve skutečný mód.

**Archives** vypisuje každý BSA nebo BA2, který zapnuté módy dodávají, mód, ze kterého
pochází, a proč se načítá: aktivní plugin stejného názvu, nebo ho registruje INI. Ten, který
nic nenačítá, se označí jako mrtvá váha. Když jiný mód dodává archiv stejného názvu, který
konflikt vyhrává, řádek jmenuje ten mód. Extract rozbalí archiv do nového módu pojmenovaného
podle něj bez přípony (`Foo.ba2` dá `Foo (extracted)`) a archiv nechá v jeho módu; volné
soubory vyhrávají nad soubory v kterémkoli archivu, ať je pořadí módů jakékoli.

**Saves** čte hlavičku každé pozice (postava, úroveň, místo, doba hraní), porovnává uložený v
ní seznam pluginů se současným a má tlačítko, které zapne módy, které potřebuje. Pozice,
které se do prefixu dostaly bez Eidosu, popisuje část
[Uložené pozice patří profilu](#uložené-pozice-patří-profilu).

**Downloads** a **Diagnostics** mají níže vlastní části.

### Informace o módu

„Information..." otevře dialog pro mód se záložkami General, Conflicts, Filetree, INI Tweaks,
Notes, Nexus files a Nexus page. Z filetree (a ze stromu Data) lze skrýt kterýkoli soubor:
přejmenuje se na `<name>.mohidden`, což ho vyjme ze sloučeného pohledu bez smazání, takže
několik zbloudilých souborů jednoho módu lze potlačit bez změny priorit. Filetree také dělá
běžné operace se soubory: nová složka, přejmenování, smazání, otevření. Každá odmítne cestu,
která není prostou cestou uvnitř toho módu: žádné `..`, žádná absolutní cesta a žádný
symlink po cestě, protože jeho následování by mohlo smazat soubor mimo složku módu. Rename
mění jen poslední část cesty, takže nikdy nepřesune soubor, a odmítne název, který je už
obsazen. Delete chce dvě kliknutí a nelze ho vrátit.

**View** na libovolném řádku ve filetree nebo stromu Data zobrazí náhled souboru: obrázky,
text, textury DDS (vyberte mip, vrstvu, stěnu a kanál) a statické modely NIF s jejich
difuzními texturami. Náhled NIF potřebuje `eidos-nif-preview` vedle Eidosu nebo v `PATH`;
balík vydání, balíček pro Arch a sestavení ze zdrojů ho mají, `eidos-linux` ne. Text se čte
do 64 KB a řekne, kde skončil.

**INI Tweaks** vypisuje fragmenty, které mód dodává ve své složce `INI Tweaks/`; zapnuté se
při spuštění slučují do herního INI profilu, v pořadí priority, a při zachycení INI běhu se
zase odstraní, takže vypnutí tweaku jeho nastavení odstraní.

Záložka **Nexus files** vypisuje soubory módu na Nexusu, nejnovější první, a označí ten
nainstalovaný zde a ten, který ho nahrazuje. Ten druhý řádek má tlačítko **Update**; viz
[Aktualizace módů](#aktualizace-módů).

Záložka **Nexus page** ukazuje popis módu a, nejnovější první, changelog každé verze souboru
na jeho stránce. Eidos čte obojí při prvním otevření záložky, z GraphQL API Nexusu, které se
nepočítá do hodinového rozpočtu požadavků. Drží je, dokud se Eidos nezavře nebo nestisknete
**Refresh**. Zobrazují se jako prostý text: formátování BBCode a HTML se odstraní, adresa
odkazu se napíše za jeho text a obrázky a videa se vynechají. **Endorse** a **Track** nad
textem dělají to, co nabídka módu na pravé tlačítko. Stránka módu pro dospělé zůstane
skrytá, pokud váš účet na Nexusu neukazuje obsah pro dospělé, a stránka bez hodnocení zůstane
skrytá vždy.

### Stahování

Stažený soubor lze přetáhnout ze seznamu Downloads na pozici v seznamu módů a nainstalovat ho
s touto prioritou a archivy nebo složky upuštěné na okno ze správce souborů se také
nainstalují. Upuštění souborů potřebuje relaci X11 nebo XWayland: winit je implementuje jen
pro X11. Stahování lze pozastavit a obnovit: pozastavení zastaví přenos a ponechá částečný
soubor a Resume požádá Nexus o nový odkaz a pokračuje od místa, kde přestal.

Záložka Downloads drží archivy, které jste stáhli. Filtrujte ji podle názvu (i podle
přátelského názvu módu, takže „skyui" najde `SkyUI_5_2_SE-12604-5-2SE.7z`), řaďte podle
nejnovější, názvu, velikosti nebo stavu a skryjte archiv, který jste dokončili: soubor zůstane
a zmizí jen řádek. „Show hidden" je vrátí a totéž tlačítko je odkryje. „Remove N installed"
smaže archivy módů, které už jste nainstalovali, dvěma kliknutími, a jen ty, které filtr
ukazuje.

**Preferences -> Nexus -> Install downloads when they finish** je ve výchozím stavu vypnuto.
Po zapnutí se stažení z Nexusu, které skončí, zatímco je okno otevřené, nainstaluje bez
návštěvy záložky Downloads. Je to obvyklá instalace: FOMOD otevře svůj instalátor a název,
který už v seznamu módů je, se zeptá, co dělat. Archivy, které byly v `downloads/` dřív, než
se okno otevřelo nebo než se volba zapnula, se nechají být, stejně jako stažení, jehož
instalaci jste zrušili. Dokud je v panelu kolekce otevřená kolekce, žádné stažení se
nenainstaluje samo a to, které mezitím skončí, se nenainstaluje samo ani později; členy
instaluje kolekce. Soubor, na který čeká čekající Update, nainstaluje Update. Tímto způsobem
se instalují jen stažení z Nexusu a to, které filtr Downloads skrývá, čeká, dokud se filtr
nevymaže. Zatímco probíhá přetahování, nebo zatímco instanci drží hra nebo jiný Eidos,
instalace čeká.

### Aktualizace módů

Tlačítko aktualizace poblíž pravého konce lišty nástrojů zkontroluje každý mód, který má id
z Nexusu. Když stránka módu ukazuje verzi, kterou máte nainstalovanou, nic dalšího se
nezjišťuje. Jinak Eidos přečte seznam souborů módu (jeden požadavek navíc, uchovaný po zbytek
kontroly) a sleduje řetězec aktualizací od id souboru, které zaznamenává `meta.ini` módu:
autoři označí nové nahrání jako nástupce staršího a MO2 čte stejný řetězec. Novější soubor v
tom řetězci je aktualizace. Když řetězec žádný nejmenuje a nainstalovaný soubor je na stránce
stále uveden, mód se počítá za aktuální, i když stránka ukazuje jinou verzi. Verze stránky
rozhoduje jen tehdy, když není zaznamenáno žádné id souboru, když byl nainstalovaný soubor
odstraněn nebo archivován, nebo když Nexus seznam souborů neposlal.

Každý mód používá doménu Nexusu té hry, kterou jmenuje jeho vlastní `meta.ini` (`gameName`),
ne hry instance: kontrola aktualizací, Endorse, Track i Visit on Nexus míří tam. Enderal SE
přijímá i módy `skyrimspecialedition`, takže mód pro Skyrim SE nainstalovaný v instanci
Enderalu SE se kontroluje na své stránce pro Skyrim SE. Odkaz Mod Manager Download jde do
instance, kterou jste použili naposledy, když její hra doménu odkazu přijímá. Jinak jde do
instance hry, jejíž vlastní doménou je, dřív než do hry, která ji jen přijímá.
`EIDOS_INSTANCE=<folder>` přepíše obojí.

Track a Untrack v nabídce módu na pravé tlačítko přidají mód mezi vaše sledované módy na
Nexusu nebo ho odeberou, takže oznámení webu sledují to, co sledujete zde. Endorse a Abstain
ve stejné nabídce mód na Nexusu schválí nebo schválení stáhnou; tlačítko endorse vedle
tlačítka aktualizace schválí vybraný mód. Každá kontrola aktualizací také čte vaše schválení a
sledované módy z Nexusu (dva požadavky) a nastaví příznaky Endorsed a Tracked každého módu
podle nich, na doméně Nexusu toho módu. Když se oba seznamy s příznaky neshodují, vyhrává Nexus.
Když nejde přečíst jeden ze seznamů, kontrola to řekne a ty příznaky ponechá, jak byly.

Tlačítko **Update** na záložce Nexus files v „Information..." nainstaluje novější soubor. S
prémiovým účtem Eidos soubor stáhne sám. S bezplatným účtem otevře stránku souboru, kde ho
Mod Manager Download pošle zpět, a instalace začne, když archiv přistane v `downloads/`.
Odtud je to obvyklá instalace: FOMOD se otevře s odpověďmi, se kterými byl mód nainstalován, a
pak výzva nabídne Replace se zaškrtnutým „Keep a backup of the existing mod", ať jste při
poslední výzvě zvolili cokoli. Replace zachová prioritu a stav zapnutí módu.

Než Replace vymění složku módu, Eidos porovná starou složku s novou podle názvu souboru a
velikosti a přečte hlavičky pluginů v obou. To platí pro Update, aktualizaci kolekce a
Replace zvolené při výzvě k instalaci. Stavový řádek pak spočítá soubory přidané, odebrané a
změněné ve velikosti a jmenuje každý master, který nová verze potřebuje a stará ne. Záložka
**General** v „Information..." končí částí **Last update**, která vypisuje verze, přidané
nebo odebrané pluginy, přidané nebo odebrané mastery, pluginy, jejichž příznak ESM, ESL nebo
ESH se změnil, módy, se kterými tento mód teď sdílí soubory nebo už nesdílí, a soubory. Soubor
přepsaný ve stejné velikosti se nevypisuje. Zpráva se drží v `logs/updates/<mod>.json` v
instanci, který `eidos pack` vynechává; čerstvá instalace nebo Merge do složky ji odstraní.
`eidos install --replace` vypíše stejné počty na řádku `changes:`.

Složka, která drží několik souborů z Nexusu, například hlavní soubor s do něj sloučeným
volitelným, se odmítne: nahrazení novým hlavním souborem by ostatní zahodilo. Takové
aktualizujte ručně.

`eidos nexus update skyrimse --apply` spustí stejnou kontrolu a pak na prémiovém účtu stáhne
každý nový soubor a nainstaluje ho přes jeho složku s `--replace --backup`. Na bezplatném účtu
místo toho vypíše `OPEN <url>` pro stránku každého souboru. Složky, které slučují několik
souborů, se přeskočí s důvodem. Mód FOMOD bez zaznamenaných odpovědí, například nainstalovaný
dřív, než je Eidos začal uchovávat, se nepřeinstaluje s výchozími hodnotami instalátoru: ta
aktualizace selže se zprávou, aby se mód jednou aktualizoval z okna, které odpovědi
zaznamená. FOMOD, jehož instalátor nemá co volit, se nainstaluje jako obvykle. Příkaz skončí
kódem 1, pokud některá aktualizace selhala.

### Aktualizace kolekcí

Když spustíte Check for updates, Eidos se také zeptá Nexusu na nejnovější revizi každé kolekce
nainstalované v této instanci. Řádek nad seznamem módů jmenuje každou kolekci s novější revizí
a počítá přidané (+), aktualizované (~) a vypuštěné (-) módy; najetím se zobrazí jejich
názvy. Počty chybí, když recept nainstalované revize už není v cache. **Update** otevře novější
revizi v panelu kolekce, kde ji *Install this collection* spustí. Seznam se při každé kontrole
staví znovu a neukládá se.

### Nastavení Nexusu

**Preferences -> Nexus -> Offline** zastaví jakýkoli kontakt Eidosu s Nexusem a přeskočí
denní kontrolu vydání na GitHubu. Kontroly aktualizací, přihlášení, stahování a kolekce to
řeknou místo selhání s chybou připojení. Ve výchozím stavu je vypnuto.

**Preferred servers** řadí uzly CDN, které stahování preferuje, nejlepší první. Víc než jedno
zrcadlo k výběru dostane jen prémiový účet, takže u všech ostatních vybírá Nexus a toto nic
nemění. Když není nabídnut žádný z vámi jmenovaných serverů, stahování přesto proběhne, z
prvního uzlu, který Nexus nabídne.

### Kategorie

Kategorie lze přiřadit jednomu módu nebo celému výběru, katalog lze upravovat ze stejného
dialogu a oficiální seznam kategorií hry lze stáhnout z Nexusu. Oba soubory katalogu jsou
vlastní soubory MO2 (`categories.dat` a `nexuscatmap.dat`), takže sdílená instance drží jeden
katalog. Mód stažený z Nexusu se při instalaci zařadí do kategorie, jakmile Fetch from Nexus
namapoval katalog.

### Editor INI, log a rozšíření

**View -> INI editor** upravuje herní INI profilu, kopii, kterou Eidos drží; kopie v prefixu
Proton se při každém spuštění přepíše. Jeho poslední záložka je `launch-env.ini` profilu
([Proměnné a obaly pro profil](#proměnné-a-obaly-pro-profil)).
**View -> Log** čte logy relací. **View -> Extensions** vypisuje vaše vlastní doplňky; viz
[extensions.md](extensions.md).

Když spuštění nenajde hlavní INI hry (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...) ani v
profilu, ani v prefixu, Eidos nejdřív zkopíruje výchozí z herní složky (`Skyrim_Default.ini`,
`Fallout4_Default.ini`...) do profilu, jako MO2. Když instance spouští kopii hry, výchozí se
bere z kopie. Prefix a složka hry se nemění.

### Instalace módů

Instalátor zvládá archivy Simple, FOMOD, balíčky Wrye Bash BAIN (zaškrtněte podbalíčky, které
se slučují v pořadí) a balíčky OMOD z Oblivionu. Skript OMOD se spustí, když používá příkazy
OBMM, které Eidos podporuje, nebo je jedním ze tří instalátorů v C#, které Eidos zná; žádný
jiný kód v C# ani Visual Basicu Eidos nespouští a řekne to. Když rozložení nerozpozná žádná
heuristika, ruční výběr ukáže strom archivu a nechá vás ukázat kořen dat.

Přeinstalace nebo aktualizace módu FOMOD otevře průvodce s odpověďmi z jeho poslední
instalace zaškrtnutými, na prvním kroku, jehož volby se od té doby změnily, s vypsanými
změnami. **Reinstall with new choices** místo toho začne od výchozích hodnot instalátoru.
`eidos install --replace` nebo `--merge` použije uložené odpovědi a odmítne, s výpisem změn,
když už nesedí.

### Diagnostika

Záložka Diagnostics spouští živé kontroly stavu. Hlásí chybějící mastery, pluginy, které LOOT
označí jako špinavé, archivy, které žádný aktivní plugin nenačte, zda seznam módů stále
odpovídá složce módů, zda je zapnutý FUSE passthrough a, když je vyžádán, zda je přítomné
oprávnění, které k spuštění potřebuje. Po běhu ukazuje, co o každé DLL svých pluginů říká
vlastní log script extenderu, takže uvidíte, zda se vaše pluginy SKSE načetly. Hlásí také
aktualizaci hry od posledního čistého spuštění, chybějící script extender nebo takový, který
neodpovídá verzi hry (se stránkou ke stažení extenderu a tlačítkem pro instalaci archivu,
který stáhnete), parametr spuštění Steamu, který nespouští Eidos s `%command%`, NVAPI
ponechané vypnuté pro DLSS, exkluzivní celou obrazovku, která brání generování snímků, ENB
`enblocal.ini` bez `LinuxVersion=true`, deklarace pluginů F4SE a SFSE a log pádu z poslední
relace s módy, které jmenuje.

Kontroly pro INI profilu a pluginy hledají řádky `sTestFile` v `[General]` vlastních INI
Falloutu 4, Falloutu 4 VR a Starfieldu, které způsobí, že hra ignoruje `plugins.txt` (Fix je
odstraní z kopie profilu a předchozí soubor ponechá jako `<ini>.pre-fix`), zapnuté pluginy
`.esp` ve Starfieldu a pluginy blueprint, kterým chybí příznak blueprint, předpona
`BlueprintShips-` nebo přípona `.esm`, nebo jejichž hlavní plugin není zapnutý. Systémové
kontroly označí `vm.max_map_count` nebo pevný limit otevřených souborů pod tím, co Proton
potřebuje, a hru, prefix nebo instanci na NTFS, exFAT nebo FAT. Kontroly módů jmenují datovou
cestu BodySlidu, když neukazuje na `Data` hry, složky `Data`, které nástroj zapsal do
`Overwrite/Root`, patch, jehož volné soubory prohrávají s módem, který záplatuje, a mód, který
nahrazuje skripty Papyrus script extenderu.

Check for updates také čte požadavky každého módu na Nexusu. Diagnostics vypíše požadovaný mód,
který chybí nebo je vypnutý, s Ignore pro požadavek splněný jinak.

Záložka také ukazuje, které sestavení Protonu hru spouští, a varuje, než starší Proton spustí
prefix, který připravil novější; Rebuild odsune rozbitý prefix stranou, aby Steam vytvořil
nový ([troubleshooting.md](troubleshooting.md#který-proton-hru-spouští-a-rozbitý-prefix)).
Další karty označí prefix, který nebyl nikdy nastaven nebo jehož registr je zastaralý, jiný
program než Eidos přijímající odkazy Mod Manager Download (Register to opraví), výstup nástroje
starší než jeho vstupy (s tlačítkem, které nástroj spustí znovu) a vyčištěnou kopii nebo kopii s
příznakem light pluginu, jehož originál se od té doby změnil. Každou kartu vysvětluje
[troubleshooting.md](troubleshooting.md#karty-health). **Copy report** dá na schránku zprávu o
systému a každém zjištění, s vaší domovskou složkou a přihlašovacím jménem skrytými, pro hlášení
chyby.

U Skyrimu SE, Enderalu SE a Falloutu 4 má Diagnostics také kartu Display. Ukazuje velikost a
režim okna nastavené v `SkyrimPrefs.ini` profilu (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), nebo v
kopii prefixu, když ji profil zatím nemá. Když velikost nenastavuje ani jedno, její pole
začínají na největší připojené obrazovce uvedené v `/sys/class/drm`, takže Steam Deck čte
1280x800. Šířka a výška jsou celá čísla od 320 do 16384. Apply zapíše `iSize W`, `iSize H`,
`bFull Screen` a `bBorderless` do kopie profilu, poté co převezme kopii prefixu, pokud ji profil
nemá. Když okno nastavuje SSE Display Tweaks, Apply zapíše stejnou velikost a režim do jeho kopie
v Overwrite; soubor samotného módu se nemění.

Kartu Game files nahoře v záložce popisuje část
[Soubory hry](#soubory-hry-instalace-z-obchodu-nebo-kopie), kartu pořadí kolekce část
[Zachování pořadí kolekce](#zachování-pořadí-kolekce) a Find the mod behind a problem část
[Testovací běhy](#hledání-módu-za-problémem).

### Spuštění hry

Chcete-li hru spustit přes okno, nastavte parametr spuštění hry ve Steamu na absolutní cestu k
programu, protože Steam nečte `PATH` vašeho shellu
([install.md](install.md#potom-nasměrovat-na-něj-steam)):

```
~/.local/bin/eidos-gui %command%
```

Z Colony je cesta `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos se otevře na instanci
hry, té, kterou jste naposledy použili, takže přenosná instance se najde znovu stejně jako
globální; klikněte na Run a spustí se přes sloučený pohled.

Se zapnutým **Start the game at once when a store opens Eidos** (Preferences, General, Running a
game; ve výchozím stavu vypnuto) Eidos to kliknutí přeskočí: zkontroluje Health a spustí hru bez
zobrazení okna. Okno se otevře jako obvykle, když Health uvádí problém, když seznam módů
potřebuje pozornost, když nástroj nastavený ke spuštění před hraním má zastaralý výstup, nebo
když u Skyrimu SE, Enderalu SE a Falloutu 4 profil zatím nemá velikost okna (okno se pak otevře
na Diagnostics). Chcete-li okno pro jedno spuštění, dejte `EIDOS_WINDOW=1` před příkaz:
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

V okně, které neotevřel žádný obchod, Run hru nespouští samo a Eidos nikdy nespouští hru přes
Proton ani Wine přímo. Run požádá o spuštění obchod, který kopii vlastní, a obchod spustí Eidos:

- Steam spustí `steam -applaunch <appid>`, když je nastaven parametr spuštění výše a hra má
  prefix Proton, který Steam vytvoří při prvním spuštění hry. Steam se nejdřív spustí, je-li
  zavřený.
- Heroic (kopie z GOG nebo Epicu, nativní nebo Flatpak) dostane odkaz `heroic://launch`, když je
  Eidos obalem v nastavení hry v Heroicu, s absolutní cestou k `eidos-gui` (nebo `eidos-linux`)
  jako spustitelným souborem a bez argumentů. Heroic se nejdřív spustí, je-li zavřený.
- Samotný Legendary spustí `legendary launch <app>`, když `~/.config/legendary/config.ini` má
  `wrapper =` a absolutní cestu k `eidos-gui` (nebo `eidos-linux`) pod sekcí hry `[<app>]`, nebo
  pod `[default]`.

Loader script extenderu, launcher i spustitelný soubor hry v seznamu Run se obchodu takto
předávají, stejně jako `eidos tool <instance> run <entry>` pro jeden z nich v terminálu. Některé
kopie Eidos svému obchodu předat nedokáže, například kopii z GOG mimo Heroic: Run pak řekne, aby
se hra spustila z toho obchodu.

Předání zaznamená tuto instanci jako tu, kterou jste použili naposledy, a obchod pak otevře na ní
druhé okno Eidosu. Při vypnutém Start the game at once v něm kliknete na Run; při zapnutém to okno
spustí hru bez zobrazení sebe sama, pokud ho neotevře některý z případů výše. První okno ten běh
nesleduje, takže po hraní v něm stiskněte F5 a znovu načtěte jeho seznamy. Když nastavení obchodu
chybí, nebo loader script extenderu neleží vedle spustitelného souboru hry, Run nespustí nic a
ukáže, co a kam nastavit, se skutečnou cestou běžící binárky.

Zatímco hra nebo nástroj běží, Run se změní na Stop. Dvě stisknutí ukončí každý program, který běh
spustil, a Eidos pak zachytí relaci a odpojí jako po každém ukončení; zastavený běh se nikdy
nepočítá jako čisté spuštění
([troubleshooting.md](troubleshooting.md#hra-nebo-nástroj-který-se-nezavře)).

`%command%` ze Steamu pro tituly Bethesdy obvykle ukazuje na `<Game>Launcher.exe`. Eidos ho nikdy
nespouští: launcher je samostatná aplikace s nastavením, která znovu prohledá `Data` a přepíše
`plugins.txt`, čímž zruší právě nasazené pořadí načítání. Eidos místo toho spustí loader script
extenderu, je-li nainstalován, jinak binárku hry, a řekne to, když musí přejít na binárku hry.

Eidos prohledá DLL zapnutých módů, zda neimportují `d3dcompiler_47.dll`, zkopíruje DLL od
Microsoftu do prefixu a teprve potom nastaví `WINEDLLOVERRIDES=d3dcompiler_47=n,b`; sami to
nastavovat nemusíte.

### Proměnné a obaly pro profil

Profil může nést vlastní proměnné prostředí a příkazy obalů v `profiles/<name>/launch-env.ini`, jedna
položka na řádek:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Platí pro každé `eidos play` toho profilu (hra spuštěná přes `%command%` ze Steamu, nebo přes Run v
okně, které otevřel obchod) a pro každý běh nástroje. `eidos shell` dostane proměnné a žádný obal.
První řádek `wrapper=` je nejvnější. Řádek `wrapper=`, který obsahuje `%command%`, dostane příkaz na
tom místě, jako v parametrech spuštění Steamu; řádek bez něj dostane příkaz na konci. Obaly běží
uvnitř soukromého jmenného prostoru Eidosu, takže vidí módy.

Proměnná zde nahradí hodnotu, kterou Eidos nastavuje pro stejný název, a vlastní řádek `env=`
nástroje v `tools.ini` nahradí obojí. `WINEDLLOVERRIDES` se místo toho přidává za přepisy, které
Eidos nastavuje pro ENB, ReShade a přibalená DLL, takže DLL jmenovaná v obou dostane nastavení
profilu a ostatní si ponechají své.

View -> INI editor otevře soubor jako svou poslední záložku, pro každou hru. Prázdné řádky a řádky
začínající `#` nebo `;` se přeskakují. Uložení vypíše čísla řádků, které Eidos ignoruje: řádky,
které nejsou ani `env=NAME=value`, ani `wrapper=command`, a řádek `wrapper=`, jehož první slovo je
`NAME=value`, jak to parametry spuštění Steamu dovolují; tu proměnnou dejte místo toho na řádek
`env=`. `eidos play <instance>` bez příkazu vypíše proměnné, které běh dostane, a obaly. Copy report
v Diagnostics jmenuje proměnné a programy obalů, bez jejich hodnot.

## Nástroje

xEdit, BodySlide, DynDOLOD a další nástroje běží přes sloučený pohled uvnitř prefixu Wine hry
(prefix Proton ze Steamu, nebo ten, který Heroic nastavil pro kopii z GOG nebo Epicu):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # spustit exe jednou bez registrace
eidos prereqs skyrimse            # co registrované nástroje potřebují, a jejich stav
eidos prereqs skyrimse --install  # stáhnout, co chybí
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen a Synthesis se najdou bez
registrace. Pandora, Nemesis, FNIS, BodySlide a Synthesis přicházejí s už nastaveným módem pro
zachycení, takže jejich výstup jde do vlastního módu (podrobnosti v
[tools.md](tools.md#posílání-výstupu-nástroje-do-jeho-vlastního-módu)).

Název, který nástroji dáte, rozhoduje o tom, jaké runtimy a DLL mu Eidos poskytne: `BodySlide`
dostane své knihovny DirectX, `BS` nedostane nic. V okně dialog Executables ukazuje pod polem
skutečný stav každé prerekvizity a chybějící jsou tlačítka.

Balík vydání nese instalátory vcrun2022 (x86 a x64), .NET 8 Desktop Runtime 8.0.12 a runtime .NET 10
10.0.10 v `runtimes/` a `install.sh` je zkopíruje do `~/.local/share/Colony/Eidos/runtimes/`, takže
`eidos prereqs <instance> --install` je nainstaluje bez stahování. Eidos spouští instalátory
vcrun2022 a .NET 8 od Microsoftu bez okna v prefixu hry, bez winetricks. `eidos-linux` a sestavení ze
zdrojů nenesou složku `runtimes/`, takže stahují tytéž připnuté soubory od Microsoftu. .NET 10 SDK,
který Synthesis potřebuje (asi 300 MB), se stahuje vždy.

Tabulka, tři úrovně prerekvizit, proč DynDOLOD potřebuje runtime .NET, který winetricks nenainstaluje,
a proč se nástroj nainstalovaný jako mód spouští ze sloučené cesty místo ze své vlastní složky, jsou v
[tools.md](tools.md).

## Rozšíření

Eidos lze rozšířit bez nového sestavení: manifest TOML v `~/.config/Colony/Eidos/addons/` přidá nástroj
do seznamu Extensions nebo kontrolu do záložky Diagnostics. Do Eidosu se nic nenačítá: rozšíření je
program, který Eidos spouští. Viz [extensions.md](extensions.md).

## Vyzkoušejte důkaz konceptu

Skript nepotřebuje žádnou hru. Ukáže sloučený pohled, copy-on-write, složku hry ponechanou nedotčenou a
sloučení, které existuje jen uvnitř jeho jmenného prostoru, a používá jen neprivilegovaný OverlayFS v
uživatelském jmenném prostoru (Linux 5.11 nebo novější):

```sh
./scripts/poc-overlay.sh
```
