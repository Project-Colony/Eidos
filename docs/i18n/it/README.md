<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Un gestore di mod nativo per Linux che non tocca mai la cartella del tuo gioco.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installazione)

Eidos dà ai giochi Bethesda su Linux quello che Mod Organizer 2 dà loro su Windows:
una vista unita delle tue mod, costruita a ogni avvio. Usa gli spazi dei nomi di
mount di Linux e FUSE invece dell'hooking delle API di Windows, e il gestore gira
senza Wine. Nulla viene copiato nella cartella del gioco, quindi non c'è niente da
ripulire dopo. Eidos ha anche le definizioni per Stellar Blade, Stardew Valley, 7
Days to Die e Crimson Desert.

Le copie Steam avviano Eidos dall'opzione di avvio del gioco. Le copie GOG ed Epic in
Heroic o Legendary lo avviano come wrapper. Il gioco parte sempre attraverso lo store
che possiede la copia; Eidos non lo avvia mai da sé con Proton.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Stato:** Skyrim SE viene giocato attraverso Eidos ogni giorno, con SKSE,
> preloader per script extender, Creation Club, ordini di caricamento ordinati da
> LOOT e salvataggi per profilo. Finora una sola famiglia di giochi è provata nel
> gioco vero. Altri dieci giochi Bethesda e quattro altri sono collegati e aspettano
> collaudatori.

## Perché Eidos

La vista unita viene montata in uno spazio dei nomi di mount privato che solo
l'albero dei processi del gioco può vedere. Gli altri programmi, un secondo gioco
compreso, non la vedono mai e non hanno bisogno di alcun permesso per essa. Quando il
gioco esce o viene ucciso, o la macchina perde la corrente, lo spazio dei nomi sparisce
con i suoi processi e l'installazione è com'era.

Ogni profilo possiede la sua lista di mod, l'ordine dei plugin, gli INI e i
salvataggi. All'avvio i file dei plugin e la cartella dei salvataggi vengono montati
in bind sopra i percorsi del gioco, così ciò che il gioco scrive lì finisce nel
profilo. Cambiare profilo cambia tutto questo.

Eidos non richiede helper setuid, demoni di sistema, `sudo setcap` né modifiche a
`/etc/fuse.conf`. Il demone FUSE gira come il tuo utente per un solo avvio ed esce con
il gioco. Installi un programma e imposti un'opzione di avvio Steam.

Prima di ogni esecuzione Eidos tiene un'istantanea dell'elenco dei plugin. Se un
crash rovina l'elenco, Diagnostics lo confronta con quell'istantanea, e Restore the
pre-session order rimette il vecchio ordine. Una cattura che cancellerebbe il tuo
ordine di caricamento viene rifiutata, con il motivo.

Test run avvia il gioco con le mod del profilo, senza mod, oppure solo con le mod
selezionate e i loro master. I salvataggi, le modifiche agli INI, lo stato dei
plugin, l'output dell'Overwrite e gli altri file di My Games di quell'esecuzione, log
compresi, vanno in `<instance>/test-run/`. Solo questi vanno lì: altre scritture, come
il registro di Wine, vanno dove vanno sempre. Quando il gioco esce, Eidos elenca quei
cinque gruppi e tieni o butti ciascuno. Find the mod behind a problem, in Diagnostics,
esegue prove con metà delle mod sospette alla volta più i master di cui hanno bisogno.
Dopo ogni esecuzione rispondi se il problema si è presentato, ed Eidos restringe i
sospetti finché non nomina la mod, o le mod che hanno bisogno dei master l'una
dell'altra; 200 sospetti richiedono circa 8 esecuzioni. Le mod selezionate nell'elenco
prima di cominciare restano attive a ogni passo. Dettagli in
[usage.md](docs/guide/usage.md#esecuzioni-di-prova) e
[usage.md](docs/guide/usage.md#trovare-la-mod-dietro-un-problema).

### Come si confronta

| | Eidos | MO2 in Wine | Fluorine-Manager | Limo / deployer a link |
|---|---|---|---|---|
| Il gestore gira in modo nativo | sì | no, un'app Windows in Wine | sì (port Qt) | sì |
| Cartella del gioco intatta | sì, sempre | sì | sì | no, ci vengono scritti dentro dei link |
| Mount visibile a | solo al gioco | solo al gioco | tutto il sistema | n/d |
| Pulizia necessaria dopo un crash | nessuna | nessuna | recupero dei mount rimasti | un-deploy manuale |
| Root mod (ENB, preloader) | sì, in modo nativo | serve un plugin | serve un plugin | parziale |
| Privilegi richiesti | nessuno | nessuno | modifica di `/etc/fuse.conf` | nessuno |

### Quanto è veloce

| | prima | adesso |
|---|---|---|
| caricare un salvataggio | ~20 secondi | 6-7 secondi |
| scansioni di cartelle in una sessione | 5,6 milioni | 335 mila |

I cambi di cella sono immediati. Trovare un file interrogava a turno ciascuna delle
cinquanta mod, ed elencare una cartella lo faceva cinquanta volte di fila. Gli strati in
sola lettura vengono ora indicizzati una volta al mount, quindi trovare un file è una
sola ricerca hash, e anche l'elenco unito di una cartella viene calcolato una volta al
mount. Le cifre vengono da un'istanza vera giocata normalmente, non da un benchmark:
[internals/performance.md](../../internals/performance.md).

### Lavori precedenti e ringraziamenti

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs): la semantica che Eidos riproduce, e
  il codice su cui è stata studiata la sua parità
- [LOOT](https://loot.github.io/): il motore di ordinamento, tramite libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e gli altri gestori per Linux, che mostrano
  che c'è una comunità che vuole vedere risolto questo problema

## Cosa fa

### Mod

Eidos installa archivi semplici, procedure guidate FOMOD, pacchetti BAIN di Wrye Bash,
contenitori OMOD con script OBMM supportati, e tutto il resto attraverso un selettore
manuale. Le root mod (preloader per script extender, ENB, Engine Fixes) funzionano in
modo nativo, senza il plugin Root Builder e senza niente copiato nella tua
installazione. Puoi nascondere singoli file, raggruppare le mod con separatori,
spostare una selezione verso una destinazione, tenere note e categorie per mod e
importare un profilo MO2.

La lista funziona come quella di MO2: otto colonne facoltative e un ordinamento su
ognuna, raggruppamento per categoria o per origine, gesti col doppio clic, salto
digitando, backup per mod che restano inerti finché non li ripristini, e segnalazioni
per una mod il cui layout questo gioco non caricherà o che è stata scaricata per un
altro gioco. I filtri scelgono le mod per stato Nexus (endorsed, tracked, aggiornamento
disponibile), note, file nascosti e tipo di contenuto (plugin, archivi, texture, mesh,
script, suono e altro). L'albero dei file crea cartelle, rinomina, elimina e apre
file, e mostra l'anteprima di immagini e testo senza avviare niente.

Ctrl+Z e Ctrl+Shift+Z annullano e ripetono le modifiche all'elenco delle mod e
all'ordine di caricamento. Disattivare o rimuovere una mod usata da uno dei tre
salvataggi più recenti del profilo chiede prima conferma e nomina i plugin (salvataggi
di Skyrim e Fallout 4). Reinstallare o aggiornare una mod FOMOD apre la procedura
guidata con le risposte della sua ultima installazione. Le informazioni di una mod
elencano le mod che richiede e le mod che la richiedono, lette dai master dei plugin
attivi.

### Plugin

L'ordine di caricamento ha l'ordinamento LOOT integrato, gli indici delle mod come li
calcola il gioco, gli avvisi sui master mancanti, e i tuoi DLC e i contenuti Creation
Club mostrati come righe non gestite. L'intestazione conta gli slot usati in ogni
spazio di indici (full, medium, light). Passando sopra un plugin si vedono il suo
autore, la descrizione e la versione dell'intestazione. Sui motori Skyrim, un plugin il
cui INI omonimo viene caricato dal gioco è segnato.

I plugin che LOOT segnala come sporchi vengono puliti dal QuickAutoClean di xEdit in una
mod `Cleaned plugins`, mai sul posto. Flag as light, in una nuova mod, scrive una copia
light di un plugin, e rifiuta quando uno dei salvataggi più recenti carica quel plugin
come plugin pieno (salvataggi di Skyrim e Fallout 4).

### Istanze

Un'istanza è globale, tenuta sotto `~/.local/share/Colony/Eidos/instances/`, oppure
portatile: una cartella autonoma dove vuoi (un secondo disco, una partizione dei
giochi), spostabile e isolata come quelle di MO2. Le istanze portatili vengono
ricordate da una sessione all'altra. La GUI, l'avvio da Steam e ogni comando della riga
di comando seguono quella che hai usato per ultima, e ogni comando accetta la cartella
ovunque accetti un identificatore di gioco. Dettagli in
[usage.md](docs/guide/usage.md#istanze-globali-e-portatili).

Un'istanza può anche avviare il gioco da una copia della cartella del gioco dentro
l'istanza. Quando Eidos apre per la prima volta una lista Wabbajack, esegue la copia
propria della lista: la cartella che il suo `ModOrganizer.ini` nomina, altrimenti una
cartella `Stock Game` o `Game Root`. Freeze this version, nella scheda Game files in
Diagnostics, copia l'installazione dello store in `<instance>/Stock Game`, così un
aggiornamento dello store non cambia più i file con cui il gioco parte. Quando entrambe
le cartelle stanno su un file system che può condividere blocchi, come btrfs o XFS, la
copia li condivide con l'installazione dello store (reflink). Su ext4, o tra due file
system, è una copia completa, e dimensione e spazio libero vengono mostrati prima. Lo
store avvia comunque il gioco. Return to the store version e Delete the copy annullano
l'operazione, e `eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
fa lo stesso in un terminale. Morrowind, e Oblivion con `bUseMyGamesDirectory=0`, non
possono partire da una copia. Dettagli in
[usage.md](docs/guide/usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia).

### Profili

Ogni profilo ha il proprio ordine delle mod, stato dei plugin, INI e salvataggi. I
salvataggi vengono letti e confrontati con i tuoi plugin attuali, con un pulsante che
attiva ciò che un salvataggio richiede, e copiati nel prefisso per Steam Cloud dopo ogni
sessione. I salvataggi che Steam Cloud ha portato da un altro dispositivo, o che un
avvio senza Eidos ha lasciato nel prefisso, sono elencati nella scheda Saves. Import li
copia nel profilo, e un salvataggio il cui nome il profilo usa già entra con un nuovo
nome `orphan-` invece di sostituirlo.

Un profilo può portare variabili e wrapper di avvio (`env=NAME=value` e
`wrapper=command`, uno per riga, nell'editor INI) per le sue esecuzioni di gioco e di
strumenti. Per Skyrim SE, Enderal SE e Fallout 4, una scheda Display in Diagnostics
imposta la dimensione e la modalità della finestra nelle copie INI del profilo. Backups
tiene i punti di ripristino degli INI del profilo, ed Eidos ne prende uno prima di ogni
esecuzione di BethINI Pie. Il collegamento desktop di un profilo avvia il gioco
attraverso il suo store. `eidos sessions` elenca le esecuzioni registrate e ne confronta
una con lo stato attuale.

### Nexus

Collega un account per ricevere i link Mod Manager Download, controllare gli
aggiornamenti e vedere gli autori e i loro profili. Ogni mod viene controllata,
approvata e collegata sul dominio Nexus da cui veniva il suo file, così una mod di
Skyrim SE in un'istanza Enderal SE resta sul sito di Skyrim SE. Un aggiornamento segue
la catena dei successori che l'autore ha impostato su Nexus, sostituisce la cartella
della mod con il nuovo file (viene offerta una copia della vecchia), mantiene la sua
priorità e il suo stato di attivazione, e ripete le risposte FOMOD memorizzate
all'installazione. `eidos nexus update <instance> --apply` fa lo stesso in un terminale
con un account premium, e stampa le pagine dei file con uno gratuito. Track ed Endorse
sono nel menu della mod, e ogni controllo degli aggiornamenti li sincronizza con il tuo
account Nexus. Le informazioni di una mod hanno una scheda Nexus page con la sua
descrizione e il changelog come testo semplice, e la loro scheda General finisce con ciò
che l'ultimo aggiornamento ha cambiato: i file aggiunti, rimossi e ridimensionati, e i
master aggiunti o rimossi. Install downloads when they finish (Preferences, Nexus,
disattivato per impostazione predefinita) installa un download appena finisce mentre la
finestra è aperta.

Le collezioni si installano dalle loro ricette, con avanzamento ripristinabile e
rapporti espliciti sui passaggi non supportati. Il confronto verifica gli esatti file di
origine; i vecchi metadati incompleti restano non verificati. Il pannello delle
collezioni sfoglia e cerca le collezioni pubblicate per un gioco, mostra quando una
collezione installata ha una revisione più recente, e guida un account gratuito
attraverso i file mancanti una pagina alla volta. Quando un trascinamento rompe le
regole d'ordine di una collezione, Diagnostics lo dice, e Restore collection order
rimette a posto le sue mod. La scheda Downloads filtra, ordina, nasconde e rimuove gli
archivi. Offline mode (Preferences, Nexus) blocca l'accesso a Nexus e il controllo
giornaliero dei rilasci.

### Strumenti

xEdit, BodySlide, DynDOLOD e altri strumenti di modding girano attraverso la vista unita
dentro il prefisso Wine del gioco (il prefisso Proton di Steam, o quello che Heroic ha
preparato). Vedono le tue mod, il loro output finisce nell'Overwrite, e un clic lo
trasforma in una mod vera. Per Skyrim SE, Eidos trova SSEEdit, SSEEdit QuickAutoClean,
PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen, Synthesis e
BethINI Pie nella cartella del gioco, dentro una mod, o nella cartella degli strumenti
che tieni accanto ai tuoi giochi, ciascuno con i runtime di cui ha bisogno. Gli altri
giochi hanno la propria lista. Pandora, Nemesis, FNIS, BodySlide e Synthesis scrivono il
loro output in una mod con nome (`Nemesis Output` e così via).

Il pacchetto di rilascio include gli installatori di Visual C++ 2015-2022 e .NET 8
Desktop Runtime di Microsoft e il runtime .NET 10. Eidos installa `vcrun2022` e .NET 8
con quegli installatori, senza winetricks, e senza il pacchetto scarica gli stessi file
da Microsoft, ciascuno verificato rispetto allo SHA-256 integrato in Eidos. Synthesis
riceve l'SDK .NET 10, un download da 300 MB che il pacchetto non include. Gli altri
prerequisiti (.NET Framework 4.8, XACT e le versioni più vecchie di .NET) passano
ancora da winetricks. Non viene scaricato nulla finché non chiedi di installare i
prerequisiti di uno strumento.

Fissa gli strumenti che usi, nascondi quelli che non usi, dai a uno strumento il suo
AppID Steam quando è un'app Steam a sé, e scrivi un collegamento `.desktop` che lo avvia
attraverso la vista unita senza aprire Eidos. `eidos tool <game> import-mo2` elenca gli
eseguibili nel `ModOrganizer.ini` della cartella di una lista, e `--write` li aggiunge.
Un exe nella scheda Data può essere eseguito una volta o aggiunto come strumento, ed
`eidos tool <instance> run --exe <path>` ne esegue uno da un terminale. Con Ask before
playing, Run chiede prima quando l'output di un generatore non è aggiornato.

Stop termina un gioco o uno strumento in esecuzione. Il menu File esegue winecfg, regedit,
il gestore file di Wine e il programma di disinstallazione di Windows nel prefisso del
gioco. Rebuild prefix, in Diagnostics, rinomina il prefisso perché Steam ne crei uno
nuovo, e non elimina nulla. `eidos shell <instance>` apre una shell nella vista unita, o
esegue il comando indicato dopo `--`, e ciò che scrive finisce nell'Overwrite.

Mentre la finestra è in secondo piano, Eidos può inviare una notifica desktop quando
finisce un'esecuzione di strumento, un'installazione di collezione, un pack o un unpack,
o l'ultimo download in corso, e quando un ordinamento LOOT fallisce. Notify the desktop
when a long job ends, in Preferences, lo attiva; è disattivato per impostazione
predefinita.

### Diagnostica

La scheda Diagnostics elenca i master mancanti, gli archivi orfani, la deriva dell'elenco
delle mod e gli insiemi di plugin danneggiati, e dopo un'esecuzione ciò che il log dello
script extender stesso dice di aver caricato. Segnala inoltre:

- uno script extender mancante o che non corrisponde alla versione del gioco
- un gioco cambiato dall'ultimo avvio pulito
- un'opzione di avvio di Steam che non avvia Eidos o non ha `%command%` dopo di esso
- un Proton più vecchio di quello che ha preparato per ultimo il prefisso, o uno dello
  Steam Flatpak
- `PROTON_ENABLE_NVAPI` mancante per DLSS, e lo schermo intero esclusivo, che blocca la
  generazione di fotogrammi di Community Shaders
- un `enblocal.ini` senza `LinuxVersion=true`
- il log di crash dell'ultima sessione e le mod che nomina
- i requisiti di Nexus mancanti o disattivati
- un gestore `nxm://` che è un altro programma o un file mancante
- le righe `sTestFile`, che fanno ignorare `plugins.txt` al gioco
- i plugin `.esp` attivi in Starfield, e i plugin blueprint non validi o senza coppia
- una patch i cui file sciolti perdono contro la mod che modifica
- una mod le cui copie degli script Papyrus dello script extender vincono su quelle
  dell'extender stesso
- un `vm.max_map_count` o un limite di file aperti bassi
- i file di gioco su un file system Windows
- i percorsi dati di BodySlide che non sono la cartella Data del gioco, e le cartelle in
  `Overwrite/Root` che appartengono a Data
- il `ModOrganizer.ini` di una lista che punta gli strumenti a un'altra cartella di gioco
- una copia del gioco mancante o senza eseguibile del gioco

`eidos check <instance> [--json]` esegue i controlli che non richiedono la finestra ed
esce con 1 quando trova un problema. Copy report copia un rapporto con la tua cartella
home scritta come `~` e il tuo nome utente come `<user>`, e Save tree nella scheda Data
scrive l'albero Data unito con la provenienza di ogni file, reso anonimo allo stesso
modo.

Vedi [Controlli di integrità e file generati](../../../docs/internals/integrity.md) (in
inglese) per la protezione delle sostituzioni, i conflitti fra file BSA/BA2, la
diagnostica statica SKSE e i registri di generazione, inclusi i loro limiti.

### Dove tiene i propri file

Eidos usa lo schema che usa ogni programma della famiglia Colony:
`~/.config/Colony/Eidos/` per ciò che hai scelto (preferenze, la tua sessione Nexus, la
tua lista di istanze, le definizioni di giochi e componenti aggiuntivi che hai scritto),
`~/.local/share/Colony/Eidos/` per le istanze globali, i runtime degli strumenti
(scaricati, o copiati dal pacchetto da `install.sh`) e i log di sessione, e
`~/.cache/Colony/Eidos/` per le masterlist di LOOT. Dentro un'istanza, `Stock Game`
contiene una copia congelata del gioco e `test-run/` contiene un'esecuzione di prova in
attesa di revisione.

Un Eidos più vecchio teneva i suoi file in `~/.config/eidos/`, `~/.local/share/eidos/` e
`~/.local/state/`. Il primo avvio dopo l'aggiornamento li trasferisce, lo scrive nel log
e lascia le vecchie cartelle com'erano. Finché un trasferimento non è completo, Eidos
continua a usare i vecchi file. Dettagli in
[internals/paths.md](../../internals/paths.md) (in inglese).

### Giochi supportati

Skyrim SE/AE è provato nel gioco vero. Anche Fallout 4 è collegato da un capo all'altro
(F4SE sostituito automaticamente, invalidazione degli archivi, ordine di caricamento con
asterisco, LOOT, salvataggi `.fos`); vedi [guide/fallout4.md](docs/guide/fallout4.md).
Collegati e in cerca di collaudatori: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR,
Fallout 3, Fallout NV, Starfield, Oblivion e Morrowind. Fallout 3, Fallout NV, Oblivion e
Morrowind ordinano i plugin per data dei file: Eidos tiene un ordine per profilo e mostra
quelle date dentro il mount, senza modificare i file su disco.

Altri quattro giochi sono collegati e cercano anch'essi collaudatori: Stellar Blade (mod
Unreal `.pak` in `~mods`), Stardew Valley (mod SMAPI, con SMAPI installato a parte), 7
Days to Die (cartelle `Mods/` accanto al gioco) e Crimson Desert. Crimson Desert accetta
solo mod di codice: plugin ASI, ReShade e DLL di loader in `bin64`. Le sue mod di dati,
che stanno in gruppi PAZ, non sono supportate.

Un gioco è un solo file TOML, e il tuo può andare in `~/.config/Colony/Eidos/games/` senza
ricompilare: [internals/adding-games.md](../../internals/adding-games.md).

## Installazione

### Tramite Colony (consigliato)

Cerca Eidos in [Colony](https://github.com/Project-Colony/Colony) e installalo. Gli
aggiornamenti arrivano attraverso il launcher. Colony installa un solo file,
`eidos-linux`: la finestra, e anche la riga di comando `eidos` quando viene richiamato
attraverso un link chiamato `eidos`. Quel file non contiene né il programma ausiliario
per l'anteprima dei modelli NIF né i runtime Microsoft del pacchetto qui sotto. Senza
il programma ausiliario l'anteprima lo dice e tutto il resto funziona, ed Eidos scarica
un runtime da Microsoft quando installi i prerequisiti di uno strumento. Cosa fare dopo
l'installazione, e come puntarci Steam:
[docs/guide/install.md](docs/guide/install.md#tramite-colony).

### Download diretto del binario

Prendi un file dall'[ultimo rilascio](https://github.com/Project-Colony/Eidos/releases/latest):

| File | Cos'è |
|---|---|
| `eidos-linux` | l'intero programma in un solo file, come lo installa Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, il programma ausiliario per l'anteprima NIF, gli installatori di Visual C++ 2015-2022 e .NET 8 Desktop Runtime e il runtime .NET 10, le icone, gli avvisi di licenza e un installatore che registra la voce desktop e il gestore dei download di Nexus |

Verifica prima il download, come mostra la
[Politica di firma del codice](#politica-di-firma-del-codice), poi:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

L'installatore mette i binari in `~/.local/bin` senza sudo e copia la cartella
`runtimes/` del pacchetto in `~/.local/share/Colony/Eidos/runtimes`. `--system` e
`--bindir DIR` mettono i binari altrove, e `--from DIR` li prende da un'altra cartella.
`--cap` chiede a sudo `CAP_SYS_ADMIN`, che serve solo al passthrough FUSE sperimentale,
e il passthrough resta spento; `--no-cap` è l'impostazione predefinita. Entrambi i file
richiedono Linux x86_64 con glibc 2.39 o più recente, FUSE (`fusermount3`) e 7-Zip per
gli archivi delle mod.

Eidos non si aggiorna da solo. Una volta al giorno all'avvio, la finestra chiede a
GitHub l'ultimo rilascio, e quando ne esce uno più recente mostra un link alle sue note
di rilascio dopo Help nella barra dei menu. Preferences lo disattiva, e una copia sotto
`/usr` fuori da `/usr/local`, che un gestore di pacchetti aggiorna, non lo chiede mai.

### Compilare dai sorgenti

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Richiede Rust 1.92 o più recente (la `rust-version` che la CI verifica; CI e rilasci
compilano con 1.99.1), più `just`, CMake 3.20+ e un compilatore C++17 per il programma
ausiliario dell'anteprima NIF. `just build` usa il profilo release: una sola unità di
codegen e LTO completo. Una compilazione a freddo di `eidos` e `eidos-gui` con 3 job di
compilazione ha richiesto 334 secondi così e 243 secondi con LTO thin, e l'LTO completo
rende `eidos` più piccolo di 1,2 MB ed `eidos-gui` di 3,8 MB (le misure sono in
`Cargo.toml`). `just profile=dev build` compila più in fretta. Un pacchetto Arch si
compila da `packaging/PKGBUILD`. Prerequisiti e i comandi senza `just`:
[docs/guide/install.md](docs/guide/install.md).

### Opzioni di avvio Steam

Eidos gira come comando di avvio del gioco, ed è così che riesce a montare prima che il
gioco parta. La riga di base è tutto ciò che serve alla maggior parte delle
configurazioni, con il percorso assoluto della tua installazione (Steam non legge il tuo
`PATH`):

```
~/.local/bin/eidos-gui %command%
```

Da Colony quel percorso è `~/.local/share/Colony/apps/Eidos/eidos-linux`. Poi premi
Gioca: Eidos si apre sull'istanza di quel gioco, e Run avvia il gioco attraverso la vista
unita.

In una finestra che hai aperto tu, Run chiede a Steam di avviare il gioco
(`steam -applaunch <appid>`), quindi l'opzione di avvio vale ancora, e premi Run nella
finestra che Steam apre. Heroic riceve un link `heroic://launch`, e Legendary esegue
`legendary launch <app>`, una volta che Eidos è impostato come wrapper del gioco lì.
Quando manca l'impostazione dello store, Run non avvia nulla e dice cosa impostare:
[guide/usage.md](docs/guide/usage.md#avviare-il-gioco). Con Start the game at once when a
store opens Eidos (Preferences, disattivato per impostazione predefinita), Eidos esegue i
controlli di Diagnostics e avvia il gioco senza mostrare la sua finestra quando nulla
richiede attenzione.

Tutto il resto sono variabili d'ambiente impilate davanti, e si combinano liberamente:

| Vuoi... | Metti davanti |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1`. Senza di essa DLSS non si inizializza mai e non dice nulla; la lista completa è [guide/graphics.md](docs/guide/graphics.md) |
| un contatore di FPS sullo schermo | `DXVK_HUD=fps` |
| interpolazione dei fotogrammi a livello di driver, zero mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, mai insieme alla generazione di fotogrammi di Community Shaders |
| log dettagliati per una segnalazione di bug | `EIDOS_LOG=debug` (i log di sessione finiscono in `~/.local/share/Colony/Eidos/logs/`) |
| un rapporto di I/O per sessione dal mount | `EIDOS_FUSE_STATS=1` |
| un numero diverso di worker FUSE | `EIDOS_FUSE_THREADS=8` (4 per impostazione predefinita; `1` è la prima cosa da provare quando insegui un bug di concorrenza) |
| questo avvio fissato a una sola istanza portatile | `EIDOS_INSTANCE=/path/to/folder`. Senza di essa Eidos apre l'istanza che hai usato per ultima |
| la finestra per un solo avvio mentre Start the game at once è attivo | `EIDOS_WINDOW=1` |

La riga da tenere per una configurazione moddata moderna (Community Shaders, DLSS,
generazione di fotogrammi) è il comando finale, non un esempio:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Aggiungi `DXVK_HUD=fps` davanti mentre verifichi che la configurazione funzioni, e toglilo
quando funziona.

Le stesse variabili possono andare nelle variabili di avvio del profilo invece che nella
riga di Steam, il che le tiene per profilo e le applica anche alle esecuzioni degli
strumenti:
[guide/usage.md](docs/guide/usage.md#variabili-e-wrapper-per-profilo).

Gli interruttori diagnostici più profondi (`EIDOS_FUSE_TRACE`, le levette di bisezione
della cache e dell'indice, perché `EIDOS_FUSE_PASSTHROUGH` è disattivato per impostazione
predefinita) sono in [guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentazione

Tutto è in [docs/](docs/), con un indice ordinato per chi legge in
[docs/README.md](docs/README.md): la guida per l'utente (installazione, la riga di
comando e la GUI, strumenti come xEdit e DynDOLOD, grafica, risoluzione dei problemi),
come funziona Eidos all'interno, e perché esiste. Politica di sicurezza e come segnalare
una vulnerabilità: [SECURITY.md](../../../SECURITY.md).

## Lingua

Le pagine che servono a un giocatore sono tradotte. L'inglese è il riferimento: quando
una traduzione è in disaccordo con esso, ha ragione il file inglese.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Tutto il resto è in inglese. `docs/internals/` e `docs/project/` vengono letti da persone
che stanno leggendo anche il Rust, `CHANGELOG.md` è generato e nemmeno `SECURITY.md` è
tradotto. Tradurre i primi due aggiungerebbe più di 19.000 parole da tenere
sincronizzate per lettori che non ne hanno bisogno.

Ogni traduzione porta l'hash del file inglese da cui è stata fatta, e la CI fallisce
finché quell'hash non corrisponde più all'inglese; vedi
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). La traduzione viene poi
aggiornata e ristampata con `scripts/i18n-check.sh --fix`. Una traduzione scaduta viene
aggiornata, non rimossa.

Una lingua è una sola cartella. `docs/i18n/<lang>/` rispecchia la radice del repository,
così `docs/i18n/de/docs/guide/install.md` è il `docs/guide/install.md` tedesco. È per
questo che un collegamento fra due pagine tradotte è la stessa stringa del collegamento
fra i loro originali inglesi, e che rimuovere una lingua è un solo `rm -r`.

## Politica di firma del codice

Ogni file di rilascio è firmato dall'organizzazione Project Colony nella CI, mai
sulla macchina di uno sviluppatore. Accanto a ogni file, nella pagina del rilascio:

| File | Cos'è |
|---|---|
| `<asset>.sig` | una firma ed25519 del file, fatta con la chiave di rilascio dell'organizzazione |
| `<asset>.meta` | tre righe che legano il file al suo nome, al suo sha256 e alla versione del rilascio |
| `<asset>.meta.sig` | una firma ed25519 del `.meta` |

La chiave privata è un segreto dell'organizzazione, usato solo dal
[workflow sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
condiviso, in un job che non compila niente; i job che compilano Eidos non la
vedono mai. Colony verifica tutti e tre i file prima di installare o aggiornare
Eidos, e rifiuta un rilascio più vecchio di quello installato. Per verificare un
download da te con OpenSSL 3 (gli stessi comandi valgono per ogni file):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Come i rilasci vengono compilati, firmati e pubblicati:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacy

Eidos non invia telemetria, né statistiche d'uso, né rapporti sui crash. Si collega solo
ai server di questa tabella, e solo per l'azione indicata. Offline mode (Preferences ->
Nexus) blocca ogni connessione a Nexus e il controllo giornaliero dei rilasci.

| Dati | Conservati o inviati | Dove, e perché |
|---|---|---|
| Preferenze, lista delle istanze, definizioni di giochi e componenti aggiuntivi | conservati | `~/.config/Colony/Eidos/` |
| Istanze, runtime degli strumenti, log di sessione | conservati | `~/.local/share/Colony/Eidos/`, o la cartella che hai scelto per un'istanza portatile |
| Masterlist di LOOT | conservati | `~/.cache/Colony/Eidos/loot/` |
| Controllo dei rilasci | inviati | All'avvio, al massimo una volta al giorno, una richiesta a `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos legge solo il tag del rilascio e non scarica nulla. È attivo per impostazione predefinita e si disattiva in Preferences, e viene saltato in Offline mode e per una copia installata sotto `/usr` fuori da `/usr/local`, che un gestore di pacchetti aggiorna. [Informativa sulla privacy di GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Accesso a Nexus | inviati | L'accesso apre `users.nexusmods.com` nel tuo browser, che restituisce il risultato a un listener che Eidos apre su `127.0.0.1` solo per quell'accesso. Eidos lo scambia poi con dei token su `users.nexusmods.com/oauth/token` e recupera le chiavi pubbliche con cui Nexus li firma. [Informativa sulla privacy di Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenziali Nexus | conservati | I token di accesso e di rinnovo, in `~/.config/Colony/Eidos/nexus.ini` con modalità 0600. Una riga `api_key=` scritta lì da un Eidos più vecchio viene conservata ma mai letta; eliminala pure, se vuoi. |
| Richieste a Nexus | inviati | `api.nexusmods.com`, quando usi una funzione di Nexus: il nome del tuo account e lo stato premium, i dettagli di mod e file, i controlli degli aggiornamenti, le collezioni e i link di download. Le tue approvazioni e le mod seguite vengono lette a ogni controllo degli aggiornamenti e scritte quando premi Endorse o Track. I requisiti delle mod, le descrizioni e i changelog, e la ricerca delle collezioni, vanno a `api.nexusmods.com/v2/graphql`. Ogni richiesta porta il tuo token di accesso e indica Eidos e la sua versione. |
| Download da Nexus | inviati | I file che chiedi, dall'host CDN di Nexus indicato dal link di download, salvati nella cartella dei download dell'istanza con un `.meta` in formato MO2 accanto a ciascuno. |
| Link `nxm://` | conservati | `eidos nxm --register`, che `install.sh` esegue per te, scrive `~/.local/share/applications/eidos-nxm.desktop` e lo rende il gestore di `nxm://` tramite `xdg-mime`, così il pulsante Mod Manager Download del sito passa il suo link a Eidos. Il pacchetto Arch installa la stessa voce per tutto il sistema. |
| Masterlist di LOOT | inviati | `raw.githubusercontent.com/loot/...`, al primo ordinamento LOOT di un gioco e ogni volta che chiedi un aggiornamento. Non viene inviato niente di tuo. [Informativa sulla privacy di GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtime degli strumenti | inviati | Solo quando installi i prerequisiti di uno strumento, e solo per ciò che il pacchetto di rilascio non includeva (il file singolo `eidos-linux` e una build dai sorgenti non includono nulla): il runtime .NET 10 da `builds.dotnet.microsoft.com` e gli installatori di Visual C++ e .NET 8 da `download.visualstudio.microsoft.com`, ciascuno rifiutato se non corrisponde allo SHA-256 integrato in Eidos; l'SDK .NET 10, sempre da `builds.dotnet.microsoft.com` e verificato allo stesso modo; e `winetricks` per gli altri prerequisiti (.NET Framework 4.8, XACT, .NET 6 e 7), che scarica ciò che installa da Microsoft e dagli altri host che winetricks usa. [Informativa sulla privacy di Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Log di sessione | conservati | `~/.local/share/Colony/Eidos/logs/`, modalità 0600: dieci sessioni per istanza, eliminate dopo 30 giorni. La tua cartella home è scritta come `~` e il tuo nome utente come `<user>`, così un log può essere incollato in una segnalazione così com'è. Non lasciano mai la tua macchina a meno che tu non li invii. |
| Record delle esecuzioni | conservati | `<instance>/logs/sessions/`: per ogni esecuzione di gioco o di strumento, il profilo, le mod attive con le loro versioni e i loro id di file Nexus, l'ordine dei plugin, un checksum di ogni INI del profilo, le versioni del gioco e di Eidos, il codice di uscita e i salvataggi che l'esecuzione ha creato. Vengono tenuti i 50 più recenti, li legge `eidos sessions`, e `eidos pack` li lascia fuori. Restano sulla tua macchina. |
| Rapporti sugli aggiornamenti | conservati | `<instance>/logs/updates/<mod>.json`: ciò che l'ultimo aggiornamento o Replace ha cambiato nella cartella di quella mod. `eidos pack` li lascia fuori. |
| Collegamenti | conservati | `install.sh` scrive `eidos.desktop` e le sue icone sotto `~/.local/share`; un collegamento a uno strumento o a un profilo che crei va in `~/.local/share/applications`. |
| Notifiche desktop | nessuno dei due | Disattivate per impostazione predefinita. Quando sono attive, vanno al servizio di notifiche del tuo desktop tramite D-Bus (`org.freedesktop.Notifications`) e nominano il lavoro che è finito. |
| Link che apri | nessuno dei due | Le pagine delle mod e gli altri link si aprono nel tuo browser tramite `xdg-open`. |
| Componenti aggiuntivi | nessuno dei due | Le estensioni che installi girano come programmi propri sulla tua macchina; ciò che fanno è affar loro. |

## Licenza

Il codice originale di Eidos è sotto licenza GPL-3.0-or-later; vedi
[LICENSE](../../../LICENSE). Queste parti sono sotto GPL-3.0 senza l'opzione "or later":

- le traduzioni degli script OMOD in `eidos-install`; vedi i loro
  [crediti degli autori originali](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- il programma ausiliario `eidos-nif-preview`, costruito sulla libreria nifly inclusa nel
  repository; vedi le
  [licenze e i crediti dei suoi componenti](../../../crates/eidos-nif-preview/UPSTREAM.md);
- i crate `esplugin` e `libloadorder` a cui si collega il codice dell'ordine di
  caricamento, che dichiarano `GPL-3.0`.

Gli altri crate Rust nei binari mantengono le proprie licenze (MIT, Apache-2.0, BSD e
simili). Il pacchetto di rilascio porta i loro avvisi in `licenses/crates/`, con un
indice di ogni crate e della sua licenza.

I binari incorporano anche otto DLL redistribuibili originali di Microsoft DirectX
(`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` e `d3dx11_43`, a 32 e 64 bit),
conservate compresse con gzip (7,9 MB invece di 17,9 MB), che Eidos decomprime e
distribuisce nel prefisso del gioco quando una mod o uno strumento ne ha bisogno. Restano
sotto i termini di redistribuzione di Microsoft; vedi la loro
[provenienza](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Il pacchetto di rilascio porta anche, sotto `runtimes/`, gli installatori di Visual C++
2015-2022, l'installatore del .NET 8 Desktop Runtime e lo zip del runtime .NET 10, così
come li pubblica Microsoft. I loro termini sono in `licenses/microsoft-runtimes/`
(`vc_redist.txt`, `dotnet.txt`).
