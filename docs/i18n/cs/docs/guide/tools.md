<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Nástroje: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Nástroj spuštěný skrz Eidos vidí sloučený pohled, uvnitř prefixu Wine hry (prefix
Proton ze Steamu, nebo prefix, který nastavil Heroic). Čte to, co bude číst hra
(každý zapnutý mód, v pořadí priority), a cokoli zapíše, přistane v Overwrite, kde
se to jedním kliknutím promění ve skutečný mód. Nástroj s `runner=native` běží bez
Protonu; viz [níže](#proměnné-prostředí-a-nástroje-spuštěné-bez-protonu).

## Ty, které si Eidos najde sám

Některé nástroje mají dost jedinečný název na to, aby se daly najít místo
deklarovat, a xEdit je ten zjevný případ: `FO4Edit.exe` pro Fallout 4,
`SSEEdit.exe` pro Skyrim SE, `TES5Edit.exe` pro původní díl a tak dále. Ke každému
patří záznam QuickAutoClean, který čistí dirty edits, na které LOOT upozorňuje.
`PGPatcher.exe` se najde stejně u každé hry od Bethesdy kromě Morrowindu a přichází
s argumentem, bez kterého nemůže pracovat (viz
[níže](#proč-pgpatcher-potřebuje---ignore-mo2vfscheck)). Eidos je hledá podle názvu
souboru v:

- `Overwrite/Root/`, ve složce hry, ze které instance běží (instalace z obchodu
  nebo její kopie hry), a ve stromech `Root/` zapnutých módů;
- `mods/` této instance, kam si nástroje instalují uživatelé MO2;
- složce s nástroji, kterou nastavíte v Preferences (General -> Tools), pro
  adresář sdílený mezi instancemi, například `/mnt/Games/Tools`.

Obvyklé generátory se najdou stejným způsobem, každý u her, které podporuje:
Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen a Synthesis. Stejně i
BethINI Pie (`Bethini.exe`) u stejných her jako PGPatcher; viz
[BethINI Pie](#bethini-pie). Ve Stardew Valley se stejně najde SMAPI
(`StardewModdingAPI.exe`). Je to i loader script extenderu té hry, takže hru
spouští přes obchod, jak popisuje
[Jak se k nástroji dostat](#jak-se-k-nástroji-dostat-skrýt-připnout-a-zástupce-na-ploše).

Seznam je zvlášť pro každou hru, takže instanci Skyrimu se nikdy nenabídne
editor Falloutu. Hledání se zastaví čtyři úrovně hluboko, protože pool módů jsou
statisíce souborů a tohle běží pokaždé, když se sestavuje seznam nástrojů, a
nenásleduje symlinky. Nástroj nalezený tímto způsobem je nastavený přesně jako
ten, který jste zadali sami: jeho prerekvizity, argumenty (`--ignore-mo2vfscheck`
u PGPatcheru) a mód pro zachycení výstupu plynou z jeho názvu, podle pravidel
níže.

Pokud je nástroj někde jinde nebo chcete jiné argumenty, přidejte ho ručně.
Uživatelská položka se stejným názvem přebije cokoli nalezeného automaticky.

## Čištění pluginu, který LOOT označí jako špinavý

Po řazení LOOTem záložka Diagnostics vypíše pluginy, které LOOT označí jako špinavé,
u her, které mají jak xEdit, tak čisticí data LOOTu (Skyrim LE, SE a VR, Enderal SE,
Fallout 3, New Vegas, Fallout 4 a 4 VR, Oblivion, Starfield). Když Eidos najde xEdit,
zaregistrovaný v Executables nebo kdekoli, kde v instanci nástroje hledá, karta má
Clean all a každý špinavý plugin má v nabídce řádku Clean with QuickAutoClean; jinak
karta řekne, aby se xEdit přidal v Executables. Eidos xEdit nestahuje. Spouští režim
QuickAutoClean xEditu na jednom pluginu najednou, přes sloučený pohled, s
`-qac -autoexit -autoload`. xEdit uloží vyčištěný plugin přes ten, který načetl, což
je ve sloučeném pohledu nový soubor v Overwrite; Eidos ho přesune do módu
`Cleaned plugins`, vytvořeného zapnutého na konci seznamu, kde vyhrává každý
konflikt. Plugin v jeho vlastním módu nebo ve složce Data hry se nikdy nezapíše.
Vypněte `Cleaned plugins`, nebo v něm skryjte jeden soubor, a čištění se vrátí.
Plugin, který leží v Overwrite, se nečistí, protože by xEdit přepsal ten soubor
sám; nejdřív z Overwrite udělejte mód.

Zálohy, cache a log xEditu jdou do `<instance>/logs/qac-<time>/`, mimo sloučený
pohled, a stavový řádek hlásí počty z toho logu. Eidos tu složku po úspěšném
čištění smaže a po neúspěšném ponechá. Eidos zaznamenává velikost a datum pluginu,
ze kterého byla každá kopie vyrobena, do `Cleaned plugins/meta.ini`. Když se ten
plugin změní, záložka Diagnostics ukáže **Out-of-date copy of <plugin>** s tlačítkem,
které starou kopii skryje.

## Přidání nástroje

V GUI: **Tools** v řádku nabídek, nebo **Executables** na liště nástrojů, pak Add. Z příkazové
řádky:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # vypsat, co je zaregistrováno
eidos tool skyrimse run BodySlide         # spustit ho skrz sloučený pohled
eidos tool skyrimse run BodySlide --print # ukázat příkaz bez spuštění
eidos tool skyrimse rm BodySlide          # odebrat položku, kterou jste přidali
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # spustí exe jednou, bez registrace
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` zachytí výstup tohoto spuštění do `mods/<mod>`, ať nástroj
nastavuje cokoli. `rm` odebere jen položky, které jste přidali; výchozí položku hry
lze skrýt, ne odebrat.

Exe spuštěné přes `--exe`, jehož název souboru Eidos pro danou hru zná, bez ohledu na
velikost písmen, dostane název toho nástroje a s ním prerekvizity, argumenty a mód
pro zachycení níže. `GenerateFNISforUsers.exe` je FNIS, takže řádek s `--exe` výše
žádá `dotnet48` a zachytává do `FNIS Output`. Jakékoli jiné exe se pojmenuje podle
názvu souboru. V záložce Data má řádek `.exe` tlačítko **Add tool**, které otevře dialog
Executables s vyplněnou položkou, a **Run**, které soubor spustí jednou jako `--exe`.
Obě pojmenují soubor stejně.

Pro příkaz, který není nástroj pro Windows, `eidos shell skyrimse` otevře váš
`$SHELL` (`/bin/sh`, když není nastaven) ve složce hry s připojeným sloučeným
pohledem. Co shell zapíše, skončí v Overwrite, a opuštěním shellu se pohled odpojí.
`eidos shell skyrimse -- <command>` místo toho spustí jeden příkaz. Ctrl+C v
interaktivním shellu zastaví příkaz, který jste napsali, a nechá shell otevřený.
Shell běží bez Protonu; program pro Windows jde přes `eidos tool`.

V okně **Stop** ukončí běh nástroje, který visí: stiskněte ho dvakrát a Eidos ukončí
programy toho běhu a pak naloží s jeho soubory jako po každém ukončení. Se zapnutým
**Notify the desktop when a long job ends** (Preferences, General, Running a game)
pošle běh nástroje, který skončí, zatímco je okno na pozadí, oznámení na plochu.

Script extender, binárka hry a launcher se detekují automaticky; registrovat je
potřeba jen nástroje navíc. U instance, která běží z kopie hry, se detekují v kopii;
viz [Nástroje na kopii hry](#nástroje-na-kopii-hry).

### Nasměrujte ho na skutečný soubor, ať je kdekoli

Zaregistrujte spustitelný soubor tam, kde doopravdy leží. Pokud byl nástroj
nainstalovaný jako mód, je to uvnitř složky módu:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

To je cesta globální instance. U přenosné instance platí totéž pravidlo pod její
vlastní složkou, `<instance>/mods/...`. Absolutní cesta jako tahle je jediná věc,
která nepřežije pozdější přesun přenosné složky.

Eidos tuhle cestu před spuštěním přepíše na tu sloučenou, takže nástroj běží z
`<game>/Data/CalienteTools/BodySlide/` a vidí tam i soubory všech ostatních módů.
BodySlide přináší prázdný adresář `SliderSets` a každé tělo, které umí postavit,
pochází z CBBE a z módů s oblečením. Spuštěn ze své vlastní složky módu nenajde nic
a vypadá rozbitě.

MO2 dělá totéž přepisování, ze stejného důvodu - jeho vlastní komentář jmenuje
FNIS.

Nástroj ve vypnutém módu přepsat nelze, protože jeho soubory nejsou ani v pohledu.
Eidos to řekne a spustí ho z jeho vlastní složky.

## Nástroje ze seznamu Mod Organizer 2

Seznam Wabbajack nebo jiná instance MO2 otevřená jako instance Eidos si ponechává svůj
`ModOrganizer.ini` a spustitelné soubory nastavené v něm lze zkopírovat do
`tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # ukáže, co by se přidalo
eidos tool /path/to/list import-mo2 --write   # přidá je
```

V okně má dialog Executables tlačítko **Import from ModOrganizer.ini** a u každé položky
políčko, které lze odškrtnout. **Add selected** přidá zaškrtnuté položky do seznamu a
nic se neuloží, dokud nestisknete Save. Počítá se jen `ModOrganizer.ini`, který
nezapsal Eidos; ten, který zapisuje Eidos, popisuje
[PGPatcher a PGLite](#pgpatcher-a-pglite-v-režimu-mod-organizer-2).

Cesty na `Z:` jsou cesty Linuxu a zůstávají beze změny. Cesta na jiném písmenu
jednotky se přesune do složky seznamu, pokud tuto složku jmenuje
(`D:\Lists\MyList\tools\SSEEdit.exe` se stane `<list>/tools/SSEEdit.exe`), ve
spustitelném souboru, pracovní složce i argumentech. Loader script extenderu,
launcher a binárka hry se změní na prosté názvy, takže se spouštějí ze složky hry,
přes kterou Eidos připojuje pohled, stejně jako vestavěné položky. `hide` z MO2 se
stane **Hide from picker** a `steamAppID` se stane **Steam AppID**. Položky mířící jinam
(systémový program Windows, nástroj mimo seznam) se vypíšou jako přeskočené s
důvodem a název, který už v `tools.ini` je, se nikdy nenahradí.

Importovaná položka si ponechá název a argumenty z MO2 a Eidos z jejího názvu nic
nedoplní: žádné prerekvizity, žádný další argument a žádný mód pro zachycení výstupu.
Nastavte je v dialogu Executables. U položky PGPatcheru ze seznamu vytvořeného ve
Windows obvykle chybí `--ignore-mo2vfscheck`; přidejte ho (viz
[níže](#proč-pgpatcher-potřebuje---ignore-mo2vfscheck)).

Importovaná cesta mířící do složky `Stock Game` seznamu se přesune na instalaci z
obchodu jen tehdy, když instance běží z té kopie; viz další část.

## Nástroje na kopii hry

Když instance běží z kopie hry (složka `Stock Game` nebo `Game Root` seznamu, nebo
zmrazená kopie; viz
[Game files](usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie)), nástroj vidí soubory
kopie, ne instalace z obchodu. Sloučený pohled je stále připojen na instalaci z
obchodu: kořenový pohled má kopii na dně a pohled Data čte `Data` kopie. Eidos hledá
script extender, launcher, binárku hry a známé nástroje v kopii místo v instalaci z
obchodu.

Cesta nástroje, argument nebo pracovní složka mířící do kopie se před během přesune
na instalaci z obchodu. Může být zapsána jako cesta Linuxu nebo jako cesta Wine
`Z:`: `-D:"Z:\...\Stock Game\Data"` se stane stejnou cestou pod instalací z obchodu.
Položky importované z `ModOrganizer.ini` seznamu to potřebují. Když instance běží z
instalace z obchodu, nic se nepřesouvá, takže taková položka čte holou kopii bez
vašich módů.

`installed path` hry v registru prefixu a `gamePath` v `ModOrganizer.ini`, který
zapisuje Eidos, stále jmenují instalaci z obchodu, kde je sloučený pohled. Když
složka kopie chybí nebo neobsahuje binárku hry, běh nástroje se zastaví, jako start
hry, a zpráva jmenuje `eidos stock-game <instance> store`, kterým se vrátíte k
instalaci z obchodu.

`eidos stock-game <instance>` ukáže, které soubory hry instance spouští a na které
kopie by mohla přepnout. `use <folder>` a `store` přepínají a `freeze --yes` zkopíruje
instalaci z obchodu do složky `Stock Game` instance a spouští z ní. Karta Game files v
záložce Diagnostics dělá totéž; viz
[Game files](usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie).

## Posílání výstupu nástroje do jeho vlastního módu

Generátor (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) zapíše stovky souborů. Ve
výchozím stavu přistanou v Overwrite se vším ostatním. Nastavte **Capture output into** v
editoru Executables a výstup tohoto běhu půjde místo toho do zvoleného módu:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide a Synthesis přijdou s už nastaveným cílovým módem:
`Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` a
`Synthesis Output`. DynDOLOD, TexGen a xLODGen přijdou bez něj, protože zapisují do
výstupní složky, kterou jim zadáte.

Mód se vytvoří, pokud neexistuje. Přesunou se jen soubory, které vyrobil tento běh;
cokoli už v Overwrite bylo, tam zůstane, takže dva nástroje s cílem pro zachycení si
výstup navzájem nekradou. Běh, který nic nezapsal, po sobě nenechá prázdný mód.

Eidos přesouvá soubory po běhu. MO2 naopak nasměruje zapisovací vrstvu na mód, což ho
na celý běh povýší na nejvyšší prioritu: každý konflikt, ve kterém je, se převrátí a
potom zase zpět. Zápisy také jdou rovnou skrz vlastní soubory módu bez copy-up.
Zachycení dosáhne stejného koncového stavu bez jednoho i druhého.

Pokud je cílový mód vypnutý, výstup se zapíše, ale hra ho neuvidí, takže by nástroj
při dalším běhu vygeneroval tytéž soubory znovu. Eidos na to upozorní.

### Když je její výstup zastaralý

Eidos si zaznamenává, z jakých módů, pořadí pluginů a profilu vznikl výstup nástroje,
velikost a datum spustitelného souboru nástroje a cestu a argumenty položky v
`tools.ini`. Když se něco z toho změní, záložka Diagnostics ukáže **Generated output
inputs changed** a karta má tlačítko **Run <tool> again**, které nástroj spustí s jeho
současným nastavením.

U generátoru, jehož zastaralý výstup rozbije hru, třeba Pandory nebo Nemesis, přidejte
k jeho záznamu v `tools.ini` řádek `check_before_play=1`, nebo v dialogu Executables
zapněte **Ask before playing when its output is out of date**. Run se pak nejdřív zeptá:
spustit nástroj, nebo přesto hrát. Volba Play anyway platí, dokud se módy, pluginy
nebo profil znovu nezmění, nebo dokud se okno nezavře. Eidos nástroj nikdy nespouští
sám. Se zapnutým **Start the game at once when a store opens Eidos** taková otázka nechá
okno otevřené místo spuštění hry.

## Prerekvizity a argumenty se berou z názvu

Název, který nástroji dáte, rozhoduje o tom, jaké běhové prerekvizity mu Eidos
zajistí. Porovnává se podřetězec názvu bez ohledu na velikost písmen.

| Pokud název obsahuje | Eidos vyžádá |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| cokoli jiného | nic |

Takže nástroj zaregistrovaný jako `BodySlide` dostane své DirectX DLL; tentýž
spustitelný soubor zaregistrovaný jako `BS` nedostane nic a může selhat při startu s
chybou, která o DLL neříká nic. Pojmenovávejte nástroje podle programu.

Seznam je v `default_prereqs` (`crates/eidos-instance/src/tools.rs`) a pole `Prereqs`
v dialogu Executables je editovatelné: název jen doplní výchozí hodnotu.

Název rozhoduje o argumentech stejně, u jediného nástroje, který nějaký potřebuje a
nikdo by ho neuhodl: název obsahující `pgpatcher` (nebo `parallaxgen`, jeho dřívější
jméno) dostane `--ignore-mo2vfscheck`. Proč a proč není volitelný, je
[níže](#proč-pgpatcher-potřebuje---ignore-mo2vfscheck). Mód pro zachycení výstupu se také
bere z názvu (viz [výše](#posílání-výstupu-nástroje-do-jeho-vlastního-módu)).

### Tři druhy prerekvizit

Úroveň 1 jsou přibalená DLL: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43` a
`d3dx11_43`. Eidos je nese uvnitř svých binárek a při spuštění je kopíruje do
prefixu. V 64bitovém prefixu, výchozím u Protonu, jde 64bitová verze do `system32` a
32bitová do `syswow64`. Nic není třeba dělat, žádná síť. Každá verze je uložena
komprimovaně gzipem, takže osm souborů zabere v každé binárce 7,9 MB místo 17,9 MB a
rozbaluje se jen ten, který se kopíruje (nanejvýš 4,7 MB).

Úroveň 2 jsou instalátory: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` a `xact_x64`. Ty zapisují klíče
registru, GAC a CLR hosty, takže je nelze zkopírovat jako soubory. Tři z nich Eidos
instaluje bez winetricks: `vcrun2022` spustí `VC_redist.x86.exe` a
`VC_redist.x64.exe` od Microsoftu a `dotnet8` a `dotnetdesktop8` spustí instalátor
.NET 8.0.12 Desktop Runtime, který obsahuje i běhové prostředí .NET. Balík vydání
(`eidos-linux-bundle.tar.gz`) tyto instalátory přibaluje ve složce `runtimes/` a
`install.sh` je zkopíruje do `~/.local/share/Colony/Eidos/runtimes/` pro uživatele,
který ho spouští, i s `--system`. Eidos hledá každý soubor nejdřív tam, pak ve složce
`runtimes/` vedle binárky `eidos` (rozbalený balík spuštěný na místě). Bez žádné z
těchto kopií (samostatný soubor `eidos-linux`, sestavení ze zdrojů nebo jiný
uživatel systémové instalace) je Eidos stáhne od Microsoftu
(`download.visualstudio.microsoft.com`) do `~/.local/share/Colony/Eidos/runtimes/` a
ponechá tam. V obou případech je soubor odmítnut, pokud jeho SHA-256 neodpovídá
hodnotě zabudované v Eidosu, a běží bez okna (`/install /quiet /norestart`) v prefixu
hry se sestavením Protonu nebo Wine, které spouští nástroje hry. Stejně jako winetricks
nastaví `vcrun2022` nejdřív knihovny DLL Visual C++ v přepisech DLL prefixu na
`native,builtin`. Potom, u sestavení Protonu, Eidos ověří, že běhové prostředí C++ v
prefixu není napůl od Microsoftu a napůl od Protonu. Pokud je, instalace selže: zpráva
jmenuje dvojice DLL pocházející z různých sestavení a říká, aby se místo toho
`vcrun2022` nainstaloval přes winetricks, který `msvcp140.dll` nahradí sám. Instaluje
se jen 64bitový .NET 8; 32bitový nástroj pro .NET 8 dál potřebuje
`winetricks dotnetdesktop8`.

Ostatních šest verbů jde přes systémový `winetricks`, který musí být v `PATH`. Eidos
ho spouští s `wine` ze sestavení Protonu nebo Wine, které spouští nástroje hry, proti
prefixu hry, čímž obchází kontejner pressure-vessel Steamu a nesoulad protontricks +
Proton-GE. Některé verby winetricks potřebují `cabextract` a Eidos varuje, když chybí.
Nástroj, který deklaruje nenainstalovaný verb úrovně 2, se přesto spustí, s varováním
jmenujícím verb a příkaz na nápravu, protože jste ho mohli nainstalovat jinak.

Verb úrovně 2 se počítá za hotový, když ho uvádí vlastní `winetricks.log` prefixu, ať
winetricks nebo protontricks spustil kdokoli, nebo když ho Eidos nainstaloval do
téhož prefixu. Vlastní instalace Eidos zaznamenává do `prereqs.done` v instanci,
vázané na cestu a inode prefixu, takže znovu vytvořený prefix ukáže své verby opět
jako nevyřízené. **Rebuild prefix**, akce v Diagnostics pro kopii ze Steamu, přejmenuje
`compatdata/<appid>` na `<appid>.eidos-backup-<time>` a nic nemaže; Steam vytvoří nový
prefix při příštím spuštění hry.

Úroveň 3 jsou runtimy `dotnet10` a `dotnetsdk10`. Moderní běhové prostředí .NET je 193
souborů, které žijí ve vlastním adresáři a hledají se přes `DOTNET_ROOT`. Nikdy se
neregistruje a do prefixu se neinstaluje, takže ho neunese ani jedna z ostatních
úrovní. Eidos ho drží mimo jakoukoli instanci, protože 78 MB není zvlášť pro každou
hru ani pro každý profil: každý runtime se rozbalí do
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos hledá zip runtimu pod jeho názvem ke stažení ve
stejných dvou složkách jako instalátory úrovně 2 a rozbalí kopii, kterou přibaluje
balík vydání (`dotnet-runtime-10.0.10-win-x64.zip`, jen pro `dotnet10`), nebo ji
stáhne. Balík nepřibaluje `dotnet-sdk-10.0.403-win-x64.zip` SDK, ale kopie tam
vložená ručně se použije a ušetří stahování. Eidos odmítne kterýkoli soubor, jehož
SHA-256 neodpovídá hodnotě zabudované v Eidosu. Zip rozbaluje přes `7z`, tentýž 7-Zip,
který používá pro archivy módů.

`dotnetsdk10` je .NET 10 SDK, stahování 301 MB, po rozbalení 808 MB. Synthesis ho
potřebuje ke kompilaci svých patcherů a k jejich sestavení a spuštění volá holé
`dotnet`, takže nástroj, který deklaruje `dotnetsdk10`, dostane navíc složku SDK v
`WINEPATH` a k tomu `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` a `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Sestavení patcheru by po sobě nechalo běžet uzly MSBuild a server VBCSCompiler, které
pod Wine drží wineserver prefixu naživu, proto nástroj dostane i
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` a
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` dovolí patcheru sestavenému
pro .NET 8 (Mutagen před 0.54) běžet na runtime .NET 10 ze SDK, když se žádný runtime
.NET 8 nenajde.
Záznam Synthesis v `tools.ini`, který má pořád starou výchozí hodnotu `dotnet8`,
`vcrun2022`, dostane při načtení Eidosem `dotnetsdk10`, `vcrun2022`.

NuGet ověřuje podpis každého balíčku, který obnovuje, proti úložišti kořenových
certifikátů Windows, a v prefixu Wine chybí kořenové certifikáty pro podepisování
kódu a časová razítka, které SDK dodává jako
`sdk/10.0.403/trustedroots/codesignctl.pem` a `timestampctl.pem`. Bez nich obnova
`Mutagen.Bethesda.Synthesis` 0.36.6 selže s chybami `NU3018` a `NU3028`. Proto než se
spustí nástroj, který deklaruje `dotnetsdk10`, a jakmile je SDK stažené, Eidos přidá
385 certifikátů z obou souborů do úložiště kořenových certifikátů aktuálního
uživatele v prefixu hry, a to ve stejném importu přes `regedit`, který zapisuje cestu
ke hře (viz níže). Pokud některý z nich zmizí, importuje je znovu. Těmto kořenovým
certifikátům pak důvěřuje každý program pro Windows v tomto prefixu. S nimi v novém
prefixu a s prázdnou mezipamětí NuGet stejná obnova i sestavení projdou se
systémovým Wine 11.21 i s GE-Proton11-9.

Eidos nenainstaluje nic z úrovně 2 ani 3, dokud o to nepožádáte:

```sh
eidos prereqs skyrimse            # ukázat, co registrované nástroje potřebují, a jejich stav
eidos prereqs skyrimse --install  # nainstalovat, co chybí (nejdřív kopie z balíku, pak stahování)
```

V GUI sedí tytéž stavy pod polem Prereqs a chybějící jsou tlačítka. Verb, který není
ani přibalený, ani runtime, ani verb úrovně 2, je nahlášen jako pravděpodobný
překlep, místo aby byl nabídnut ke stažení.

`eidos prereqs <instance> --install` odmítne běžet, dokud prefix drží hra, Steam nebo
nástroj. Runtimy úrovně 3 se nainstalují i předtím, než prefix existuje. Instalátory,
které Eidos spouští sám, potřebují existující prefix: dokud obchod hru jednou
nespustil, zastaví se s „start the game once through its store, then install again".
Verby úrovně 2 se instalují po jednom a každý úspěch se zaznamená, takže po selhání
stejný příkaz pokračuje. U kopie z GOG nebo Epicu bez nastavení Heroicu `--install`
zkopíruje přibalená DLL a pak se zastaví před verby úrovně 2: řekne, aby se použil
vlastní winetricks toho runneru, a při další kontrole přečte `winetricks.log` prefixu.

### Proč DynDOLOD potřebuje `dotnet10`

DynDOLOD nestaví object LOD sám: volá LODGen a přináší tři jeho verze.
`LODGenx64.exe` cílí na .NET Framework 4.8, který je pod Protonem směrován na Wine
Mono, jehož inicializátor `System.Uri` volá metodu, kterou Mono neimplementuje. LODGen
umře dřív, než odvede první kus práce, a nechá po sobě log s hlavičkou verze a ničím
dalším a dialog DynDOLODu, který říká jen „failed for one or more worlds".

Instalace skutečného .NET Frameworku to nespraví: Proton nahradí `mscoree.dll` -
zavaděč, který by ho našel - symlinkem do vlastního stromu, a dělá to znovu při každé
aktualizaci prefixu.

Verze, která funguje, je `LODGenx64Win10.exe`, která cílí na moderní .NET a `mscoree`
se vůbec nedotkne. Nasměrujte `DOTNET_ROOT` na runtime .NET 10 a poběží. To je to, co
`dotnet10` zajistí, a Eidos tu proměnnou nastavuje při spouštění každého nástroje,
který ji deklaruje.

### Proč Pandora potřebuje `dotnet10`

Pandora je aplikace Avalonia, ne WPF, a od Pandory 4.1.0 cílí na `net10.0`, takže ji
spustí samotný runtime .NET 10. Když žádný runtime nenajde, řekne, aby se
nainstaloval .NET Desktop Runtime. To je obecný text, který launcher .NET ukazuje u
každého grafického programu, který runtime nenajde, a `dotnetdesktop8` to nespraví.

Eidos 1.21.0 a starší dával Pandoře `dotnetdesktop8` a záznam si ponechává svůj řádek
`prereqs=`. Když Eidos načte záznam Pandory, který pořád uvádí jen `dotnetdesktop8`, a
`.runtimeconfig.json` vedle jejího spustitelného souboru jmenuje `net10.0`, dostane
záznam místo toho `dotnet10`. Pandora 2.x cílí na .NET 8 a ponechá si, co má.

### Proč PGPatcher potřebuje `--ignore-mo2vfscheck`

PGPatcher přepisuje sítě a pluginy tak, aby vaše módy textur (vanilla, parallax,
complex material, PBR) používaly na každém povrchu správný shader. Právem odmítá běžet
mimo správce módů, protože zaplatovat holou složku hry by bylo zhoubné.

PGPatcher kontroluje usvfs, vrstvu zachytávání DLL, kterou MO2 vstřikuje do hry. Eidos
žádnou nemá: sloučený pohled je připojení FUSE, do kterého Wine vstupuje, aniž by o
něm věděl. Kontrola tedy nic nenajde a nástroj okamžitě skončí s

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

a jmenuje program, který uživatel vůbec nespouští, ještě před jakoukoli prací.
Odpovědí samotného autora na dvě linuxová hlášení ([#716][pg716] a [#725][pg725], obě z
Fluorine, linuxového forku MO2, který ze stejného důvodu stojí na FUSE) je přepínač,
který kontrolu přeskočí.

Eidos ho proto předvyplní. PGPatcher nalezený ve fondu módů přichází s už nastaveným
argumentem a napsání `PGPatcher` do názvu v dialogu Executables vyplní pole Arguments
stejně. To, co napíšete, vždy vyhrává: předvyplní se jen pole, které jste nechali
prázdné, přesně jako u prerekvizit výše.

Seznam je v `default_args` (`crates/eidos-instance/src/tools.rs`), vedle
`default_prereqs`.

Jeho runtime nevyžaduje nic, což je u nástroje .NET neobvyklé: nese si vlastní
(`dotnetlib/`, s `runtimeconfig.json` deklarujícím `includedFrameworks`) a dodává
knihovny CRT od Microsoftu vedle spustitelného souboru. Nechte Prereqs prázdné.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher a PGLite v režimu Mod Organizer 2

S Mod Manager nastaveným na `None` vidí PGPatcher jedinou sloučenou složku `Data` a
nepozná, ze kterého módu pochází mesh nebo textura, takže nemá žádné konflikty k
zobrazení ani pořadí módů, kterým by se řídil. Jeho režim `Mod Organizer 2` čte
samotnou instanci: soubor `modlist.txt` profilu, zapnuté módy v pořadí priority a
soubory ve složce každého módu. PGLite, verze běžící ve hře, čte instanci stejně.
Instance Eidos už má rozložení MO2 (`mods/`, `profiles/<name>/modlist.txt`,
`overwrite/`), takže tento režim s ní funguje.

V launcheru PGPatcheru:

1. Nastavte **Mod Manager** na `Mod Organizer 2`.
2. Nastavte **MO2 Options -> Instance Location** na složku instance Eidos, zapsanou jako
   cesta Windows na disku `Z:` ve Wine, kterým je `/`. Přenosná instance v
   `/mnt/Games/Eidos-Skyrim` je `Z:\mnt\Games\Eidos-Skyrim`; globální je
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Ponechte `--ignore-mo2vfscheck` v argumentech nástroje. V tomto režimu PGPatcher
   hledá usvfs a skončí, když chybí.

PGPatcher přijme složku jen tehdy, když obsahuje `modorganizer.ini`, a proto Eidos
zapisuje `ModOrganizer.ini` do kořene instance při každém spuštění hry nebo nástroje,
pro spouštěný profil. Soubor se přepisuje jen tehdy, když se změní jeho obsah.
Obsahuje:

- v `[General]`: `gameName` (název hry podle MO2, tedy `New Vegas` pro Fallout: New
  Vegas), `gamePath` (instalační složku hry z obchodu jako cestu `Z:`, i u instance,
  která běží z kopie hry) a `selected_profile`;
- v `[Settings]`: `mod_directory`, `profiles_directory` a `overwrite_directory` jako
  absolutní cesty `Z:`, a navíc `download_directory`, když má instance složku
  `downloads/`.

Jeho první řádek je komentář, který říká, že soubor zapsal Eidos. Soubor
`ModOrganizer.ini` bez tohoto řádku, v jakékoli velikosti písmen v názvu, se nikdy
nemění ani nemaže: instance MO2, kterou jste otevřeli v Eidosu, si ponechá svůj a
PGPatcher čte tento soubor tak, jak je. MO2 při ukládání nastavení komentáře zahazuje,
takže soubor, přes který MO2 uložil, od té chvíle patří MO2. Prázdný soubor se počítá
jako soubor Eidosu.

Takový soubor uvádí profil, který MO2 vybral naposledy, a PGPatcher použije tento
profil, ne ten, který spouští Eidos. Když se liší, Eidos při spuštění zapíše do logu
varování. Vyberte v MO2 stejný profil, nebo soubor přejmenujte či odstraňte, aby
Eidos zapsal vlastní.

Nástroje v režimu Mod Organizer 2 také čtou hru z `gamePath` v tom souboru. Když
nejmenuje ani složku hry, ze které instance běží, ani instalaci z obchodu, záložka
Diagnostics ukáže **ModOrganizer.ini points tools at another game folder** s touto cestou
a složkou hry, kterou tato instance spouští. Změňte `gamePath` v souboru, nebo soubor
odstraňte, aby Eidos zapsal vlastní.

Eidos zapisuje svůj soubor jen při spuštění něčeho, takže mezi dvěma spuštěními uvádí
profil toho posledního. Nástroj, který spustí Eidos, v něm najde svůj vlastní profil.

PGPatcher skončí s chybou, když profil nemá žádný zapnutý mód, a když je jeho výstupní
složka složkou zapnutého módu, stejně jako pod MO2.

## Vlastní programy Wine v prefixu hry

Nabídka File spouští čtyři programy Wine v prefixu hry: winecfg, regedit, správce
souborů Wine a odinstalační program. Startují jako každý nástroj, se sestavením
Protonu nebo Wine, které spouští nástroje hry, s připojenými módy, a okno na ně čeká.
**Prefix drive_c** otevře disk `C:` prefixu ve správci souborů. Řádek je šedý, dokud prefix
daný program nemá, což nastane při prvním spuštění hry z obchodu. Z terminálu totéž
udělá `eidos tool <instance> run winecfg`, a stejně `regedit`, `explorer` a
`uninstaller`, když žádný nástroj v seznamu nemá tento název.

Změna provedená v regeditu u `installed path` hry se při příštím běhu nástroje vrátí;
viz další část.

## Cesta ke hře v prefixu

Windowsové nástroje najdou svou hru přečtením
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, klíče, který zapisuje
vlastní instalátor hry. Steam pod Protonem ten instalátor nikdy nespustí. Bez klíče se
xEdit, Wrye Bash a DynDOLOD otevřou na prázdné cestě.

Před během nástroje Eidos zapíše klíč importem přes `regedit` v prefixu hry, pokud
dřívější import nezapsal stejnou cestu a prefix ji stále drží. Klíč jmenuje instalaci z
obchodu, i u instance, která běží z kopie hry, protože sloučený pohled je připojen
tam. U nástroje, který deklaruje `dotnetsdk10`, stejný import přidá i podpisové kořeny
NuGetu (viz [výše](#tři-druhy-prerekvizit)). Nástroj s `runner=native` tento
krok vynechá. Když prefix zatím neexistuje nebo ho drží jiný program, nástroj se přesto
spustí a Eidos vypíše varování, které jmenuje obchod, ze kterého hru jednou spustit,
nebo program, který zavřít. Když klíč jmenuje jinou složku než instalaci z obchodu,
záložka Diagnostics ukáže **Windows tools cannot find <game>**, dokud ho příští běh
nástroje nezapíše znovu.

## Jak se k nástroji dostat: skrýt, připnout a zástupce na ploše

Ve výchozím vybavení hry jsou nástroje, které možná nikdy nepoužijete. V dialogu
Executables:

- **Pin to top** postaví položku na začátek seznamu Run.
- **Hide from picker** ji z něj vyjme, aniž by ji smazal.
- **Desktop shortcut** zapíše `.desktop` do `~/.local/share/applications`, takže se objeví
  v nabídce aplikací a ve vyhledávání, ne na ploše. Spouští přímo
  `eidos tool <instance> run <title>`, takže nástroj naběhne skrz sloučený pohled s
  profilem této instance, aniž by okno Eidosu bylo otevřené.

Skrývání a připínání mění jen to, jak se k nástroji dostanete, takže platí i pro výchozí
nástroje dané hry, nejen pro vaše vlastní položky.

Loader script extenderu, launcher a binárka hry tlačítko Desktop shortcut nemají.
Spouštějí hru a hra se vždy spouští přes svůj obchod, což už dělá zástupce samotného
obchodu. Zástupce, který jste pro některý z nich vytvořili v Eidosu 1.22 nebo starším,
funguje dál: stejně jako `eidos tool <instance> run <title>` zadané v terminálu požádá
obchod, aby hru spustil, tak jako Run v okně, které neotevřel žádný obchod. Když
nastavení v obchodě chybí, příkaz nic nespustí a vypíše, co nastavit. Zástupce žádnou
zprávu neukáže, proto si ji přečtěte spuštěním stejného příkazu v terminálu.

Eidos takto předá obchodu každou položku, jejíž spustitelný soubor má název loaderu
script extenderu, launcheru nebo binárky hry, bez ohledu na velikost písmen: výchozí
položku, položku, kterou jste přidali, importovanou z MO2, nebo běh přes `--exe`.
Nejdřív označí tuto instanci jako naposledy použitou, aby okno, které obchod otevře,
ukázalo tuto instanci, a pak spustí `steam -applaunch <id>`,
`xdg-open heroic://launch?appName=<app>&runner=<runner>` pro Heroic, nebo
`legendary launch <app>`. S `--print` vypíše ten příkaz obchodu a nic nespustí. Příkaz
`steam` musí být v `PATH`: Steam z Flatpaku ho tam nedává, takže start selže a zpráva
řekne, aby se hra spustila ze Steamu.

Položka běží přímo skrz sloučený pohled, jako každý nástroj, když proces spustil obchod:
je nastavena jedna z proměnných `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`,
`HEROIC_APP_RUNNER` nebo `EIDOS_STORE_STARTED`. Bez žádné z nich Eidos hru přes Proton
sám nespouští. Když instance nemá loader script extenderu (ve složce hry, ze které
běží, v `Root/` zapnutého módu nebo v `Overwrite/Root/`), nic se nespustí a řádek řekne,
aby se nejdřív nainstaloval script extender. U kopie z GOG mimo Heroic nebo id obchodu,
které Eidos nedokáže předat, řekne, aby se hra spustila z toho obchodu.

Profil může mít pro hru vlastního zástupce. Klikněte na profil pravým tlačítkem a
zvolte **Desktop shortcut**: položka spustí `eidos tool <instance> run "<game>" --profile <name>`,
což z tohoto profilu udělá aktivní profil, stejně jako jeho výběr v okně, a pak požádá
obchod, aby hru spustil. Okno Eidosu, které obchod otevře, ukazuje tento profil.
`--profile` funguje s každým `eidos tool ... run` a vybere profil dřív, než se nástroj
spustí; s `--print` nevybere nic.

## Nástroj, který je vlastní aplikací na Steamu

Creation Kit je samostatná aplikace na Steamu a chce vlastní AppID; pár dalších
moddingových nástrojů dodávaných přes Steam je na tom stejně. Nastavte na položce **Steam
AppID** a Eidos ji spustí pod tímto id místo id hry.

Na Windows to znamená jiný launcher. Tady jsou to dvě proměnné prostředí u téhož běhu,
`SteamAppId` a `SteamGameId`. Eidos nastaví obě, protože Proton čte jednu a vlastní
knihovny Steamu druhou, a nástroj, který je vidí nesouhlasit, selže bez jasné chyby.
`eidos tool ... --print` ukáže přesně to, co by skutečný běh dostal.

## Proměnné prostředí a nástroje spuštěné bez Protonu

Tři klíče v `<instance>/tools.ini` zatím nemají v dialogu Executables žádné pole, takže
je do sekce nástroje dopište ručně. Každý řádek `env=NAME=value` nastaví jednu
proměnnou jen pro běhy tohoto nástroje:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos tyto řádky použije až po všem, co nastavuje sám, a po proměnných v
`launch-env.ini` profilu, takže hodnota zadaná tady nahradí obojí. To platí i pro
`WINEDLLOVERRIDES`, který Eidos skládá z DLL dodaných vašimi módy: nástroj, který ho
nastaví, v něm musí uvést každou DLL, kterou potřebuje. Obaly profilu jdou kolem
příkazu nástroje stejně jako kolem příkazu hry; viz
[Variables and wrappers per profile](usage.md#proměnné-a-obaly-pro-profil).
Jméno tvoří písmena, číslice a `_` a nezačíná číslicí. Řádek s jiným jménem se
ignoruje.

`runner=native` spustí přímo `exe`, bez Protonu. Sloučený pohled se připojí jako u
každého nástroje a řádky `arg0=`, `arg1=` jsou jeho argumenty. Použijte to pro
linuxový program nebo pro nástroj ke kopii, kterou nespravuje Steam a pro kterou Heroic
nemá zapsaný žádný runner: `exe` je Wine této kopie, program pro Windows je první
argument a prefix přijde z `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` musí být absolutní cesta. Pracovní adresář je ve výchozím stavu složka, ve které
leží `exe`, tady tedy složka runneru, a proto nastavte `workdir=`. Na sloučenou cestu
se přepisuje jen `exe`: u programu pro Windows uvnitř módu zadejte jeho cestu ve složce
`Data` hry, kde ho ukazuje sloučený pohled. Nativní nástroj vynechá kroky pro Proton:
klíč registru popsaný výše, shader cache Steamu, prázdné kořeny .NET a odstranění
overlaye Steamu z `LD_PRELOAD`.

U kopie z GOG nebo Epicu nainstalované přes Heroic takový záznam nástroje nepotřebují,
pokud nastavení hry v Heroicu (`GamesConfig/<appName>.json`) uvádí sestavení Wine nebo
Protonu: Eidos je spustí s tímto sestavením v prefixu hry z Heroicu a
`eidos prereqs <instance> --install` ho použije pro instalátory i pro winetricks.
Kopie nainstalovaná jen přes Legendary žádná nastavení Heroicu nemá.

`slr=1` spustí nástroj i import do registru před ním uvnitř kontejneru Steam Linux
Runtime, který jmenuje sestavení Protonu nástroje (`require_tool_appid` v jeho
`toolmanifest.vdf`), stejně jako Steam spouští hru. Tento runtime musí být
nainstalovaný v některé knihovně Steamu; pokud není, běh se zastaví a uvede aplikaci
Steamu, kterou je třeba nainstalovat. `eidos prereqs` dál spouští své instalátory i
winetricks na hostiteli a nástroj s `runner=native` tento klíč ignoruje. Kontejner vidí
vaši domovskou složku a knihovnu Steamu se hrou. Nástroj, mód nebo runtime kdekoli
jinde, například na jiném disku, potřebuje `env=STEAM_COMPAT_MOUNTS=/ta/cesta` (více
cest oddělte `:`). Proton nainstalovaný v `/usr/share/steam/compatibilitytools.d` s
`slr=1` nejspíš použít nepůjde, protože `/usr` uvnitř kontejneru patří runtimu. Běh
nástroje uvnitř kontejneru zatím nebyl testován.

`eidos tool <instance> run <title> --print` ukáže výsledný příkaz a každou proměnnou,
kterou běh nastaví nad zděděné prostředí, včetně těchto řádků. U položky, která spouští
hru, vypíše místo toho příkaz obchodu (viz
[Jak se k nástroji dostat](#jak-se-k-nástroji-dostat-skrýt-připnout-a-zástupce-na-ploše)).

## BethINI Pie

Eidos BethINI Pie nedodává. Vydání stažené ze stránky na Nexusu dejte do složky `mods/`
této instance nebo do složky s nástroji a `Bethini.exe` se objeví v seznamu nástrojů, u
stejných her jako PGPatcher. Spuštěný přes Eidos upravuje kopii herních INI v profilu.
Před každým během nástroje, jehož název obsahuje `bethini`, Eidos uchová bod obnovy
těchto INI; **Backups** na liště nástrojů a pak **INIs** jeden vrátí. INI, které v okamžiku
zálohy neexistovalo, zůstane tak, jak je. Dialog drží 10 nejnovějších bodů obnovy od
každého druhu a obnova nejdřív zálohuje INI tak, jak jsou, takže chybný výběr jde vrátit.

## Vlastní nastavení nástroje zůstává jeho

Eidos většinu nastavení nástrojů nekontroluje; chybné obvykle selže bez chyby.

**Game Data Path** BodySlidu (Settings) musí ukazovat na adresář `Data` hry, ne na složku
hry nad ním. Nastavený o úroveň výš hlásí dávkový build „All sets processed
successfully" a zapíše 1439 meshů tam, kde je hra nikdy hledat nebude. Eidos je zachytí
v `Overwrite/Root/`, takže instalace hry zůstane nedotčená, a jediným příznakem ve hře
je, že vaše těla nejsou postavená.

Před během BodySlidu nebo Outfit Studia stavový řádek varuje, když `GameDataPath` nebo
`OutputDataPath` ve vítězném `CalienteTools/BodySlide/Config.xml` během běhu nevede do
složky `Data` hry, a záložka Diagnostics ukáže totéž varování jako kartu s hodnotou,
kterou je třeba nastavit. Hlásí ho i `eidos check <instance>`. Cesta na `S:` se čte tak,
jak Eidos mapuje `S:` pro nástroj, tedy na složku knihovny Steamu. `OutputDataPath`
uvnitř složky `mods/` instance, pro výstupní mód, je přijat. Diagnostics také ukáže
kartu, když `Overwrite/Root/` obsahuje složku `meshes` nebo `textures`.

Výstup nástroje patří do Overwrite. Když běh vyprodukuje něco, co stojí za uchování,
**Overwrite -> Create mod...** z toho udělá běžný mód, který se dá řadit, vypnout a
odebrat jako každý jiný.
