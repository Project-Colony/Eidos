<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
Details in [usage.nl.md](docs/guide/usage.nl.md#instanties-globaal-en-draagbaar).

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
de weg via de CLI: **[docs/guide/install.nl.md](docs/guide/install.nl.md)**.

## Steam-opstartopties

De basisregel is alles wat de meeste opstellingen nodig hebben:

```
~/.local/bin/eidos-gui %command%
```

Al het andere bestaat uit omgevingsvariabelen die ervoor gestapeld worden, en ze
laten zich vrij combineren:

| Je wilt... | Zet ervoor |
|---|---|
| DLSS met Community Shaders | `PROTON_ENABLE_NVAPI=1` - zonder haar initialiseert DLSS zich stilzwijgend nooit; de volledige checklist is [guide/graphics.nl.md](docs/guide/graphics.nl.md) |
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
[guide/troubleshooting.nl.md](docs/guide/troubleshooting.nl.md).

## Waar je hierna heen gaat

| Als je wilt... | |
|---|---|
| het installeren | [guide/install.nl.md](docs/guide/install.nl.md) |
| de CLI en de GUI leren | [guide/usage.nl.md](docs/guide/usage.nl.md) |
| xEdit, BodySlide of DynDOLOD instellen | [guide/tools.nl.md](docs/guide/tools.nl.md) |
| Fallout 4 spelen (F4SE, versies, de NVIDIA-debris-crash) | [guide/fallout4.nl.md](docs/guide/fallout4.nl.md) |
| DLSS / frame generation aan de praat krijgen (Community Shaders) | [guide/graphics.nl.md](docs/guide/graphics.nl.md) |
| iets repareren dat er verkeerd uitziet | [guide/troubleshooting.nl.md](docs/guide/troubleshooting.nl.md) |
| weten waarom het snel is, en het zelf nagaan | [internals/performance.md](docs/internals/performance.md) |
| begrijpen hoe het van binnen werkt | [internals/architecture.md](docs/internals/architecture.md) |
| het bouwen, testen, eraan bijdragen | [internals/contributing.md](docs/internals/contributing.md) |
| weten waarom het überhaupt bestaat | [project/landscape.md](docs/project/landscape.md) |

De volledige index staat in [docs/README.nl.md](docs/README.nl.md); het
beveiligingsbeleid en hoe je een kwetsbaarheid meldt in [SECURITY.md](SECURITY.md).

## Taal

De pagina's die een speler nodig heeft zijn vertaald. **Het Engels is
canoniek**: als een vertaling het ermee oneens is, heeft het Engelse bestand
gelijk.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Al het andere is met opzet Engels, niet uit nalatigheid.** `docs/internals/` en
`docs/project/` worden gelezen door mensen die ook de Rust lezen, en `CHANGELOG.md`
wordt gegenereerd. Ze vertalen zou 17.678 woorden extra zijn om eerlijk te houden,
voor een publiek dat ze niet nodig heeft.

Elke vertaling draagt de hash van het Engelse bestand waaruit ze gemaakt is, en de
CI faalt wanneer het Engels vooruitloopt - zie [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Een vertaling die niet weer bij de tijd gebracht kan worden wordt **verwijderd**,
niet laten staan: een verouderde pagina ziet er nog steeds gezaghebbend uit en
deelt de opdrachten van vorige maand uit, wat voor de lezer erger is dan naar het
Engels gestuurd worden.

Een taal toevoegen is vier bestanden en een rij in deze tabel;
[`docs/internals/contributing.md`](docs/internals/contributing.md) heeft de stappen.

## Ondersteunde spellen

**Skyrim SE/AE** - bewezen in echt spel. **Fallout 4** is ook van begin tot eind
aangesloten (F4SE wordt automatisch in de plaats gezet, archiefinvalidatie,
laadvolgorde met asterisken, LOOT, `.fos`-saves) - zie
[guide/fallout4.nl.md](docs/guide/fallout4.nl.md). Aangesloten volgens de
gedeelde speldescriptor en op zoek naar testers: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion en Morrowind
(die laatste twee koppelen en beheren mods; hun op tijdstempel geordende
pluginlijsten worden nog niet beheerd).

Een familie toevoegen is één descriptorregel:
[internals/adding-games.md](docs/internals/adding-games.md).

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
