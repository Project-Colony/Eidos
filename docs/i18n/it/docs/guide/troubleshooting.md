<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c365b34dbc69a832b1c9305498c94eb223b567a1 -->

# Risoluzione dei problemi e diagnostica

Gli interruttori d'ambiente, come leggere i contatori delle operazioni, le schede
Health della scheda Diagnostics, come trovare la mod dietro un crash, cosa
allegare a una segnalazione di bug e i problemi noti con la loro storia.

## Diagnosticare il VFS

Due variabili d'ambiente coprono la maggior parte dei casi in cui il gioco vede
qualcosa su cui il file system non è d'accordo:

```sh
EIDOS_FUSE_STATS=1                  # contatori delle operazioni, riversati allo smontaggio
EIDOS_FUSE_NO_CACHE=1               # ogni cache lato kernel spenta
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # oppure nominarle una per una
```

È la forma granulare ad aver trovato il crash di `FOPEN_KEEP_CACHE` descritto nei
problemi noti: spegnerle tutte e quattro risponde a "è il caching?", e solo
nominarle risponde a "quale". I contatori rispondono all'altra metà. Con il
passthrough attivo (è disattivato per impostazione predefinita), un caricamento
che mostra `read 0` è uno in cui `FUSE_PASSTHROUGH` ha servito ogni byte nel
kernel, quindi tutto ciò che stavi per ottimizzare sul percorso di lettura è già
gratis.

Alcuni interruttori in più restringono ancora il guasto:

```sh
EIDOS_FUSE_TRACE=open,opendir       # registra ogni open / opendir (1 = ogni canale)
EIDOS_FUSE_THREADS=1                # un solo thread del demone (predefinito: numero di CPU, al massimo 4)
EIDOS_FUSE_OPENDIR=1                # tieni gli handle di opendir anche quando il kernel può farne a meno
EIDOS_NO_INDEX=1                    # percorri gli strati invece di usare l'indice degli strati
EIDOS_NO_OW_INDEX=1                 # lo stesso per l'indice di appartenenza dell'Overwrite
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE di nuovo attivo, vedi Problemi noti
```

Per un avvio da Steam, mettili prima del comando nell'opzione di avvio, per
esempio `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. L'output va nel log
dell'esecuzione: il `logs/run-*.log` dell'istanza per un avvio dalla finestra, la
console di Steam per un avvio da Steam.

Per guardare la vista unita stessa, `eidos shell <instance>` apre una shell nella
cartella del gioco così come la vede il gioco, e
`eidos shell <instance> -- ls Data/meshes` esegue lì un solo comando; tutto ciò che
viene scritto finisce nell'Overwrite
([usage.md](usage.md#una-shell-nella-vista-unita)). Il pulsante Save tree della
scheda Data scrive l'intero albero `Data` unito in un file di testo, un file per
riga con la mod da cui proviene, con la tua cartella home nascosta.

## Montare un'unione a mano

`eidos-fuse` arriva con il pacchetto Arch e con le compilazioni dai sorgenti:
`just build` lo compila, e `just install` o `install.sh` lo copiano quando c'è. Il
pacchetto di rilascio ed `eidos-linux` non lo contengono. La sua riga d'uso:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

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

## Perché il passthrough è disattivato per impostazione predefinita

Il passthrough consegna al kernel il file di appoggio reale, così le letture
saltano del tutto questo demone. L'A/B qui sotto è stato misurato con una
versione precedente del demone, prima della modifica descritta più in basso nel
problema noto «Il passthrough FUSE impedisce al gioco di caricare qualsiasi
contenuto delle mod». Quella modifica evita la causa più probabile, ma non è
ancora stata rimisurata in un gioco, quindi il passthrough resta disattivato per
impostazione predefinita finché non lo sarà. Misurato in A/B su Skyrim SE
1.6.1170, proton-cachyos 11.0, kernel 7.1.4, lo stesso ordine di caricamento di
82 plugin, con l'unica variabile del fatto che il binario portasse o meno la
capability:

| passthrough | fallimenti di `NtCreateFile` con `STATUS_ACCESS_VIOLATION` |
|-------------|-------------------------------------------------------------|
| attivo      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`            |
| spento      | 0                                                           |

Con esso attivo il gioco non apriva nessuno dei propri archivi o plugin, cosa
che in gioco si manifestava come mod assenti - nessun errore, nessuna riga di
log. Con esso spento lo stesso ordine di caricamento arrivava alla partita con i
suoi plugin, archivi e script Papyrus vivi.

Il fallimento era invisibile dall'interno del demone: la sua `open` riusciva ogni
volta e il kernel non rifiutava mai un file di appoggio (verificato su un'intera
sessione fallimentare con `EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero
`passthrough refused`). L'errore veniva prodotto dopo che il demone aveva
risposto `opened_passthrough`, quindi nessun log lato demone poteva vederlo. Non
dipendeva nemmeno dall'estensione: colpiva archivi e plugin allo stesso modo,
cioè i file che il gioco tiene aperti per tutta la sua esecuzione.

`EIDOS_FUSE_PASSTHROUGH=1` lo riattiva, per misurare cosa porta o per ricollaudare
il meccanismo. Gli avvisi di capability nel lanciatore e nella scheda Diagnostics
compaiono solo quando l'hai chiesto.

## L'indice degli strati è davvero in uso?

`LayerStack::new` costruisce una mappa completa degli strati in sola lettura
oppure nessuna. Senza mappa, ogni interrogazione percorre gli strati come prima:
le risposte sono le stesse, caricare un salvataggio richiede circa tre volte il
tempo e nient'altro appare diverso.

Quando l'indice è spento, il mount ne dice il motivo, una volta, all'avvio:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

Il motivo indica una directory illeggibile, uno strato profondo più di 64
directory, più di 4.000.000 di voci oppure `EIDOS_NO_INDEX`. La riga finisce nel
log dell'esecuzione: il `logs/run-*.log` dell'istanza per un avvio dalla finestra,
la console di Steam per un avvio da Steam. Con `EIDOS_FUSE_STATS=1`, il riepilogo
allo smontaggio contiene anche una riga `layer index:`, e un contatore
`unindexed` diverso da zero su un mount normale significa che l'indice era spento.

Gli esempi qui sotto verificano la stessa cosa offline, sulle cartelle di
un'istanza, senza mount:

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

## Avviare il gioco con la CLI

Per avviare il gioco attraverso la CLI `eidos` invece che dalla finestra, imposta la
sua opzione di avvio Steam sul percorso assoluto di `eidos`, per esempio:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Non serve alcun prefisso `WINEDLLOVERRIDES`. Quando una mod abilitata importa
`d3dcompiler_47`, Eidos distribuisce la propria copia inclusa nel prefisso, la
forza come nativa e fonde questo con qualsiasi override di DLL che una mod porta
con sé (loader ENB/ReShade/`.asi`).

Eidos non avvia mai il gioco da solo attraverso Proton. `eidos tool <instance> run
<title>` con una voce che avvia il gioco (il loader dello script extender, il
lanciatore o il binario del gioco) chiede allo store che possiede la copia di
avviarlo, come fa Run in una finestra che nessuno store ha aperto, e stampa cosa
impostare quando l'impostazione dello store non avvia Eidos. `--print` stampa il
comando dello store invece di eseguirlo.

## Un gioco o uno strumento che non si chiude

Mentre un gioco o uno strumento è in esecuzione, il pulsante Run diventa Stop, e
anche la scheda di blocco ha un pulsante Stop. Premilo due volte. La seconda
pressione chiude ogni programma avviato dall'esecuzione: prima SIGTERM, poi
SIGKILL dopo 10 secondi per quello che resta. Ciò che il gioco non aveva salvato
va perso. Eidos fa poi quello che fa dopo ogni uscita: le modifiche agli INI e ai
plugin tornano nel profilo, l'output degli strumenti viene registrato e le mod
vengono smontate. L'esecuzione viene registrata con codice di uscita 143
qualunque cosa il programma abbia restituito, quindi `eidos sessions <instance>
diff` non la prende mai come ultima esecuzione riuscita. Unlock è diverso:
restituisce la finestra e lascia il programma in esecuzione. Da un terminale,
`kill <pid>` sul processo `eidos` (il pid è nella scheda di blocco) fa lo stesso
di Stop.

## Trovare cosa si è rotto

Per trovare quale modifica ha rotto il gioco:

- `eidos sessions <instance> diff` elenca cosa è cambiato dall'ultima esecuzione
  del gioco terminata con codice 0: mod, versioni, ordine dei plugin, INI e
  l'eseguibile del gioco
  ([usage.md](usage.md#cosa-è-cambiato-dallultima-esecuzione-riuscita)).
- Test run, accanto a Run, gioca con le mod del profilo, senza mod, oppure con le
  mod selezionate che sono attive, più i master di cui hanno bisogno, e manda tutto ciò che il gioco
  scrive in `<instance>/test-run/`. Dopo l'esecuzione tieni o butti ogni gruppo di
  file: salvataggi, modifiche agli INI, stato dei plugin, output dell'Overwrite e
  log ([usage.md](usage.md#esecuzioni-di-prova)).
- Find the mod behind a problem, nella scheda Diagnostics, esegue prove con metà
  dei sospetti rimasti ogni volta e chiede se il problema si è presentato. 200
  sospetti richiedono circa 8 esecuzioni, e la ricerca sopravvive alla chiusura di
  Eidos ([usage.md](usage.md#trovare-la-mod-dietro-un-problema)).

Un'esecuzione interrotta non precompila mai un Sì in quella ricerca, perché viene
registrata con codice di uscita 143 (vedi sopra).

## Schede Health

La scheda Diagnostics mostra una scheda Health per ogni problema che trova, con la
sua causa e, per alcune, un pulsante che lo corregge. Le schede dell'opzione di
avvio sono descritte in [install.md](install.md), quelle di NVAPI, schermo intero
ed ENB in [graphics.md](graphics.md), e quelle di Proton e del prefisso nella
sezione successiva. `eidos check <instance>` stampa i riscontri che non richiedono
la finestra ([usage.md](usage.md#controlli-di-salute-dal-terminale) li elenca),
tra cui le schede del prefisso, del registro e dell'unità `S:`. Solo la finestra
mostra la scheda Proton con le sue varianti Flatpak, build sconosciuta e build più
vecchia, l'ordine delle patch, gli script dell'extender, gli archivi che nessun
plugin carica, i requisiti di Nexus, le copie di plugin non aggiornate e la
scheda Display.

### Il sistema

- "vm.max_map_count is <value>" compare sotto 1048576, il valore predefinito su
  Arch Linux, Fedora 39 e successive e Ubuntu 24.04 e successive. Alcuni giochi
  sotto Proton mappano più regioni di memoria e poi vanno in crash senza errore.
  Metti `vm.max_map_count = 1048576` in un file sotto `/etc/sysctl.d`, per esempio
  `80-gamecompatibility.conf`, poi esegui `sudo sysctl --system`.
- "The open files limit is <value>" compare quando il limite hard è sotto 524288,
  che serve all'esync di Proton; sotto quel valore i giochi possono non avviarsi o
  andare in crash. systemd 240 e successivi lo impostano. Su una configurazione
  più vecchia, imposta `DefaultLimitNOFILE=524288` in `/etc/systemd/system.conf` e
  `/etc/systemd/user.conf`, poi accedi di nuovo.
- "Game files on a Windows file system" nomina la cartella del gioco, il prefisso
  o la radice dell'istanza quando sta su `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g,
  exfat-fuse), `exfat` o `vfat`. Proton ha bisogno di link simbolici, permessi
  Unix e nomi sensibili alle maiuscole. Sposta il gioco, il suo prefisso e
  l'istanza su un file system Linux come ext4, btrfs o xfs.

### I file di gioco

La scheda Game files in cima alla scheda mostra se l'istanza usa l'installazione
dello store o una copia del gioco dentro l'istanza
([usage.md](usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia)). Gli
avvii vengono rifiutati, con una scheda che dice perché, in tre casi:

- "The game copy '<folder>' is missing": l'istanza usa una copia che non c'è più.
  Rimettila a posto, oppure torna alla versione dello store.
- "'<folder>' has no <binary>": la cartella che l'istanza usa non contiene alcun
  eseguibile del gioco, quindi non è una copia del gioco.
- "<game> cannot run from a copy yet": Morrowind, e Oblivion con
  `bUseMyGamesDirectory=0`, tengono la loro lista dei plugin nella cartella del
  gioco.

"ModOrganizer.ini points tools at another game folder" compare quando il
`ModOrganizer.ini` di una lista, non scritto da Eidos, ha un `gamePath` diverso dai
file di gioco che l'istanza usa. Gli strumenti in modalità Mod Organizer 2, come
PGPatcher, leggono il gioco da lì. Cambia `gamePath` in quel file, oppure rimuovi
il file perché Eidos scriva il suo.

### L'ordine di caricamento

- "sTestFile lines make the game ignore plugins.txt" compare per Fallout 4,
  Fallout 4 VR e Starfield quando il `Fallout4Custom.ini` o lo
  `StarfieldCustom.ini` del profilo ha righe da `sTestFile1` a `sTestFile10` in
  `[General]`. Finché una è impostata, il gioco carica quei plugin e l'ordine di
  caricamento che Eidos scrive non ha effetto. Fix rimuove le righe dalla copia
  del profilo e tiene il file precedente come `<ini>.pre-fix`.
- Starfield: i plugin `.esp` abilitati senza il flag master ricevono una scheda,
  perché Starfield non sa ordinarli tra gli `.esm` e i plugin con flag master e
  tiene caricati per tutta la sessione alcuni dei loro record. Una versione `.esm`
  dell'autore evita entrambi i problemi.
- Starfield: "Invalid blueprint plugins" elenca i plugin che hanno alcuni ma non
  tutti gli elementi del flag blueprint, un nome che inizia con `BlueprintShips-`
  e l'estensione `.esm`. "Blueprint plugins without their main plugin" elenca
  quelli il cui plugin principale non è abilitato: `BlueprintShips-Example.esm` ha
  bisogno di `Example.esm`, `.esp` o `.esl`.
- "<patch> loses <n> file(s) to <master>" compare quando un plugin in una mod ha
  bisogno di un master di un'altra mod, e quell'altra mod ha la priorità più alta,
  quindi vincono le sue copie dei file che entrambe portano. Una patch di solito
  va sotto la mod che modifica, e il pulsante Move della scheda ce la mette. Tieni
  l'ordine se la patch deve perdere quei file.
- "Out-of-date copy of <plugin>" compare quando una mod contiene una copia pulita
  o con flag light di un plugin il cui originale è cambiato da allora (un
  aggiornamento di una mod o del gioco). Il gioco carica la vecchia copia.
  Nascondila, poi pulisci o marca di nuovo il nuovo plugin.

### L'elenco delle mod

- "Order rules of the collection <name> that no longer hold" compare quando le mod
  di una collezione installata non stanno più nell'ordine che le sue regole
  chiedono. Restore collection order sposta indietro solo le mod di quella
  collezione, e Ctrl+Z lo annulla.
- "<mod> requires <mod>, which is not installed" (o "which is disabled") viene dai
  requisiti di Nexus che legge Check for updates. Open on Nexus va alla pagina
  della mod richiesta, Download main file la scarica con un account premium, e
  Ignore nasconde un requisito che un'altra mod già soddisfa. Un requisito DLC è
  solo un consiglio: Eidos cerca un plugin con il nome del DLC.
- "fontconfig.txt names a font library that is missing" (motori Skyrim) nomina i
  font che nessuno strato e nessun archivio contiene, e la mod il cui
  `Interface/fontconfig.txt` vince. Il testo dei menu può comparire come
  riquadri finché il font non viene installato o la riga rimossa.
- "BA2 archive stores audio compressed" (Fallout 4 e Fallout 4 VR) conta i file
  `.wav` e `.xwm` che il BA2 di una mod conserva compressi. Il gioco può andare in
  crash quando li riproduce. Reimpacchetta l'archivio con l'audio non compresso.

### Lo script extender e gli aggiornamenti del gioco

- "<loader> is not installed" compare quando sono installati plugin dello script
  extender (in `SKSE/Plugins`, `F4SE/Plugins` e così via) ma il loader non è in
  nessuna mod attiva né nella cartella del gioco. Il gioco parte senza di essi.
- "The script extender does not match game version <version>" compare quando il
  loader è installato ma manca la DLL di runtime per la versione del gioco in
  esecuzione, per esempio `skse64_1_6_1170.dll`. Il loader si ferma con un errore.
  Di solito succede dopo un aggiornamento del gioco. Questa scheda e quella qui
  sopra hanno un pulsante Open the download page per il sito dell'extender e un
  pulsante Install the downloaded archive, che mette il loader nella cartella
  `Root/` della mod.
- "<game> changed since the last clean launch" compare quando l'eseguibile del
  gioco, o per una copia Steam il suo build id, è cambiato dall'ultimo avvio
  tramite Eidos terminato senza errore. Un aggiornamento Steam che lascia
  l'eseguibile com'era non lo fa comparire. I plugin dello script extender
  costruiti per la versione vecchia possono smettere di caricarsi, e le schede
  sotto di essa nominano quelli che la dichiarano. Per mantenere la versione del
  gioco su cui è stata costruita una lista, congelala nell'istanza con Freeze
  this version nella scheda Game files, oppure con
  `eidos stock-game <instance> freeze --yes` dopo che `freeze` da solo ha
  mostrato la dimensione: gli aggiornamenti successivi dello store cambiano
  l'installazione dello store, non la copia.
- "<mod> replaces <n> <extender> script(s)" compare quando una mod con priorità
  più alta dello script extender ha copie sciolte `.pex` degli script dell'extender
  stesso. Una copia vecchia rompe le funzioni che dichiara, e il gioco non mostra
  alcun errore. Sposta l'extender sotto quella mod (il pulsante Move della scheda
  lo fa quando entrambi sono mod), oppure nascondi quei file nella mod. Quando gli
  script dell'extender stanno sciolti nel `Data` del gioco, installa l'extender
  come mod così può essere ordinato. Gli archivi non vengono controllati, solo i
  file sciolti.

### Output degli strumenti

- "Overwrite/Root holds meshes" (o textures) significa che uno strumento il cui
  percorso dati puntava alla cartella del gioco ha scritto lì quelle cartelle,
  dove il gioco non le legge mai. Correggi il percorso dello strumento, elimina le
  cartelle ed esegui di nuovo lo strumento. Il pulsante Open folder apre
  `Overwrite/Root`.
- "BodySlide's GameDataPath is not the game's Data folder" (o `OutputDataPath`):
  vedi [Strumenti](tools.md#le-impostazioni-di-uno-strumento-restano-affar-suo).

### Un crash nell'ultima sessione

"The game crashed in the last session" compare quando un crash logger ha scritto
un log durante l'ultimo avvio. La scheda nomina l'eccezione e le DLL e i plugin
del log che le tue mod o l'Overwrite installano. Trattali come indizi: una mod
nominata in un log di crash è spesso il punto in cui il crash si è visto, non la
sua causa. Quando il log non nomina nessun file delle tue mod, la scheda lo dice.
Open the log lo apre, e Copy the log (home path hidden) lo copia con la tua
cartella home nascosta.

### La dimensione della finestra

Per Skyrim SE, Enderal SE e Fallout 4, una scheda Display mostra la dimensione e
la modalità della finestra con cui il profilo avvia il gioco, e dice "No window
size set for this profile" quando né l'INI Prefs del profilo né quello del
prefisso ne impostano una. Apply scrive la dimensione e la modalità nella copia
del profilo, e in una copia dell'INI di SSE Display Tweaks nell'Overwrite quando è
quella mod a impostare la finestra ([usage.md](usage.md#diagnostica)).

## Quale Proton esegue il gioco, e un prefisso rotto

La scheda Diagnostics ha una scheda Proton: la versione che Eidos e Steam usano
per il gioco, l'impostazione di Steam stesso e la versione che ha eseguito per
ultima il prefisso. Quando un Proton più recente di quello selezionato è stato
l'ultimo a preparare il prefisso, la scheda diventa un consiglio, perché un Proton
più vecchio su un prefisso più recente può reimpostarne i file di Wine o non
avviarsi. Scegli in Steam una versione almeno altrettanto recente, in Proprietà
del gioco, Compatibilità, oppure ricostruisci il prefisso.

Le altre schede di Proton e del prefisso:

- "Proton comes from the Flatpak Steam install": quel Proton tiene il proprio
  runtime e le librerie steamclient dentro la sandbox Flatpak, quindi eseguirlo
  dall'host può fallire. Eidos non lo avvia tramite `flatpak run`, che metterebbe
  il gioco nella sandbox, fuori dalla vista unita. Installa un Proton in
  `~/.steam/root/compatibilitytools.d/` e selezionalo per il gioco.
- "Eidos cannot tell which Proton runs this game": il `config.vdf` di Steam non
  nomina alcuno strumento di compatibilità per esso, e il prefisso non registra
  nessuna build installata. Gli strumenti non partono finché non ne viene scelto
  uno in Steam (Proprietà del gioco, Compatibilità).
- "<game> has no Proton prefix yet": avvia il gioco una volta tramite Steam, poi
  premi F5. Se Steam non crea il prefisso, controlla che la cartella del gioco
  sotto `steamapps/compatdata` sia scrivibile; una vuota di proprietà di root lo
  blocca senza messaggio.
- "Windows tools cannot find <game>": la registrazione del percorso
  d'installazione nel prefisso è sbagliata o manca, e la leggono xEdit, DynDOLOD,
  Wrye Bash e il Creation Kit. Eidos scrive il percorso giusto la prossima volta
  che esegui uno strumento.
- "The prefix's S: drive points outside the Steam library": `S:` non coincide né
  con la radice della libreria, dove Eidos lo punta per uno strumento, né con
  `<library>/steamapps`, dove Steam lo punta per il gioco. La libreria
  probabilmente è stata spostata, oppure il prefisso viene da un'altra macchina.
  Ogni percorso relativo a `S:` memorizzato nel prefisso non porta da nessuna
  parte finché il prossimo avvio tramite Eidos o Steam non lo punta di nuovo.
- "<store> installation: started through its store" è la scheda di una copia GOG
  o Epic. Run chiede a Heroic o Legendary di avviare il gioco quando Eidos è il
  wrapper del gioco lì, e gli strumenti girano con la build di Wine o Proton che
  Heroic registra per la copia.

Il menu File esegue winecfg, regedit, l'esplora risorse di Wine e il programma di
disinstallazione nel prefisso del gioco, con il Proton che esegue gli strumenti
del gioco ([tools.md](tools.md#i-programmi-di-wine-nel-prefisso-del-gioco)). Le
loro righe restano disattivate finché il prefisso non ha quei programmi.

Il pulsante Rebuild prefix sulla scheda Proton serve per un prefisso che non
funziona più. Premilo due volte. Eidos non elimina nulla: rinomina
`steamapps/compatdata/<appid>` in `<appid>.eidos-backup-<date>-<time>` nella
stessa cartella, e Steam crea un nuovo prefisso la prossima volta che avvii il
gioco da Steam. Si rifiuta finché un gioco, uno strumento o un qualsiasi
programma Wine usa il prefisso. Finché il nuovo prefisso non esiste, Run nella
finestra non può passare il gioco a Steam e mostra invece l'opzione di avvio,
quindi premi Gioca in Steam. I salvataggi e le impostazioni tenuti solo nel
vecchio prefisso restano nella cartella rinominata: ricopia ciò che ti serve, ed
elimina quella cartella tu stesso quando non la vuoi più.

## Cosa allegare a una segnalazione di bug

- Copy report, nella scheda Diagnostics, copia la versione di Eidos, il gioco, la
  build di Proton, se Steam è il Flatpak, lo stato del passthrough, il tipo di
  sessione, la distribuzione, il kernel, `vm.max_map_count`, il limite dei file
  aperti, se `/dev/fuse` e `fusermount3` ci sono, i nomi delle variabili di avvio
  e dei programmi wrapper del profilo, e ogni scheda Health. La tua cartella home
  compare come `~` e il tuo nome di accesso come `<user>`.
  `eidos check <instance>` stampa la stessa intestazione in un terminale, e
  `--json` la dà come JSON.
- Il log dell'esecuzione, `<instance>/logs/run-*.log`, comincia con il comando
  esatto che Eidos ha eseguito. File, Eidos logs apre la cartella dei log propri
  di Eidos, `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), sulla scheda del crash, copia il log del crash.
- Save tree, nella scheda Data, salva l'albero `Data` unito con la provenienza di
  ogni file.

## Problemi noti

### DLSS o la generazione di fotogrammi non fa nulla, in silenzio

Tre cause distinte, ciascuna senza alcun messaggio d'errore: NVAPI non abilitato
nelle opzioni di avvio, schermo intero esclusivo, o un tetto FPS di Reflex ormai
vecchio. L'elenco completo sta in [graphics.md](graphics.md).

### Run non fa nulla con Steam Flatpak

In una finestra che nessuno store ha aperto, Run chiede a Steam di avviare il
gioco eseguendo `steam -applaunch <appid>`. Steam Flatpak non mette nessun `steam`
nel `PATH` dell'host, quindi la riga di stato dice "Could not start Steam" con
l'errore. Premi invece Gioca in Steam.

### Un salvataggio da un altro dispositivo non compare

Un salvataggio che arriva nel prefisso senza passare da Eidos, come un download
da Steam Cloud da uno Steam Deck, resta nascosto mentre i salvataggi del profilo
sono montati. Nulla lo elimina. La scheda Saves conta questi salvataggi sotto la
sua intestazione, e Import li copia nel profilo senza sostituire un salvataggio
del profilo ([usage.md](usage.md#i-salvataggi-appartengono-al-profilo)).

### Una mod che scrive una cartella in due modi perdeva tutto ciò che stava sotto il secondo

Corretto. ext4 tiene distinti `meshes/` e `Meshes/`; la vista unita non deve, e
mod reali forniscono entrambi - XP32 Maximum Skeleton ha le sue animazioni e il
suo file di comportamento FNIS sotto la versione con la maiuscola, i suoi
`character assets` sotto l'altra.

Il resolver prendeva la corrispondenza esatta di maiuscole per ciascun componente
del percorso e vi si impegnava: entrava in `meshes/`, non trovava lì il resto del
percorso e abbandonava L'INTERO STRATO. Ogni file sotto l'altra grafia era
invisibile al gioco - nessun errore, nessun log, niente in alcuna diagnostica. Su
un'istanza reale da 50 strati facevano 74 file.

Un componente che corrisponde è ora un candidato, non una decisione; la maiuscola
esatta viene ancora provata per prima, e solo quando il resto sotto di essa
fallisce la scansione cerca fratelli equivalenti a meno di maiuscole. Le
elencazioni avevano lo stesso difetto una cartella più su e ora leggono ogni
cartella equivalente per strato.

L'indice dei percorsi non ha mai avuto questo bug, perché percorre ogni cartella
che trova. Quindi per un certo periodo l'indice restituiva file che il ripiego non
riusciva a trovare, benché il ripiego sia la risposta pensata per non sbagliare
mai.

### Il LODGen di DynDOLOD muore lasciando un log vuoto

Corretto da `dotnet10`; vedi [tools.md](tools.md). Il
`LODGen_SSE_<world>_log.txt` di ogni mondo contiene un'intestazione di versione,
una riga `.NET Version:` e nient'altro, e una finestra dice soltanto "failed to
generate object LOD for one or more worlds". La causa è il Mono di Wine che
risponde al posto di .NET Framework, e nessuna quantità di .NET Framework
installato lo risolve - Proton sostituisce `mscoree.dll` con un collegamento nel
proprio albero a ogni aggiornamento del prefisso. Il pacchetto di rilascio include
il runtime .NET 10; le altre installazioni lo scaricano una volta.

### Wine non riusciva a capire che il mount ripiega le maiuscole

Corretto.

`get_dir_case_sensitivity` di Wine interroga prima la cartella con
`FS_IOC_GETFLAGS`, poi fiuta il marcatore che CIOPFS lascia nelle cartelle che
serve. In assenza di entrambi Wine presume SENSIBILE alle maiuscole, e ogni
ricerca la cui grafia non combacia byte per byte ripiega sulla lettura
dell'INTERA cartella per trovare una corrispondenza senza distinzione. I giochi
Bethesda chiedono `data/ccbgssse001-fish.bsa` mentre il file è
`ccBGSSSE001-Fish.bsa`, quindi scattava su quasi ogni risorsa: 4471 sondaggi del
marcatore e 2236 riletture complete di cartella in otto secondi, e 195796
enumerazioni di `Data` in novanta. Skyrim SE non raggiungeva mai il menu
principale - restava a 240 MB residenti mentre il demone bruciava il 92 % di un
core.

Eidos ripiegava le maiuscole in `resolve_read` fin dall'inizio. Tutto il costo
veniva dal non dirlo mai. Ora `lookup` risponde `.ciopfs`; `readdir` continua a
non elencarlo. `ioctl` risponde anche a `FS_IOC_GETFLAGS` sulle cartelle con il
flag casefold, che il Wine attuale chiede prima di cercare il marcatore.

Due cose lo hanno reso fatale anziché semplicemente lento. Il costo cresce con la
dimensione della cartella, quindi installare i contenuti Anniversary (`Data` da 37
a 177 file) ha fatto traboccare il vaso. E `opendir` costruiva avidamente l'elenco
unito, che è puro spreco quando Wine apre una cartella solo per fare `stat` su quel
marcatore al suo interno - l'istantanea ora viene presa al primo `readdir`.

Dopo: il menu principale, 2,1 GB residenti, demone allo 0 % di CPU.

`EIDOS_FUSE_TRACE=opendir` è ciò che l'ha trovato, ed è incluso. I contatori delle
operazioni dicono quante; 195796 enumerazioni di una sola cartella sono invisibili
dentro un totale.

### Il gioco ha riscritto `plugins.txt` vuoto

Molto probabilmente la stessa cosa del ripiegamento delle maiuscole qui sopra - un
`Data` che non riusciva a enumerare in tempi ragionevoli, da cui concludeva che lì
non ci fosse nulla e salvava quello. Non dimostrato, e vale la pena riverificarlo.
In ogni caso la guardia sulla cattura (una cattura che azzera del tutto l'insieme
attivo viene rifiutata a qualsiasi dimensione) fa sì che non possa più danneggiare
il profilo.

### `FOPEN_KEEP_CACHE` è spento

Corretto. Faceva crashare Skyrim SE su una dereferenziazione nulla pochi secondi
dopo il menu principale, in modo deterministico, con zero mod installate; le altre
tre cache lato kernel sono state escluse una per una per bisezione e solo questa
contava. Quella bisezione girava con `FUSE_PASSTHROUGH` attivo, e il kernel fa
fallire con EIO un'apertura in passthrough che porta `FOPEN_KEEP_CACHE`, quindi
con il flag acceso ogni apertura in passthrough falliva. Forse erano quelle
aperture fallite a far crashare il gioco, e il flag da solo, con il passthrough
spento, non è mai stato provato. Le risposte in passthrough non lo portano più.
Perderlo fu misurato come gratuito all'epoca, ma con il passthrough attivo il
demone serve *zero* letture (`EIDOS_FUSE_STATS` riportava `read 0` per un
caricamento completo), quindi con il passthrough spento per impostazione
predefinita il costo reale non è misurato. Riattivalo con
`EIDOS_FUSE_KEEP_CACHE=1` per provarlo da solo.

### Il passthrough FUSE impedisce al gioco di caricare qualsiasi contenuto delle mod

Il demone ora evita la causa più probabile (sotto), ma questo non è ancora stato
rimisurato in un gioco, quindi il passthrough resta spento per impostazione
predefinita; `EIDOS_FUSE_PASSTHROUGH=1` lo accende. Prima di questa modifica, con
il passthrough attivo, Skyrim SE non riusciva ad aprire 152 dei propri file con
`STATUS_ACCESS_VIOLATION`, quindi nessun contenuto delle mod si caricava, in
silenzio. La misura e il motivo per cui i log del demone mostravano
un'esecuzione pulita sono sotto "Perché il passthrough è disattivato per
impostazione predefinita" qui sopra.

Il kernel lega un solo file di appoggio a ogni inode (`fs/fuse/iomode.c`).
Un'apertura che porta un secondo identificativo di appoggio per un inode che ne
ha già uno fallisce con EIO, e così un'apertura normale (senza
`FOPEN_PASSTHROUGH`) di un inode che ha un file di appoggio. Anche un'apertura in
passthrough che porta `FOPEN_KEEP_CACHE` fallisce con EIO. Il demone registrava
un identificativo nuovo a ogni apertura, quindi falliva la seconda apertura di un
file che il gioco teneva già aperto. Wine riporta EIO come
`STATUS_ACCESS_VIOLATION`, l'errore dietro le 152 aperture fallite. Questa regola
viene dal codice sorgente del kernel e corrisponde a quei 152 fallimenti, il che
la rende la causa più probabile.

Il demone ora registra un solo file di appoggio per inode, condiviso da tutte le
sue aperture in passthrough, e non mette mai `FOPEN_KEEP_CACHE` su un'apertura in
passthrough. Il passthrough riguarda solo le aperture in sola lettura di file
degli strati del gioco e delle mod che pesano 1 MiB o più o finiscono in `.dll`,
`.exe` o `.asi`. Una volta che un file ha un file di appoggio, ogni apertura in
sola lettura lo usa, e finché ha aperture normali anche i nuovi lettori ricevono
aperture normali. Un file nell'Overwrite non passa mai dal passthrough, e un
montaggio che proietta i timestamp dei plugin non lo usa affatto. Uno scrittore
che apre un file mentre lettori in passthrough lo tengono aperto riceve ETXTBSY
(in Wine: `STATUS_SHARING_VIOLATION`) prima di qualsiasi copy-up, così
un'apertura con `O_TRUNC` non può lasciare nell'Overwrite una copia vuota che
nasconda il file della mod.

### Il pulsante Mod Manager Download non fa nulla

Preferences, Nexus, Downloads mostra quale programma riceve i link `nxm://`.
Compare una scheda Health quando quel programma non è Eidos, manca o non ne è
impostato nessuno. Il pulsante Register scrive `eidos-nxm.desktop` e lo imposta
come gestore predefinito di `x-scheme-handler/nxm`. Da terminale,
`eidos nxm --register` fa lo stesso.

Se lo stato indica già Eidos e il pulsante continua a non fare nulla, il browser
potrebbe usare una scelta precedente. Cancella la scelta salvata per i link nxm
nelle impostazioni del browser, così che la richieda di nuovo. Un browser Flatpak
non può avviare i programmi dell'host e passa il link tramite il portale desktop,
che legge il gestore registrato sull'host.
