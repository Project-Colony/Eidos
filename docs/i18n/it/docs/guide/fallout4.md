<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 attraverso Eidos

Fallout 4 non richiede alcuna opzione di avvio speciale, nessun eseguibile
rinominato e nessuno script involucro. Vale la pena dirlo chiaramente, perché ogni
altra guida Linux per F4SE sostiene il contrario - e i loro consigli si rompono al
prossimo aggiornamento di Steam.

## L'opzione di avvio

```
~/.local/bin/eidos-gui %command%
```

Il bersaglio d'avvio di Steam per Fallout 4 è `Fallout4Launcher.exe`, mai
`Fallout4.exe`, quindi far partire lo script extender è in realtà la domanda "come
faccio ad avviare un programma diverso da Steam". Le risposte solite riscrivono
`%command%` in bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

oppure copiano `f4se_loader.exe` sopra `Fallout4Launcher.exe`, che Steam ripristina
in sordina a ogni aggiornamento del gioco - dopodiché stai giocando senza F4SE e
niente lo dice.

Eidos fa lo scambio da sé, a partire dal descrittore del gioco: sostituisce il
lanciatore con `f4se_loader.exe` quando ce n'è uno installato, ripiega su
`Fallout4.exe` quando non c'è, e **te lo dice** quando ha dovuto ripiegare. Un gioco
che parte con tutte le mod F4SE inerti è peggio di un gioco che non parte.

C'è una seconda ragione per non eseguire mai il lanciatore: riscansiona `Data` e
riscrive `plugins.txt`, disfacendo l'ordine di caricamento appena distribuito. Eidos
non lo esegue mai.

## Di cosa si occupa Eidos al posto tuo

| | |
|---|---|
| Invalidazione degli archivi | `Fallout4Custom.ini` riceve `[Archive]` `bInvalidateOlderFiles=1` e un `sResourceDataDirsFinal=` vuoto, le due chiavi che permettono ai file sciolti fuori da `Data` di essere visti del tutto. Scritto nel profilo, non nella cartella del gioco. |
| Ordine di caricamento | `plugins.txt` nel formato con asterisco usato da Fallout 4 (`*` indica attivo), con `Fallout4.ccc` rispettato per i plugin impliciti del Creation Club |
| LOOT | L'ordinamento funziona come per Skyrim - `eidos sort <instance>` scarica la masterlist `fallout4` |
| Salvataggi | I salvataggi `.fos` e i loro cosave `.f4se` sono elencati, copiati e tenuti per profilo; il pannello dei dettagli legge la tabella dei plugin del salvataggio stesso, così un salvataggio che richiede un plugin disattivato lo dice prima che tu lo carichi |
| Mod root | Tutto ciò che una mod porta accanto all'eseguibile (F4SE stesso, ENB, un `dxvk.conf`) finisce lì attraverso lo stesso meccanismo `Root/` usato da Skyrim |

## La questione delle versioni

Fallout 4 non è più il gioco congelato che era tra il 2019 e il 2024. Ad agosto 2026
ci sono tre rami vivi, e una DLL di mod costruita per uno non si carica su un altro:

| Ramo | Versione | F4SE |
|---|---|---|
| Classico ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Due conseguenze da conoscere prima di costruire una lista di mod:

- **Controlla cosa hai davvero.** Le cartelle `Creations/` e `Mods/` nella radice
  del gioco significano che sei sulla linea 1.11.x. Il pannello dei dettagli di un
  salvataggio in Eidos mostra anche la build che l'ha scritto - Fallout la incide
  nel salvataggio, ed Eidos la espone come "Game build".
- **Una patch appena uscita non è un buon giorno per cominciare.** F4SE di solito
  arriva entro un giorno o due da un aggiornamento Bethesda, ma *Address Library for
  F4SE Plugins* - attraverso cui la maggior parte delle mod DLL risolve i propri
  offset - segue un calendario suo. Nel mezzo, la metà DLL dell'ecosistema è a
  terra. Le mod senza DLL (texture, mesh, plugin) non ne risentono.

Una volta che il tuo assetto funziona, disattiva gli aggiornamenti automatici di
Steam per Fallout 4 (Proprietà → Aggiornamenti → "Aggiorna questo gioco solo
quando lo avvio"), altrimenti la prossima patch romperà ogni DLL installata.

## Nota hardware: i detriti delle armi mandano in crash su NVIDIA

L'effetto detriti delle armi di Fallout 4 gira su NVIDIA FleX, un derivato di PhysX
che NVIDIA ha smesso di supportare dopo la generazione Pascal. Su qualsiasi scheda
Turing o più recente - GTX 16, RTX 20 fino a RTX 50 - manda il gioco in crash. È un
bug del gioco, nulla a che vedere con Linux, Proton o Eidos.

Due rimedi, va bene l'uno o l'altro: disattiva "Weapon Debris" nelle impostazioni
del gioco, oppure installa *Weapon Debris Crash Fix* (Nexus 48078), che disabilita
la collisione dei frammenti invece dell'effetto.

## Se qualcosa sembra sbagliato

La lista generale è in [troubleshooting.it.md](troubleshooting.md); la prima
domanda specifica di Fallout è sempre *quale eseguibile è partito davvero*. Eidos
scrive il comando d'avvio completo nel registro d'esecuzione dell'istanza, quindi:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Se nomina `f4se_loader.exe`, lo scambio è avvenuto. Se nomina
`Fallout4Launcher.exe`, F4SE non è installato dove Eidos possa trovarlo - va accanto
all'eseguibile del gioco, il che in un assetto gestito significa la cartella `Root/`
di una mod (o la cartella del gioco stessa, installato a mano).
