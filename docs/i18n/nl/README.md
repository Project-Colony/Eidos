<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

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

### Hoe het zich verhoudt

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / uitrollers met links |
|---|---|---|---|---|
| Manager draait native | ✅ | ❌ Windows-app in Wine | ✅ (Qt-port) | ✅ |
| Spelmap onaangeroerd | ✅ altijd | ✅ | ✅ | ❌ er worden links in geschreven |
| Koppeling zichtbaar voor | alleen het spel | alleen het spel | **het hele systeem** | n.v.t. |
| Opruimen na crash nodig | geen, bij ontwerp | geen | herstel van een dode koppeling | handmatig uitrollen ongedaan maken |
| Root-mods (ENB, preloaders) | ✅ native | plugin vereist | plugin vereist | gedeeltelijk |
| Vereiste privileges | geen | geen | `/etc/fuse.conf` aanpassen | geen |

### Hoe snel het is

| | voorheen | nu |
|---|---|---|
| een save laden | ~20 seconden | **6-7 seconden** |
| maplezingen in één sessie | 5,6 miljoen | 465 duizend |

Celovergangen zijn onmiddellijk. De winst kwam van je mods minder vragen te
stellen: één bestand vinden ondervroeg vroeger alle vijftig op hun beurt, en één
map opsommen deed dat vijftig keer over. Geen van beide doet dat nog. Gemeten op
een echte instantie die normaal gespeeld werd, niet op een benchmark.

### Eerder werk en dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) en
  [usvfs](https://github.com/ModOrganizer2/usvfs) - de semantiek die Eidos
  reproduceert, en de codebase waartegen zijn pariteit bestudeerd is
- [LOOT](https://loot.github.io/) - de sorteermotor, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) en de andere Linux-managers - het
  bewijs dat er een gemeenschap is die dit opgelost wil zien

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

**Instanties.** Globaal - centraal beheerd onder `~/.local/share/Colony/Eidos/instances/` - of
draagbaar: een op zichzelf staande map waar je maar wilt (een tweede schijf, een
spellenpartitie), verplaatsbaar en geïsoleerd, zoals die van MO2. Draagbare
instanties worden van sessie tot sessie onthouden; de GUI, de Steam-start en elke
CLI-opdracht volgen degene die je het laatst gebruikt hebt, en elke opdracht
neemt de map aan waar ze ook een spel-id aanneemt.
Details in [usage.md](docs/guide/usage.md#instanties-globaal-en-draagbaar).

**Profielen.** Modvolgorde, pluginstatus, INI's en saves per profiel. Saves
worden ontleed, vergeleken met je huidige plugins - met een knop die inschakelt
wat een save nodig heeft - en na elke sessie teruggesynchroniseerd voor Steam
Cloud.

**Nexus.** Koppel een account om Mod Manager Download-links te ontvangen, updates te controleren en auteurs en hun profielen te bekijken. Collecties worden vanuit hun recepten geïnstalleerd, met hervatbare voortgang en duidelijke rapporten over niet-ondersteunde stappen. De vergelijking controleert de exacte bronbestanden; onvolledige oudere metadata blijven ongeverifieerd. Het tabblad Downloads kan archieven filteren, sorteren, verbergen en verwijderen. Een **offline**-schakelaar stopt de toegang tot Nexus.

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

**Waar het zijn eigen bestanden bewaart.** De indeling die elk programma uit de
Colony-familie gebruikt: `~/.config/Colony/Eidos/` voor wat jij gekozen hebt -
voorkeuren, je Nexus-sessie, je instantielijst, de spel- en add-on-definities
die je geschreven hebt - `~/.local/share/Colony/Eidos/` voor globale instanties,
gedownloade runtimes en sessielogs, en `~/.cache/Colony/Eidos/` voor
LOOT-masterlists. Een oudere Eidos hield die in `~/.config/eidos/`,
`~/.local/share/eidos/` en `~/.local/state/`; de eerste start na het bijwerken
zet ze over, meldt dat in het log, en laat de oude mappen precies zoals ze
waren. Zolang een verhuizing niet klaar is, blijft Eidos de oude bestanden
gebruiken. Details in [internals/paths.md](../../internals/paths.md) (Engels).

Zie [Integriteitscontroles en gegenereerde bestanden](../../../docs/internals/integrity.md) (Engels) voor bescherming bij vervanging, conflicten tussen BSA/BA2-bestanden, statische SKSE-diagnostiek en generatieregistraties, inclusief hun beperkingen.

### Ondersteunde spellen

**Skyrim SE/AE** - bewezen in echt spel. **Fallout 4** is ook van begin tot eind
aangesloten (F4SE wordt automatisch in de plaats gezet, archiefinvalidatie,
laadvolgorde met asterisken, LOOT, `.fos`-saves) - zie
[guide/fallout4.md](docs/guide/fallout4.md). Aangesloten volgens de
gedeelde speldescriptor en op zoek naar testers: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion en Morrowind
(die laatste twee koppelen en beheren mods; hun op tijdstempel geordende
pluginlijsten worden nog niet beheerd).

Een familie toevoegen is één descriptorregel:
[internals/adding-games.md](../../internals/adding-games.md).

## Installatie

### Via Colony (aanbevolen)

Zoek naar **Eidos** in [Colony](https://github.com/Project-Colony/Colony) en
installeer het. Updates komen binnen via de launcher. Colony installeert één
bestand, `eidos-linux`: het venster, en ook de opdrachtregel `eidos` wanneer het
aangeroepen wordt via een link met de naam `eidos`. De NIF-modelvoorvertoning
heeft een hulpprogramma nodig dat alleen de bundel hieronder meelevert; zonder
dat programma zegt de voorvertoning dat en werkt al het andere. Wat je na het
installeren doet, en hoe je Steam ernaar laat wijzen:
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Binair bestand direct downloaden

Haal een bestand op van de [nieuwste release](https://github.com/Project-Colony/Eidos/releases/latest):

| Bestand | Wat het is |
|---|---|
| `eidos-linux` | het hele programma in één bestand, zoals Colony het installeert |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, het hulpprogramma voor NIF-voorvertoningen, pictogrammen en een installatieprogramma dat het bureaubladitem en de downloadhandler van Nexus registreert |

Controleer de download eerst, zoals het
[beleid voor codeondertekening](#beleid-voor-codeondertekening) laat zien, en
daarna:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Het installatieprogramma zet alles in `~/.local/bin`, zonder sudo; `--system` en
`--bindir DIR` kiezen een andere plek. Beide bestanden vereisen x86_64-Linux met
glibc 2.39 of nieuwer, FUSE (`fusermount3`) en 7-Zip voor modarchieven.

### Bouwen vanuit de broncode

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Vereist Rust 1.89 of nieuwer (de `rust-version` die de CI controleert; CI en
releases bouwen met 1.99.0), plus `just`, CMake 3.20+ en een C++17-compiler voor
het hulpprogramma voor NIF-voorvertoningen. Een Arch-pakket wordt gebouwd vanuit
`packaging/PKGBUILD`. Vereisten en de opdrachten zonder `just`:
[docs/guide/install.md](docs/guide/install.md).

### Steam-opstartopties

Eidos draait *als* de startopdracht van het spel, en zo koppelt het voordat het
spel begint. De basisregel is alles wat de meeste opstellingen nodig hebben, met
het absolute pad van je installatie (Steam leest je `PATH` niet):

```
~/.local/bin/eidos-gui %command%
```

Vanuit Colony is dat pad `~/.local/share/Colony/apps/Eidos/eidos-linux`. Druk
daarna op Spelen: Eidos opent op de instantie van dat spel, en Run start het spel
door de samengevoegde weergave.

Al het andere bestaat uit omgevingsvariabelen die ervoor gestapeld worden, en ze
laten zich vrij combineren:

| Je wilt... | Zet ervoor |
|---|---|
| DLSS met Community Shaders | `PROTON_ENABLE_NVAPI=1` - zonder haar initialiseert DLSS zich stilzwijgend nooit; de volledige checklist is [guide/graphics.md](docs/guide/graphics.md) |
| een fps-teller op het scherm | `DXVK_HUD=fps` |
| frame-interpolatie op driverniveau, nul mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nooit samen met de eigen frame generation van Community Shaders |
| uitgebreide logs voor een bugrapport | `EIDOS_LOG=debug` (sessielogs belanden in `~/.local/share/Colony/Eidos/logs/`) |
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
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentatie

Alles staat in [docs/](docs/), op lezer ingedeeld in
[docs/README.md](docs/README.md): de gebruikersgids (installatie, de CLI en de
GUI, tools zoals xEdit en DynDOLOD, graphics, probleemoplossing), hoe Eidos van
binnen werkt, en waarom het bestaat. Beveiligingsbeleid en hoe je een
kwetsbaarheid meldt: [SECURITY.md](../../../SECURITY.md).

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
wordt gegenereerd. Ze vertalen zou 18.536 woorden extra zijn om eerlijk te houden,
voor een publiek dat ze niet nodig heeft.

Elke vertaling draagt de hash van het Engelse bestand waaruit ze gemaakt is, en de
CI faalt wanneer het Engels vooruitloopt - zie [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Een vertaling die niet weer bij de tijd gebracht kan worden, wordt **verwijderd**
en niet laten staan: een verouderde pagina ziet er nog steeds gezaghebbend uit en
deelt de opdrachten van vorige maand uit, wat voor de lezer erger is dan naar het
Engels gestuurd worden.

Een taal is één map. `docs/i18n/<lang>/` spiegelt de wortel van de repository, dus
`docs/i18n/de/docs/guide/install.md` is de Duitse `docs/guide/install.md` - en dat
maakt een link tussen twee vertaalde pagina's DEZELFDE tekenreeks als de link
tussen hun Engelse originelen, en maakt het opheffen van een taal één `rm -r`.

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

Eidos verstuurt geen telemetrie, geen analysegegevens en geen crashrapporten. Het
maakt met geen enkele server verbinding behalve die in deze tabel, en alleen voor
de handeling die daar genoemd wordt. De **offline**-modus (Preferences -> Nexus)
stopt elke verbinding met Nexus.

| Gegevens | Opgeslagen of verstuurd | Waar, en waarom |
|---|---|---|
| Voorkeuren, instantielijst, spel- en add-on-definities | opgeslagen | `~/.config/Colony/Eidos/` |
| Instanties, gedownloade runtimes, sessielogs | opgeslagen | `~/.local/share/Colony/Eidos/`, of de map die je voor een draagbare instantie gekozen hebt |
| LOOT-masterlists | opgeslagen | `~/.cache/Colony/Eidos/loot/` |
| Aanmelden bij Nexus | verstuurd | Aanmelden opent `users.nexusmods.com` in je browser, die het resultaat teruggeeft aan een luisteraar die Eidos alleen voor die aanmelding op `127.0.0.1` opent. Eidos ruilt het daarna in voor tokens bij `users.nexusmods.com/oauth/token` en haalt de publieke sleutels op waarmee Nexus ze ondertekent. [Privacybeleid van Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus-inloggegevens | opgeslagen | De access- en refreshtokens, in `~/.config/Colony/Eidos/nexus.ini` met modus 0600. Een regel `api_key=` die een oudere Eidos daar geschreven heeft, blijft staan maar wordt nooit gelezen; verwijder hem gerust. |
| Nexus-verzoeken | verstuurd | `api.nexusmods.com`, wanneer je een Nexus-functie gebruikt: je accountnaam en premiumstatus, details van mods en bestanden, updatecontroles, collecties en downloadlinks. Elk verzoek draagt je access-token en noemt Eidos en zijn versie. |
| Nexus-downloads | verstuurd | De bestanden waar je om vraagt, van de CDN-host van Nexus die de downloadlink noemt, opgeslagen in de downloadmap van de instantie met naast elk ervan een `.meta` in MO2-formaat. |
| `nxm://`-links | opgeslagen | `eidos nxm --register`, dat `install.sh` voor je uitvoert, schrijft `~/.local/share/applications/eidos-nxm.desktop` en maakt dat via `xdg-mime` de handler voor `nxm://`, zodat de knop Mod Manager Download van de site zijn link aan Eidos geeft. Het Arch-pakket installeert hetzelfde item systeembreed. |
| LOOT-masterlists | verstuurd | `raw.githubusercontent.com/loot/...`, bij de eerste LOOT-sortering van een spel en telkens wanneer je om een update vraagt. Er wordt niets van jou verstuurd. [Privacyverklaring van GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Tool-runtimes | verstuurd | Alleen wanneer je een vereiste voor een tool installeert of een mod er een nodig heeft: de .NET-runtime van `builds.dotnet.microsoft.com`, geweigerd tenzij die overeenkomt met de SHA-256 die in Eidos is ingebouwd, en `winetricks`, dat wat het in het Proton-prefix van het spel installeert (herdistribueerbare pakketten van Visual C++ en .NET) downloadt van Microsoft en de andere hosts die winetricks gebruikt. [Privacyverklaring van Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Sessielogs | opgeslagen | `~/.local/share/Colony/Eidos/logs/`, modus 0600: tien sessies per instantie, opgeruimd na 30 dagen. Je thuismap wordt geschreven als `~` en je gebruikersnaam als `<user>`, zodat een log zoals het is in een issue geplakt kan worden. Ze verlaten je machine nooit, tenzij je ze zelf verstuurt. |
| Snelkoppelingen | opgeslagen | `install.sh` schrijft `eidos.desktop` en zijn pictogrammen onder `~/.local/share`; een snelkoppeling naar een tool die je maakt, komt in `~/.local/share/applications`. |
| Links die je opent | geen van beide | Modpagina's en andere links openen in je eigen browser via `xdg-open`. |
| Add-ons | geen van beide | Extensies die je installeert, draaien als eigen programma's op je machine; wat ze doen is hun zaak. |

## Licentie

De oorspronkelijke code van Eidos valt onder GPL-3.0-or-later; zie
[LICENSE](../../../LICENSE). Deze onderdelen vallen onder GPL-3.0 zonder de optie
"or later":

- de vertalingen van OMOD-scripts in `eidos-install`; zie hun
  [vermelding van de oorspronkelijke auteurs](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- het hulpprogramma `eidos-nif-preview`, gebouwd op de meegeleverde bibliotheek
  nifly; zie de [licenties en auteursvermeldingen van zijn onderdelen](../../../crates/eidos-nif-preview/UPSTREAM.md);
- de crates `esplugin` en `libloadorder` waartegen de laadvolgordecode linkt, die
  `GPL-3.0` opgeven.

De andere Rust-crates in de binaire bestanden behouden hun eigen licenties (MIT,
Apache-2.0, BSD en dergelijke). De releasebundel bevat hun vermeldingen in
`licenses/crates/`, met een index van elke crate en zijn licentie.

De binaire bestanden bevatten ook acht echte herdistribueerbare DLL's van
Microsoft DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` en `d3dx11_43`,
32- en 64-bits), die Eidos in het prefix van het spel plaatst wanneer een mod of
tool ze nodig heeft. Ze blijven onder de herdistributievoorwaarden van Microsoft
vallen; zie hun [herkomst](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Modbeheer is van iedereen.
