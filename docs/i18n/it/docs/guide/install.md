<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Installare Eidos

Quattro vie d'ingresso. Colony installa un solo file, `eidos-linux`, che è
l'intero programma. Il pacchetto di rilascio, il pacchetto Arch e una
compilazione dai sorgenti forniscono, una accanto all'altra, `eidos` (la riga di
comando), `eidos-gui` e il programma ausiliario statico per l'anteprima NIF
`eidos-nif-preview`. Il pacchetto Arch e una compilazione dai sorgenti installano
anche `eidos-fuse` ed `eidos-launch`, le due CLI di livello più basso. Ogni via
può registrare il gestore `nxm://` che porta nella tua istanza il pulsante "Mod
Manager Download" di Nexus.

## Cosa ti serve prima

| Requisito | Perché |
|---|---|
| Linux con FUSE | `fusermount3` nel PATH. Ogni distribuzione attuale lo fornisce. |
| Un gioco Proton, avviato una volta | Steam crea il prefisso Wine del gioco solo al primo avvio, ed Eidos lavora al suo interno. Eidos riconosce anche le copie GOG ed Epic di Skyrim Special Edition, le copie Epic di Fallout 3 e Fallout: New Vegas e la copia GOG di Stardew Valley. Queste girano tramite Heroic o Legendary con Eidos come wrapper del gioco: vedi [usage.md](usage.md#avviare-il-gioco). |
| Un binario 7-Zip: `7z`, `7zz` o `7za` | Per installare gli archivi delle mod. `7zip` su Arch, `p7zip` nella maggior parte delle altre distribuzioni. |
| `par2` (facoltativo) | Dati di recupero PAR2 per le copie di `eidos pack`. `par2cmdline` su Arch e Fedora, `par2` su Debian e Ubuntu. |

Niente root, niente demone, nessuna modifica a `/etc/fuse.conf` e niente da
aggiungere ai tuoi gruppi. Eidos monta dentro uno spazio dei nomi privato che
appartiene al processo del gioco.

Alcune impostazioni di sistema si vedono solo come crash. La scheda Diagnostics
controlla `vm.max_map_count` (1048576 o più), il limite hard dei file aperti
(524288 o più) e se il gioco, il suo prefisso o l'istanza stanno su NTFS, exFAT o
FAT; [troubleshooting.md](troubleshooting.md#il-sistema) ha il rimedio per
ciascuno.

## Tramite Colony

Cerca Eidos in [Colony](https://github.com/Project-Colony/Colony) e installalo;
gli aggiornamenti arrivano attraverso il launcher. Colony conserva un solo file,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Eseguilo e si apre la finestra.
Richiamato attraverso un link chiamato `eidos`, lo stesso file è la riga di
comando. Mettine uno nel tuo `PATH` e registra il gestore dei download di Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

La finestra non ha bisogno del link: ogni volta che avvia un gioco o uno
strumento, esegue il proprio file come riga di comando. L'anteprima dei modelli
NIF richiede `eidos-nif-preview`, che solo il pacchetto di rilascio, il pacchetto
Arch e le compilazioni dai sorgenti contengono; senza di esso l'anteprima lo dice,
e nient'altro ne risente. `eidos-linux` non contiene nemmeno i runtime Microsoft
che il pacchetto include ([più sotto](#i-runtime-microsoft-nel-pacchetto)):
quando uno strumento ne ha bisogno, Eidos lo scarica da Microsoft appena avvii
l'installazione dei prerequisiti. Punta Steam sul file stesso, come più sotto.

## Un pacchetto di rilascio

Scarica `eidos-linux-bundle.tar.gz` e i suoi `.sig`, `.meta` e `.meta.sig`
dall'[ultimo rilascio](https://github.com/Project-Colony/Eidos/releases/latest),
e verificali come mostra la
[Politica di firma del codice](../../README.md#politica-di-firma-del-codice) del
README. Il pacchetto contiene `eidos`, `eidos-gui` ed `eidos-nif-preview` al
livello superiore, con `install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, le
icone in `assets/brand/png` e gli avvisi di terze parti in `licenses/` (nifly,
omod, microsoft-directx, microsoft-runtimes e i crate Rust) e la cartella
`runtimes/` descritta sotto. Non contiene `eidos-fuse` né `eidos-launch`. Poi:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` copia i binari in `~/.local/bin`, oppure in `$XDG_BIN_HOME` se è
impostata. `--system` li mette in `/usr/local/bin` tramite sudo, `--bindir DIR`
altrove, e `--from DIR` prende i binari da un'altra cartella. Lo script inoltre:

- registra il gestore `nxm://` eseguendo `eidos nxm --register`, quindi il
  passaggio di registrazione della sezione Colony non serve;
- scrive `~/.local/share/applications/eidos.desktop` e le icone di Eidos sotto
  `~/.local/share/icons/hicolor`;
- copia gli avvisi di licenza in `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` con `--system`);
- copia `runtimes/` in `~/.local/share/Colony/Eidos/runtimes`, anche con
  `--system`, così i runtime appartengono all'utente che esegue lo script;
- finisce stampando l'opzione di avvio di Steam con il percorso assoluto
  dell'`eidos-gui` che ha installato.

`--cap` chiede a sudo di concedere `CAP_SYS_ADMIN` all'`eidos` installato (vedi
il passthrough FUSE più sotto). `--no-cap` è l'impostazione predefinita e lo
lascia senza privilegi. Rieseguire lo script è il modo previsto per aggiornare. Il
pacchetto ed `eidos-linux` richiedono Linux x86_64 con glibc 2.39 o più recente.

### I runtime Microsoft nel pacchetto

La cartella `runtimes/` del pacchetto contiene i file Microsoft che gli strumenti
richiedono: `VC_redist.x86.exe` e `VC_redist.x64.exe` (Visual C++ 2015-2022, il
prerequisito `vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (il .NET 8
Desktop Runtime, per `dotnet8` e `dotnetdesktop8`) e
`dotnet-runtime-10.0.10-win-x64.zip` (il runtime .NET 10, per `dotnet10`). Eidos
esegue da sé gli installatori di Visual C++ e .NET 8 nel prefisso del gioco, senza
winetricks, e decomprime lo zip di .NET 10 una sola volta in
`~/.local/share/Colony/Eidos/runtimes`, dove lo condividono tutte le istanze (78
MB decompresso). Ogni file viene usato solo se il suo SHA-256 coincide con quello
fissato in Eidos. L'SDK .NET 10 che serve a Synthesis (`dotnetsdk10`, circa 300
MB) non è nel pacchetto e viene sempre scaricato.

Un pacchetto decompresso gira anche sul posto: dopo la cartella dati qui sopra,
Eidos cerca `runtimes/` accanto al proprio binario.

## Arch

```bash
cd packaging && makepkg -si
```

Il PKGBUILD compila dai sorgenti il `main` attuale. Installa `eidos`, `eidos-gui`,
`eidos-nif-preview`, `eidos-fuse` ed `eidos-launch` in `/usr/bin`, con un gestore
`nxm://` di sistema, una voce desktop e le icone. Dipende da `fuse3`, `7zip` e
`xdg-utils`. Non porta una cartella `runtimes/`, quindi i runtime Microsoft
vengono scaricati quando uno strumento ne ha bisogno. `winetricks` è facoltativo:
Eidos lo usa ancora per i prerequisiti che non installa da sé, come `dotnet48` e
`xact`.

## Dai sorgenti

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

La compilazione dai sorgenti richiede Rust 1.92 o più recente, `just`, CMake 3.20+,
un compilatore C++17 e Python 3 per i test del programma ausiliario nativo. `just
build` compila l'intero workspace, quindi vengono compilati anche `eidos-fuse` ed
`eidos-launch`, e `just install` esegue `packaging/install.sh` sull'output della
compilazione, che li copia insieme al resto. Per i comandi senza `just`, consulta
le [istruzioni del programma ausiliario nativo](../../../../../crates/eidos-nif-preview/README.md),
compila Rust con `cargo build --release --locked`, copia il programma ausiliario in
`target/release`, quindi esegui `packaging/install.sh --from target/release`.
Neanche una compilazione dai sorgenti ha una cartella `runtimes/`, quindi i
runtime Microsoft vengono scaricati quando uno strumento ne ha bisogno.

Il profilo release usa una sola unità di codegen, LTO completo e simboli rimossi.
Con rustc 1.99.1 e 3 job di compilazione, una compilazione a freddo di `eidos` e
`eidos-gui` ha richiesto 334 s, contro i 243 s con LTO thin, e ha dato un `eidos`
da 20,9 MB e un `eidos-gui` da 49,7 MB. Ogni binario incorpora otto DLL DirectX
conservate compresse con gzip: 7,9 MB invece di 17,9 MB. `just profile=dev build`
compila senza ottimizzazione.

## Poi: puntarci Steam

Eidos gira *come* comando di avvio del tuo gioco, ed è così che riesce a montare
prima che il gioco parta. In Steam, tasto destro sul gioco -> Proprietà ->
Opzioni di avvio, e inserisci il percorso assoluto della finestra di Eidos seguito
da `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Usa il percorso assoluto - Steam non legge il `PATH` della tua shell. Il percorso
dipende da come hai installato Eidos, e `install.sh` stampa la riga per la cartella
che ha usato:

| Installazione | Opzione di avvio |
|---|---|
| Pacchetto di rilascio o `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Pacchetto Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Premi Gioca. Eidos si apre sull'istanza di quel gioco; installa mod, ordina con
LOOT, clicca Run. All'uscita il mount se ne va con lui e la tua installazione è
esattamente com'era.

Il gioco parte sempre attraverso il suo store. In una finestra che hai aperto tu,
Run esegue `steam -applaunch <appid>`, Steam avvia Eidos tramite l'opzione di
avvio, e si apre una seconda finestra di Eidos in cui premi Run. Finché l'opzione
di avvio non fa partire Eidos, Run non avvia nulla e la riga di stato mostra la
riga da impostare. Con Start the game at once when a store opens Eidos
(Preferences, General, Running a game; disattivato per impostazione predefinita),
Gioca in Steam avvia il gioco senza la finestra quando Health non elenca problemi.
Heroic e Legendary funzionano allo stesso modo tramite la loro impostazione del
wrapper; [usage.md](usage.md#avviare-il-gioco) ha i dettagli.

La scheda Diagnostics controlla la riga. Una scheda Health mostra la riga da
impostare quando l'opzione di avvio è vuota, non nomina nessun binario di Eidos,
punta a un file che non esiste, non ha `%command%` dopo Eidos, oppure avvia
un'altra copia di Eidos rispetto a quella in esecuzione.

### Se preferisci il terminale

```sh
eidos init skyrimse               # creare un'istanza (indica una cartella per renderla portatile)
eidos install skyrimse mod.7z     # mod Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordinare il caricamento con LOOT
eidos check skyrimse              # i controlli Health che non richiedono la finestra
eidos play skyrimse -- %command%  # eseguire qualsiasi cosa attraverso la vista unita
eidos shell skyrimse              # una shell nella vista unita, le scritture finiscono in Overwrite
```

Ogni comando che accetta un identificatore di gioco accetta anche la cartella di
un'istanza portatile - vedi [usage.md](usage.md#istanze-globali-e-portatili). Il
giro completo è in [usage.md](usage.md).

## Aggiornamenti

Colony aggiorna `eidos-linux` tramite il suo launcher. Per il pacchetto, esegui
l'`install.sh` del nuovo pacchetto; per il pacchetto Arch, esegui di nuovo
`makepkg -si`; per una compilazione dai sorgenti, fai pull ed esegui
`just install`. All'avvio la finestra chiede a GitHub l'ultimo rilascio, al
massimo una volta al giorno, e quando ne esce uno più recente mostra un link alle
sue note di rilascio dopo Help nella barra dei menu. Non scarica né sostituisce
mai se stessa. Preferences, About, Version disattiva il controllo, la modalità
offline lo salta, e una copia installata sotto `/usr` (il pacchetto Arch) non lo
chiede mai, perché lo aggiorna il gestore dei pacchetti.

## Facoltativo: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` assegna la capability
facoltativa; non attiva il passthrough. `./install.sh --cap` e `just setcap` la
assegnano all'`eidos` che installano o compilano. Su un mount `nosuid` il kernel
ignora le capability dei file, quindi `install.sh` salta l'assegnazione con un
avviso e `just setcap` si ferma con un errore. La capability va sul binario
`eidos` separato del pacchetto, del pacchetto Arch o di una compilazione dai
sorgenti, mai su `eidos-linux`, che è anche la finestra
([SECURITY.md](../../../../../SECURITY.md)). Un binario ricompilato o reinstallato
è un file nuovo senza la capability, quindi assegnala di nuovo dopo ogni
aggiornamento se la usi.

`EIDOS_FUSE_PASSTHROUGH=1` è l'interruttore separato a tempo di esecuzione, ed è
disattivato per impostazione predefinita. Con il passthrough attivo, una versione
precedente del demone impediva a Skyrim SE di caricare qualsiasi contenuto delle
mod. La causa più probabile è che registrava un nuovo file di appoggio a ogni
apertura, mentre il kernel ne accetta uno solo per inode. Il demone ora condivide
un solo file di appoggio tra tutte le aperture in passthrough di un file, ma
questo non è ancora stato rimisurato in un gioco. Fino ad allora, attiva il
passthrough solo per provarlo. Le misure e la causa sono in
[troubleshooting.md](troubleshooting.md).

## Qualcosa già non va?

[troubleshooting.md](troubleshooting.md) copre gli interruttori d'ambiente, come
leggere i contatori delle operazioni, le schede Health e ogni problema che finora
ha morso qualcuno.
