<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**De native Linux-modmanager die je spel nooit aanraakt.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installatie)

Eidos geeft Bethesda-spellen op Linux wat Mod Organizer 2 ze op Windows geeft: een bij
elke start opnieuw samengevoegde weergave van je mods. Het gebruikt Linux-koppelnaamruimten
en FUSE in plaats van het onderscheppen van Windows-API's, en de manager draait zonder Wine.
Er wordt niets naar de spelmap gekopieerd, dus er valt achteraf niets op te ruimen. Eidos heeft
ook definities voor Stellar Blade, Stardew Valley, 7 Days to Die en Crimson Desert.

Steam-kopieën draaien Eidos vanuit de opstartoptie van het spel. GOG- en Epic-kopieën in Heroic
of Legendary draaien het als wrapper. Het spel start altijd via de winkel die de kopie bezit;
Eidos start het nooit zelf met Proton.

```
Steam ──> eidos-gui %command% ──> [ privénaamruimte ]
                                  │  mods ⊕ spel  ──> wat het spel ziet
                                  └─ sterft met het spel; de installatie blijft ongerept
```

> **Status:** Skyrim SE wordt dagelijks via Eidos gespeeld, met SKSE, preloaders van script
> extenders, Creation Club, met LOOT gesorteerde laadvolgordes en saves per profiel. Eén
> spelfamilie is tot nu toe in echt spel bewezen. Tien andere Bethesda-spellen en vier andere
> spellen zijn aangesloten en wachten op testers.

## Waarom Eidos

De samengevoegde weergave wordt gekoppeld in een privé-koppelnaamruimte die alleen de
procesboom van het spel kan zien. Andere programma's, een tweede spel inbegrepen, zien haar
nooit en hebben er geen toestemming voor nodig. Sluit het spel af, of wordt het gedood, of valt
de stroom uit, dan verdwijnt de naamruimte met haar processen en is de installatie zoals ze was.

Elk profiel bezit zijn eigen modlijst, pluginvolgorde, INI's en saves. Bij de start worden de
pluginbestanden en de savemap met een bind-mount over de eigen paden van het spel gelegd, zodat
wat het spel daar schrijft in het profiel belandt. Van profiel wisselen wisselt dat allemaal.

Eidos heeft geen setuid-helper nodig, geen systeemdaemon, geen `sudo setcap` en geen aanpassing
van `/etc/fuse.conf`. De FUSE-daemon draait als jouw gebruiker voor één start en eindigt met het
spel. Je installeert één programma en stelt één Steam-opstartoptie in.

Vóór elke run bewaart Eidos een momentopname van de pluginlijst. Beschadigt een crash de lijst,
dan vergelijkt Diagnostics haar met die momentopname, en Restore the pre-session order zet de
oude volgorde terug. Een capture die je laadvolgorde zou wissen wordt geweigerd, met de reden.

Test run start het spel met de mods van het profiel, zonder mods, of alleen met de geselecteerde
mods die aanstaan, plus hun masters. De saves, INI-wijzigingen, pluginstatus, Overwrite-uitvoer en andere
My Games-bestanden van die run, logs inbegrepen, gaan naar `<instance>/test-run/`. Het Wine-register
hoort daar niet bij en gaat waar het altijd heen gaat. Als het spel stopt, toont Eidos die vijf groepen
en bewaar of gooi je elke groep weg. Find the mod behind a problem, in Diagnostics, draait testruns
met telkens de helft van de verdachte mods plus de masters die ze nodig hebben. Na elke run antwoord
je of het probleem optrad, en Eidos versmalt de verdachten tot het de mod noemt, of de mods die elkaars
masters nodig hebben; 200 verdachten kosten ongeveer 8 runs. Ingeschakelde mods die je vóór de start in de lijst
selecteerde blijven bij elke stap aan. Details in [Testruns](docs/guide/usage.md#testruns) en
[De mod achter een probleem vinden](docs/guide/usage.md#de-mod-achter-een-probleem-vinden).

### Hoe het zich verhoudt

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / uitrollers met links |
|---|---|---|---|---|
| Manager draait native | ja | nee, een Windows-app in Wine | ja (Qt-port) | ja |
| Spelmap onaangeroerd | ja, altijd | ja | ja | nee, er worden links in geschreven |
| Koppeling zichtbaar voor | alleen het spel | alleen het spel | het hele systeem | n.v.t. |
| Opruimen na crash nodig | geen | geen | herstel van een dode koppeling | handmatig uitrollen ongedaan maken |
| Root-mods (ENB, preloaders) | ja, native | plugin vereist | plugin vereist | gedeeltelijk |
| Vereiste privileges | geen | geen | `/etc/fuse.conf` aanpassen | geen |

### Hoe snel het is

| | voorheen | nu |
|---|---|---|
| een save laden | ~20 seconden | 6-7 seconden |
| maplezingen in één sessie | 5,6 miljoen | 335 duizend |

Celovergangen zijn onmiddellijk. Eén bestand vinden ondervroeg vroeger elk van vijftig mods op hun
beurt, en één map opsommen deed dat vijftig keer over. De alleen-lezen lagen worden nu bij het
koppelen één keer geïndexeerd, dus een bestand vinden is één hash-opzoeking, en de samengevoegde
lijst van een map wordt ook één keer bij het koppelen berekend. De cijfers komen van een echte
instantie die normaal gespeeld werd, niet van een benchmark:
[internals/performance.md](../../internals/performance.md).

### Eerder werk en dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) en
  [usvfs](https://github.com/ModOrganizer2/usvfs): de semantiek die Eidos reproduceert, en de
  codebase waartegen zijn pariteit bestudeerd is
- [LOOT](https://loot.github.io/): de sorteermotor, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) en de andere Linux-managers, die laten zien dat er een
  gemeenschap is die dit opgelost wil zien

## Wat het doet

### Mods

Eidos installeert simpele archieven, FOMOD-wizards, BAIN-pakketten van Wrye Bash, OMOD-containers
met ondersteunde OBMM-scripts, en al het andere via een handmatige kiezer. Root-mods (preloaders van
script extenders, ENB, Engine Fixes) werken native, zonder Root Builder-plugin en zonder iets naar je
installatie te kopiëren. Je kunt losse bestanden verbergen, mods groeperen met scheidingen, een
selectie naar een doel verplaatsen, notities en categorieën per mod bijhouden, en een MO2-profiel
importeren.

De lijst werkt zoals die van MO2: acht optionele kolommen en sorteren op elk ervan, groeperen per
categorie of per bron, gebaren met dubbelklik, typen om te springen, back-ups per mod die niets doen
tot je ze terugzet, en vlaggen voor een mod waarvan de indeling door dit spel niet geladen wordt of die
voor een ander spel gedownload is. Filters kiezen mods op Nexus-status (endorsed, tracked, update
beschikbaar), notities, verborgen bestanden en soort inhoud (plugins, archieven, textures, meshes,
scripts, geluid en meer). De bestandsboom maakt mappen, hernoemt, verwijdert en opent bestanden, en toont
voorbeelden van afbeeldingen en tekst zonder iets te starten.

Ctrl+Z en Ctrl+Shift+Z maken wijzigingen in de modlijst en de laadvolgorde ongedaan en voeren ze opnieuw
uit. Een mod uitschakelen of verwijderen waarvan de plugin door een van de drie nieuwste saves van het
profiel wordt gebruikt, vraagt eerst en noemt de plugins (saves van Skyrim en Fallout 4). Een FOMOD-mod
opnieuw installeren of bijwerken opent de wizard op de antwoorden van zijn laatste installatie. De
informatie van een mod toont de mods die ze nodig heeft en de mods die haar nodig hebben, gelezen uit de
masters van de ingeschakelde plugins.

### Plugins

De laadvolgorde heeft LOOT-sortering ingebouwd, modindexen zoals het spel ze berekent, waarschuwingen
voor ontbrekende masters, en je DLC- en Creation Club-inhoud getoond als onbeheerde rijen. De kop telt de
gebruikte slots in elke indexruimte (full, medium, light). Over een plugin zweven toont zijn auteur,
beschrijving en headerversie. Bij de Skyrim-engines wordt een plugin gemarkeerd waarvan de INI met dezelfde
naam door het spel geladen wordt.

Plugins die LOOT als vuil markeert worden door de QuickAutoClean van xEdit opgeschoond in een mod
`Cleaned plugins`, nooit ter plekke. Flag as light schrijft in een nieuwe mod een light-kopie van een
plugin, en weigert wanneer een van de nieuwste saves die plugin als volledige plugin laadt (saves van Skyrim
en Fallout 4).

### Instanties

Een instantie is globaal, bewaard onder `~/.local/share/Colony/Eidos/instances/`, of draagbaar: een op
zichzelf staande map waar je maar wilt (een tweede schijf, een spellenpartitie), verplaatsbaar en geïsoleerd
zoals die van MO2. Draagbare instanties worden van sessie tot sessie onthouden. De GUI, de Steam-start en elke
CLI-opdracht volgen degene die je het laatst gebruikt hebt, en elke opdracht neemt de map aan waar ze ook een
spel-id aanneemt. Details in [usage.md](docs/guide/usage.md#instanties-globaal-en-draagbaar).

Een instantie kan het spel ook draaien vanuit een kopie van de spelmap in de instantie. Opent Eidos een
Wabbajack-lijst voor het eerst, dan draait het de eigen kopie van de lijst: de map die haar `ModOrganizer.ini`
noemt, anders een map `Stock Game` of `Game Root`. Freeze this version, op de kaart Game files in Diagnostics,
kopieert de winkelinstallatie naar `<instance>/Stock Game`, zodat een winkelupdate de bestanden die het spel
draait niet meer verandert. Staan beide mappen op één bestandssysteem dat blokken kan delen, zoals btrfs of XFS,
dan deelt de kopie ze met de winkelinstallatie (reflink). Op ext4, of over twee bestandssystemen heen, is het een
volledige kopie, en de grootte en vrije ruimte worden eerst getoond. De winkel start het spel nog steeds.
Return to the store version en Delete the copy maken het ongedaan, en
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]` doet hetzelfde in een
terminal. Morrowind, en Oblivion met `bUseMyGamesDirectory=0`, kunnen niet vanuit een kopie draaien. Details in
[usage.md](docs/guide/usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie).

### Profielen

Elk profiel heeft zijn eigen modvolgorde, pluginstatus, INI's en saves. Saves worden ontleed en vergeleken met je
huidige plugins, met een knop die inschakelt wat een save nodig heeft, en na elke sessie teruggekopieerd naar het
prefix voor Steam Cloud. Saves die Steam Cloud van een ander apparaat bracht, of die een start zonder Eidos in het
prefix achterliet, staan op het tabblad Saves. Import kopieert ze in het profiel, en een save waarvan het profiel de
naam al gebruikt, komt binnen onder een nieuwe naam met `orphan-` in plaats van hem te vervangen.

Een profiel kan startvariabelen en wrappers bevatten (`env=NAME=value` en `wrapper=command`, één per regel, in de
INI-editor) voor zijn spel- en toolruns. Voor Skyrim SE, Enderal SE en Fallout 4 stelt een Display-kaart in
Diagnostics de venstergrootte en -modus in de INI-kopieën van het profiel in. Backups bewaart herstelpunten van de
INI's van het profiel, en Eidos maakt er een vóór elke run van BethINI Pie. De bureaubladsnelkoppeling van een profiel
start het spel via zijn winkel. `eidos sessions` toont de vastgelegde runs en vergelijkt er een met de huidige staat.

### Nexus

Koppel een account om Mod Manager Download-links te ontvangen, updates te controleren en auteurs en hun profielen te
bekijken. Elke mod wordt gecontroleerd, geëndorst en gelinkt op het Nexus-domein waar zijn bestand vandaan kwam, dus
een Skyrim SE-mod in een Enderal SE-instantie blijft op de Skyrim SE-site. Een update volgt de opvolgerketen die de
auteur op Nexus instelde, vervangt de map van de mod door het nieuwe bestand (een back-up van de oude wordt
aangeboden), behoudt zijn prioriteit en of hij aanstaat, en herspeelt de FOMOD-antwoorden die bij de installatie zijn
opgeslagen. `eidos nexus update <instance> --apply` doet hetzelfde in een terminal met een premium-account, en drukt bij
een gratis account de bestandspagina's af. Track en Endorse staan in het modmenu, en elke updatecontrole synchroniseert
ze met je Nexus-account. De informatie van een mod heeft een tabblad Nexus page met zijn beschrijving en changelog als
platte tekst, en zijn tabblad General eindigt met wat de laatste update veranderde: de toegevoegde, verwijderde en in
grootte gewijzigde bestanden, en de toegevoegde of verwijderde masters. Install downloads when they finish (Preferences,
Nexus, standaard uit) installeert een download zodra hij klaar is terwijl het venster open is.

Collecties worden geïnstalleerd vanuit hun recepten, met hervatbare voortgang en expliciete rapporten over niet-ondersteunde
stappen. De vergelijking controleert exacte bronbestanden; onvolledige oudere metadata blijven ongeverifieerd. Het
collectiepaneel bladert in en doorzoekt de collecties die voor een spel zijn gepubliceerd, toont wanneer een geïnstalleerde
collectie een nieuwere revisie heeft, en leidt een gratis account één pagina tegelijk langs de ontbrekende bestanden. Breekt een
sleepactie de volgorderegels van een collectie, dan zegt Diagnostics dat, en Restore collection order zet haar mods terug. Het
tabblad Downloads filtert, sorteert, verbergt en verwijdert archieven. Offline mode (Preferences, Nexus) stopt de toegang tot
Nexus en de dagelijkse releasecontrole.

### Tools

xEdit, BodySlide, DynDOLOD en andere modtools draaien door de samengevoegde weergave binnen het Wine-prefix van het spel (het
Proton-prefix van Steam, of dat wat Heroic heeft ingericht). Ze zien je mods, hun uitvoer landt in Overwrite, en één klik maakt
van die uitvoer een echte mod. Voor Skyrim SE vindt Eidos SSEEdit, SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS,
BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis en BethINI Pie in de spelmap, in een mod, of in de toolsmap die je naast je
spellen bewaart, elk met de runtimes die het nodig heeft. De andere spellen hebben hun eigen lijst. Pandora, Nemesis, FNIS,
BodySlide en Synthesis schrijven hun uitvoer in een benoemde mod (`Nemesis Output` enzovoort).

De releasebundel levert de installatieprogramma's van Visual C++ 2015-2022 en de .NET 8 Desktop Runtime van Microsoft mee, plus de
.NET 10-runtime. Eidos installeert `vcrun2022` en .NET 8 met die installatieprogramma's, zonder winetricks, en zonder de bundel
downloadt het dezelfde bestanden van Microsoft, elk gecontroleerd tegen de SHA-256 die in Eidos is ingebouwd. Synthesis krijgt de
.NET 10 SDK, een download van 300 MB die de bundel niet meelevert. De overige vereisten (.NET Framework 4.8, XACT en oudere
.NET-versies) lopen nog via winetricks. Er wordt niets gedownload tot je vraagt de vereisten van een tool te installeren.

Zet de tools die je gebruikt vast, verberg de andere, geef een tool zijn eigen Steam-AppID wanneer hij zijn eigen Steam-app is, en
schrijf een `.desktop`-snelkoppeling die hem door de samengevoegde weergave start zonder Eidos te openen. `eidos tool <game> import-mo2`
toont de uitvoerbare bestanden in de `ModOrganizer.ini` van de map van een lijst, en `--write` voegt ze toe. Een exe in het tabblad
Data kan eenmalig worden gedraaid of als tool worden toegevoegd, en `eidos tool <instance> run --exe <path>` draait er een vanuit een
terminal. Met Ask before playing vraagt Run eerst wanneer de uitvoer van een generator verouderd is.

Stop beëindigt een draaiend spel of een draaiende tool. Het menu File draait winecfg, regedit, de Wine-bestandsverkenner en het
Windows-verwijderprogramma in het prefix van het spel. Rebuild prefix, in Diagnostics, hernoemt het prefix zodat Steam een nieuw maakt,
en verwijdert niets. `eidos shell <instance>` opent een shell in de samengevoegde weergave, of draait de opdracht na `--`, en wat die
schrijft landt in Overwrite.

Terwijl het venster op de achtergrond staat, kan Eidos een desktopmelding sturen wanneer een toolrun, een collectie-installatie, een pack
of unpack, of de laatste lopende download eindigt, en wanneer een LOOT-sortering mislukt. Notify the desktop when a long job ends, in
Preferences, zet dit aan; het staat standaard uit.

### Diagnostics

Het tabblad Diagnostics toont ontbrekende masters, verweesde archieven, drift in de modlijst en beschadigde pluginsets, en na een run wat het
eigen log van de script extender zegt dat er geladen is. Het meldt ook:

- een script extender die ontbreekt of niet bij de spelversie past
- een spel dat veranderd is sinds de laatste schone start
- een Steam-opstartoptie die Eidos niet start of er geen `%command%` achter heeft
- een Proton die ouder is dan die welke het prefix het laatst inrichtte, of een uit de Flatpak-Steam
- `PROTON_ENABLE_NVAPI` dat voor DLSS ontbreekt, en exclusief volledig scherm, dat de frame generation van Community Shaders tegenhoudt
- een `enblocal.ini` zonder `LinuxVersion=true`
- het crashlog van de laatste sessie en de mods die het noemt
- Nexus-vereisten die ontbreken of zijn uitgeschakeld
- een `nxm://`-handler die een ander programma is of een ontbrekend bestand
- `sTestFile`-regels, waardoor het spel `plugins.txt` negeert
- ingeschakelde `.esp`-plugins in Starfield, en ongeldige of niet-gepaarde blueprintplugins
- een patch waarvan de losse bestanden verliezen van de mod die hij patcht
- een mod waarvan de kopieën van de Papyrus-scripts van de script extender winnen van die van de extender zelf
- een lage `vm.max_map_count` of limiet voor open bestanden
- spelbestanden op een Windows-bestandssysteem
- BodySlide-datapaden die niet de Data-map van het spel zijn, en mappen in `Overwrite/Root` die in Data horen
- de `ModOrganizer.ini` van een lijst die tools naar een andere spelmap laat wijzen
- een spelkopie die ontbreekt of geen spelprogramma heeft

`eidos check <instance> [--json]` draait de controles die geen venster nodig hebben en sluit af met 1 wanneer het een probleem vindt. Copy report
kopieert een rapport met je thuismap geschreven als `~` en je gebruikersnaam als `<user>`, en Save tree op het tabblad Data schrijft de samengevoegde
Data-boom met de bron van elk bestand, op dezelfde manier geredigeerd.

Zie [Integriteitscontroles en gegenereerde bestanden](../../internals/integrity.md) (Engels) voor bescherming bij vervanging, conflicten tussen
BSA/BA2-bestanden, statische SKSE-diagnostiek en generatieregistraties, inclusief hun beperkingen.

### Waar het zijn eigen bestanden bewaart

Eidos gebruikt de indeling die elk programma uit de Colony-familie gebruikt: `~/.config/Colony/Eidos/` voor wat jij gekozen hebt (voorkeuren, je
Nexus-sessie, je instantielijst, de spel- en add-on-definities die je geschreven hebt), `~/.local/share/Colony/Eidos/` voor globale instanties,
tool-runtimes (gedownload, of door `install.sh` uit de bundel gekopieerd) en sessielogs, en `~/.cache/Colony/Eidos/` voor LOOT-masterlists. In een
instantie bevat `Stock Game` een bevroren kopie van het spel en bevat `test-run/` een testrun die op beoordeling wacht.

Een oudere Eidos hield zijn bestanden in `~/.config/eidos/`, `~/.local/share/eidos/` en `~/.local/state/`. De eerste start na het bijwerken neemt ze mee,
meldt dat in het log en laat de oude mappen zoals ze waren. Zolang een verhuizing niet klaar is, blijft Eidos de oude bestanden gebruiken. Details in
[internals/paths.md](../../internals/paths.md) (Engels).

### Ondersteunde spellen

Skyrim SE/AE is bewezen in echt spel. Fallout 4 is ook van begin tot eind aangesloten (F4SE wordt automatisch in de plaats gezet, archiefinvalidatie,
laadvolgorde met asterisken, LOOT, `.fos`-saves); zie [guide/fallout4.md](docs/guide/fallout4.md). Aangesloten en op zoek naar testers: Skyrim LE, Skyrim VR,
Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV, Starfield, Oblivion en Morrowind. Fallout 3, Fallout NV, Oblivion en Morrowind ordenen plugins op de datums
van de bestanden: Eidos houdt één volgorde per profiel bij en toont die datums binnen de mount, zonder de bestanden op schijf te wijzigen.

Vier andere spellen zijn aangesloten en zoeken ook testers: Stellar Blade (Unreal-`.pak`-mods in `~mods`), Stardew Valley (SMAPI-mods, met SMAPI apart
geïnstalleerd), 7 Days to Die (`Mods/`-mappen naast het spel) en Crimson Desert. Crimson Desert neemt alleen codemods: ASI-plugins, ReShade en loader-DLL's in
`bin64`. Zijn datamods, die in PAZ-groepen staan, worden niet ondersteund.

Een spel is één TOML-bestand, en het jouwe kan zonder opnieuw te bouwen in `~/.config/Colony/Eidos/games/`:
[internals/adding-games.md](../../internals/adding-games.md).

## Installatie

### Via Colony (aanbevolen)

Zoek naar **Eidos** in [Colony](https://github.com/Project-Colony/Colony) en installeer het. Updates komen binnen via de launcher. Colony installeert één bestand,
`eidos-linux`: het venster, en ook de opdrachtregel `eidos` wanneer het aangeroepen wordt via een link met de naam `eidos`. Dat bestand bevat noch het
hulpprogramma voor NIF-modelvoorvertoning noch de Microsoft-runtimes van de bundel hieronder. Zonder het hulpprogramma zegt de voorvertoning dat en werkt al het andere,
en Eidos downloadt een runtime van Microsoft wanneer je de vereisten van een tool installeert. Wat je na het installeren doet, en hoe je Steam ernaar laat wijzen:
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Binair bestand direct downloaden

Haal een bestand op van de [nieuwste release](https://github.com/Project-Colony/Eidos/releases/latest):

| Bestand | Wat het is |
|---|---|
| `eidos-linux` | het hele programma in één bestand, zoals Colony het installeert |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, het hulpprogramma voor NIF-voorvertoningen, de installatieprogramma's van Visual C++ 2015-2022 en de .NET 8 Desktop Runtime en de .NET 10-runtime, pictogrammen, licentiekennisgevingen en een installatieprogramma dat het bureaubladitem en de downloadhandler van Nexus registreert |

Controleer de download eerst, zoals het [beleid voor codeondertekening](#beleid-voor-codeondertekening) laat zien, en daarna:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Het installatieprogramma zet de binaire bestanden zonder sudo in `~/.local/bin` en kopieert de map `runtimes/` van de bundel naar `~/.local/share/Colony/Eidos/runtimes`.
`--system` en `--bindir DIR` zetten de binaire bestanden ergens anders, en `--from DIR` haalt ze uit een andere map. `--cap` vraagt sudo om `CAP_SYS_ADMIN`, wat alleen
de experimentele FUSE-passthrough nodig heeft, en passthrough blijft uit; `--no-cap` is de standaard. Beide bestanden vereisen x86_64-Linux met glibc 2.39 of nieuwer,
FUSE (`fusermount3`) en 7-Zip voor modarchieven.

Eidos werkt zichzelf niet bij. Eens per dag vraagt het venster bij de start aan GitHub naar de nieuwste release, en wanneer er een nieuwere is toont het na Help in de
menubalk een link naar de release-opmerkingen. Preferences zet dit uit, en een kopie onder `/usr` buiten `/usr/local`, die een pakketbeheerder bijwerkt, vraagt nooit.

### Bouwen vanuit de broncode

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Vereist Rust 1.92 of nieuwer (de `rust-version` die de CI controleert; CI en releases bouwen met 1.99.1), plus `just`, CMake 3.20+ en een C++17-compiler voor het
hulpprogramma voor NIF-voorvertoningen. `just build` gebruikt het releaseprofiel: één codegenerator-eenheid en fat LTO. Een koude build van `eidos` en `eidos-gui` met 3 buildtaken
duurde zo 334 seconden en 243 seconden met thin LTO, en fat LTO maakt `eidos` 1,2 MB en `eidos-gui` 3,8 MB kleiner (metingen in `Cargo.toml`).
`just profile=dev build` compileert sneller. Een Arch-pakket wordt gebouwd vanuit `packaging/PKGBUILD`. Vereisten en de opdrachten zonder `just`:
[docs/guide/install.md](docs/guide/install.md).

### Steam-opstartopties

Eidos draait als de startopdracht van het spel, en zo koppelt het voordat het spel begint. De basisregel is alles wat de meeste opstellingen nodig hebben, met het absolute pad van je
installatie (Steam leest je `PATH` niet):

```
~/.local/bin/eidos-gui %command%
```

Vanuit Colony is dat pad `~/.local/share/Colony/apps/Eidos/eidos-linux`. Druk daarna op Spelen: Eidos opent op de instantie van dat spel, en Run start het spel door de samengevoegde weergave.

In een venster dat je zelf opende, vraagt Run Steam het spel te starten (`steam -applaunch <appid>`), dus de opstartoptie blijft van toepassing, en je drukt op Run in het venster dat Steam opent. Heroic krijgt
een `heroic://launch`-link, en Legendary draait `legendary launch <app>`, zodra Eidos daar als wrapper van het spel is ingesteld. Ontbreekt de instelling in de winkel, dan start Run niets en zegt wat je moet
instellen: [guide/usage.md](docs/guide/usage.md#het-spel-starten). Met Start the game at once when a store opens Eidos (Preferences, standaard uit) draait Eidos de Diagnostics-controles en start het spel zonder
zijn venster te tonen wanneer niets aandacht nodig heeft.

Al het andere bestaat uit omgevingsvariabelen die ervoor gestapeld worden, en ze laten zich vrij combineren:

| Je wilt... | Zet ervoor |
|---|---|
| DLSS met Community Shaders | `PROTON_ENABLE_NVAPI=1`. Zonder deze variabele initialiseert DLSS nooit en zegt het niets; de volledige checklist is [guide/graphics.md](docs/guide/graphics.md) |
| een fps-teller op het scherm | `DXVK_HUD=fps` |
| frame-interpolatie op driverniveau, nul mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, nooit samen met de eigen frame generation van Community Shaders |
| uitgebreide logs voor een bugrapport | `EIDOS_LOG=debug` (sessielogs belanden in `~/.local/share/Colony/Eidos/logs/`) |
| een I/O-rapport per sessie vanuit de koppeling | `EIDOS_FUSE_STATS=1` |
| een ander aantal FUSE-werkers | `EIDOS_FUSE_THREADS=8` (standaard 4; `1` is het eerste om te proberen bij het jagen op een concurrency-bug) |
| deze start vastgepind op één draagbare instantie | `EIDOS_INSTANCE=/path/to/folder`. Zonder deze variabele opent Eidos de instantie die je het laatst gebruikt hebt |
| het venster voor één start terwijl Start the game at once aan staat | `EIDOS_WINDOW=1` |

De regel om te bewaren voor een moderne gemodde opstelling (Community Shaders, DLSS, frame generation) is de uiteindelijke opdracht, geen voorbeeld:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Zet `DXVK_HUD=fps` ervoor terwijl je controleert of de opstelling werkt, en haal het weg zodra dat zo is.

Dezelfde variabelen kunnen in de startvariabelen van het profiel in plaats van op de Steam-regel, wat ze per profiel houdt en ze ook op toolruns toepast:
[guide/usage.md](docs/guide/usage.md#variabelen-en-wrappers-per-profiel).

De diepere diagnoseschakelaars (`EIDOS_FUSE_TRACE`, de bisectieschakelaars voor cache en index, waarom `EIDOS_FUSE_PASSTHROUGH` standaard uit staat) staan in
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentatie

Alles staat in [docs/](docs/), op lezer ingedeeld in [docs/README.md](docs/README.md): de gebruikersgids (installatie, de CLI en de GUI, tools zoals xEdit en DynDOLOD, graphics,
probleemoplossing), hoe Eidos van binnen werkt, en waarom het bestaat. Beveiligingsbeleid en hoe je een kwetsbaarheid meldt: [SECURITY.md](../../../SECURITY.md).

## Taal

De pagina's die een speler nodig heeft zijn vertaald. Het Engels is de referentie: als een vertaling het ermee oneens is, heeft het Engelse bestand gelijk.

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
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)

Al het andere is Engels. `docs/internals/` en `docs/project/` worden gelezen door mensen die ook de Rust lezen, `CHANGELOG.md` wordt gegenereerd, en `SECURITY.md` is ook niet
vertaald. De eerste twee vertalen zou meer dan 28.000 woorden toevoegen om synchroon te houden voor lezers die ze niet nodig hebben.

Elke vertaling draagt de hash van het Engelse bestand waaruit ze gemaakt is, en de CI faalt zolang die hash niet meer met het Engels overeenkomt; zie
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). De vertaling wordt dan bijgewerkt en opnieuw gestempeld met `scripts/i18n-check.sh --fix`. Een verouderde vertaling
wordt bijgewerkt, niet verwijderd.

Een taal is één map. `docs/i18n/<lang>/` spiegelt de wortel van de repository, dus `docs/i18n/de/docs/guide/install.md` is de Duitse `docs/guide/install.md`. Daarom is een link
tussen twee vertaalde pagina's dezelfde tekenreeks als de link tussen hun Engelse originelen, en is het opheffen van een taal één `rm -r`.

## Beleid voor codeondertekening

Elk releasebestand wordt in de CI ondertekend door de organisatie Project Colony,
nooit op de machine van een ontwikkelaar. Naast elk bestand op de releasepagina:

| Bestand | Wat het is |
|---|---|
| `<asset>.sig` | een ed25519-handtekening over het bestand, gemaakt met de releasesleutel van de organisatie |
| `<asset>.meta` | drie regels die het bestand binden aan zijn bestandsnaam, zijn sha256 en de releaseversie |
| `<asset>.meta.sig` | een ed25519-handtekening over de `.meta` |

De privésleutel is een geheim van de organisatie, dat alleen gebruikt wordt door de gedeelde
[sign-and-publish-workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in een job die niets bouwt; de jobs die Eidos compileren krijgen hem nooit te zien.
Colony controleert alle drie de bestanden voordat het Eidos installeert of bijwerkt,
en weigert een release die ouder is dan de geïnstalleerde. Om een download zelf te
controleren met OpenSSL 3 (dezelfde opdrachten werken voor elk bestand):

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

Hoe releases gebouwd, ondertekend en gepubliceerd worden:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacy

Eidos verstuurt geen telemetrie, geen analysegegevens en geen crashrapporten. Het maakt alleen verbinding met de servers in deze tabel, en alleen voor de handeling die daar genoemd wordt.
Offline mode (Preferences, Nexus) stopt elke verbinding met Nexus en de dagelijkse releasecontrole.

| Gegevens | Opgeslagen of verstuurd | Waar, en waarom |
|---|---|---|
| Voorkeuren, instantielijst, spel- en add-on-definities | opgeslagen | `~/.config/Colony/Eidos/` |
| Instanties, tool-runtimes, sessielogs | opgeslagen | `~/.local/share/Colony/Eidos/`, of de map die je voor een draagbare instantie gekozen hebt |
| LOOT-masterlists | opgeslagen | `~/.cache/Colony/Eidos/loot/` |
| Releasecontrole | verstuurd | Bij de start, hooguit eens per dag, één verzoek aan `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos leest alleen de releasetag en downloadt niets. Het staat standaard aan en uit in Preferences, en wordt overgeslagen in offlinemodus en voor een kopie die onder `/usr` buiten `/usr/local` is geïnstalleerd, die een pakketbeheerder bijwerkt. [Privacyverklaring van GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Aanmelden bij Nexus | verstuurd | Aanmelden opent `users.nexusmods.com` in je browser, die het resultaat teruggeeft aan een luisteraar die Eidos alleen voor die aanmelding op `127.0.0.1` opent. Eidos ruilt het daarna in voor tokens bij `users.nexusmods.com/oauth/token` en haalt de publieke sleutels op waarmee Nexus ze ondertekent. [Privacybeleid van Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus-inloggegevens | opgeslagen | De access- en refreshtokens, in `~/.config/Colony/Eidos/nexus.ini` met modus 0600. Een regel `api_key=` die een oudere Eidos daar geschreven heeft, blijft staan maar wordt nooit gelezen; verwijder hem gerust. |
| Nexus-verzoeken | verstuurd | `api.nexusmods.com`, wanneer je een Nexus-functie gebruikt: je accountnaam en premiumstatus, details van mods en bestanden, updatecontroles, collecties en downloadlinks. Je endorsements en gevolgde mods worden bij elke updatecontrole gelezen en geschreven wanneer je op Endorse of Track drukt. Modvereisten, beschrijvingen en changelogs, en het zoeken naar collecties, gaan naar `api.nexusmods.com/v2/graphql`. Elk verzoek draagt je access-token en noemt Eidos en zijn versie. |
| Nexus-downloads | verstuurd | De bestanden waar je om vraagt, van de CDN-host van Nexus die de downloadlink noemt, opgeslagen in de downloadmap van de instantie met naast elk ervan een `.meta` in MO2-formaat. |
| `nxm://`-links | opgeslagen | `eidos nxm --register`, dat `install.sh` voor je uitvoert, schrijft `~/.local/share/applications/eidos-nxm.desktop` en maakt dat via `xdg-mime` de handler voor `nxm://`, zodat de knop Mod Manager Download van de site zijn link aan Eidos geeft. Het Arch-pakket installeert hetzelfde item systeembreed. |
| LOOT-masterlists | verstuurd | `raw.githubusercontent.com/loot/...`, bij de eerste LOOT-sortering van een spel en telkens wanneer je om een update vraagt. Er wordt niets van jou verstuurd. [Privacyverklaring van GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Tool-runtimes | verstuurd | Alleen wanneer je de vereisten van een tool installeert, en alleen voor wat de releasebundel niet meeleverde (het losse bestand `eidos-linux` en een build uit de broncode leveren niets mee): de .NET 10-runtime van `builds.dotnet.microsoft.com` en de installatieprogramma's van Visual C++ en .NET 8 van `download.visualstudio.microsoft.com`, elk geweigerd tenzij het overeenkomt met de SHA-256 die in Eidos is ingebouwd; de .NET 10 SDK, altijd van `builds.dotnet.microsoft.com` en op dezelfde manier gecontroleerd; en `winetricks` voor de overige vereisten (.NET Framework 4.8, XACT, .NET 6 en 7), dat wat het installeert downloadt van Microsoft en de andere hosts die winetricks gebruikt. [Privacyverklaring van Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Sessielogs | opgeslagen | `~/.local/share/Colony/Eidos/logs/`, modus 0600: tien sessies per instantie, opgeruimd na 30 dagen. Je thuismap wordt geschreven als `~` en je gebruikersnaam als `<user>`, zodat een log zoals het is in een issue geplakt kan worden. Ze verlaten je machine nooit, tenzij je ze zelf verstuurt. |
| Runverslagen | opgeslagen | `<instance>/logs/sessions/`: per spel- of toolrun het profiel, de ingeschakelde mods met hun versies en Nexus-bestands-id's, de pluginvolgorde, een checksum van elke profiel-INI, de versies van het spel en van Eidos, de exitcode en de saves die de run aanmaakte. De 50 nieuwste blijven bewaard, `eidos sessions` leest ze, en `eidos pack` laat ze weg. Ze blijven op je machine. |
| Updateverslagen | opgeslagen | `<instance>/logs/updates/<mod>.json`: wat de laatste update of Replace in de map van die mod veranderde. `eidos pack` laat ze weg. |
| Snelkoppelingen | opgeslagen | `install.sh` schrijft `eidos.desktop` en zijn pictogrammen onder `~/.local/share`; een snelkoppeling naar een tool of profiel die je maakt, komt in `~/.local/share/applications`. |
| Desktopmeldingen | geen van beide | Standaard uit. Wanneer aan, gaan ze via D-Bus (`org.freedesktop.Notifications`) naar de meldingsservice van je desktop en noemen ze de taak die eindigde. |
| Links die je opent | geen van beide | Modpagina's en andere links openen in je eigen browser via `xdg-open`. |
| Add-ons | geen van beide | Extensies die je installeert, draaien als eigen programma's op je machine; wat ze doen is hun zaak. |

## Licentie

De oorspronkelijke code van Eidos valt onder GPL-3.0-or-later; zie [LICENSE](../../../LICENSE). Deze onderdelen vallen onder GPL-3.0 zonder de optie "or later":

- de vertalingen van OMOD-scripts in `eidos-install`; zie hun
  [vermelding van de oorspronkelijke auteurs](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- het hulpprogramma `eidos-nif-preview`, gebouwd op de meegeleverde bibliotheek nifly; zie de [licenties en auteursvermeldingen van zijn onderdelen](../../../crates/eidos-nif-preview/UPSTREAM.md);
- de crates `esplugin` en `libloadorder` waartegen de laadvolgordecode linkt, die `GPL-3.0` opgeven.

De andere Rust-crates in de binaire bestanden behouden hun eigen licenties (MIT, Apache-2.0, BSD en dergelijke). De releasebundel bevat hun vermeldingen in `licenses/crates/`, met een index van elke crate en zijn licentie.

De binaire bestanden bevatten ook acht echte herdistribueerbare DLL's van Microsoft DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` en `d3dx11_43`, 32- en 64-bits), gzip-gecomprimeerd opgeslagen
(7,9 MB in plaats van 17,9 MB), die Eidos uitpakt en in het prefix van het spel plaatst wanneer een mod of tool ze nodig heeft. Ze blijven onder de herdistributievoorwaarden van Microsoft vallen; zie hun
[herkomst](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

De releasebundel bevat onder `runtimes/` ook de installatieprogramma's van Visual C++ 2015-2022 van Microsoft, het installatieprogramma van de .NET 8 Desktop Runtime en de zip van de .NET 10-runtime, zoals Microsoft ze publiceert.
Hun voorwaarden staan in `licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
