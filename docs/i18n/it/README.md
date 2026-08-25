<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Il gestore di mod nativo per Linux che non tocca mai il tuo gioco.**

</div>

Eidos dà ai giochi Bethesda su Linux quello che Mod Organizer 2 dà loro su
Windows - una vista unita e virtuale delle tue mod, rifatta a ogni avvio -
costruita con le primitive di Linux invece che con l'hooking delle API di
Windows. Niente Wine per il gestore. Nessun file copiato nella cartella del
gioco. Nessuna procedura di pulizia, perché non c'è niente da pulire.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Stato:** Skyrim SE viene giocato attraverso Eidos tutti i giorni - SKSE,
> preloader per script extender, Creation Club, ordini di caricamento ordinati
> da LOOT, salvataggi per profilo, tutto quanto. Finora una sola famiglia di
> giochi provata nel gioco vero; altre dieci sono collegate e aspettano
> collaudatori.

## Perché Eidos

- 🔒 **Un mount che solo il tuo gioco può vedere.** La vista unita vive in uno
  spazio dei nomi di mount privato: il tuo file manager, il tuo backup, un
  secondo gioco - nessuno di loro la vede, nessuno di loro ha bisogno di
  permessi per essa. Chiudi il gioco a forza, stacca la corrente: lo spazio dei
  nomi muore con l'albero dei processi e la tua installazione è esattamente
  com'era. Non c'è residuo *per costruzione*.
- 🧾 **Una sola copia della verità.** Il tuo profilo possiede la sua lista di
  mod, l'ordine dei plugin, gli INI e i salvataggi. I file dei plugin e la
  cartella dei salvataggi vengono montati in bind sopra i percorsi del gioco
  all'avvio, così anche le scritture del gioco stesso atterrano nel tuo profilo.
  Cambiare profilo cambia tutto.
- 🐧 **Completamente senza root.** Nessun helper setuid, nessun demone, nessun
  `sudo setcap`, nessuna modifica a `/etc/fuse.conf`. Un eseguibile,
  un'opzione di avvio Steam.
- 🛡️ **Protezioni che portano le prove.** Un crash che rovina la tua lista di
  plugin viene segnalato rispetto a un'istantanea presa prima della sessione,
  con un ripristino in un clic. Una cattura che cancellerebbe il tuo ordine di
  caricamento viene rifiutata e dice perché.

## Cosa fa

**Mod.** Archivi semplici, procedure guidate FOMOD, pacchetti BAIN di Wrye Bash,
un selettore manuale per il resto - e le **root mod in modo nativo** (preloader
per script extender, ENB, Engine Fixes), senza il plugin Root Builder e senza
niente copiato nella tua installazione. Nascondi singoli file, raggruppa con i
separatori, spostamenti mirati, note e categorie per mod, e un importatore di
profili MO2.

La lista è quella di MO2, con le sue abitudini: otto colonne facoltative e
l'ordinamento su ognuna di esse, raggruppamento per categoria o per origine,
gesti col doppio clic, salto digitando, backup per mod che restano inerti finché
non li ripristini, e segnalazioni per una mod il cui layout questo gioco non
caricherà o che è stata scaricata per un altro. Il suo albero dei file fa le
operazioni ordinarie - nuova cartella, rinomina, elimina, apri - e mostra
l'anteprima di immagini e testo senza avviare niente.

**Plugin.** L'ordine di caricamento con l'ordinamento LOOT integrato, gli indici
delle mod come li calcola il gioco, gli avvisi sui master mancanti, e i tuoi DLC
e i contenuti Creation Club mostrati per le righe non gestite che sono.

**Istanze.** Globali - gestite centralmente sotto `~/.local/share/eidos` - o
portatili: una cartella autonoma dove vuoi (un secondo disco, una partizione dei
giochi), spostabile e isolata, come quelle di MO2. Le istanze portatili vengono
ricordate da una sessione all'altra; la GUI, l'avvio da Steam e ogni comando
della riga di comando seguono quella che hai usato per ultima, e ogni comando
accetta la cartella ovunque accetti un identificatore di gioco. I dettagli in
[usage.it.md](docs/guide/usage.md#istanze-globali-e-portatili).

**Profili.** Ordine delle mod, stato dei plugin, INI e salvataggi per profilo. I
salvataggi vengono letti, confrontati con i tuoi plugin attuali - con un
pulsante che abilita ciò che un salvataggio richiede - e risincronizzati per
Steam Cloud dopo ogni sessione.

**Nexus.** Collega un account e il pulsante "Mod Manager Download" del sito
atterra direttamente nella tua istanza, con il controllo degli aggiornamenti
rispetto a ciò che hai installato, chi ha fatto ogni mod e un link al suo
profilo. Il link a una **collezione** elenca i suoi membri incrociati con la tua
istanza - installati, scaricati, mancanti - il che è leggere una collezione, non
installarla, e il pannello dice perché. La scheda Downloads è una biblioteca di
archivi: filtra, ordina, nascondi senza eliminare, e ripulisci quelli già
installati. Un interruttore **offline** ferma tutto questo.

**Strumenti.** xEdit, BodySlide, DynDOLOD e compagnia girano *attraverso la
vista unita* dentro il prefisso Proton del gioco - vedono le tue mod, il loro
output atterra in Overwrite, e un clic lo trasforma in una mod vera. Qualunque
runtime serva a ciascuno viene scaricato su richiesta, così una DLL mancante è
un pulsante invece che un pomeriggio. xEdit e il suo gemello QuickAutoClean
vengono trovati per te - nella cartella del gioco, dentro una mod, o nella
cartella degli strumenti che tieni accanto ai tuoi giochi - con i runtime giusti
già scelti. Fissa quelli che usi, nascondi quelli che non usi, dai a uno
strumento il suo AppID Steam quando è un'app Steam a sé, e scrivi un collegamento
`.desktop` che lo avvia attraverso la vista unita senza aprire Eidos affatto.

**Diagnostica.** Master mancanti, archivi orfani, deriva della lista delle mod,
insiemi di plugin danneggiati - e, dopo un'esecuzione, cosa dice il log dello
script extender su ciò che è stato davvero caricato.

**Dove tiene i propri file.** `~/.config/Colony/Eidos/` per ciò che hai scelto -
preferenze, la tua sessione Nexus, la lista delle istanze, le definizioni di
giochi e add-on che hai scritto - con i log sotto
`~/.local/state/Colony/Eidos/`. Lo schema che usa ogni programma della famiglia
Colony. Un Eidos più vecchio li teneva in `~/.config/eidos/`; il primo avvio
dopo l'aggiornamento li copia, lo scrive nel log, e lascia la vecchia cartella
esattamente com'era.

## Come si confronta

| | Eidos | MO2 in Wine | Fluorine-Manager | Limo / deployer a link |
|---|---|---|---|---|
| Il gestore gira in modo nativo | ✅ | ❌ app Windows in Wine | ✅ (port Qt) | ✅ |
| Cartella del gioco intatta | ✅ sempre | ✅ | ✅ | ❌ ci scrive dentro dei link |
| Mount visibile a | solo al gioco | solo al gioco | **tutto il sistema** | n/d |
| Pulizia necessaria dopo un crash | nessuna, per progetto | nessuna | recupero dei mount rimasti | un-deploy manuale |
| Root mod (ENB, preloader) | ✅ nativo | serve un plugin | serve un plugin | parziale |
| Privilegi richiesti | nessuno | nessuno | modifica di `/etc/fuse.conf` | nessuno |

## Quanto è veloce

| | prima | adesso |
|---|---|---|
| caricare un salvataggio | ~20 secondi | **6-7 secondi** |
| letture di cartelle in una sessione | 5,6 milioni | 465 mila |

I cambi di cella sono immediati. Il guadagno è arrivato ponendo meno domande
alle tue mod: trovare un file interrogava tutte e cinquanta a turno, ed elencare
una cartella lo faceva cinquanta volte di fila. Né l'una né l'altra cosa
accadono più. Misurato su un'istanza vera giocata normalmente, non su un
benchmark.

## Iniziare

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Poi imposta l'opzione di avvio Steam del tuo gioco su
`~/.local/bin/eidos-gui %command%` e premi Gioca.

Pacchetti Arch e archivi di rilascio, cosa ti serve installato prima, e la via
della riga di comando: **[docs/guide/install.it.md](docs/guide/install.md)**.

## Opzioni di avvio Steam

La riga di base è tutto ciò che serve alla maggior parte delle configurazioni:

```
~/.local/bin/eidos-gui %command%
```

Tutto il resto sono variabili d'ambiente impilate davanti, e si combinano
liberamente:

| Vuoi... | Metti davanti |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1` - senza di essa DLSS non si inizializza mai, in silenzio; la lista completa è [guide/graphics.it.md](docs/guide/graphics.md) |
| un contatore di FPS sullo schermo | `DXVK_HUD=fps` |
| interpolazione dei fotogrammi a livello di driver, zero mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - mai insieme alla generazione di fotogrammi di Community Shaders |
| log dettagliati per una segnalazione di bug | `EIDOS_LOG=debug` (i log di sessione atterrano in `~/.local/state/Colony/Eidos/logs/`) |
| un rapporto di I/O per sessione dal mount | `EIDOS_FUSE_STATS=1` |
| un numero diverso di worker FUSE | `EIDOS_FUSE_THREADS=8` (4 per impostazione predefinita; `1` è la prima cosa da provare quando insegui un bug di concorrenza) |
| questo avvio fissato a una sola istanza portatile | `EIDOS_INSTANCE=/path/to/folder` - senza di essa Eidos apre l'istanza che hai usato per ultima, che di solito è ciò che vuoi |

La riga da tenere per una configurazione moddata moderna (Community Shaders,
DLSS, generazione di fotogrammi) - questo è il comando finale, non un esempio:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Aggiungi `DXVK_HUD=fps` davanti mentre verifichi che la configurazione funzioni,
toglilo una volta che funziona.

Gli interruttori diagnostici più profondi (`EIDOS_FUSE_TRACE`, le levette di
bisezione della cache e dell'indice, perché `EIDOS_FUSE_PASSTHROUGH` è
disattivato per impostazione predefinita) vivono in
[guide/troubleshooting.it.md](docs/guide/troubleshooting.md).

## Dove andare poi

| Se vuoi... | |
|---|---|
| installarlo | [guide/install.it.md](docs/guide/install.md) |
| imparare la riga di comando e la GUI | [guide/usage.it.md](docs/guide/usage.md) |
| configurare xEdit, BodySlide o DynDOLOD | [guide/tools.it.md](docs/guide/tools.md) |
| giocare a Fallout 4 (F4SE, versioni, il crash dei detriti NVIDIA) | [guide/fallout4.it.md](docs/guide/fallout4.md) |
| far funzionare DLSS / la generazione di fotogrammi (Community Shaders) | [guide/graphics.it.md](docs/guide/graphics.md) |
| riparare qualcosa che sembra sbagliato | [guide/troubleshooting.it.md](docs/guide/troubleshooting.md) |
| sapere perché è veloce, e verificarlo di persona | [internals/performance.md](../../internals/performance.md) |
| capire come funziona dentro | [internals/architecture.md](../../internals/architecture.md) |
| compilarlo, testarlo, contribuire | [internals/contributing.md](../../internals/contributing.md) |
| sapere perché esiste | [project/landscape.md](../../project/landscape.md) |

Una lingua è una sola cartella: `docs/i18n/it/` rispecchia la radice del
repository, il che rende un collegamento fra due pagine tradotte identico a quello
fra i loro originali inglesi.

## Lingua

Le pagine che servono a un giocatore sono tradotte. **L'inglese è canonico**:
quando una traduzione è in disaccordo con esso, ha ragione il file inglese.

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

**Tutto il resto è in inglese di proposito, non per dimenticanza.**
`docs/internals/` e `docs/project/` vengono letti da persone che stanno leggendo
anche il Rust, e `CHANGELOG.md` è generato. Tradurli sarebbe altre 17.678 parole
da tenere oneste per un pubblico che non ne ha bisogno.

Ogni traduzione porta l'hash del file inglese da cui è stata fatta, e la CI
fallisce quando l'inglese va avanti - vedi
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Una traduzione che non si
riesce a rimettere in pari viene **eliminata**, non lasciata lì: una pagina
scaduta sembra comunque autorevole e distribuisce i comandi del mese scorso, il
che per chi legge è peggio che essere mandato all'inglese.

Aggiungere una lingua sono quattro file e una riga in questa tabella;
[`docs/internals/contributing.md`](../../internals/contributing.md) ha i passaggi.

## Giochi supportati

**Skyrim SE/AE** - provato nel gioco vero. Anche **Fallout 4** è collegato da un
capo all'altro (F4SE sostituito automaticamente, invalidazione degli archivi,
ordine di caricamento con asterisco, LOOT, salvataggi `.fos`) - vedi
[guide/fallout4.it.md](docs/guide/fallout4.md). Collegati tramite il
descrittore di gioco condiviso e in cerca di collaudatori: Skyrim LE, Skyrim VR,
Enderal SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion e
Morrowind (gli ultimi due montano e gestiscono le mod; le loro liste di plugin
ordinate per data non sono ancora gestite).

Aggiungere una famiglia è una riga di descrittore:
[internals/adding-games.md](../../internals/adding-games.md).

## Lavori precedenti e ringraziamenti

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la semantica che Eidos
  riproduce, e il codice su cui è stata studiata la sua parità
- [LOOT](https://loot.github.io/) - il motore di ordinamento, tramite libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e gli altri gestori per Linux - la
  prova che c'è una comunità che vuole vedere risolto questo problema

## Licenza

GPL-3.0-or-later. La gestione delle mod appartiene a tutti.
