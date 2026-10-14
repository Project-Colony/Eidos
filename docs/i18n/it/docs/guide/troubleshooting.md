<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Risoluzione dei problemi e diagnostica

Tutto per il giorno in cui il gioco vede qualcosa su cui il file system non è
d'accordo: gli interruttori d'ambiente, come leggere i contatori delle operazioni, i
problemi noti con la loro storia e la faccenda del passthrough.

### Diagnosticare il VFS

Esistono due variabili d'ambiente per quando il gioco vede qualcosa su cui il file
system non è d'accordo:

```sh
EIDOS_FUSE_STATS=1                  # contatori delle operazioni, riversati allo smontaggio
EIDOS_FUSE_NO_CACHE=1               # ogni cache lato kernel spenta
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # oppure nominarle una per una
```

È la forma granulare ad aver trovato il crash descritto più sotto: spegnerle tutte e
quattro risponde a "è il caching?", e solo nominarle risponde a "quale". I contatori
rispondono all'altra metà: un caricamento che mostra `read 0` è uno in cui
`FUSE_PASSTHROUGH` ha servito ogni byte nel kernel, quindi tutto ciò che stavi per
ottimizzare sul percorso di lettura è già gratis.

## Montare un'unione a mano

La prima `--layer` vince in caso di conflitto; l'ultima sono i tuoi dati di gioco
intatti. Il montaggio richiede solo `/dev/fuse` e `fusermount3` (niente overlayfs,
niente Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... leggi e scrivi attraverso /mnt/point ...
fusermount3 -u /mnt/point
```

Le scritture atterrano in `--overwrite <dir>` (una cartella temporanea se omesso),
così gli strati stessi restano intatti anche qui.

#### Perché il passthrough è disattivato per impostazione predefinita

Il passthrough consegna al kernel il file di appoggio reale, così le letture saltano
del tutto questo demone. È un guadagno di throughput che qui costa correttezza.
Misurato in A/B su Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, lo stesso
ordine di caricamento di 82 plugin, con l'unica variabile del fatto che il binario
portasse o meno la capability:

| passthrough | fallimenti di `NtCreateFile` con `STATUS_ACCESS_VIOLATION` |
|-------------|-------------------------------------------------------------|
| attivo      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`            |
| spento      | 0                                                           |

Con esso attivo il gioco non apre nessuno dei propri archivi o plugin, cosa che in
gioco si manifesta come mod che semplicemente non ci sono - nessun errore, nessuna
riga di log. Con esso spento lo stesso ordine di caricamento arriva alla partita con
i suoi plugin, archivi e script Papyrus vivi.

Il fallimento è invisibile dall'interno del demone, ed è ciò che l'ha reso costoso da
trovare: la nostra `open` riesce ogni volta e il kernel non rifiuta mai un file di
appoggio (verificato su un'intera sessione fallimentare con `EIDOS_FUSE_TRACE=open`:
zero `open FAILED`, zero `passthrough refused`). L'errore viene prodotto dopo che il
demone ha risposto `opened_passthrough`, quindi nessun log lato demone può vederlo.
Non dipende nemmeno dall'estensione: colpisce archivi e plugin allo stesso modo,
cioè i file che il gioco tiene aperti per tutta la sua esecuzione.

`EIDOS_FUSE_PASSTHROUGH=1` lo riattiva, per misurare cosa porta o per ricollaudare il
meccanismo. Gli avvisi di capability nel lanciatore e nella scheda Diagnostics
compaiono solo quando l'hai chiesta.

Per avviare il gioco stesso attraverso Eidos, imposta la sua opzione di avvio Steam
su:

```
eidos play skyrimse -- %command%
```

Anteponi `WINEDLLOVERRIDES="d3dcompiler_47=n"` se Proton ha bisogno del d3dcompiler
nativo per compilare gli shader; Eidos lo fonde con qualsiasi override di DLL che una
mod porta con sé (loader ENB/ReShade/`.asi`).

### L'indice degli strati è davvero in uso?

L'indice è tutto o niente e viene costruito in silenzio: `LayerStack::new` ottiene o
una mappa completa degli strati in sola lettura o `None`, dopodiché ogni
interrogazione li percorre esattamente come prima. Nulla in un log di sessione
distingue i due casi, quindi uno stack che è silenziosamente ricaduto sul percorso
appare identico a uno che funziona - pagando però il vecchio costo.

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` risolve percorsi reali con e senza indice e confronta le scansioni
delle cartelle. `index_agrees` verifica che i due rispondano LA STESSA cosa, su ogni
percorso e ogni elencazione di un'istanza reale. `listing_cost` misura ciò che la
mappa dei figli uniti risparmia su `readdir`.

`EIDOS_NO_INDEX=1` forza il percorso, per quando la differenza fra le due risposte è
proprio ciò che si sta indagando.

## Problemi noti

### DLSS o la generazione di fotogrammi non fa nulla, in silenzio

Tre cause distinte, ciascuna senza alcun messaggio d'errore: NVAPI non abilitato
nelle opzioni di avvio, schermo intero esclusivo, o un tetto FPS di Reflex ormai
vecchio. L'elenco completo sta in [graphics.it.md](graphics.md).

**Una mod che scrive una cartella in due modi perdeva tutto ciò che stava sotto il
secondo.** Corretto. ext4 tiene distinti `meshes/` e `Meshes/`; la vista unita non
deve, e mod reali forniscono entrambi - XP32 Maximum Skeleton ha le sue animazioni e
il suo file di comportamento FNIS sotto la versione con la maiuscola, i suoi
`character assets` sotto l'altra.

Il resolver prendeva la corrispondenza esatta di maiuscole per ciascun componente del
percorso e vi si impegnava: entrava in `meshes/`, non trovava lì il resto del
percorso e abbandonava L'INTERO STRATO. Ogni file sotto l'altra grafia era invisibile
al gioco - nessun errore, nessun log, niente in alcuna diagnostica. Su un'istanza
reale da 50 strati facevano 74 file.

Un componente che corrisponde è ora un candidato, non una decisione; la maiuscola
esatta viene ancora provata per prima, e solo quando il resto sotto di essa fallisce
la scansione cerca fratelli equivalenti a meno di maiuscole. Le elencazioni avevano
lo stesso difetto una cartella più su e ora leggono ogni cartella equivalente per
strato.

**Il LODGen di DynDOLOD muore lasciando un log vuoto.** Corretto da `dotnet10`; vedi
[tools.md](tools.md). Il sintomo è inconfondibile: `LODGen_SSE_<world>_log.txt` con
un'intestazione di versione, una riga `.NET Version:` e nient'altro, per ogni mondo,
e una finestra che dice soltanto "failed to generate object LOD for one or more
worlds". La causa è il Mono di Wine che risponde al posto di .NET Framework, e
nessuna quantità di .NET Framework installato lo risolve - Proton sostituisce
`mscoree.dll` con un collegamento nel proprio albero a ogni aggiornamento del
prefisso.

**Wine non riusciva a capire che il mount ripiega le maiuscole.** Corretto, ed era
quello che contava.

Non esiste un'API per "questo file system è insensibile alle maiuscole", quindi
`get_dir_case_sensitivity` di Wine fiuta il marcatore che CIOPFS lascia nelle
cartelle che serve. In sua assenza Wine presume SENSIBILE alle maiuscole, e ogni
ricerca la cui grafia non combacia byte per byte ripiega sulla lettura dell'INTERA
cartella per trovare una corrispondenza senza distinzione. I giochi Bethesda chiedono
`data/ccbgssse001-fish.bsa` mentre il file è `ccBGSSSE001-Fish.bsa`, quindi scattava
su quasi ogni risorsa: 4471 sondaggi del marcatore e 2236 riletture complete di
cartella in otto secondi, e 195796 enumerazioni di `Data` in novanta. Skyrim SE non
raggiungeva mai il menu principale - restava a 240 MB residenti mentre il demone
bruciava il 92 % di un core.

Eidos ripiegava le maiuscole in `resolve_read` fin dall'inizio. Tutto il costo veniva
dal non dirlo mai. Ora `lookup` risponde `.ciopfs`; `readdir` continua a non
elencarlo.

Due cose lo hanno reso fatale anziché semplicemente lento. Il costo cresce con la
dimensione della cartella, quindi installare i contenuti Anniversary (`Data` da 37 a
177 file) ha fatto traboccare il vaso. E `opendir` costruiva avidamente l'elenco
unito, che è puro spreco quando Wine apre una cartella solo per fare `stat` su quel
marcatore al suo interno - l'istantanea ora viene presa al primo `readdir`.

Dopo: il menu principale, 2,1 GB residenti, demone allo 0 % di CPU.

`EIDOS_FUSE_TRACE=opendir` è ciò che l'ha trovato, ed è incluso. I contatori delle
operazioni dicono quante; 195796 enumerazioni di una sola cartella sono invisibili
dentro un totale.

**Il gioco che riscriveva `plugins.txt` vuoto** era molto probabilmente la stessa
cosa - un `Data` che non riusciva a enumerare in tempi ragionevoli, da cui concludeva
che lì non ci fosse nulla e salvava quello. Non dimostrato, e vale la pena
riverificarlo. In ogni caso la guardia sulla cattura (una cattura che azzera del
tutto l'insieme attivo viene rifiutata a qualsiasi dimensione) fa sì che non possa
più danneggiare il profilo.

**`FOPEN_KEEP_CACHE` è spento.** Corretto, e vale la pena sapere perché. Faceva
crashare Skyrim SE su una dereferenziazione nulla pochi secondi dopo il menu
principale, in modo deterministico, con zero mod installate; le altre tre cache lato
kernel sono state escluse una per una per bisezione e solo questa contava. Perderla
fu misurato come gratuito all'epoca, ma quella misura fu presa con `FUSE_PASSTHROUGH`
attivo, dove il demone serve *zero* letture (`EIDOS_FUSE_STATS` riportava `read 0`
per un caricamento completo) e il kernel già metteva in cache quelle pagine contro il
file di appoggio. Il passthrough ora è spento per impostazione predefinita (sotto),
quindi quell'argomento non vale più e il costo reale non è misurato - il crash basta
comunque a lasciarlo spento. Riattiva con `EIDOS_FUSE_KEEP_CACHE=1` per indagare; i
due flag non sono più intrecciati, quindi ora può essere provato da solo.

### Il passthrough FUSE impedisce al gioco di caricare qualsiasi contenuto delle mod

Corretto spegnendolo; `EIDOS_FUSE_PASSTHROUGH=1` lo riporta. Con il passthrough
attivo, Skyrim SE non riesce ad aprire 152 dei propri file (75 `.bsa`, 65 `.esl`, 10
`.esm`, 2 `.esp`) con `STATUS_ACCESS_VIOLATION`, contro 0 con esso spento, su kernel
7.1.4 - quindi nessun contenuto delle mod si carica, in silenzio. Il kernel solleva
l'errore dopo che il demone ha risposto `opened_passthrough`, così i log del demone
mostrano un'esecuzione pulita (zero aperture fallite, zero file di appoggio
rifiutati). La causa radice nel percorso del kernel non è accertata; l'interruttore è
mantenuto perché si possa ricollaudare, e perché il passthrough possa essere
ristretto alle sole DLL se dovesse risultare che la mappatura delle immagini ne ha
bisogno.
