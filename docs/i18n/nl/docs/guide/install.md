<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Eidos installeren

Vier manieren om binnen te komen. Colony installeert één bestand, `eidos-linux`, dat het hele programma is; de andere leveren `eidos` (de opdrachtregel), `eidos-gui` en het hulpprogramma voor statische NIF-voorvertoningen naast elkaar, plus de `nxm://`-handler waardoor de knop "Mod Manager Download" op Nexus in jouw instantie landt.

## Wat je eerst nodig hebt

| | |
|---|---|
| **Linux met FUSE** | `fusermount3` in je PATH. Elke huidige distributie levert het mee. |
| **Een Proton-spel dat één keer gestart is** | Steam maakt het Wine-prefix van het spel pas bij de eerste start aan, en Eidos werkt daarbinnen. |
| **`7z`** | Voor het installeren van modarchieven. `p7zip` op de meeste distributies. |

Geen root, geen daemon, geen aanpassing van `/etc/fuse.conf` en niets om aan je
groepen toe te voegen. Eidos koppelt binnen een privénaamruimte die van het
spelproces is.

## Via Colony

Zoek naar **Eidos** in [Colony](https://github.com/Project-Colony/Colony) en
installeer het; updates komen binnen via de launcher. Colony bewaart één bestand,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Voer het uit en het venster gaat
open. Aangeroepen via een link met de naam `eidos` is hetzelfde bestand de
opdrachtregel. Zet er een in je `PATH` en registreer de downloadhandler van Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

Het venster heeft geen link nodig: telkens wanneer het een spel of een tool start,
voert het zijn eigen bestand uit als de CLI. De NIF-modelvoorvertoning heeft
`eidos-nif-preview` nodig, dat alleen de releasebundel en builds vanuit de
broncode meeleveren; zonder dat programma zegt de voorvertoning dat, en verder
wordt niets geraakt. Laat Steam naar het bestand zelf wijzen, zoals hieronder.

## Een releasebundel

Download `eidos-linux-bundle.tar.gz` en de bijbehorende `.sig`, `.meta` en
`.meta.sig` van de [nieuwste release](https://github.com/Project-Colony/Eidos/releases/latest),
en controleer ze zoals het
[beleid voor codeondertekening](../../README.md#beleid-voor-codeondertekening) in de README laat zien. Daarna:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Installeert standaard in `~/.local/bin`. `--system` zet het in `/usr/local/bin`,
`--bindir DIR` ergens anders. Opnieuw uitvoeren is de bedoelde manier om bij te
werken. De bundel en `eidos-linux` vereisen x86_64-Linux met glibc 2.39 of nieuwer.

## Arch

```bash
cd packaging && makepkg -si
```

De PKGBUILD bouwt de huidige `main` vanuit de broncode.

## Vanaf de broncode

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Bouwen vanuit broncode vereist Rust 1.89 of nieuwer, `just`, CMake 3.20+, een C++17-compiler en Python 3 voor de tests van het native hulpprogramma. Zie de [instructies voor het native hulpprogramma](../../../../../crates/eidos-nif-preview/README.md) voor opdrachten zonder `just`, bouw Rust met `cargo build --release --locked`, kopieer het hulpprogramma naar `target/release` en voer daarna `packaging/install.sh --from target/release` uit.

## Daarna: Steam ernaar laten wijzen

Eidos draait *als* de startopdracht van je spel, en zo komt het ertoe te
koppelen voordat het spel begint. Rechtsklik in Steam op het spel ->
Eigenschappen -> Opstartopties:

```
~/.local/bin/eidos-gui %command%
```

Vanuit Colony is het pad `~/.local/share/Colony/apps/Eidos/eidos-linux`. Druk op
Spelen. Eidos opent op de instantie van dat spel; installeer mods,
sorteer met LOOT, klik op Run. Bij het afsluiten verdwijnt de koppeling ermee en
je installatie is precies zoals ze was.

Gebruik het absolute pad - Steam leest het `PATH` van je shell niet.

### Als je de terminal verkiest

```sh
eidos init skyrimse               # een instantie maken (geef een map op om haar draagbaar te maken)
eidos install skyrimse mod.7z     # Simple- / FOMOD- / BAIN- / root-mods
eidos sort skyrimse               # de laadvolgorde met LOOT sorteren
eidos play skyrimse -- %command%  # wat dan ook door de samengevoegde weergave draaien
```

Elke opdracht die een spel-id aanneemt, neemt ook de map van een draagbare
instantie - zie [usage.md](usage.md#instanties-globaal-en-draagbaar). De
volledige rondleiding staat in [usage.md](usage.md).

## Optioneel: FUSE-passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` verleent de optionele capability, maar schakelt passthrough niet in.
Ze hoort op het afzonderlijke `eidos`-binaire bestand van de bundel of van een
build vanuit de broncode, nooit op `eidos-linux`, dat ook het venster is
([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` is de afzonderlijke schakelaar tijdens het uitvoeren. Het staat **standaard uit en dat wil je vrijwel zeker zo laten**:
gemeten op Skyrim SE belet het het spel zijn eigen archieven en plugins te
openen, zodat mods stilzwijgend niet laden. De schakelaar bestaat om het
mechanisme opnieuw te testen, niet omdat hij aanbevolen wordt.

Details, en de metingen achter die beslissing, in
[troubleshooting.md](troubleshooting.md).

## Nu al iets mis?

[troubleshooting.md](troubleshooting.md) behandelt de omgevingsschakelaars,
hoe je de bewerkingstellers leest, en elk probleem dat tot nu toe iemand gebeten
heeft.
