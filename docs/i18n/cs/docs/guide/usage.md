<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Používání Eidosu

Praktická příručka: CLI, GUI, parametr spuštění ve Steamu, sestavení ze
zdrojových kódů a skript s důkazem konceptu. Co dělat, když něco vypadá špatně,
najdete v [troubleshooting.cs.md](troubleshooting.md).

## Použití (CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into <instance>/mods/ (the global instance lives
#    at ~/.local/share/eidos/skyrimse; `eidos init` prints yours)...
eidos install skyrimse mod.7z     # or install a downloaded archive (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopt an existing MO2 profile's order + plugin state
eidos sort skyrimse               # LOOT-sort the plugin load order
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` a `eidos export` sadu
doplňují; úplný seznam vypíše `eidos` bez argumentů.

### Instance: globální a přenosné

Každý příkaz výše se obrací na instanci. `skyrimse` pojmenovává tu **globální** -
uloženou centrálně v `~/.local/share/eidos/skyrimse` a spravovanou Eidosem.
Druhý druh je **přenosná**: soběstačná složka kdekoli chcete (druhý disk, herní
oddíl), přesunutelná a izolovaná, přesně jako přenosné instance v MO2. Kdekoli
příkaz bere identifikátor hry, bere i složku přenosné instance:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

Složka se popisuje sama (její `eidos-instance.ini` pojmenovává hru), takže nic
dalšího není potřeba - a `EIDOS_INSTANCE=<folder>` v prostředí přesměruje
identifikátor hry na tu složku, což se hodí v parametrech spuštění ve Steamu.
Přenosné instance, které jste vytvořili nebo otevřeli, si Eidos pamatuje
(naposledy použité první) v `~/.config/Colony/Eidos/instances.ini`; uvítací
obrazovka GUI je nabízí k otevření jedním kliknutím, spuštění ze Steamu přistane
na té, kterou jste hráli naposledy, a obsluha `nxm://` stahuje do ní. Dvě
výhrady, které stojí za to znát: přesun přenosné složky zachová všechno kromě
záznamů nástrojů, které jste zaregistrovali absolutními cestami do původního
umístění (ty přidejte znovu), a sdílená mezipaměť běhových prostředí
(`~/.local/share/Colony/Eidos/runtimes/`) záměrně zůstává společná pro celý
stroj - 78MB hostitel .NET není záležitost jednotlivé instance.

Eidos drží vlastní soubory pod `Colony/Eidos`, v rozvržení, které používá každý
program z rodiny Colony: `~/.config/Colony/Eidos/` pro to, co jste zvolili
(předvolby, vaše relace na Nexusu, seznam instancí, definice her a doplňků,
které jste napsali), `~/.local/state/Colony/Eidos/logs/` pro logy relací a
`~/.local/share/Colony/Eidos/` pro to, co Eidos stáhl. Starší Eidos je držel v
`~/.config/eidos/` a `~/.local/state/eidos/`; první spuštění po aktualizaci je
**zkopíruje** a napíše to do logu. Staré adresáře zůstanou přesně takové, jaké
byly - nic se nemaže, takže vás špatná aktualizace nemůže stát přihlášení - a až
budete spokojeni, můžete je smazat sami.

Vaše módy do toho nepatří. Globální instance dál žije v
`~/.local/share/eidos/<game>/` a přenosná tam, kam jste ji dali, protože tyto
cesty jsou zapsané ve vašem seznamu instancí a možná i v parametru spuštění ve
Steamu: jejich přesun by rozbil odkaz, jehož oba konce Eidos nevlastní.

Jedno místo je odmítnuto rovnou: **uvnitř instalační složky hry** (reflex
veteránů MO2). Ten strom vlastní Steam - aktualizace, „ověření integrity" nebo
odinstalace jej mohou přepsat nebo smazat a vzít s sebou celou vaši sestavu - a
Eidos připojuje přes kořen hry, takže instance uvnitř by seděla uvnitř vlastního
cíle připojení. Průvodce, `eidos init` i `eidos play` řeknou ne; dejte složku
raději VEDLE hry (sourozenec na stejném disku dá stejné pohodlí).

`play` připojí módy instance přes vlastní adresář `Data` hry (přes bind-stash,
takže démon dál čte nedotčené soubory) uvnitř soukromého jmenného prostoru a pak
skrz tento pohled spustí příkaz. Zápisy (savy, znovu vygenerované konfigurace)
přistanou ve vrstvě `overwrite/` dané instance; instalace hry i každý zdroj módu
zůstanou bajt po bajtu nedotčené.

### Žádný privilegovaný krok

Eidos běží plně bez roota. Připojuje v soukromém uživatelském + připojovacím
jmenném prostoru, takže žádný setuid pomocník, žádný démon a nic, co byste
museli udělovat.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` je **volitelné** a otevírá
přesně jednu věc: jaderný FUSE passthrough, který je ve výchozím stavu vypnutý,
protože rozbíjí hru (níže). S touto schopností si Eidos vezme prostý připojovací
jmenný prostor místo uživatelského; módy se nasadí v obou případech stejně.


Proč stará rada se `setcap` zmizela - a proč se FUSE passthrough dodává vypnutý -
vysvětluje [troubleshooting.cs.md](troubleshooting.md#proč-je-passthrough-ve-výchozím-stavu-vypnutý).

## GUI

```sh
cargo run -p eidos-gui
```

Průvodce prvním spuštěním ve stylu MO2 ve vzhledu Colony pergamen / bordó:
uvítání -> typ instance (přenosná / globální) -> hra -> název a umístění ->
shrnutí -> vytvoření -> hlavní obrazovka. Uvítací obrazovka také vypisuje každou
známou existující instanci (globální i přenosnou, naposledy použité první)
k otevření jedním kliknutím - slouží zároveň jako přepínač instancí - a když
průvodce nasměrujete na složku, která už instanci obsahuje, PŘEVEZME ji tak, jak
je, místo aby vytvářel přes ni (a rovnou odmítne, pokud složka patří jiné hře).

Hlavní okno o dvou panelech je hotové také: výběr profilu (přepnout, nebo
vytvořit nový zkopírováním současného), seznam módů, který filtrujete, vybíráte,
přeuspořádáváte, seskupujete oddělovači, zužujete podle kategorie a nad kterým
pravým tlačítkem vyvoláte akce, plus záložky Data / Plugins / Conflicts /
Overwrite / Saves / Downloads / Diagnostics a tlačítko Run s výběrem cíle
spuštění.

Přeuspořádání není jen poslat nahoru/dolů: cílené přesuny z MO2 jsou tu také -
poslat nad první konfliktní mód, pod poslední, na výslovnou prioritu nebo do
skupiny oddělovače. Všechny procházejí jedním sdíleným pomocníkem pro přesun,
takže chyba o jedničku, která vzniká odebráním řádků před jejich opětovným
vložením, existuje na jednom místě místo na pěti.

### Sloupce, řazení a seskupování

Seznam ve výchozím stavu kreslí čtyři sloupce a nabízí osm: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Zaškrtnete je v nabídce View.
Že výchozí stav není všech osm, je záměr - seznam se všemi zobrazenými sloupci
nemá místo na NÁZEV, což je sloupec, který ve skutečnosti čtete.

Kliknutím na kterékoli záhlaví seřadíte podle něj. Další kliknutí obrátí pořadí
a třetí vrátí **pořadí načítání**, na čemž záleží víc, než to zní: pořadí
načítání je jediné pořadí, ve kterém lze seznamem táhnout, protože mezera pro
vložení se obrací na skutečný seznam, kdežto seřazený řádek je někde úplně
jinde. Když je řazení zapnuté, vkládací proužky se nekreslí a tažení je
odmítnuto, místo aby přistálo někde, kam nikdo nemířil - přesně to dělá MO2, a
ze stejného důvodu. Nabídka View to říká a nabízí cestu zpět.

Nabídka View umí celý seznam také **seskupit**, podle kategorie nebo podle
původu (z Nexusu, nebo instalované ručně). Záhlaví skupin nejsou oddělovače:
není za nimi nic, co by šlo přejmenovat, obarvit nebo přesunout, sbalují se a
při sbalení zůstává počet na záhlaví. Oddělovače ze seznamu při řazení nebo
seskupení zmizí - oddělovač stojí v čele řádků, které po něm následují v pořadí
načítání, a obojí s nimi pohnulo.

### Myš a klávesnice

Dvojklik na mód otevře Information, Ctrl+dvojklik jeho složku, Shift+dvojklik
jeho stránku na Nexusu. Ctrl+F umístí kurzor do filtračního pole. Napsáním
písmene skočíte na další mód, který jím začíná, a dalším stiskem projdete
zbytek, místo abyste uvízli na prvním. Žádný z nich nemůže přistát na řádku,
který skrývá filtr, sbalený oddělovač nebo sbalená skupina - přesouvat
zvýraznění, které nevidíte, je způsob, jak další mezerník přepne mód, na který
jste se nedívali.

„Collapse others" v nabídce oddělovače sbalí všechny skupiny kromě té jedné.
Během tažení se zastavením nad sbalenou skupinou skupina otevře, takže mód lze
pustit dovnitř, aniž byste tažení nejdřív opustili - zastavením, ne přejetím.

### Co vám seznam o módu řekne

Dva poradní příznaky, oba glyf s vysvětlením po najetí myší. **No valid game
data** znamená, že nic na vrcholu módu nevypadá jako něco, co tato hra načítá;
možná potřebuje posunout složky o úroveň výš, nebo to není mód pro tuto hru.
**Another game** znamená, že vlastní `meta.ini` módu pojmenovává jinou. Ani
jeden nic neblokuje - mód se stejně nasadí - a „Mark as valid" v nabídce řádku
umlčí kterýkoli z nich, skrze vlastní klíč `validated=` z MO2, takže mód, za
který jste se zaručili v jednom správci, dorazí tichý i do druhého.

Kontrola rozvržení je záměrně velkorysá: strom `Root/` se počítá, nečitelná
složka se počítá, prázdná se počítá. Chybné varování na seznamu o pěti stech
řádcích je horší než chybějící.

### Záloha módu, než na něj sáhnete

„Back up this mod" zkopíruje jeho složku stranou jako `<name>_backup` (pak
`_backup2` a tak dál - záloha nikdy nenahradí předchozí). Kopie je **netečná**:
není to mód, její zaškrtávátko nedělá nic a do sloučeného pohledu nepřispívá
ničím, protože jeho zaškrtnutí by nasadilo dvě kopie jednoho módu přes sebe.
„Restore this backup over the mod" ji vrátí zpět, na dvě kliknutí; současný
obsah se nejdřív odsune stranou a zahodí se až poté, co kopírování uspěje.

**Data** je skutečný strom sloučeného pohledu, rozbalovaný po jedné úrovni,
takže otevření uzlu stojí jedno čtení adresáře za každou vrstvu, která jej má,
místo rekurzivní procházky každého zapnutého módu. Odpovídá na něj TÝŽ zásobník
vrstev, ze kterého se obsluhuje připojení, takže whiteouty a skryté soubory jsou
respektovány a záložka se nemůže rozejít s tím, co uvidí hra. Filtrujte podle
názvu, zužte jen na sporné soubory, roztřiďte, co kde je, sloupci Size a
Modified, a kterýkoli řádek ukažte pomocí Reveal ve správci souborů. **Plugins**
je pořadí načítání ESP/ESM/ESL (přepínání, ruční přeuspořádání, nebo řazení
LOOTem a čtení zprávy po seřazení, jejíž odkazy na rady se otevírají ve vašem
prohlížeči). **Conflicts** vysvětluje vítěze a poražené u jednotlivých souborů.
**Overwrite** promění to, co hra zapsala, v jednom kroku ve skutečný mód.
**Saves** rozebere hlavičku každého savu - postava, úroveň, místo, odehraný čas
- a porovná seznam pluginů zapečený uvnitř s vaším současným, s tlačítkem, které
zapne módy, jež sav potřebuje, protože pojmenovat je a nechat to na vás je ta
nudná polovina.

„Information..." otevře dialog konkrétního módu: obecné, konflikty, strom
souborů, úpravy INI, poznámky. Ze stromu souborů (a ze stromu Data) lze
kterýkoli soubor **skrýt** - přejmenovat na `<name>.mohidden`, což jej vyřadí
z virtuálního pohledu, aniž by se smazal, takže tři zatoulané meshe jednoho módu
lze potlačit, aniž byste sáhli na priority. Strom souborů zvládá i běžné
souborové operace: nová složka, přejmenovat, smazat, otevřít. Všechny procházejí
jedním resolverem, který odmítne cokoli, co není prostá cesta uvnitř toho módu -
žádné `..`, žádná absolutní cesta a žádná komponenta, která je symlink, protože
jeho následování by mazání odneslo úplně mimo složku módu. Přejmenování nahrazuje
jen poslední komponentu, takže se z něj nikdy nemůže stát přesun, a odmítne už
obsazený název, místo aby ten soubor mlčky nahradilo. Smazání vyžaduje dvě
kliknutí; je to jediná zdejší akce, kterou další kliknutí nevrátí.

**View** na kterémkoli řádku stromu souborů nebo stromu Data soubor zobrazí:
obrázky a text. Ne DDS ani NIF - ty potřebují blokový dekodér a vykreslovač,
které tento strom nemá - ale řeknou to, místo aby ukázaly prázdné okno, a
odkážou na Reveal. Text se čte do 64 KB a řekne, kde skončil, protože náhled je
letmý pohled a log Papyrusu může mít sto megabajtů. **INI Tweaks** vypisuje
fragmenty, které mód dodává ve své složce `INI Tweaks/`; zapnuté se při spuštění
slučují do herního INI profilu, v pořadí priorit, a při zachycení INI souborů
daného běhu se zase sundají - jinak by se z úpravy tiše stalo nastavení a její
vypnutí by neudělalo nic.

Stažený soubor lze **přetáhnout ze seznamu Downloads na pozici v seznamu módů**
a nainstalovat jej s touto prioritou; archivy nebo složky puštěné na okno ze
správce souborů se instalují také (tahle půlka potřebuje relaci X11 nebo
XWayland - winit implementuje pouštění souborů jen pro X11). Samotná stahování
lze pozastavit a obnovit: pozastavení zastaví přenos a ponechá rozpracovaný
soubor, Resume znovu vyřeší čerstvý odkaz a pokračuje tam, kde skončilo.

Záložka Downloads je **knihovna** archivů, ne fronta přenosů. Filtrujte podle
názvu (i podle přívětivého názvu módu, takže „skyui" najde
`SkyUI_5_2_SE-12604-5-2SE.7z`), řaďte podle nejnovějších, názvu, velikosti nebo
stavu a archiv, se kterým jste hotovi, **skryjte** - což zachová soubor a
odstraní jen řádek, takže odložit knihu neznamená spálit ji. „Show hidden" je
vrátí a totéž tlačítko skrytí zruší. „Remove N installed" smaže archivy módů,
které jste už nainstalovali, na dvě kliknutí, a jen ty **na obrazovce**: filtrem
jste řekli, které jste mysleli.

### Kolekce z Nexusu

Vložte odkaz na kolekci - nebo na něj klikněte na webu - a Eidos vypíše členy
dané revize, každého spárovaného s touto instancí: nainstalovaný, stažený, nebo
chybějící. Kolekci **čte**; neinstaluje ji, a panel to říká. Instalátor tu dělají
spíš nepoctivým než jen obtížným čtyři věci: členové jsou obyčejné soubory
z Nexusu, které potřebují klíč pro každý soubor zvlášť, jejž mimo vlastní
tlačítko webu dokáže vyrobit jen prémiový účet; plná instalace jsou tři volání
API na člena proti rozpočtu, který tento klient odmítá přečerpat; fáze, pravidla
a přehrané odpovědi FOMOD z manifestu se nepodařilo ověřit proti skutečné
publikované kolekci pro hru od Bethesdy a hádání vyrobí pořadí načítání, které
vypadá správně a správně není. Čtení stojí jeden požadavek a je přesné.

Kolekci lze číst jen proti **její vlastní hře**. Otevřete kolekci pro Skyrim
s načtenou instancí Fallout 4 a Eidos ji jmenovitě odmítne, místo aby členy
spároval s nesprávným seznamem módů, kde by každé „nainstalovaný" a každé
„chybějící" byl šum ve tvaru odpovědi.

### Režim offline

**Settings -> Nexus -> Offline** Eidosu zcela zabrání kontaktovat Nexus.
Kontroly aktualizací, přihlášení, stahování a kolekce to řeknou, místo aby
selhaly chybou spojení. Je vypnutý, dokud jej nezapnete - soubor nastavení
zapsaný starším Eidosem takový klíč nemá a číst chybějící jako „zapnuto" by
odřízlo síť každému, kdo aktualizuje.

**Preferred servers** řadí uzly CDN, které stahování upřednostňuje, nejlepší
první. Více než jedno zrcadlo na výběr dostane jedině prémiový účet, takže pro
všechny ostatní vybírá Nexus a tohle nemění nic. Je to řazení, ne filtr: pokud
dnes nic z toho, co jste jmenovali, není v nabídce, stahování stejně proběhne,
z uzlu, který Nexus nabídl jako první.

**Categories** jsou upravitelné, nejen zobrazované: přiřaďte je jednomu módu
nebo celému výběru, upravte z téhož dialogu i samotný katalog a stáhněte
oficiální seznam kategorií hry z Nexusu. Oba soubory katalogu jsou vlastní
soubory MO2 (`categories.dat` a `nexuscatmap.dat`), takže sdílená instance drží
jeden katalog.

**View -> INI editor** upravuje herní INI soubory profilu - tu kopii, která
přetrvá, ne tu zahrabanou v prefixu Protonu, která se při každém spuštění
přepíše. **View -> Log** čte logy relací. **View -> Extensions** vypisuje vaše
vlastní doplňky; viz [extensions.cs.md](extensions.md).

Instalace přijme všechno: cesty Simple a FOMOD, plus balíčky **BAIN** z Wrye
Bash (zaškrtnete podbalíčky, které se slučují v pořadí) a **ruční** výběr, který
ukáže strom archivu a nechá vás ukázat na kořen dat, když rozvržení nerozpozná
žádná heuristika. Žádný archiv není odmítnut.

**Diagnostics** provádí živé kontroly zdraví: především schopnost spustit hru,
chybějící mastery (nejspolehlivější jediný předpovídač pádů), archivy, které
nenačte žádný aktivní plugin, jestli seznam módů pořád odpovídá složce mods, a -
po běhu - co vlastní log script extenderu říká o každé z jeho pluginových DLL,
což z otázky „načetly se moje SKSE pluginy?" dělá místo dohadu důkaz.

Chcete-li hru spouštět skrz GUI, nastavte parametr spuštění hry ve Steamu na
absolutní cestu k binárce (Steam nevidí `~/.cargo/bin` v PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos se otevře na instanci té hry - té, kterou jste použili naposledy, takže
přenosná instance se najde znovu stejně jako globální; kliknutím na Run ji
spustíte skrz sloučený pohled. (Tlačítko Run ukáže přesně tento řádek, se
skutečnou cestou ke spuštěné binárce, pokud jej stisknete mimo Steam.)

`%command%` ze Steamu u titulů Bethesdy obvykle ukazuje na
`<Game>Launcher.exe`. Eidos jej nikdy nespouští: launcher je samostatná
aplikace nastavení, která znovu prohledá `Data` a přepíše `plugins.txt`, čímž
zruší právě nasazené pořadí načítání. Místo něj dosadí zavaděč script
extenderu, pokud je nainstalovaný, jinak binárku hry, a řekne to, když musí
ustoupit - hra, která nastartuje s každým SKSE módem netečným, je horší než ta,
která nenastartuje.

Starší instrukce tady vynucovaly `WINEDLLOVERRIDES="d3dcompiler_47=n"`. To už
není potřeba a nikdy to nebylo úplně správně: přepsání na *native* pomůže jen
tehdy, když je pravá `d3dcompiler_47.dll` už v prefixu. Eidos teď prochází
importy DLL zapnutých módů, sám nasadí skutečnou DLL od Microsoftu a teprve pak
nastaví přepsání.

## Vyzkoušejte důkaz konceptu

Není potřeba žádná hra. Dokazuje sjednocení + copy-on-write + nulový zásah +
rozsah na jmenný prostor jen pomocí neprivilegovaného OverlayFS v uživatelském
jmenném prostoru (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Nástroje

xEdit, BodySlide, DynDOLOD a spol. běží skrz sloučený pohled uvnitř prefixu
Protonu dané hry:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

Jedna věc, kterou je dobré vědět před pojmenováním nástroje: **název rozhoduje,
které běhové DLL mu Eidos poskytne** - `BodySlide` dostane své knihovny DirectX,
`BS` nedostane nic. V GUI dialog Executables ukazuje pod polem skutečný stav
každé podmínky a ty chybějící jsou tlačítka.

Tabulka, tři úrovně podmínek, proč DynDOLOD potřebuje běhové prostředí .NET,
které winetricks neumí nainstalovat, a proč se nástroj nainstalovaný jako mód
spouští ze sloučené cesty místo z vlastní složky, jsou v
[tools.cs.md](tools.md).

Sestavení ze zdrojových kódů a rozvržení repozitáře jsou v
[../internals/contributing.md](../../../../internals/contributing.md).

## Rozšíření

Eidos lze rozšířit, aniž byste jej znovu sestavovali: manifest TOML v
`~/.config/Colony/Eidos/addons/` přidá nástroj do seznamu Extensions nebo
kontrolu do záložky Health. Do Eidosu se nic nenačítá - rozšíření je program,
který spustí. Viz [extensions.cs.md](extensions.md).
