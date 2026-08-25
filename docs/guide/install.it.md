<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Installare Eidos

Tre vie d'ingresso. Tutte danno gli stessi due eseguibili - `eidos` (la riga di
comando) e `eidos-gui` - più il gestore `nxm://` che fa atterrare nella tua
istanza il pulsante "Mod Manager Download" di Nexus.

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
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

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
un'istanza portatile - vedi [usage.it.md](usage.it.md). Il giro completo è lì.

## Facoltativo: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` abilita il passthrough FUSE
del kernel. È **disattivato per impostazione predefinita e quasi certamente lo
vuoi così**: misurato su Skyrim SE, impedisce al gioco di aprire i propri archivi
e plugin, così le mod silenziosamente non si caricano. L'interruttore esiste per
ricollaudare il meccanismo, non perché sia consigliato.

I dettagli, e le misure dietro quella decisione, in
[troubleshooting.it.md](troubleshooting.it.md).

## Qualcosa già non va?

[troubleshooting.it.md](troubleshooting.it.md) copre gli interruttori
d'ambiente, come leggere i contatori delle operazioni e ogni problema che finora
ha morso qualcuno.
