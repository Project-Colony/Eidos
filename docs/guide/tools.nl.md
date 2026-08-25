<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Tools: xEdit, BodySlide, DynDOLOD, FNIS

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
- de **tools folder** die je in Settings instelt (Tools -> Tools folder), voor de
  map die tussen instanties gedeeld wordt - `/mnt/Games/Tools` en dergelijke.

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
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Drie soorten vereiste

**Tier 1 - meegeleverde DLL's** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
levert ze mee en kopieert ze bij het starten naar het prefix. Niets te doen, geen
netwerk.

**Tier 2 - winetricks-verbs** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Die schrijven registersleutels, de GAC en CLR-hosts, dus
ze laten zich niet met een bestandskopie afhandelen. Ze **downloaden van
Microsoft**.

**Tier 3 - runtimes** (`dotnet10`). Een moderne .NET-runtime is 193 bestanden die
in hun eigen map staan en via `DOTNET_ROOT` gevonden worden: nooit geregistreerd,
nooit in het prefix geïnstalleerd, zodat geen van de andere lagen hem kan dragen.
Eidos downloadt hem zelf, controleert hem tegen een in het binaire bestand
ingebouwde checksum, en bewaart hem in `~/.local/share/Colony/Eidos/runtimes/` -
**buiten elke instantie**, omdat 78 MB niet per spel en niet per profiel is.

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
