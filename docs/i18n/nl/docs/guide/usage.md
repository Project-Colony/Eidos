<!-- eidos-i18n: source=docs/guide/usage.md sha=0433d51887d1d1997dd785a7a18f7a4be58dfcd5 -->

# Eidos gebruiken

De praktische handleiding: de CLI, de GUI, de Steam-opstartoptie, bouwen vanaf de
broncode, en het proof-of-concept-script. Wat te doen wanneer er iets mis lijkt,
staat in [troubleshooting.md](troubleshooting.md).

## Gebruiken (CLI)

```sh
eidos games                       # hier geïnstalleerde ondersteunde spellen (zoals de lijst van MO2)
eidos init skyrimse               # een modding-instantie maken
# ...zet elke mod als map in <instance>/mods/ (de globale instantie staat
#    in ~/.local/share/Colony/Eidos/instances/skyrimse; `eidos init` toont die van jou)...
eidos install skyrimse mod.7z     # of een gedownload archief installeren (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # de volgorde + pluginstatus van een bestaand MO2-profiel overnemen
eidos sort skyrimse               # de laadvolgorde van de plugins met LOOT sorteren
eidos play skyrimse               # tonen wat er gekoppeld zou worden
eidos play skyrimse -- <command>  # <command> draaien met de mods over het spel gekoppeld
eidos shell skyrimse              # een shell in de samengevoegde weergave, schrijfacties komen in Overwrite
eidos check skyrimse              # gezondheidscontroles zonder het venster (--json voor scripts)
eidos sessions skyrimse           # de vastgelegde spel- en toolruns, nieuwste eerst
eidos sessions skyrimse diff      # wat er veranderd is sinds de laatste run die met code 0 eindigde
eidos collection skyrimse <link>  # een Nexus-collectie installeren zoals de auteur haar gebouwd heeft
eidos nexus update skyrimse --apply  # de Nexus-bestanden installeren die de jouwe vervingen
eidos pack skyrimse backup.eidos  # de hele instantie in één bestand, om ze elders heen te verhuizen
eidos unpack backup.eidos <folder>   # zet ze terug op de andere machine
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` en `eidos export` maken
het geheel compleet; draai `eidos` zonder argumenten voor de volledige lijst.

### Gezondheidscontroles vanuit de terminal

`eidos check <instance>` draait de controles op prefix, plugins, ontbrekende masters en script extender uit het tabblad **Diagnostics** zonder het venster te openen, zodat ze via SSH of in Game Mode kunnen draaien en in een bugrapport terechtkomen. Het drukt dezelfde systeemregels af als **Copy report**, daarna één regel per bevinding, met je thuismap getoond als `~` en je inlognaam als `<user>`. De exitcode is 0 als er geen probleem is gevonden (alleen adviezen geven nog steeds 0), 1 als er minstens één probleem is gevonden, en 2 als Eidos niet kon controleren: verkeerde argumenten, een instantie die het niet kan openen, of een spel dat het niet detecteert.

`eidos check <instance> --json` drukt in plaats daarvan één JSON-object af. De velden op het hoogste niveau zijn `eidos` (de versie), `game`, `instance`, `system` (`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`, `max_map_count`, `open_files_hard`) en `findings`. `max_map_count` en `open_files_hard` zijn getallen, of `null` als Eidos ze niet kon lezen. Elke bevinding is een `code`, een `level` (`problem` of `advice`) en zijn `args`; het tabblad Diagnostics toont de zin bij elke code.

### Een shell in de samengevoegde weergave

`eidos shell <instance>` start je `$SHELL` (of `/bin/sh`) in de spelmap zoals het spel die zou zien: elke ingeschakelde mod over `Data` gekoppeld, Root-mods over de spelroot, en alles wat daar wordt geschreven komt in de Overwrite terecht. `eidos shell <instance> -- <command>` voert in plaats daarvan één commando uit, bijvoorbeeld `eidos shell skyrimse -- ls Data/meshes` of een native tool zoals NifSkope. De run houdt het instantieslot vast zoals een spelsessie, dus het venster kan geen wijzigingen opslaan tot je afsluit, en hij staat in `eidos sessions` onder de naam `shell`, nooit als spelstart. Ctrl+C bij een commando stopt de run zoals Stop: Eidos beëindigt het commando, legt de sessie vast en ontkoppelt. In de interactieve shell blijft Ctrl+C van de shell, zoals gewoonlijk; typ `exit` om de run te beëindigen.

### Instanties: globaal en draagbaar

Elke opdracht hierboven spreekt een instantie aan. `skyrimse` benoemt de
**globale** - centraal opgeslagen in `~/.local/share/Colony/Eidos/instances/skyrimse`, beheerd
door Eidos. De andere soort is **draagbaar**: een op zichzelf staande map waar je
maar wilt (een tweede schijf, een spellenpartitie), verplaatsbaar en geïsoleerd,
precies zoals de draagbare instanties van MO2. Overal waar een opdracht een
spel-id aanneemt, neemt ze ook de map van een draagbare instantie:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # daar een draagbare instantie maken
eidos install /mnt/games/EidosSkyrim mod.7z  # elke opdracht neemt de map aan
eidos play /mnt/games/EidosSkyrim -- %command%
```

De map beschrijft zichzelf (haar `eidos-instance.ini` benoemt het spel), dus meer
is er niet nodig - en `EIDOS_INSTANCE=<folder>` in de omgeving leidt een spel-id
naar die map om, wat handig is in Steam-opstartopties. Draagbare instanties die
je gemaakt of geopend hebt worden onthouden (meest recent gebruikte eerst) in
`~/.config/Colony/Eidos/instances.ini`; het welkomscherm van de GUI toont ze om
met één klik te openen, de Steam-start landt op de laatst gespeelde, en de
`nxm://`-handler downloadt erin. Twee kanttekeningen zijn het weten waard: een
draagbare map met de hand verplaatsen behoudt alles behalve tool-vermeldingen die
je met absolute paden naar de oude locatie geregistreerd hebt (`eidos pack` /
`eidos unpack`, hieronder, repareren die voor je; een gewone `mv` niet), en de
gedeelde runtime-cache (`~/.local/share/Colony/Eidos/runtimes/`) blijft
bewust machinebreed - een .NET-host van 78 MB hoort niet per instantie.

Eidos bewaart zijn eigen bestanden onder `Colony/Eidos`, de indeling die elk
programma uit de Colony-familie gebruikt: `~/.config/Colony/Eidos/` voor wat jij
gekozen hebt (voorkeuren, je Nexus-sessie, je instantielijst, de spel- en
add-on-definities die je geschreven hebt), `~/.local/share/Colony/Eidos/` voor
globale instanties (`instances/`), gedownloade runtimes en sessielogs (`logs/`),
en `~/.cache/Colony/Eidos/` voor de LOOT-masterlists, die opnieuw gedownload
worden zodra ze ontbreken.

Een oudere Eidos hield die in `~/.config/eidos/`, `~/.local/share/eidos/` en
`~/.local/state/`. De eerste start na het bijwerken **kopieert** ze over en
meldt dat in het log. Een globale instantie wordt met hardlinks verplaatst in
plaats van met een kopie byte voor byte, zodat tientallen gigabytes aan mods in
een oogwenk verhuizen zonder extra ruimte in te nemen, en haar tool-items worden
naar de nieuwe map omgezet. De oude mappen blijven precies zoals ze waren - er
wordt niets verwijderd, zodat een mislukte upgrade je geen aanmelding en geen
savegame kan kosten - en je kunt ze zelf weghalen zodra je tevreden bent. Lukt
een kopie niet (een volle of alleen-lezen schijf, een oude instantiemap op een
andere schijf, een spel dat er nog vanuit draait), dan blijft Eidos de oude
locatie gebruiken en probeert het bij de volgende start opnieuw. Een
Steam-opstartoptie die nog de oude instantiemap noemt, opent de verplaatste.
Alleen in het begin neemt de verhuizing geen extra ruimte in: een mod of download
die je daarna verwijdert of opnieuw installeert, houdt via de oude instantiemap
zijn oude kopie op de schijf, dus pas het verwijderen van
`~/.local/share/eidos/<game>/` geeft die ruimte terug.

Een draagbare instantie wordt nooit verplaatst: ze blijft waar jij ze gezet
hebt.

Eén plek wordt botweg geweigerd: **in de installatiemap van een spel** (de reflex
van de MO2-veteraan). Steam bezit die boom - een update, een "verify integrity"
of een deïnstallatie kan hem herschrijven of verwijderen en je hele opstelling
meenemen - en Eidos koppelt over de spelwortel, dus een instantie daarbinnen zou
in haar eigen koppeldoel zitten. De wizard, `eidos init` en `eidos play` zeggen
alle drie nee; zet de map NAAST het spel (een buurmap op dezelfde schijf geeft je
hetzelfde gemak).

`play` koppelt de mods van de instantie over de eigen `Data`-map van het spel
(via een bind-stash, zodat de daemon nog steeds de ongerepte bestanden leest)
binnen een privénaamruimte, en draait de opdracht dan door die weergave.
Schrijfacties (saves, opnieuw gegenereerde configs) landen in de
`overwrite/`-laag van de instantie; de spelinstallatie en elke modbron blijven
byte voor byte ongerept.

### Wabbajack-lijsten

Eidos heeft geen Wabbajack-installatieprogramma. Installeer de lijst met CLF3 of Jackify, buiten Eidos, en kies de installatiemap in de wizard als draagbare instantie. Eidos leest de map in de indeling van MO2: `mods/`, `profiles/<naam>/modlist.txt` en de pluginlijsten. Het `ModOrganizer.ini` dat het installatieprogramma schreef blijft staan en wordt nooit overschreven, zodat de eigen programma's van de lijst geregistreerd blijven.

Veel lijsten voegen een map `Stock Game` of `Game Root` toe met een aangepaste kopie van het spel, en MO2 start het spel vandaar. Eidos start de Steam-installatie en heeft geen instelling voor een andere spelmap. Vindt de wizard zo'n map, dan meldt hij dat op het overzichtsscherm en doet verder niets: verplaats zelf wat de lijst daar heeft neergezet naar de map `Root/` van een mod. CLF3 bouwt BSA- en DDS-bestanden opnieuw op met eigen hulpmiddelen, dus het resultaat kan afwijken van een Wabbajack-installatie, en lijstauteurs ondersteunen het niet.

### De modlijst

Net als in MO2 heeft elk profiel zijn eigen `modlist.txt` (welke mods
ingeschakeld zijn, en met welke prioriteit), terwijl de modmappen in de gedeelde
`mods/` van de instantie staan. Een map die je met de hand in `mods/` zet, staat
nog niet in de lijst: ze verschijnt **uitgeschakeld op de hoogste prioriteit** -
onderaan de lijst van het venster, waar MO2 haar ook zet - zodat ze, zodra je
haar inschakelt, haar conflicten wint. Oudere versies zetten haar bovenaan, waar
elke andere mod haar overstemde.

Omdat de mappen gedeeld zijn, verandert het verwijderen of hernoemen van een mod
elk profiel: haar regel verdwijnt uit, of wordt hernoemd in, de `modlist.txt` van
elk profiel, en voor de rest houdt elk profiel zijn eigen staat en volgorde. Een
profiel dat geen mods meer heeft, heeft helemaal geen `modlist.txt` in plaats
van een lege, die als een beschadigd bestand gelezen werd en de volgende
installatie blokkeerde.

Het venster werkt vanuit zijn eigen kopie van de modlijst en de laadvolgorde.
Schrijft een ander Eidos-proces een van beide terwijl het open is - `eidos install`
of `eidos sort` in een terminal, een collectie, een sessie die Steam gestart
heeft, een tweede venster - dan wordt de volgende bewerking van het venster
geweigerd in plaats van dat werk te overschrijven. Het laadt opnieuw vanaf schijf
en zegt *Not saved: another Eidos process changed the mod list. It has been reloaded - redo the change.*
(of hetzelfde voor de laadvolgorde); alleen die ene klik gaat verloren, dus doe
hem opnieuw. Rename, New mod, Add separator, Remove en van de Overwrite een mod
maken controleren dit eerst, voordat ze aan `mods/` komen, zodat een geweigerde
bewerking niets op schijf verandert. De INI-editor weigert evenzo op te slaan
over een bestand dat veranderd is sinds hij het opende.

### Saves horen bij het profiel

Elk profiel bewaart zijn eigen saves in `profiles/<name>/saves/`, en zolang een
run duurt wordt die map over de savemap van het spel in het Proton-prefix
gekoppeld. De eerste start van een profiel kopieert de saves die al in het prefix
staan erin, één keer, zodat een save die je later uit het profiel verwijdert ook
verwijderd blijft. Na elke run worden de nieuwste saves van het profiel
teruggekopieerd naar het prefix voor Steam Cloud; een save in het prefix die
Eidos er niet neergezet heeft, wordt eerst in het profiel bewaard als
`orphan-<time>-<name>`, nooit overschreven.

Een save die na die eerste kopie in het prefix belandt zonder via Eidos te gaan -
een Steam Cloud-download vanaf een Steam Deck, een start zonder Eidos - blijft
verborgen zolang de saves van het profiel gekoppeld zijn. Niets verwijdert hem:
de start benoemt hem in het log van de run (de `logs/run-*.log` van de instantie
bij een start vanuit het venster, de console van Steam bij een start via Steam),
één keer telkens wanneer die verzameling bestanden verandert. Om hem te spelen,
kopieer je hem en zijn co-save naar de map `saves/` van het profiel.

### Wat er veranderd is sinds de laatste goede run

Elke run via `eidos play` of `eidos tool run`, gestart vanuit het venster, een terminal of Steam, schrijft een record naar `<instance>/logs/sessions/`: het profiel, elke ingeschakelde mod met versie en Nexus-bestands-id's, de pluginvolgorde, een checksum van elke profiel-INI, de versie van het spelprogramma, de Eidos-versie, de exitcode en de saves die de run aanmaakte. De 50 nieuwste blijven bewaard, en `logs/` zit niet in een ingepakte instantie. `eidos sessions <instance> diff` vergelijkt de nieuwste spelrun met exitcode 0 met de huidige stand; `diff <id>` kiest een ander record uit de lijst. Eidos herkent hier nog geen crashes, en een spel dat onder Proton crashte kan toch met code 0 eindigen: hoort de gekozen run daarbij, noem dan met `diff <id>` een eerdere.

### Geen bevoorrechte stap nodig

Eidos draait volledig zonder root. Het koppelt in een privé-user- +
mount-naamruimte, dus geen setuid-helper, geen daemon, en niets te verlenen.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` is **optioneel** en regelt
precies één ding: kernel-FUSE-passthrough, dat standaard uit staat omdat een
eerdere versie van de daemon er het spel mee brak en de wijziging die de
waarschijnlijkste oorzaak vermijdt nog niet opnieuw in een spel gemeten is
(hieronder). Met die capability neemt Eidos een gewone mount-naamruimte in
plaats van een user-naamruimte; mods worden hoe dan ook identiek uitgerold.


Waarom het oude `setcap`-advies weg is - en waarom FUSE-passthrough uit
geleverd wordt - wordt uitgelegd in
[troubleshooting.md](troubleshooting.md#waarom-passthrough-standaard-uit-staat).

## Een Nexus-collectie installeren

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run     # haar lezen en zeggen wat er zou gebeuren
```

In het venster: plak de link in **File -> Open a Nexus collection...** en druk op
*Install this collection*.

### Waarom dit geen "download deze mods" is

Een collectie is een RECEPT, geen boodschappenlijstje, en bijna niets van dat
recept is zichtbaar vanaf de Nexus-API. De installatievolgorde, de antwoorden die
de auteur aan de gescripte installer van elke mod gaf, welke mod een
bestandsconflict wint, welke plugins waar laden, de binaire patches - dat alles
staat in een `collection.json` binnen het eigen archief van de collectie.
Dezelfde mods downloaden en ze met hun standaardopties installeren levert een
ander spel op.

Dus downloadt Eidos het archief, leest het recept, en past het toe:

| De collectie zegt | Eidos doet |
| --- | --- |
| `phase` | installeert in die volgorde, optionele leden als laatste |
| `choices` | herspeelt de antwoorden van de auteur op elke FOMOD, en zegt het wanneer dat niet lukt |
| `modRules` | herordent de modlijst zodat de juiste mod elk bestand wint |
| `plugins` / `pluginRules` | voegt de LOOT-regels samen met je userlist en sorteert |
| `INI Tweaks/` | installeert ze als een mod op zichzelf |
| `tools` | vertelt je welke tools het verwacht; maakt er geen aan |

Leden worden **ingeschakeld** zodra ze installeren, achteraan de modlijst in
installatievolgorde, voordat `modRules` ze verplaatst - een mod die door niets
vermeld wordt is een mod die door niets geladen wordt, en een collectie die
roerloos op schijf ligt is de ene mislukking die het verslag niet kon zien.

De staat wordt na **elk** lid weggeschreven naar
`<instance>/collections/<slug>-<revision>.state.json` - naast de map van die
revisie, nooit erin, zodat een collectie haar eigen boekhouding niet kan
meeleveren - en een installatie die bij lid 147 van 200 onderbroken is, gaat
vanaf 147 verder. Ze wordt onder een tijdelijke naam geschreven en op haar plaats
hernoemd, zodat een onderbreking de oude staat achterlaat of de nieuwe en nooit
de helft van een van beide; als ze ooit niet gelezen kan worden, stopt Eidos en
zegt het, in plaats van stilletjes de hele collectie opnieuw te beginnen. Draai
dezelfde opdracht opnieuw.

Een nieuwere revisie installeren van een collectie die je al hebt, neemt de
mappen van de vorige revisie over: een ongewijzigd lid wordt ter plekke
geverifieerd, een lid waarvan de auteur het bestand bijgewerkt heeft wordt binnen
zijn oude map vervangen, en een lid dat de nieuwe revisie geschrapt heeft blijft
geïnstalleerd en wordt in het verslag benoemd, zodat jij het kunt uitschakelen of
verwijderen. Een map die een ander profiel inschakelt, wordt nooit overgenomen,
want de mods-map is gedeeld en dat profiel zou buiten je om veranderen: de nieuwe
revisie installeert dan haar eigen kopie ernaast, en het verslag vraagt je de
oude in dit profiel uit te schakelen, nooit om haar te verwijderen. Teruggaan naar
een oudere revisie neemt haar mappen op dezelfde manier terug, of installeert
verse kopieën van de mappen die ze niet kan overnemen.
Telkens als een lid zo vervangen wordt terwijl je in zijn map bestanden
gewijzigd of toegevoegd had, wordt de oude map eerst ernaast bewaard als
`<name>_backup`, die nooit in het spel geladen wordt, en het verslag noemt hem.

Een overblijfsel van een eerdere revisie dat ingeschakeld blijft, zorgt ervoor
dat de installatie niet telt als die welke de auteur gebouwd heeft; de melding
verdwijnt zodra je het uitschakelt. Een lid dat je met `--no-optional`
overgeslagen hebt, wordt niet overgenomen en wordt op dezelfde manier benoemd.
Mislukt de overname zelf, dan stopt de run met
*previous revision: its folders could not be taken over* en de reden; los dat op
en draai de opdracht opnieuw.

### Wat het niet zal voorwenden

**Een gratis Nexus-account kan de mods van de leden niet ophalen.** Nexus maakt
via de API geen downloadlinks aan voor gratis accounts; elk bestand vergt de
eigen knop "Mod Manager Download" van de site. Het ARCHIEF van de collectie
downloadt prima op een gratis account - dus je kunt het hele recept lezen - maar
elk lid wordt gemeld met zijn exacte pagina-URL om het zelf op te halen. Dat is
een zakelijke regel van Nexus, en geen hoeveelheid opnieuw proberen komt eraan
voorbij. Haal ze op via de knop van de site en draai de opdracht opnieuw: er
wordt bij elke run in `downloads/` naar die leden gezocht, zodat de collectie
afgemaakt wordt naarmate jij ze aanlevert.

In het venster krijgt een gratis account in het collectiepaneel
*Fetch the N missing required files, one by one* in plaats van het ophalen in
partijen. Het opent op Nexus de pagina van het eerste ontbrekende verplichte
bestand, wacht tot dat bestand in `downloads/` aankomt en opent dan het
volgende. De downloadknop van de pagina druk je nog steeds zelf in, voor elk
bestand. Het paneel toont hoe ver het is (*3 of 12*), met *Skip* en *Stop*.
Komt er in plaats daarvan een ander bestand van dezelfde mod binnen, dan noemt
het paneel beide bestands-id's en blijft het wachten op het bestand dat de
collectie vastlegt. Optionele leden doen niet mee en houden hun eigen knop
*Open*.

Toegepast zoals de collectie het beschrijft: **binaire patches** (elk gecontroleerd tegen de vastgelegde CRC), **bestandsoverrides** (de genoemde bestanden worden met `.mohidden` verborgen, zoals MO2 doet) en leden waarvan de bestanden binnen het collectiearchief meereizen (`bundle`). Leden die de collectie je met de hand
laat ophalen (`browse`, `manual`) dragen de eigen instructies van de auteur door
tot in het verslag. Een lid waarvan de naam al een mod van jou is, wordt ernaast
geïnstalleerd onder een vrije naam, nooit eroverheen, en het verslag zegt welke.

### Het verslag

Het punt van het verslag is dat "geïnstalleerd" het vertrouwen waard is. Een lid
waarvan niet alle installer-antwoorden herspeeld konden worden is
**geïnstalleerd, maar niet zoals de collectie het vraagt** - een eigen regel,
apart van de successen waar het oppervlakkig op lijkt, want de bestanden op
schijf verschillen van die van de auteur. Ordeningsregels die niets in de
collectie benoemden, mods gevangen in een lus van tegenstrijdige regels,
LOOT-groepen die niet bestaan, en elk deel van het manifest dat deze versie van
Eidos niet begrijpt, worden ook allemaal vermeld.

Het alternatief - zoiets loggen en de installatie klaar noemen - is hoe een
collectie verkeerd gaat spelen om redenen die niemand kan vinden.

## Een instantie naar een andere machine verhuizen

`eidos pack` schrijft een hele instantie - elke mod, de laadvolgorde, alle
profielen, de Overwrite met je saves erin, en de archieven waaruit alles
geïnstalleerd is - weg naar één enkel bestand. `eidos unpack` zet ze terug:

```sh
eidos pack skyrimse ~/backup.eidos                  # of geef een map op: genoemd naar het spel en de datum
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # op de andere machine
```

`eidos pack --dry-run` somt precies op wat erin zou gaan, wat niet en waarom, en
hoeveel ruimte het nodig heeft, zonder iets te schrijven. `eidos unpack --info`
doet hetzelfde voor een bestand dat je al hebt.

Als 7-Zip iets niet kon lezen, of als er bij het doorlopen iets overgeslagen
moest worden wat je in een back-up zou verwachten (een symbolische link, een map
die niet gelezen kon worden, een naam die niet veilig aan 7-Zip meegegeven kan
worden), wordt het archief alsnog geschreven en van een naam voorzien - het is
het waard om te hebben - maar `eidos pack` sluit af met
**1** en zegt wat er ontbreekt. Een onvolledige back-up is geen succes, en dit is
een opdracht die mensen vóór `&&` zetten.

Een `.eidos`-bestand is een 7-Zip-archief onder een eigen naam, en dat is een
keuze, geen vermomming: 7-Zip is toch al nodig om überhaupt een mod te
installeren, dus dit voegt geen afhankelijkheid toe, en raak je Eidos ooit
kwijt, dan opent je back-up nog steeds in elk archiefprogramma dat je hebt. Het
wordt gecomprimeerd met LZMA2 op `-mx1` - ongeveer 43% van het origineel bij een
textuurzware lijst, in een fractie van de tijd die `-mx9` besteedt om een paar
punten meer te winnen. Texturen en meshes zijn al gecomprimeerde formaten; er
blijft heel weinig over voor een groter woordenboek om te vinden. `--level` (0,
1, 3, 5, 7, 9) overstemt dat als je het voor je eigen inhoud oneens bent, en
`--no-downloads` laat `downloads/` weg.

Op het standaardniveau 1 gaan de bestanden in
**solide blokken van hoogstens 16 MB**. Eén bestand uit een back-up van 70 GB
halen decodeert hoogstens het blok waarin het staat, nooit alles wat ervoor
staat, en het packen van game-inhoud gaat 2 tot 5 keer sneller dan met elk
bestand apart gecomprimeerd (uitpakken 2 tot 4 keer). De prijs betaal je bij
schade: één corrupte byte kan elk bestand in zijn blok meenemen, tot 16 MB aan
naburige bestanden. De herstelgegevens hieronder dekken dat af. Alle andere
niveaus blijven niet-solide: elk bestand is een eigen stroom, en een corrupte
byte kost alleen dat ene bestand. Niveau 0 slaat de bestanden ongecomprimeerd
op. Vanaf niveau 3 werkt LZMA2 al in stukken van 16 MB of meer, en daar waren
solide blokken bij het meten trager: op niveau 5 duurde het packen van meshes
1,4 keer zo lang en het uitpakken van texturen 6,8 keer zo lang.

Packen neemt het slot van de instantie, dus het kan niet draaien terwijl het
spel of een andere Eidos naar die instantie schrijft - de back-up is een
momentopname, geen veeg. Ze wordt onder een `.part`-naam geschreven, naar de
schijf weggeschreven en met `7z t` volledig teruggelezen voordat ze de naam
krijgt die je gekozen hebt. Die controle decodeert elk blok en vergelijkt elke
checksum, hetzelfde werk dat uitpakken op de andere machine doet. Ze laat zien
wat de schijf direct na het schrijven teruggaf, niet dat de schijf het bewaart;
daarvoor zijn de herstelgegevens hieronder. Mislukt ze, dan wordt de `.part`
verwijderd, wordt er niets onder jouw naam geschreven en citeert `eidos pack` de
regel waarin 7-Zip de schade vond. Een onderbroken of mislukte pack laat iets
zichtbaar onafs achter, of niets, in plaats van een `.eidos`-bestand dat
compleet lijkt en het niet is.

Als par2cmdline geïnstalleerd is (`sudo pacman -S par2cmdline` op Arch,
`sudo apt install par2` op Debian en Ubuntu, `sudo dnf install par2cmdline` op
Fedora), schrijft `eidos pack` ook PAR2-herstelgegevens naast het archief:
`backup.eidos.par2` en een paar `backup.eidos.volNN+MM.par2`-bestanden, samen 5%
van het archief plus een kleine index. Houd ze in dezelfde map als het
`.eidos`-bestand, onder de namen waarmee ze geschreven zijn, en kopieer ze mee:
een back-up die zonder ze verhuist, heeft niets om uit hersteld te worden, en de
herstelgegevens leggen de naam van het archief vast, dus een hernoemde back-up
past er niet meer bij. `eidos unpack` vindt ze op naam en controleert het
archief ermee voordat er iets uitgepakt wordt, ook een back-up waarvan het einde
zo is afgekapt dat 7-Zip hem niet kan openen. Zijn er een paar bytes kapot, dan
herstelt par2 eerst het archief en houdt de beschadigde kopie als
`backup.eidos.1`, waarvoor in de map evenveel vrije ruimte nodig is als het
archief groot is; schade die verder gaat dan die 5% dekt, stopt het uitpakken
voordat er één bestand geschreven is. Herstelbestanden die niet bij het archief
passen, zoals die van een oudere back-up met dezelfde naam, worden niet gebruikt
als 7-Zip het archief leest en het intact vindt, en het uitpakken meldt dat. Op
een machine zonder par2 gaat het uitpakken toch door, met een waarschuwing dat
de herstelgegevens niet gebruikt zijn. `--no-recovery` packt zonder, en
`eidos pack --dry-run` zegt of ze geschreven worden.

### Wat er niet in zit, en waarom

| Weggelaten | Waarom |
| --- | --- |
| Het Proton-prefix | Het is naar de machine gevormd (schijftoewijzingen, een `Z:`-kijk op *dit* bestandssysteem) en Eidos bouwt het in een minuut opnieuw op met `eidos prereqs <instance> --install`. Het meenemen zou het archief ruwweg verdubbelen om iets te versturen dat de andere machine toch opnieuw moet aanmaken. |
| Het spel | Een instantie modt een spel dat ze niet bevat. Installeer het daarginds vanaf Steam. |
| Tools buiten de instantie | xEdit, DynDOLOD en consorten zijn programma's van derden met hun eigen installatieprogramma's. Ze worden in het archief wel **benoemd**, dus bij het uitpakken hoor je precies welke er op de nieuwe machine ontbreken. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Verslagen van de machine die de back-up gemaakt heeft. `prereqs.done` zou in het bijzonder beweren dat de runtime-bibliotheken al geïnstalleerd zijn in een prefix dat er niet is. |
| `loot/`, behalve `userlist.yaml` | Een cache van de masterlist die Eidos op verzoek opnieuw ophaalt. Je eigen LOOT-regels zijn geen cache - niemand haalt die opnieuw op - dus dat ene bestand gaat mee. |
| `.base/`, `.base-root/` | Lege koppelpunten waar de eigen bestanden van het spel tijdens een sessie gestald worden. |
| Halfgeschreven bestanden | Een gepauzeerde download (`*.unfinished`), een atomaire schrijfactie onderweg (`*.eidos-tmp*`). |
| Symbolische links | 7-Zip zou er een volgen en kopiëren waar hij naar wijst, wat bij een absolute link betekent dat er een vreemde boom je back-up in getrokken wordt. Ze worden in plaats daarvan gemeld, en de back-up telt als onvolledig: een `mods/` of `downloads/` die naar een andere schijf gelinkt is, wordt NIET ingepakt. Wil je bewust een back-up maken zonder een gelinkte `downloads/`, geef dan `--no-downloads` mee (of vink *Include downloads/* uit in het venster); een bewuste uitsluiting telt niet als ontbrekend. |

Elk van deze belandt in `eidos-backup.ini` in de wortel van het archief, met de
reden erbij - dus het antwoord op "wat zit hier niet in" is `cat`, geen gok.
Datzelfde bestand noteert waar de instantie vroeger stond, welk profiel actief
was, hoe groot ze uitgepakt wordt, en welke tools de nieuwe machine nodig zal
hebben.

Downloadverslagen (`downloads/*.meta`) gaan mee met hun `url=`-regel
**leeggemaakt**. Die URL is een ondertekende Nexus-link: hij houdt binnen enkele
uren op te werken, en hij draagt het `user_id` van het account dat het bestand
gedownload heeft. Een back-up is gemaakt om aan iemand anders gegeven te worden,
dus draagt ze dat niet mee. Alles wat de download terugvindbaar maakt - het
mod-id, het bestands-id, de versie - blijft.

### In het venster

Beide zitten in het menu **File**: *Pack this instance...* en
*Unpack a backup...*. Uitpakken staat ook op het welkomscherm, onder de lijst
met instanties, want de machine die het het hardst nodig heeft, is die zonder
instanties.

De Pack-dialoog toont een voorbeeld voordat ze zich vastlegt: hoeveel bestanden,
hoe groot, hoeveel daarvan `downloads/` is, ruwweg waar het archief op uitkomt,
en welke tools wel benoemd maar niet meegenomen worden. Het vakje *Write
recovery data* is aangevinkt als par2 geïnstalleerd is, en grijs met het te
installeren pakket als dat niet zo is. De Unpack-dialoog leest het manifest van
de back-up zodra je het bestand kiest, zodat ze je kan vertellen welk spel erin
zit, wanneer ze gemaakt is, waar ze vroeger stond en welke van haar tools hier
ontbreken - voordat je haar een map geeft.

Beide draaien op een werkthread met een voortgangsbalk, zodat het venster blijft
antwoorden terwijl ze bezig zijn, en de balk wordt het verslag zodra ze klaar
zijn. Packen houdt het slot van de instantie de hele run vast: niets anders mag
de instantie aanraken terwijl ze gelezen wordt, en dat is wat de back-up een
momentopname maakt in plaats van een veeg.

### Wat uitpakken repareert

Een gewone kopie van de map zou elke toolknop laten wijzen naar een pad op de
oude machine. `eidos unpack` herschrijft de waarden die de oude instantiewortel
benoemen en alleen die: `exe`, `workdir` en de genummerde `arg`-sleutels in
`tools.ini`, en `installationFile` in de `meta.ini` van elke mod. Een tool die
nooit binnen de instantie zat (`/opt/xedit/SSEEdit.exe`) blijft precies zoals hij
was en wordt vermeld als iets om opnieuw te installeren - gokken waar de xEdit
van iemand anders gebleven is, is verzinnen, geen repareren. Het corrigeert ook
of de instantie zichzelf centraal of draagbaar noemt, zodat latere opdrachten
haar zoeken waar ze nu staat.

Uitpakken weigert een map die niet leeg is (geef `--force` mee als je er echt
overheen wilt), een schijf die te klein is voor het cijfer van het manifest zelf,
een archief gemaakt door een nieuwere Eidos dan de jouwe, en elk archief met een
item dat *buiten* de door jou gekozen map geschreven zou worden.

Daarna:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # het Proton-prefix opnieuw opbouwen
eidos play /mnt/games/EidosSkyrim
```

## GUI

```sh
cargo run -p eidos-gui
```

Een wizard bij de eerste start in MO2-stijl, getekend in de gedeelde
Colony-thema's: welkom -> instantietype (draagbaar / globaal) -> spel -> naam & locatie
-> samenvatting -> maken -> hoofdscherm. Het welkomscherm toont ook elke bekende
bestaande instantie (globaal en draagbaar, laatst gebruikte eerst) om met één
klik te openen - het dient meteen als instantiewisselaar - en de wizard op een
map richten die al een instantie bevat NEEMT die over zoals ze is in plaats van
eroverheen te maken (met botte weigering als de map bij een ander spel hoort).

Het hoofdvenster met twee panelen is er ook: een profielkiezer (wisselen, of een
nieuw profiel maken door het huidige te kopiëren), een modlijst die je filtert,
selecteert, herordent, groepeert met scheidingen, per categorie versmalt en
rechtsklikt voor acties, plus de tabbladen Data / Plugins / Conflicts / Overwrite
/ Saves / Downloads / Diagnostics en een Run-knop met een keuzelijst voor het
doel.

Herordenen is niet alleen naar boven of naar onder sturen: de gerichte
verplaatsingen van MO2 zitten er ook in - boven de eerste botsende mod, onder de
laatste, naar een expliciete prioriteit, of in de groep van een scheiding. Ze
lopen allemaal door één gedeelde verplaatshelper, zodat de off-by-one die
ontstaat door rijen te verwijderen voor je ze opnieuw invoegt op één plek bestaat
in plaats van vijf.

### Preferences

Klik linksboven op **Eidos** om Preferences te openen in plaats van de twee
panelen, en klik er nogmaals op (of op **Close**) om terug te gaan. General,
Appearance en Accessibility komen eerst, zoals in elk Colony-programma, daarna
Mod list, Nexus en About. Elke wijziging wordt meteen toegepast en opgeslagen; er
is geen Save-knop.

- **General**: het spel dat Eidos opent, de venstergrootte, de taal van de
  interface, de tools folder, hoe een spel gestart wordt, en of een
  bureaubladmelding laat weten wanneer een lange taak eindigt terwijl het venster
  op de achtergrond staat. Het venster spreekt
  Engels en Frans; **System** volgt `LC_ALL`, `LC_MESSAGES` of `LANG`, en de
  wijziging gaat meteen in. De opdrachtregel `eidos` antwoordt altijd in het Engels.
- **Appearance**: het thema (25 families, 57 paletten, standaard het perkament
  van Eidos), de accentkleur, de tekstgrootte, effecten en een live voorvertoning.
- **Accessibility**: hoog contrast, een lettertype dat prettig is bij dyslexie,
  minder beweging en een leestekstgrootte die vermenigvuldigd wordt met die van
  Appearance.

Deze staan in `~/.config/Colony/Eidos/preferences.json`. Het thema dat een oudere
Eidos in `settings.ini` bewaarde, wordt de eerste keer overgenomen.

### Kolommen, sorteren en groeperen

De lijst tekent standaard vier kolommen en biedt er acht: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Vink ze aan in het View-menu.
Dat niet alle acht standaard aan staan is opzet - een lijst waarin elke kolom
getoond wordt houdt geen ruimte over voor de NAAM, en dat is de kolom die je
werkelijk leest.

Klik op een kop om erop te sorteren. Nog eens klikken keert om, en een derde klik
keert terug naar de **laadvolgorde**, wat meer uitmaakt dan het klinkt: de
laadvolgorde is de enige volgorde waarin de lijst gesleept kan worden, omdat een
invoegopening de echte lijst aanspreekt terwijl een gesorteerde rij ergens heel
anders staat. Zolang een sortering aan staat worden de invoegstroken niet
getekend en wordt een sleep geweigerd in plaats van ergens te landen waar niemand
op mikte - hetzelfde wat MO2 doet, en om dezelfde reden. Het View-menu zegt dat
en biedt de weg terug.

Het View-menu kan de hele lijst ook **groeperen**, per categorie of per bron (van
Nexus, of met de hand geïnstalleerd). Groepskoppen zijn geen scheidingen: er zit
niets achter om te hernoemen, te kleuren of te verplaatsen, ze klappen in, en het
aantal blijft bij het inklappen op de kop staan. Scheidingen verlaten de lijst
onder een sortering of een groepering - een scheiding voert de rijen aan die in
de laadvolgorde op haar volgen, en beide hebben die rijen verplaatst.

### Muis en toetsenbord

Dubbelklik op een mod voor Information, Ctrl+dubbelklik voor haar map,
Shift+dubbelklik voor haar Nexus-pagina. Ctrl+F zet de cursor in het filtervak.
Een letter typen springt naar de volgende mod die ermee begint, en nog eens
drukken loopt de rest af in plaats van op de eerste te blijven hangen. Geen ervan
kan landen op een rij die het filter, een ingeklapte scheiding of een ingeklapte
groep verbergt - een markering verplaatsen die je niet ziet is hoe de volgende
spatie een mod omschakelt waar je niet naar keek.

"Collapse others" in het menu van een scheiding klapt elke groep in behalve die
ene. Tijdens een sleep opent een ingeklapte groep als je erop blijft rusten,
zodat een mod erin gelaten kan worden zonder de sleep eerst op te geven - rusten,
niet er even langs strijken.

Ctrl+Z maakt de laatste wijziging in de modlijst of de laadvolgorde ongedaan: een
vinkje, een verplaatsing, een batchgewijze uitschakeling, een LOOT-sortering. Ctrl+Shift+Z voert die opnieuw uit.
Het venster bewaart 50 stappen en vergeet ze bij F5, aan het eind van een spel- of
toolrun, bij een profielwissel en wanneer een ander Eidos-proces de lijst wijzigt.
Een mod installeren, hernoemen of verwijderen verandert de mappen op schijf, dus
ongedaan maken voorbij zo'n stap wordt geweigerd.

### Wat de lijst je over een mod vertelt

Twee adviserende vlaggen, allebei een teken met de uitleg bij het zweven. **No
valid game data** betekent dat niets bovenin de mod eruitziet als iets wat dit
spel laadt; misschien moeten haar mappen een niveau omhoog, of misschien is het
geen mod voor dit spel. **Another game** betekent dat de eigen `meta.ini` van de
mod een ander spel benoemt. Geen van beide blokkeert iets - de mod wordt nog
steeds uitgerold - en "Mark as valid" in het rijmenu legt ze allebei het zwijgen
op, via de eigen `validated=`-sleutel van MO2, zodat een mod waarvoor je in de
ene manager ingestaan hebt stil aankomt in de andere.

De indelingscontrole is bewust ruimhartig: een `Root/`-boom telt, een onleesbare
map telt, een lege telt. Een verkeerde waarschuwing in een lijst van vijfhonderd
rijen is erger dan een ontbrekende.

### Een mod uitzetten die je saves of andere mods nodig hebben

Voordat een mod wordt uitgezet of verwijderd, leest Eidos de pluginlijsten van de
drie nieuwste saves van het profiel. Gebruikt een ervan een plugin uit die mod,
dan noemt een balk boven de lijst de plugins en wacht op "Disable anyway" of
"Cancel" (Esc annuleert ook). Remove zet dezelfde namen in zijn melding om
nogmaals te klikken. Alleen saves van Skyrim en Fallout 4 worden gelezen; andere
games zetten de mod uit zonder te vragen.

Het tabblad General van Information bij een mod toont de mods die ze nodig heeft en de mods die haar nodig hebben, gelezen uit de masters van de ingeschakelde plugins. Een mod uitzetten die anderen nodig hebben toont dezelfde balk met "Disable them too", wat ook elke mod uitzet die haar nodig heeft en de mods die die weer nodig hebben, of "Keep them enabled". De kaart voor ontbrekende masters op het tabblad Health heeft "Enable what provides them": die schakelt de mod in die elke ontbrekende master bevat en vinkt een masterplugin aan die wel aanwezig is maar niet is aangevinkt.

### Een mod back-uppen voor je eraan komt

"Back up this mod" kopieert haar map opzij als `<name>_backup` (daarna
`_backup2`, enzovoort - een back-up vervangt nooit de vorige). De kopie is
**inert**: het is geen mod, haar aanvinkvakje doet niets, en ze draagt niets bij
aan de samengevoegde weergave, want haar aanvinken zou twee kopieën van één mod
over elkaar uitrollen. "Restore this backup over the mod" zet haar terug, in twee
klikken; de huidige inhoud wordt eerst opzijgezet en pas weggegooid zodra de
kopie geslaagd is.

**Data** is een echte boom van de samengevoegde weergave, telkens één niveau
uitgeklapt, zodat een knoop openen één maplezing kost per laag die hem heeft in
plaats van een recursieve wandeling door elke ingeschakelde mod. Ze wordt
beantwoord door DEZELFDE lagenstapel waaruit de koppeling bedient, dus whiteouts
en verborgen bestanden worden gerespecteerd en het tabblad kan niet in
tegenspraak zijn met wat het spel zal zien. Filter op naam, versmal tot alleen
betwiste bestanden, zoek met de kolommen Size en Modified uit wat waar staat, en
toon elke rij met Reveal in een bestandsbeheerder. De `meta.ini`, `readme.txt`,
`fomod/` en `Root/` in de bovenste map van een mod blijven buiten Data, in dit
tabblad en in de koppeling: de eerste drie zijn voor de modbeheerder, en `Root/`
gaat naar de spelmap. Een bestand dat het spel of een hulpprogramma onder een
van die namen schrijft, komt in de Overwrite terecht en blijft zichtbaar.
**Plugins** is de
ESP/ESM/ESL-laadvolgorde (omschakelen, met de hand herordenen, of sorteren met
LOOT en het verslag na de sortering lezen, waarvan de adviezenlinks in je browser
openen). De kop telt de ingeschakelde plugins per indexruimte tegen de limiet van het spel, bijvoorbeeld `full 231/254, light 512/4096`, en wordt oranje zodra er een vol is. Na een LOOT-sortering laat een volledige plugin waarvan alle nieuwe records in het light-bereik passen dat bij aanwijzen zien, en het menu van zijn rij bevat Flag as light. Eidos schrijft een kopie met de light-vlag in een nieuwe mod met de naam `<plugin> (light)`, onderaan de modlijst (de hoogste prioriteit), en laat het origineel ongemoeid, dus die mod uitzetten maakt het ongedaan. Eidos weigert als een van de drie nieuwste saves van het profiel de plugin als volledige plugin laadt. Alleen saves van Skyrim en Fallout 4 worden gelezen, dus bij Starfield wordt geen save gecontroleerd. LOOT controleert alleen recordbereiken: bekijk een plugin die cellen of worldspaces toevoegt in xEdit voordat je hem markeert. **Conflicts** legt de winnaars en verliezers per bestand uit.
**Overwrite** maakt in één stap een echte mod van wat het spel geschreven heeft.
**Saves** ontleedt de kop van elke save - personage, niveau, locatie, speeltijd -
en vergelijkt de erin gebakken pluginlijst met je huidige, met een knop die de
mods inschakelt die ze nodig heeft, want ze benoemen en het verder aan jou laten
is de saaie helft.

"Information..." opent een dialoog per mod: algemeen, conflicten, bestandsboom,
INI-tweaks, notities. Vanuit de bestandsboom (en vanuit de Data-boom) kan elk
bestand **verborgen** worden - hernoemd naar `<name>.mohidden`, wat het uit de
virtuele weergave haalt zonder het te verwijderen, zodat drie verdwaalde meshes
van één mod onderdrukt kunnen worden zonder aan prioriteiten te komen. De
bestandsboom doet ook de gewone bestandsbewerkingen: nieuwe map, hernoemen,
verwijderen, openen. Ze lopen allemaal door één resolver die alles weigert wat
geen gewoon pad binnen die mod is - geen `..`, geen absoluut pad, en geen
component dat een symlink is, want er een volgen zou een verwijdering helemaal
buiten de modmap brengen. Hernoemen vervangt alleen het laatste component, zodat
het nooit een verplaatsing kan worden, en het weigert een naam die al bezet is in
plaats van dat bestand stilzwijgend te vervangen. Verwijderen vergt twee klikken;
het is de ene actie hier die nog eens klikken niet ongedaan maakt.

**View** op elke rij in de bestandsboom of de Data-boom toont een voorbeeld van
het bestand: afbeeldingen en tekst. Geen DDS of NIF - die vergen een blokdecoder
en een renderer die deze boom niet heeft - maar ze zeggen dat in plaats van een
leeg vak te tonen, en wijzen naar Reveal. Tekst wordt tot 64 KB gelezen en meldt
waar ze gestopt is, want een voorbeeld is een blik en een Papyrus-log kan honderd
megabyte zijn. **INI Tweaks** toont de fragmenten die een mod in haar map
`INI Tweaks/` meelevert; de ingeschakelde worden bij het starten in
prioriteitsvolgorde samengevoegd met de spel-INI van het profiel, en er weer af
gehaald wanneer de INI's van de run vastgelegd worden - anders wordt een tweak
stilzwijgend een instelling en zou hem uitschakelen niets doen.

Een download kan **vanuit de lijst Downloads op een positie in de modlijst
gesleept** worden om hem op die prioriteit te installeren, en archieven of mappen
die je vanuit een bestandsbeheerder op het venster laat vallen installeren ook
(die helft vergt een X11- of XWayland-sessie - winit implementeert bestandsdrops
alleen voor X11). Downloads zelf kunnen gepauzeerd en hervat worden: pauzeren
stopt de overdracht en houdt het gedeeltelijke bestand, en Resume lost een verse
link opnieuw op en gaat verder waar het gestopt is.

Het tabblad Downloads is een **bibliotheek** van archieven, geen
overdrachtswachtrij. Filter op naam (ook de vriendelijke modnaam, dus "skyui"
vindt `SkyUI_5_2_SE-12604-5-2SE.7z`), sorteer op nieuwste, naam, grootte of
staat, en **verberg** een archief waarmee je klaar bent - wat het bestand houdt
en alleen de rij laat vallen, zodat een boek wegzetten niet hetzelfde is als het
verbranden. "Show hidden" haalt ze terug, en dezelfde knop maakt ze weer
zichtbaar. "Remove N installed" verwijdert de archieven van mods die je al
geïnstalleerd hebt, in twee klikken, en alleen die **op het scherm**: met het
filter heb je gezegd welke je bedoelde.

### Mod-updates

De updateknop aan het rechtereinde van de werkbalk vergelijkt elk geïnstalleerd
bestand met het bestand dat het op Nexus verving. Auteurs markeren een nieuwe
upload als opvolger van een oudere, en Eidos volgt die keten vanaf het
bestands-id dat de `meta.ini` van de mod vastlegt. Noemt de keten geen opvolger,
dan wordt in plaats daarvan de versie van de modpagina vergeleken.

Het tabblad **Nexus files** van "Information..." toont de bestanden van de mod
op Nexus, nieuwste eerst, en markeert het bestand dat hier geïnstalleerd is en
het bestand dat het vervangt. Die tweede rij heeft een knop **Update**. Met een
premium-account downloadt Eidos het bestand zelf. Met een gratis account opent
het de pagina van het bestand, waar Mod Manager Download het bestand
terugstuurt, en de installatie begint zodra het archief in `downloads/` aankomt.
Vanaf daar is het de gewone installatie: een FOMOD opent met de antwoorden
waarmee de mod geïnstalleerd werd, daarna biedt de vraag Replace aan met "Keep a
backup of the existing mod" aangevinkt, wat je bij de vorige vraag ook koos.
Replace behoudt de prioriteit van de mod en of hij aanstaat.

Een map met meerdere Nexus-bestanden, zoals een hoofdbestand met een optioneel
bestand erin samengevoegd, wordt geweigerd: hem vervangen door het nieuwe
hoofdbestand zou de andere kwijtraken. Werk die met de hand bij.

`eidos nexus update skyrimse --apply` doet dezelfde controle en downloadt daarna
met een premium-account elk nieuw bestand en installeert het met
`--replace --backup` over zijn map. Met een gratis account drukt het in plaats
daarvan `OPEN <url>` af voor elke bestandspagina. Mappen die meerdere bestanden
samenvoegen worden met de reden overgeslagen. Een FOMOD-mod zonder opgeslagen
antwoorden, zoals een die geïnstalleerd is voordat Eidos ze bewaarde, wordt niet
opnieuw geïnstalleerd met de standaardwaarden van het installatieprogramma: die
update mislukt met een melding om de mod één keer vanuit het venster bij te
werken, dat de antwoorden dan opslaat. Een FOMOD waarvan het
installatieprogramma niets te kiezen geeft, wordt gewoon geïnstalleerd. De
opdracht eindigt met 1 als een update mislukte.

### Nexus-collecties

Plak een collectielink - of klik er een aan op de site - en Eidos toont de leden
van die revisie, elk gekoppeld aan deze instantie: geïnstalleerd, gedownload of
ontbrekend. *Install this collection* draait vervolgens het hele recept op een
werkthread, met dezelfde voortgangsbalk die de Pack- en Unpack-taken gebruiken,
en de balk wordt het verslag zodra de run klaar is. Alles hierboven over wat er
toegepast wordt en wat er alleen benoemd wordt, geldt hier woord voor woord: het
venster en `eidos collection` zijn één motor met twee front-ends.

Een collectie kan alleen tegen **haar eigen spel** gelezen worden. Open een
Skyrim-collectie met een Fallout 4-instantie geladen en het weigert met naam en
toenaam in plaats van de leden aan de verkeerde modlijst te koppelen, waar elke
"installed" en elke "missing" ruis zou zijn in de vorm van een antwoord.

Als u op updates controleert, vraagt Eidos Nexus ook om de nieuwste revisie van elke collectie die in deze instantie is geïnstalleerd. Een regel boven de modlijst noemt elke collectie met een nieuwere revisie en telt de toegevoegde (+), bijgewerkte (~) en vervallen (-) mods; houd de muis erboven voor de namen. De aantallen ontbreken als het recept van de geïnstalleerde revisie niet meer in de cache staat. **Bijwerken** opent de nieuwere revisie in het collectiepaneel, waar *Install this collection* hem uitvoert. De lijst wordt bij elke controle opnieuw opgebouwd en niet bewaard.

### Offlinemodus

**Preferences -> Nexus -> Offline** laat Eidos Nexus helemaal niet meer benaderen.
Updatecontroles, aanmelden, downloads en collecties zeggen dat, in plaats van te
falen met een verbindingsfout. Het staat uit tenzij je het aanzet - een
instellingenbestand dat door een oudere Eidos geschreven is heeft zo'n sleutel
niet, en een ontbrekende als "aan" lezen zou het netwerk afsnijden voor iedereen
die bijwerkt.

**Preferred servers** rangschikt de CDN-knooppunten die een download verkiest,
beste eerst. Alleen een premium-account krijgt ooit meer dan één mirror om uit te
kiezen, dus voor alle anderen kiest Nexus en verandert dit niets. Het is een
volgorde, geen filter: staat vandaag niets van wat je genoemd hebt op het menu,
dan gebeurt de download toch, vanaf het knooppunt dat Nexus als eerste aanbood.

**Categories** zijn bewerkbaar, niet alleen zichtbaar: ken ze toe aan één mod of
een hele selectie, bewerk de catalogus zelf vanuit dezelfde dialoog, en haal de
officiële categorielijst van het spel op bij Nexus. Beide catalogusbestanden zijn
die van MO2 (`categories.dat` en `nexuscatmap.dat`), dus een gedeelde instantie
houdt één catalogus. Een van Nexus gedownloade mod krijgt bij de installatie een categorie, zodra Ophalen van Nexus de catalogus heeft gekoppeld.

**View -> INI editor** bewerkt de spel-INI's van het profiel - de kopie die
blijft bestaan, niet die begraven in het Proton-prefix die bij elke start
overschreven wordt. **View -> Log** leest de sessielogs. **View -> Extensions**
toont je eigen add-ons; zie [extensions.md](extensions.md).

Installeren aanvaardt alles: de Simple- en FOMOD-paden, plus Wrye
Bash-**BAIN**-pakketten (vink de subpakketten aan, die op volgorde samenvloeien)
en een **handmatige** kiezer die de archiefboom toont en je op de dataroot laat
wijzen wanneer geen enkele heuristiek de indeling herkent. Geen enkel archief
wordt geweigerd.

Een FOMOD-mod opnieuw installeren of bijwerken opent de wizard met de antwoorden
van de vorige installatie aangevinkt, op de eerste stap waarvan de opties
sindsdien veranderd zijn, met een lijst van de wijzigingen. **Reinstall with new
choices** begint in plaats daarvan bij de standaardwaarden van het
installatieprogramma. `eidos install --replace` of `--merge` hergebruikt de
opgeslagen antwoorden en weigert, met de lijst van wijzigingen, wanneer ze niet
meer passen.

**Diagnostics** draait live gezondheidscontroles: bovenal de startmogelijkheid,
ontbrekende masters (de betrouwbaarste crashvoorspeller die er is), archieven die
geen enkele actieve plugin zal laden, of de modlijst nog overeenkomt met de
mods-map, en - na een run - wat het eigen log van de script extender over elk van
zijn plugin-DLL's zegt, wat "zijn mijn SKSE-plugins geladen?" van een
gevolgtrekking in bewijs verandert.
Het meldt ook een game-update sinds de laatste schone start, een ontbrekende script extender of een die niet bij de spelversie past (met de downloadpagina van de extender en een knop die het gedownloade archief installeert), een Steam-startoptie die Eidos niet met `%command%` start, NVAPI dat voor DLSS uit blijft, exclusief volledig scherm dat framegeneratie tegenhoudt, een ENB-`enblocal.ini` zonder `LinuxVersion=true`, de plugindeclaraties van F4SE- en SFSE-plugins, en het crashlog van de laatste sessie met de mods die het noemt.
Op updates controleren leest ook de Nexus-vereisten van elke mod. Diagnostics toont een vereiste mod die ontbreekt of is uitgeschakeld, met Ignore voor een vereiste die op een andere manier is vervuld.

Om het spel via de GUI te starten, zet je de Steam-opstartoptie van het spel op
het absolute pad van het binaire bestand (Steam ziet `~/.cargo/bin` niet in
PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos opent op de instantie van dat spel - de laatst gebruikte, dus een draagbare
instantie wordt net zo goed teruggevonden als de globale; klik op Run om het door
de samengevoegde weergave te starten.

Met **Start the game at once when a store opens Eidos** aan (Preferences, General, Running a game) slaat Eidos die klik over: het controleert Health en start het spel zonder zijn venster te tonen. Het venster opent zoals gewoonlijk wanneer Health een probleem meldt, wanneer de modlijst aandacht nodig heeft of wanneer een tool die vóór het spelen moet draaien een verouderde uitvoer heeft. Wil je het venster voor één start, zet dan `EIDOS_WINDOW=1` voor het commando: `EIDOS_WINDOW=1 ~/.cargo/bin/eidos-gui %command%`.

Buiten Steam start Run het spel niet zelf via Proton of Wine. Het vraagt de
winkel die de kopie bezit om het te starten, en de winkel start Eidos:

- Steam draait `steam -applaunch <appid>` als de opstartoptie hierboven is
  ingesteld. Is Steam gesloten, dan start Steam eerst.
- Heroic (een GOG- of Epic-kopie, native of Flatpak) krijgt een
  `heroic://launch`-link als Eidos een wrapper is in de instellingen van het
  spel in Heroic, met het absolute pad van `eidos-gui` als uitvoerbaar bestand
  en zonder argumenten. Is Heroic gesloten, dan start Heroic eerst.
- Legendary op zichzelf draait `legendary launch <app>` als
  `~/.config/legendary/config.ini` in de sectie `[<app>]` van het spel, of onder
  `[default]`, `wrapper =` met het absolute pad van `eidos-gui` bevat.

Eidos opent dan een tweede venster op de instantie die je het laatst gebruikte,
en daar klik je op Run. Het eerste venster volgt die sessie niet, dus druk
daarin na het spelen op F5 om zijn lijsten opnieuw te laden. Ontbreekt de
instelling in de winkel, of staat de loader van de script extender niet naast
het uitvoerbare bestand van het spel, dan start Run niets en toont het wat je
waar moet instellen, met het echte pad van het draaiende binaire bestand.

De `%command%` van Steam wijst bij de Bethesda-titels meestal naar
`<Game>Launcher.exe`. Eidos draait die nooit: de launcher is een aparte
instellingenapp die `Data` opnieuw scant en `plugins.txt` herschrijft, en zo de
zojuist uitgerolde laadvolgorde ongedaan maakt. Het zet er de loader van de
script extender voor in de plaats als er een geïnstalleerd is, anders het binaire
bestand van het spel, en meldt het wanneer het moet terugvallen - een spel dat
start met elke SKSE-mod inert is erger dan een spel dat niet start.

Oudere instructies hier dwongen `WINEDLLOVERRIDES="d3dcompiler_47=n"` af. Dat is
niet meer nodig en klopte nooit helemaal: een override naar *native* helpt alleen
als er al een echte `d3dcompiler_47.dll` in het prefix staat. Eidos scant nu de
DLL-imports van de ingeschakelde mods, rolt zelf de echte Microsoft-DLL uit, en
zet pas dan de override.

## Het proof of concept proberen

Geen spel nodig. Het bewijst union + copy-on-write + zero-touch + bereik per
naamruimte met alleen onbevoorrechte OverlayFS in een user-naamruimte
(Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Tools

xEdit, BodySlide, DynDOLOD en consorten draaien door de samengevoegde weergave
binnen het Proton-prefix van het spel:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # wat de geregistreerde tools nodig hebben, en de staat ervan
eidos prereqs skyrimse --install  # ophalen wat ontbreekt
```

Eén ding om te weten voor je een tool een naam geeft: **de titel bepaalt welke
runtime-DLL's Eidos ervoor klaarzet** - `BodySlide` krijgt zijn
DirectX-bibliotheken, `BS` krijgt niets. In de GUI toont de dialoog Executables
onder het veld de echte staat van elke vereiste, en de ontbrekende zijn knoppen.

De tabel, de drie niveaus van vereisten, waarom DynDOLOD een .NET-runtime nodig
heeft die winetricks niet kan installeren, en waarom een als mod geïnstalleerde
tool vanaf het samengevoegde pad gestart wordt in plaats van vanuit zijn eigen
map, staan in [tools.md](tools.md).

Bouwen vanaf de broncode en de indeling van de repository staan in
[../internals/contributing.md](../../../../internals/contributing.md).

## Extensies

Eidos kan uitgebreid worden zonder opnieuw gebouwd te worden: een TOML-manifest
in `~/.config/Colony/Eidos/addons/` voegt een tool toe aan de lijst Extensions of
een controle aan het tabblad Health. Er wordt niets in Eidos geladen - een
extensie is een programma dat het uitvoert. Zie
[extensions.md](extensions.md).
