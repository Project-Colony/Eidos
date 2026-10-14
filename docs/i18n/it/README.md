<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Il gestore di mod nativo per Linux che non tocca mai il tuo gioco.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installazione)

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

### Come si confronta

| | Eidos | MO2 in Wine | Fluorine-Manager | Limo / deployer a link |
|---|---|---|---|---|
| Il gestore gira in modo nativo | ✅ | ❌ app Windows in Wine | ✅ (port Qt) | ✅ |
| Cartella del gioco intatta | ✅ sempre | ✅ | ✅ | ❌ ci scrive dentro dei link |
| Mount visibile a | solo al gioco | solo al gioco | **tutto il sistema** | n/d |
| Pulizia necessaria dopo un crash | nessuna, per progetto | nessuna | recupero dei mount rimasti | un-deploy manuale |
| Root mod (ENB, preloader) | ✅ nativo | serve un plugin | serve un plugin | parziale |
| Privilegi richiesti | nessuno | nessuno | modifica di `/etc/fuse.conf` | nessuno |

### Quanto è veloce

| | prima | adesso |
|---|---|---|
| caricare un salvataggio | ~20 secondi | **6-7 secondi** |
| letture di cartelle in una sessione | 5,6 milioni | 465 mila |

I cambi di cella sono immediati. Il guadagno è arrivato ponendo meno domande
alle tue mod: trovare un file interrogava tutte e cinquanta a turno, ed elencare
una cartella lo faceva cinquanta volte di fila. Né l'una né l'altra cosa
accadono più. Misurato su un'istanza vera giocata normalmente, non su un
benchmark.

### Lavori precedenti e ringraziamenti

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) e
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la semantica che Eidos
  riproduce, e il codice su cui è stata studiata la sua parità
- [LOOT](https://loot.github.io/) - il motore di ordinamento, tramite libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) e gli altri gestori per Linux - la
  prova che c'è una comunità che vuole vedere risolto questo problema

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

**Istanze.** Globali - gestite centralmente sotto `~/.local/share/Colony/Eidos/instances/` - o
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

**Nexus.** Collega un account per ricevere i link «Mod Manager Download», controllare gli aggiornamenti e vedere gli autori e i loro profili. Le collezioni vengono installate dalle rispettive ricette, con avanzamento ripristinabile e rapporti espliciti sui passaggi non supportati. Il confronto verifica gli esatti file di origine; i vecchi metadati incompleti restano non verificati. La scheda Download permette di filtrare, ordinare, nascondere ed eliminare gli archivi. L'interruttore **offline** blocca l'accesso a Nexus.

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

**Dove tiene i propri file.** Lo schema che usa ogni programma della famiglia
Colony: `~/.config/Colony/Eidos/` per ciò che hai scelto - preferenze, la tua
sessione Nexus, la lista delle istanze, le definizioni di giochi e add-on che
hai scritto - `~/.local/share/Colony/Eidos/` per le istanze globali, i runtime
scaricati e i log di sessione, e `~/.cache/Colony/Eidos/` per le masterlist di
LOOT. Un Eidos più vecchio li teneva in `~/.config/eidos/`,
`~/.local/share/eidos/` e `~/.local/state/`; il primo avvio dopo l'aggiornamento
li trasferisce, lo scrive nel log, e lascia le vecchie cartelle esattamente
com'erano. Finché un trasferimento non è completo, Eidos continua a usare i
vecchi file. Dettagli in [internals/paths.md](../../internals/paths.md) (in
inglese).

Consulta [Controlli di integrità e file generati](../../../docs/internals/integrity.md) (in inglese) per la protezione delle sostituzioni, i conflitti fra file BSA/BA2, la diagnostica statica SKSE e i registri di generazione, inclusi i loro limiti.

### Giochi supportati

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

## Installazione

### Tramite Colony (consigliato)

Cerca **Eidos** in [Colony](https://github.com/Project-Colony/Colony) e
installalo. Gli aggiornamenti arrivano attraverso il launcher. Colony installa un
solo file, `eidos-linux`: la finestra, e anche la riga di comando `eidos` quando
viene richiamato attraverso un link chiamato `eidos`. L'anteprima dei modelli NIF
richiede un programma ausiliario che solo il pacchetto qui sotto contiene; senza
di esso l'anteprima lo dice e tutto il resto funziona. Cosa fare dopo
l'installazione, e come puntarci Steam:
[docs/guide/install.md](docs/guide/install.md#tramite-colony).

### Download diretto del binario

Prendi un file dall'[ultimo rilascio](https://github.com/Project-Colony/Eidos/releases/latest):

| File | Cos'è |
|---|---|
| `eidos-linux` | l'intero programma in un solo file, come lo installa Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, il programma ausiliario per l'anteprima NIF, le icone e un installatore che registra la voce desktop e il gestore dei download di Nexus |

Verifica prima il download, come mostra la
[Politica di firma del codice](#politica-di-firma-del-codice), poi:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

L'installatore mette tutto in `~/.local/bin` senza sudo; `--system` e
`--bindir DIR` scelgono un altro posto. Entrambi i file richiedono Linux x86_64
con glibc 2.39 o più recente, FUSE (`fusermount3`) e 7-Zip per gli archivi delle
mod.

### Compilare dai sorgenti

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Richiede Rust 1.89 o più recente (la `rust-version` che la CI verifica; CI e
rilasci compilano con 1.99.0), più `just`, CMake 3.20+ e un compilatore C++17
per il programma ausiliario dell'anteprima NIF. Un pacchetto Arch si compila da
`packaging/PKGBUILD`. Prerequisiti e i comandi senza `just`:
[docs/guide/install.md](docs/guide/install.md).

### Opzioni di avvio Steam

Eidos gira *come* comando di avvio del gioco, ed è così che riesce a montare
prima che il gioco parta. La riga di base è tutto ciò che serve alla maggior
parte delle configurazioni, con il percorso assoluto della tua installazione
(Steam non legge il tuo `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Da Colony quel percorso è `~/.local/share/Colony/apps/Eidos/eidos-linux`. Poi
premi Gioca: Eidos si apre sull'istanza di quel gioco, e Run avvia il gioco
attraverso la vista unita.

Tutto il resto sono variabili d'ambiente impilate davanti, e si combinano
liberamente:

| Vuoi... | Metti davanti |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1` - senza di essa DLSS non si inizializza mai, in silenzio; la lista completa è [guide/graphics.it.md](docs/guide/graphics.md) |
| un contatore di FPS sullo schermo | `DXVK_HUD=fps` |
| interpolazione dei fotogrammi a livello di driver, zero mod (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - mai insieme alla generazione di fotogrammi di Community Shaders |
| log dettagliati per una segnalazione di bug | `EIDOS_LOG=debug` (i log di sessione atterrano in `~/.local/share/Colony/Eidos/logs/`) |
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

## Documentazione

Tutto è in [docs/](docs/), con un indice ordinato per chi legge in
[docs/README.md](docs/README.md): la guida per l'utente (installazione, la riga
di comando e la GUI, strumenti come xEdit e DynDOLOD, grafica, risoluzione dei
problemi), come funziona Eidos all'interno, e perché esiste. Politica di
sicurezza e come segnalare una vulnerabilità: [SECURITY.md](../../../SECURITY.md).

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
anche il Rust, e `CHANGELOG.md` è generato. Tradurli sarebbe altre 18.536 parole
da tenere oneste per un pubblico che non ne ha bisogno.

Ogni traduzione porta l'hash del file inglese da cui è stata fatta, e la CI
fallisce quando l'inglese va avanti - vedi
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Una traduzione che non si
riesce a rimettere in pari viene **eliminata**, non lasciata lì: una pagina
scaduta sembra comunque autorevole e distribuisce i comandi del mese scorso, il
che per chi legge è peggio che essere mandato all'inglese.

Una lingua è una sola cartella. `docs/i18n/<lang>/` rispecchia la radice del
repository, così `docs/i18n/de/docs/guide/install.md` è il `docs/guide/install.md`
tedesco - ed è questo che rende un collegamento fra due pagine tradotte la STESSA
stringa del collegamento fra i loro originali inglesi, e che rende il ritiro di
una lingua un solo `rm -r`.

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

Eidos non invia telemetria, né statistiche d'uso, né rapporti sui crash. Non si
collega ad alcun server tranne quelli di questa tabella, e solo per l'azione
indicata. La modalità **Offline** (Preferences -> Nexus) blocca ogni connessione
a Nexus.

| Dati | Conservati o inviati | Dove, e perché |
|---|---|---|
| Preferenze, lista delle istanze, definizioni di giochi e add-on | conservati | `~/.config/Colony/Eidos/` |
| Istanze, runtime scaricati, log di sessione | conservati | `~/.local/share/Colony/Eidos/`, o la cartella che hai scelto per un'istanza portatile |
| Masterlist di LOOT | conservati | `~/.cache/Colony/Eidos/loot/` |
| Accesso a Nexus | inviati | L'accesso apre `users.nexusmods.com` nel tuo browser, che restituisce il risultato a un listener che Eidos apre su `127.0.0.1` solo per quell'accesso. Eidos lo scambia poi con dei token su `users.nexusmods.com/oauth/token` e recupera le chiavi pubbliche con cui Nexus li firma. [Informativa sulla privacy di Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Credenziali Nexus | conservati | I token di accesso e di rinnovo, in `~/.config/Colony/Eidos/nexus.ini` con modalità 0600. Una riga `api_key=` scritta lì da un Eidos più vecchio viene conservata ma mai letta; eliminala pure, se vuoi. |
| Richieste a Nexus | inviati | `api.nexusmods.com`, quando usi una funzione di Nexus: il nome del tuo account e lo stato premium, i dettagli di mod e file, i controlli degli aggiornamenti, le collezioni e i link di download. Ogni richiesta porta il tuo token di accesso e indica Eidos e la sua versione. |
| Download da Nexus | inviati | I file che chiedi, dall'host CDN di Nexus indicato dal link di download, salvati nella cartella dei download dell'istanza con un `.meta` in formato MO2 accanto a ciascuno. |
| Link `nxm://` | conservati | `eidos nxm --register`, che `install.sh` esegue per te, scrive `~/.local/share/applications/eidos-nxm.desktop` e lo rende il gestore di `nxm://` tramite `xdg-mime`, così il pulsante Mod Manager Download del sito passa il suo link a Eidos. Il pacchetto Arch installa la stessa voce per tutto il sistema. |
| Masterlist di LOOT | inviati | `raw.githubusercontent.com/loot/...`, al primo ordinamento LOOT di un gioco e ogni volta che chiedi un aggiornamento. Non viene inviato niente di tuo. [Informativa sulla privacy di GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtime degli strumenti | inviati | Solo quando installi un prerequisito di uno strumento o una mod ne ha bisogno: il runtime .NET da `builds.dotnet.microsoft.com`, rifiutato se non corrisponde allo SHA-256 integrato in Eidos, e `winetricks`, che scarica ciò che installa nel prefisso Proton del gioco (i redistribuibili Visual C++ e .NET) da Microsoft e dagli altri host che winetricks usa. [Informativa sulla privacy di Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Log di sessione | conservati | `~/.local/share/Colony/Eidos/logs/`, modalità 0600: dieci sessioni per istanza, eliminati dopo 30 giorni. La tua cartella home è scritta come `~` e il tuo nome utente come `<user>`, così un log può essere incollato in una segnalazione così com'è. Non lasciano mai la tua macchina a meno che tu non li invii. |
| Collegamenti | conservati | `install.sh` scrive `eidos.desktop` e le sue icone sotto `~/.local/share`; un collegamento a uno strumento che crei va in `~/.local/share/applications`. |
| Link che apri | nessuno dei due | Le pagine delle mod e gli altri link si aprono nel tuo browser tramite `xdg-open`. |
| Add-on | nessuno dei due | Le estensioni che installi girano come programmi propri sulla tua macchina; ciò che fanno è affar loro. |

## Licenza

Il codice originale di Eidos è sotto licenza GPL-3.0-or-later; vedi
[LICENSE](../../../LICENSE). Queste parti sono sotto GPL-3.0 senza l'opzione
"or later":

- le traduzioni degli script OMOD in `eidos-install`; vedi i loro
  [crediti degli autori originali](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- il programma ausiliario `eidos-nif-preview`, costruito sulla libreria nifly
  inclusa nel repository; vedi le
  [licenze e i crediti dei suoi componenti](../../../crates/eidos-nif-preview/UPSTREAM.md);
- i crate `esplugin` e `libloadorder` a cui si collega il codice dell'ordine di
  caricamento, che dichiarano `GPL-3.0`.

Gli altri crate Rust nei binari mantengono le proprie licenze (MIT, Apache-2.0,
BSD e simili). Il pacchetto di rilascio porta i loro avvisi in
`licenses/crates/`, con un indice di ogni crate e della sua licenza.

I binari incorporano anche otto DLL redistribuibili originali di Microsoft
DirectX (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` e `d3dx11_43`, a 32 e
64 bit), che Eidos distribuisce nel prefisso del gioco quando una mod o uno
strumento ne ha bisogno. Restano sotto i termini di redistribuzione di Microsoft;
vedi la loro [provenienza](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

La gestione delle mod appartiene a tutti.
