<!-- eidos-i18n: source=docs/guide/install.md sha=c47b7d58351d0b72f6e700bf9acb7e3d78af6a5c -->

# Installare Eidos

Tre vie d'ingresso. Tutte forniscono `eidos` (la riga di comando), `eidos-gui` e il programma ausiliario per l'anteprima statica NIF, oltre al gestore `nxm://` che porta nella tua istanza il pulsante "Mod Manager Download" di Nexus.

## Cosa ti serve prima

| | |
|---|---|
| **Linux con FUSE** | `fusermount3` nel PATH. Ogni distribuzione attuale lo fornisce. |
| **Un gioco Proton, avviato una volta** | Steam crea il prefisso Wine del gioco solo al primo avvio, ed Eidos lavora al suo interno. |
| **`7z`** | Per installare gli archivi delle mod. `p7zip` nella maggior parte delle distribuzioni. |

Niente root, niente demone, nessuna modifica a `/etc/fuse.conf` e niente da
aggiungere ai tuoi gruppi. Eidos monta dentro uno spazio dei nomi privato che
appartiene al processo del gioco.

## Arch

```bash
cd packaging && makepkg -si
```

## Un archivio di rilascio

```bash
./install.sh
```

Installa in `~/.local/bin` per impostazione predefinita. `--system` lo mette in
`/usr/local/bin`, `--bindir DIR` altrove. Rieseguirlo è il modo previsto per
aggiornare.

## Dai sorgenti

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

La compilazione dai sorgenti richiede Rust, `just`, CMake 3.20+, un compilatore C++17 e Python 3 per i test del programma ausiliario nativo. Per i comandi senza `just`, consulta le [istruzioni del programma ausiliario nativo](../../../../../native/eidos-nif-preview/README.md), compila Rust con `cargo build --release --locked`, copia il programma ausiliario in `target/release`, quindi esegui `packaging/install.sh --from target/release`.

## Poi: puntarci Steam

Eidos gira *come* comando di avvio del tuo gioco, ed è così che riesce a montare
prima che il gioco parta. In Steam, tasto destro sul gioco -> Proprietà ->
Opzioni di avvio:

```
~/.local/bin/eidos-gui %command%
```

Premi Gioca. Eidos si apre sull'istanza di quel gioco; installa mod, ordina con
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
