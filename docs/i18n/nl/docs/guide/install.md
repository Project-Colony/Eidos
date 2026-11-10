<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Eidos installeren

Vier manieren om binnen te komen. Colony installeert één bestand, `eidos-linux`, dat het hele programma is. De releasebundel, het Arch-pakket en een build vanuit de broncode leveren `eidos` (de opdrachtregel), `eidos-gui` en het statische NIF-voorvertoningsprogramma `eidos-nif-preview` naast elkaar. Het Arch-pakket en een build vanuit de broncode installeren ook `eidos-fuse` en `eidos-launch`, de twee CLI's op lager niveau. Elke route kan de `nxm://`-handler registreren waardoor de knop "Mod Manager Download" op Nexus in jouw instantie landt.

## Wat je eerst nodig hebt

| Vereiste | Waarom |
|---|---|
| Linux met FUSE | `fusermount3` in je PATH. Elke huidige distributie levert het mee. |
| Een Proton-spel dat één keer gestart is | Steam maakt het Wine-prefix van het spel pas bij de eerste start aan, en Eidos werkt daarbinnen. Eidos herkent ook de GOG- en Epic-kopieën van Skyrim Special Edition, de Epic-kopieën van Fallout 3 en Fallout: New Vegas en de GOG-kopie van Stardew Valley. Die draaien via Heroic of Legendary, met Eidos als wrapper van het spel: zie [usage.md](usage.md#het-spel-starten). |
| Een 7-Zip-programma: `7z`, `7zz` of `7za` | Voor het installeren van modarchieven. `7zip` op Arch, `p7zip` op de meeste andere distributies. |
| `par2` (optioneel) | PAR2-herstelgegevens voor back-ups met `eidos pack`. `par2cmdline` op Arch en Fedora, `par2` op Debian en Ubuntu. |

Geen root, geen daemon, geen aanpassing van `/etc/fuse.conf` en niets om aan je
groepen toe te voegen. Eidos koppelt binnen een privénaamruimte die van het
spelproces is.

Sommige systeeminstellingen vallen alleen op als crashes. Het tabblad Diagnostics
controleert `vm.max_map_count` (1048576 of meer), de harde limiet voor open
bestanden (524288 of meer) en of het spel, zijn prefix of de instantie op NTFS,
exFAT of FAT staat; [troubleshooting.md](troubleshooting.md#het-systeem) heeft
de oplossing voor elk ervan.

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
`eidos-nif-preview` nodig, dat alleen de releasebundel, het Arch-pakket en builds
vanuit de broncode meeleveren; zonder dat programma zegt de voorvertoning dat, en
verder wordt niets geraakt. `eidos-linux` bevat ook geen van de Microsoft-runtimes
die de bundel meelevert ([hieronder](#de-microsoft-runtimes-in-de-bundel)): heeft een
tool er een nodig, dan downloadt Eidos die van Microsoft zodra je de installatie van
de vereisten start. Laat Steam naar het bestand zelf wijzen, zoals hieronder.

## Een releasebundel

Download `eidos-linux-bundle.tar.gz` en de bijbehorende `.sig`, `.meta` en
`.meta.sig` van de [nieuwste release](https://github.com/Project-Colony/Eidos/releases/latest),
en controleer ze zoals het
[beleid voor codeondertekening](../../README.md#beleid-voor-codeondertekening) in de README laat zien. De bundel
bevat `eidos`, `eidos-gui` en `eidos-nif-preview` op het hoogste niveau, met
`install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, de pictogrammen in
`assets/brand/png`, de kennisgevingen van derden in `licenses/` (nifly, omod,
microsoft-directx, microsoft-runtimes en de Rust-crates) en de map `runtimes/`
hieronder. `eidos-fuse` en `eidos-launch` zitten er niet in. Daarna:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` kopieert de binaire bestanden naar `~/.local/bin`, of naar
`$XDG_BIN_HOME` als die is ingesteld. `--system` zet ze via sudo in
`/usr/local/bin`, `--bindir DIR` ergens anders, en `--from DIR` haalt de binaire
bestanden uit een andere map. Het script doet ook het volgende:

- het registreert de `nxm://`-handler met `eidos nxm --register`, dus de
  registratiestap in het Colony-gedeelte is niet nodig;
- het schrijft `~/.local/share/applications/eidos.desktop` en de Eidos-pictogrammen
  onder `~/.local/share/icons/hicolor`;
- het kopieert de licentiekennisgevingen naar `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` met `--system`);
- het kopieert `runtimes/` naar `~/.local/share/Colony/Eidos/runtimes`, ook met
  `--system`, zodat de runtimes bij de gebruiker horen die het script draait;
- het sluit af met het afdrukken van de Steam-opstartoptie met het absolute pad
  van de `eidos-gui` die het installeerde.

`--cap` vraagt sudo om `CAP_SYS_ADMIN` aan het geïnstalleerde `eidos` te geven (zie
FUSE-passthrough hieronder). `--no-cap` is de standaard en laat het rootloos.
Het script opnieuw uitvoeren is de bedoelde manier om bij te werken. De bundel en
`eidos-linux` vereisen x86_64-Linux met glibc 2.39 of nieuwer.

### De Microsoft-runtimes in de bundel

De map `runtimes/` van de bundel bevat de Microsoft-bestanden waar tools om vragen:
`VC_redist.x86.exe` en `VC_redist.x64.exe` (Visual C++ 2015-2022, de vereiste
`vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (de .NET 8 Desktop
Runtime, voor `dotnet8` en `dotnetdesktop8`) en `dotnet-runtime-10.0.10-win-x64.zip`
(de .NET 10-runtime, voor `dotnet10`). Eidos draait de installatieprogramma's van
Visual C++ en .NET 8 zelf in het prefix van het spel, zonder winetricks, en pakt de
.NET 10-zip één keer uit in `~/.local/share/Colony/Eidos/runtimes`, waar elke
instantie hem deelt (78 MB uitgepakt). Elk bestand wordt alleen gebruikt wanneer zijn
SHA-256 overeenkomt met die welke in Eidos vastligt. De .NET 10 SDK die Synthesis
nodig heeft (`dotnetsdk10`, ongeveer 300 MB) zit niet in de bundel en wordt altijd
gedownload.

Een uitgepakte bundel draait ook ter plekke: Eidos zoekt na de datamap hierboven naar
`runtimes/` naast zijn eigen binaire bestand.

## Arch

```bash
cd packaging && makepkg -si
```

De PKGBUILD bouwt de huidige `main` vanuit de broncode. Hij installeert `eidos`,
`eidos-gui`, `eidos-nif-preview`, `eidos-fuse` en `eidos-launch` in `/usr/bin`, met
een systeembrede `nxm://`-handler, een desktopitem en de pictogrammen. Hij hangt af
van `fuse3`, `7zip` en `xdg-utils`. Hij levert geen map `runtimes/` mee, dus de
Microsoft-runtimes worden gedownload wanneer een tool ze nodig heeft. `winetricks` is
optioneel: Eidos gebruikt het nog voor de vereisten die het niet zelf installeert,
zoals `dotnet48` en `xact`.

## Vanaf de broncode

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Bouwen vanuit broncode vereist Rust 1.92 of nieuwer, `just`, CMake 3.20+, een C++17-compiler en Python 3 voor de tests van het native hulpprogramma. `just build` bouwt de hele workspace, dus `eidos-fuse` en `eidos-launch` worden ook gebouwd, en `just install` draait `packaging/install.sh` op de uitvoer van de build, dat ze met de rest kopieert. Zie de [instructies voor het native hulpprogramma](../../../../../crates/eidos-nif-preview/README.md) voor opdrachten zonder `just`, bouw Rust met `cargo build --release --locked`, kopieer het hulpprogramma naar `target/release` en voer daarna `packaging/install.sh --from target/release` uit. Ook een build vanuit de broncode heeft geen map `runtimes/`, dus de Microsoft-runtimes worden gedownload wanneer een tool ze nodig heeft.

Het releaseprofiel gebruikt één codegenerator-eenheid, fat LTO en gestripte symbolen. Met
rustc 1.99.1 en 3 buildtaken duurde een koude build van `eidos` en `eidos-gui` 334 s,
tegen 243 s met thin LTO, en gaf een `eidos` van 20,9 MB en een `eidos-gui` van
49,7 MB. Elk binair bestand bevat acht DirectX-DLL's die gzip-gecomprimeerd zijn
opgeslagen: 7,9 MB in plaats van 17,9 MB. `just profile=dev build` bouwt zonder
optimalisatie.

## Daarna: Steam ernaar laten wijzen

Eidos draait *als* de startopdracht van je spel, en zo komt het ertoe te
koppelen voordat het spel begint. Rechtsklik in Steam op het spel ->
Eigenschappen -> Opstartopties, en voer het absolute pad van het Eidos-venster in,
gevolgd door `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Gebruik het absolute pad - Steam leest het `PATH` van je shell niet. Het pad hangt af
van hoe je Eidos installeerde, en `install.sh` drukt de regel af voor de map die het
gebruikte:

| Installatie | Opstartoptie |
|---|---|
| Releasebundel of `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch-pakket | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Druk op Spelen. Eidos opent op de instantie van dat spel; installeer mods,
sorteer met LOOT, klik op Run. Bij het afsluiten verdwijnt de koppeling ermee en
je installatie is precies zoals ze was.

Het spel start altijd via zijn winkel. In een venster dat je zelf opende draait Run
`steam -applaunch <appid>`, Steam start Eidos via de opstartoptie, en er opent een
tweede Eidos-venster waarin je op Run drukt. Zolang de opstartoptie Eidos niet
start, start Run niets en toont de statusregel de regel die je moet instellen. Met
Start the game at once when a store opens Eidos (Preferences, General, Running a
game; standaard uit) start Play in Steam het spel zonder het venster wanneer Health
geen probleem meldt. Heroic en Legendary werken op dezelfde manier via hun
wrapperinstelling; [usage.md](usage.md#het-spel-starten) geeft de details.

Het tabblad Diagnostics controleert de regel. Een Health-kaart toont de regel die je
moet instellen wanneer de opstartoptie leeg is, geen Eidos-binair bestand noemt,
naar een bestand wijst dat niet bestaat, geen `%command%` na Eidos heeft, of een
andere kopie van Eidos start dan die welke draait.

### Als je de terminal verkiest

```sh
eidos init skyrimse               # een instantie maken (geef een map op om haar draagbaar te maken)
eidos install skyrimse mod.7z     # Simple- / FOMOD- / BAIN- / root-mods
eidos sort skyrimse               # de laadvolgorde met LOOT sorteren
eidos check skyrimse              # de Health-controles die geen venster nodig hebben
eidos play skyrimse -- %command%  # wat dan ook door de samengevoegde weergave draaien
eidos shell skyrimse              # een shell in de samengevoegde weergave, schrijfacties landen in Overwrite
```

Elke opdracht die een spel-id aanneemt, neemt ook de map van een draagbare
instantie - zie [usage.md](usage.md#instanties-globaal-en-draagbaar). De
volledige rondleiding staat in [usage.md](usage.md).

## Bijwerken

Colony werkt `eidos-linux` bij via zijn launcher. Voer voor de bundel de `install.sh`
van de nieuwe bundel uit; draai voor het Arch-pakket opnieuw `makepkg -si`; haal bij
een build vanuit de broncode de wijzigingen binnen en draai `just install`. Het
venster vraagt bij de start, hooguit eens per dag, aan GitHub naar de nieuwste
release, en wanneer er een nieuwere is toont het na Help in de menubalk een link naar
de release-opmerkingen. Het downloadt of vervangt zichzelf nooit. Preferences, About,
Version zet de controle uit, offlinemodus slaat haar over, en een kopie die onder
`/usr` is geïnstalleerd (het Arch-pakket) vraagt nooit, omdat de pakketbeheerder hem
bijwerkt.

## Optioneel: FUSE-passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` verleent de optionele
capability, maar schakelt passthrough niet in. `./install.sh --cap` en
`just setcap` verlenen haar aan het `eidos` dat ze installeren of bouwen. Op een
`nosuid`-koppeling negeert de kernel bestandscapabilities, dus `install.sh` slaat het
verlenen met een waarschuwing over en `just setcap` stopt met een fout. De capability
hoort op het afzonderlijke `eidos`-binaire bestand van de bundel, het Arch-pakket of
een build vanuit de broncode, nooit op `eidos-linux`, dat ook het venster is
([SECURITY.md](../../../../../SECURITY.md)). Een opnieuw gebouwd of opnieuw
geïnstalleerd binair bestand is een nieuw bestand zonder de capability, dus verleen
haar na elke upgrade opnieuw als je haar gebruikt.

`EIDOS_FUSE_PASSTHROUGH=1` is de afzonderlijke schakelaar tijdens het uitvoeren, en
die staat standaard uit. Met passthrough aan belette een eerdere versie van de daemon
Skyrim SE om modinhoud te laden. De waarschijnlijkste oorzaak is dat die bij elke
opening een nieuw onderliggend bestand registreerde, terwijl de kernel er maar één per
inode aanvaardt. De daemon deelt er nu één tussen alle passthrough-openingen van een
bestand, maar dat is nog niet opnieuw in een spel gemeten. Zet passthrough tot dan
alleen aan om het te testen. De metingen en de oorzaak staan in
[troubleshooting.md](troubleshooting.md).

## Nu al iets mis?

[troubleshooting.md](troubleshooting.md) behandelt de omgevingsschakelaars,
hoe je de bewerkingstellers leest, de Health-kaarten, en elk probleem dat tot nu toe
iemand gebeten heeft.
