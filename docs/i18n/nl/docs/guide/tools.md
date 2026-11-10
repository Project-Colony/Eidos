<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Een tool die via Eidos draait ziet de samengevoegde weergave, binnen het Wine-prefix
van het spel (het Proton-prefix van Steam, of het prefix dat Heroic heeft ingericht).
Hij leest wat het spel zal lezen (elke ingeschakelde mod, op prioriteitsvolgorde), en
wat hij ook schrijft, dat landt in de Overwrite, waar één klik er een echte mod van
maakt. Een tool met `runner=native` draait zonder Proton; zie
[hieronder](#omgevingsvariabelen-en-tools-zonder-proton).

## Degene die Eidos zelf vindt

Sommige tools heten uniek genoeg om gevonden te worden in plaats van opgegeven, en
xEdit is het duidelijke geval: `FO4Edit.exe` voor Fallout 4, `SSEEdit.exe` voor Skyrim
SE, `TES5Edit.exe` voor het origineel, enzovoort. Elk komt met een
QuickAutoClean-item, dat de dirty edits opschoont waarvoor LOOT waarschuwt.
`PGPatcher.exe` wordt op dezelfde manier gevonden bij elk Bethesda-spel behalve
Morrowind, en komt met een argument waarzonder het niet kan werken (zie
[hieronder](#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft)). Eidos zoekt ernaar,
op bestandsnaam, in:

- `Overwrite/Root/`, de spelmap waaruit de instantie draait (de winkelinstallatie, of
  haar spelkopie), en de `Root/`-bomen van ingeschakelde mods;
- de `mods/` van deze instantie, waar MO2-gebruikers hun tools installeren;
- de map voor tools die je in Preferences instelt (General -> Tools), voor een map die
  tussen instanties gedeeld wordt, zoals `/mnt/Games/Tools`.

De gangbare generatoren worden op dezelfde manier gevonden, elk bij de spellen die het ondersteunt: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen en Synthesis. Net als BethINI Pie (`Bethini.exe`), bij dezelfde spellen als PGPatcher; zie [BethINI Pie](#bethini-pie). Bij Stardew Valley wordt SMAPI (`StardewModdingAPI.exe`) op dezelfde manier gevonden. Het is ook de loader van de script extender van dat spel, dus het start het spel via de winkel, zoals beschreven bij [Bij een tool komen](#bij-een-tool-komen-verbergen-vastzetten-en-een-bureaubladsnelkoppeling).

De lijst geldt per spel, dus aan een Skyrim-instantie wordt nooit de editor van
Fallout aangeboden. Het zoeken stopt vier niveaus diep, omdat een modvoorraad
honderdduizenden bestanden telt en dit draait telkens als de toollijst opgebouwd
wordt, en het volgt geen symlinks. Een zo gevonden tool is precies zo ingesteld
als een die je zelf ingetypt hebt: zijn vereisten, zijn argumenten (`--ignore-mo2vfscheck`
van PGPatcher) en zijn opvangmod volgen uit zijn titel, volgens de regels hieronder.

Staat een tool ergens anders, of wil je andere argumenten, voeg hem dan met de hand
toe. Een eigen invoer met dezelfde titel overschrijft alles wat automatisch gevonden
is.

## Een plugin opschonen die LOOT als vuil markeert

Na een LOOT-sortering toont het tabblad Diagnostics de plugins die LOOT als vuil
(dirty) markeert, voor de spellen die zowel een xEdit als LOOT-opschoningsgegevens
hebben (Skyrim LE, SE en VR, Enderal SE, Fallout 3, New Vegas, Fallout 4 en 4 VR,
Oblivion, Starfield). Als Eidos xEdit vindt, geregistreerd in Executables of ergens
waar het in de instantie naar tools zoekt, heeft de kaart Clean all en heeft elke
vuile plugin Clean with QuickAutoClean in het menu van zijn rij; anders zegt de kaart
dat je xEdit in Executables moet toevoegen. Eidos downloadt xEdit niet. Het draait de
QuickAutoClean-modus van xEdit op één plugin tegelijk, via de samengevoegde weergave,
met `-qac -autoexit -autoload`. xEdit slaat de opgeschoonde plugin op over de plugin
die het laadde, en in de samengevoegde weergave is dat een nieuw bestand in de
Overwrite; Eidos verplaatst het naar een mod met de naam `Cleaned plugins`, die
ingeschakeld onderaan de lijst wordt aangemaakt, waar hij elk conflict wint. Naar de
plugin in zijn eigen mod, of in de Data-map van het spel, wordt nooit geschreven.
Schakel `Cleaned plugins` uit, of verberg er één bestand in, om een opschoning
ongedaan te maken. Een plugin die in de Overwrite staat wordt niet opgeschoond, omdat
xEdit dat bestand zelf zou herschrijven; maak eerst een mod van de Overwrite.

De back-ups, cache en log van xEdit gaan naar `<instance>/logs/qac-<time>/`, buiten de
samengevoegde weergave, en de statusregel meldt de aantallen uit die log. Eidos
verwijdert die map na een geslaagde opschoning en houdt hem na een mislukte. Eidos
legt de grootte en datum van de plugin waaruit elke kopie gemaakt is vast in
`Cleaned plugins/meta.ini`. Als die plugin verandert, toont het tabblad Diagnostics
**Out-of-date copy of <plugin>**, met een knop die de oude kopie verbergt.

## Er een toevoegen

In de GUI: Tools in de menubalk, of Executables op de werkbalk, dan Add. Vanaf de
opdrachtregel:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` legt de uitvoer van die run vast in `mods/<mod>`, wat de eigen instelling van de tool ook zegt. `rm` verwijdert alleen items die je zelf toevoegde; een standaard per spel kan verborgen worden, niet verwijderd.

Een exe die met `--exe` draait en waarvan de bestandsnaam, hoofdletters genegeerd, er een is die Eidos voor het spel kent, krijgt de titel van die tool, en daarmee de vereisten, argumenten en opvangmod hieronder. `GenerateFNISforUsers.exe` is FNIS, dus de `--exe`-regel hierboven vraagt `dotnet48` aan en vangt op in `FNIS Output`. Elke andere exe krijgt zijn bestandsnaam als titel. In het tabblad Data heeft een `.exe`-rij **Add tool**, dat het dialoogvenster Executables opent met het item ingevuld, en **Run**, dat het bestand eenmalig draait zoals `--exe` dat doet. Beide geven het bestand dezelfde titel.

Voor een opdracht die geen Windows-tool is opent `eidos shell skyrimse` je `$SHELL`
(`/bin/sh` als die niet is ingesteld) in de spelmap met de samengevoegde weergave
gekoppeld. Wat de shell schrijft, landt in de Overwrite, en de shell verlaten
ontkoppelt de weergave. `eidos shell skyrimse -- <command>` draait in plaats daarvan
één opdracht. Ctrl+C in de interactieve shell stopt de opdracht die je typte en laat
de shell open. De shell draait zonder Proton; een Windows-programma gaat via
`eidos tool`.

In het venster beëindigt **Stop** een toolrun die vastloopt: druk er twee keer op, en
Eidos beëindigt de programma's van de run en handelt zijn bestanden daarna af zoals na
elke afsluiting. Met **Notify the desktop when a long job ends** aan (Preferences,
General, Running a game) stuurt een toolrun die eindigt terwijl het venster op de
achtergrond staat een desktopmelding.

De script extender, het binaire bestand van het spel en de launcher worden automatisch
herkend; alleen extra tools moeten geregistreerd worden. Bij een instantie die uit een
spelkopie draait worden ze in de kopie herkend; zie
[Tools op een spelkopie](#tools-op-een-spelkopie).

### Wijs naar het echte bestand, waar dat ook staat

Registreer het uitvoerbare bestand daar waar het werkelijk staat. Is de tool als mod
geïnstalleerd, dan is dat binnen de modmap:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Dat is het pad van de globale instantie. Voor een draagbare instantie geldt dezelfde
regel onder haar eigen map, `<instance>/mods/...`. Een absoluut pad als dit is het enige
dat het later verplaatsen van een draagbare map niet overleeft.

Eidos herschrijft dat pad naar het samengevoegde voordat het start, zodat de tool
draait vanuit `<game>/Data/CalienteTools/BodySlide/` en daar ook de bestanden van
elke andere mod ziet. BodySlide levert een lege `SliderSets`-map mee, en elk lichaam
dat hij kan bouwen komt uit CBBE en de outfit-mods. Gestart vanuit zijn eigen modmap
vindt hij niets en lijkt hij kapot.

MO2 herschrijft net zo, om dezelfde reden: zijn eigen commentaar noemt FNIS.

Een tool in een uitgeschakelde mod kan niet herschreven worden, omdat zijn bestanden
ook niet in de weergave zitten. Eidos zegt dat en draait hem vanuit zijn eigen map.

## Hulpmiddelen uit een Mod Organizer 2-lijst

Een Wabbajack-lijst of een andere MO2-instantie die als Eidos-instantie is geopend, houdt zijn `ModOrganizer.ini`, en de uitvoerbare bestanden die daar staan kunnen naar `tools.ini` worden gekopieerd:

```sh
eidos tool /path/to/list import-mo2           # toont wat er zou worden toegevoegd
eidos tool /path/to/list import-mo2 --write   # voegt ze toe
```

In het venster heeft het dialoogvenster Executables **Import from ModOrganizer.ini**, met per item een vakje om uit te zetten. **Add selected** zet de aangevinkte items in de lijst, en er wordt niets opgeslagen tot je op Save drukt. Alleen een `ModOrganizer.ini` die Eidos niet schreef telt mee; zie [PGPatcher en PGLite](#pgpatcher-en-pglite-in-de-modus-mod-organizer-2) voor die welke Eidos schrijft.

Paden op `Z:` zijn Linux-paden en blijven zoals ze zijn. Een pad op een andere stationsletter gaat onder de map van de lijst als het die map noemt (`D:\Lists\MyList\tools\SSEEdit.exe` wordt `<list>/tools/SSEEdit.exe`), in het uitvoerbare bestand, de werkmap en de argumenten. De loader van de script extender, de launcher en het spelbestand worden gewone namen, zodat ze starten vanuit de spelmap waarover Eidos de weergave legt, net als de ingebouwde items. `hide` van MO2 wordt **Hide from picker**, en `steamAppID` wordt **Steam AppID**. Items die ergens anders heen wijzen (een Windows-systeemprogramma, een hulpmiddel buiten de lijst) staan als overgeslagen in de lijst, met de reden, en een titel die al in `tools.ini` staat wordt nooit vervangen.

Een geïmporteerd item houdt de titel en argumenten van MO2, en Eidos vult niets uit zijn titel in: geen vereisten, geen extra argument en geen opvangmod. Stel ze in het dialoogvenster Executables in. Een PGPatcher-item uit een lijst die op Windows is gemaakt mist meestal `--ignore-mo2vfscheck`; voeg het toe (zie [hieronder](#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft)).

Een geïmporteerd pad dat in de map `Stock Game` van de lijst wijst, wordt pas bij het starten naar de winkelinstallatie verplaatst, en alleen wanneer de instantie uit die kopie draait; zie het volgende gedeelte.

## Tools op een spelkopie

Draait de instantie uit een spelkopie (de map `Stock Game` of `Game Root` van een
lijst, of een bevroren kopie; zie
[Spelbestanden](usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie)), dan ziet
een tool de bestanden van de kopie, niet die van de winkelinstallatie. De samengevoegde
weergave is nog steeds op de winkelinstallatie gemount: de rootweergave heeft de kopie
onderaan, en de Data-weergave leest de `Data` van de kopie. Eidos zoekt de script
extender, de launcher, het spelbestand en de bekende tools in de kopie in plaats van in
de winkelinstallatie.

Een toolpad, een argument of een werkmap die in de kopie wijst, wordt vóór de run naar
de winkelinstallatie verplaatst. Het kan als Linux-pad of als Wine-pad met `Z:` zijn
geschreven: `-D:"Z:\...\Stock Game\Data"` wordt hetzelfde pad onder de winkelinstallatie.
Items die uit de `ModOrganizer.ini` van een lijst zijn geïmporteerd hebben dit nodig.
Draait de instantie uit de winkelinstallatie, dan wordt niets verplaatst, dus zo'n item
leest de kale kopie zonder je mods.

Het `installed path` van het spel in het prefix-register en het `gamePath` in de
`ModOrganizer.ini` die Eidos schrijft, noemen nog steeds de winkelinstallatie, waar de
samengevoegde weergave is. Ontbreekt de kopiemap, of bevat ze geen spelbestand, dan
stopt een toolrun, zoals een spelstart doet, en het bericht noemt
`eidos stock-game <instance> store` om terug te gaan naar de winkelinstallatie.

`eidos stock-game <instance>` toont welke spelbestanden de instantie draait en naar
welke kopieën ze kan overschakelen. `use <folder>` en `store` schakelen om, en
`freeze --yes` kopieert de winkelinstallatie naar de map `Stock Game` van de instantie
en draait daaruit. De kaart Game files op het tabblad Diagnostics doet hetzelfde; zie
[Spelbestanden](usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie).

## De uitvoer van een tool naar een eigen mod sturen

Een generator (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) schrijft honderden
bestanden. Standaard landen ze samen met al het andere in de Overwrite. Stel **Capture
output into** in de Executables-editor in en de uitvoer van deze draai gaat in plaats
daarvan naar die mod:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide en Synthesis komen met een al ingestelde opvangmod: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` en `Synthesis Output`. DynDOLOD, TexGen en xLODGen komen zonder, omdat ze naar de uitvoermap schrijven die je opgeeft.

De mod wordt aangemaakt als hij niet bestaat. Alleen de bestanden die deze draai
voortbracht verhuizen; wat al in de Overwrite stond blijft daar, zodat twee tools
met capture-doelen elkaars uitvoer niet stelen. Een draai die niets schreef laat
geen lege mod achter.

Eidos verplaatst de bestanden na de draai. MO2 richt in plaats daarvan de schrijflaag
op de mod, wat die mod voor de hele draai naar de hoogste prioriteit tilt: elk
conflict waar hij in zit keert om, en daarna weer terug. De schrijfacties gaan ook
zonder copy-up dwars door de eigen bestanden van de mod heen. De capture bereikt
dezelfde eindtoestand zonder beide.

Is de doelmod uitgeschakeld, dan wordt de uitvoer wel geschreven maar het spel
ziet hem niet, zodat de tool bij de volgende draai dezelfde bestanden opnieuw zou
aanmaken. Eidos waarschuwt wanneer dat zo is.

### Als de uitvoer verouderd is

Eidos legt vast uit welke mods, welke pluginvolgorde en welk profiel de uitvoer van een tool is gemaakt, de grootte en datum van het uitvoerbare bestand van de tool, en het pad en de argumenten van het item in `tools.ini`. Verandert een van die dingen, dan toont het tabblad Diagnostics **Generated output inputs changed**, en de kaart heeft een knop **Run <tool> again** die de tool met de huidige instellingen start.

Voor een generator waarvan verouderde uitvoer het spel kapotmaakt, zoals Pandora of Nemesis, zet je `check_before_play=1` bij zijn item in `tools.ini`, of schakel je in het dialoogvenster Executables **Ask before playing when its output is out of date** in. Run vraagt dan eerst: de tool starten of toch spelen. Toch spelen wordt onthouden tot de mods, plugins of het profiel opnieuw veranderen, of tot het venster sluit. Eidos start de tool nooit zelf. Met **Start the game at once when a store opens Eidos** aan houdt zo'n vraag het venster open in plaats van het spel te starten.

## Vereisten en argumenten komen uit de titel

De titel die je een tool geeft bepaalt welke runtime-vereisten Eidos ervoor klaarzet.
De vergelijking is een deelstring van de titel, hoofdletterongevoelig.

| Als de titel bevat | vraagt Eidos aan |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| iets anders | niets |

Een tool die als `BodySlide` geregistreerd is krijgt dus zijn DirectX-DLL's;
hetzelfde uitvoerbare bestand geregistreerd als `BS` krijgt niets en start
misschien niet, met een fout die niets over DLL's zegt. Noem tools naar het
programma.

De lijst staat in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), en het
veld `Prereqs` in het Executables-venster is bewerkbaar: de titel vult alleen de
standaard in.

De titel bepaalt de argumenten op dezelfde manier, voor de ene tool die er een nodig
heeft die niemand kan raden: een titel met `pgpatcher` (of `parallaxgen`, de oude
naam) krijgt `--ignore-mo2vfscheck`. Waarom, en waarom het niet optioneel is, staat
[hieronder](#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft). De opvangmod komt ook
uit de titel (zie [hierboven](#de-uitvoer-van-een-tool-naar-een-eigen-mod-sturen)).

### Drie soorten vereiste

Niveau 1 zijn de meegeleverde DLL's: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`
en `d3dx11_43`. Eidos draagt ze in zijn binaire bestanden en kopieert ze bij het
starten naar het prefix. In een 64-bit prefix, de standaard van Proton, gaat de
64-bit build naar `system32` en de 32-bit naar `syswow64`. Niets te doen, geen
netwerk. Elke build is gzip-gecomprimeerd opgeslagen, zodat de acht bestanden 7,9 MB
van elk binair bestand innemen in plaats van 17,9 MB, en alleen het bestand dat wordt
gekopieerd wordt uitgepakt (hooguit 4,7 MB).

Niveau 2 zijn de installatieprogramma's: `vcrun2022`, `dotnet6`, `dotnet7`,
`dotnet8`, `dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` en `xact_x64`. Die
schrijven registersleutels, de GAC en CLR-hosts, dus ze laten zich niet met een
bestandskopie afhandelen. Drie ervan installeert Eidos zonder winetricks: `vcrun2022`
draait `VC_redist.x86.exe` en `VC_redist.x64.exe` van Microsoft, en `dotnet8` en
`dotnetdesktop8` draaien het installatieprogramma van de .NET 8.0.12 Desktop Runtime,
dat ook de .NET-runtime bevat. De releasebundel (`eidos-linux-bundle.tar.gz`) levert
deze installatieprogramma's mee in zijn map `runtimes/`, en `install.sh` kopieert ze
naar `~/.local/share/Colony/Eidos/runtimes/` voor de gebruiker die het draait, ook met
`--system`. Eidos zoekt elk bestand eerst daar, daarna in een map `runtimes/` naast het
`eidos`-binaire bestand (een uitgepakte bundel die ter plekke draait). Zonder een van
beide kopieën (het losse bestand `eidos-linux`, een build uit de broncode, of een
andere gebruiker van een systeeminstallatie) downloadt Eidos ze van Microsoft
(`download.visualstudio.microsoft.com`) naar `~/.local/share/Colony/Eidos/runtimes/` en
bewaart ze daar. In beide gevallen wordt een bestand geweigerd tenzij zijn SHA-256
overeenkomt met die in Eidos is ingebouwd, en het draait zonder venster
(`/install /quiet /norestart`) in het prefix van het spel, met de Proton- of Wine-build
die de tools van het spel draait. Net als winetricks zet `vcrun2022` eerst de
Visual C++-DLL's op `native,builtin` in de DLL-overrides van het prefix. Daarna
controleert Eidos met een Proton-build dat de C++-runtime van het prefix niet half van
Microsoft en half van Proton is. Is dat zo, dan mislukt de installatie: het bericht
noemt de DLL-paren die uit verschillende builds komen, en zegt in plaats daarvan
`vcrun2022` met winetricks te installeren, wat `msvcp140.dll` zelf vervangt. Alleen de
64-bit .NET 8 wordt geïnstalleerd; een 32-bit .NET 8-tool heeft nog steeds
`winetricks dotnetdesktop8` nodig.

De overige zes verbs lopen via de `winetricks` van het systeem, die in `PATH` moet
staan. Eidos draait het met de `wine` van de Proton- of Wine-build die de tools van het
spel draait, tegen het prefix van het spel, wat de pressure-vessel-container van Steam
en de mismatch tussen protontricks en Proton-GE omzeilt. Sommige winetricks-verbs
hebben `cabextract` nodig, en Eidos waarschuwt wanneer het ontbreekt. Een tool die een
niet-geïnstalleerd niveau-2-verb opgeeft start toch, met een waarschuwing die het verb
noemt en de opdracht om het te verhelpen, omdat je het misschien op een andere manier
hebt geïnstalleerd.

Een niveau-2-verb telt als gedaan wanneer de eigen `winetricks.log` van het prefix het
noemt, wie winetricks of protontricks ook draaide, of wanneer Eidos het in datzelfde
prefix installeerde. Eidos legt zijn eigen installaties vast in `prereqs.done` in de
instantie, gekoppeld aan het pad en de inode van het prefix, zodat een opnieuw
opgebouwd prefix zijn verbs weer als openstaand toont. **Rebuild prefix**, een
Diagnostics-actie voor een Steam-kopie, hernoemt `compatdata/<appid>` naar
`<appid>.eidos-backup-<time>` en verwijdert niets; Steam maakt een nieuw prefix aan de
volgende keer dat het het spel start.

Niveau 3 zijn de runtimes, `dotnet10` en `dotnetsdk10`. Een moderne .NET-runtime is 193
bestanden die in hun eigen map staan en via `DOTNET_ROOT` gevonden worden. Hij wordt
nooit geregistreerd en nooit in het prefix geïnstalleerd, dus geen van de andere
niveaus kan hem dragen. Eidos bewaart hem buiten elke instantie, omdat 78 MB niet per
spel en niet per profiel is: elke runtime wordt uitgepakt naar
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos zoekt de zip van de runtime onder zijn downloadnaam in
dezelfde twee mappen als de niveau-2-installatieprogramma's, en pakt de kopie uit die de
releasebundel meelevert (`dotnet-runtime-10.0.10-win-x64.zip`, alleen voor `dotnet10`)
of downloadt hem. De bundel levert `dotnet-sdk-10.0.403-win-x64.zip` van de SDK niet
mee, maar een kopie die met de hand daar is gezet wordt gebruikt en bespaart de
download. Eidos weigert elk van beide bestanden tenzij zijn SHA-256 overeenkomt met die
in Eidos is ingebouwd. Het pakt de zip uit met `7z`, hetzelfde 7-Zip dat het voor
modarchieven gebruikt.

`dotnetsdk10` is de .NET 10 SDK, een download van 301 MB die uitgepakt 808 MB beslaat.
Synthesis heeft hem nodig om zijn patchers te compileren, en roept een kale `dotnet`
aan om ze te bouwen en te starten. Een tool die `dotnetsdk10` opgeeft, krijgt daarom
ook de SDK-map in `WINEPATH`, plus `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` en `DOTNET_CLI_TELEMETRY_OPTOUT=1`. Een
patcher-build zou MSBuild-nodes en de VBCSCompiler-server laten draaien, en onder Wine
houden die de wineserver van het prefix in leven, dus krijgt de tool ook
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` en
`UseSharedCompilation=false`. Met `DOTNET_ROLL_FORWARD=Major` draait een patcher die voor
.NET 8 gebouwd is (Mutagen vóór 0.54) op de .NET 10-runtime van de SDK als er geen .NET
8-runtime gevonden wordt. Een Synthesis-item in `tools.ini` dat nog de oude standaard
`dotnet8`, `vcrun2022` heeft, krijgt `dotnetsdk10`, `vcrun2022` wanneer Eidos het
inleest.

NuGet controleert de handtekening van elk pakket dat het herstelt tegen de rootstore
van Windows, en in een Wine-prefix ontbreken de roots voor codeondertekening en
tijdstempels die de SDK meelevert als `sdk/10.0.403/trustedroots/codesignctl.pem` en
`timestampctl.pem`. Zonder die roots mislukt het herstellen van
`Mutagen.Bethesda.Synthesis` 0.36.6 met `NU3018` en `NU3028`. Daarom voegt Eidos,
voordat een tool die `dotnetsdk10` declareert start en zodra de SDK gedownload is, de
385 certificaten uit beide bestanden toe aan de rootstore van de huidige gebruiker in
het prefix van het spel, in dezelfde `regedit`-import die het spelpad schrijft (zie
hieronder). Verdwijnt er een, dan importeert Eidos ze opnieuw. Elk Windows-programma in
dat prefix vertrouwt deze roots daarna. Met deze roots slagen hetzelfde herstel en de
build in een nieuw prefix met een lege NuGet-cache, met de systeem-Wine 11.21 en met
GE-Proton11-9.

Eidos installeert niets uit niveau 2 of 3 tot je erom vraagt:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

In de GUI staan dezelfde toestanden onder het Prereqs-veld, en de ontbrekende zijn
knoppen. Een verb dat niet meegeleverd is, geen runtime is en ook geen niveau-2-verb,
wordt als waarschijnlijke tikfout gemeld in plaats van als download aangeboden.

`eidos prereqs <instance> --install` weigert zolang het spel, Steam of een tool het
prefix nog vasthoudt. Niveau-3-runtimes installeren zelfs voordat het prefix bestaat.
De installatieprogramma's die Eidos zelf draait hebben een bestaand prefix nodig: voordat
de winkel het spel één keer heeft gestart, stoppen ze met "start the game once through
its store, then install again". Niveau-2-verbs worden één voor één geïnstalleerd en elk
succes wordt vastgelegd, dus na een mislukking hervat dezelfde opdracht. Bij een GOG- of
Epic-kopie zonder Heroic-instellingen kopieert `--install` de meegeleverde DLL's en stopt
dan vóór de niveau-2-verbs: het zegt de eigen winetricks van die runner te gebruiken, en
leest bij de volgende controle de `winetricks.log` van het prefix.

### Waarom DynDOLOD `dotnet10` nodig heeft

DynDOLOD bouwt object LOD niet zelf: het roept LODGen aan, en het levert er drie
mee. `LODGenx64.exe` richt zich op .NET Framework 4.8, dat onder Proton naar
Wine's Mono omgeleid wordt, en de `System.Uri`-initialisator van Mono roept een methode
aan die Mono niet implementeert. LODGen sterft voor zijn eerste regel werk, en
laat een log achter met een versiebanner en verder niets, en een DynDOLOD-venster
dat alleen "failed for one or more worlds" zegt.

Het echte .NET Framework installeren lost het niet op: Proton vervangt
`mscoree.dll`, de loader die het zou vinden, door een symlink in zijn eigen
boom, en doet dat bij elke prefix-update opnieuw.

De build die werkt is `LODGenx64Win10.exe`, die zich op modern .NET richt en
`mscoree` nooit aanraakt. Wijs `DOTNET_ROOT` naar een .NET 10-runtime en hij
draait. Dat is wat `dotnet10` klaarzet, en Eidos zet de variabele bij het starten
van elke tool die hem opgeeft.

### Waarom Pandora `dotnet10` nodig heeft

Pandora is een Avalonia-app, geen WPF-app, en richt zich sinds Pandora 4.1.0 op
`net10.0`, dus de .NET 10-runtime alleen laat hem draaien. Vindt hij geen runtime, dan
zegt hij de .NET Desktop Runtime te installeren. Dat is de algemene tekst die de
.NET-launcher toont voor elk GUI-programma dat geen runtime vindt, en `dotnetdesktop8`
lost het niet op.

Eidos 1.21.0 en ouder gaf Pandora `dotnetdesktop8`, en een item houdt zijn
`prereqs=`-regel. Leest Eidos een Pandora-item dat nog alleen `dotnetdesktop8` noemt,
en noemt de `.runtimeconfig.json` naast zijn uitvoerbare bestand `net10.0`, dan krijgt
het item in plaats daarvan `dotnet10`. Pandora 2.x richt zich op .NET 8 en houdt wat
het heeft.

### Waarom PGPatcher `--ignore-mo2vfscheck` nodig heeft

PGPatcher herschrijft meshes en plugins zodat de texture-mods die je hebt
(vanilla, parallax, complex material, PBR) op elk oppervlak de juiste shader
gebruiken. Het weigert terecht buiten een mod-manager te draaien: een kale
spelmap patchen zou destructief zijn.

PGPatcher zoekt naar usvfs, de DLL-hooking-laag die MO2 in het spel injecteert. Eidos
heeft die niet: de samengevoegde weergave is een FUSE-mount waar Wine in loopt zonder te
weten dat hij bestaat. De controle vindt dus niets en het gereedschap stopt meteen met

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

en noemt daarbij een programma dat de gebruiker niet draait, nog voor enig werk.
Het antwoord van de auteur zelf op de twee Linux-meldingen ([#716][pg716] en
[#725][pg725], beide van Fluorine, MO2's Linux-fork, die om dezelfde reden FUSE
gebruikt) is een vlag die de controle overslaat.

Eidos vult die daarom vast in. Een PGPatcher die in een mod-pool gevonden wordt
komt met het argument al gezet, en `PGPatcher` als titel typen in het
Executables-venster vult het veld Arguments net zo. Wat jij typt wint altijd: er
wordt alleen een veld ingevuld dat je leeg liet, precies zoals bij de vereisten
hierboven.

De lijst staat in `default_args` (`crates/eidos-instance/src/tools.rs`), naast
`default_prereqs`.

Zijn runtime heeft niets nodig, ongebruikelijk voor een .NET-tool: hij brengt zijn
eigen mee (`dotnetlib/`, met een `runtimeconfig.json` die `includedFrameworks`
verklaart) en levert de Microsoft-CRT-DLL's naast het uitvoerbare bestand. Laat
Prereqs leeg.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher en PGLite in de modus Mod Organizer 2

Met Mod Manager op `None` ziet PGPatcher één samengevoegde map `Data` en kan het
niet zien uit welke mod een mesh of texture komt, dus heeft het geen conflicten
om te tonen en geen modvolgorde om te volgen. Zijn modus `Mod Organizer 2` leest
de instantie zelf: de `modlist.txt` van het profiel, de ingeschakelde mods in hun
prioriteitsvolgorde en de bestanden in de map van elke mod. PGLite, de versie in
het spel, leest de instantie op dezelfde manier. Een Eidos-instantie heeft al de
indeling van MO2 (`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), dus
deze modus werkt ermee.

In de launcher van PGPatcher:

1. Zet **Mod Manager** op `Mod Organizer 2`.
2. Zet **MO2 Options -> Instance Location** op de map van de Eidos-instantie,
   geschreven als Windows-pad op het station `Z:` van Wine, dat `/` is. Een
   draagbare instantie in `/mnt/Games/Eidos-Skyrim` wordt
   `Z:\mnt\Games\Eidos-Skyrim`; een globale wordt
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Houd `--ignore-mo2vfscheck` in de argumenten van de tool. In deze modus zoekt
   PGPatcher naar usvfs en stopt het als dat ontbreekt.

PGPatcher accepteert de map alleen als die een `modorganizer.ini` bevat. Daarom
schrijft Eidos `ModOrganizer.ini` in de hoofdmap van de instantie telkens als
het het spel of een tool start, voor het profiel dat start. Het bestand wordt
alleen herschreven als de inhoud verandert. Het bevat:

- in `[General]`: `gameName` (de naam van het spel in MO2, dus `New Vegas` voor
  Fallout: New Vegas), `gamePath` (de winkelinstallatiemap van het spel als `Z:`-pad,
  ook bij een instantie die uit een spelkopie draait) en `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` en
  `overwrite_directory` als absolute `Z:`-paden, plus `download_directory` als de
  instantie een map `downloads/` heeft.

De eerste regel is een commentaar dat zegt dat Eidos het bestand schreef. Een
`ModOrganizer.ini` zonder die regel, in welke schrijfwijze van de naam ook, wordt
nooit gewijzigd of verwijderd: een MO2-instantie die je in Eidos opende houdt de
hare, en PGPatcher leest dat bestand zoals het is. MO2 laat commentaar weg
wanneer het zijn instellingen opslaat, dus een bestand waar MO2 overheen heeft
opgeslagen is vanaf dan van MO2. Een leeg bestand telt als dat van Eidos.

Zo'n bestand noemt het profiel dat MO2 het laatst selecteerde, en PGPatcher
gebruikt dat profiel, niet het profiel dat Eidos start. Als de twee verschillen,
logt Eidos bij het starten een waarschuwing. Selecteer hetzelfde profiel in MO2,
of hernoem of verwijder het bestand zodat Eidos een eigen bestand schrijft.

Tools in de modus Mod Organizer 2 lezen het spel ook uit het `gamePath` van dat
bestand. Noemt het noch de spelmap waaruit de instantie draait, noch de
winkelinstallatie, dan toont het tabblad Diagnostics **ModOrganizer.ini points tools at
another game folder**, met dat pad en de spelmap die deze instantie draait. Wijzig
`gamePath` in het bestand, of verwijder het bestand zodat Eidos een eigen bestand
schrijft.

Eidos schrijft zijn bestand alleen wanneer het iets start, dus tussen twee starts
noemt het het profiel van de laatste. Een tool die Eidos start vindt er
zijn eigen profiel.

PGPatcher stopt met een fout als het profiel geen ingeschakelde mod heeft, en
als zijn uitvoermap de map van een ingeschakelde mod is, net als onder MO2.

## De eigen programma's van Wine in het prefix van het spel

Het menu File start vier programma's van Wine in het prefix van het spel: winecfg, regedit, de bestandsverkenner van Wine en het verwijderprogramma. Ze starten zoals elke tool, met de Proton- of Wine-build die de tools van het spel draait, met de mods gemount, en het venster wacht erop. **Prefix drive_c** opent het station `C:` van het prefix in je bestandsbeheerder. Een rij blijft grijs tot het prefix dat programma heeft, wat gebeurt wanneer de winkel het spel voor het eerst start. Vanuit een terminal doet `eidos tool <instance> run winecfg` hetzelfde, en `regedit`, `explorer` en `uninstaller` ook, wanneer geen tool in de lijst die titel heeft.

Een wijziging die in regedit aan het `installed path` van het spel is gemaakt, wordt bij de volgende toolrun teruggezet; zie het volgende gedeelte.

## Het spelpad in het prefix

Windows-tools vinden hun spel door
`HKLM\Software\Bethesda Softworks\<game>` `installed path` te lezen, een sleutel
die de installer van het spel zelf schrijft. Steam onder Proton voert die installer
nooit uit. Zonder die sleutel openen xEdit, Wrye Bash en DynDOLOD op een leeg pad.

Vóór een toolrun schrijft Eidos de sleutel met een `regedit`-import in het prefix van
het spel, tenzij een eerdere import hetzelfde pad al schreef en het prefix het nog
bevat. De sleutel noemt de winkelinstallatie, ook bij een instantie die uit een
spelkopie draait, omdat de samengevoegde weergave daar gemount is. Voor een tool die
`dotnetsdk10` declareert voegt dezelfde import de NuGet-ondertekeningsroots toe (zie
[hierboven](#drie-soorten-vereiste)). Een tool met `runner=native` slaat deze stap
over. Bestaat het prefix nog niet, of houdt een ander programma het vast, dan draait de
tool toch, en Eidos drukt een waarschuwing af die de winkel noemt waaruit je het spel
eenmaal moet starten, of het programma dat je moet sluiten. Noemt de sleutel een andere
map dan de winkelinstallatie, dan toont het tabblad Diagnostics **Windows tools cannot
find <game>** tot de volgende toolrun hem opnieuw schrijft.

## Bij een tool komen: verbergen, vastzetten en een bureaubladsnelkoppeling

De standaardinstellingen van een spel bevatten tools die je misschien nooit
gebruikt. In het Executables-venster:

- **Pin to top** zet een invoer bovenaan de Run-lijst.
- **Hide from picker** haalt er een uit zonder hem te verwijderen.
- **Desktop shortcut** schrijft een `.desktop` in `~/.local/share/applications`, dus
  hij verschijnt in je toepassingenmenu en in een zoekopdracht, niet op het
  bureaublad. Het draait rechtstreeks `eidos tool <instance> run <title>`, dus de tool
  komt via de samengevoegde weergave met het profiel van deze instantie op, zonder dat
  het Eidos-venster open is.

Verbergen en vastzetten veranderen alleen hoe een tool bereikt wordt, dus ze gelden
voor de standaardinstellingen per spel net zo goed als voor je eigen invoeren.

De loader van de script extender, de launcher en het binaire bestand van het
spel hebben geen knop Desktop shortcut. Ze starten het spel, en het spel start
altijd via zijn winkel, wat de snelkoppeling van de winkel zelf al doet. Een
snelkoppeling die je voor een ervan met Eidos 1.22 of ouder hebt gemaakt, blijft
werken: net als `eidos tool <instance> run <title>` getypt in een terminal
vraagt ze de winkel het spel te starten, zoals Run doet in een venster dat geen
winkel heeft geopend. Ontbreekt de instelling in de winkel, dan start de
opdracht niets en toont ze wat je moet instellen. Een snelkoppeling toont geen
melding, dus draai dezelfde opdracht in een terminal om die te lezen.

Eidos geeft elk item op deze manier aan de winkel door wanneer de bestandsnaam van zijn
uitvoerbare bestand die van de loader van de script extender, de launcher of het
spelbestand is, hoofdletters genegeerd: een standaarditem, een door jou toegevoegd
item, een uit MO2 geïmporteerd item, of een `--exe`-run. Het markeert eerst deze
instantie als de laatst gebruikte, zodat het venster dat de winkel opent deze instantie
toont, en draait dan `steam -applaunch <id>`,
`xdg-open heroic://launch?appName=<app>&runner=<runner>` voor Heroic, of
`legendary launch <app>`. Met `--print` drukt het die winkelopdracht af en start het
niets. De opdracht `steam` moet in `PATH` staan: de Flatpak-Steam zet hem daar niet, dus
de start mislukt en het bericht zegt het spel vanuit Steam te starten.

Het item draait rechtstreeks via de samengevoegde weergave, zoals elke tool, wanneer
een winkel het proces startte: een van `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`,
`HEROIC_APP_RUNNER` of `EIDOS_STORE_STARTED` is ingesteld. Zonder een daarvan start
Eidos het spel niet zelf met Proton. Heeft de instantie geen loader van de script
extender (in de spelmap waaruit ze draait, in de `Root/` van een ingeschakelde mod of in
`Overwrite/Root/`), dan start er niets en zegt de regel eerst de script extender te
installeren. Voor een GOG-kopie buiten Heroic, of een winkel-id die Eidos niet kan
doorgeven, zegt het het spel vanuit die winkel te starten.

Een profiel kan een eigen snelkoppeling voor het spel hebben. Klik met de
rechtermuisknop op het profiel en kies **Desktop shortcut**: de snelkoppeling
draait `eidos tool <instance> run "<game>" --profile <name>`, dat dat profiel
het actieve maakt, net als het kiezen ervan in het venster, en vraagt de winkel
daarna het spel te starten. Het Eidos-venster dat de winkel opent, toont dat
profiel. `--profile` werkt met elke `eidos tool ... run` en kiest het profiel
voordat de tool start; met `--print` kiest het niets.

## Een tool die zijn eigen Steam-app is

De Creation Kit is een aparte Steam-toepassing en wil zijn eigen AppID; een paar
andere modtools die via Steam geleverd worden zijn net zo. Stel **Steam AppID** in
op de invoer en Eidos start hem onder dat id in plaats van dat van het spel.

Op Windows betekent dit een andere launcher. Hier zijn het twee
omgevingsvariabelen op dezelfde run, `SteamAppId` en `SteamGameId`. Eidos zet
allebei, omdat Proton de ene leest en Steams eigen bibliotheken de andere, en een
tool die ze ziet verschillen faalt zonder duidelijke fout.
`eidos tool ... --print` toont precies wat de echte run zou krijgen.

## Omgevingsvariabelen en tools zonder Proton

Drie sleutels in `<instance>/tools.ini` hebben nog geen veld in het
Executables-venster, dus zet je ze met de hand in de sectie van de tool. Elke
regel `env=NAME=value` zet één variabele die alleen geldt wanneer die tool
draait:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos past deze regels toe na alles wat het zelf zet en na de variabelen in de
`launch-env.ini` van het profiel, dus een waarde hier vervangt beide. Dat geldt ook voor
de `WINEDLLOVERRIDES` die Eidos opbouwt uit de DLL's die je mods meeleveren: een tool die
hem zet, moet er elke DLL in opnemen die hij nodig heeft. De wrappers van het profiel
gaan om de opdracht van een tool heen zoals om die van het spel; zie
[Variabelen en wrappers per profiel](usage.md#variabelen-en-wrappers-per-profiel). Een
naam bestaat uit letters, cijfers en `_`, en begint niet met een cijfer. Een regel met
een andere naam wordt genegeerd.

`runner=native` start `exe` zelf, zonder Proton. De samengevoegde weergave wordt
gemount zoals voor elke tool, en de regels `arg0=`, `arg1=` zijn zijn
argumenten. Gebruik het voor een Linux-programma, of voor een tool bij een kopie
die Steam niet beheert en waarvoor Heroic geen runner vastlegt: `exe` is de
Wine van die kopie, het Windows-programma is het eerste argument en het prefix
komt uit `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` moet een absoluut pad zijn. De werkmap is standaard de map waarin `exe`
staat, hier die van de runner, dus zet `workdir=`. Alleen `exe` wordt
herschreven naar zijn samengevoegde pad: geef voor een Windows-programma in een
mod het pad onder de map `Data` van het spel, waar de samengevoegde weergave het
toont. Een native tool slaat de Proton-stappen over: de registersleutel
hierboven, de shadercache van Steam, de lege .NET-roots en het weghalen van de
Steam-overlay uit `LD_PRELOAD`.

Bij een GOG- of Epic-kopie die met Heroic is geïnstalleerd, hebben tools zo'n
regel niet nodig als de Heroic-instellingen van het spel
(`GamesConfig/<appName>.json`) een Wine- of Proton-build noemen: Eidos start ze
met die build in het Heroic-prefix van het spel, en
`eidos prereqs <instance> --install` gebruikt die voor de installatieprogramma's
en voor winetricks. Een kopie die alleen met Legendary is geïnstalleerd, heeft
geen Heroic-instellingen.

`slr=1` start de tool, en de registerimport ervoor, in de container van de Steam
Linux Runtime die de Proton-build van de tool noemt (`require_tool_appid` in
zijn `toolmanifest.vdf`), zoals Steam het spel start. Die runtime moet in een
Steam-bibliotheek geïnstalleerd zijn; anders stopt de run en noemt hij de
Steam-app die je moet installeren. `eidos prereqs` draait zijn
installatieprogramma's en winetricks nog steeds op de host, en een tool met
`runner=native` negeert de sleutel. De container ziet je thuismap en de
Steam-bibliotheek van het spel. Een tool, mod of runtime ergens anders,
bijvoorbeeld op een andere schijf, heeft `env=STEAM_COMPAT_MOUNTS=/dat/pad`
nodig (meerdere paden gescheiden door `:`). Een Proton-build onder
`/usr/share/steam/compatibilitytools.d` is met `slr=1` waarschijnlijk niet
bruikbaar, omdat `/usr` in de container die van de runtime is. Geen enkele toolrun in de
container is al getest.

`eidos tool <instance> run <title> --print` toont het uiteindelijke commando en
elke variabele die de run boven op de geërfde omgeving zet, deze regels
inbegrepen. Voor een item dat het spel start drukt het in plaats daarvan de
winkelopdracht af (zie [Bij een tool komen](#bij-een-tool-komen-verbergen-vastzetten-en-een-bureaubladsnelkoppeling)).

## BethINI Pie

Eidos levert BethINI Pie niet mee. Zet de versie die je van de Nexus-pagina hebt gedownload in `mods/` van deze instantie of in je toolsmap, dan verschijnt `Bethini.exe` in de toollijst, bij dezelfde spellen als PGPatcher. Via Eidos gestart bewerkt het de kopie van de spel-INI's in het profiel. Vóór elke run van een tool waarvan de titel `bethini` bevat, bewaart Eidos een herstelpunt van die INI's; **Backups** op de werkbalk, dan **INIs**, zet er een terug. Een INI die er bij het maken van de kopie nog niet was, blijft zoals hij is. Het dialoogvenster bewaart de 10 nieuwste herstelpunten van elke soort, en een herstelpunt terugzetten maakt eerst een back-up van de INI's zoals ze nu zijn, zodat een verkeerde keuze ongedaan kan worden gemaakt.

## De eigen instellingen van een tool blijven de zijne

Eidos controleert de meeste toolinstellingen niet; een foute faalt meestal zonder
fout.

De **Game Data Path** van BodySlide (Settings) moet naar de `Data`-map van het spel
wijzen, niet naar de spelmap erboven. Eén niveau te hoog ingesteld meldt een batch
build "All sets processed successfully" en schrijft 1439 meshes daar waar het spel er
nooit naar zal kijken. Eidos vangt ze op in `Overwrite/Root/`, dus je spelinstallatie
blijft onaangeroerd, en het enige teken in het spel is dat je lichamen niet gebouwd
zijn.

Vóór een run van BodySlide of Outfit Studio waarschuwt de statusregel wanneer
`GameDataPath` of `OutputDataPath` in de winnende `CalienteTools/BodySlide/Config.xml`
tijdens de run niet naar de `Data`-map van het spel leidt, en het tabblad Diagnostics
toont dezelfde waarschuwing als kaart met de waarde die je moet instellen.
`eidos check <instance>` meldt hem ook. Een `S:`-pad wordt gelezen zoals Eidos `S:` voor
een tool koppelt, aan de map van de Steam-bibliotheek. Een `OutputDataPath` in de map
`mods/` van de instantie, voor een uitvoermod, wordt geaccepteerd. Diagnostics toont
ook een kaart wanneer `Overwrite/Root/` een map `meshes` of `textures` bevat.

Uitvoer van tools hoort in de Overwrite. Wanneer een run iets oplevert dat het
bewaren waard is, maakt **Overwrite -> Create mod...** er een gewone mod van die
net als elke andere geordend, uitgeschakeld en verwijderd kan worden.
