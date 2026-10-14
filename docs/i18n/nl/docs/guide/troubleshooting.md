<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Probleemoplossing en diagnostiek

Alles voor de dag waarop het spel iets ziet waarmee het bestandssysteem het niet
eens is: de omgevingsschakelaars, hoe je de bewerkingstellers leest, de bekende
problemen en hun geschiedenis, en het verhaal van passthrough.

### De VFS diagnosticeren

Er bestaan twee omgevingsvariabelen voor wanneer het spel iets ziet waarmee het
bestandssysteem het niet eens is:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

De fijnmazige vorm is wat de crash gevonden heeft die in troubleshooting.md
beschreven staat: alle vier uitzetten beantwoordt "ligt het aan de caching?", en
alleen ze bij naam noemen beantwoordt "aan welke". De tellers beantwoorden de
andere helft - een laadbeurt die `read 0` toont, is er een waarbij
`FUSE_PASSTHROUGH` elke byte in de kernel geleverd heeft, dus alles wat je op het
leespad wilde bijstellen is al gratis.

## Een union met de hand koppelen

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


#### Waarom passthrough standaard uit staat

Passthrough geeft de kernel het echte onderliggende bestand, zodat lezingen deze
daemon volledig overslaan. Het is een doorvoerwinst die hier correctheid kost.
A/B gemeten op Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, dezelfde
laadvolgorde van 82 plugins, met als enige variabele of het binaire bestand de
capability droeg:

| passthrough | `NtCreateFile`-fouten met `STATUS_ACCESS_VIOLATION`    |
|-------------|--------------------------------------------------------|
| aan         | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| uit         | 0                                                      |

Met passthrough aan opent het spel geen enkel eigen archief of plugin, wat in het
spel zichtbaar wordt als mods die er domweg niet zijn - geen fout, geen logregel.
Met passthrough uit bereikt dezelfde laadvolgorde het spelen, met haar plugins,
archieven en Papyrus-scripts actief.

De storing is van binnen de daemon onzichtbaar, en dat is wat het duur maakte om
haar te vinden: onze eigen `open` slaagt elke keer en de kernel weigert nooit een
onderliggend bestand (geverifieerd over een volledige mislukte sessie met
`EIDOS_FUSE_TRACE=open`: nul `open FAILED`, nul `passthrough refused`). De fout
ontstaat nadat de daemon `opened_passthrough` geantwoord heeft, dus geen enkele
logging aan de daemonkant kan haar zien. Ze is ook niet extensiespecifiek - ze
treft archieven en plugins evengoed, dat wil zeggen de bestanden die het spel
zijn hele draaitijd open houdt.

`EIDOS_FUSE_PASSTHROUGH=1` zet het weer aan, om te meten wat het oplevert of om
het mechanisme opnieuw te testen. De capability-waarschuwingen in de launcher en
het tabblad Diagnostics verschijnen alleen wanneer je erom gevraagd hebt.

Om het spel zelf via Eidos te starten, zet je zijn Steam-opstartoptie op:

```
eidos play skyrimse -- %command%
```

Zet er `WINEDLLOVERRIDES="d3dcompiler_47=n"` voor als Proton native
d3dcompiler nodig heeft voor het compileren van shaders; Eidos voegt dat samen
met alle DLL-overrides die een mod meelevert (ENB/ReShade/`.asi`-loaders).


### Wordt de laagindex ook echt gebruikt?

De index is alles-of-niets en wordt in stilte gebouwd: `LayerStack::new` krijgt
ofwel een volledige kaart van de alleen-lezen lagen ofwel `None`, waarna elke
bevraging ze precies zoals vroeger doorloopt. Niets in een sessielog houdt die
twee uit elkaar, dus een stack die stilletjes teruggevallen is, ziet er identiek
uit aan een die werkt - terwijl hij de oude kosten betaalt.

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

## Bekende problemen

### DLSS of frame generation doet stilzwijgend niets

Drie afzonderlijke oorzaken, elk zonder enige foutmelding: NVAPI niet
ingeschakeld in de opstartopties, exclusive fullscreen, of een verouderde
Reflex-FPS-limiet. De volledige checklist staat in
[graphics.nl.md](graphics.md).

**Een mod die één map op twee manieren spelt, verloor alles onder de tweede.**
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

De vorm ervan is het weten waard: de padindex heeft deze bug nooit gehad, omdat
hij elke map doorloopt die hij tegenkomt. Hij gaf stilletjes bestanden terug die
de fallback niet kon geven, en dat is de verkeerde volgorde - de fallback is het
antwoord dat nooit fout hoort te zijn.

**DynDOLOD's LODGen sterft en laat een leeg log achter.** Opgelost door
`dotnet10`; zie [tools.nl.md](tools.md). Het symptoom is onmiskenbaar:
`LODGen_SSE_<world>_log.txt` met een versiebanner, een regel `.NET Version:` en
verder niets, voor elke wereld, en een dialoogvenster dat alleen "failed to
generate object LOD for one or more worlds" zegt. De oorzaak is dat Wine's Mono
antwoordt voor .NET Framework, en hoeveel .NET Framework je ook installeert, het
lost het niet op - Proton vervangt `mscoree.dll` bij elke prefix-update door een
symlink naar zijn eigen boom.

**Wine kon niet zien dat de koppeling hoofdletters vouwt.** Opgelost, en dit was
degene die ertoe deed.

Er bestaat geen API voor "is dit bestandssysteem hoofdletterongevoelig", dus
Wine's `get_dir_case_sensitivity` snuffelt naar de markering die CIOPFS
achterlaat in de mappen die het bedient. Ontbreekt die, dan gaat Wine uit van
HOOFDLETTERGEVOELIG, en elke opzoeking waarvan de spelling niet byte voor byte
klopt, valt terug op het lezen van de HELE map om een hoofdletterongevoelige
treffer te vinden. Bethesda-spellen vragen om `data/ccbgssse001-fish.bsa` terwijl
het bestand `ccBGSSSE001-Fish.bsa` heet, dus het ging bij vrijwel elke asset af:
4471 markeringspeilingen en 2236 volledige herlezingen van mappen in acht
seconden, en 195796 opsommingen van `Data` in negentig. Skyrim SE bereikte zijn
hoofdmenu nooit - het bleef op 240 MB resident staan terwijl de daemon 92% van
een core verstookte.

Eidos vouwde hoofdletters vanaf het begin in `resolve_read`. De hele kost zat
erin dat het dat nooit zei. `lookup` antwoordt nu `.ciopfs`; `readdir` toont het
nog altijd niet in lijsten.

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

**Dat het spel `plugins.txt` leeg herschreef** was zeer waarschijnlijk hetzelfde
- een `Data` die het binnen geen redelijke tijd kon opsommen, dus concludeerde
het dat er niets was en bewaarde dat. Niet bewezen, en het opnieuw nakijken
waard. Hoe dan ook betekent de capture-bewaking (een capture die de actieve set
volledig leegmaakt, wordt bij elke omvang geweigerd) dat het het profiel niet
meer kan beschadigen.

**`FOPEN_KEEP_CACHE` staat uit.** Opgelost, en het is de moeite waard te weten
waarom. Het liet Skyrim SE crashen op een null-dereferentie, seconden na het
hoofdmenu, deterministisch, met nul mods geïnstalleerd; de andere drie caches aan
kernelkant zijn stuk voor stuk weggebisecteerd en alleen deze deed ertoe. Het
verlies ervan werd destijds als gratis gemeten, maar die meting is gedaan met
`FUSE_PASSTHROUGH` actief, waar de daemon *nul* lezingen bedient
(`EIDOS_FUSE_STATS` meldde `read 0` voor een volledige laadbeurt) en de kernel
die pagina's al tegen het onderliggende bestand cachete. Passthrough staat nu
standaard uit (hieronder), dus dat argument geldt niet meer en de echte kost is
ongemeten - de crash is hoe dan ook reden genoeg om het uit te laten. Zet het met
`EIDOS_FUSE_KEEP_CACHE=1` weer aan om te onderzoeken; de twee vlaggen zijn niet
langer verstrengeld, dus het kan nu op zichzelf getest worden.

### FUSE-passthrough belet het spel enige modinhoud te laden

Opgelost door het uit te zetten; `EIDOS_FUSE_PASSTHROUGH=1` brengt het terug. Met
passthrough aan slaagt Skyrim SE er niet in 152 van zijn eigen bestanden (75
`.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`) te openen, met
`STATUS_ACCESS_VIOLATION`, tegenover 0 met passthrough uit, op kernel 7.1.4 - dus
laadt er stilzwijgend geen modinhoud. De kernel werpt de fout op nadat de daemon
`opened_passthrough` geantwoord heeft, dus de eigen logs van de daemon tonen een
schone draaibeurt (nul mislukte opens, nul geweigerde onderliggende bestanden).
De grondoorzaak in het kernelpad is niet vastgesteld; de schakelaar blijft
bestaan zodat het opnieuw getest kan worden, en zodat passthrough tot alleen
DLL's beperkt zou kunnen worden mocht image-mapping het nodig blijken te hebben.
