<!-- eidos-i18n: source=docs/guide/tools.md sha=9940c6b6adb7d68eb703f9b4314e8ba13cc06e17 -->

# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Een tool die via Eidos draait ziet **de samengevoegde weergave**, binnen het
Proton-prefix van het spel zelf. Hij leest wat het spel zal lezen - elke
ingeschakelde mod, op prioriteitsvolgorde - en wat hij ook schrijft, dat landt in
de Overwrite, waar één klik er een echte mod van maakt.

## Degene die Eidos zelf vindt

Sommige tools heten uniek genoeg om gevonden te worden in plaats van opgegeven,
en xEdit is het duidelijke geval: `FO4Edit.exe` voor Fallout 4, `SSEEdit.exe`
voor Skyrim SE, `TES5Edit.exe` voor het origineel, enzovoort - samen met de
**QuickAutoClean**-tweeling van elk, de knop voor de dirty edits waar LOOT steeds
voor waarschuwt. Eidos zoekt ernaar, op bestandsnaam, in:

- de installatiemap van het spel, en de `Root/`-bomen van ingeschakelde mods;
- **de `mods/` van deze instantie**, waar MO2-gebruikers hun tools installeren;
- de **tools folder** die je in Preferences instelt (General -> Tools), voor de
  map die tussen instanties gedeeld wordt - `/mnt/Games/Tools` en dergelijke.

`PGPatcher.exe` wordt op dezelfde manier gevonden bij de Creation Engine-spellen, en komt met een argument waarzonder het niet kan werken - zie [hieronder](#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft).

De lijst geldt per spel, dus aan een Skyrim-instantie wordt nooit de editor van
Fallout aangeboden. Het zoeken stopt vier niveaus diep, omdat een modvoorraad
honderdduizenden bestanden telt en dit draait telkens als de toollijst opgebouwd
wordt, en het volgt geen symlinks. Een zo gevonden tool is precies zo ingesteld
als een die je zelf ingetypt hebt: zijn runtimes volgen uit zijn naam, volgens
dezelfde regel als alles hieronder.

Staat een tool ergens anders, of wil je andere argumenten, voeg hem dan met de
hand toe - een eigen invoer met dezelfde titel overschrijft alles wat automatisch
gevonden is.

## Er een toevoegen

In de GUI: **Tools -> Executables**, dan Add. Vanaf de opdrachtregel:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

De script extender, het binaire bestand van het spel en de launcher worden
automatisch herkend; alleen extra tools moeten geregistreerd worden.

### Wijs naar het echte bestand, waar dat ook staat

Registreer het uitvoerbare bestand daar waar het werkelijk staat. Is de tool als
mod geïnstalleerd, dan is dat binnen de modmap:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(dat is het pad van de globale instantie - voor een draagbare instantie geldt
dezelfde regel onder haar eigen map, `<instance>/mods/...`; let erop dat een
absoluut pad als dit het enige is dat het later VERPLAATSEN van een draagbare map
niet overleeft).

Eidos herschrijft dat pad naar het samengevoegde voordat het start, zodat de tool
draait vanuit `<game>/Data/CalienteTools/BodySlide/` en daar ook de bestanden van
elke andere mod ziet. Dit weegt zwaarder dan het klinkt: BodySlide levert een
**lege** `SliderSets`-map mee, en elk lichaam dat hij kan bouwen komt uit CBBE en
de outfit-mods. Gestart vanuit zijn eigen modmap vindt hij niets en lijkt hij
kapot.

MO2 herschrijft net zo, om dezelfde reden - zijn eigen commentaar noemt FNIS.

Een tool in een **uitgeschakelde** mod kan niet herschreven worden, omdat zijn
bestanden ook niet in de weergave zitten. Eidos zegt dat en draait hem vanuit
zijn eigen map in plaats van te doen alsof.

## De uitvoer van een tool naar een eigen mod sturen

Een generator - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - schrijft honderden
bestanden. Standaard landen ze samen met al het andere in de Overwrite. Stel
**Capture output into** in de Executables-editor in en de uitvoer van deze draai
gaat in plaats daarvan naar die mod:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

De mod wordt aangemaakt als hij niet bestaat. Alleen de bestanden die DEZE draai
voortbracht verhuizen; wat al in de Overwrite stond blijft daar, zodat twee tools
met capture-doelen elkaars uitvoer niet stelen. Een draai die niets schreef laat
geen lege mod achter.

Het gebeurt na de draai, in plaats van door de schrijflaag op de mod te richten,
zoals MO2 het doet. De schrijflaag op een mod richten zou hem voor de hele draai
naar de hoogste prioriteit tillen - elk conflict waar hij in zit omkeren en daarna
weer terug - en zou zonder copy-up dwars door de eigen bestanden van de mod heen
schrijven. De capture bereikt dezelfde eindtoestand zonder allebei.

Is de doelmod uitgeschakeld, dan wordt de uitvoer wel geschreven maar het spel
ziet hem niet, zodat de tool bij de volgende draai dezelfde bestanden opnieuw zou
aanmaken. Eidos waarschuwt wanneer dat zo is.

## De DLL's die een tool nodig heeft worden gekozen op zijn NAAM

Dit is het verrassende deel, dus het is de moeite waard het gewoon te zeggen: **de
titel die je een tool geeft bepaalt welke runtime-vereisten Eidos ervoor
klaarzet.** De vergelijking is een deelstring van de titel, hoofdletterongevoelig.

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

Een tool die als **`BodySlide`** geregistreerd is krijgt dus zijn DirectX-DLL's;
hetzelfde uitvoerbare bestand geregistreerd als **`BS`** krijgt niets en start
misschien niet, met een fout die niets over DLL's zegt. Noem tools naar het
programma.

De lijst staat in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), en het
veld `Prereqs` in het Executables-venster is bewerkbaar - de detectie is een
standaard, geen regel.

De titel bepaalt de **argumenten** op dezelfde manier, voor het ene gereedschap
dat er een nodig heeft die niemand kan raden: een titel met `pgpatcher` (of
`parallaxgen`, de oude naam) krijgt `--ignore-mo2vfscheck`. Waarom, en waarom
het niet optioneel is, staat [hieronder](#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft).

### Drie soorten vereiste

**Tier 1 - meegeleverde DLL's** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos levert ze mee en kopieert ze bij het starten naar het prefix. Niets te doen, geen
netwerk.

**Tier 2 - winetricks-verbs** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Die schrijven registersleutels, de GAC en CLR-hosts, dus
ze laten zich niet met een bestandskopie afhandelen. Ze **downloaden van
Microsoft**.

**Tier 3 - runtimes** (`dotnet10`, `dotnetsdk10`). Een moderne .NET-runtime is 193 bestanden die
in hun eigen map staan en via `DOTNET_ROOT` gevonden worden: nooit geregistreerd,
nooit in het prefix geïnstalleerd, zodat geen van de andere lagen hem kan dragen.
Eidos downloadt hem zelf, controleert hem tegen een in het binaire bestand
ingebouwde checksum, en bewaart hem in `~/.local/share/Colony/Eidos/runtimes/` -
**buiten elke instantie**, omdat 78 MB niet per spel en niet per profiel is.

`dotnetsdk10` is de .NET 10 SDK, een download van 301 MB die uitgepakt 808 MB
beslaat. Synthesis heeft hem nodig om zijn patchers te compileren, en roept een
kale `dotnet` aan om ze te bouwen en te starten. Een tool die `dotnetsdk10`
opgeeft, krijgt daarom ook de SDK-map in `WINEPATH`, plus
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
en `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Een patcher-build zou MSBuild-nodes en de VBCSCompiler-server laten draaien, en
onder Wine houden die de wineserver van de prefix in leven, dus krijgt de tool
ook `MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` en
`UseSharedCompilation=false`. Met `DOTNET_ROLL_FORWARD=Major` draait een patcher
die voor .NET 8 gebouwd is (Mutagen vóór 0.54) op de .NET 10-runtime van de SDK
als er geen .NET 8-runtime gevonden wordt.
Een Synthesis-item in `tools.ini` dat nog de
oude standaard `dotnet8`, `vcrun2022` heeft, krijgt `dotnetsdk10`, `vcrun2022`
wanneer Eidos het inleest.

NuGet controleert de handtekening van elk pakket dat het herstelt tegen de
rootstore van Windows, en in een Wine-prefix ontbreken de roots voor
codeondertekening en tijdstempels die de SDK meelevert als
`sdk/10.0.403/trustedroots/codesignctl.pem` en `timestampctl.pem`. Zonder die
roots mislukt het herstellen van `Mutagen.Bethesda.Synthesis` 0.36.6 met
`NU3018` en `NU3028`. Daarom voegt Eidos, voordat een tool die `dotnetsdk10`
declareert start en zodra de SDK gedownload is, de 385 certificaten uit beide
bestanden toe aan de rootstore van de huidige gebruiker in de prefix van het
spel, in dezelfde `regedit`-import die het spelpad schrijft (zie hieronder).
Verdwijnt er een, dan importeert Eidos ze opnieuw. Elk Windows-programma in die
prefix vertrouwt deze roots daarna. Met deze roots slagen hetzelfde herstel en
de build in een nieuwe prefix met een lege NuGet-cache, met de systeem-Wine
11.21 en met GE-Proton11-9.

Niets in tier 2 of 3 draait stilzwijgend:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

In de GUI staan dezelfde toestanden onder het Prereqs-veld, en de ontbrekende zijn
knoppen. Een verb dat niet meegeleverd is, geen runtime is en ook geen bekende
winetricks-verb is, wordt als waarschijnlijke tikfout gemeld in plaats van als
download aangeboden.

### Waarom DynDOLOD `dotnet10` nodig heeft

DynDOLOD bouwt object LOD niet zelf: het roept LODGen aan, en het levert er drie
mee. `LODGenx64.exe` richt zich op .NET Framework 4.8, dat onder Proton naar
Wine's Mono omgeleid wordt - waarvan de `System.Uri`-initialisator een methode
aanroept die Mono niet implementeert. Het sterft voor zijn eerste regel werk, en
laat een log achter met een versiebanner en verder niets, en een DynDOLOD-venster
dat alleen "failed for one or more worlds" zegt.

Het echte .NET Framework installeren lost het niet op: Proton vervangt
`mscoree.dll` - de loader die het zou vinden - door een symlink in zijn eigen
boom, en doet dat bij elke prefix-update opnieuw.

De build die werkt is `LODGenx64Win10.exe`, die zich op modern .NET richt en
`mscoree` nooit aanraakt. Wijs `DOTNET_ROOT` naar een .NET 10-runtime en hij
draait. Dat is wat `dotnet10` klaarzet, en Eidos zet de variabele bij het starten
van elke tool die hem opgeeft.

Eidos draait de `winetricks` van het systeem tegen Protons eigen `wine` en het
prefix van het spel, wat de pressure-vessel-container van Steam en de mismatch
tussen protontricks en Proton-GE omzeilt. Een tool die een niet-geïnstalleerde
Tier-2-verb opgeeft start toch, met een waarschuwing die de verb noemt en de
opdracht om het te verhelpen - de gebruiker heeft hem misschien ergens anders
vandaan.

### Waarom PGPatcher `--ignore-mo2vfscheck` nodig heeft

PGPatcher herschrijft meshes en plugins zodat de texture-mods die je hebt -
vanilla, parallax, complex material, PBR - op elk oppervlak de juiste shader
gebruiken. Het weigert terecht buiten een mod-manager te draaien: een kale
spelmap patchen zou destructief zijn.

Het probleem is HOE het dat controleert: het zoekt naar **usvfs**, de
DLL-hooking-laag die MO2 onder Windows in het spel injecteert. Eidos heeft die
niet en krijgt die nooit - de samengevoegde weergave is een FUSE-mount waar Wine
in loopt zonder te weten dat hij bestaat - dus de controle vindt niets en het
gereedschap stopt meteen met

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
`default_prereqs`. Hij heeft een regel, en een is genoeg voor een eigen functie:
een vlag kan niemand raden, en de fout die hij voorkomt noemt het verkeerde
programma.

Bij het draaien vraagt het niets, ongebruikelijk voor een .NET-gereedschap: het
brengt zijn eigen mee (`dotnetlib/`, met een `runtimeconfig.json` die
`includedFrameworks` verklaart) en levert de Microsoft-CRT-DLL's naast het
uitvoerbare bestand. Laat Prereqs leeg.

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

- in `[General]`: `gameName` (de naam van het spel in MO2, dus `New Vegas` voor Fallout: New Vegas), `gamePath` (de installatiemap van het spel als
  `Z:`-pad) en `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` en
  `overwrite_directory` als absolute `Z:`-paden, plus `download_directory` als de
  instantie een map `downloads/` heeft.

De eerste regel is een commentaar dat zegt dat Eidos het bestand schreef. Een
`ModOrganizer.ini` zonder die regel, in welke schrijfwijze van de naam ook, wordt
nooit gewijzigd of verwijderd: een MO2-instantie die u in Eidos opende houdt de
hare, en PGPatcher leest dat bestand zoals het is. MO2 laat commentaar weg
wanneer het zijn instellingen opslaat, dus een bestand waar MO2 overheen heeft
opgeslagen is vanaf dan van MO2. Een leeg bestand telt als dat van Eidos.

Zo'n bestand noemt het profiel dat MO2 het laatst selecteerde, en PGPatcher
gebruikt dat profiel, niet het profiel dat Eidos start. Als de twee verschillen,
logt Eidos bij het starten een waarschuwing. Selecteer hetzelfde profiel in MO2,
of hernoem of verwijder het bestand zodat Eidos een eigen bestand schrijft.

Eidos schrijft zijn bestand alleen wanneer het iets start, dus tussen twee starts
noemt het het profiel van de laatste. Een tool die Eidos start vindt er
zijn eigen profiel.

PGPatcher stopt met een fout als het profiel geen ingeschakelde mod heeft, en
als zijn uitvoermap de map van een ingeschakelde mod is, net als onder MO2.

## Het spelpad in het prefix

Windows-tools vinden hun spel door
`HKLM\Software\Bethesda Softworks\<game>` `installed path` te lezen, een sleutel
die de installer van het spel zelf schrijft - en die Steam onder Proton nooit
uitvoert. Zonder die sleutel openen xEdit, Wrye Bash en DynDOLOD op een leeg pad.
Eidos schrijft hem voordat het een tool draait: idempotent, aanvullend, en
overgeslagen als het prefix niet geïnitialiseerd is of in gebruik is.

## Bij een tool komen: verbergen, vastzetten en een bureaubladsnelkoppeling

De standaardinstellingen van een spel bevatten tools die je misschien nooit
gebruikt, en een keuzelijst die acht invoeren opsomt om bij de tweede te komen is
een keuzelijst die niemand leest. In het Executables-venster:

- **Pin to top** zet een invoer bovenaan de Run-lijst.
- **Hide from picker** haalt er een uit zonder hem te verwijderen.
- **Desktop shortcut** schrijft een `.desktop` in
  `~/.local/share/applications` - waar een starter hoort op een
  freedesktop-systeem, zodat hij in je toepassingenmenu en in een zoekopdracht
  opduikt en niet op het bureaublad. Het draait rechtstreeks
  `eidos tool <instance> run <title>`, wat betekent dat de tool **via de
  samengevoegde weergave met het profiel van deze instantie** opkomt zonder dat
  het Eidos-venster ook maar open is.

Verbergen en vastzetten gaan over hoe een tool *bereikt* wordt en niet over wat
hij draait, dus ze gelden voor de standaardinstellingen per spel net zo goed als
voor je eigen invoeren.

## Een tool die zijn eigen Steam-app is

De Creation Kit is een aparte Steam-toepassing en wil zijn eigen AppID; een paar
andere modtools die via Steam geleverd worden zijn net zo. Stel **Steam AppID** in
op de invoer en Eidos start hem onder dat id in plaats van dat van het spel.

Op Windows betekent dit een andere launcher. Hier zijn het twee
omgevingsvariabelen op de draai die toch al opgebouwd werd - `SteamAppId` en
`SteamGameId`, allebei, omdat Proton de ene leest en Steams eigen bibliotheken de
andere, en een tool die ze ziet verschillen faalt vreemd in plaats van duidelijk.
`eidos tool ... --print` toont precies wat de echte draai zou krijgen.

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

Eidos past deze regels toe na alles wat het zelf zet, dus een waarde hier
vervangt die van Eidos. Dat geldt ook voor de `WINEDLLOVERRIDES` die Eidos
opbouwt uit de DLL's die je mods meeleveren: een tool die hem zet, moet er elke
DLL in opnemen die hij nodig heeft. Een naam bestaat uit letters, cijfers en
`_`, en begint niet met een cijfer. Een regel met een andere naam wordt
genegeerd.

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
`eidos prereqs <instance> --install` geeft die door aan winetricks. Een kopie
die alleen met Legendary is geïnstalleerd, heeft geen Heroic-instellingen.

`slr=1` start de tool, en de registerimport ervoor, in de container van de Steam
Linux Runtime die de Proton-build van de tool noemt (`require_tool_appid` in
zijn `toolmanifest.vdf`), zoals Steam het spel start. Die runtime moet in een
Steam-bibliotheek geïnstalleerd zijn; anders stopt de run en noemt hij de
Steam-app die je moet installeren. `eidos prereqs` draait winetricks nog steeds
op de host, en een tool met `runner=native` negeert de sleutel. De container
ziet je thuismap en de Steam-bibliotheek van het spel. Een tool, mod of runtime
ergens anders, bijvoorbeeld op een andere schijf, heeft
`env=STEAM_COMPAT_MOUNTS=/dat/pad` nodig (meerdere paden gescheiden door `:`).
Een Proton-build onder `/usr/share/steam/compatibilitytools.d` is met `slr=1`
waarschijnlijk niet bruikbaar, omdat `/usr` in de container die van de runtime
is. Een toolrun in de container is nog niet getest.

`eidos tool <instance> run <title> --print` toont het uiteindelijke commando en
elke variabele die de draai boven op de geërfde omgeving zet, deze regels
inbegrepen.

## De eigen instellingen van een tool blijven de zijne

Eidos zet een tool op de juiste plek met de juiste DLL's. Wat de tool daarna met
zijn configuratie doet is een zaak tussen jou en de tool, en de fout is meestal
stil.

Het uitgewerkte voorbeeld, omdat het anders een uur kost: de **Game Data Path**
van BodySlide (Settings) moet naar de `Data`-map van het spel wijzen, niet naar de
spelmap erboven. Eén niveau te hoog ingesteld meldt een batch build "All sets
processed successfully" en schrijft 1439 meshes daar waar het spel er nooit naar
zal kijken. Eidos vangt ze op - ze landen in `Overwrite/Root/` en niet in je
installatie - maar vanuit het spel gezien is er niets mis behalve dat je lichamen
niet gebouwd zijn.

Uitvoer van tools hoort in de Overwrite. Wanneer een draai iets oplevert dat het
bewaren waard is, maakt **Overwrite -> Create mod...** er een gewone mod van die
net als elke andere geordend, uitgeschakeld en verwijderd kan worden.
