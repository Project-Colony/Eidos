<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Eidos gebruiken

Hoe je Eidos gebruikt: instanties en spelkopieën, de opdrachtregel, testruns, het
venster, Nexus-collecties en updates, een instantie naar een andere machine
verhuizen, tools en extensies. Bij problemen zie
[troubleshooting.md](troubleshooting.md); voor bouwen vanaf de broncode en de indeling
van de repository zie [../internals/contributing.md](../../../../internals/contributing.md).

## Instanties: globaal en draagbaar

Opdrachten die een instantie aannemen accepteren een spel-id of de map van een
draagbare instantie. `skyrimse` benoemt de globale instantie, die Eidos bewaart in
`~/.local/share/Colony/Eidos/instances/skyrimse`. Een draagbare instantie is een op
zichzelf staande map waar je maar wilt (een tweede schijf, een spellenpartitie), en je
kunt haar verplaatsen, zoals de draagbare instanties van MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # daar een draagbare instantie maken
eidos install /mnt/games/EidosSkyrim mod.7z  # elke opdracht neemt de map aan
eidos play /mnt/games/EidosSkyrim -- %command%
```

De `eidos-instance.ini` in de map benoemt het spel, dus meer is er niet nodig.
`EIDOS_INSTANCE=<folder>` in de omgeving leidt een spel-id naar die map om, bijvoorbeeld
in een Steam-opstartoptie. Draagbare instanties die je gemaakt of geopend hebt worden
onthouden, meest recent gebruikte eerst, in `~/.config/Colony/Eidos/instances.ini`: het
welkomscherm van het venster toont ze, de Steam-start landt op de laatst gespeelde, en
de `nxm://`-handler downloadt erin.

Een draagbare map met de hand verplaatsen behoudt alles behalve tool-items die je met
absolute paden naar de oude locatie hebt geregistreerd. `eidos pack` en `eidos unpack`
([hieronder](#een-instantie-naar-een-andere-machine-verhuizen)) herstellen die; een
gewone `mv` niet. De runtime-cache in `~/.local/share/Colony/Eidos/runtimes/` wordt door
elke instantie op de machine gedeeld, zodat een .NET-host van 78 MB niet in elke
instantie wordt gekopieerd.

Eén plek wordt geweigerd: in de installatiemap van een spel, waar MO2-gebruikers hun
instantie vaak neerzetten. Steam bezit die boom, en een update, een "verify integrity" of
een deïnstallatie kan hem herschrijven of verwijderen samen met je opstelling. Eidos
koppelt ook over de spelwortel, dus een instantie daarbinnen zou in haar eigen koppeldoel
zitten. De wizard, `eidos init`, `eidos play` en `eidos shell` weigeren het. Zet de map
naast het spel; een buurmap op dezelfde schijf is even handig.

### Waar Eidos zijn bestanden bewaart

Eidos bewaart zijn eigen bestanden onder `Colony/Eidos`, de indeling die elk programma uit
de Colony-familie gebruikt: `~/.config/Colony/Eidos/` voor wat jij gekozen hebt
(voorkeuren, je Nexus-sessie, je instantielijst, de spel- en add-on-definities die je
geschreven hebt), `~/.local/share/Colony/Eidos/` voor globale instanties (`instances/`),
gedownloade runtimes en sessielogs (`logs/`), en `~/.cache/Colony/Eidos/` voor de
LOOT-masterlists, die opnieuw gedownload worden zodra ze ontbreken.

Vindt Eidos bestanden in de indeling van een oudere Eidos (`~/.config/eidos/`,
`~/.local/share/eidos/` en `~/.local/state/`), dan kopieert de eerste start ze over en
meldt dat in het log. Een globale instantie wordt met hardlinks verplaatst in plaats van
met een kopie byte voor byte, zodat haar mods geen extra ruimte innemen, en haar
tool-items worden naar de nieuwe map omgezet. Er wordt niets in de oude mappen gewijzigd of
verwijderd; haal ze zelf weg zodra de nieuwe indeling werkt. Lukt een kopie niet (een volle
of alleen-lezen schijf, een oude instantiemap op een andere schijf, een spel dat er nog
vanuit draait), dan blijft Eidos de oude locatie gebruiken en probeert het bij de volgende
start opnieuw. Een Steam-opstartoptie die nog de oude instantiemap noemt, opent de
verplaatste.

De hardlinks besparen alleen in het begin ruimte. Een mod of download die je daarna
verwijdert of opnieuw installeert, houdt via de oude instantiemap zijn oude kopie op de
schijf, en `~/.local/share/eidos/<game>/` verwijderen geeft die ruimte terug.

Een draagbare instantie wordt nooit verplaatst: ze blijft waar je haar zet.

## De opdrachtregel

```sh
eidos games                                # hier geïnstalleerde ondersteunde spellen, zoals de lijst van MO2
eidos init skyrimse [folder]               # een instantie maken; met een map is ze draagbaar
eidos init skyrimse --game-path <folder>   # de kopie in <folder> gebruiken als er meerdere geïnstalleerd zijn
eidos install skyrimse mod.7z [name]       # een archief installeren (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # over de mod heen herinstalleren (--merge installeert over zijn bestanden)
eidos import skyrimse <mo2-profile>        # de modvolgorde en pluginstatus van een MO2-profiel overnemen
eidos sort skyrimse [--dry-run]            # de laadvolgorde met LOOT sorteren (--update-masterlist ververst eerst)
eidos play skyrimse                        # tonen wat er gekoppeld zou worden
eidos play skyrimse -- <command>           # <command> draaien met de mods over het spel gekoppeld
eidos shell skyrimse                       # een shell in de samengevoegde weergave; schrijfacties landen in Overwrite
eidos check skyrimse [--json]              # Health-controles zonder het venster
eidos sessions skyrimse                    # de vastgelegde spel- en toolruns, nieuwste eerst
eidos sessions skyrimse diff [<id>]        # wat er veranderd is sinds de laatste run die met code 0 eindigde
eidos stock-game skyrimse                  # welke spelbestanden de instantie draait; een kopie bevriezen of wisselen
eidos tool skyrimse                        # de tools tonen (add, rm en run: zie Tools hieronder)
eidos tool skyrimse run --exe <path>       # een exe eenmalig door de samengevoegde weergave draaien
eidos tool skyrimse import-mo2 [--write]   # de uitvoerbare bestanden van een ModOrganizer.ini; --write voegt ze toe
eidos prereqs skyrimse [--install]         # wat de tools nodig hebben; --install haalt op wat ontbreekt
eidos collection skyrimse <link>           # een Nexus-collectie installeren (--dry-run, --no-optional)
eidos nexus status                         # de opgeslagen Nexus-aanmelding controleren
eidos nexus update skyrimse [--apply]      # controleren op modupdates; --apply installeert ze
eidos nxm <link>                           # een Mod Manager Download-link downloaden
eidos nxm --register                       # de knop Mod Manager Download van de site Eidos laten openen
eidos export skyrimse -o mods.csv --active # het CSV-formaat van MO2; --active toont alleen ingeschakelde mods
eidos pack skyrimse backup.eidos           # de hele instantie in één bestand
eidos unpack backup.eidos [folder]         # zonder map gaat een globale instantie terug naar haar centrale plek
eidos --version                            # de versie tonen
```

`eidos` zonder argumenten toont de lijst met opdrachten. De meeste opdrachten tonen hun
eigen opties wanneer een argument ontbreekt (`eidos init` en `eidos import` tonen in plaats
daarvan de volledige lijst), en `eidos tool --help` toont de subopdrachten van tool.
`eidos pack` neemt `--dry-run`, `--level`, `--no-downloads`, `--no-recovery` en `--force`,
en `eidos unpack` neemt `--info` en `--force`; beide staan
[hieronder](#een-instantie-naar-een-andere-machine-verhuizen) beschreven. `eidos stock-game`
staat beschreven onder
[Spelbestanden](#spelbestanden-de-winkelinstallatie-of-een-kopie).

### Een spel draaien via de samengevoegde weergave

`eidos play` koppelt de mods van de instantie over de eigen `Data`-map van het spel binnen
een privénaamruimte, en draait de opdracht dan door die weergave. De eigen bestanden van het
spel worden eerst via een bind-stash opzijgezet, zodat de daemon nog steeds de ongewijzigde
bestanden leest. Schrijfacties (saves, opnieuw gegenereerde configs) landen in de
`overwrite/`-laag van de instantie; de spelinstallatie en elke modmap blijven byte voor byte
zoals ze waren.

Eidos heeft geen root nodig. Het koppelt in een privé gebruikers- en koppelnaamruimte, zonder
setuid-helper, zonder systeemservice en zonder iets om te verlenen. Een optionele capability
op het `eidos`-binaire bestand staat FUSE-passthrough toe, wat uit blijft tenzij
`EIDOS_FUSE_PASSTHROUGH=1` is ingesteld; zie
[install.md](install.md#optioneel-fuse-passthrough). Met de capability gebruikt Eidos een
gewone koppelnaamruimte in plaats van een gebruikersnaamruimte, en de mods worden op
dezelfde manier uitgerold.

### Een shell in de samengevoegde weergave

`eidos shell <instance>` start je `$SHELL` (of `/bin/sh`) in de spelmap zoals het spel die zou zien: elke ingeschakelde mod over `Data` gekoppeld, Root-mods over de spelroot, en alles wat daar wordt geschreven komt in de Overwrite terecht. `eidos shell <instance> -- <command>` voert in plaats daarvan één commando uit, bijvoorbeeld `eidos shell skyrimse -- ls Data/meshes` of een native tool zoals NifSkope. De run houdt het instantieslot vast zoals een spelsessie, dus het venster kan geen wijzigingen opslaan tot je afsluit, en hij staat in `eidos sessions` onder de naam `shell`, nooit als spelstart. Ctrl+C bij een commando stopt de run zoals Stop: Eidos beëindigt het commando, legt de sessie vast en ontkoppelt. In de interactieve shell blijft Ctrl+C van de shell, zoals gewoonlijk; typ `exit` om de run te beëindigen.

### Gezondheidscontroles vanuit de terminal

`eidos check <instance>` draait de Health-controles die geen venster nodig hebben, zodat ze
via SSH of in Game Mode kunnen draaien en in een bugrapport terechtkomen:

- het Proton-prefix en zijn register;
- plugindiagnostiek en ontbrekende masters;
- de script extender en zijn plugins;
- een spelupdate sinds de laatste schone start;
- de Steam-opstartoptie;
- grafische instellingen (NVAPI, volledig scherm, ENB);
- de crashlog van de laatste run;
- `sTestFile`-regels in de INI's van het profiel;
- Starfield-`.esp`- en blueprintplugins;
- `vm.max_map_count` en de limiet voor open bestanden;
- de spelkopie die de instantie draait, en een `gamePath` in de `ModOrganizer.ini` van een
  lijst die een andere spelmap noemt;
- collectievolgorderegels die niet meer kloppen;
- spelbestanden op NTFS, exFAT of FAT;
- BodySlide-datapaden;
- `Data`-mappen die in `Overwrite/Root` zijn achtergebleven;
- lettertypebibliotheken die `Interface/fontconfig.txt` noemt en die geen mod levert;
- gecomprimeerde audio in de BA2-archieven van Fallout 4- en Fallout 4 VR-mods.

Sommige kaarten berekent alleen het venster, op het tabblad Diagnostics: de patchvolgorde,
extenderscripts die een mod vervangt, archieven die niets laadt, Nexus-vereisten, plugins
die LOOT als vuil markeert, de Proton-build die het spel draait, de Display-kaart en
toolresultaten die ouder zijn dan hun invoer.

De opdracht drukt dezelfde systeemregels af als **Copy report**, daarna één regel per bevinding, met je thuismap getoond als `~` en je inlognaam als `<user>`. De exitcode is 0 als er geen probleem is gevonden (alleen adviezen geven nog steeds 0), 1 als er minstens één probleem is gevonden, en 2 als Eidos niet kon controleren: verkeerde argumenten, een instantie die het niet kan openen, of een spel dat het niet detecteert.

`eidos check <instance> --json` drukt in plaats daarvan één JSON-object af. De velden op het hoogste niveau zijn `eidos` (de versie), `game`, `instance`, `system` (`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`, `max_map_count`, `open_files_hard`) en `findings`. `max_map_count` en `open_files_hard` zijn getallen, of `null` als Eidos ze niet kon lezen. Elke bevinding is een `code`, een `level` (`problem` of `advice`) en zijn `args`; het tabblad Diagnostics toont de zin bij elke code.

### Wat er veranderd is sinds de laatste goede run

Elke run via `eidos play`, `eidos shell` of `eidos tool run`, gestart vanuit het venster, een terminal of Steam, schrijft een record naar `<instance>/logs/sessions/`: het profiel, elke ingeschakelde mod met zijn versie en Nexus-bestands-id's, de pluginvolgorde, een checksum van elke profiel-INI, de versie van het uitvoerbare bestand van het spel, de Eidos-versie, de exitcode en de saves die de run aanmaakte. De 50 nieuwste blijven bewaard, en `logs/` blijft buiten een ingepakte instantie. `eidos sessions <instance> diff` vergelijkt de nieuwste spelrun die met code 0 eindigde met de huidige toestand; `diff <id>` kiest een ander record uit de lijst. De lijst markeert een [testrun](#testruns) als `test`, en een testrun wordt nooit als laatste goede run gekozen. Een run die je stopte legt exitcode 143 vast, dus ook die wordt nooit gekozen. Eidos detecteert hier nog geen crashes, en een spel dat onder Proton crashte kan toch met code 0 eindigen: was de gekozen run er zo een, noem dan met `diff <id>` een eerdere.

## De modlijst

Net als in MO2 heeft elk profiel zijn eigen `modlist.txt` (welke mods ingeschakeld zijn,
en met welke prioriteit), terwijl de modmappen in de gedeelde `mods/` van de instantie
staan. Een map die je met de hand in `mods/` zet, staat nog niet in de lijst: ze verschijnt
uitgeschakeld op de hoogste prioriteit, onderaan de lijst van het venster zoals in MO2,
zodat ze, zodra je haar inschakelt, haar conflicten wint.

Omdat de mappen gedeeld zijn, verandert het verwijderen of hernoemen van een mod elk
profiel: haar regel verdwijnt uit, of wordt hernoemd in, de `modlist.txt` van elk profiel,
en voor de rest houdt elk profiel zijn eigen staat en volgorde. Een profiel dat geen mods
meer heeft, heeft helemaal geen `modlist.txt`.

Het venster werkt vanuit zijn eigen kopie van de modlijst en de laadvolgorde. Schrijft een
ander Eidos-proces een van beide terwijl het open is (`eidos install` of `eidos sort` in
een terminal, een collectie, een sessie die Steam gestart heeft, een tweede venster), dan
wordt de volgende bewerking van het venster geweigerd in plaats van dat werk te
overschrijven. Het laadt opnieuw vanaf schijf en zegt *Not saved: another Eidos process
changed the mod list. It has been reloaded - redo the change.* (of hetzelfde voor de
laadvolgorde); alleen die ene klik gaat verloren, dus doe hem opnieuw. Rename, New mod, Add
separator, Remove en van de Overwrite een mod maken controleren dit eerst, voordat ze aan
`mods/` komen, zodat een geweigerde bewerking niets op schijf verandert. De INI-editor
weigert evenzo op te slaan over een bestand dat veranderd is sinds hij het opende.

## Saves horen bij het profiel

Elk profiel bewaart zijn eigen saves in `profiles/<name>/saves/`, en zolang een run duurt
wordt die map over de savemap van het spel in het Proton-prefix gekoppeld. De eerste start
van een profiel kopieert de saves die al in het prefix staan erin, één keer, zodat een save
die je later uit het profiel verwijdert ook verwijderd blijft. Na elke run worden de
nieuwste saves van het profiel teruggekopieerd naar het prefix voor Steam Cloud; een save in
het prefix die Eidos er niet neergezet heeft, wordt eerst in het profiel bewaard als
`orphan-<time>-<name>`, nooit overschreven.

Een save die na die eerste kopie in het prefix belandt zonder via Eidos te gaan (een Steam
Cloud-download vanaf een Steam Deck, een start zonder Eidos) blijft verborgen zolang de
saves van het profiel gekoppeld zijn. Niets verwijdert hem: de start benoemt hem in het log
van de run (de `logs/run-*.log` van de instantie bij een start vanuit het venster, de
console van Steam bij een start via Steam), één keer telkens wanneer die verzameling
bestanden verandert, en het tabblad Saves telt deze saves op de regel onder zijn kop. Om er
een te spelen druk je op Import in het tabblad Saves. Import kopieert elke save met zijn
co-save naar het profiel en vervangt nooit een save van het profiel: een save waarvan de
naam het profiel al gebruikt, wordt geïmporteerd als `orphan-<time>-<name>`.

## Testruns

De knop Test run naast Run start het spel zoals Run dat doet, maar alles wat het spel schrijft gaat naar `<instance>/test-run/` in plaats van naar je profiel, de Overwrite en het prefix: nieuwe en gewijzigde saves, INI-wijzigingen, `plugins.txt`, bestanden die in Data of de spelmap worden geschreven, en de logs in `Documents/My Games/<game>`. Je kunt testen met de mods van het profiel, zonder enige mod, of alleen met de geselecteerde mods die zijn ingeschakeld, plus de mods waarvan ze de plugins als master nodig hebben.

Als het spel stopt, toont Eidos die bestanden in vijf groepen (saves, INI-wijzigingen, pluginstatus, Overwrite-uitvoer, logs) en vraagt per groep of je die houdt of weggooit. Apply wacht tot elke groep die bestanden bevat een antwoord heeft. Pluginstatus wordt niet aangeboden wanneer de run een andere set mods gebruikte dan die van het profiel. Saves houden verwijdert nooit een save uit het profiel, ook niet een die het spel tijdens de run verwijderde. De Overwrite-uitvoer houden verwijdert ook de bestanden uit de Overwrite die het spel verwijderde, en de lijst telt ze eerst. Iets weggooien vraagt een tweede klik. Decide later sluit de lijst en laat de bestanden in `test-run/`, en de lijst opent weer de volgende keer dat je de instantie opent of op F5 drukt. Stopt Eidos of de computer tijdens een testrun, dan blijven de bestanden ook daar en opent de lijst op dezelfde manier.

Is het venster niet door de winkel van het spel gestart, dan geeft Test run het spel door aan die winkel (Steam, Heroic of Legendary), zoals Run doet, en de run die de winkel start is de testrun. Start de winkel het spel niet, dan blijft het verzoek wachten: de eerstvolgende spelrun van dat profiel, vanuit Run of vanuit de winkel, is de testrun, en Cancel the waiting test run laat hem vallen. Een testrun dekt alleen de mappen hierboven: andere schrijfacties van het spel, zoals in het Wine-register, gaan waar ze altijd heen gaan. Saves uit een testrun komen pas in Steam Cloud nadat je ze houdt en een normale run speelt.

### De mod achter een probleem vinden

De knop Find the mod behind a problem in Diagnostics zoekt de mod die een probleem veroorzaakt dat je kunt reproduceren, zoals een crash op een bepaalde plek. Elke stap is een testrun met de helft van de overgebleven verdachten, de masters die ze nodig hebben en de mods die je altijd aan houdt: de ingeschakelde mods die bij de start in de lijst geselecteerd zijn, en de mod die de script extender levert. Na elke run antwoord je of het probleem optrad. Not sure verdeelt de verdachten anders en Run it again herhaalt de stap. Een antwoord, of Run it again, gooit de bestanden die de stap schreef weg na een tweede klik. Wil je er een paar houden, druk dan op See the files voordat je antwoordt: dat opent dezelfde lijst als na een testrun. Crashte het spel en schreef een crash logger een log, of eindigde het spel met een foutcode, dan markeert Eidos Yes als waarschijnlijk antwoord. 200 verdachten kosten ongeveer 8 runs.

De zoektocht wordt bewaard in `<instance>/bisect.json`, dus je kunt Eidos tussen de stappen sluiten, en ze hoort bij het profiel waarin ze begon. Stop the search laat hem vallen na een tweede klik. Het resultaat noemt de mod, of de groep mods die elkaars masters nodig hebben, en wordt bewaard in `<instance>/logs/bisect-<time>.txt`. Gegenereerde mods (uitvoer van Nemesis of DynDOLOD, een Bashed Patch) zijn gebouwd voor de volledige lijst en kunnen met de helft ervan mislukken: houd ze altijd aan, of laat ze de hele zoektocht uit.

## Wabbajack-lijsten

Eidos heeft geen Wabbajack-installatieprogramma. Installeer de lijst met CLF3 of Jackify, buiten Eidos, en kies de installatiemap in de wizard als draagbare instantie. Eidos leest de map in de indeling van MO2: `mods/`, `profiles/<naam>/modlist.txt` en de pluginlijsten. Het `ModOrganizer.ini` dat het installatieprogramma schreef blijft staan en wordt nooit overschreven, zodat de eigen programma's van de lijst geregistreerd blijven.

Veel lijsten voegen een map `Stock Game` of `Game Root` toe met een aangepaste kopie van het spel, en MO2 start het spel vandaar. Neemt de wizard of `eidos init` een lijstmap voor het eerst over en bevat die map het uitvoerbare bestand van het spel, dan start ook de instantie het spel vanuit die map. Het spel start nog steeds via Steam of de winkel die het bezit, dus hoeft er niets verplaatst te worden. Eidos vindt de map via de `gamePath` in de `ModOrganizer.ini` van de lijst, of aan die twee namen. Een instantie die met een eerdere Eidos is overgenomen, houdt de winkelinstallatie tot je overschakelt (zie de volgende sectie). Noemt die `gamePath` een spelmap die de instantie niet draait, dan zegt Diagnostics dat, omdat tools in de modus Mod Organizer 2, zoals PGPatcher, het spel van daar lezen. CLF3 bouwt BSA- en DDS-bestanden opnieuw op met eigen hulpmiddelen, dus het resultaat kan afwijken van een Wabbajack-installatie, en lijstauteurs ondersteunen het niet.

## Spelbestanden: de winkelinstallatie of een kopie

Standaard draait het spel vanuit zijn winkelinstallatie, de map waarin Steam, Heroic of Legendary het bewaart. Een instantie kan in plaats daarvan draaien vanuit een kopie van die map in de instantie. De mods liggen boven op de kopie, en de winkelinstallatie is alleen nog de plek waar Eidos het resultaat mount. De winkel start het spel nog steeds, dus Steam houdt zijn overlay, zijn shadercache en zijn cloudsaves.

De kaart Game files bovenaan het tabblad Diagnostics toont welke bestanden de instantie draait, met de spelversie van de kopie en van de winkelinstallatie, en wisselt tussen beide. In een terminal drukt `eidos stock-game <instance>` dezelfde feiten af, en `use <folder>`, `store`, `freeze` en `delete --yes` doen wat de knoppen van de kaart doen. `freeze` drukt de grootte en de soort kopie af, en `freeze --yes` kopieert.

**Freeze this version** kopieert de winkelinstallatie naar `<instance>/Stock Game` en start het spel daarvandaan, zodat een winkelupdate geen script-extender-plugins meer breekt: de update wijzigt de winkelinstallatie, niet de kopie. De eerste druk toont de grootte en wat het bestandssysteem ermee doet, de tweede kopieert. Op een bestandssysteem dat blokken kan delen (reflink), zoals btrfs of XFS, neemt de kopie bijna geen ruimte in tot de winkel een bestand wijzigt. Op ext4, of als het spel en de instantie op verschillende bestandssystemen staan, is het een volledige kopie, even groot als de spelmap. Eidos weigert zolang Steam het spel als bezig of niet volledig geïnstalleerd meldt, zolang een spel of tool van de instantie draait, wanneer de instantie al de spelkopie van een lijst, een bevroren kopie of een onvoltooide kopie bevat, en wanneer het bestandssysteem minder vrije ruimte heeft dan een volledige kopie nodig heeft. Kopieën van Heroic en Legendary worden niet gecontroleerd op een lopende update. Stop, een fout, of een Steam-update van het spel tijdens het kopiëren laat `Stock Game.eidos-tmp` achter, dat de kaart aanbiedt te verwijderen; die map wordt nooit gebruikt.

Een kopie verbergt elk bestand dat de winkel na het kopiëren in de spelmap toevoegt, wijzigt of verwijdert: nieuwe Creation Club-bestanden, een nieuw uitvoerbaar bestand, nieuwe DLL's. Het spel, de pluginlijst, LOOT en de Health-controles lezen allemaal de kopie. Wat buiten de spelmap gebeurt, verbergt ze niet: een update kan bij de volgende start nog steeds runtimes in het Proton-prefix installeren, en Steam kan het spel naar een andere Proton overzetten.

**Return to the store version** haalt de kopie na een tweede klik uit de volgende start. De kopie blijft op schijf staan, en **Delete the copy** verwijdert haar, opnieuw na een tweede klik. Delete wordt geweigerd zolang de instantie de kopie nog draait. Eidos verwijdert nooit de eigen map `Stock Game` of `Game Root` van een lijst. Verdwijnt de map van de kopie, of bevat ze het uitvoerbare bestand van het spel niet meer, dan worden starts geweigerd en noemt Diagnostics de map, zodat de winkelversie nooit onverwacht draait.

Morrowind, en Oblivion met `bUseMyGamesDirectory=0`, houden hun pluginlijst in de spelmap, en Eidos start ze nog niet vanuit een kopie.

## Een Nexus-collectie installeren

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # haar lezen en zeggen wat er zou gebeuren
eidos collection skyrimse rqhcxy --no-optional  # de leden overslaan die ze als optioneel markeert
```

In het venster plak je de link in **File -> Open a Nexus collection...**, of klik je er een
aan op de site. Eidos toont de leden van de revisie, elk vergeleken met deze instantie:
geïnstalleerd, gedownload of ontbrekend. *Install this collection* draait het hele recept op
een werkthread, met de voortgangsbalk die de pack- en unpack-taken gebruiken, en de balk
wordt het verslag wanneer ze klaar is. Het venster en `eidos collection` draaien dezelfde
installatie.

Zonder link opent **File -> Open a Nexus collection...** op het tabblad **Browse**. Het toont
de collecties die voor het spel van de geopende instantie zijn gepubliceerd, 20 per keer,
gesorteerd op meest gedownload, recent bijgewerkt of nieuwste, en het zoekvak beperkt de
lijst tot collecties waarvan de naam of tekst overeenkomt. Elke rij geeft de auteur, de
nieuwste revisie, het aantal mods, de downloads en een datum. **View** opent de nieuwste
revisie van die collectie op het tabblad **Link**, net als het plakken van haar link.
Collecties die als volwassen zijn gemarkeerd worden weggelaten tenzij je Nexus-account
volwassen inhoud toont, collecties zonder beoordeling worden altijd weggelaten, en de regel
onder de lijst telt beide. Bladeren vereist een aangemeld Nexus-account, zoals de rest van
het paneel.

Een collectie opent alleen bij een instantie van haar eigen spel. Een Skyrim-collectie die
wordt geopend terwijl een Fallout 4-instantie geladen is, wordt bij naam geweigerd, in plaats
van haar leden te vergelijken met de verkeerde modlijst.

### Wat Eidos uit een collectie leest

Bijna niets van een collectie is zichtbaar via de Nexus-API. De installatievolgorde, de
antwoorden die de auteur aan de gescripte installer van elke mod gaf, welke mod een
bestandsconflict wint, welke plugins waar laden en de binaire patches staan allemaal in een
`collection.json` binnen het eigen archief van de collectie. Dezelfde mods downloaden en ze
met hun standaardopties installeren levert een ander spel op.

Eidos downloadt het archief, leest `collection.json` en past het toe:

| De collectie zegt | Eidos doet |
| --- | --- |
| `phase` | installeert in die volgorde, optionele leden als laatste |
| `choices` | herspeelt de antwoorden van de auteur op elke FOMOD, en zegt het wanneer dat niet lukt |
| `modRules` | herordent de modlijst zodat de juiste mod elk bestand wint |
| `plugins` / `pluginRules` | voegt de LOOT-regels samen met je userlist en sorteert |
| `INI Tweaks/` | installeert ze als een mod op zichzelf |
| `tools` | vertelt je welke tools het verwacht; maakt er geen aan |

Leden worden ingeschakeld zodra ze installeren, achteraan de modlijst in installatievolgorde,
voordat `modRules` ze verplaatst. Een lid dat niet in de modlijst staat zou niet laden, en het
verslag zou het niet tonen.

De staat wordt na elk lid weggeschreven naar `<instance>/collections/<slug>-<revision>.state.json`.
Het bestand staat naast de map van die revisie, nooit erin, zodat een collectie geen eigen
staatsbestand kan meeleveren, en een installatie die bij lid 147 van 200 onderbroken is, gaat
vanaf 147 verder. Het wordt onder een tijdelijke naam geschreven en op zijn plaats hernoemd,
zodat een onderbreking de oude staat achterlaat of de nieuwe en nooit de helft van een van
beide. Als het ooit niet gelezen kan worden, stopt Eidos en zegt het, in plaats van de hele
collectie opnieuw te beginnen. Draai dezelfde opdracht opnieuw.

Een nieuwere revisie installeren van een collectie die je al hebt, neemt de mappen van de
vorige revisie over: een ongewijzigd lid wordt ter plekke geverifieerd, een lid waarvan de
auteur het bestand bijgewerkt heeft wordt binnen zijn oude map vervangen, en een lid dat de
nieuwe revisie geschrapt heeft blijft geïnstalleerd en wordt in het verslag benoemd, zodat jij
het kunt uitschakelen of verwijderen. Een map die een ander profiel inschakelt, wordt nooit
overgenomen, want de mods-map is gedeeld en dat profiel zou buiten je om veranderen: de nieuwe
revisie installeert dan haar eigen kopie ernaast, en het verslag vraagt je de oude in dit
profiel uit te schakelen, nooit om haar te verwijderen. Teruggaan naar een oudere revisie neemt
haar mappen op dezelfde manier terug, of installeert verse kopieën van de mappen die ze niet
kan overnemen. Telkens als een lid zo vervangen wordt terwijl je in zijn map bestanden
gewijzigd of toegevoegd had, wordt de oude map eerst ernaast bewaard als `<name>_backup`, die
nooit in het spel geladen wordt, en het verslag noemt hem.

Een overblijfsel van een eerdere revisie dat ingeschakeld blijft, zorgt ervoor dat de
installatie niet telt als die welke de auteur gebouwd heeft; de melding verdwijnt zodra je het
uitschakelt. Een lid dat je met `--no-optional` overgeslagen hebt, wordt niet overgenomen en
wordt op dezelfde manier benoemd. Mislukt de overname zelf, dan stopt de run met *previous
revision: its folders could not be taken over* en de reden; los dat op en draai de opdracht
opnieuw.

### Gratis accounts en leden die je zelf ophaalt

Een gratis Nexus-account kan de mods van de leden niet ophalen. Nexus geeft gratis accounts
via de API geen downloadlinks: elk bestand vergt de eigen knop "Mod Manager Download" van de
site. Het archief van de collectie downloadt wel op een gratis account, dus je kunt het hele
recept lezen, maar elk lid wordt gemeld met zijn exacte pagina-URL om het zelf op te halen, en
opnieuw proberen verandert dat niet. Haal ze op via de knop van de site en draai de opdracht
opnieuw: er wordt bij elke run in `downloads/` naar die leden gezocht, zodat de collectie
afgemaakt wordt naarmate jij ze aanlevert.

In het venster krijgt een gratis account in het collectiepaneel *Fetch the N missing required
files, one by one* in plaats van het ophalen in partijen. Het opent op Nexus de pagina van het
eerste ontbrekende verplichte bestand, wacht tot dat bestand in `downloads/` aankomt en opent
dan het volgende. De downloadknop van de pagina druk je nog steeds zelf in, voor elk bestand.
Het paneel toont hoe ver het is (*3 of 12*), met *Skip* en *Stop*. Komt er in plaats daarvan
een ander bestand van dezelfde mod binnen, dan noemt het paneel beide bestands-id's en blijft
het wachten op het bestand dat de collectie vastlegt. Optionele leden doen niet mee en houden
hun eigen knop *Open*.

Eidos past ook binaire patches toe (elk gecontroleerd tegen de vastgelegde CRC), bestandsoverrides
(de genoemde bestanden worden met `.mohidden` verborgen, zoals MO2 doet) en leden waarvan de
bestanden binnen het collectiearchief meereizen (`bundle`). Leden die de collectie je met de
hand laat ophalen (`browse`, `manual`) dragen de eigen instructies van de auteur door tot in het
verslag. Een lid waarvan de naam al een mod van jou is, wordt ernaast geïnstalleerd onder een
vrije naam, nooit eroverheen, en het verslag zegt welke.

### Het verslag

Een lid waarvan niet alle installer-antwoorden herspeeld konden worden krijgt een eigen regel,
*installed, but not the way the collection asks*, apart van de leden die geïnstalleerd zijn
zoals de auteur ze bouwde, omdat de bestanden op schijf verschillen van die van de auteur. Het
verslag noemt ook ordeningsregels die niets in de collectie benoemden, mods gevangen in een lus
van tegenstrijdige regels, LOOT-groepen die niet bestaan, en elk deel van het manifest dat deze
versie van Eidos niet begrijpt.

### De collectievolgorde bewaren

Eidos bewaart de regels van elke geïnstalleerde collectie na de installatie. Als een latere sleepactie twee van haar mods tegen een volgorderegel (voor of na) in plaatst, toont het tabblad Diagnostics één kaart voor die collectie, die het eerste zulke paar noemt en de regels telt die niet meer kloppen. Alleen mods die in het huidige profiel zijn ingeschakeld tellen mee, en regels die in een lus zitten worden overgeslagen, omdat geen volgorde ze kan naleven. **Restore collection order** zet de mods van de collectie terug in een volgorde die elke regel naleeft, houdt je eigen volgorde aan waar de regels dat toelaten en laat de mods die niet bij de collectie horen op hun plaats. Ctrl+Z maakt het ongedaan. `eidos check` meldt dezelfde bevinding als `collection_order_broken`. De controle heeft het recept van de revisie in `collections/` nodig, dus een collectie waarvan het recept uit de cache is verwijderd, wordt niet gecontroleerd.

## Een instantie naar een andere machine verhuizen

`eidos pack` schrijft een hele instantie (elke mod, de laadvolgorde, alle profielen, de
Overwrite met je saves erin, en de archieven waaruit alles geïnstalleerd is) weg naar één
enkel bestand. `eidos unpack` zet ze terug:

```sh
eidos pack skyrimse ~/backup.eidos                  # of geef een map op: genoemd naar het spel en de datum
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # op de andere machine
```

`eidos pack --dry-run` somt precies op wat erin zou gaan, wat niet en waarom, en hoeveel
ruimte het nodig heeft, zonder iets te schrijven. `eidos unpack --info` doet hetzelfde voor
een bestand dat je al hebt.

Als 7-Zip iets niet kon lezen, of als er bij het doorlopen iets overgeslagen moest worden wat
je in een back-up zou verwachten (een symbolische link, een map die niet gelezen kon worden,
een naam die niet veilig aan 7-Zip meegegeven kan worden), wordt het archief alsnog onder de
door jou gekozen naam geschreven, maar `eidos pack` sluit af met 1 en zegt wat er ontbreekt,
zodat een script dat `eidos pack ... && ...` draait daar stopt.

Een `.eidos`-bestand is een 7-Zip-archief onder een eigen extensie. 7-Zip is toch al nodig om
een mod te installeren, dus dit voegt geen afhankelijkheid toe, en de back-up opent zonder
Eidos in elk archiefprogramma dat 7-Zip leest. Het wordt gecomprimeerd met LZMA2 op `-mx1`,
tot ongeveer 43% van het origineel bij een textuurzware lijst; `-mx9` duurt veel langer en
wint een paar punten meer. Texturen en meshes zijn al gecomprimeerde formaten, dus een groter
woordenboek vindt er heel weinig meer te comprimeren. `--level` (0, 1, 3, 5, 7 of 9) kiest een
ander niveau, en `--no-downloads` laat `downloads/` weg.

Op het standaardniveau 1 gaan de bestanden in solide blokken van hoogstens 16 MB. Eén bestand
uit een back-up van 70 GB halen decodeert hoogstens het blok waarin het staat, nooit alles wat
ervoor staat, en het packen van game-inhoud gaat 2 tot 5 keer sneller dan met elk bestand apart
gecomprimeerd (uitpakken 2 tot 4 keer). De prijs betaal je bij schade: één corrupte byte kan
elk bestand in zijn blok meenemen, tot 16 MB aan naburige bestanden. De herstelgegevens
hieronder dekken dat af. Alle andere niveaus blijven niet-solide: elk bestand is een eigen
stroom, en een corrupte byte kost alleen dat ene bestand. Niveau 0 slaat de bestanden op. Vanaf
niveau 3 werkt LZMA2 al in stukken van 16 MB of meer, en daar waren solide blokken bij het
meten trager: op niveau 5 duurde het packen van meshes 1,4 keer zo lang en het uitpakken van
texturen 6,8 keer zo lang.

Packen neemt het slot van de instantie, dus het kan niet draaien terwijl het spel of een andere
Eidos naar die instantie schrijft, en niets verandert de instantie terwijl ze gelezen wordt. De
back-up wordt onder een `.part`-naam geschreven, naar de schijf weggeschreven en met `7z t`
volledig teruggelezen voordat ze de naam krijgt die je gekozen hebt. Die controle decodeert elk
blok en vergelijkt elke checksum, hetzelfde werk dat uitpakken op de andere machine doet. Ze laat
zien wat de schijf direct na het schrijven teruggaf, niet dat de schijf het bewaart; daarvoor zijn
de herstelgegevens hieronder. Mislukt ze, dan wordt de `.part` verwijderd, wordt er niets onder
jouw naam geschreven en citeert `eidos pack` de regel waarin 7-Zip de schade vond. Een onderbroken
of mislukte pack laat een `.part`-bestand achter of niets, nooit een `.eidos`-bestand dat compleet
lijkt en het niet is.

Als par2cmdline geïnstalleerd is (`sudo pacman -S par2cmdline` op Arch, `sudo apt install par2` op
Debian en Ubuntu, `sudo dnf install par2cmdline` op Fedora), schrijft `eidos pack` ook
PAR2-herstelgegevens naast het archief: `backup.eidos.par2` en een paar
`backup.eidos.volNN+MM.par2`-bestanden, samen 5% van het archief plus een kleine index. Houd ze in
dezelfde map als het `.eidos`-bestand, onder de namen waarmee ze geschreven zijn, en kopieer ze
mee: een back-up die zonder ze verhuist, heeft niets om uit hersteld te worden, en de
herstelgegevens leggen de naam van het archief vast, dus een hernoemde back-up past er niet meer
bij. `eidos unpack` vindt ze op naam en controleert het archief ermee voordat er iets uitgepakt
wordt, ook een back-up waarvan het einde zo is afgekapt dat 7-Zip hem niet kan openen. Zijn er een
paar bytes kapot, dan herstelt par2 eerst het archief en houdt de beschadigde kopie als
`backup.eidos.1`, waarvoor in de map evenveel vrije ruimte nodig is als het archief groot is;
schade die verder gaat dan die 5% dekt, stopt het uitpakken voordat er één bestand geschreven is.
Herstelbestanden die niet bij het archief passen, zoals die van een oudere back-up met dezelfde
naam, worden niet gebruikt als 7-Zip het archief leest en het intact vindt, en het uitpakken meldt
dat. Op een machine zonder par2 gaat het uitpakken toch door, met een waarschuwing dat de
herstelgegevens niet gebruikt zijn. `--no-recovery` packt zonder, en `eidos pack --dry-run` zegt of
ze geschreven worden. `--force` vervangt een bestaand bestand met dezelfde naam.

### Wat er niet in zit, en waarom

| Weggelaten | Waarom |
| --- | --- |
| Het Proton-prefix | Het is naar de machine gevormd (schijftoewijzingen, een `Z:`-kijk op *dit* bestandssysteem) en Eidos bouwt het in een minuut opnieuw op met `eidos prereqs <instance> --install`. Het meenemen zou het archief ruwweg verdubbelen om iets te versturen dat de andere machine toch opnieuw moet aanmaken. |
| Het spel | Een instantie modt een spel dat ze niet bevat. Installeer het daarginds vanaf Steam. |
| Tools buiten de instantie | xEdit, DynDOLOD en consorten zijn programma's van derden met hun eigen installatieprogramma's. Ze worden in het archief wel benoemd, dus bij het uitpakken hoor je precies welke er op de nieuwe machine ontbreken. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Verslagen van de machine die de back-up gemaakt heeft. `prereqs.done` zou in het bijzonder beweren dat de runtime-bibliotheken al geïnstalleerd zijn in een prefix dat er niet is. |
| `loot/`, behalve `userlist.yaml` | Een cache van de masterlist die Eidos op verzoek opnieuw ophaalt. Je eigen LOOT-regels in `userlist.yaml` kunnen niet opnieuw opgehaald worden, dus dat ene bestand gaat mee. |
| `.base/`, `.base-root/` | Lege koppelpunten waar de eigen bestanden van het spel tijdens een sessie gestald worden. |
| `test-run/`, `bisect.json` | Een [testrun](#testruns) die op zijn beoordeling wacht, en een zoektocht naar de mod achter een probleem tussen twee stappen. Beide horen bij runs op deze machine. |
| Halfgeschreven bestanden | Een gepauzeerde download (`*.unfinished`), een atomaire schrijfactie onderweg (`*.eidos-tmp*`). |
| Symbolische links | 7-Zip zou er een volgen en kopiëren waar hij naar wijst, wat bij een absolute link betekent dat er een vreemde boom je back-up in getrokken wordt. Ze worden in plaats daarvan gemeld, en de back-up telt als onvolledig: een `mods/` of `downloads/` die naar een andere schijf gelinkt is, wordt niet ingepakt. Wil je bewust een back-up maken zonder een gelinkte `downloads/`, geef dan `--no-downloads` mee (of vink *Include downloads/* uit in het venster); een bewuste uitsluiting telt niet als ontbrekend. |

Een spelkopie in de instantie (`Stock Game`) wordt ingepakt zoals elke andere map, dus de back-up van een bevroren instantie is ongeveer de omvang van het spel groter. Een onvoltooide kopie (`Stock Game.eidos-tmp`) blijft erbuiten.

Elk van deze staat met zijn reden in `eidos-backup.ini` in de wortel van het archief, dus dat
bestand lezen laat zien wat er is weggelaten. Datzelfde bestand noteert waar de instantie vroeger
stond, welk profiel actief was, hoe groot ze uitgepakt wordt, en welke tools de nieuwe machine
nodig zal hebben.

Downloadverslagen (`downloads/*.meta`) gaan mee met hun `url=`-regel leeggemaakt. Die URL is een
ondertekende Nexus-link die binnen enkele uren ophoudt te werken en het `user_id` draagt van het
account dat het bestand gedownload heeft, en een back-up kan aan iemand anders gegeven worden. Het
mod-id, het bestands-id en de versie blijven, zodat de download terug te vinden is.

### In het venster

Beide staan in het menu **File**: *Pack this instance...* en *Unpack a backup...*. Uitpakken staat
ook op het welkomscherm, onder de lijst met instanties, zodat een machine zonder instantie er een
kan uitpakken.

Voordat er iets geschreven wordt, toont de Pack-dialoog hoeveel bestanden, hoe groot, hoeveel
daarvan `downloads/` is, ruwweg waar het archief op uitkomt, en welke tools wel benoemd maar niet
meegenomen worden. Het vakje *Write recovery data* is aangevinkt als par2 geïnstalleerd is, en grijs
met het te installeren pakket als dat niet zo is. De Unpack-dialoog leest het manifest van de
back-up zodra je het bestand kiest, zodat ze je kan vertellen welk spel erin zit, wanneer ze gemaakt
is, waar ze vroeger stond en welke van haar tools hier ontbreken, voordat je een map kiest.

Beide draaien op een werkthread met een voortgangsbalk, en de balk wordt het verslag zodra ze klaar
zijn.

### Wat uitpakken repareert

Een gewone kopie van de map zou elke toolknop laten wijzen naar een pad op de oude machine.
`eidos unpack` herschrijft de waarden die de oude instantiewortel benoemen en alleen die: `exe`,
`workdir` en de genummerde `arg`-sleutels in `tools.ini`, en `installationFile` in de `meta.ini` van
elke mod. Een tool die nooit binnen de instantie zat (`/opt/xedit/SSEEdit.exe`) blijft precies zoals
hij was en wordt vermeld als iets om opnieuw te installeren; Eidos raadt niet waar die tool op de
nieuwe machine staat. Het corrigeert ook of de instantie zichzelf centraal of draagbaar noemt, zodat
latere opdrachten haar zoeken waar ze nu staat.

Uitpakken weigert een map die niet leeg is (geef `--force` mee als je er echt overheen wilt), een
schijf die te klein is voor het cijfer van het manifest zelf, een archief gemaakt door een nieuwere
Eidos dan de jouwe, en elk archief met een item dat *buiten* de door jou gekozen map geschreven zou
worden. Zonder map gaat een back-up van een globale instantie terug naar de centrale plek voor haar
spel; een draagbare heeft een map nodig.

Daarna:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # het Proton-prefix opnieuw opbouwen
eidos play /mnt/games/EidosSkyrim
```

## Het venster

Start `eidos-gui`, of het bestand `eidos-linux` dat Colony installeert.

De eerste start opent een wizard zoals die van MO2: welkom, instantietype (draagbaar of
globaal), spel, naam en locatie, samenvatting, daarna het hoofdvenster. Het welkomscherm toont
ook elke bekende instantie, globaal en draagbaar, laatst gebruikte eerst, om met één klik te
openen, dus het dient meteen als instantiewisselaar. De wizard op een map richten die al een
instantie bevat opent die instantie zoals ze is in plaats van er een nieuwe overheen te maken,
en een map die bij een ander spel hoort wordt geweigerd.

Het hoofdvenster heeft twee panelen: de modlijst, en de tabbladen Data, Plugins, Conflicts,
Overwrite, Archives, Saves, Downloads en Diagnostics, met een Run-knop, een Test run-knop
([Testruns](#testruns)) en een keuzelijst voor het doel. Je filtert de modlijst, selecteert,
herordent, groepeert met scheidingen, versmalt per categorie en rechtsklikt voor acties. Naast
naar boven en naar onder sturen heeft het modmenu de gerichte verplaatsingen van MO2: boven de
eerste botsende mod, onder de laatste, naar een gegeven prioriteit, of in de groep van een
scheiding.

De profielkiezer wisselt van profiel. Klik met de rechtermuisknop op een profiel voor Rename,
Copy to new..., Desktop shortcut, Import from MO2... en delete, wat twee klikken vraagt en het
actieve of het laatste profiel weigert. Import from MO2... neemt de modvolgorde en laadvolgorde van
een MO2-profiel over. Desktop shortcut schrijft een desktopitem dat
`eidos tool <instance> run "<game>" --profile <name>` draait: het maakt dat profiel het actieve en
vraagt de winkel het spel te starten (details in
[tools.md](tools.md#bij-een-tool-komen-verbergen-vastzetten-en-een-bureaubladsnelkoppeling)).

Het menu **File** draait ook winecfg, regedit, de Wine-bestandsverkenner en het verwijderprogramma
in het prefix van het spel, en Prefix drive_c opent het station `C:` van het prefix; zie
[tools.md](tools.md#de-eigen-programmas-van-wine-in-het-prefix-van-het-spel). *Instances...* toont de
instanties die Eidos kent; Forget haalt er een van die lijst en raakt niets op schijf aan. *Export the
mod list...* schrijft de lijst in het CSV-formaat van MO2, zoals `eidos export` doet.

### Preferences

Klik linksboven op **Eidos** om Preferences te openen in plaats van de twee panelen, en klik er
nogmaals op (of op **Close**) om terug te gaan. General, Appearance en Accessibility komen eerst,
zoals in elk Colony-programma, daarna Mod list, Nexus en About. Elke wijziging wordt meteen
toegepast en opgeslagen; er is geen Save-knop.

- General: het spel dat Eidos opent, de venstergrootte, de taal van de interface, de tools folder,
  en hoe een spel gestart wordt. Lock the window while a game runs staat standaard aan. Start the
  game at once when a store opens Eidos, en de desktopmelding die zegt wanneer een lange taak
  eindigt terwijl het venster op de achtergrond staat, staan standaard uit. Het venster spreekt
  Engels en Frans; System volgt `LC_ALL`, `LC_MESSAGES` of `LANG`, en de wijziging gaat meteen in.
  De opdrachtregel `eidos` antwoordt altijd in het Engels.
- Appearance: het thema (26 families, 59 paletten, standaard het perkament van Eidos), de
  accentkleur, de tekstgrootte, effecten en een live voorvertoning.
- Accessibility: hoog contrast, een lettertype dat prettig is bij dyslexie, minder beweging en een
  leestekstgrootte die vermenigvuldigd wordt met die van Appearance.
- Mod list: de scrollsnelheid wanneer een mod naar de boven- of onderrand van de lijst wordt gesleept.
- Nexus: aanmelden, de downloadmap, de `nxm://`-handler en zijn knop Register, voorkeursservers,
  Install downloads when they finish, en Offline mode.
- About: waar de bestanden staan, de sneltoetsen, de versie en de releasecontrole.

Het uiterlijk en de taal van de interface staan in `~/.config/Colony/Eidos/preferences.json`. De
andere instellingen, die ook de opdrachtregel leest (het standaardspel, de tools folder, de
Nexus-servers, Offline mode en de rest), staan in `settings.ini` ernaast, en de Nexus-sessie in
`nexus.ini`. Het thema dat een oudere Eidos in `settings.ini` bewaarde, wordt de eerste keer
overgenomen.

**About -> Version** bevat de releasecontrole, standaard aan. Bij het starten vraagt Eidos GitHub hoogstens één keer per dag naar de nieuwste release van `Project-Colony/Eidos`. Is die nieuwer dan de draaiende kopie, dan toont de menubalk na Help `Eidos <version> is available: release notes`, en de link opent de pagina van die release met de changelog. Eidos downloadt of vervangt zichzelf nooit. In offlinemodus wordt de controle overgeslagen. Ze staat uit voor een kopie die een pakketbeheerder onder `/usr` installeerde, want die werkt hem bij; `/usr/local`, waar `install.sh --system` Eidos neerzet, telt als je eigen installatie.

### Kolommen, sorteren en groeperen

De lijst toont standaard vier kolommen (Category, Content, Version en Flags, de kolommen die MO2
toont) en biedt er acht: Category, Content, Version, Author, Installed, Nexus id, Game, Flags. Vink ze
aan in het View-menu. Met alle acht aan blijft er weinig ruimte over voor de naamkolom.

Klik op een kop om erop te sorteren, nog eens om om te keren, en een derde keer om terug te keren naar
de laadvolgorde. De lijst kan alleen in laadvolgorde gesleept worden: een invoegpositie spreekt de
echte lijst aan, en onder een sortering is de plaats van een rij op het scherm niet haar plaats in die
lijst. Zolang een sortering aan staat worden de invoegstroken niet getekend en wordt een sleep
geweigerd, zoals in MO2. Het View-menu zegt dat en biedt de weg terug.

Het View-menu kan de hele lijst ook groeperen, per categorie of per bron (van Nexus, of met de hand
geïnstalleerd). Groepskoppen zijn geen scheidingen: ze kunnen niet hernoemd, gekleurd of verplaatst
worden. Ze klappen in, en een ingeklapte kop behoudt zijn aantal. Scheidingen zijn verborgen zolang
een sortering of groepering aan staat, omdat een scheiding de rijen aanvoert die in de laadvolgorde op
haar volgen, en beide die volgorde op het scherm veranderen.

### Muis en toetsenbord

Dubbelklik op een mod voor Information, Ctrl+dubbelklik voor haar map, Shift+dubbelklik voor haar
Nexus-pagina. Ctrl+F zet de cursor in het filtervak. Een letter typen springt naar de volgende mod die
ermee begint, en nog eens drukken loopt de rest af in plaats van op de eerste te blijven hangen. Geen
ervan landt op een rij die het filter, een ingeklapte scheiding of een ingeklapte groep verbergt, dus
de volgende spatie schakelt nooit een mod om die je niet ziet.

"Collapse others" in het menu van een scheiding klapt elke groep in behalve die ene. Tijdens een sleep
opent een ingeklapte groep als je de aanwijzer erboven stil houdt, zodat de mod erin gelaten kan worden;
eroverheen bewegen opent haar niet.

Ctrl+Z maakt de laatste wijziging in de modlijst of de laadvolgorde ongedaan: een vinkje, een
verplaatsing, een batchgewijze uitschakeling, een LOOT-sortering. Ctrl+Shift+Z voert die opnieuw uit. Het
venster bewaart 50 stappen en vergeet ze bij F5, aan het eind van een spel- of toolrun, bij een
profielwissel en wanneer een ander Eidos-proces de lijst wijzigt. Een mod installeren, hernoemen of
verwijderen verandert de mappen op schijf, dus ongedaan maken voorbij zo'n stap wordt geweigerd.

### De lijst filteren

De knop Filters opent een regel per criterium: actief, in conflict, update beschikbaar, geen categorie,
endorsed, tracked, met een Nexus-id, met verborgen bestanden, geen geldige speldata, voor een ander spel,
met notities, en een regel per soort inhoud (plugins, BSA- of BA2-archieven, textures, meshes, scripts,
plugins van de script extender, interfacebestanden, geluid, FaceGen). Een klik zet een regel een stap
verder: uit, only, except. De bovenste regel wisselt tussen mods die aan alle aangezette criteria voldoen
en mods die aan minstens één voldoen. Het naamvak en de categorielijst beperken het resultaat altijd nog
verder. De regel voor verborgen bestanden kijkt alleen naar ingeschakelde mods.

### Wat de lijst je over een mod vertelt

Twee adviserende vlaggen, allebei een teken met de uitleg bij het zweven. No valid game data betekent dat
niets bovenin de mod eruitziet als iets wat dit spel laadt; misschien moeten haar mappen een niveau
omhoog, of misschien is het geen mod voor dit spel. Another game betekent dat de eigen `meta.ini` van de
mod een ander spel benoemt. Geen van beide blokkeert iets: de mod wordt nog steeds uitgerold. "Mark as
valid" in het rijmenu legt ze allebei het zwijgen op, via de eigen `validated=`-sleutel van MO2, zodat een
mod die in de ene manager als geldig is gemarkeerd in de andere niet wordt gevlagd.

De indelingscontrole vlagt een mod alleen wanneer ze zeker is: een `Root/`-boom, een onleesbare map en een
lege map tellen allemaal als geldige speldata.

### Een mod uitzetten die je saves of andere mods nodig hebben

Voordat een mod wordt uitgezet of verwijderd, leest Eidos de pluginlijsten van de drie nieuwste saves van
het profiel. Gebruikt een ervan een plugin uit die mod, dan noemt een balk boven de lijst de plugins en
wacht op Disable anyway of Cancel (Esc annuleert ook). Remove zet dezelfde namen in zijn melding om nogmaals
te klikken. Alleen saves van Skyrim en Fallout 4 worden gelezen; andere games zetten de mod uit zonder te
vragen.

Het tabblad General van Information bij een mod toont de mods die ze nodig heeft en de mods die haar nodig hebben, gelezen uit de masters van de ingeschakelde plugins. Een mod uitzetten die anderen nodig hebben toont dezelfde balk met Disable them too, wat ook elke mod uitzet die haar nodig heeft en de mods die die weer nodig hebben, of Keep them enabled. De kaart voor ontbrekende masters op het tabblad Diagnostics heeft Enable what provides them: die schakelt de mod in die elke ontbrekende master bevat en vinkt een masterplugin aan die wel aanwezig is maar niet is aangevinkt.

### Een mod back-uppen voor je eraan komt

"Back up this mod" kopieert haar map opzij als `<name>_backup` (daarna `_backup2`, enzovoort; een back-up
vervangt nooit de vorige). De kopie is geen mod: haar aanvinkvakje doet niets en ze draagt niets bij aan de
samengevoegde weergave, want haar aanvinken zou twee kopieën van één mod over elkaar uitrollen. "Restore this
backup over the mod" zet haar terug, in twee klikken; de huidige inhoud wordt eerst opzijgezet en pas
weggegooid zodra de kopie geslaagd is.

**Backups** op de werkbalk bewaart herstelpunten van de modlijst van het profiel, zijn laadvolgorde
(`plugins.txt` en `loadorder.txt` samen) en zijn spel-INI's, 10 van elk, in `<profile>/backups/`. Back up now
voegt er een toe, en Restore maakt eerst een back-up van de huidige toestand, zodat een verkeerde keuze
ongedaan gemaakt kan worden. Eidos bewaart ook een INI-herstelpunt vóór elke BethINI-run (zie
[tools.md](tools.md#bethini-pie)).

### De tabbladen

**Data** is een boom van de samengevoegde weergave, telkens één niveau uitgeklapt: een map
openen leest die map in elke laag die hem heeft, nooit elke ingeschakelde mod tegelijk. Ze
wordt opgebouwd uit dezelfde lagenstapel waaruit de koppeling bedient, dus whiteouts en
verborgen bestanden worden gerespecteerd en het tabblad toont wat het spel zal zien. Filter op
naam, versmal tot alleen betwiste bestanden, zoek met de kolommen Size en Modified uit wat waar
staat, en toon elke rij met Reveal in een bestandsbeheerder. Over een bestand zweven toont de
andere mods die het meeleveren. De `meta.ini`, `readme.txt`, `fomod/` en `Root/` in de bovenste
map van een mod blijven buiten Data, in dit tabblad en in de koppeling: de eerste drie zijn voor
de modbeheerder, en `Root/` gaat naar de spelmap. Een bestand dat het spel of een tool onder een
van die namen schrijft, komt in de Overwrite terecht en blijft zichtbaar.

Een `.exe`-rij in Data heeft Add tool, dat de Executables-editor opent met dat bestand al ingevuld (er wordt niets opgeslagen tot je op Save drukt), en Run, dat het bestand één keer start via de samengevoegde weergave, zoals een geregistreerde tool ([Tools](#tools)). Save tree schrijft de hele samengevoegde Data-boom naar een tekstbestand, één regel per bestand met de mod die het levert, zonder je thuismap en gebruikersnaam, voor een bugrapport.

**Plugins** is de ESP/ESM/ESL-laadvolgorde: omschakelen, met de hand herordenen, of sorteren met
LOOT en het verslag na de sortering lezen, waarvan de adviezenlinks in je browser openen. De kop
telt de ingeschakelde plugins per indexruimte tegen de limiet van het spel, bijvoorbeeld
`full 231/254, light 512/4096`, en wordt oranje zodra er een vol is. Het filtervak beperkt de lijst
op naam. Spatie, Ctrl+A, Shift-bereiken en Activate all shown en Deactivate all shown in het
rijmenu werken dan alleen op de getoonde rijen, en een sleep wordt geweigerd tot het filter is
gewist. Een plugin selecteren kleurt zijn masters in de pluginlijst en de mod die hem levert in de
modlijst. Plugins die LOOT als vuil markeert krijgen een Diagnostics-kaart die ze met xEdit kan
opschonen ([tools.md](tools.md#een-plugin-opschonen-die-loot-als-vuil-markeert)).

Na een LOOT-sortering laat een volledige plugin waarvan alle nieuwe records in het light-bereik
passen dat bij aanwijzen zien, en het menu van zijn rij bevat Flag as light. Eidos schrijft een kopie
met de light-vlag in een nieuwe mod met de naam `<plugin> (light)`, onderaan de modlijst (de hoogste
prioriteit), en laat het origineel ongemoeid, dus die mod uitzetten maakt het ongedaan. Eidos weigert
als een van de drie nieuwste saves van het profiel de plugin als volledige plugin laadt. Alleen saves
van Skyrim en Fallout 4 worden gelezen, dus bij Starfield wordt geen save gecontroleerd. LOOT
controleert alleen recordbereiken: bekijk een plugin die cellen of worldspaces toevoegt in xEdit
voordat je hem markeert.

Over een plugin zweven toont waar die vandaan komt, de auteur, de header- en formulierversie, de eerste 300 tekens van de beschrijving, de masters en, na een LOOT-sortering, de berichten van LOOT. Bij Skyrim, Skyrim SE, Skyrim VR en Enderal SE leest het spel ook een INI die naar een geladen plugin is genoemd; de kolom INI markeert die plugins en een klik opent dat bestand alleen-lezen.

**Conflicts** legt de winnaars en verliezers per bestand uit. **Overwrite** maakt in één stap een echte
mod van wat het spel geschreven heeft.

**Archives** toont elke BSA of BA2 die de ingeschakelde mods meeleveren, de mod waar hij vandaan komt en
waarom hij laadt: een actieve plugin met dezelfde naam, of de INI registreert hem. Een archief dat niets
laadt wordt als dood gewicht gemarkeerd. Levert een andere mod een archief met dezelfde naam dat het
conflict wint, dan noemt de rij die mod. Extract pakt een archief uit in een nieuwe mod die naar het
archief is genoemd zonder zijn extensie (`Foo.ba2` geeft `Foo (extracted)`) en laat het archief in zijn
mod staan; losse bestanden winnen van bestanden in elk archief, wat de modvolgorde ook is.

**Saves** leest de kop van elke save (personage, niveau, locatie, speeltijd), vergelijkt de erin opgeslagen
pluginlijst met de huidige, en heeft een knop die de mods inschakelt die ze nodig heeft. Saves die het
prefix zonder Eidos bereikten, staan beschreven onder [Saves horen bij het profiel](#saves-horen-bij-het-profiel).

**Downloads** en **Diagnostics** hebben hieronder hun eigen secties.

### Modinformatie

"Information..." opent een dialoog per mod met de tabbladen General, Conflicts, Filetree, INI Tweaks,
Notes, Nexus files en Nexus page. Vanuit de bestandsboom (en vanuit de Data-boom) kan elk bestand
verborgen worden: het wordt hernoemd naar `<name>.mohidden`, wat het uit de samengevoegde weergave haalt
zonder het te verwijderen, zodat een paar verdwaalde bestanden van één mod onderdrukt kunnen worden zonder
prioriteiten te wijzigen. De bestandsboom doet ook de gewone bestandsbewerkingen: nieuwe map, hernoemen,
verwijderen, openen. Elk weigert een pad dat geen gewoon pad binnen die mod is: geen `..`, geen absoluut
pad en geen symlink onderweg, want er een volgen kan een bestand buiten de modmap verwijderen. Hernoemen
verandert alleen het laatste deel van het pad, dus het verplaatst nooit een bestand, en het weigert een
naam die al bezet is. Verwijderen vergt twee klikken en kan niet ongedaan worden gemaakt.

**View** op elke rij in de bestandsboom of de Data-boom toont een voorbeeld van het bestand: afbeeldingen,
tekst, DDS-textures (kies de mip, laag, zijde en het kanaal) en statische NIF-modellen met hun
diffuse-textures. De NIF-voorvertoning heeft `eidos-nif-preview` nodig naast Eidos of in `PATH`; de
releasebundel, het Arch-pakket en builds vanuit de broncode hebben het, `eidos-linux` niet. Tekst wordt tot
64 KB gelezen en meldt waar ze gestopt is.

**INI Tweaks** toont de fragmenten die een mod in haar map `INI Tweaks/` meelevert; de ingeschakelde worden
bij het starten in prioriteitsvolgorde samengevoegd met de spel-INI van het profiel, en er weer afgehaald
wanneer de INI's van de run vastgelegd worden, dus een tweak uitschakelen haalt zijn instellingen weg.

Het tabblad **Nexus files** toont de bestanden van de mod op Nexus, nieuwste eerst, en markeert het bestand
dat hier geïnstalleerd is en het bestand dat het vervangt. Die tweede rij heeft een knop **Update**; zie
[Modupdates](#modupdates).

Het tabblad **Nexus page** toont de beschrijving van de mod en, nieuwste eerst, de changelog van elke
bestandsversie op haar pagina. Eidos leest beide de eerste keer dat je het tabblad opent, uit de
GraphQL-API van Nexus, die niet meetelt voor het verzoekenbudget per uur. Het bewaart ze tot Eidos sluit of
je op **Refresh** drukt. Ze worden getoond als platte tekst: BBCode- en HTML-opmaak wordt verwijderd, het
adres van een link staat achter de tekst ervan, en afbeeldingen en video's worden weggelaten. **Endorse** en
**Track** boven de tekst doen wat het rechtermuismenu van de mod doet. De pagina van een mod voor volwassenen
blijft verborgen tenzij je Nexus-account inhoud voor volwassenen toont, en een pagina zonder beoordeling
blijft in elk geval verborgen.

### Downloads

Een download kan vanuit de lijst Downloads op een positie in de modlijst gesleept worden om hem op die
prioriteit te installeren, en archieven of mappen die je vanuit een bestandsbeheerder op het venster laat
vallen installeren ook. Bestandsdrops vergen een X11- of XWayland-sessie: winit implementeert ze alleen voor
X11. Downloads kunnen gepauzeerd en hervat worden: pauzeren stopt de overdracht en houdt het gedeeltelijke
bestand, en Resume vraagt Nexus om een verse link en gaat verder waar het gestopt is.

Het tabblad Downloads bewaart de archieven die je downloadde. Filter op naam (ook de vriendelijke modnaam,
dus "skyui" vindt `SkyUI_5_2_SE-12604-5-2SE.7z`), sorteer op nieuwste, naam, grootte of staat, en verberg een
archief waarmee je klaar bent: het bestand blijft en alleen de rij verdwijnt. "Show hidden" haalt ze terug,
en dezelfde knop maakt ze weer zichtbaar. "Remove N installed" verwijdert de archieven van mods die je al
geïnstalleerd hebt, in twee klikken, en alleen die welke het filter toont.

**Preferences -> Nexus -> Install downloads when they finish** staat standaard uit. Staat de optie aan, dan
wordt een Nexus-download die klaar is terwijl het venster open is geïnstalleerd zonder omweg via het tabblad
Downloads. Het is de gewone installatie: een FOMOD opent zijn installatieprogramma en een naam die al in de
modlijst staat vraagt wat er moet gebeuren. Archieven die al in `downloads/` stonden voordat het venster
openging of voordat de optie aanging, blijven met rust, net als een download waarvan je de installatie hebt
geannuleerd. Zolang een collectie in het collectiepaneel openstaat, installeert geen download vanzelf, en een
die intussen klaar komt installeert ook later niet vanzelf; de collectie installeert haar leden. Het bestand
waarop een lopende Update wacht, wordt door die Update geïnstalleerd. Alleen Nexus-downloads worden zo
geïnstalleerd, en een die het Downloads-filter verbergt, wacht tot het filter is gewist. Zolang een sleep
bezig is, of zolang het spel of een ander Eidos de instantie vasthoudt, wacht de installatie.

### Modupdates

De updateknop aan het rechtereinde van de werkbalk controleert elke mod met een Nexus-id. Toont de modpagina
de versie die je hebt geïnstalleerd, dan wordt er niets meer gevraagd. Anders leest Eidos de bestandslijst van
de mod (nog één verzoek, bewaard voor de rest van de controle) en volgt de updateketen vanaf het bestands-id
dat de `meta.ini` van de mod vastlegt: auteurs markeren een nieuwe upload als opvolger van een oudere, en MO2
leest dezelfde keten. Een nieuwer bestand in die keten is de update. Noemt de keten er geen en staat het
geïnstalleerde bestand nog op de pagina, dan telt de mod als actueel, ook al toont de pagina een andere versie.
De versie van de pagina beslist alleen wanneer er geen bestands-id is vastgelegd, wanneer het geïnstalleerde
bestand is verwijderd of gearchiveerd, of wanneer Nexus de bestandslijst niet stuurde.

Elke mod gebruikt het Nexus-domein van het spel dat zijn eigen `meta.ini` noemt (`gameName`), niet dat van de
instantie: de updatecontrole, Endorse, Track en Visit on Nexus gaan er allemaal heen. Enderal SE accepteert
ook `skyrimspecialedition`-mods, dus een Skyrim SE-mod die in een Enderal SE-instantie is geïnstalleerd, wordt
op zijn Skyrim SE-pagina gecontroleerd. Een Mod Manager Download-link gaat naar de instantie die je het laatst
gebruikte wanneer haar spel het domein van de link accepteert. Anders gaat hij naar een instantie van het spel
waarvan het eigen domein het is, vóór een van een spel dat het alleen accepteert. `EIDOS_INSTANCE=<folder>`
overstemt beide.

Track en Untrack in het rechtermuismenu van een mod voegen de mod toe aan je gevolgde mods op Nexus of halen
hem eruit, zodat de meldingen van de site overeenkomen met wat je hier volgt. Endorse en Abstain in hetzelfde
menu endorsen de mod op Nexus of trekken de endorsement in; de endorse-knop naast de updateknop endorst de
geselecteerde mod. Elke updatecontrole leest ook je endorsements en gevolgde mods van Nexus (twee verzoeken) en
zet de vlaggen Endorsed en Tracked van elke mod daarnaar, op het eigen Nexus-domein van de mod. Verschillen die
twee, dan wint Nexus. Kan een van beide lijsten niet worden gelezen, dan meldt de controle dat en blijven die
vlaggen zoals ze waren.

De knop **Update** op het tabblad Nexus files van "Information..." installeert het nieuwere bestand. Met een
premium-account downloadt Eidos het bestand zelf. Met een gratis account opent het de pagina van het bestand,
waar Mod Manager Download het bestand terugstuurt, en de installatie begint zodra het archief in `downloads/`
aankomt. Vanaf daar is het de gewone installatie: een FOMOD opent met de antwoorden waarmee de mod geïnstalleerd
werd, daarna biedt de vraag Replace aan met "Keep a backup of the existing mod" aangevinkt, wat je bij de vorige
vraag ook koos. Replace behoudt de prioriteit van de mod en of hij aanstaat.

Voordat een Replace de map van een mod verwisselt, vergelijkt Eidos de oude map met de nieuwe op bestandsnaam en
grootte en leest het de headers van de plugins in beide. Dit geldt voor Update, voor een collectie-update en voor
Replace gekozen in de installatievraag. De statusregel telt daarna de toegevoegde, verwijderde en in grootte
gewijzigde bestanden en noemt elke master die de nieuwe versie nodig heeft en de oude niet. Het tabblad
**General** van "Information..." eindigt met een sectie **Last update** met de versies, de toegevoegde of
verwijderde plugins, de toegevoegde of verwijderde masters, de plugins waarvan de ESM-, ESL- of ESH-vlag
veranderde, de mods waarmee deze nu bestanden deelt of niet meer deelt, en de bestanden. Een bestand dat met
dezelfde grootte is herschreven, staat er niet in. Het verslag staat in `logs/updates/<mod>.json` in de
instantie, die `eidos pack` weglaat; een verse installatie of een Merge in de map verwijdert het.
`eidos install --replace` toont dezelfde aantallen op een regel `changes:`.

Een map met meerdere Nexus-bestanden, zoals een hoofdbestand met een optioneel bestand erin samengevoegd, wordt
geweigerd: hem vervangen door het nieuwe hoofdbestand zou de andere kwijtraken. Werk die met de hand bij.

`eidos nexus update skyrimse --apply` doet dezelfde controle en downloadt daarna met een premium-account elk
nieuw bestand en installeert het met `--replace --backup` over zijn map. Met een gratis account drukt het in
plaats daarvan `OPEN <url>` af voor elke bestandspagina. Mappen die meerdere bestanden samenvoegen worden met de
reden overgeslagen. Een FOMOD-mod zonder opgeslagen antwoorden, zoals een die geïnstalleerd is voordat Eidos ze
bewaarde, wordt niet opnieuw geïnstalleerd met de standaardwaarden van het installatieprogramma: die update
mislukt met een melding om de mod één keer vanuit het venster bij te werken, dat de antwoorden dan opslaat. Een
FOMOD waarvan het installatieprogramma niets te kiezen geeft, wordt gewoon geïnstalleerd. De opdracht eindigt met
1 als een update mislukte.

### Collectie-updates

Als je Check for updates draait, vraagt Eidos Nexus ook om de nieuwste revisie van elke collectie die in deze instantie is geïnstalleerd. Een regel boven de modlijst noemt elke collectie met een nieuwere revisie en telt de toegevoegde (+), bijgewerkte (~) en vervallen (-) mods; houd de muis erboven voor de namen. De aantallen ontbreken als het recept van de geïnstalleerde revisie niet meer in de cache staat. **Update** opent de nieuwere revisie in het collectiepaneel, waar *Install this collection* hem uitvoert. De lijst wordt bij elke controle opnieuw opgebouwd en niet bewaard.

### Nexus-instellingen

**Preferences -> Nexus -> Offline** laat Eidos Nexus helemaal niet meer benaderen en slaat de dagelijkse releasecontrole op GitHub over. Updatecontroles, aanmelden, downloads en collecties zeggen dat, in plaats van te falen met een verbindingsfout. Het staat standaard uit.

**Preferred servers** rangschikt de CDN-knooppunten die een download verkiest, beste eerst. Alleen een premium-account krijgt ooit meer dan één mirror om uit te kiezen, dus voor alle anderen kiest Nexus en verandert dit niets. Wordt geen van de servers die je noemde aangeboden, dan gebeurt de download toch, vanaf het eerste knooppunt dat Nexus aanbiedt.

### Categorieën

Categorieën kunnen aan één mod of aan een hele selectie worden toegekend, de catalogus kan vanuit dezelfde dialoog worden bewerkt, en de officiële categorielijst van het spel kan bij Nexus worden opgehaald. Beide catalogusbestanden zijn die van MO2 (`categories.dat` en `nexuscatmap.dat`), dus een gedeelde instantie houdt één catalogus. Een van Nexus gedownloade mod krijgt bij de installatie een categorie, zodra Fetch from Nexus de catalogus heeft gekoppeld.

### INI-editor, log en extensies

**View -> INI editor** bewerkt de spel-INI's van het profiel, de kopie die Eidos bewaart; de kopie in het Proton-prefix wordt bij elke start overschreven. Het laatste tabblad is de `launch-env.ini` van het profiel ([Variabelen en wrappers per profiel](#variabelen-en-wrappers-per-profiel)). **View -> Log** leest de sessielogs. **View -> Extensions** toont je eigen add-ons; zie [extensions.md](extensions.md).

Vindt een start de hoofd-INI van het spel (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...) niet in het profiel en niet in het prefix, dan kopieert Eidos eerst de standaardversie uit de spelmap (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) naar het profiel, zoals MO2 doet. Draait de instantie een spelkopie, dan komt de standaardversie uit de kopie. Het prefix en de spelmap blijven ongewijzigd.

### Mods installeren

Het installatieprogramma verwerkt Simple-archieven, FOMOD, Wrye Bash-BAIN-pakketten (vink de subpakketten aan, die op volgorde samenvloeien) en de OMOD-pakketten van Oblivion. Een OMOD-script draait wanneer het de OBMM-opdrachten gebruikt die Eidos ondersteunt of een van de drie C#-installatieprogramma's is die Eidos kent; Eidos draait geen andere C#- of Visual Basic-code en zegt dat. Herkent geen enkele heuristiek de indeling, dan toont een handmatige kiezer de archiefboom en laat je op de dataroot wijzen.

Een FOMOD-mod opnieuw installeren of bijwerken opent de wizard met de antwoorden van de vorige installatie aangevinkt, op de eerste stap waarvan de opties sindsdien veranderd zijn, met een lijst van de wijzigingen. **Reinstall with new choices** begint in plaats daarvan bij de standaardwaarden van het installatieprogramma. `eidos install --replace` of `--merge` hergebruikt de opgeslagen antwoorden en weigert, met de lijst van wijzigingen, wanneer ze niet meer passen.

### Diagnostics

Het tabblad Diagnostics draait live Health-controles. Het meldt ontbrekende masters, plugins die LOOT als vuil markeert, archieven die geen enkele actieve plugin zal laden, of de modlijst nog overeenkomt met de mods-map, of FUSE-passthrough aan staat en, wanneer erom gevraagd is, of de start-capability die het nodig heeft er is. Na een run toont het wat het eigen log van de script extender zegt over elk van zijn plugin-DLL's, zodat je kunt zien of je SKSE-plugins laadden. Het meldt ook een spelupdate sinds de laatste schone start, een ontbrekende script extender of een die niet bij de spelversie past (met de downloadpagina van de extender en een knop om het gedownloade archief te installeren), een Steam-opstartoptie die Eidos niet met `%command%` start, NVAPI dat voor DLSS uit blijft, exclusief volledig scherm dat framegeneratie tegenhoudt, een ENB-`enblocal.ini` zonder `LinuxVersion=true`, de plugindeclaraties van F4SE- en SFSE-plugins, en het crashlog van de laatste sessie met de mods die het noemt.

De controles voor de profiel-INI's en plugins zoeken `sTestFile`-regels in `[General]` van de aangepaste INI's van Fallout 4, Fallout 4 VR en Starfield, waardoor het spel `plugins.txt` negeert (Fix verwijdert ze uit de kopie van het profiel en bewaart het vorige bestand als `<ini>.pre-fix`), ingeschakelde `.esp`-plugins in Starfield, en blueprintplugins die de blueprintvlag, het voorvoegsel `BlueprintShips-` of de extensie `.esm` missen, of waarvan de hoofdplugin niet is ingeschakeld. De `sTestFile`-controle geldt alleen voor Fallout 4, Fallout 4 VR en Starfield. De systeemcontroles vlaggen een `vm.max_map_count` of een harde limiet voor open bestanden onder wat Proton nodig heeft, en een spel, prefix of instantie op NTFS, exFAT of FAT. De modcontroles noemen het datapad van BodySlide wanneer het niet naar de `Data` van het spel wijst, `Data`-mappen die een tool in `Overwrite/Root` schreef, een patch waarvan de losse bestanden verliezen van de mod die hij patcht, en een mod die de Papyrus-scripts van de script extender vervangt.

Op updates controleren leest ook de Nexus-vereisten van elke mod. Diagnostics toont een vereiste mod die ontbreekt of is uitgeschakeld, met Ignore voor een vereiste die op een andere manier is vervuld.

Het tabblad toont ook welke Proton-build het spel draait en waarschuwt voordat een oudere Proton een prefix draait dat een nieuwere heeft ingericht; Rebuild zet een kapot prefix opzij zodat Steam een nieuw maakt ([troubleshooting.md](troubleshooting.md#welke-proton-het-spel-draait-en-een-kapot-prefix)). Andere kaarten vlaggen een prefix dat nooit is ingericht of waarvan het register verouderd is, een ander programma dan Eidos dat Mod Manager Download-links ontvangt (Register lost het op), toolresultaat dat ouder is dan zijn invoer (met een knop die de tool opnieuw draait), en een opgeschoonde of light-kopie van een plugin waarvan het origineel sindsdien is gewijzigd. Elke kaart wordt uitgelegd in [troubleshooting.md](troubleshooting.md#health-kaarten). **Copy report** zet een rapport van het systeem en elke bevinding op het klembord, met je thuismap en inlognaam verborgen, voor een bugrapport.

Voor Skyrim SE, Enderal SE en Fallout 4 heeft Diagnostics ook een kaart Display. Die toont de venstergrootte en de venstermodus uit de `SkyrimPrefs.ini` van het profiel (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), of uit de kopie in het prefix zolang het profiel er nog geen heeft. Stelt geen van beide een grootte in, dan beginnen de velden bij het grootste aangesloten scherm dat in `/sys/class/drm` staat, zodat een Steam Deck 1280x800 toont. De breedte en de hoogte zijn gehele getallen van 320 tot 16384. Apply schrijft `iSize W`, `iSize H`, `bFull Screen` en `bBorderless` in de kopie van het profiel, nadat het de kopie uit het prefix heeft overgenomen als het profiel er geen heeft. Stelt SSE Display Tweaks het venster in, dan schrijft Apply dezelfde grootte en modus in de kopie daarvan in de Overwrite; het bestand van de mod zelf verandert niet.

De kaart Game files bovenaan het tabblad staat beschreven onder [Spelbestanden](#spelbestanden-de-winkelinstallatie-of-een-kopie), de kaart voor de collectievolgorde onder [De collectievolgorde bewaren](#de-collectievolgorde-bewaren), en Find the mod behind a problem onder [Testruns](#de-mod-achter-een-probleem-vinden).

### Het spel starten

Om het spel via het venster te starten, zet je de Steam-opstartoptie van het spel op het absolute pad van het programma, omdat Steam het `PATH` van je shell niet leest ([install.md](install.md#daarna-steam-ernaar-laten-wijzen)):

```
~/.local/bin/eidos-gui %command%
```

Vanuit Colony is het pad `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos opent op de instantie van het spel, de laatst gebruikte, zodat een draagbare instantie net zo goed wordt teruggevonden als de globale; klik op Run om het door de samengevoegde weergave te starten.

Met **Start the game at once when a store opens Eidos** aan (Preferences, General, Running a game; standaard uit) slaat Eidos die klik over: het controleert Health en start het spel zonder zijn venster te tonen. Het venster opent zoals gewoonlijk wanneer Health een probleem meldt, wanneer de modlijst aandacht nodig heeft, wanneer een tool die vóór het spelen moet draaien een verouderde uitvoer heeft of, bij Skyrim SE, Enderal SE en Fallout 4, wanneer het profiel nog geen venstergrootte heeft (het venster opent dan op Diagnostics). Wil je het venster voor één start, zet dan `EIDOS_WINDOW=1` voor het commando: `EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

In een venster dat geen winkel opende, start Run het spel niet zelf, en Eidos start een spel nooit rechtstreeks via Proton of Wine. Run vraagt de winkel die de kopie bezit om het te starten, en de winkel start Eidos:

- Steam draait `steam -applaunch <appid>` als de opstartoptie hierboven is ingesteld en het spel een Proton-prefix heeft, wat Steam maakt de eerste keer dat het het spel start. Is Steam gesloten, dan start Steam eerst.
- Heroic (een GOG- of Epic-kopie, native of Flatpak) krijgt een `heroic://launch`-link als Eidos een wrapper is in de instellingen van het spel in Heroic, met het absolute pad van `eidos-gui` (of `eidos-linux`) als uitvoerbaar bestand en zonder argumenten. Is Heroic gesloten, dan start Heroic eerst.
- Legendary op zichzelf draait `legendary launch <app>` als `~/.config/legendary/config.ini` in de sectie `[<app>]` van het spel, of onder `[default]`, `wrapper =` met het absolute pad van `eidos-gui` (of `eidos-linux`) bevat.

De loader van de script extender, de launcher en het uitvoerbare bestand van het spel in de Run-lijst worden allemaal zo aan de winkel doorgegeven, en `eidos tool <instance> run <entry>` voor een ervan in een terminal ook. Eidos kan sommige kopieën niet aan hun winkel doorgeven, zoals een GOG-kopie buiten Heroic: Run zegt dan het spel vanuit die winkel te starten.

De overdracht legt deze instantie vast als de laatst gebruikte, en de winkel opent dan een tweede Eidos-venster erop. Met Start the game at once uit klik je daar op Run; met aan start dat venster het spel zonder zichzelf te tonen, tenzij een van de gevallen hierboven het opent. Het eerste venster volgt die run niet, dus druk daarin na het spelen op F5 om zijn lijsten opnieuw te laden. Ontbreekt de instelling in de winkel, of staat de loader van de script extender niet naast het uitvoerbare bestand van het spel, dan start Run niets en toont het wat je waar moet instellen, met het echte pad van het draaiende binaire bestand.

Terwijl een spel of tool draait, wordt Run Stop. Twee keer drukken beëindigt elk programma dat de run startte, en Eidos legt daarna de sessie vast en ontkoppelt zoals na elke afsluiting; een gestopte run telt nooit als een schone start ([troubleshooting.md](troubleshooting.md#een-spel-of-tool-dat-niet-afsluit)).

De `%command%` van Steam wijst bij de Bethesda-titels meestal naar `<Game>Launcher.exe`. Eidos draait die nooit: de launcher is een aparte instellingenapp die `Data` opnieuw scant en `plugins.txt` herschrijft, en zo de zojuist uitgerolde laadvolgorde ongedaan maakt. Eidos draait in plaats daarvan de loader van de script extender als er een geïnstalleerd is, anders het binaire bestand van het spel, en meldt het wanneer het moet terugvallen op het binaire bestand van het spel.

Eidos scant de DLL's van de ingeschakelde mods op een import van `d3dcompiler_47.dll`, kopieert de DLL van Microsoft naar het prefix en zet pas dan `WINEDLLOVERRIDES=d3dcompiler_47=n,b`; je hoeft die zelf niet in te stellen.

### Variabelen en wrappers per profiel

Een profiel kan eigen omgevingsvariabelen en wrapper-commando's bevatten in `profiles/<naam>/launch-env.ini`, één regel per item:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Ze gelden voor elke `eidos play` van dat profiel (het spel dat door Steams `%command%` wordt gestart, of door Run in een venster dat een winkel opende) en voor elke toolrun. `eidos shell` krijgt de variabelen en geen wrapper. De eerste `wrapper=`-regel is de buitenste. Een `wrapper=`-regel met `%command%` krijgt het commando op die plek, zoals in Steams opstartopties; een regel zonder krijgt het commando aan het eind. Wrappers draaien in de privénaamruimte van Eidos, dus ze zien de mods.

Een variabele hier vervangt de waarde die Eidos voor dezelfde naam instelt, en de eigen `env=`-regel van een tool in `tools.ini` vervangt beide. `WINEDLLOVERRIDES` wordt juist toegevoegd na de overrides die Eidos instelt voor ENB, ReShade en de meegeleverde DLL's: een DLL die in beide staat krijgt de instelling van het profiel, de overige houden hun eigen.

View -> INI editor opent het bestand als laatste tabblad, voor elk spel. Lege regels en regels die met `#` of `;` beginnen worden overgeslagen. Bij het opslaan noemt Eidos de regelnummers die het negeert: regels die noch `env=NAAM=waarde` noch `wrapper=commando` zijn, en een `wrapper=`-regel waarvan het eerste woord `NAAM=waarde` is, zoals Steams opstartopties toestaan; zet die variabele op een `env=`-regel. `eidos play <instance>` zonder commando toont de variabelen die een run krijgt en de wrappers. Copy report in Diagnostics noemt de variabelen en de wrapperprogramma's, zonder hun waarden.

## Tools

xEdit, BodySlide, DynDOLOD en andere tools draaien door de samengevoegde weergave binnen het Wine-prefix van het spel (het Proton-prefix van Steam, of dat wat Heroic voor een GOG- of Epic-kopie heeft ingericht):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # een exe eenmalig draaien zonder te registreren
eidos prereqs skyrimse            # wat de geregistreerde tools nodig hebben, en de staat ervan
eidos prereqs skyrimse --install  # ophalen wat ontbreekt
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen en Synthesis worden gevonden zonder ze te registreren. Pandora, Nemesis, FNIS, BodySlide en Synthesis komen met een al ingestelde opvangmod, dus hun uitvoer gaat in een eigen mod (details in [tools.md](tools.md#de-uitvoer-van-een-tool-naar-een-eigen-mod-sturen)).

De titel die je een tool geeft bepaalt welke runtimes en DLL's Eidos ervoor klaarzet: `BodySlide` krijgt zijn DirectX-bibliotheken, `BS` krijgt niets. In het venster toont de dialoog Executables de echte staat van elke vereiste onder het veld, en de ontbrekende zijn knoppen.

De releasebundel bevat de installatieprogramma's van vcrun2022 (x86 en x64), de .NET 8 Desktop Runtime 8.0.12 en de .NET 10-runtime 10.0.10 in `runtimes/`, en `install.sh` kopieert ze naar `~/.local/share/Colony/Eidos/runtimes/`, zodat `eidos prereqs <instance> --install` ze zonder download installeert. Eidos draait de installatieprogramma's van vcrun2022 en .NET 8 van Microsoft zonder venster in het prefix van het spel, zonder winetricks. `eidos-linux` en builds vanuit de broncode hebben geen map `runtimes/`, dus ze downloaden dezelfde vastgelegde bestanden van Microsoft. De .NET 10 SDK die Synthesis nodig heeft (ongeveer 300 MB) is altijd een download.

De tabel, de drie niveaus van vereisten, waarom DynDOLOD een .NET-runtime nodig heeft die winetricks niet kan installeren, en waarom een als mod geïnstalleerde tool vanaf het samengevoegde pad gestart wordt in plaats van vanuit zijn eigen map, staan in [tools.md](tools.md).

## Extensies

Eidos kan uitgebreid worden zonder opnieuw gebouwd te worden: een TOML-manifest in `~/.config/Colony/Eidos/addons/` voegt een tool toe aan de lijst Extensions of een controle aan het tabblad Diagnostics. Er wordt niets in Eidos geladen: een extensie is een programma dat het uitvoert. Zie [extensions.md](extensions.md).

## Het proof of concept proberen

Het script heeft geen spel nodig. Het toont de samengevoegde weergave, copy-on-write, een spelmap die onaangeroerd blijft, en een samenvoeging die alleen binnen zijn naamruimte bestaat, met alleen onbevoorrechte OverlayFS in een gebruikersnaamruimte (Linux 5.11 of nieuwer):

```sh
./scripts/poc-overlay.sh
```
