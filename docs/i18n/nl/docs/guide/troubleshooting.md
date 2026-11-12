<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c365b34dbc69a832b1c9305498c94eb223b567a1 -->

# Probleemoplossing en diagnostiek

De omgevingsschakelaars, hoe je de bewerkingstellers leest, de Health-kaarten van
het tabblad Diagnostics, hoe je de mod achter een crash vindt, wat je bij een
bugrapport voegt, en de bekende problemen met hun geschiedenis.

## De VFS diagnosticeren

Twee omgevingsvariabelen dekken de meeste gevallen waarin het spel iets ziet waarmee
het bestandssysteem het niet eens is:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

De fijnmazige vorm is wat de `FOPEN_KEEP_CACHE`-crash onder Bekende problemen
gevonden heeft: alle vier uitzetten beantwoordt "ligt het aan de caching?", en
alleen ze bij naam noemen beantwoordt "aan welke". De tellers beantwoorden de
andere helft. Met passthrough aan (standaard staat het uit) is een laadbeurt die
`read 0` toont er een waarbij `FUSE_PASSTHROUGH` elke byte in de kernel geleverd
heeft, dus alles wat je op het leespad wilde bijstellen is al gratis.

Nog enkele schakelaars grenzen een fout verder af:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

Zet ze bij een Steam-start vóór de opdracht in de opstartoptie, bijvoorbeeld
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. De uitvoer gaat naar het log
van de run: de `logs/run-*.log` van de instantie bij een start vanuit het venster, de
console van Steam bij een start via Steam.

Om de samengevoegde weergave zelf te bekijken opent `eidos shell <instance>` een
shell in de spelmap zoals het spel die ziet, en `eidos shell <instance> -- ls Data/meshes`
draait daar één opdracht; alles wat je schrijft belandt in de Overwrite
([usage.md](usage.md#een-shell-in-de-samengevoegde-weergave)). De knop Save tree op
het tabblad Data schrijft de hele samengevoegde `Data`-boom naar een tekstbestand,
één bestand per regel met de mod waar het vandaan komt, met je thuismap verborgen.

## Een union met de hand koppelen

`eidos-fuse` zit in het Arch-pakket en in builds vanuit de broncode: `just build`
bouwt het, en `just install` of `install.sh` kopieert het wanneer het er is. De
releasebundel en `eidos-linux` bevatten het niet. De gebruiksregel:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

De eerste `--layer` wint bij conflict; de laatste is je ongerepte speldata. De
koppeling heeft alleen `/dev/fuse` en `fusermount3` nodig (geen overlayfs, geen
Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Schrijfacties belanden in `--overwrite <dir>` (een tijdelijke map als je die
weglaat), zodat de lagen zelf ook hier ongerept blijven.

## Waarom passthrough standaard uit staat

Passthrough geeft de kernel het echte onderliggende bestand, zodat lezingen deze
daemon volledig overslaan. De A/B hieronder is gemeten met een eerdere versie
van de daemon, vóór de wijziging die verderop staat beschreven bij het bekende
probleem "FUSE-passthrough belet het spel enige modinhoud te laden". Die
wijziging vermijdt de waarschijnlijkste oorzaak, maar is nog niet opnieuw in een
spel gemeten, dus passthrough blijft standaard uit tot dat gebeurd is. A/B
gemeten op Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, dezelfde
laadvolgorde van 82 plugins, met als enige variabele of het binaire bestand de
capability droeg:

| passthrough | `NtCreateFile`-fouten met `STATUS_ACCESS_VIOLATION`    |
|-------------|--------------------------------------------------------|
| aan         | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| uit         | 0                                                      |

Met passthrough aan opende het spel geen enkel eigen archief of plugin, wat in
het spel zichtbaar werd als mods die er niet waren - geen fout, geen logregel.
Met passthrough uit bereikte dezelfde laadvolgorde het spelen, met haar plugins,
archieven en Papyrus-scripts actief.

De storing was van binnen de daemon onzichtbaar, en dat is wat het duur maakte
om haar te vinden: zijn eigen `open` slaagde elke keer en de kernel weigerde
nooit een onderliggend bestand (geverifieerd over een volledige mislukte sessie
met `EIDOS_FUSE_TRACE=open`: nul `open FAILED`, nul `passthrough refused`). De
fout ontstond nadat de daemon `opened_passthrough` geantwoord had, dus geen
enkele logging aan de daemonkant kon haar zien. Ze was ook niet
extensiespecifiek - ze trof archieven en plugins evengoed, dat wil zeggen de
bestanden die het spel zijn hele draaitijd open houdt.

`EIDOS_FUSE_PASSTHROUGH=1` zet het weer aan, om te meten wat het oplevert of om
het mechanisme opnieuw te testen. De capability-waarschuwingen in de launcher en
het tabblad Diagnostics verschijnen alleen wanneer je erom gevraagd hebt.

## Wordt de laagindex ook echt gebruikt?

`LayerStack::new` bouwt een volledige kaart van de alleen-lezen lagen of
helemaal geen. Zonder kaart doorloopt elke bevraging de lagen zoals vroeger: de
antwoorden zijn dezelfde, een save laden duurt ongeveer drie keer zo lang, en
verder ziet niets er anders uit.

Staat de index uit, dan zegt de mount waarom, één keer, bij het starten:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

De reden noemt een onleesbare map, een laag van meer dan 64 mappen diep, meer dan
4.000.000 items, of `EIDOS_NO_INDEX`. De regel komt in het log van de run: de
`logs/run-*.log` van de instantie bij een start vanuit het venster, de console van
Steam bij een start via Steam. Met `EIDOS_FUSE_STATS=1` heeft de dump bij het
ontkoppelen ook een regel `layer index:`, en een `unindexed`-teller die niet nul
is op een normale mount betekent dat de index uit stond.

De voorbeelden hieronder controleren hetzelfde offline, op de mappen van een
instantie, zonder mount:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` lost echte paden op met en zonder de index en vergelijkt de
mapscans. `index_agrees` controleert of de twee HETZELFDE antwoorden, op elk pad
en elke lijst van een echte instantie. `listing_cost` meet wat de samengevoegde
kinderenkaart bespaart op `readdir`.

`EIDOS_NO_INDEX=1` dwingt de doorloop af, voor wanneer het verschil tussen de
twee antwoorden juist datgene is wat je aan het debuggen bent.

## Het spel starten met de CLI

Om het spel via de `eidos`-CLI te starten in plaats van via het venster, zet je zijn
Steam-opstartoptie op het absolute pad van `eidos`, bijvoorbeeld:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Een `WINEDLLOVERRIDES`-voorvoegsel is niet nodig. Wanneer een ingeschakelde mod
`d3dcompiler_47` importeert, plaatst Eidos zijn meegeleverde kopie in het prefix,
dwingt die af als native, en voegt dat samen met alle DLL-overrides die een mod
meelevert (ENB/ReShade/`.asi`-loaders).

Eidos start het spel nooit zelf via Proton. `eidos tool <instance> run <title>` met een
item dat het spel start (de loader van de script extender, de launcher of het
spelprogramma) vraagt de winkel die de kopie bezit om het te starten, zoals Run doet in
een venster dat geen winkel opende, en drukt af wat je moet instellen wanneer de
instelling van de winkel Eidos niet start. `--print` drukt de opdracht van de winkel af
in plaats van haar uit te voeren.

## Een spel of tool dat niet afsluit

Terwijl een spel of tool draait, wordt de Run-knop Stop, en de vergrendelkaart heeft
ook een Stop-knop. Druk er twee keer op. De tweede druk beëindigt elk programma dat de
run startte: eerst SIGTERM, daarna na 10 seconden SIGKILL voor wat er nog over is. Wat
het spel niet had opgeslagen, gaat verloren. Eidos doet daarna wat het na elke
afsluiting doet: INI- en pluginwijzigingen gaan terug naar het profiel, de uitvoer van
tools wordt vastgelegd en de mods worden ontkoppeld. De run wordt vastgelegd met
exitcode 143, wat het programma ook teruggaf, dus `eidos sessions <instance> diff`
neemt hem nooit als de laatste goede run. Unlock is iets anders: het geeft het venster
terug en laat het programma draaien. Vanuit een terminal doet `kill <pid>` op het
`eidos`-proces (de pid staat op de vergrendelkaart) hetzelfde als Stop.

## Uitzoeken wat kapot ging

Om te vinden welke wijziging het spel kapot maakte:

- `eidos sessions <instance> diff` toont wat er veranderd is sinds de laatste
  spelrun die met code 0 eindigde: mods, versies, pluginvolgorde, INI's en het
  uitvoerbare bestand van het spel
  ([usage.md](usage.md#wat-er-veranderd-is-sinds-de-laatste-goede-run)).
- Test run, naast Run, speelt met de mods van het profiel, zonder mods, of met de
  geselecteerde mods die aanstaan, plus de masters die ze nodig hebben, en stuurt alles wat het spel
  schrijft naar `<instance>/test-run/`. Na de run bewaar je elke groep bestanden of
  gooi je haar weg: saves, INI-wijzigingen, pluginstatus, Overwrite-uitvoer en logs
  ([usage.md](usage.md#testruns)).
- Find the mod behind a problem, op het tabblad Diagnostics, draait testruns met
  telkens de helft van de resterende verdachten en vraagt of het probleem optrad. 200
  verdachten kosten ongeveer 8 runs, en de zoektocht overleeft het sluiten van Eidos
  ([usage.md](usage.md#de-mod-achter-een-probleem-vinden)).

Een gestopte run vult in die zoektocht nooit vooraf Yes in, omdat hij met exitcode 143
wordt vastgelegd (zie hierboven).

## Health-kaarten

Het tabblad Diagnostics toont een Health-kaart voor elk probleem dat het vindt, met de
oorzaak en, bij sommige, een knop die het oplost. De kaarten voor de opstartoptie
staan beschreven in [install.md](install.md), de NVAPI-, volledigscherm- en
ENB-kaarten in [graphics.md](graphics.md), en de Proton- en prefixkaarten in het
volgende gedeelte. `eidos check <instance>` drukt de bevindingen af die geen venster
nodig hebben ([usage.md](usage.md#gezondheidscontroles-vanuit-de-terminal) somt ze
op), onder meer de kaarten voor prefix, register en de `S:`-schijf. Alleen het venster
toont de Proton-kaart met zijn varianten voor Flatpak, onbekende build en oudere
build, de patchvolgorde, extenderscripts, archieven die geen plugin laadt,
Nexus-vereisten, verouderde pluginkopieën en de Display-kaart.

### Het systeem

- "vm.max_map_count is <value>" verschijnt onder 1048576, de standaard op Arch Linux,
  Fedora 39 en later, en Ubuntu 24.04 en later. Sommige spellen onder Proton
  koppelen meer geheugengebieden en crashen dan zonder fout. Zet
  `vm.max_map_count = 1048576` in een bestand onder `/etc/sysctl.d`, bijvoorbeeld
  `80-gamecompatibility.conf`, en draai dan `sudo sysctl --system`.
- "The open files limit is <value>" verschijnt wanneer de harde limiet onder 524288
  ligt, wat esync van Proton nodig heeft; spellen kunnen er onder niet starten of
  crashen. systemd 240 en later stellen het in. Zet op een oudere opstelling
  `DefaultLimitNOFILE=524288` in `/etc/systemd/system.conf` en
  `/etc/systemd/user.conf`, en log daarna opnieuw in.
- "Game files on a Windows file system" noemt de spelmap, het prefix of de wortel van
  de instantie wanneer die op `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse),
  `exfat` of `vfat` staat. Proton heeft symlinks, Unix-rechten en hoofdlettergevoelige
  namen nodig. Verplaats het spel, zijn prefix en de instantie naar een Linux-bestandssysteem
  zoals ext4, btrfs of xfs.

### De spelbestanden

De kaart Game files bovenaan het tabblad toont of de instantie de winkelinstallatie
draait of een kopie van het spel in de instantie
([usage.md](usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie)). Starts worden
geweigerd, met een kaart die zegt waarom, in drie gevallen:

- "The game copy '<folder>' is missing": de instantie draait een kopie die weg is.
  Zet haar terug, of ga terug naar de winkelversie.
- "'<folder>' has no <binary>": de map die de instantie draait bevat geen
  spelprogramma, dus het is geen kopie van het spel.
- "<game> cannot run from a copy yet": Morrowind, en Oblivion met
  `bUseMyGamesDirectory=0`, bewaren hun pluginlijst in de spelmap.

"ModOrganizer.ini points tools at another game folder" verschijnt wanneer de eigen
`ModOrganizer.ini` van een lijst, niet door Eidos geschreven, een `gamePath` heeft die
afwijkt van de spelbestanden die de instantie draait. Tools in de Mod Organizer
2-modus, zoals PGPatcher, lezen het spel van daar. Wijzig `gamePath` in dat bestand,
of verwijder het bestand zodat Eidos zijn eigen schrijft.

### De laadvolgorde

- "sTestFile lines make the game ignore plugins.txt" verschijnt voor Fallout 4,
  Fallout 4 VR en Starfield wanneer de `Fallout4Custom.ini` of `StarfieldCustom.ini`
  van het profiel regels `sTestFile1` tot `sTestFile10` in `[General]` heeft. Zolang
  er een is ingesteld, laadt het spel die plugins en heeft de laadvolgorde die Eidos
  schrijft geen effect. Fix verwijdert de regels uit de kopie van het profiel en
  bewaart het vorige bestand als `<ini>.pre-fix`.
- Starfield: ingeschakelde `.esp`-plugins zonder de masterflag krijgen een kaart,
  omdat Starfield ze niet kan sorteren tussen `.esm`- en master-gemarkeerde plugins en
  een deel van hun records de hele sessie geladen houdt. Een `.esm`-release van de
  auteur voorkomt beide.
- Starfield: "Invalid blueprint plugins" noemt plugins die een deel maar niet alles
  hebben van de blueprintvlag, een naam die met `BlueprintShips-` begint en de
  extensie `.esm`. "Blueprint plugins without their main plugin" noemt de plugins
  waarvan de hoofdplugin niet is ingeschakeld: `BlueprintShips-Example.esm` heeft
  `Example.esm`, `.esp` of `.esl` nodig.
- "<patch> loses <n> file(s) to <master>" verschijnt wanneer een plugin in de ene mod
  een master uit een andere mod nodig heeft en die andere mod de hogere prioriteit
  heeft, zodat zijn kopieën van de bestanden die beide leveren winnen. Een patch hoort
  meestal onder de mod die hij patcht, en de knop Move van de kaart zet hem daar. Houd
  de volgorde aan als de patch die bestanden hoort te verliezen.
- "Out-of-date copy of <plugin>" verschijnt wanneer een mod een opgeschoonde of
  light-gemarkeerde kopie bevat van een plugin waarvan het origineel sindsdien is
  gewijzigd (een mod- of spelupdate). Het spel laadt de oude kopie. Verberg haar en
  schoon of markeer de nieuwe plugin dan opnieuw.

### De modlijst

- "Order rules of the collection <name> that no longer hold" verschijnt wanneer de mods
  van een geïnstalleerde collectie niet meer in de volgorde staan die haar regels
  vragen. Restore collection order zet alleen de mods van die collectie terug, en
  Ctrl+Z maakt het ongedaan.
- "<mod> requires <mod>, which is not installed" (of "which is disabled") komt uit de
  Nexus-vereisten die Check for updates leest. Open on Nexus gaat naar de pagina van de
  vereiste mod, Download main file haalt hem op met een premium-account, en Ignore
  verbergt een vereiste waaraan een andere mod al voldoet. Een DLC-vereiste is alleen
  advies: Eidos zoekt een plugin met de naam van de DLC.
- "fontconfig.txt names a font library that is missing" (Skyrim-engines) noemt de
  lettertypen die geen enkele laag en geen enkel archief bevat, en de mod waarvan
  `Interface/fontconfig.txt` wint. Menutekst kan als blokjes verschijnen tot het
  lettertype is geïnstalleerd of de regel is verwijderd.
- "BA2 archive stores audio compressed" (Fallout 4 en Fallout 4 VR) telt de
  `.wav`- en `.xwm`-bestanden die het BA2 van een mod gecomprimeerd bewaart. Het spel
  kan crashen wanneer het ze afspeelt. Verpak het archief met ongecomprimeerde audio.

### De script extender en spelupdates

- "<loader> is not installed" verschijnt wanneer er plugins voor de script extender
  zijn geïnstalleerd (in `SKSE/Plugins`, `F4SE/Plugins` enzovoort) maar de loader in
  geen enkele ingeschakelde mod en niet in de spelmap zit. Het spel start zonder die
  plugins.
- "The script extender does not match game version <version>" verschijnt wanneer de
  loader is geïnstalleerd maar de runtime-DLL voor de draaiende spelversie niet, bijvoorbeeld
  `skse64_1_6_1170.dll`. De loader stopt met een fout. Dit volgt meestal op een
  spelupdate. Deze kaart en die hierboven hebben een knop Open the download page voor de
  site van de extender en een knop Install the downloaded archive, die de loader in de
  map `Root/` van de mod zet.
- "<game> changed since the last clean launch" verschijnt wanneer het uitvoerbare
  bestand van het spel, of bij een Steam-kopie zijn build-id, is gewijzigd sinds de
  laatste start via Eidos die zonder fout eindigde. Een Steam-update die het
  uitvoerbare bestand zoals het was laat, activeert hem niet. Plugins voor de script
  extender die voor de oude versie zijn gebouwd kunnen niet meer laden, en de kaarten
  eronder noemen degene die het declareren. Om de spelversie te behouden waarop een
  lijst is gebouwd, bevries je haar in de instantie met Freeze this version op de kaart
  Game files, of met `eidos stock-game <instance> freeze --yes` nadat `freeze` alleen
  de grootte heeft getoond: latere winkelupdates wijzigen de winkelinstallatie, niet
  de kopie.
- "<mod> replaces <n> <extender> script(s)" verschijnt wanneer een mod met hogere
  prioriteit dan de script extender losse `.pex`-kopieën heeft van de eigen scripts van
  de extender. Een oude kopie breekt de functies die ze declareert, en het spel toont
  geen fout. Zet de extender onder die mod (de knop Move van de kaart doet het wanneer
  beide mods zijn), of verberg die bestanden in de mod. Staan de scripts van de extender
  los in de `Data` van het spel, installeer de extender dan als mod zodat hij gesorteerd
  kan worden. Archieven worden niet gecontroleerd, alleen losse bestanden.

### Uitvoer van tools

- "Overwrite/Root holds meshes" (of textures) betekent dat een tool waarvan het
  datapad naar de spelmap wees die mappen daar schreef, waar het spel ze nooit leest.
  Corrigeer het pad van de tool, verwijder de mappen en draai de tool opnieuw. De knop
  Open folder opent `Overwrite/Root`.
- "BodySlide's GameDataPath is not the game's Data folder" (of `OutputDataPath`): zie
  [Tools](tools.md#de-eigen-instellingen-van-een-tool-blijven-de-zijne).

### Een crash in de laatste sessie

"The game crashed in the last session" verschijnt wanneer een crashlogger tijdens de
laatste start een log schreef. De kaart noemt de uitzondering en de DLL's en plugins in
de log die je mods of de Overwrite installeren. Zie die als aanwijzingen: een mod die
in een crashlog wordt genoemd is vaak waar de crash zichtbaar werd, niet de oorzaak.
Noemt de log geen bestand uit je mods, dan zegt de kaart dat. Open the log opent de log,
en Copy the log (home path hidden) kopieert hem met je thuismap verborgen.

### De venstergrootte

Voor Skyrim SE, Enderal SE en Fallout 4 toont een Display-kaart de venstergrootte en
-modus waarmee het profiel het spel start, en zegt "No window size set for this
profile" wanneer noch de Prefs-INI van het profiel noch die van het prefix er een
instelt. Apply schrijft de grootte en modus in de kopie van het profiel, en in een kopie
van de INI van SSE Display Tweaks in de Overwrite wanneer die mod het venster instelt
([usage.md](usage.md#diagnostics)).

## Welke Proton het spel draait, en een kapot prefix

Het tabblad Diagnostics heeft een Proton-kaart: de build die Eidos en Steam voor het
spel gebruiken, de eigen instelling van Steam en de build die het prefix het laatst
draaide. Als een nieuwere Proton dan de gekozen build het prefix het laatst heeft
ingericht, wordt de kaart een advies, want een oudere Proton op een nieuwer prefix kan
de Wine-bestanden ervan terugzetten of niet starten. Kies in Steam een build die
minstens even nieuw is, onder Eigenschappen, Compatibiliteit van het spel, of bouw het
prefix opnieuw op.

De andere Proton- en prefixkaarten:

- "Proton comes from the Flatpak Steam install": die Proton bewaart zijn runtime en
  steamclient-bibliotheken binnen de Flatpak-sandbox, dus hem vanaf de host draaien kan
  mislukken. Eidos start hem niet via `flatpak run`, want dat zou het spel in de
  sandbox zetten, buiten de samengevoegde weergave. Installeer een Proton in
  `~/.steam/root/compatibilitytools.d/` en kies die voor het spel.
- "Eidos cannot tell which Proton runs this game": de `config.vdf` van Steam noemt er
  geen compatibiliteitstool voor, en het prefix legt geen geïnstalleerde build vast.
  Tools starten niet tot er in Steam een is gekozen (Eigenschappen van het spel,
  Compatibiliteit).
- "<game> has no Proton prefix yet": start het spel één keer via Steam en druk dan op
  F5. Maakt Steam het prefix niet aan, controleer dan of de map van het spel onder
  `steamapps/compatdata` beschrijfbaar is; een lege map van root blokkeert het zonder
  bericht.
- "Windows tools cannot find <game>": het vastgelegde installatiepad in het prefix is
  fout of ontbreekt, en xEdit, DynDOLOD, Wrye Bash en de Creation Kit lezen het.
  Eidos schrijft het juiste pad de volgende keer dat je een tool draait.
- "The prefix's S: drive points outside the Steam library": `S:` komt noch overeen met
  de bibliotheekwortel, waar Eidos hem voor een tool op laat wijzen, noch met
  `<library>/steamapps`, waar Steam hem voor het spel op laat wijzen. De bibliotheek is
  waarschijnlijk verhuisd, of het prefix komt van een andere machine. Elk aan `S:`
  gerelateerd pad dat in het prefix staat, wijst nergens heen tot de volgende start via
  Eidos of Steam hem opnieuw instelt.
- "<store> installation: started through its store" is de kaart van een GOG- of
  Epic-kopie. Run vraagt Heroic of Legendary het spel te starten wanneer Eidos daar de
  wrapper van het spel is, en tools draaien met de Wine- of Proton-build die Heroic voor
  de kopie vastlegt.

Het menu File draait winecfg, regedit, de Wine-bestandsverkenner en de verwijderaar in
het prefix van het spel, met de Proton die de tools van het spel draait
([tools.md](tools.md#de-eigen-programmas-van-wine-in-het-prefix-van-het-spel)). Hun
rijen blijven grijs tot het prefix die programma's heeft.

De knop Rebuild prefix op de Proton-kaart is voor een prefix dat niet meer werkt. Druk
er twee keer op. Eidos verwijdert niets: het hernoemt `steamapps/compatdata/<appid>`
naar `<appid>.eidos-backup-<date>-<time>` in dezelfde map, en Steam maakt een nieuw
prefix aan de volgende keer dat je het spel vanuit Steam start. Het weigert zolang een
spel, een tool of een ander Wine-programma het prefix gebruikt. Tot het nieuwe prefix
bestaat, kan Run in het venster het spel niet aan Steam overdragen en toont het de
opstartoptie, dus druk op Play in Steam. Saves en instellingen die alleen in het oude
prefix staan, blijven in de hernoemde map: kopieer terug wat je nodig hebt, en
verwijder die map zelf als je hem niet meer wilt.

## Wat je bij een bugrapport voegt

- Copy report, op het tabblad Diagnostics, kopieert de Eidos-versie, het spel, de
  Proton-build, of Steam de Flatpak is, de passthrough-status, het sessietype, de
  distributie, de kernel, `vm.max_map_count`, de limiet voor open bestanden, of
  `/dev/fuse` en `fusermount3` er zijn, de namen van de opstartvariabelen en
  wrapperprogramma's van het profiel, en elke Health-kaart. Je thuismap staat als `~` en
  je inlognaam als `<user>`. `eidos check <instance>` drukt dezelfde kop af in een
  terminal, en `--json` geeft hem als JSON.
- Het log van de run, `<instance>/logs/run-*.log`, begint met de exacte opdracht die
  Eidos draaide. File, Eidos logs opent de map met de eigen logs van Eidos,
  `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), op de crashkaart, kopieert de crashlog.
- Save tree, op het tabblad Data, bewaart de samengevoegde `Data`-boom met de bron van
  elk bestand.

## Bekende problemen

### DLSS of frame generation doet stilzwijgend niets

Drie afzonderlijke oorzaken, elk zonder enige foutmelding: NVAPI niet ingeschakeld in
de opstartopties, exclusive fullscreen, of een verouderde Reflex-FPS-limiet. De
volledige checklist staat in [graphics.md](graphics.md).

### Run doet niets met de Flatpak-Steam

In een venster dat geen winkel opende, vraagt Run Steam het spel te starten door
`steam -applaunch <appid>` uit te voeren. De Flatpak-Steam zet geen `steam` in het
`PATH` van de host, dus de statusregel zegt "Could not start Steam" met de fout. Druk
in plaats daarvan op Play in Steam.

### Een save van een ander apparaat verschijnt niet

Een save die het prefix bereikt zonder via Eidos te gaan, zoals een Steam
Cloud-download van een Steam Deck, blijft verborgen zolang de saves van het profiel
gemount zijn. Niets verwijdert hem. Het tabblad Saves telt deze saves onder zijn kop,
en Import kopieert ze in het profiel zonder een save van het profiel te vervangen
([usage.md](usage.md#saves-horen-bij-het-profiel)).

### Een mod die één map op twee manieren spelt, verloor alles onder de tweede

Opgelost. ext4 houdt `meshes/` en `Meshes/` uit elkaar; de samengevoegde
weergave mag dat niet, en echte mods leveren beide - XP32 Maximum Skeleton heeft
zijn animaties en zijn FNIS-behaviourbestand onder de versie met hoofdletter,
zijn `character assets` onder de andere.

De resolver nam voor elk padonderdeel de treffer met de exacte schrijfwijze en
legde zich daarop vast: hij ging `meshes/` binnen, vond de rest van het pad daar
niet, en liet DE HELE LAAG vallen. Elk bestand onder de andere spelling was
onzichtbaar voor het spel - geen fout, geen log, niets in enige diagnostiek. Op
een echte instantie met 50 lagen waren dat 74 bestanden.

Een onderdeel dat overeenkomt is nu een kandidaat, geen beslissing; de exacte
schrijfwijze wordt nog altijd eerst geprobeerd, en pas wanneer de rest daaronder
faalt, zoekt de scan naar buren die na case-folding gelijk zijn. Lijsten hadden
dezelfde fout een map hoger en lezen nu elke na case-folding gelijke map per
laag.

De padindex heeft deze bug nooit gehad, omdat hij elke map doorloopt die hij
tegenkomt. Een tijdlang gaf de index dus bestanden terug die de fallback niet kon
geven, terwijl de fallback het antwoord is dat nooit fout hoort te zijn.

### DynDOLOD's LODGen sterft en laat een leeg log achter

Opgelost door `dotnet10`; zie [tools.md](tools.md). Het `LODGen_SSE_<world>_log.txt`
van elke wereld bevat een versiebanner, een regel `.NET Version:` en verder niets, en
een dialoogvenster zegt alleen "failed to generate object LOD for one or more worlds".
De oorzaak is dat Wine's Mono antwoordt voor .NET Framework, en hoeveel .NET Framework
je ook installeert, het lost het niet op - Proton vervangt `mscoree.dll` bij elke
prefix-update door een symlink naar zijn eigen boom. De releasebundel levert de .NET
10-runtime mee; andere installaties downloaden hem één keer.

### Wine kon niet zien dat de koppeling hoofdletters vouwt

Opgelost.

Wine's `get_dir_case_sensitivity` vraagt de map eerst via `FS_IOC_GETFLAGS` en
snuffelt daarna naar de markering die CIOPFS achterlaat in de mappen die het
bedient. Ontbreekt beide, dan gaat Wine uit van HOOFDLETTERGEVOELIG, en elke
opzoeking waarvan de spelling niet byte voor byte klopt, valt terug op het lezen
van de HELE map om een hoofdletterongevoelige treffer te vinden.
Bethesda-spellen vragen om `data/ccbgssse001-fish.bsa` terwijl het bestand
`ccBGSSSE001-Fish.bsa` heet, dus het ging bij vrijwel elke asset af: 4471
markeringspeilingen en 2236 volledige herlezingen van mappen in acht seconden,
en 195796 opsommingen van `Data` in negentig. Skyrim SE bereikte zijn hoofdmenu
nooit - het bleef op 240 MB resident staan terwijl de daemon 92% van een core
verstookte.

Eidos vouwde hoofdletters vanaf het begin in `resolve_read`. De hele kost zat
erin dat het dat nooit zei. `lookup` antwoordt nu `.ciopfs`; `readdir` toont het
nog altijd niet in lijsten. `ioctl` beantwoordt ook `FS_IOC_GETFLAGS` op mappen
met de casefold-vlag, die huidige Wine vraagt voordat het naar de markering
zoekt.

Twee dingen maakten het fataal in plaats van alleen maar traag. De kost schaalt
mee met de mapgrootte, dus het installeren van de Anniversary-inhoud (`Data` van
37 bestanden naar 177) deed het omslaan. En `opendir` bouwde gretig de
samengevoegde lijst op, wat pure verspilling is wanneer Wine een map alleen
opent om die markering erin te `stat`ten - de momentopname wordt nu bij de eerste
`readdir` genomen.

Daarna: het hoofdmenu, 2,1 GB resident, daemon op 0% CPU.

`EIDOS_FUSE_TRACE=opendir` is wat het gevonden heeft, en wordt meegeleverd. De
bewerkingstellers zeggen hoeveel; 195796 opsommingen van één map zijn onzichtbaar
in een totaal.

### Het spel herschreef `plugins.txt` leeg

Zeer waarschijnlijk hetzelfde als de hoofdlettervouwing hierboven - een `Data` die het
binnen geen redelijke tijd kon opsommen, dus concludeerde het dat er niets was en
bewaarde dat. Niet bewezen, en het opnieuw nakijken waard. Hoe dan ook betekent de
capture-bewaking (een capture die de actieve set volledig leegmaakt, wordt bij elke
omvang geweigerd) dat het het profiel niet meer kan beschadigen.

### `FOPEN_KEEP_CACHE` staat uit

Opgelost. Het liet Skyrim SE crashen op een null-dereferentie, seconden na het
hoofdmenu, deterministisch, met nul mods geïnstalleerd; de andere drie caches
aan kernelkant zijn stuk voor stuk weggebisecteerd en alleen deze deed ertoe.
Die bisectie draaide met `FUSE_PASSTHROUGH` aan, en de kernel laat een
passthrough-opening die `FOPEN_KEEP_CACHE` draagt mislukken met EIO, dus met de
vlag aan mislukte elke passthrough-opening. Misschien waren het die mislukte
openingen die het spel lieten crashen, en de vlag op zichzelf, met passthrough
uit, is nooit getest. Passthrough-antwoorden dragen hem niet meer. Het verlies
ervan werd destijds als gratis gemeten, maar met passthrough aan bedient de
daemon *nul* lezingen (`EIDOS_FUSE_STATS` meldde `read 0` voor een volledige
laadbeurt), dus met passthrough standaard uit (hieronder) is de echte kost
ongemeten. Zet hem met `EIDOS_FUSE_KEEP_CACHE=1` weer aan om hem op zichzelf te
testen.

### FUSE-passthrough belet het spel enige modinhoud te laden

De daemon vermijdt nu de waarschijnlijkste oorzaak (hieronder), maar dat is nog niet
opnieuw in een spel gemeten, dus passthrough blijft standaard uit;
`EIDOS_FUSE_PASSTHROUGH=1` zet het aan. Vóór die wijziging slaagde Skyrim SE er met
passthrough aan niet in 152 van zijn eigen bestanden te openen, met
`STATUS_ACCESS_VIOLATION`, dus laadde er stilzwijgend geen modinhoud. De meting en de
reden waarom de eigen logs van de daemon een schone run toonden staan onder "Waarom
passthrough standaard uit staat" hierboven.

De kernel koppelt één onderliggend bestand aan elke inode (`fs/fuse/iomode.c`).
Een opening die voor een inode die er al een heeft een tweede id van een
onderliggend bestand meebrengt, mislukt met EIO, net als een normale opening
(zonder `FOPEN_PASSTHROUGH`) van een inode die een onderliggend bestand heeft.
Een passthrough-opening die `FOPEN_KEEP_CACHE` draagt, mislukt ook met EIO. De
daemon registreerde vroeger bij elke opening een nieuwe id, dus mislukte de
tweede opening van een bestand dat het spel al open hield. Wine meldt EIO als
`STATUS_ACCESS_VIOLATION`, de fout achter de 152 mislukte openingen. Deze regel
komt uit de broncode van de kernel en past bij die 152 mislukkingen, wat haar de
waarschijnlijkste oorzaak maakt.

De daemon registreert nu één onderliggend bestand per inode, gedeeld door al
zijn passthrough-openingen, en zet nooit `FOPEN_KEEP_CACHE` op een
passthrough-opening. Passthrough geldt alleen voor openingen om te lezen van
bestanden uit de spel- en modlagen die 1 MiB of groter zijn of eindigen op
`.dll`, `.exe` of `.asi`. Zodra dat onderliggende bestand geregistreerd is,
gebruikt elke opening om te lezen van het bestand het, en zolang het bestand
normale openingen heeft, krijgen nieuwe lezers ook normale openingen. Een
bestand in de Overwrite gaat nooit via passthrough, en een koppeling die
tijdstempels van plugins projecteert, gebruikt het helemaal niet. Een schrijver
die een bestand opent terwijl passthrough-lezers het vasthouden, krijgt ETXTBSY
(in Wine: `STATUS_SHARING_VIOLATION`) vóór enige copy-up, zodat een opening met
`O_TRUNC` geen lege kopie in de Overwrite kan achterlaten die het bestand van de
mod verbergt.

### De knop Mod Manager Download doet niets

Preferences, Nexus, Downloads toont welk programma `nxm://`-links ontvangt. Er verschijnt
een Health-kaart wanneer dat programma niet Eidos is, ontbreekt, of wanneer er geen is
ingesteld. De knop Register schrijft `eidos-nxm.desktop` en maakt het de standaardhandler
voor `x-scheme-handler/nxm`. In een terminal doet `eidos nxm --register` hetzelfde.

Staat in de status al Eidos en doet de knop nog steeds niets, dan gebruikt de browser mogelijk een eerdere keuze. Wis in de browserinstellingen de opgeslagen keuze voor nxm-links, zodat de browser opnieuw vraagt. Een Flatpak-browser kan geen programma's van de host starten en geeft de link door via de desktopportal, die de op de host geregistreerde handler leest.
