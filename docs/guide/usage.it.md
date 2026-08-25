<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Usare Eidos

Il manuale pratico: la riga di comando, l'interfaccia grafica, l'opzione di
avvio di Steam, la compilazione dai sorgenti e lo script di prova di concetto.
Per cosa fare quando qualcosa sembra andare storto, vedi
[troubleshooting.it.md](troubleshooting.it.md).

## Usarlo (CLI)

```sh
eidos games                       # i giochi supportati installati qui (come l'elenco di MO2)
eidos init skyrimse               # creare un'istanza di modding
# ...metti ogni mod come cartella dentro <instance>/mods/ (l'istanza globale sta
#    in ~/.local/share/eidos/skyrimse; `eidos init` stampa la tua)...
eidos install skyrimse mod.7z     # oppure installa un archivio scaricato (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adottare ordine e stato dei plugin di un profilo MO2 esistente
eidos sort skyrimse               # ordinare il caricamento dei plugin con LOOT
eidos play skyrimse               # mostrare cosa verrebbe montato
eidos play skyrimse -- <command>  # eseguire <command> con le mod montate sopra il gioco
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` ed `eidos export`
completano l'insieme; esegui `eidos` senza argomenti per l'elenco completo.

### Istanze: globali e portatili

Ogni comando qui sopra si rivolge a un'istanza. `skyrimse` nomina quella
**globale** - conservata in modo centralizzato in
`~/.local/share/eidos/skyrimse`, gestita da Eidos. L'altro tipo è **portatile**:
una cartella autosufficiente dove vuoi tu (un secondo disco, una partizione per
i giochi), spostabile e isolata, esattamente come le istanze portatili di MO2.
Ovunque un comando accetti un identificatore di gioco accetta anche la cartella
di un'istanza portatile:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # creare lì un'istanza portatile
eidos install /mnt/games/EidosSkyrim mod.7z  # ogni comando accetta la cartella
eidos play /mnt/games/EidosSkyrim -- %command%
```

La cartella si descrive da sé (il suo `eidos-instance.ini` nomina il gioco),
quindi non serve altro - e `EIDOS_INSTANCE=<folder>` nell'ambiente reindirizza un
identificatore di gioco a quella cartella, cosa comoda nelle opzioni di avvio di
Steam. Le istanze portatili che hai creato o aperto vengono ricordate (la più
recente per prima) in `~/.config/Colony/Eidos/instances.ini`; la schermata di
benvenuto della GUI le elenca per aprirle con un clic, l'avvio da Steam atterra
su quella con cui hai giocato per ultima e il gestore `nxm://` scarica lì dentro.
Due avvertenze da conoscere: spostare una cartella portatile conserva tutto
tranne le voci degli strumenti che hai registrato con percorsi assoluti nella
vecchia posizione (quelle vanno riaggiunte), e la cache condivisa dei runtime
(`~/.local/share/Colony/Eidos/runtimes/`) resta deliberatamente globale alla
macchina - un host .NET da 78 MB non sta per istanza.

Eidos tiene i propri file sotto `Colony/Eidos`, la disposizione che usa ogni
programma della famiglia Colony: `~/.config/Colony/Eidos/` per quello che hai
scelto (preferenze, la tua sessione Nexus, il tuo elenco di istanze, le
definizioni di giochi e add-on che hai scritto),
`~/.local/state/Colony/Eidos/logs/` per i log di sessione e
`~/.local/share/Colony/Eidos/` per quello che Eidos ha scaricato. Un Eidos più
vecchio li teneva in `~/.config/eidos/` e `~/.local/state/eidos/`; il primo
avvio dopo l'aggiornamento li **copia** e lo scrive nel log. Le vecchie cartelle
restano esattamente com'erano - non viene cancellato nulla, quindi un
aggiornamento andato male non può costarti un accesso - e puoi rimuoverle tu
quando sei soddisfatto.

Le tue mod non fanno parte di tutto questo. Un'istanza globale sta ancora in
`~/.local/share/eidos/<game>/`, e una portatile dove l'hai messa, perché quei
percorsi sono scritti nel tuo elenco di istanze e forse in un'opzione di avvio
di Steam: spostarli spezzerebbe un collegamento di cui Eidos non possiede
entrambi i capi.

Un posto è rifiutato senza appello: **dentro la cartella di installazione di un
gioco** (il riflesso del veterano di MO2). Quell'albero appartiene a Steam - un
aggiornamento, una "verifica dell'integrità" o una disinstallazione possono
riscriverlo o cancellarlo, portandosi via tutta la tua configurazione - ed Eidos
monta sopra la radice del gioco, quindi un'istanza lì dentro starebbe dentro il
proprio bersaglio di mount. La procedura guidata, `eidos init` ed `eidos play`
dicono tutti di no; metti la cartella ACCANTO al gioco (una sorella sullo stesso
disco ti dà la stessa comodità).

`play` monta le mod dell'istanza sopra la directory `Data` del gioco (tramite un
bind-stash, così il demone continua a leggere i file intatti) dentro uno spazio
dei nomi privato, poi esegue il comando attraverso quella vista. Le scritture
(salvataggi, configurazioni rigenerate) finiscono nello strato `overwrite/`
dell'istanza; l'installazione del gioco e ogni sorgente delle mod restano
intatte byte per byte.

### Nessun passaggio privilegiato

Eidos gira interamente senza root. Monta in uno spazio dei nomi privato di utente
+ mount, quindi niente helper setuid, niente demone e niente da concedere.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` è **facoltativo** e comanda
esattamente una cosa: il passthrough FUSE del kernel, disattivato per
impostazione predefinita perché rompe il gioco (sotto). Con la capability Eidos
prende un semplice spazio dei nomi di mount invece di uno di utente; le mod
vengono distribuite in modo identico in entrambi i casi.


Perché il vecchio consiglio su `setcap` non c'è più - e perché il passthrough
FUSE viene spedito disattivato - è spiegato in
[troubleshooting.it.md](troubleshooting.it.md#perché-il-passthrough-è-disattivato-per-impostazione-predefinita).

## GUI

```sh
cargo run -p eidos-gui
```

Una procedura guidata di primo avvio in stile MO2 nell'aspetto pergamena /
bordeaux di Colony: benvenuto -> tipo di istanza (portatile / globale) -> gioco
-> nome e posizione -> riepilogo -> creazione -> schermata principale. La
schermata di benvenuto elenca anche ogni istanza esistente conosciuta (globale e
portatile, la più recente per prima) da aprire con un clic - fa anche da
selettore di istanza - e puntare la procedura guidata a una cartella che
contiene già un'istanza la ADOTTA com'è invece di crearne una sopra (rifiutando
senza appello se la cartella appartiene a un altro gioco).

Anche la finestra principale a due riquadri è fatta: un selettore di profilo
(cambiarlo, o crearne uno nuovo copiando quello corrente), un elenco di mod che
filtri, selezioni, riordini, raggruppi con separatori, restringi per categoria e
su cui apri il menu contestuale per le azioni, più le schede Data / Plugins /
Conflicts / Overwrite / Saves / Downloads / Diagnostics e un pulsante Run con un
selettore del bersaglio da eseguire.

Il riordino non è solo manda in cima o in fondo: ci sono anche gli spostamenti
mirati di MO2 - manda sopra la prima mod in conflitto, sotto l'ultima, a una
priorità esplicita, o dentro il gruppo di un separatore. Passano tutti da un
unico helper di spostamento condiviso, così l'errore di uno che nasce dal
togliere le righe prima di reinserirle esiste in un posto solo invece che in
cinque.

### Colonne, ordinamento e raggruppamento

L'elenco disegna quattro colonne di serie e ne offre otto: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Le spunti nel menu View.
L'impostazione predefinita non è tutte e otto di proposito - un elenco con ogni
colonna visibile non lascia più spazio al NOME, che è la colonna che stai
davvero leggendo.

Clicca una qualsiasi intestazione per ordinare in base a essa. Cliccando di
nuovo si inverte, e un terzo clic torna all'**ordine di caricamento**, che conta
più di quanto sembri: l'ordine di caricamento è l'unico ordine in cui l'elenco
può essere trascinato, perché uno spazio di inserimento si riferisce all'elenco
reale mentre una riga ordinata sta tutt'altrove. Mentre un ordinamento è attivo
le strisce di inserimento non vengono disegnate e un trascinamento viene
rifiutato invece di atterrare dove nessuno mirava - la stessa cosa che fa MO2, e
per la stessa ragione. Il menu View lo dice e offre la strada per tornare
indietro.

Il menu View può anche **raggruppare** tutto l'elenco, per categoria o per
origine (da Nexus, o installate a mano). Le intestazioni di gruppo non sono
separatori: dietro di loro non c'è nulla da rinominare, colorare o spostare, si
ripiegano, e il conteggio resta sull'intestazione quando sono ripiegate. I
separatori restano nell'elenco sotto un ordinamento o un raggruppamento - un
separatore fa da testa alle righe che lo seguono nell'ordine di caricamento, ed
entrambi le hanno spostate.

### Mouse e tastiera

Doppio clic su una mod per Information, Ctrl+doppio clic per la sua cartella,
Shift+doppio clic per la sua pagina Nexus. Ctrl+F mette il cursore nella casella
del filtro. Digitare una lettera salta alla mod successiva che comincia con
quella, e premerla di nuovo percorre le altre invece di restare incollata alla
prima. Nessuno di questi può atterrare su una riga che il filtro, un separatore
ripiegato o un gruppo ripiegato stanno nascondendo - spostare un'evidenziazione
che non vedi è il modo in cui lo Spazio successivo commuta una mod che non stavi
guardando.

"Collapse others" nel menu di un separatore ripiega ogni gruppo tranne quello.
Durante un trascinamento, fermarsi su un gruppo ripiegato lo apre, così una mod
può esservi rilasciata dentro senza abbandonare prima il trascinamento -
fermarsi, non passarci sopra di sfuggita.

### Cosa l'elenco ti dice di una mod

Due segnalazioni informative, entrambe un glifo con la spiegazione al passaggio
del mouse. **No valid game data** significa che niente in cima alla mod sembra
qualcosa che questo gioco carica; potrebbe servire spostare le sue cartelle su
di un livello, oppure potrebbe non essere una mod per questo gioco. **Another
game** significa che il `meta.ini` della mod stessa ne nomina un altro. Nessuna
delle due blocca niente - la mod viene comunque distribuita - e "Mark as valid"
nel menu della riga zittisce l'una o l'altra, attraverso la chiave `validated=`
di MO2, così una mod per cui hai messo la mano sul fuoco in un gestore arriva
silenziosa nell'altro.

Il controllo della disposizione è deliberatamente generoso: un albero `Root/`
conta, una cartella illeggibile conta, una vuota conta. Un avviso sbagliato su
un elenco di cinquecento righe è peggio di uno mancante.

### Fare una copia di una mod prima di toccarla

"Back up this mod" copia la sua cartella da parte come `<name>_backup` (poi
`_backup2`, e così via - una copia non sostituisce mai la precedente). La copia
è **inerte**: non è una mod, la sua casella non fa nulla e non contribuisce in
niente alla vista unita, perché spuntarla distribuirebbe due copie della stessa
mod una sopra l'altra. "Restore this backup over the mod" la rimette al suo
posto, in due clic; il contenuto attuale viene prima spostato da parte e
scartato solo quando la copia è riuscita.

**Data** è un vero albero della vista unita, espanso un livello alla volta così
che aprire un nodo costi una lettura di directory per ogni strato che ce l'ha
invece di una discesa ricorsiva in ogni mod attiva. Risponde dalla STESSA pila
di strati da cui serve il mount, quindi i whiteout e i file nascosti sono
rispettati e la scheda non può essere in disaccordo con quello che vedrà il
gioco. Filtralo per nome, restringilo ai soli file contesi, capisci cosa sta
dove con le colonne Size e Modified, e apri con Reveal qualsiasi riga in un
gestore di file. **Plugins** è l'ordine di caricamento ESP/ESM/ESL (attivare,
riordinare a mano, oppure ordinare con LOOT e leggere il rapporto successivo,
i cui link ai consigli si aprono nel tuo browser). **Conflicts** spiega
vincitori e perdenti file per file. **Overwrite** trasforma in un passo quello
che il gioco ha scritto in una vera mod. **Saves** analizza l'intestazione di
ogni salvataggio - personaggio, livello, luogo, tempo di gioco - e confronta
l'elenco di plugin cotto dentro con quello attuale, con un pulsante che attiva
le mod che gli servono, perché nominarle e lasciare il resto a te è la metà
noiosa.

"Information..." apre una finestra per mod: generale, conflitti, albero dei
file, modifiche agli INI, note. Dall'albero dei file (e dall'albero Data)
qualsiasi file può essere **nascosto** - rinominato in `<name>.mohidden`, cosa
che lo toglie dalla vista virtuale senza cancellarlo, così le tre mesh sparse di
una mod possono essere soppresse senza toccare le priorità. L'albero dei file fa
anche le normali operazioni sui file: nuova cartella, rinomina, cancella, apri.
Passano tutte da un unico risolutore che rifiuta qualsiasi cosa non sia un
percorso semplice dentro quella mod - niente `..`, nessun percorso assoluto e
nessun componente che sia un symlink, dato che seguirne uno porterebbe una
cancellazione del tutto fuori dalla cartella della mod. La rinomina sostituisce
solo l'ultimo componente, così non può mai diventare uno spostamento, e rifiuta
un nome già preso invece di sostituire quel file in silenzio. La cancellazione
richiede due clic; è l'unica azione qui che un altro clic non può annullare.

**View** su qualsiasi riga dell'albero dei file o dell'albero Data mostra
un'anteprima del file: immagini e testo. Non DDS o NIF - servono un decodificatore
a blocchi e un renderer che questo albero non ha - ma lo dicono invece di
mostrare un riquadro vuoto, e rimandano a Reveal. Il testo viene letto fino a
64 KB e dice dove si è fermato, perché un'anteprima è un'occhiata e un log di
Papyrus può pesare cento megabyte. **INI Tweaks** elenca i frammenti che una mod
spedisce nella sua cartella `INI Tweaks/`; quelli attivi vengono uniti all'INI
di gioco del profilo all'avvio, in ordine di priorità, e ritolti quando gli INI
della sessione vengono catturati - altrimenti una modifica diventa
silenziosamente un'impostazione e disattivarla non farebbe nulla.

Un download può essere **trascinato dall'elenco Downloads su una posizione
nell'elenco delle mod** per installarlo a quella priorità, e anche gli archivi o
le cartelle rilasciati sulla finestra da un gestore di file vengono installati
(quella metà richiede una sessione X11 o XWayland - winit implementa il rilascio
di file solo per X11). I download stessi possono essere messi in pausa e
ripresi: la pausa ferma il trasferimento e conserva il parziale, e Resume
risolve di nuovo un link fresco e continua da dove si era fermato.

La scheda Downloads è una **biblioteca** di archivi, non una coda di
trasferimenti. Filtrala per nome (anche il nome leggibile della mod, così
"skyui" trova `SkyUI_5_2_SE-12604-5-2SE.7z`), ordina per più recente, nome,
dimensione o stato, e **nascondi** un archivio con cui hai finito - cosa che
conserva il file e toglie solo la riga, perché mettere via un libro non è
bruciarlo. "Show hidden" li riporta indietro, e lo stesso pulsante li rende di
nuovo visibili. "Remove N installed" cancella gli archivi delle mod che hai già
installato, in due clic, e solo quelli **a schermo**: il filtro è il modo in cui
hai detto quali intendevi.

### Collezioni di Nexus

Incolla il link di una collezione - o cliccane uno sul sito - ed Eidos elenca i
membri della revisione, ciascuno confrontato con questa istanza: installato,
scaricato o mancante. **Legge** una collezione; non ne installa una, e il
riquadro lo dice. Quattro cose rendono un installatore disonesto e non solo
difficile, qui: i membri sono normali file di Nexus che richiedono una chiave
per file che solo un account premium può coniare fuori dal pulsante del sito
stesso; un'installazione completa sono tre chiamate API per membro a fronte di
un budget che questo client si rifiuta di sforare; le fasi del manifest, le sue
regole e le risposte FOMOD riprodotte non hanno potuto essere verificate contro
una vera collezione Bethesda pubblicata, e tirare a indovinare produce un ordine
di caricamento che sembra giusto e non lo è. Leggere costa una richiesta ed è
esatto.

Una collezione può essere letta solo contro **il proprio gioco**. Apri una
collezione di Skyrim con un'istanza Fallout 4 caricata e rifiuta dicendone il
nome invece di confrontare i membri con l'elenco di mod sbagliato, dove ogni
"installato" e ogni "mancante" sarebbe rumore con la forma di una risposta.

### Modalità offline

**Settings -> Nexus -> Offline** impedisce del tutto a Eidos di contattare
Nexus. Il controllo degli aggiornamenti, l'accesso, i download e le collezioni
lo dicono invece di fallire con un errore di connessione. È disattivata a meno
che tu non l'attivi - un file di impostazioni scritto da un Eidos più vecchio
non ha quella chiave, e leggerne una mancante come "attiva" taglierebbe la rete
a chiunque aggiorni.

**Preferred servers** ordina i nodi CDN che un download preferisce, il migliore
per primo. Solo a un account premium viene mai offerto più di un mirror tra cui
scegliere, quindi per tutti gli altri sceglie Nexus e questo non cambia nulla. È
un ordinamento, non un filtro: se oggi nessuno di quelli che hai indicato è
disponibile il download avviene comunque, dal nodo che Nexus ha offerto per
primo.

Le **Categories** sono modificabili, non solo mostrate: assegnale a una mod o a
un'intera selezione, modifica il catalogo stesso dalla stessa finestra, e
scarica da Nexus l'elenco ufficiale delle categorie del gioco. Entrambi i file
di catalogo sono quelli di MO2 (`categories.dat` e `nexuscatmap.dat`), quindi
un'istanza condivisa mantiene un solo catalogo.

**View -> INI editor** modifica gli INI di gioco del profilo - la copia che
resta, non quella sepolta nel prefisso Proton che viene sovrascritta a ogni
avvio. **View -> Log** legge i log di sessione. **View -> Extensions** elenca i
tuoi add-on; vedi [extensions.it.md](extensions.it.md).

L'installazione accetta tutto: i percorsi Simple e FOMOD, più i pacchetti
**BAIN** di Wrye Bash (spunta i sotto-pacchetti, che si uniscono in ordine) e un
selettore **manuale** che mostra l'albero dell'archivio e ti lascia indicare la
radice dei dati quando nessuna euristica riconosce la disposizione. Nessun
archivio viene rifiutato.

**Diagnostics** esegue controlli di salute dal vivo: prima di tutto la capacità
di avviare, i master mancanti (il singolo predittore di crash più affidabile),
gli archivi che nessun plugin attivo caricherà, se l'elenco delle mod
corrisponde ancora alla cartella mods e - dopo una sessione - cosa dice il log
dello script extender su ciascuna delle sue DLL plugin, cosa che trasforma "i
miei plugin SKSE si sono caricati?" da deduzione in prova.

Per avviare il gioco attraverso la GUI, imposta l'opzione di avvio Steam del
gioco al percorso assoluto del binario (Steam non vede `~/.cargo/bin` nel PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos si apre sull'istanza di quel gioco - quella che hai usato per ultima, così
un'istanza portatile viene ritrovata proprio come quella globale; clicca Run per
avviarlo attraverso la vista unita. (Il pulsante Run mostra esattamente questa
riga, con il percorso reale del binario in esecuzione, se lo premi fuori da
Steam.)

Il `%command%` di Steam per i titoli Bethesda di solito punta a
`<Game>Launcher.exe`. Eidos non lo esegue mai: il launcher è un'app di
impostazioni separata che riscansiona `Data` e riscrive `plugins.txt`,
disfacendo l'ordine di caricamento appena distribuito. Al suo posto mette il
loader dello script extender se ne è installato uno, altrimenti il binario del
gioco, e lo dice quando deve ripiegare - un gioco che parte con ogni mod SKSE
inerte è peggio di uno che non parte.

Istruzioni più vecchie qui imponevano `WINEDLLOVERRIDES="d3dcompiler_47=n"`.
Non serve più e non è mai stato del tutto giusto: un override a *native* aiuta
solo se nel prefisso c'è già una vera `d3dcompiler_47.dll`. Adesso Eidos
scansiona gli import DLL delle mod attive, distribuisce lui stesso la vera DLL
Microsoft, e solo dopo imposta l'override.

## Provare la prova di concetto

Non serve nessun gioco. Dimostra unione + copy-on-write + zero-touch + ambito
per spazio dei nomi usando solo OverlayFS non privilegiato in uno spazio dei
nomi utente (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Strumenti

xEdit, BodySlide, DynDOLOD e compagnia girano attraverso la vista unita dentro
il prefisso Proton del gioco:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # cosa serve agli strumenti registrati, e il suo stato
eidos prereqs skyrimse --install  # scaricare quello che manca
```

Una cosa da sapere prima di dare un nome a uno strumento: **il titolo decide
quali DLL di runtime Eidos gli predispone** - `BodySlide` ottiene le sue
librerie DirectX, `BS` non ottiene niente. Nella GUI la finestra Executables
mostra sotto il campo lo stato reale di ogni prerequisito, e quelli mancanti
sono pulsanti.

La tabella, i tre livelli di prerequisiti, perché DynDOLOD ha bisogno di un
runtime .NET che winetricks non sa installare, e perché uno strumento installato
come mod viene avviato dal percorso unito invece che dalla sua cartella sono in
[tools.it.md](tools.it.md).

La compilazione dai sorgenti e la disposizione del repository sono in
[../internals/contributing.md](../internals/contributing.md).

## Estensioni

Eidos può essere esteso senza essere ricompilato: un manifest TOML in
`~/.config/Colony/Eidos/addons/` aggiunge uno strumento all'elenco Extensions o
un controllo alla scheda Health. Niente viene caricato dentro Eidos -
un'estensione è un programma che lui esegue. Vedi
[extensions.it.md](extensions.it.md).
