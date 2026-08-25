<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**De native Linux-modmanager die je spel nooit aanraakt.**

</div>

Eidos geeft Bethesda-spellen op Linux wat Mod Organizer 2 ze op Windows geeft -
een virtuele, bij elke start opnieuw samengevoegde weergave van je mods -
gebouwd uit Linux-primitieven in plaats van uit het onderscheppen van
Windows-API's. Geen Wine voor de manager. Geen bestanden die naar de spelmap
gekopieerd worden. Geen opruimpad, want er valt niets op te ruimen.

```
Steam ──> eidos-gui %command% ──> [ privénaamruimte ]
                                  │  mods ⊕ spel  ──> wat het spel ziet
                                  └─ sterft met het spel; de installatie blijft ongerept
```

> **Status:** Skyrim SE wordt dagelijks via Eidos gespeeld - SKSE,
> preloaders van script extenders, Creation Club, met LOOT gesorteerde
> laadvolgordes, saves per profiel, alles. Eén spelfamilie tot nu toe in echt
> spel bewezen; tien andere zijn aangesloten en wachten op testers.

## Waarom Eidos

- 🔒 **Een koppeling die alleen je spel ziet.** De samengevoegde weergave leeft
  in een privénaamruimte voor koppelingen: je bestandsbeheerder, je back-uptaak,
  een tweede spel - geen van alle ziet haar, geen van alle heeft er toestemming
  voor nodig. Dood het spel, trek de stekker eruit: de naamruimte sterft met de
  procesboom en je installatie is precies zoals ze was. Er is *per constructie*
  geen residu.
- 🧾 **Eén kopie van de waarheid.** Je profiel bezit zijn eigen modlijst,
  pluginvolgorde, INI's en saves. De pluginbestanden en de savemap worden bij de
  start met een bind-mount over de eigen paden van het spel gelegd, zodat zelfs
  de schrijfacties van het spel zelf in je profiel belanden. Van profiel wisselen
  wisselt alles.
- 🐧 **Volledig zonder root.** Geen setuid-helper, geen daemon, geen
  `sudo setcap`, geen aanpassingen aan `/etc/fuse.conf`. Eén binair bestand, één
  Steam-opstartoptie.
- 🛡️ **Beveiligingen met bewijs.** Een crash die je pluginlijst sloopt wordt
  gesignaleerd tegen een momentopname van voor de sessie, met een herstel in één
  klik. Een capture die je laadvolgorde zou wissen wordt geweigerd, met de reden
  erbij.

## Wat het doet

**Mods.** Simpele archieven, FOMOD-wizards, BAIN-pakketten van Wrye Bash, een
handmatige kiezer voor de rest - en **root-mods native** (preloaders van script
extenders, ENB, Engine Fixes), zonder Root Builder-plugin en zonder iets naar je
installatie te kopiëren. Verberg losse bestanden, groepeer met scheidingen,
gerichte verplaatsingen, notities en categorieën per mod, en een importeur van
MO2-profielen.

De lijst is die van MO2, met haar gewoontes: acht optionele kolommen en sorteren
op elk ervan, groeperen per categorie of per bron, gebaren met dubbelklik, typen
om te springen, back-ups per mod die inert zijn tot je ze terugzet, en
adviserende vlaggen voor een mod waarvan de indeling door dit spel niet geladen
wordt of die voor een ander spel gedownload is. Haar bestandsboom doet de gewone
bewerkingen - nieuwe map, hernoemen, verwijderen, openen - en toont voorbeelden
van afbeeldingen en tekst zonder iets te starten.

**Plugins.** De laadvolgorde met LOOT-sortering ingebouwd, modindexen zoals het
spel ze berekent, waarschuwingen voor ontbrekende masters, en je DLC en Creation
Club-inhoud getoond als de onbeheerde rijen die ze zijn.

**Instanties.** Globaal - centraal beheerd onder `~/.local/share/eidos` - of
draagbaar: een op zichzelf staande map waar je maar wilt (een tweede schijf, een
spellenpartitie), verplaatsbaar en geïsoleerd, zoals die van MO2. Draagbare
instanties worden van sessie tot sessie onthouden; de GUI, de Steam-start en elke
CLI-opdracht volgen degene die je het laatst gebruikt hebt, en elke opdracht
neemt de map aan waar ze ook een spel-id aanneemt.
Details in [usage.nl.md](docs/guide/usage.md#instanties-globaal-en-draagbaar).

**Profielen.** Modvolgorde, pluginstatus, INI's en saves per profiel. Saves
worden ontleed, vergeleken met je huidige plugins - met een knop die inschakelt
wat een save nodig heeft - en na elke sessie teruggesynchroniseerd voor Steam
Cloud.

**Nexus.** Koppel een account en de knop "Mod Manager Download" van de site landt
rechtstreeks in je instantie, met updatecontroles tegen wat je geïnstalleerd
hebt, wie elke mod gemaakt heeft en een link naar zijn profiel. Een
**collectie**-link somt haar leden op, gekruist met je instantie - geïnstalleerd,
gedownload, ontbrekend - wat neerkomt op een collectie lezen in plaats van er een
installeren, en het paneel zegt waarom. Het tabblad Downloads is een
archiefbibliotheek: filteren, sorteren, verbergen zonder verwijderen, en de al
geïnstalleerde opruimen. Een **offline**-schakelaar zet dat alles stil.

**Tools.** xEdit, BodySlide, DynDOLOD en consorten draaien *door de samengevoegde
weergave* binnen het Proton-prefix van het spel - ze zien je mods, hun uitvoer
belandt in de Overwrite, en één klik maakt er een echte mod van. Welke runtime
elk ervan nodig heeft wordt op verzoek opgehaald, zodat een ontbrekende DLL een
knop is in plaats van een middag. xEdit en zijn QuickAutoClean-tweeling worden
voor je gevonden - in de spelmap, in een mod, of in de toolsmap die je naast je
spellen bewaart - met de juiste runtimes al gekozen. Pin degene die je gebruikt
vast, verberg degene die je niet gebruikt, geef een tool zijn eigen
Steam-AppID wanneer hij zijn eigen Steam-app is, en schrijf een
`.desktop`-snelkoppeling die hem door de samengevoegde weergave start zonder
Eidos ook maar te openen.

**Diagnostics.** Ontbrekende masters, verweesde archieven, drift in de modlijst,
beschadigde pluginsets - en, na een run, wat het eigen log van de script extender
zegt dat er werkelijk geladen is.

**Waar het zijn eigen bestanden bewaart.** `~/.config/Colony/Eidos/` voor wat jij
gekozen hebt - voorkeuren, je Nexus-sessie, je instantielijst, de spel- en
add-on-definities die je geschreven hebt - met logs onder
`~/.local/state/Colony/Eidos/`. De indeling die elk programma uit de
Colony-familie gebruikt. Een oudere Eidos hield die in `~/.config/eidos/`; de
eerste start na het bijwerken kopieert ze over, meldt dat in het log, en laat de
oude map precies zoals ze was.

## Hoe het zich verhoudt

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / uitrollers met links |
|---|---|---|---|---|
| Manager draait native | ✅ | ❌ Windows-app in Wine | ✅ (Qt-port) | ✅ |
| Spelmap onaangeroerd | ✅ altijd | ✅ | ✅ | ❌ er worden links in geschreven |
| Koppeling zichtbaar voor | alleen het spel | alleen het spel | **het hele systeem** | n.v.t. |
| Opruimen na crash nodig | geen, bij ontwerp | geen | herstel van een dode koppeling | handmatig uitrollen ongedaan maken |
| Root-mods (ENB, preloaders) | ✅ native | plugin vereist | plugin vereist | gedeeltelijk |
| Vereiste privileges | geen | geen | `/etc/fuse.conf` aanpassen | geen |

## Hoe snel het is

| | voorheen | nu |
|---|---|---|
| een save laden | ~20 seconden | **6-7 seconden** |
| maplezingen in één sessie | 5,6 miljoen | 465 duizend |

Celovergangen zijn onmiddellijk. De winst kwam van je mods minder vragen te
stellen: één bestand vinden ondervroeg vroeger alle vijftig op hun beurt, en één
map opsommen deed dat vijftig keer over. Geen van beide doet dat nog. Gemeten op
een echte instantie die normaal gespeeld werd, niet op een benchmark.

## Aan de slag

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Zet daarna de Steam-opstartoptie van je spel op
`~/.local/bin/eidos-gui %command%` en druk op Spelen.

Arch-pakketten en release-archieven, wat je eerst geïnstalleerd moet hebben, en
de weg via de CLI: **[docs/guide/install.nl.md](docs/guide/install.md)**.

## Steam-opstartopties

De basisregel is alles wat de meeste opstellingen nodig hebben:

```
~/.local/bin/eidos-gui %command%
```

Al het andere bestaat uit omgevingsvariabelen die ervoor gestapeld worden, en ze
laten zich vrij combineren:

| Je wilt... | Zet ervoor |
|---|---|
| DLSS met Community Shaders | `PROTON_ENABLE_NVAPI=1` - zonder haar initialiseert DLSS zich stilzwijgend nooit; de volledige checklist is [guide/graphics.nl.md](docs/guide/graphics.md) |
| een fps-teller op het scherm | `DXVK_HUD=fps` |
| frame-interpolatie op driverniveau, nul mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nooit samen met de eigen frame generation van Community Shaders |
| uitgebreide logs voor een bugrapport | `EIDOS_LOG=debug` (sessielogs belanden in `~/.local/state/Colony/Eidos/logs/`) |
| een I/O-rapport per sessie vanuit de koppeling | `EIDOS_FUSE_STATS=1` |
| een ander aantal FUSE-werkers | `EIDOS_FUSE_THREADS=8` (standaard 4; `1` is het eerste om te proberen bij het jagen op een concurrency-bug) |
| deze start vastgepind op één draagbare instantie | `EIDOS_INSTANCE=/path/to/folder` - zonder haar opent Eidos de instantie die je het laatst gebruikt hebt, wat meestal is wat je wilt |

De regel om te bewaren voor een moderne gemodde opstelling (Community Shaders,
DLSS, frame generation) - dit is de uiteindelijke opdracht, geen voorbeeld:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Zet `DXVK_HUD=fps` ervoor terwijl je controleert of de opstelling werkt, en haal
het weg zodra dat zo is.

De diepere diagnoseschakelaars (`EIDOS_FUSE_TRACE`, de bisectieschakelaars voor
cache en index, waarom `EIDOS_FUSE_PASSTHROUGH` standaard uit staat) staan in
[guide/troubleshooting.nl.md](docs/guide/troubleshooting.md).

## Waar je hierna heen gaat

| Als je wilt... | |
|---|---|
| het installeren | [guide/install.nl.md](docs/guide/install.md) |
| de CLI en de GUI leren | [guide/usage.nl.md](docs/guide/usage.md) |
| xEdit, BodySlide of DynDOLOD instellen | [guide/tools.nl.md](docs/guide/tools.md) |
| Fallout 4 spelen (F4SE, versies, de NVIDIA-debris-crash) | [guide/fallout4.nl.md](docs/guide/fallout4.md) |
| DLSS / frame generation aan de praat krijgen (Community Shaders) | [guide/graphics.nl.md](docs/guide/graphics.md) |
| iets repareren dat er verkeerd uitziet | [guide/troubleshooting.nl.md](docs/guide/troubleshooting.md) |
| weten waarom het snel is, en het zelf nagaan | [internals/performance.md](../../internals/performance.md) |
| begrijpen hoe het van binnen werkt | [internals/architecture.md](../../internals/architecture.md) |
| het bouwen, testen, eraan bijdragen | [internals/contributing.md](../../internals/contributing.md) |
| weten waarom het überhaupt bestaat | [project/landscape.md](../../project/landscape.md) |

Een taal is één map: `docs/i18n/nl/` spiegelt de wortel van de repository, waardoor
een link tussen twee vertaalde pagina's dezelfde tekenreeks is als de link tussen
hun Engelse originelen.

## Taal

De pagina's die een speler nodig heeft zijn vertaald. **Het Engels is
canoniek**: als een vertaling het ermee oneens is, heeft het Engelse bestand
gelijk.

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

**Al het andere is met opzet Engels, niet uit nalatigheid.** `docs/internals/` en
`docs/project/` worden gelezen door mensen die ook de Rust lezen, en `CHANGELOG.md`
wordt gegenereerd. Ze vertalen zou 17.678 woorden extra zijn om eerlijk te houden,
voor een publiek dat ze niet nodig heeft.

Elke vertaling draagt de hash van het Engelse bestand waaruit ze gemaakt is, en de
CI faalt wanneer het Engels vooruitloopt - zie [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Een vertaling die niet weer bij de tijd gebracht kan worden wordt **verwijderd**,
niet laten staan: een verouderde pagina ziet er nog steeds gezaghebbend uit en
deelt de opdrachten van vorige maand uit, wat voor de lezer erger is dan naar het
Engels gestuurd worden.

Een taal toevoegen is vier bestanden en een rij in deze tabel;
[`docs/internals/contributing.md`](../../internals/contributing.md) heeft de stappen.

## Ondersteunde spellen

**Skyrim SE/AE** - bewezen in echt spel. **Fallout 4** is ook van begin tot eind
aangesloten (F4SE wordt automatisch in de plaats gezet, archiefinvalidatie,
laadvolgorde met asterisken, LOOT, `.fos`-saves) - zie
[guide/fallout4.nl.md](docs/guide/fallout4.md). Aangesloten volgens de
gedeelde speldescriptor en op zoek naar testers: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion en Morrowind
(die laatste twee koppelen en beheren mods; hun op tijdstempel geordende
pluginlijsten worden nog niet beheerd).

Een familie toevoegen is één descriptorregel:
[internals/adding-games.md](../../internals/adding-games.md).

## Eerder werk en dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) en
  [usvfs](https://github.com/ModOrganizer2/usvfs) - de semantiek die Eidos
  reproduceert, en de codebase waartegen zijn pariteit bestudeerd is
- [LOOT](https://loot.github.io/) - de sorteermotor, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) en de andere Linux-managers - het
  bewijs dat er een gemeenschap is die dit opgelost wil zien

## Licentie

GPL-3.0-or-later. Modbeheer is van iedereen.
