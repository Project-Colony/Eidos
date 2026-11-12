<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Usare Eidos

Come usare Eidos: istanze e copie del gioco, la riga di comando, le esecuzioni di
prova, la finestra, le collezioni e gli aggiornamenti di Nexus, lo spostamento di
un'istanza su un'altra macchina, strumenti ed estensioni. Per i problemi, vedi
[troubleshooting.md](troubleshooting.md); per la compilazione dai sorgenti e la
struttura del repository, vedi
[../internals/contributing.md](../../../../internals/contributing.md).

## Istanze: globali e portatili

I comandi che accettano un'istanza accettano un identificatore di gioco o la
cartella di un'istanza portatile. `skyrimse` indica l'istanza globale, che Eidos
tiene in `~/.local/share/Colony/Eidos/instances/skyrimse`. Un'istanza portatile è
una cartella autonoma dove vuoi (un secondo disco, una partizione dei giochi), e
puoi spostarla, come con le istanze portatili di MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # crea lì un'istanza portatile
eidos install /mnt/games/EidosSkyrim mod.7z  # ogni comando accetta la cartella
eidos play /mnt/games/EidosSkyrim -- %command%
```

L'`eidos-instance.ini` della cartella nomina il gioco, quindi alla cartella non
serve altro. `EIDOS_INSTANCE=<folder>` nell'ambiente reindirizza un identificatore
di gioco a quella cartella, per esempio in un'opzione di avvio di Steam. Le
istanze portatili che hai creato o aperto vengono ricordate, le più recenti per
prime, in `~/.config/Colony/Eidos/instances.ini`: la schermata di benvenuto della
finestra le elenca, l'avvio da Steam cade su quella che hai giocato per ultima, e il
gestore `nxm://` scarica al suo interno.

Spostare a mano una cartella portatile conserva tutto tranne le voci degli
strumenti che hai registrato con percorsi assoluti nella vecchia posizione.
`eidos pack` ed `eidos unpack` ([sotto](#spostare-unistanza-su-unaltra-macchina))
le correggono; un semplice `mv` no. La cache dei runtime in
`~/.local/share/Colony/Eidos/runtimes/` è condivisa da ogni istanza della
macchina, quindi un host .NET da 78 MB non viene copiato in ciascuna.

Un posto viene rifiutato: dentro la cartella d'installazione di un gioco, dove gli
utenti MO2 mettono spesso la loro istanza. Quell'albero appartiene a Steam, e un
aggiornamento, un "verifica l'integrità" o una disinstallazione possono riscriverlo
o cancellarlo insieme alla tua configurazione. Eidos inoltre monta sopra la radice
del gioco, quindi un'istanza lì dentro starebbe dentro il proprio bersaglio di
montaggio. La procedura guidata, `eidos init`, `eidos play` ed `eidos shell` lo
rifiutano. Metti invece la cartella accanto al gioco; una cartella sorella sullo
stesso disco è altrettanto comoda.

### Dove Eidos tiene i suoi file

Eidos tiene i propri file sotto `Colony/Eidos`, la struttura che usa ogni programma
della famiglia Colony: `~/.config/Colony/Eidos/` per ciò che hai scelto (preferenze,
la tua sessione Nexus, il tuo elenco di istanze, le definizioni di giochi e
componenti aggiuntivi che hai scritto), `~/.local/share/Colony/Eidos/` per le
istanze globali (`instances/`), i runtime scaricati e i log delle sessioni
(`logs/`), e `~/.cache/Colony/Eidos/` per le masterlist di LOOT, che vengono
scaricate di nuovo ogni volta che mancano.

Quando trova file nella struttura di un Eidos più vecchio (`~/.config/eidos/`,
`~/.local/share/eidos/` e `~/.local/state/`), il primo avvio li copia nella nuova e
lo dice nel log. Un'istanza globale viene spostata con hard link invece di una copia
byte per byte, quindi le sue mod non occupano spazio in più, e le sue voci degli
strumenti vengono ripuntate sulla nuova cartella. Nulla nelle vecchie directory
viene modificato o cancellato; rimuovile tu quando la nuova struttura funziona. Se
una copia non si può fare (un disco pieno o in sola lettura, una vecchia cartella
d'istanza su un altro disco, un gioco ancora in esecuzione da essa), Eidos continua
a usare la vecchia posizione e riprova al lancio successivo. Un'opzione di avvio di
Steam che nomina ancora la vecchia cartella dell'istanza apre quella spostata.

Gli hard link risparmiano spazio solo all'inizio. Una mod o un download che rimuovi
o reinstalli dopo mantiene la sua vecchia copia su disco attraverso la vecchia
cartella dell'istanza, e cancellare `~/.local/share/eidos/<game>/` restituisce
quello spazio.

Un'istanza portatile non viene mai spostata: resta dove l'hai messa.

## La riga di comando

```sh
eidos games                                # giochi supportati installati qui, come l'elenco di MO2
eidos init skyrimse [folder]               # crea un'istanza; con una cartella è portatile
eidos init skyrimse --game-path <folder>   # usa la copia installata in <folder> quando ce ne sono più di una
eidos install skyrimse mod.7z [name]       # installa un archivio (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstalla sopra la mod (--merge installa sopra i suoi file)
eidos import skyrimse <mo2-profile>        # riprende l'ordine delle mod e lo stato dei plugin di un profilo MO2
eidos sort skyrimse [--dry-run]            # ordina il caricamento con LOOT (--update-masterlist la aggiorna prima)
eidos play skyrimse                        # mostra cosa verrebbe montato
eidos play skyrimse -- <command>           # esegue <command> con le mod montate sopra il gioco
eidos shell skyrimse                       # una shell nella vista unita; le scritture finiscono in Overwrite
eidos check skyrimse [--json]              # controlli Health senza la finestra
eidos sessions skyrimse                    # le esecuzioni registrate di giochi e strumenti, le più recenti per prime
eidos sessions skyrimse diff [<id>]        # cosa è cambiato dall'ultima esecuzione terminata con codice 0
eidos stock-game skyrimse                  # quali file di gioco usa l'istanza; congela una copia o cambia
eidos tool skyrimse                        # elenca gli strumenti (add, rm e run: vedi Strumenti più sotto)
eidos tool skyrimse run --exe <path>       # esegue un exe una volta attraverso la vista unita
eidos tool skyrimse import-mo2 [--write]   # gli eseguibili di un ModOrganizer.ini; --write li aggiunge
eidos prereqs skyrimse [--install]         # di cosa hanno bisogno gli strumenti; --install recupera ciò che manca
eidos collection skyrimse <link>           # installa una collezione di Nexus (--dry-run, --no-optional)
eidos nexus status                         # controlla l'accesso Nexus memorizzato
eidos nexus update skyrimse [--apply]      # controlla gli aggiornamenti delle mod; --apply li installa
eidos nxm <link>                           # scarica un link Mod Manager Download
eidos nxm --register                       # fa aprire Eidos al pulsante Mod Manager Download del sito
eidos export skyrimse -o mods.csv --active # formato CSV di MO2; --active elenca solo le mod attive
eidos pack skyrimse backup.eidos           # l'intera istanza in un solo file
eidos unpack backup.eidos [folder]         # senza una cartella un'istanza globale torna al suo posto centrale
eidos --version                            # stampa la versione
```

`eidos` senza argomenti stampa l'elenco dei comandi. La maggior parte dei comandi
stampa le proprie opzioni quando manca un argomento (`eidos init` ed `eidos import`
stampano invece l'elenco completo), ed `eidos tool --help` elenca i sottocomandi di
tool. `eidos pack` accetta `--dry-run`, `--level`, `--no-downloads`, `--no-recovery`
e `--force`, ed `eidos unpack` accetta `--info` e `--force`; entrambi sono descritti
[sotto](#spostare-unistanza-su-unaltra-macchina). `eidos stock-game` è descritto
sotto [File di gioco](#file-di-gioco-linstallazione-dello-store-o-una-copia).

### Eseguire un gioco attraverso la vista unita

`eidos play` monta le mod dell'istanza sopra la directory `Data` del gioco dentro
uno spazio dei nomi privato, poi esegue il comando attraverso quella vista. I file
del gioco vengono prima messi da parte con un bind, così il demone li legge ancora
invariati. Le scritture (salvataggi, configurazioni rigenerate) finiscono nel
livello `overwrite/` dell'istanza; l'installazione del gioco e ogni cartella di mod
restano byte per byte com'erano.

Eidos non richiede root. Monta in uno spazio dei nomi privato di utente e di mount,
senza helper setuid, senza servizio di sistema e senza nulla da concedere. Una
capability facoltativa sul binario `eidos` consente il passthrough FUSE, che resta
spento a meno che non sia impostato `EIDOS_FUSE_PASSTHROUGH=1`; vedi
[install.md](install.md#facoltativo-passthrough-fuse). Con la capability Eidos usa
uno spazio dei nomi di mount semplice invece di uno spazio dei nomi utente, e le
mod vengono distribuite allo stesso modo in entrambi.

### Una shell nella vista unita

`eidos shell <instance>` avvia la tua `$SHELL` (o `/bin/sh`) nella cartella del
gioco come la vedrebbe il gioco: ogni mod attiva montata sopra `Data`, le mod Root
sopra la radice del gioco, e tutto ciò che viene scritto lì finisce nell'Overwrite.
`eidos shell <instance> -- <command>` esegue invece un solo comando, per esempio
`eidos shell skyrimse -- ls Data/meshes` o uno strumento nativo come NifSkope.
L'esecuzione tiene il blocco dell'istanza come una sessione di gioco, quindi la
finestra non può salvare modifiche finché non esci, e viene registrata in
`eidos sessions` con il nome `shell`, mai come avvio di un gioco. Ctrl+C su un
comando ferma l'esecuzione come fa Stop: Eidos termina il comando, cattura la
sessione e smonta. Nella shell interattiva Ctrl+C appartiene alla shell, come al
solito; digita `exit` per terminare l'esecuzione.

### Controlli di salute dal terminale

`eidos check <instance>` esegue i controlli Health che non richiedono la finestra,
così possono girare via SSH o in Game Mode ed entrare in una segnalazione di bug:

- il prefisso Proton e il suo registro;
- la diagnostica dei plugin e i master mancanti;
- lo script extender e i suoi plugin;
- un aggiornamento del gioco dall'ultimo avvio pulito;
- l'opzione di avvio di Steam;
- le impostazioni grafiche (NVAPI, schermo intero, ENB);
- il log di crash dell'ultima esecuzione;
- le righe `sTestFile` negli INI del profilo;
- i plugin `.esp` e blueprint di Starfield;
- `vm.max_map_count` e il limite dei file aperti;
- la copia del gioco che l'istanza usa, e un `gamePath` nel `ModOrganizer.ini` di
  una lista che nomina un'altra cartella di gioco;
- le regole d'ordine delle collezioni che non valgono più;
- i file di gioco su NTFS, exFAT o FAT;
- i percorsi dati di BodySlide;
- le cartelle `Data` rimaste in `Overwrite/Root`;
- le librerie di font che `Interface/fontconfig.txt` nomina e nessuna mod fornisce;
- l'audio compresso negli archivi BA2 delle mod di Fallout 4 e Fallout 4 VR.

Alcune schede le calcola solo la finestra, nella scheda Diagnostics: l'ordine delle
patch, gli script dell'extender che una mod sostituisce, gli archivi che nulla
carica, i requisiti di Nexus, i plugin che LOOT segnala come sporchi, la build di
Proton che esegue il gioco, la scheda Display e l'output degli strumenti più
vecchio dei suoi input.

Il comando stampa le stesse righe di sistema di Copy report, poi una riga per ogni
riscontro, con la tua cartella home mostrata come `~` e il tuo nome di accesso come
`<user>`. Il codice di uscita è 0 quando non è stato trovato alcun problema (i soli
consigli danno comunque 0), 1 quando è stato trovato almeno un problema, e 2
quando Eidos non ha potuto controllare: argomenti sbagliati, un'istanza che non
riesce ad aprire o un gioco che non rileva.

`eidos check <instance> --json` stampa invece un solo oggetto JSON. I suoi campi di
primo livello sono `eidos` (la versione), `game`, `instance`, `system` (`proton`,
`flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`,
`fusermount3`, `max_map_count`, `open_files_hard`) e `findings`. `max_map_count` e
`open_files_hard` sono numeri, oppure `null` quando Eidos non ha potuto leggerli.
Ogni riscontro è un `code`, un `level` (`problem` o `advice`) e i suoi `args`; la
scheda Diagnostics mostra la frase per ogni codice.

### Cosa è cambiato dall'ultima esecuzione riuscita

Ogni esecuzione tramite `eidos play`, `eidos shell` o `eidos tool run`, avviata
dalla finestra, da un terminale o da Steam, scrive un record in
`<instance>/logs/sessions/`: il profilo, ogni mod attiva con la sua versione e i
suoi id di file Nexus, l'ordine dei plugin, un checksum di ogni INI del profilo, la
versione dell'eseguibile del gioco, la versione di Eidos, il codice di uscita e i
salvataggi che l'esecuzione ha creato. Vengono tenuti i 50 più recenti, e `logs/`
resta fuori da un'istanza impacchettata. `eidos sessions <instance> diff` confronta
l'esecuzione di gioco più recente terminata con codice 0 con lo stato attuale;
`diff <id>` sceglie un altro record dall'elenco. L'elenco segna come `test` una
[esecuzione di prova](#esecuzioni-di-prova), e un'esecuzione di prova non viene mai
scelta come ultima esecuzione riuscita. Un'esecuzione che hai fermato registra il
codice di uscita 143, quindi non viene scelta nemmeno lei. Eidos non rileva ancora i
crash qui, e un gioco andato in crash sotto Proton può comunque uscire con codice
0: se l'esecuzione che sceglie era una di queste, nomina una precedente con
`diff <id>`.

## L'elenco delle mod

Come in MO2, ogni profilo ha il proprio `modlist.txt` (quali mod sono attive, con
quale priorità), mentre le cartelle delle mod stanno nella `mods/` condivisa
dell'istanza. Una cartella che metti a mano in `mods/` non è ancora elencata:
compare disattivata alla priorità più alta, in fondo all'elenco della finestra come
in MO2, così vince i suoi conflitti una volta attivata.

Poiché le cartelle sono condivise, rimuovere o rinominare una mod cambia ogni
profilo: la sua riga viene tolta dal `modlist.txt` di ogni profilo, o rinominata in
esso, e ogni profilo mantiene il proprio stato e il proprio ordine per il resto. Un
profilo rimasto senza mod non ha affatto un `modlist.txt`.

La finestra lavora su una propria copia dell'elenco delle mod e dell'ordine di
caricamento. Se un altro processo Eidos scrive uno dei due mentre è aperta
(`eidos install` o `eidos sort` in un terminale, una collezione, una sessione avviata
da Steam, una seconda finestra), la modifica successiva della finestra viene
rifiutata invece di sovrascrivere quel lavoro. Ricarica dal disco e dice *Not saved:
another Eidos process changed the mod list. It has been reloaded - redo the change.*
(o lo stesso per l'ordine di caricamento); si perde solo quel clic, quindi rifallo.
Rename, New mod, Add separator, Remove e la trasformazione dell'Overwrite in una mod
controllano prima di toccare `mods/`, così una modifica rifiutata non cambia nulla
su disco. Anche l'editor INI si rifiuta di salvare sopra un file cambiato da
quando l'ha aperto.

## I salvataggi appartengono al profilo

Ogni profilo tiene i propri salvataggi in `profiles/<name>/saves/`, e per tutta la
durata di un'esecuzione quella cartella viene montata sopra la cartella dei
salvataggi del gioco nel prefisso Proton. Il primo avvio di un profilo copia in esso
i salvataggi già presenti nel prefisso, una volta sola, così un salvataggio che
cancelli in seguito dal profilo resta cancellato. Dopo ogni esecuzione i
salvataggi più recenti del profilo vengono copiati nel prefisso per Steam Cloud; un
salvataggio del prefisso che Eidos non ha messo lì viene prima conservato nel
profilo come `orphan-<time>-<name>`, mai sovrascritto.

Un salvataggio che arriva nel prefisso dopo quella prima copia senza passare da
Eidos (un download da Steam Cloud da uno Steam Deck, un avvio senza Eidos) resta
nascosto mentre i salvataggi del profilo sono montati. Nulla lo elimina: l'avvio lo
nomina nel log dell'esecuzione (il `logs/run-*.log` dell'istanza per un avvio dalla
finestra, la console di Steam per un avvio da Steam), una volta ogni volta che quel
gruppo di file cambia, e la scheda Saves conta questi salvataggi nella riga sotto la
sua intestazione. Per giocarne uno, premi Import nella scheda Saves. Import copia
ciascuno con il suo co-salvataggio nel profilo e non sostituisce mai un salvataggio
del profilo: un salvataggio il cui nome il profilo usa già viene importato come
`orphan-<time>-<name>`.

## Esecuzioni di prova

Il pulsante Test run accanto a Run avvia il gioco come fa Run, ma tutto ciò che il
gioco scrive va in `<instance>/test-run/` invece che nel tuo profilo, nell'Overwrite
e nel prefisso: salvataggi nuovi e modificati, modifiche agli INI, `plugins.txt`, file
scritti in Data o nella cartella del gioco, e i log in `Documents/My Games/<game>`.
Puoi provare con le mod del profilo, senza alcuna mod, oppure solo con le mod
selezionate che sono attive, più le mod i cui plugin servono loro come master.

Quando il gioco esce, Eidos elenca quei file in cinque gruppi (salvataggi, modifiche
agli INI, stato dei plugin, output dell'Overwrite, log) e ti chiede di tenere o
buttare ciascuno. Apply aspetta finché ogni gruppo che contiene file ha una
risposta. Lo stato dei plugin non viene offerto quando l'esecuzione ha usato un
insieme di mod diverso da quello del profilo. Tenere i salvataggi non elimina mai un
salvataggio dal profilo, nemmeno uno che il gioco ha cancellato durante
l'esecuzione. Tenere l'output dell'Overwrite rimuove anche dall'Overwrite i file che
il gioco ha cancellato, e l'elenco li conta prima. Buttare qualsiasi cosa richiede
una seconda pressione. Decide later chiude l'elenco e lascia i file in `test-run/`,
e l'elenco si riapre la prossima volta che apri l'istanza o premi F5. Se Eidos o il
computer si fermano durante un'esecuzione di prova, i file restano lì allo stesso
modo e l'elenco si apre nello stesso modo.

Quando la finestra non è stata avviata dallo store del gioco, Test run passa il
gioco a quello store (Steam, Heroic o Legendary), come fa Run, e l'esecuzione che lo
store avvia è l'esecuzione di prova. Se lo store non avvia il gioco, la richiesta
resta in attesa: la prossima esecuzione del gioco di quel profilo, da Run o dallo
store, è l'esecuzione di prova, e Cancel the waiting test run la scarta. Un'esecuzione
di prova copre solo le cartelle elencate sopra: le altre scritture del gioco, come il
registro di Wine, vanno dove vanno sempre. I salvataggi di un'esecuzione di prova
arrivano a Steam Cloud solo dopo che li hai tenuti e hai fatto un'esecuzione
normale.

### Trovare la mod dietro un problema

Find the mod behind a problem, nella scheda Diagnostics, cerca la mod che causa un
problema che sai riprodurre, come un crash in un dato punto. Ogni passo è
un'esecuzione di prova con metà dei sospetti rimasti, i master di cui hanno bisogno e
le mod che tieni sempre attive: le mod selezionate nell'elenco e attive quando
inizi, e la mod che fornisce lo script extender. Dopo ogni esecuzione rispondi se il
problema si è presentato. Not sure divide diversamente i sospetti, e Run it again
ripete il passo. Rispondere, o premere Run it again, butta i file che il passo ha
scritto, dopo una seconda pressione. Per tenerne alcuni, premi See the files prima di rispondere:
apre lo stesso elenco che compare dopo un'esecuzione di prova. Quando il gioco è
andato in crash e un crash logger ha scritto un log, o il gioco è uscito con un
codice di errore, Eidos segna Yes come risposta probabile. 200 sospetti richiedono
circa 8 esecuzioni.

La ricerca viene salvata in `<instance>/bisect.json`, così puoi chiudere Eidos tra un
passo e l'altro, e appartiene al profilo in cui è iniziata. Stop the search la scarta
dopo una seconda pressione. Il risultato nomina la mod, o l'insieme di mod che hanno
bisogno dei master l'una dell'altra, ed è conservato in
`<instance>/logs/bisect-<time>.txt`. Le mod generate (un output di Nemesis o
DynDOLOD, una Bashed Patch) sono costruite per la lista completa e possono fallire
con metà di essa: tienile sempre attive, oppure lasciale spente per tutta la
ricerca.

## Liste Wabbajack

Eidos non ha un installatore Wabbajack. Installa la lista con CLF3 o Jackify, fuori
da Eidos, poi scegli la cartella d'installazione come istanza portatile nella
procedura guidata. Eidos legge la cartella come la dispone MO2: `mods/`,
`profiles/<name>/modlist.txt` e gli elenchi dei plugin. Conserva il
`ModOrganizer.ini` che l'installatore ha scritto e non lo sovrascrive mai, così gli
eseguibili della lista restano registrati.

Molte liste aggiungono una cartella `Stock Game` o `Game Root` che contiene una
copia patchata del gioco, e MO2 esegue il gioco da lì. Quando la procedura guidata o
`eidos init` adotta per la prima volta una cartella di lista e quella cartella
contiene l'eseguibile del gioco, anche l'istanza esegue il gioco da essa. Il gioco
parte comunque tramite Steam o lo store che lo possiede, quindi non c'è nulla da
spostare. Eidos trova la cartella dal `gamePath` nel `ModOrganizer.ini` della lista,
oppure da quei due nomi. Un'istanza adottata con un Eidos precedente mantiene
l'installazione dello store finché non la cambi (vedi la sezione successiva). Quando
quel `gamePath` nomina una cartella di gioco che l'istanza non usa, Diagnostics lo
dice, perché gli strumenti in modalità Mod Organizer 2, come PGPatcher, leggono il
gioco da lì. CLF3 ricostruisce i file BSA e DDS con i propri strumenti, quindi il
risultato può differire da un'installazione Wabbajack, e gli autori delle liste non
lo supportano.

## File di gioco: l'installazione dello store o una copia

Per impostazione predefinita il gioco parte dalla sua installazione nello store, la
cartella in cui lo tengono Steam, Heroic o Legendary. Un'istanza può invece usare una
copia di quella cartella dentro l'istanza. Le mod stanno sopra la copia, e
l'installazione dello store è solo il posto in cui Eidos monta il risultato. Lo store
avvia comunque il gioco, quindi Steam mantiene il suo overlay, la sua cache degli
shader e i suoi salvataggi nel cloud.

La scheda Game files in cima alla scheda Diagnostics mostra quali file l'istanza usa,
con la versione del gioco della copia e dell'installazione dello store, e passa
dall'uno all'altro. In un terminale, `eidos stock-game <instance>` stampa gli stessi
fatti, e `use <folder>`, `store`, `freeze` e `delete --yes` fanno ciò che fanno i
pulsanti della scheda. `freeze` stampa la dimensione e il tipo di copia, e
`freeze --yes` copia.

Freeze this version copia l'installazione dello store in `<instance>/Stock Game` e
avvia il gioco da essa, così un aggiornamento dello store non rompe più i plugin
dello script extender: l'aggiornamento cambia l'installazione dello store, non la
copia. La prima pressione mostra la dimensione e cosa ne fa il file system, e la
seconda copia. Su un file system che può condividere blocchi (reflink), come btrfs o
XFS, la copia occupa quasi nulla finché lo store non cambia un file. Su ext4, o
quando il gioco e l'istanza sono su file system diversi, è una copia completa grande
quanto la cartella del gioco. Eidos rifiuta mentre Steam segnala il gioco occupato o
non del tutto installato, mentre un gioco o uno strumento dell'istanza è in
esecuzione, quando l'istanza contiene già la copia del gioco di una lista, una copia
congelata o una non finita, e quando il file system ha meno spazio libero di quello
che serve a una copia completa. Le copie di Heroic e Legendary non vengono
controllate per un aggiornamento in corso. Uno Stop, un fallimento, o un
aggiornamento di Steam al gioco durante la copia lascia `Stock Game.eidos-tmp`, che
la scheda offre di eliminare; non viene mai usato.

Una copia nasconde ogni file che lo store aggiunge, cambia o rimuove nella cartella
del gioco dopo che la copia è stata fatta: nuovi file del Creation Club, un nuovo
eseguibile, nuove DLL. Il gioco, l'elenco dei plugin, LOOT e i controlli Health
leggono tutti la copia. Non nasconde ciò che accade fuori dalla cartella del gioco:
un aggiornamento può comunque installare runtime nel prefisso Proton al lancio
successivo, e Steam può passare il gioco a un altro Proton.

Return to the store version toglie la copia dal lancio successivo dopo una seconda
pressione. La copia resta su disco, e Delete the copy la rimuove, di nuovo dopo una
seconda pressione. Delete viene rifiutato finché l'istanza usa ancora la copia.
Eidos non elimina mai la cartella `Stock Game` o `Game Root` di una lista. Se la
cartella della copia sparisce, o non contiene più l'eseguibile del gioco, gli avvii
vengono rifiutati e Diagnostics nomina la cartella, così la versione dello store non
parte mai a sorpresa.

Morrowind, e Oblivion con `bUseMyGamesDirectory=0`, tengono la loro lista dei plugin
nella cartella del gioco, ed Eidos non li esegue ancora da una copia.

## Installare una collezione di Nexus

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # la legge e dice cosa succederebbe
eidos collection skyrimse rqhcxy --no-optional  # salta i membri che segna come facoltativi
```

Nella finestra, incolla il link in File -> Open a Nexus collection..., oppure clicca
su uno sul sito. Eidos elenca i membri della revisione, ciascuno confrontato con
questa istanza: installato, scaricato o mancante. *Install this collection* esegue
l'intera ricetta su un thread di lavoro, con la barra di avanzamento che usano i
lavori di pack e unpack, e la barra diventa il rapporto quando finisce. La finestra
ed `eidos collection` eseguono la stessa installazione.

Senza un link, File -> Open a Nexus collection... si apre sulla scheda Browse.
Elenca le collezioni pubblicate per il gioco dell'istanza aperta, 20 alla volta,
ordinate per più scaricate, aggiornate di recente o più nuove, e la casella di
ricerca restringe l'elenco alle collezioni il cui nome o testo corrisponde. Ogni riga
dà l'autore, l'ultima revisione, il numero di mod, i download e una data. View apre
l'ultima revisione di quella collezione nella scheda Link, come incollarne il link.
Le collezioni segnate come per adulti vengono escluse a meno che il tuo account Nexus
mostri contenuti per adulti, le collezioni senza valutazione vengono sempre escluse,
e la riga sotto l'elenco conta entrambe. Sfogliare richiede un account Nexus
connesso, come il resto del pannello.

Una collezione si apre solo con un'istanza del suo stesso gioco. Una collezione di
Skyrim aperta mentre è caricata un'istanza di Fallout 4 viene rifiutata per nome,
invece di confrontare i suoi membri con l'elenco di mod sbagliato.

### Cosa Eidos legge da una collezione

Quasi nulla di una collezione è visibile attraverso l'API di Nexus. L'ordine di
installazione, le risposte che l'autore ha dato all'installatore con script di ogni
mod, quale mod vince un conflitto di file, quali plugin si caricano dove e le patch
binarie stanno tutti in un `collection.json` dentro l'archivio della collezione
stessa. Scaricare le stesse mod e installarle con le opzioni predefinite dà un gioco
diverso.

Eidos scarica l'archivio, legge `collection.json` e lo applica:

| La collezione dice | Eidos fa |
| --- | --- |
| `phase` | installa in quell'ordine, i membri facoltativi per ultimi |
| `choices` | ripete le risposte dell'autore a ogni FOMOD, e lo dice quando non può |
| `modRules` | riordina l'elenco delle mod perché la mod giusta vinca ogni file |
| `plugins` / `pluginRules` | unisce le regole di LOOT nel tuo userlist e ordina |
| `INI Tweaks/` | le installa come una mod a sé |
| `tools` | ti dice quali strumenti si aspetta; non crea nulla |

I membri vengono attivati man mano che si installano, in fondo all'elenco delle mod
nell'ordine di installazione, prima che `modRules` li sposti. Un membro lasciato fuori
dall'elenco delle mod non si caricherebbe, e il rapporto non lo mostrerebbe.

Lo stato viene scritto in `<instance>/collections/<slug>-<revision>.state.json` dopo
ogni membro. Sta accanto alla cartella di quella revisione, mai dentro di essa, quindi
una collezione non può portare un file di stato proprio, e un'installazione
interrotta al membro 147 di 200 riprende dal 147. Viene scritto con un nome
temporaneo e rinominato al suo posto, quindi un'interruzione lascia il vecchio
record o il nuovo e mai metà di uno dei due. Se non si riesce mai a leggerlo, Eidos si
ferma e lo dice invece di ricominciare l'intera collezione da capo. Esegui di nuovo
lo stesso comando.

Installare una revisione più recente di una collezione che hai già prende in carico
le cartelle della revisione precedente: un membro invariato viene verificato sul
posto, un membro il cui file l'autore ha aggiornato viene sostituito dentro la sua
vecchia cartella, e un membro che la nuova revisione ha eliminato resta installato e
viene nominato nel rapporto perché tu lo disattivi o lo rimuova. Una cartella che
un altro profilo attiva non viene mai presa in carico, perché la cartella delle mod
è condivisa e quel profilo cambierebbe sotto di te: la nuova revisione installa invece
una propria copia accanto ad essa, e il rapporto ti chiede di disattivare la vecchia
in questo profilo, mai di rimuoverla. Tornare a una revisione più vecchia riprende le
sue cartelle allo stesso modo, oppure installa copie nuove di quelle che non può
riprendere. Ogni volta che un membro viene sostituito in questo modo e avevi
modificato o aggiunto file nella sua cartella, la vecchia cartella viene prima
conservata accanto come `<name>_backup`, che non viene mai caricata nel gioco, e il
rapporto la nomina.

Un residuo di una revisione precedente che resta attivo impedisce che l'installazione
conti come quella costruita dall'autore; la nota sparisce quando lo disattivi. Un
membro che hai saltato con `--no-optional` non viene preso in carico e viene nominato
allo stesso modo. Se la presa in carico stessa fallisce, l'esecuzione si ferma con
*previous revision: its folders could not be taken over* e il motivo; correggilo ed
esegui di nuovo il comando.

### Account gratuiti e mod che scarichi tu

Un account Nexus gratuito non può scaricare le mod membro. Nexus non dà agli account
gratuiti alcun link di download attraverso l'API: ogni file richiede il pulsante
"Mod Manager Download" del sito. L'archivio della collezione si scarica con un
account gratuito, quindi puoi leggere l'intera ricetta, ma ogni membro viene
segnalato con l'URL esatto della sua pagina perché lo scarichi tu, e riprovare non
cambia nulla. Scaricali con il pulsante del sito ed esegui di nuovo il comando:
quei membri vengono cercati in `downloads/` a ogni esecuzione, quindi la collezione
si completa man mano che li fornisci.

Nella finestra, un account gratuito riceve *Fetch the N missing required files, one
by one* nel pannello della collezione al posto del download in blocco. Apre la
pagina del primo file obbligatorio mancante su Nexus, aspetta che quel file arrivi in
`downloads/`, poi apre il successivo. Premi comunque tu il pulsante di download
della pagina per ogni file. Il pannello mostra a che punto è (*3 of 12*), con *Skip* e
*Stop*. Quando arriva invece un altro file della stessa mod, il pannello nomina
entrambi gli id di file e continua ad aspettare quello fissato. I membri facoltativi
non ne fanno parte e mantengono il proprio pulsante *Open*.

Eidos applica anche le patch binarie (ciascuna verificata rispetto al suo CRC
registrato), gli override di file (i file nominati vengono nascosti con `.mohidden`,
come fa MO2) e i membri i cui file viaggiano dentro l'archivio della collezione
(`bundle`). I membri che la collezione si aspetta che tu scarichi a mano (`browse`,
`manual`) portano nel rapporto le istruzioni dell'autore stesso. Un membro il cui
nome è già una tua mod viene installato accanto ad essa con un nome libero, mai sopra
di essa, e il rapporto dice quale.

### Il rapporto

Un membro le cui risposte dell'installatore non si sono potute ripetere tutte
riceve una riga a sé, *installed, but not the way the collection asks*, separata dai
membri che si sono installati come li ha costruiti l'autore, perché i suoi file su
disco differiscono da quelli dell'autore. Il rapporto elenca anche le regole d'ordine
che non nominavano nulla nella collezione, le mod prese in un ciclo di regole
contraddittorie, i gruppi LOOT che non esistono e qualsiasi parte del manifesto che
questa versione di Eidos non capisce.

### Mantenere l'ordine della collezione

Eidos conserva le regole di ogni collezione installata dopo l'installazione. Quando
in seguito sposti due delle sue mod contro una regola prima o dopo, la scheda
Diagnostics mostra una scheda per quella collezione, che nomina la prima coppia di
questo tipo e conta le regole che non valgono più. Contano solo le mod attive nel
profilo corrente, e le regole prese in un ciclo vengono saltate, perché nessun ordine
può rispettarle. Restore collection order riporta le mod della collezione in un
ordine che rispetta ogni regola, mantiene il tuo ordine dove le regole lo permettono
e lascia dove sono le mod che non fanno parte della collezione. Ctrl+Z lo annulla.
`eidos check` elenca lo stesso riscontro come `collection_order_broken`. Il
controllo legge la ricetta della revisione in `collections/`, quindi una collezione
la cui ricetta in cache è stata rimossa non viene controllata.

## Spostare un'istanza su un'altra macchina

`eidos pack` scrive un'intera istanza (ogni mod, l'ordine di caricamento, tutti i
profili, l'Overwrite con i tuoi salvataggi dentro e gli archivi da cui tutto è stato
installato) in un unico file. `eidos unpack` la rimette a posto:

```sh
eidos pack skyrimse ~/backup.eidos                  # oppure indica una cartella: prende il nome dal gioco e dalla data
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # sull'altra macchina
```

`eidos pack --dry-run` elenca esattamente cosa entrerebbe, cosa no e perché, e
quanto spazio serve, senza scrivere nulla. `eidos unpack --info` fa lo stesso per un
file che hai già.

Se 7-Zip non ha potuto leggere qualcosa, o la scansione ha dovuto saltare qualcosa
che ti aspetteresti in un backup (un link simbolico, una cartella che non ha potuto
leggere, un nome che 7-Zip non può ricevere in modo sicuro), l'archivio viene
comunque scritto con il nome che hai scelto, ma `eidos pack` esce con 1 e dice cosa
manca, così uno script che esegue `eidos pack ... && ...` si ferma lì.

Un file `.eidos` è un archivio 7-Zip con una propria estensione. 7-Zip è già
richiesto per installare una mod, quindi questo non aggiunge dipendenze, e il backup
si apre senza Eidos in qualsiasi archiviatore che legga 7-Zip. È compresso con
LZMA2 a `-mx1`, a circa il 43 % dell'originale su una lista pesante di texture;
`-mx9` richiede molto più tempo e risparmia qualche punto in più. Texture e mesh sono
formati già compressi, quindi un dizionario più grande trova pochissimo ancora da
comprimere. `--level` (0, 1, 3, 5, 7 o 9) sceglie un altro livello, e
`--no-downloads` lascia fuori `downloads/`.

Al livello predefinito 1 i file vanno in blocchi solidi di al massimo 16 MB.
Estrarre un solo file da un backup di 70 GB decodifica al massimo il blocco in cui
sta, mai tutto ciò che lo precede, e impacchettare contenuti di gioco va da 2 a 5
volte più veloce che con ogni file compresso da solo (lo spacchettamento da 2 a 4
volte). Il costo sta nel danno: un byte corrotto può trascinare con sé ogni file del
suo blocco, fino a 16 MB di file vicini. I dati di recupero descritti più sotto
coprono questo caso. Ogni altro livello resta non solido: ogni file è un flusso a sé,
e un byte corrotto costa quel solo file. Il livello 0 memorizza i file. Dal livello 3
in su, LZMA2 lavora già a blocchi di 16 MB o più, e i blocchi solidi sono risultati
più lenti: al livello 5, impacchettare mesh ha richiesto 1,4 volte il tempo e
spacchettare texture 6,8 volte.

L'impacchettamento prende il blocco dell'istanza, quindi non può girare mentre il
gioco o un altro Eidos scrive in quell'istanza, e nulla cambia l'istanza mentre viene
letta. Il backup viene scritto con un nome `.part`, svuotato su disco e riletto per
intero con `7z t` prima di ricevere il nome che hai scelto. Quel controllo decodifica
ogni blocco e confronta ogni checksum, lo stesso lavoro che fa uno spacchettamento
sull'altra macchina. Mostra ciò che il disco ha restituito subito dopo la scrittura,
non che l'unità lo conserverà; i dati di recupero qui sotto servono a questo. Se
fallisce, il `.part` viene cancellato, non viene scritto nulla con il tuo nome, ed
`eidos pack` cita la riga in cui 7-Zip ha trovato il danno. Un impacchettamento
interrotto o fallito lascia un file `.part` o niente, mai un file `.eidos` che sembra
completo e non lo è.

Quando par2cmdline è installato (`sudo pacman -S par2cmdline` su Arch,
`sudo apt install par2` su Debian e Ubuntu, `sudo dnf install par2cmdline` su
Fedora), `eidos pack` scrive anche dati di recupero PAR2 accanto all'archivio:
`backup.eidos.par2` e alcuni file `backup.eidos.volNN+MM.par2`, il 5 % dell'archivio
più un piccolo indice. Tienili nella stessa cartella del file `.eidos`, con i nomi
con cui sono stati scritti, e copiali insieme ad esso: un backup spostato senza di
essi non ha nulla da cui essere riparato, e i dati di recupero registrano il nome
dell'archivio, quindi un backup rinominato non corrisponde più. `eidos unpack` li
trova per nome e controlla l'archivio rispetto ad essi prima di estrarre qualsiasi
cosa, compreso un backup la cui fine è stata tagliata così che 7-Zip non può
aprirlo. Se pochi byte sono andati male, par2 ripara prima l'archivio e conserva la
copia danneggiata come `backup.eidos.1`, che richiede nella sua cartella uno spazio
libero pari alla dimensione dell'archivio; un danno oltre ciò che copre il 5 %
ferma lo spacchettamento prima che venga scritto un solo file. I file di recupero che
non corrispondono all'archivio, come quelli di un backup più vecchio con lo stesso
nome, non vengono usati quando 7-Zip legge l'archivio e lo trova integro, e lo
spacchettamento lo dice. Su una macchina senza par2 lo spacchettamento gira comunque,
con un avviso che i dati di recupero non sono stati usati. `--no-recovery`
impacchetta senza, ed `eidos pack --dry-run` dice se verranno scritti. `--force`
sostituisce un file esistente con lo stesso nome.

### Cosa non c'è dentro, e perché

| Lasciato fuori | Perché |
| --- | --- |
| Il prefisso Proton | Ha la forma della macchina (mappature delle unità, una vista `Z:` di *questo* file system) ed Eidos lo ricostruisce in un minuto con `eidos prereqs <instance> --install`. Portarselo dietro raddoppierebbe all'incirca l'archivio per spedire qualcosa che l'altra macchina deve comunque rigenerare. |
| Il gioco | Un'istanza modifica un gioco che non contiene. Installalo da Steam sull'altra macchina. |
| Strumenti fuori dall'istanza | xEdit, DynDOLOD e altri strumenti sono binari di terze parti con i propri installatori. Sono nominati nell'archivio, quindi spacchettando ti viene detto esattamente quali mancano sulla nuova macchina. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Registri della macchina che ha fatto il backup. `prereqs.done` in particolare direbbe che le librerie di runtime sono già installate in un prefisso che non c'è. |
| `loot/`, tranne `userlist.yaml` | Una cache della masterlist che Eidos riscarica su richiesta. Le tue regole LOOT in `userlist.yaml` non si possono riscaricare, quindi quel solo file entra. |
| `.base/`, `.base-root/` | Punti di montaggio vuoti in cui i file del gioco vengono messi da parte durante una sessione. |
| `test-run/`, `bisect.json` | Un'[esecuzione di prova](#esecuzioni-di-prova) in attesa della revisione, e una ricerca della mod dietro un problema tra due passi. Entrambi appartengono a esecuzioni su questa macchina. |
| File scritti a metà | Un download in pausa (`*.unfinished`), una scrittura atomica in corso (`*.eidos-tmp*`). |
| Link simbolici | 7-Zip ne seguirebbe uno e copierebbe ciò a cui punta, il che per un link assoluto significa portare un albero estraneo nel tuo backup. Vengono invece segnalati, e il backup conta come incompleto: una `mods/` o `downloads/` collegata a un altro disco non viene impacchettata. Per fare il backup senza una `downloads/` collegata di proposito, passa `--no-downloads` (o togli la spunta a *Include downloads/* nella finestra); un'esclusione deliberata non conta come mancante. |

Una copia del gioco nell'istanza (`Stock Game`) viene impacchettata come qualsiasi
altra cartella, quindi il backup di un'istanza congelata è più grande all'incirca
quanto il gioco. Una copia non finita (`Stock Game.eidos-tmp`) viene lasciata fuori.

Ognuna di queste voci è elencata con il suo motivo in `eidos-backup.ini` alla radice
dell'archivio, quindi leggere quel file mostra cosa è stato lasciato fuori. Lo stesso
file registra dove viveva l'istanza, quale profilo era attivo, quanto occupa una
volta spacchettata e di quali strumenti avrà bisogno la nuova macchina.

I record dei download (`downloads/*.meta`) entrano con la riga `url=` svuotata. Quell'URL
è un link Nexus firmato che smette di funzionare entro poche ore e porta lo `user_id`
dell'account che ha scaricato il file, e un backup può essere passato a qualcun
altro. L'id della mod, l'id del file e la versione restano, così il download si può
ritrovare.

### Nella finestra

Entrambi sono nel menu File: *Pack this instance...* e *Unpack a backup...*. Lo
spacchettamento è anche nella schermata di benvenuto, sotto l'elenco delle istanze,
così una macchina che non ha ancora alcuna istanza può spacchettarne una.

Prima che venga scritto qualsiasi cosa, la finestra Pack mostra quanti file, quanto
grandi, quanta parte è `downloads/`, a quanto verrà all'incirca l'archivio e quali
strumenti sono nominati ma non portati. La sua casella *Write recovery data* è
spuntata quando par2 è installato, e disattivata con il pacchetto da installare
quando non lo è. La finestra Unpack legge il manifesto del backup appena scegli il
file, così può dirti quale gioco contiene, quando è stato fatto, dove viveva e quali
dei suoi strumenti mancano qui, prima che tu scelga una cartella.

Entrambi girano su un thread di lavoro con una barra di avanzamento, e la barra
diventa il rapporto quando finiscono.

### Cosa ripara lo spacchettamento

Una semplice copia della cartella lascerebbe ogni pulsante di strumento puntato a un
percorso sulla vecchia macchina. `eidos unpack` riscrive i valori che nominano la
vecchia radice dell'istanza, e solo quelli: `exe`, `workdir` e le chiavi `arg`
numerate in `tools.ini`, e `installationFile` nel `meta.ini` di ogni mod. Uno
strumento che non è mai stato dentro l'istanza (`/opt/xedit/SSEEdit.exe`) viene
lasciato esattamente com'era ed elencato come qualcosa da reinstallare; Eidos non
indovina dove sia quello strumento sulla nuova macchina. Corregge anche se l'istanza
si dichiara centrale o portatile, così i comandi successivi la cercano dove si trova
ora.

Lo spacchettamento rifiuta una cartella non vuota (passa `--force` se intendi
sovrascrivere), un disco troppo piccolo per la cifra del manifesto stesso, un archivio
fatto da un Eidos più recente del tuo e qualsiasi archivio che contenga una voce che
verrebbe scritta *fuori* dalla cartella che hai scelto. Senza una cartella, il backup
di un'istanza globale torna al posto centrale del suo gioco; uno portatile richiede
una cartella.

Poi:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # ricostruisci il prefisso Proton
eidos play /mnt/games/EidosSkyrim
```

## La finestra

Avvia `eidos-gui`, oppure il file `eidos-linux` che Colony installa.

Il primo avvio apre una procedura guidata come quella di MO2: benvenuto, tipo di
istanza (portatile o globale), gioco, nome e posizione, riepilogo, poi la finestra
principale. La schermata di benvenuto elenca anche ogni istanza nota, globale e
portatile, l'ultima usata per prima, da aprire con un clic, quindi fa anche da
selettore di istanza. Puntare la procedura guidata a una cartella che contiene già
un'istanza apre quell'istanza così com'è invece di crearne una nuova sopra, e una
cartella che appartiene a un altro gioco viene rifiutata.

La finestra principale ha due pannelli: l'elenco delle mod, e le schede Data,
Plugins, Conflicts, Overwrite, Archives, Saves, Downloads e Diagnostics, con un
pulsante Run, un pulsante Test run ([Esecuzioni di prova](#esecuzioni-di-prova)) e un
selettore del bersaglio di esecuzione. Filtri l'elenco delle mod, selezioni,
riordini, raggruppi con separatori, restringi per categoria e clicchi con il tasto
destro per le azioni. Oltre a invia in cima e in fondo, il menu delle mod ha gli
spostamenti mirati di MO2: invia sopra la prima mod in conflitto, sotto l'ultima, a
una data priorità, o nel gruppo di un separatore.

Il selettore dei profili cambia profilo. Fai clic destro su un profilo per Rename,
Copy to new..., Desktop shortcut, Import from MO2... ed elimina, che richiede due
clic e rifiuta il profilo attivo o l'ultimo. Import from MO2... riprende l'ordine
delle mod e l'ordine di caricamento di un profilo MO2. Desktop shortcut scrive una
voce desktop che esegue `eidos tool <instance> run "<game>" --profile <name>`: rende
attivo quel profilo e chiede allo store di avviare il gioco (dettagli in
[tools.md](tools.md#raggiungere-uno-strumento-nascondere-fissare-e-una-scorciatoia-sul-desktop)).

Il menu File esegue anche winecfg, regedit, il gestore file di Wine e il programma di
disinstallazione nel prefisso del gioco, e Prefix drive_c apre l'unità `C:` del
prefisso; vedi
[tools.md](tools.md#i-programmi-di-wine-nel-prefisso-del-gioco). *Instances...*
elenca le istanze che Eidos conosce; Forget ne toglie una da quell'elenco e non
tocca nulla su disco. *Export the mod list...* scrive l'elenco nel formato CSV di
MO2, come fa `eidos export`.

### Preferenze

Clicca su Eidos, in alto a sinistra, per aprire Preferences al posto dei due
pannelli, e clicca di nuovo (o Close) per tornare indietro. General, Appearance e
Accessibility vengono per prime, come in ogni programma Colony, poi Mod list, Nexus e
About. Ogni modifica si applica e si salva subito; non c'è un pulsante Save.

- General: il gioco che Eidos apre, la dimensione della finestra, la lingua
  dell'interfaccia, la cartella degli strumenti e come viene avviato un gioco. Lock
  the window while a game runs è attivo per impostazione predefinita. Start the game
  at once when a store opens Eidos, e la notifica desktop che dice quando un lavoro
  lungo finisce mentre la finestra è in secondo piano, sono disattivati per
  impostazione predefinita. La finestra parla inglese e francese; System segue
  `LC_ALL`, `LC_MESSAGES` o `LANG`, e la modifica si applica subito. La riga di
  comando `eidos` risponde sempre in inglese.
- Appearance: il tema (26 famiglie, 59 palette, la pergamena di Eidos per
  impostazione predefinita), il colore d'accento, la dimensione del testo, gli effetti
  e un'anteprima dal vivo.
- Accessibility: alto contrasto, un carattere adatto alla dislessia, movimento
  ridotto e una dimensione del testo di lettura che si moltiplica con quella di
  Appearance.
- Mod list: la velocità di scorrimento automatico mentre una mod viene trascinata
  verso il bordo superiore o inferiore dell'elenco.
- Nexus: l'accesso, la cartella dei download, il gestore `nxm://` e il suo pulsante
  Register, i server preferiti, Install downloads when they finish e Offline mode.
- About: dove stanno i file, le scorciatoie da tastiera, la versione e il controllo
  dei rilasci.

L'aspetto e la lingua dell'interfaccia stanno in
`~/.config/Colony/Eidos/preferences.json`. Le altre impostazioni, che legge anche la
riga di comando (il gioco predefinito, la cartella degli strumenti, i server Nexus,
Offline mode e il resto), stanno in `settings.ini` accanto ad esso, e la sessione
Nexus in `nexus.ini`. Il tema che un Eidos più vecchio teneva in `settings.ini` viene
riportato la prima volta.

About -> Version contiene il controllo dei rilasci, attivo per impostazione
predefinita. All'avvio, al massimo una volta al giorno, Eidos chiede a GitHub l'ultimo
rilascio di `Project-Colony/Eidos`. Quando è più recente della copia in esecuzione, la
barra dei menu mostra `Eidos <version> is available: release notes` dopo Help, e il
link apre la pagina di quel rilascio con il suo changelog. Eidos non scarica né
sostituisce mai se stesso. Il controllo viene saltato in modalità offline. È
disattivato per una copia installata sotto `/usr` da un gestore di pacchetti, che la
aggiorna; `/usr/local`, dove `install.sh --system` mette Eidos, conta come
un'installazione tua.

### Colonne, ordinamento e raggruppamento

L'elenco mostra per impostazione predefinita quattro colonne (Category, Content,
Version e Flags, quelle che mostra MO2) e ne offre otto: Category, Content, Version,
Author, Installed, Nexus id, Game, Flags. Spuntale nel menu View. Con tutte e otto
attive, alla colonna del nome resta poco spazio.

Clicca su un'intestazione per ordinare per essa, di nuovo per invertire, e una terza
volta per tornare all'ordine di caricamento. L'elenco si può trascinare solo
nell'ordine di caricamento: una posizione di rilascio indica l'elenco reale, e sotto
un ordinamento il posto di una riga sullo schermo non è il suo posto in quell'elenco.
Mentre un ordinamento è attivo, le strisce di inserimento non vengono disegnate e un
trascinamento viene rifiutato, come in MO2. Il menu View lo dice e offre la via del
ritorno.

Il menu View può anche raggruppare l'intero elenco, per categoria o per origine (da
Nexus, o installata a mano). Le intestazioni dei gruppi non sono separatori: non si
possono rinominare, colorare o spostare. Si ripiegano, e un'intestazione ripiegata
mantiene il suo conteggio. I separatori vengono nascosti mentre un ordinamento o un
raggruppamento è attivo, perché un separatore intesta le righe che lo seguono
nell'ordine di caricamento, ed entrambi cambiano quell'ordine sullo schermo.

### Mouse e tastiera

Doppio clic su una mod per Information, Ctrl+doppio clic per la sua cartella,
Shift+doppio clic per la sua pagina Nexus. Ctrl+F mette il cursore nella casella del
filtro. Digitare una lettera salta alla mod successiva che inizia con essa, e
premerla di nuovo scorre le altre invece di restare sulla prima. Nessuna finisce su
una riga che il filtro, un separatore ripiegato o un gruppo ripiegato nasconde,
quindi il prossimo Spazio non attiva mai una mod che non vedi.

"Collapse others" nel menu di un separatore ripiega ogni gruppo tranne quello.
Durante un trascinamento, tenere il puntatore fermo su un gruppo ripiegato lo apre,
così la mod può essere rilasciata dentro; passarci sopra non lo apre.

Ctrl+Z annulla l'ultima modifica all'elenco delle mod o all'ordine di caricamento: una
spunta, uno spostamento, una disattivazione in blocco, un ordinamento LOOT.
Ctrl+Shift+Z la ripete. La finestra tiene 50 passi e li dimentica con F5, alla fine di
un'esecuzione di gioco o di strumento, al cambio di profilo e quando un altro processo
Eidos cambia l'elenco. Installare, rinominare o rimuovere una mod cambia le cartelle
su disco, quindi un annullamento oltre uno di questi viene rifiutato.

### Filtrare l'elenco

Il pulsante Filters apre una riga per ogni criterio: attiva, in conflitto,
aggiornamento disponibile, nessuna categoria, endorsed, tracked, ha un id Nexus, ha file
nascosti, nessun dato di gioco valido, per un altro gioco, ha note, e una riga per ogni
tipo di contenuto (plugin, archivi BSA o BA2, texture, mesh, script, plugin dello
script extender, file di interfaccia, suono, FaceGen). Un clic fa scorrere una riga tra
off, only ed except. La riga in cima passa tra la corrispondenza con tutti i criteri
attivi e la corrispondenza con uno qualsiasi. La casella del nome e l'elenco delle
categorie restringono sempre ulteriormente il risultato. La riga dei file nascosti
guarda solo le mod attive.

### Cosa l'elenco ti dice di una mod

Due indicatori consultivi, entrambi un glifo con la spiegazione al passaggio del
mouse. Nessun dato di gioco valido significa che nulla in cima alla mod somiglia a
qualcosa che questo gioco carica; potrebbe servire spostare le sue cartelle di un
livello in su, oppure potrebbe non essere una mod per questo gioco. Altro gioco
significa che il `meta.ini` della mod stessa ne nomina uno diverso. Nessuno dei due
blocca nulla: la mod viene comunque distribuita. "Mark as valid" nel menu della riga
silenzia entrambi attraverso la chiave `validated=` di MO2 stesso, così una mod
segnata come valida in un gestore non viene segnalata nell'altro.

Il controllo della struttura segnala una mod solo quando è sicuro: un albero `Root/`,
una cartella illeggibile e una cartella vuota contano tutti come dati di gioco
validi.

### Disattivare una mod che i tuoi salvataggi o altre mod usano

Prima che una mod venga disattivata o rimossa, Eidos legge gli elenchi dei plugin dei
tre salvataggi più recenti del profilo. Quando uno di essi usa un plugin di quella
mod, una barra sopra l'elenco nomina i plugin e aspetta Disable anyway o Cancel
(anche Esc annulla). Remove aggiunge gli stessi nomi al suo messaggio di
riconferma. Vengono letti solo i salvataggi di Skyrim e Fallout 4; gli altri giochi
disattivano senza chiedere.

La scheda General di Information di una mod elenca le mod che richiede e le mod che
la richiedono, lette dai master dei plugin attivi. Disattivare una mod richiesta da
altre mostra la stessa barra con Disable them too, che disattiva anche ogni mod che ne
ha bisogno e le mod che hanno bisogno di quelle, oppure Keep them enabled. La scheda
dei master mancanti nella scheda Diagnostics ha Enable what provides them: attiva la
mod che contiene ogni master mancante e spunta un plugin master presente ma non
spuntato.

### Fare una copia di una mod prima di toccarla

"Back up this mod" copia la sua cartella da parte come `<name>_backup` (poi
`_backup2`, e così via; un backup non sostituisce mai il precedente). La copia non è
una mod: la sua casella non fa nulla e non aggiunge niente alla vista unita, perché
spuntarla distribuirebbe due copie di una mod una sopra l'altra. "Restore this backup
over the mod" la rimette a posto, in due clic; il contenuto attuale viene prima messo
da parte e scartato solo quando la copia è riuscita.

Backups nella barra degli strumenti tiene i punti di ripristino dell'elenco delle mod
del profilo, del suo ordine di caricamento (`plugins.txt` e `loadorder.txt` insieme) e
dei suoi INI di gioco, 10 per ciascuno, in `<profile>/backups/`. Back up now ne
aggiunge uno, e Restore fa prima una copia dello stato attuale, così una scelta
sbagliata si può annullare. Eidos tiene anche un punto di ripristino degli INI prima
di ogni esecuzione di BethINI (vedi [tools.md](tools.md#bethini-pie)).

### Le schede

Data è un albero della vista unita, espanso un livello alla volta: aprire una cartella
legge quella cartella in ogni strato che la ha, mai ogni mod attiva in una volta. È
costruito dallo stesso stack di strati che il mount serve, quindi i whiteout e i file
nascosti vengono rispettati e la scheda mostra ciò che il gioco vedrà. Filtralo per
nome, restringilo ai soli file contesi, capisci cosa sta dove con le colonne Size e
Modified, e fai Reveal di qualsiasi riga in un gestore file. Passando sopra un file
vengono elencate le altre mod che lo portano. I `meta.ini`, `readme.txt`, `fomod/` e
`Root/` al livello superiore di una mod sono esclusi da Data, in questa scheda e nel
mount: i primi tre sono per il gestore delle mod, e `Root/` va invece nella cartella del
gioco. Un file che il gioco o uno strumento scrive sotto uno di quei nomi finisce
nell'Overwrite e resta visibile.

Una riga `.exe` in Data ha Add tool, che apre l'editor Executables con quel file già
compilato (non viene salvato nulla finché non premi Save), e Run, che lo esegue una
volta attraverso la vista unita come uno strumento registrato ([Strumenti](#strumenti)).
Save tree scrive l'intero albero Data unito in un file di testo, una riga per file con
la mod che lo fornisce, con la tua cartella home e il tuo nome utente rimossi, per una
segnalazione di bug.

Plugins è l'ordine di caricamento ESP/ESM/ESL: attiva, riordina a mano, oppure ordina
con LOOT e leggi il rapporto dopo l'ordinamento, i cui link di consiglio si aprono nel
tuo browser. La sua intestazione conta i plugin attivi in ogni spazio di indici
rispetto al limite del gioco, per esempio `full 231/254, light 512/4096`, e diventa
arancione quando uno è pieno. La casella del filtro restringe l'elenco per nome. Spazio,
Ctrl+A, gli intervalli con Shift e Activate all shown e Deactivate all shown del menu
della riga agiscono allora solo sulle righe mostrate, e un trascinamento viene rifiutato
finché il filtro non viene cancellato. Selezionare un plugin colora i suoi master
nell'elenco dei plugin e la mod che lo porta nell'elenco delle mod. I plugin che LOOT
segnala come sporchi ricevono una scheda in Diagnostics che può pulirli con xEdit
([tools.md](tools.md#pulire-un-plugin-che-loot-segnala-come-sporco)).

Dopo un ordinamento LOOT, un plugin pieno i cui nuovi record rientrano tutti
nell'intervallo light lo dice al passaggio del mouse, e il menu della sua riga ha Flag
as light. Eidos scrive una copia con il flag light in una nuova mod chiamata
`<plugin> (light)` in fondo all'elenco delle mod (la priorità più alta) e lascia
intatto l'originale, quindi disattivare quella mod annulla l'operazione. Rifiuta
quando uno dei tre salvataggi più recenti del profilo carica il plugin come plugin
pieno. Vengono letti solo i salvataggi di Skyrim e Fallout 4, quindi su Starfield non
viene controllato nessun salvataggio. LOOT controlla solo gli intervalli dei record:
guarda in xEdit un plugin che aggiunge celle o worldspace prima di marcarlo.

Passando sopra un plugin si vede da dove viene, il suo autore, la sua intestazione e la
versione del form, i primi 300 caratteri della sua descrizione, i suoi master e, dopo
un ordinamento LOOT, i messaggi di LOOT. Su Skyrim, Skyrim SE, Skyrim VR ed Enderal SE
il gioco legge anche un INI chiamato come un plugin caricato; la colonna INI segna quei
plugin, e un clic apre quel file in sola lettura.

Conflicts spiega i vincitori e i perdenti per file. Overwrite trasforma in un solo passo
ciò che il gioco ha scritto in una mod vera.

Archives elenca ogni BSA o BA2 che le mod attive portano, la mod da cui viene e perché
si carica: un plugin attivo con lo stesso nome, oppure l'INI lo registra. Uno che nulla
carica è segnato come peso morto. Quando un'altra mod porta un archivio con lo stesso
nome che vince il conflitto, la riga nomina quella mod. Extract decomprime un archivio
in una nuova mod che prende il suo nome senza estensione (`Foo.ba2` dà
`Foo (extracted)`) e lascia l'archivio nella sua mod; i file sciolti vincono sui file
di qualsiasi archivio, qualunque sia l'ordine delle mod.

Saves legge l'intestazione di ogni salvataggio (personaggio, livello, luogo, tempo di
gioco), confronta l'elenco dei plugin memorizzato in esso con quello attuale, e ha un
pulsante che attiva le mod di cui ha bisogno. I salvataggi che sono arrivati nel
prefisso senza Eidos sono descritti sotto
[I salvataggi appartengono al profilo](#i-salvataggi-appartengono-al-profilo).

Downloads e Diagnostics hanno le proprie sezioni qui sotto.

### Informazioni sulla mod

"Information..." apre una finestra per mod con le schede General, Conflicts, Filetree,
INI Tweaks, Notes, Nexus files e Nexus page. Dal filetree (e dall'albero Data) qualsiasi
file può essere nascosto: viene rinominato in `<name>.mohidden`, il che lo toglie
dalla vista unita senza cancellarlo, così alcuni file isolati di una mod possono
essere soppressi senza cambiare le priorità. Il filetree esegue anche le normali
operazioni sui file: nuova cartella, rinomina, elimina, apri. Ognuna rifiuta un
percorso che non sia un semplice percorso dentro quella mod: niente `..`, niente
percorso assoluto e nessun symlink lungo la strada, perché seguirne uno potrebbe
cancellare un file fuori dalla cartella della mod. Rename cambia solo l'ultima parte
del percorso, quindi non sposta mai un file, e rifiuta un nome già preso. Delete
richiede due clic e non si può annullare.

View su qualsiasi riga del filetree o dell'albero Data mostra l'anteprima del file:
immagini, testo, texture DDS (scegli mip, livello, faccia e canale) e modelli NIF
statici con le loro texture diffuse. L'anteprima NIF richiede `eidos-nif-preview`
accanto a Eidos o nel `PATH`; il pacchetto di rilascio, il pacchetto Arch e le
compilazioni dai sorgenti lo hanno, `eidos-linux` no. Il testo viene letto fino a 64
KB e dice dove si è fermato.

INI Tweaks elenca i frammenti che una mod porta nella sua cartella `INI Tweaks/`; quelli
attivi vengono uniti all'INI di gioco del profilo all'avvio, in ordine di priorità, e
tolti di nuovo quando gli INI dell'esecuzione vengono catturati, quindi disattivare
un tweak ne rimuove le impostazioni.

La scheda Nexus files elenca i file della mod su Nexus, dal più recente, e segna quello
installato qui e quello che lo sostituisce. Quella seconda riga ha un pulsante Update;
vedi [Aggiornamenti delle mod](#aggiornamenti-delle-mod).

La scheda Nexus page mostra la descrizione della mod e, dal più recente, il changelog
di ogni versione di file sulla sua pagina. Eidos legge entrambi la prima volta che apri
la scheda, dall'API GraphQL di Nexus, che non conta nel budget orario di richieste. Li
tiene finché Eidos non si chiude o non premi Refresh. Vengono mostrati come testo
semplice: la formattazione BBCode e HTML viene rimossa, l'indirizzo di un link viene
scritto dopo il suo testo, e immagini e video vengono omessi. Endorse e Track sopra il
testo fanno ciò che fa il menu col tasto destro della mod. La pagina di una mod per
adulti resta nascosta a meno che il tuo account Nexus mostri contenuti per adulti, e una
pagina senza valutazione resta nascosta in ogni caso.

### Download

Un download può essere trascinato dall'elenco Downloads su una posizione nell'elenco
delle mod per installarlo a quella priorità, e anche archivi o cartelle rilasciati sulla
finestra da un gestore file si installano. I rilasci di file richiedono una sessione
X11 o XWayland: winit li implementa solo per X11. I download si possono mettere in
pausa e riprendere: la pausa ferma il trasferimento e conserva il file parziale, e
Resume chiede a Nexus un link nuovo e continua da dove si era fermato.

La scheda Downloads conserva gli archivi che hai scaricato. Filtrala per nome (anche il
nome amichevole della mod, così "skyui" trova `SkyUI_5_2_SE-12604-5-2SE.7z`), ordina per
più recenti, nome, dimensione o stato, e nascondi un archivio con cui hai finito: il file
resta e va via solo la riga. "Show hidden" li riporta, e lo stesso pulsante li mostra di
nuovo. "Remove N installed" elimina gli archivi delle mod che hai già installato, in due
clic, e solo quelli che il filtro mostra.

Preferences -> Nexus -> Install downloads when they finish è disattivato per
impostazione predefinita. Attivo, un download Nexus che finisce mentre la finestra è
aperta si installa senza passare dalla scheda Downloads. È la normale installazione: un
FOMOD apre il suo installatore, e un nome già presente nell'elenco delle mod chiede cosa
fare. Gli archivi che erano in `downloads/` prima dell'apertura della finestra o prima
che l'opzione venisse attivata vengono lasciati stare, e così un download di cui hai
annullato l'installazione. Mentre una collezione è aperta nel pannello delle collezioni,
nessun download si installa da solo, e uno che finisce nel frattempo non si installa da
solo nemmeno dopo; la collezione installa i suoi membri. Il file che un Update in
sospeso aspetta viene installato dall'Update. In questo modo vengono installati solo i
download Nexus, e uno che il filtro di Downloads nasconde aspetta finché il filtro non
viene cancellato. Mentre un trascinamento è in corso, o mentre il gioco o un altro Eidos
tiene l'istanza, l'installazione aspetta.

### Aggiornamenti delle mod

Il pulsante di aggiornamento verso l'estremo destro della barra degli strumenti
controlla ogni mod che ha un id Nexus. Quando la pagina della mod mostra la versione
che hai installato, non viene chiesto altro. Altrimenti Eidos legge l'elenco dei file
della mod (una richiesta in più, conservata per il resto del controllo) e segue la
catena degli aggiornamenti a partire dall'id del file che il `meta.ini` della mod
registra: gli autori segnano un nuovo caricamento come successore di uno più vecchio, e
MO2 legge la stessa catena. Un file più recente in quella catena è l'aggiornamento.
Quando la catena non ne nomina nessuno e il file installato è ancora elencato sulla
pagina, la mod conta come aggiornata, anche se la pagina mostra un'altra versione. La
versione della pagina decide solo quando non è registrato alcun id di file, quando il
file installato è stato rimosso o archiviato, o quando Nexus non ha inviato l'elenco dei
file.

Ogni mod usa il dominio Nexus del gioco che il suo `meta.ini` nomina (`gameName`), non
quello dell'istanza: il controllo degli aggiornamenti, Endorse, Track e Visit on Nexus
vanno tutti lì. Enderal SE accetta anche le mod `skyrimspecialedition`, quindi una mod di
Skyrim SE installata in un'istanza Enderal SE viene controllata sulla sua pagina di
Skyrim SE. Un link Mod Manager Download va all'istanza che hai usato per ultima quando il
suo gioco accetta il dominio del link. Altrimenti va a un'istanza del gioco di cui è il
dominio proprio, prima di una di un gioco che lo accetta soltanto. `EIDOS_INSTANCE=<folder>`
prevale su entrambi.

Track e Untrack nel menu col tasto destro di una mod aggiungono la mod ai tuoi mod
seguiti su Nexus o la tolgono, così le notifiche del sito seguono ciò che segui qui.
Endorse e Abstain nello stesso menu approvano la mod su Nexus o ritirano l'approvazione;
il pulsante endorse accanto al pulsante di aggiornamento approva la mod selezionata.
Ogni controllo degli aggiornamenti legge anche le tue approvazioni e le mod seguite da
Nexus (due richieste) e imposta i flag Endorsed e Tracked di ogni mod di conseguenza, sul
dominio Nexus della mod stessa. Quando i due non concordano, vince Nexus. Se uno dei due
elenchi non si può leggere, il controllo lo dice e lascia quei flag com'erano.

Il pulsante Update nella scheda Nexus files di "Information..." installa il file più
recente. Con un account premium Eidos scarica il file da sé. Con un account gratuito
apre la pagina del file, dove Mod Manager Download rimanda indietro il file, e
l'installazione parte quando l'archivio arriva in `downloads/`. Da lì è la normale
installazione: un FOMOD si apre con le risposte con cui la mod era stata installata, poi
la richiesta offre Replace con "Keep a backup of the existing mod" spuntato, qualunque
cosa tu abbia scelto all'ultima richiesta. Replace mantiene la priorità e lo stato di
attivazione della mod.

Prima che un Replace scambi la cartella di una mod, Eidos confronta la vecchia cartella
con la nuova per nome e dimensione dei file e legge le intestazioni dei plugin in
entrambe. Questo vale per Update, un aggiornamento di collezione e un Replace scelto
alla richiesta di installazione. La riga di stato conta poi i file aggiunti, rimossi e
cambiati di dimensione, e nomina ogni master di cui la nuova versione ha bisogno e la
vecchia no. La scheda General di "Information..." finisce con una sezione Last update che
elenca le versioni, i plugin aggiunti o rimossi, i master aggiunti o rimossi, i plugin il
cui flag ESM, ESL o ESH è cambiato, le mod con cui questa ora condivide file o non più, e
i file. Un file riscritto con la stessa dimensione non viene elencato. Il rapporto è
conservato in `logs/updates/<mod>.json` nell'istanza, che `eidos pack` lascia fuori; una
nuova installazione o un Merge nella cartella lo rimuove. `eidos install --replace`
stampa gli stessi conteggi su una riga `changes:`.

Una cartella che contiene più file Nexus, come un file principale con un file
facoltativo unito ad esso, viene rifiutata: sostituirla con il nuovo file principale
farebbe perdere gli altri. Aggiorna quelle a mano.

`eidos nexus update skyrimse --apply` esegue lo stesso controllo, poi con un account
premium scarica ogni file nuovo e lo installa sopra la sua cartella con
`--replace --backup`. Con un account gratuito stampa invece `OPEN <url>` per la pagina
di ogni file. Le cartelle che uniscono più file vengono saltate con il motivo. Una mod
FOMOD senza risposte registrate, come una installata prima che Eidos le conservasse, non
viene reinstallata con i valori predefiniti dell'installatore: quell'aggiornamento
fallisce con un messaggio che dice di aggiornare la mod una volta dalla finestra, che
registra le risposte. Un FOMOD il cui installatore non ha nulla da scegliere viene
installato come al solito. Il comando esce con 1 se un aggiornamento è fallito.

### Aggiornamenti delle collezioni

Quando esegui Check for updates, Eidos chiede anche a Nexus l'ultima revisione di ogni
collezione installata in questa istanza. Una riga sopra l'elenco delle mod nomina ogni
collezione con una revisione più recente e conta le mod aggiunte (+), aggiornate (~) e
tolte (-); passaci sopra per i loro nomi. I conteggi mancano quando la ricetta della
revisione installata non è più in cache. Update apre la revisione più recente nel
pannello della collezione, dove *Install this collection* la esegue. L'elenco viene
ricostruito a ogni controllo e non viene salvato.

### Impostazioni di Nexus

Preferences -> Nexus -> Offline impedisce del tutto a Eidos di contattare Nexus, e salta
il controllo giornaliero dei rilasci su GitHub. Controlli degli aggiornamenti, accesso,
download e collezioni lo dicono invece di fallire con un errore di connessione. È
disattivato per impostazione predefinita.

Preferred servers ordina i nodi CDN che un download preferisce, il migliore per primo.
Solo a un account premium vengono dati più mirror tra cui scegliere, quindi per tutti gli
altri sceglie Nexus e questo non cambia nulla. Quando nessuno dei server che hai
nominato viene offerto, il download avviene comunque, dal primo nodo che Nexus offre.

### Categorie

Le categorie possono essere assegnate a una mod o a un'intera selezione, il catalogo può
essere modificato dalla stessa finestra, e l'elenco ufficiale delle categorie del gioco
può essere ripreso da Nexus. Entrambi i file del catalogo sono quelli di MO2 stesso
(`categories.dat` e `nexuscatmap.dat`), quindi un'istanza condivisa mantiene un solo
catalogo. Una mod scaricata da Nexus viene categorizzata all'installazione, una volta
che Fetch from Nexus ha mappato il catalogo.

### Editor INI, log ed estensioni

View -> INI editor modifica gli INI di gioco del profilo, la copia che Eidos tiene; la
copia nel prefisso Proton viene sovrascritta a ogni avvio. La sua ultima scheda è il
`launch-env.ini` del profilo
([Variabili e wrapper per profilo](#variabili-e-wrapper-per-profilo)). View -> Log legge
i log delle sessioni. View -> Extensions elenca i tuoi componenti aggiuntivi; vedi
[extensions.md](extensions.md).

Quando un avvio trova l'INI principale del gioco (`Skyrim.ini`, `Fallout4.ini`,
`Oblivion.ini`...) né nel profilo né nel prefisso, Eidos copia prima quello predefinito
della cartella del gioco (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) nel profilo,
come fa MO2. Quando l'istanza usa una copia del gioco, il predefinito viene dalla copia.
Il prefisso e la cartella del gioco non vengono modificati.

### Installare le mod

L'installatore gestisce archivi Simple, FOMOD, pacchetti BAIN di Wrye Bash (spunta i
sotto-pacchetti, che si uniscono in ordine) e pacchetti OMOD di Oblivion. Uno script
OMOD viene eseguito quando usa i comandi OBMM che Eidos supporta o è uno dei tre
installatori C# che Eidos conosce; Eidos non esegue nessun altro codice C# o Visual
Basic e lo dice. Quando nessuna euristica riconosce la struttura, un selettore manuale
mostra l'albero dell'archivio e ti lascia indicare la radice dei dati.

Reinstallare o aggiornare una mod FOMOD apre la procedura guidata con le risposte della
sua ultima installazione spuntate, al primo passo le cui opzioni sono cambiate da
allora, con le modifiche elencate. Reinstall with new choices parte invece dai valori
predefiniti dell'installatore. `eidos install --replace` o `--merge` riusa le risposte
memorizzate e rifiuta, elencando le modifiche, quando non vanno più bene.

### Diagnostica

La scheda Diagnostics esegue controlli di salute dal vivo. Segnala i master mancanti, i
plugin che LOOT segnala come sporchi, gli archivi che nessun plugin attivo caricherà, se
l'elenco delle mod corrisponde ancora alla cartella delle mod, se il passthrough FUSE è
attivo e, quando è richiesto, se la capability di avvio di cui ha bisogno c'è. Dopo
un'esecuzione mostra cosa dice il log dello script extender stesso su ciascuna DLL dei
suoi plugin, così puoi vedere se i tuoi plugin SKSE si sono caricati. Segnala anche un
aggiornamento del gioco dall'ultimo avvio pulito, uno script extender mancante o che non
corrisponde alla versione del gioco (con la pagina di download dell'extender e un pulsante
di installazione per l'archivio che scarichi), un'opzione di avvio di Steam che non avvia
Eidos con `%command%`, NVAPI lasciato spento per DLSS, uno schermo intero esclusivo che
impedisce la generazione di fotogrammi, un `enblocal.ini` di ENB senza
`LinuxVersion=true`, le dichiarazioni dei plugin F4SE e SFSE e il log di crash
dell'ultima sessione con le mod che nomina.

I controlli per gli INI del profilo e i plugin cercano righe `sTestFile` in `[General]`
degli INI personalizzati di Fallout 4, Fallout 4 VR e Starfield, che fanno ignorare
`plugins.txt` al gioco (Fix le rimuove dalla copia del profilo e tiene il file
precedente come `<ini>.pre-fix`); questo controllo vale solo per Fallout 4, Fallout 4 VR
e Starfield. Segnalano inoltre i plugin `.esp` attivi in Starfield, e i plugin blueprint
che non hanno il flag blueprint, il prefisso `BlueprintShips-` o l'estensione `.esm`, o
il cui plugin principale non è attivo. I controlli di sistema segnalano un
`vm.max_map_count` o un limite hard dei file aperti sotto quanto serve a Proton, e un
gioco, un prefisso o un'istanza su NTFS, exFAT o FAT. I controlli sulle mod nominano il
percorso dati di BodySlide quando non punta al `Data` del gioco, le cartelle `Data` che
uno strumento ha scritto in `Overwrite/Root`, una patch i cui file sciolti perdono contro
la mod che modifica, e una mod che sostituisce gli script Papyrus dello script extender.

Check for updates legge anche i requisiti Nexus di ogni mod. Diagnostics elenca una mod
richiesta che manca o è disattivata, con Ignore per un requisito soddisfatto in un altro
modo.

La scheda mostra anche quale build di Proton esegue il gioco e avvisa prima che un Proton
più vecchio esegua un prefisso che uno più recente ha preparato; Rebuild mette da parte un
prefisso rotto perché Steam ne crei uno nuovo
([troubleshooting.md](troubleshooting.md#quale-proton-esegue-il-gioco-e-un-prefisso-rotto)).
Altre schede segnalano un prefisso mai preparato o il cui registro è obsoleto, un
programma diverso da Eidos che riceve i link Mod Manager Download (Register lo corregge),
l'output di uno strumento più vecchio dei suoi input (con un pulsante che esegue di nuovo
lo strumento), e una copia pulita o light di un plugin il cui originale è cambiato da
allora. Ogni scheda è spiegata in [troubleshooting.md](troubleshooting.md#schede-health).
Copy report mette negli appunti un rapporto del sistema e di ogni riscontro, con la tua
cartella home e il tuo nome di accesso nascosti, per una segnalazione di bug.

Per Skyrim SE, Enderal SE e Fallout 4, Diagnostics ha anche una scheda Display. Mostra la
dimensione e la modalità della finestra impostate nel `SkyrimPrefs.ini` del profilo
(`EnderalPrefs.ini`, `Fallout4Prefs.ini`), o nella copia del prefisso quando il profilo
non ne ha ancora una. Quando nessuno dei due imposta una dimensione, i suoi campi
partono dallo schermo connesso più grande elencato in `/sys/class/drm`, così uno Steam
Deck legge 1280x800. La larghezza e l'altezza sono numeri interi da 320 a 16384. Apply
scrive `iSize W`, `iSize H`, `bFull Screen` e `bBorderless` nella copia del profilo, dopo
aver adottato la copia del prefisso se il profilo non ne ha una. Quando è SSE Display
Tweaks a impostare la finestra, Apply scrive la stessa dimensione e modalità nella sua
copia nell'Overwrite; il file della mod stessa non viene cambiato.

La scheda Game files in cima è descritta sotto
[File di gioco](#file-di-gioco-linstallazione-dello-store-o-una-copia), la scheda
dell'ordine della collezione sotto
[Mantenere l'ordine della collezione](#mantenere-lordine-della-collezione), e Find the mod
behind a problem sotto [Esecuzioni di prova](#trovare-la-mod-dietro-un-problema).

### Avviare il gioco

Per avviare il gioco attraverso la finestra, imposta l'opzione di avvio Steam del gioco sul
percorso assoluto del programma, perché Steam non legge il `PATH` della tua shell
([install.md](install.md#poi-puntarci-steam)):

```
~/.local/bin/eidos-gui %command%
```

Da Colony, il percorso è `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos si apre
sull'istanza del gioco, quella che hai usato per ultima, così un'istanza portatile viene
ritrovata come quella globale; clicca Run per avviarlo attraverso la vista unita.

Con Start the game at once when a store opens Eidos attivo (Preferences, General, Running
a game; disattivato per impostazione predefinita), Eidos salta quel clic: controlla Health
e avvia il gioco senza mostrare la sua finestra. La finestra si apre come al solito quando
Health elenca un problema, quando l'elenco delle mod richiede attenzione, quando uno
strumento impostato per girare prima di giocare ha un output non aggiornato, oppure, per
Skyrim SE, Enderal SE e Fallout 4, quando il profilo non ha ancora una dimensione della
finestra (la finestra si apre allora su Diagnostics). Per avere la finestra per un solo
avvio, metti `EIDOS_WINDOW=1` prima del comando:
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

In una finestra che nessuno store ha aperto, Run non avvia il gioco da sé, ed Eidos non
avvia mai un gioco direttamente attraverso Proton o Wine. Run chiede allo store che
possiede la copia di avviarlo, e lo store avvia Eidos:

- Steam esegue `steam -applaunch <appid>` quando l'opzione di avvio qui sopra è
  impostata e il gioco ha un prefisso Proton, che Steam crea la prima volta che avvia il
  gioco. Steam parte per primo se è chiuso.
- Heroic (una copia GOG o Epic, nativo o Flatpak) riceve un link `heroic://launch` quando
  Eidos è un wrapper nelle impostazioni del gioco in Heroic, con il percorso assoluto di
  `eidos-gui` (o `eidos-linux`) come eseguibile e nessun argomento. Heroic parte per primo
  se è chiuso.
- Legendary da solo esegue `legendary launch <app>` quando `~/.config/legendary/config.ini`
  ha `wrapper =` e il percorso assoluto di `eidos-gui` (o `eidos-linux`) sotto la sezione
  `[<app>]` del gioco, oppure sotto `[default]`.

Il loader dello script extender, il lanciatore e l'eseguibile del gioco nell'elenco Run
vengono tutti passati allo store in questo modo, e così `eidos tool <instance> run <entry>`
per uno di essi in un terminale. Eidos non può passare alcune copie al loro store, come
una copia GOG fuori da Heroic: Run dice allora di avviare il gioco da quello store.

Il passaggio registra questa istanza come l'ultima che hai usato, e lo store apre poi una
seconda finestra di Eidos su di essa. Con Start the game at once disattivato, clicchi Run
lì; con esso attivo, quella finestra avvia il gioco senza mostrarsi, a meno che uno dei
casi sopra non la apra. La prima finestra non segue quell'esecuzione, quindi premi F5 in
essa dopo aver giocato per ricaricare i suoi elenchi. Quando manca l'impostazione dello
store, o il loader dello script extender non è accanto all'eseguibile del gioco, Run non
avvia nulla e mostra cosa impostare e dove, con il vero percorso del binario in
esecuzione.

Mentre un gioco o uno strumento è in esecuzione, Run diventa Stop. Due pressioni
terminano ogni programma che l'esecuzione ha avviato, ed Eidos cattura poi la sessione e
smonta come dopo qualsiasi uscita; un'esecuzione fermata non conta mai come un avvio pulito
([troubleshooting.md](troubleshooting.md#un-gioco-o-uno-strumento-che-non-si-chiude)).

Il `%command%` di Steam per i titoli Bethesda di solito punta a `<Game>Launcher.exe`.
Eidos non lo esegue mai: il lanciatore è un'app di impostazioni separata che riscansiona
`Data` e riscrive `plugins.txt`, disfacendo l'ordine di caricamento appena distribuito.
Eidos esegue invece il loader dello script extender se ce n'è uno installato, il binario
del gioco altrimenti, e lo dice quando deve ripiegare sul binario del gioco.

Eidos scansiona le DLL delle mod attive cercando un'importazione di `d3dcompiler_47.dll`,
copia la DLL di Microsoft nel prefisso e solo allora imposta
`WINEDLLOVERRIDES=d3dcompiler_47=n,b`; non serve impostarlo tu stesso.

### Variabili e wrapper per profilo

Un profilo può portare le proprie variabili d'ambiente e i propri comandi wrapper in
`profiles/<name>/launch-env.ini`, una voce per riga:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Si applicano a ogni `eidos play` di quel profilo (il gioco avviato dal `%command%` di
Steam, o da Run in una finestra che uno store ha aperto) e a ogni esecuzione di strumento.
`eidos shell` riceve le variabili e nessun wrapper. La prima riga `wrapper=` è la più
esterna. Una riga `wrapper=` che contiene `%command%` riceve il comando in quel punto,
come nelle opzioni di avvio di Steam; una riga senza di esso riceve il comando in fondo. I
wrapper girano dentro lo spazio dei nomi privato di Eidos, quindi vedono le mod.

Una variabile qui sostituisce il valore che Eidos imposta per lo stesso nome, e una riga
`env=` di uno strumento in `tools.ini` sostituisce entrambi. `WINEDLLOVERRIDES` viene
invece aggiunto dopo gli override che Eidos imposta per ENB, ReShade e le DLL incluse,
quindi una DLL nominata in entrambi prende l'impostazione del profilo e le altre
mantengono la propria.

View -> INI editor apre il file come ultima scheda, per ogni gioco. Le righe vuote e le
righe che iniziano con `#` o `;` vengono saltate. Salvando vengono elencati i numeri di
riga che Eidos ignora: righe che non sono né `env=NAME=value` né `wrapper=command`, e una
riga `wrapper=` la cui prima parola è `NAME=value`, come permettono le opzioni di avvio di
Steam; metti invece quella variabile su una riga `env=`. `eidos play <instance>` senza un
comando elenca le variabili che un'esecuzione riceve e i wrapper. Copy report in
Diagnostics nomina le variabili e i programmi wrapper, senza i loro valori.

## Strumenti

xEdit, BodySlide, DynDOLOD e altri strumenti girano attraverso la vista unita dentro il
prefisso Wine del gioco (il prefisso Proton di Steam, o quello che Heroic ha preparato per
una copia GOG o Epic):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # avvia un exe una volta senza registrarlo
eidos prereqs skyrimse            # di cosa hanno bisogno gli strumenti registrati, e il suo stato
eidos prereqs skyrimse --install  # recupera ciò che manca
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen e Synthesis si trovano senza
registrarli. Pandora, Nemesis, FNIS, BodySlide e Synthesis arrivano con una mod di cattura
già impostata, quindi il loro output va in una mod a sé (dettagli in
[tools.md](tools.md#mandare-loutput-di-uno-strumento-in-una-mod-sua)).

Il titolo che dai a uno strumento decide quali runtime e DLL Eidos gli fornisce:
`BodySlide` ottiene le sue librerie DirectX, `BS` non ottiene nulla. Nella finestra,
Executables mostra lo stato reale di ogni prerequisito sotto il campo, e quelli mancanti
sono pulsanti.

Il pacchetto di rilascio contiene gli installatori vcrun2022 (x86 e x64), il .NET 8 Desktop
Runtime 8.0.12 e il runtime .NET 10 10.0.10 in `runtimes/`, e `install.sh` li copia in
`~/.local/share/Colony/Eidos/runtimes/`, quindi `eidos prereqs <instance> --install` li
installa senza un download. Eidos esegue gli installatori vcrun2022 e .NET 8 di Microsoft
senza finestra nel prefisso del gioco, senza winetricks. `eidos-linux` e le compilazioni
dai sorgenti non hanno una cartella `runtimes/`, quindi scaricano da Microsoft gli stessi
file fissati. L'SDK .NET 10 di cui Synthesis ha bisogno (circa 300 MB) è sempre un
download.

La tabella, i tre livelli di prerequisito, perché DynDOLOD ha bisogno di un runtime .NET che
winetricks non può installare, e perché uno strumento installato come mod viene avviato dal
percorso unito invece che dalla propria cartella sono in [tools.md](tools.md).

## Estensioni

Eidos può essere esteso senza essere ricompilato: un manifesto TOML in
`~/.config/Colony/Eidos/addons/` aggiunge uno strumento all'elenco Extensions o un controllo
alla scheda Diagnostics. Nulla viene caricato dentro Eidos: un'estensione è un programma che
esso esegue. Vedi [extensions.md](extensions.md).

## Provare la prova di concetto

Lo script non richiede alcun gioco. Mostra la vista unita, il copy-on-write, una cartella di
gioco lasciata intatta e un'unione che esiste solo dentro il proprio spazio dei nomi, usando
solo OverlayFS senza privilegi in uno spazio dei nomi utente (Linux 5.11 o più recente):

```sh
./scripts/poc-overlay.sh
```
