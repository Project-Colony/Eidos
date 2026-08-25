<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Eidos installeren

Drie manieren om binnen te komen. Alle drie leveren dezelfde twee programma's -
`eidos` (de opdrachtregel) en `eidos-gui` - plus de `nxm://`-handler waardoor de
knop "Mod Manager Download" op Nexus in jouw instantie landt.

## Wat je eerst nodig hebt

| | |
|---|---|
| **Linux met FUSE** | `fusermount3` in je PATH. Elke huidige distributie levert het mee. |
| **Een Proton-spel dat één keer gestart is** | Steam maakt het Wine-prefix van het spel pas bij de eerste start aan, en Eidos werkt daarbinnen. |
| **`7z`** | Voor het installeren van modarchieven. `p7zip` op de meeste distributies. |

Geen root, geen daemon, geen aanpassing van `/etc/fuse.conf` en niets om aan je
groepen toe te voegen. Eidos koppelt binnen een privénaamruimte die van het
spelproces is.

## Arch

```bash
cd packaging && makepkg -si
```

## Een release-archief

```bash
./install.sh
```

Installeert standaard in `~/.local/bin`. `--system` zet het in `/usr/local/bin`,
`--bindir DIR` ergens anders. Opnieuw uitvoeren is de bedoelde manier om bij te
werken.

## Vanaf de broncode

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Daarna: Steam ernaar laten wijzen

Eidos draait *als* de startopdracht van je spel, en zo komt het ertoe te
koppelen voordat het spel begint. Rechtsklik in Steam op het spel ->
Eigenschappen -> Opstartopties:

```
~/.local/bin/eidos-gui %command%
```

Druk op Spelen. Eidos opent op de instantie van dat spel; installeer mods,
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
instantie - zie [usage.nl.md](usage.md). De volledige rondleiding staat daar.

## Optioneel: FUSE-passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` schakelt kernel-FUSE-
passthrough in. Het staat **standaard uit en dat wil je vrijwel zeker zo laten**:
gemeten op Skyrim SE belet het het spel zijn eigen archieven en plugins te
openen, zodat mods stilzwijgend niet laden. De schakelaar bestaat om het
mechanisme opnieuw te testen, niet omdat hij aanbevolen wordt.

Details, en de metingen achter die beslissing, in
[troubleshooting.nl.md](troubleshooting.md).

## Nu al iets mis?

[troubleshooting.nl.md](troubleshooting.md) behandelt de omgevingsschakelaars,
hoe je de bewerkingstellers leest, en elk probleem dat tot nu toe iemand gebeten
heeft.
