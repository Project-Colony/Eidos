<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Installare Eidos

Quattro vie d'ingresso. Colony installa un solo file, `eidos-linux`, che è
l'intero programma; le altre forniscono, una accanto all'altra, `eidos` (la riga
di comando), `eidos-gui` e il programma ausiliario per l'anteprima statica NIF,
oltre al gestore `nxm://` che porta nella tua istanza il pulsante "Mod Manager
Download" di Nexus.

## Cosa ti serve prima

| | |
|---|---|
| **Linux con FUSE** | `fusermount3` nel PATH. Ogni distribuzione attuale lo fornisce. |
| **Un gioco Proton, avviato una volta** | Steam crea il prefisso Wine del gioco solo al primo avvio, ed Eidos lavora al suo interno. |
| **`7z`** | Per installare gli archivi delle mod. `p7zip` nella maggior parte delle distribuzioni. |

Niente root, niente demone, nessuna modifica a `/etc/fuse.conf` e niente da
aggiungere ai tuoi gruppi. Eidos monta dentro uno spazio dei nomi privato che
appartiene al processo del gioco.

## Tramite Colony

Cerca **Eidos** in [Colony](https://github.com/Project-Colony/Colony) e
installalo; gli aggiornamenti arrivano attraverso il launcher. Colony conserva un
solo file, `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eseguilo e si apre la
finestra. Richiamato attraverso un link chiamato `eidos`, lo stesso file è la
riga di comando. Mettine uno nel tuo `PATH` e registra il gestore dei download di
Nexus:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

La finestra non ha bisogno del link: ogni volta che avvia un gioco o uno
strumento, esegue il proprio file come riga di comando. L'anteprima dei modelli
NIF richiede `eidos-nif-preview`, che solo il pacchetto di rilascio e le
compilazioni dai sorgenti contengono; senza di esso l'anteprima lo dice, e
nient'altro ne risente. Punta Steam sul file stesso, come più sotto.

## Un pacchetto di rilascio

Scarica `eidos-linux-bundle.tar.gz` e i suoi `.sig`, `.meta` e `.meta.sig`
dall'[ultimo rilascio](https://github.com/Project-Colony/Eidos/releases/latest),
e verificali come mostra la
[Politica di firma del codice](../../README.md#politica-di-firma-del-codice) del
README. Poi:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Installa in `~/.local/bin` per impostazione predefinita. `--system` lo mette in
`/usr/local/bin`, `--bindir DIR` altrove. Rieseguirlo è il modo previsto per
aggiornare. Il pacchetto ed `eidos-linux` richiedono Linux x86_64 con glibc 2.39
o più recente.

## Arch

```bash
cd packaging && makepkg -si
```

Il PKGBUILD compila dai sorgenti il `main` attuale.

## Dai sorgenti

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

La compilazione dai sorgenti richiede Rust 1.89 o più recente, `just`, CMake 3.20+, un compilatore C++17 e Python 3 per i test del programma ausiliario nativo. Per i comandi senza `just`, consulta le [istruzioni del programma ausiliario nativo](../../../../../crates/eidos-nif-preview/README.md), compila Rust con `cargo build --release --locked`, copia il programma ausiliario in `target/release`, quindi esegui `packaging/install.sh --from target/release`.

## Poi: puntarci Steam

Eidos gira *come* comando di avvio del tuo gioco, ed è così che riesce a montare
prima che il gioco parta. In Steam, tasto destro sul gioco -> Proprietà ->
Opzioni di avvio:

```
~/.local/bin/eidos-gui %command%
```

Da Colony, il percorso è `~/.local/share/Colony/apps/Eidos/eidos-linux`. Premi
Gioca. Eidos si apre sull'istanza di quel gioco; installa mod, ordina con
LOOT, clicca Run. All'uscita il mount se ne va con lui e la tua installazione è
esattamente com'era.

Usa il percorso assoluto - Steam non legge il `PATH` della tua shell.

### Se preferisci il terminale

```sh
eidos init skyrimse               # creare un'istanza (indica una cartella per renderla portatile)
eidos install skyrimse mod.7z     # mod Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordinare il caricamento con LOOT
eidos play skyrimse -- %command%  # eseguire qualsiasi cosa attraverso la vista unita
```

Ogni comando che accetta un identificatore di gioco accetta anche la cartella di
un'istanza portatile - vedi [usage.it.md](usage.md). Il giro completo è lì.

## Facoltativo: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` assegna la capability facoltativa, ma non attiva il passthrough.
Va sul binario `eidos` separato del pacchetto o di una compilazione dai sorgenti,
mai su `eidos-linux`, che è anche la finestra ([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` è il comando separato per attivarlo durante l’esecuzione. È **disattivato per impostazione predefinita e quasi certamente lo
vuoi così**: misurato su Skyrim SE, impedisce al gioco di aprire i propri archivi
e plugin, così le mod silenziosamente non si caricano. L'interruttore esiste per
ricollaudare il meccanismo, non perché sia consigliato.

I dettagli, e le misure dietro quella decisione, in
[troubleshooting.it.md](troubleshooting.md).

## Qualcosa già non va?

[troubleshooting.it.md](troubleshooting.md) copre gli interruttori
d'ambiente, come leggere i contatori delle operazioni e ogni problema che finora
ha morso qualcuno.
