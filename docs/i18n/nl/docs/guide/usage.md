<!-- eidos-i18n: source=docs/guide/usage.md sha=46ad634368dc712808acd2f7916663f4b7b900a3 -->

# Eidos gebruiken

De praktische handleiding: de CLI, de GUI, de Steam-opstartoptie, bouwen vanaf de
broncode, en het proof-of-concept-script. Wat te doen wanneer er iets mis lijkt,
staat in [troubleshooting.nl.md](troubleshooting.md).

## Gebruiken (CLI)

```sh
eidos games                       # hier geïnstalleerde ondersteunde spellen (zoals de lijst van MO2)
eidos init skyrimse               # een modding-instantie maken
# ...zet elke mod als map in <instance>/mods/ (de globale instantie staat
#    in ~/.local/share/eidos/skyrimse; `eidos init` toont die van jou)...
eidos install skyrimse mod.7z     # of een gedownload archief installeren (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # de volgorde + pluginstatus van een bestaand MO2-profiel overnemen
eidos sort skyrimse               # de laadvolgorde van de plugins met LOOT sorteren
eidos play skyrimse               # tonen wat er gekoppeld zou worden
eidos play skyrimse -- <command>  # <command> draaien met de mods over het spel gekoppeld
eidos pack skyrimse backup.eidos  # de hele instantie in één bestand, om ze elders heen te verhuizen
eidos unpack backup.eidos <folder>   # zet ze terug op de andere machine
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` en `eidos export` maken
het geheel compleet; draai `eidos` zonder argumenten voor de volledige lijst.

### Instanties: globaal en draagbaar

Elke opdracht hierboven spreekt een instantie aan. `skyrimse` benoemt de
**globale** - centraal opgeslagen in `~/.local/share/eidos/skyrimse`, beheerd
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
add-on-definities die je geschreven hebt), `~/.local/state/Colony/Eidos/logs/`
voor sessielogs, en `~/.local/share/Colony/Eidos/` voor wat Eidos gedownload
heeft. Een oudere Eidos hield die in `~/.config/eidos/` en
`~/.local/state/eidos/`; de eerste start na het bijwerken **kopieert** ze over en
meldt dat in het log. De oude mappen blijven precies zoals ze waren - er wordt
niets verwijderd, zodat een mislukte upgrade je geen aanmelding kan kosten - en
je kunt ze zelf weghalen zodra je tevreden bent.

Je mods horen daar niet bij. Een globale instantie staat nog steeds in
`~/.local/share/eidos/<game>/`, en een draagbare waar jij ze gezet hebt, omdat
die paden in je instantielijst geschreven staan en mogelijk in een
Steam-opstartoptie: ze verplaatsen zou een verbinding breken waarvan Eidos niet
beide uiteinden bezit.

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

### Geen bevoorrechte stap nodig

Eidos draait volledig zonder root. Het koppelt in een privé-user- +
mount-naamruimte, dus geen setuid-helper, geen daemon, en niets te verlenen.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` is **optioneel** en regelt
precies één ding: kernel-FUSE-passthrough, dat standaard uit staat omdat het het
spel breekt (hieronder). Met die capability neemt Eidos een gewone
mount-naamruimte in plaats van een user-naamruimte; mods worden hoe dan ook
identiek uitgerold.


Waarom het oude `setcap`-advies weg is - en waarom FUSE-passthrough uit
geleverd wordt - wordt uitgelegd in
[troubleshooting.nl.md](troubleshooting.md#waarom-passthrough-standaard-uit-staat).

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

Als 7-Zip iets niet kon lezen, wordt het archief alsnog geschreven en van een
naam voorzien - het is het waard om te hebben - maar `eidos pack` sluit af met
**1** en zegt wat er ontbreekt. Een onvolledige back-up is geen succes, en dit is
een opdracht die mensen vóór `&&` zetten.

Een `.eidos`-bestand is een 7-Zip-archief onder een eigen naam, en dat is een
keuze, geen vermomming: 7-Zip is toch al nodig om überhaupt een mod te
installeren, dus dit voegt geen afhankelijkheid toe, en raak je Eidos ooit kwijt,
dan opent je back-up nog steeds in elk archiefprogramma dat je hebt. Het is
**niet-solide**, dus één bestand kan uit een back-up van 70 GB gehaald worden
zonder alles wat ervoor staat uit te pakken, en het wordt gecomprimeerd met LZMA2
op `-mx1` - ongeveer 43% van het origineel bij een textuurzware lijst, in een
fractie van de tijd die `-mx9` besteedt om een paar punten meer te winnen.
Texturen en meshes zijn al gecomprimeerde formaten; er blijft heel weinig over
voor een groter woordenboek om te vinden. `--level` (0, 1, 3, 5, 7, 9) overstemt
dat als je het voor je eigen inhoud oneens bent, en `--no-downloads` laat
`downloads/` weg.

Packen neemt het slot van de instantie, dus het kan niet draaien terwijl het spel
of een andere Eidos naar die instantie schrijft - de back-up is een momentopname,
geen veeg. Ze wordt onder een `.part`-naam geschreven en op het eind hernoemd,
zodat een onderbroken pack iets zichtbaar onafs achterlaat in plaats van een
`.eidos`-bestand dat compleet lijkt en het niet is.

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
| Symbolische links | 7-Zip zou er een volgen en kopiëren waar hij naar wijst, wat bij een absolute link betekent dat er een vreemde boom je back-up in getrokken wordt. Ze worden in plaats daarvan gemeld. |

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
en welke tools wel benoemd maar niet meegenomen worden. De Unpack-dialoog leest
het manifest van de back-up zodra je het bestand kiest, zodat ze je kan vertellen
welk spel erin zit, wanneer ze gemaakt is, waar ze vroeger stond en welke van
haar tools hier ontbreken - voordat je haar een map geeft.

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

Een wizard bij de eerste start in MO2-stijl, in de perkament-/bordeauxlook van
Colony: welkom -> instantietype (draagbaar / globaal) -> spel -> naam & locatie
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
toon elke rij met Reveal in een bestandsbeheerder. **Plugins** is de
ESP/ESM/ESL-laadvolgorde (omschakelen, met de hand herordenen, of sorteren met
LOOT en het verslag na de sortering lezen, waarvan de adviezenlinks in je browser
openen). **Conflicts** legt de winnaars en verliezers per bestand uit.
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

### Nexus-collecties

Plak een collectielink - of klik er een aan op de site - en Eidos toont de leden
van die revisie, elk gekoppeld aan deze instantie: geïnstalleerd, gedownload of
ontbrekend. Het **leest** een collectie; het installeert er geen, en het paneel
zegt dat. Vier dingen maken een installer hier oneerlijk in plaats van alleen
moeilijk: de leden zijn gewone Nexus-bestanden die een sleutel per bestand vergen
die alleen een premium-account buiten de knop van de site zelf kan aanmaken; een
volledige installatie is drie API-oproepen per lid tegen een budget dat deze
client weigert te overschrijden; de fasen, regels en herspeelde FOMOD-antwoorden
van het manifest konden niet gecontroleerd worden tegen een echt gepubliceerde
Bethesda-collectie, en gokken levert een laadvolgorde die er goed uitziet en het
niet is. Lezen kost één verzoek en is exact.

Een collectie kan alleen tegen **haar eigen spel** gelezen worden. Open een
Skyrim-collectie met een Fallout 4-instantie geladen en het weigert met naam en
toenaam in plaats van de leden aan de verkeerde modlijst te koppelen, waar elke
"installed" en elke "missing" ruis zou zijn in de vorm van een antwoord.

### Offlinemodus

**Settings -> Nexus -> Offline** laat Eidos Nexus helemaal niet meer benaderen.
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
houdt één catalogus.

**View -> INI editor** bewerkt de spel-INI's van het profiel - de kopie die
blijft bestaan, niet die begraven in het Proton-prefix die bij elke start
overschreven wordt. **View -> Log** leest de sessielogs. **View -> Extensions**
toont je eigen add-ons; zie [extensions.nl.md](extensions.md).

Installeren aanvaardt alles: de Simple- en FOMOD-paden, plus Wrye
Bash-**BAIN**-pakketten (vink de subpakketten aan, die op volgorde samenvloeien)
en een **handmatige** kiezer die de archiefboom toont en je op de dataroot laat
wijzen wanneer geen enkele heuristiek de indeling herkent. Geen enkel archief
wordt geweigerd.

**Diagnostics** draait live gezondheidscontroles: bovenal de startmogelijkheid,
ontbrekende masters (de betrouwbaarste crashvoorspeller die er is), archieven die
geen enkele actieve plugin zal laden, of de modlijst nog overeenkomt met de
mods-map, en - na een run - wat het eigen log van de script extender over elk van
zijn plugin-DLL's zegt, wat "zijn mijn SKSE-plugins geladen?" van een
gevolgtrekking in bewijs verandert.

Om het spel via de GUI te starten, zet je de Steam-opstartoptie van het spel op
het absolute pad van het binaire bestand (Steam ziet `~/.cargo/bin` niet in
PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos opent op de instantie van dat spel - de laatst gebruikte, dus een draagbare
instantie wordt net zo goed teruggevonden als de globale; klik op Run om het door
de samengevoegde weergave te starten. (De Run-knop toont precies deze regel, met
het echte pad van het draaiende binaire bestand, als je erop drukt buiten Steam.)

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
map, staan in [tools.nl.md](tools.md).

Bouwen vanaf de broncode en de indeling van de repository staan in
[../internals/contributing.md](../../../../internals/contributing.md).

## Extensies

Eidos kan uitgebreid worden zonder opnieuw gebouwd te worden: een TOML-manifest
in `~/.config/Colony/Eidos/addons/` voegt een tool toe aan de lijst Extensions of
een controle aan het tabblad Health. Er wordt niets in Eidos geladen - een
extensie is een programma dat het uitvoert. Zie
[extensions.nl.md](extensions.md).
