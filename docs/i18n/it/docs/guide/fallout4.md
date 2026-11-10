<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 attraverso Eidos

Con Eidos, Fallout 4 non richiede alcuna opzione di avvio speciale, nessun
eseguibile rinominato e nessuno script involucro. Usa la stessa opzione di avvio
di ogni altro gioco.

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
a ogni aggiornamento del gioco - dopodiché stai giocando senza F4SE e niente lo
dice.

Eidos fa lo scambio da sé, a partire dal descrittore del gioco. Quando la finestra
avvia il gioco, sostituisce `Fallout4Launcher.exe` con `f4se_loader.exe` se il
loader si trova nella cartella del gioco, in un `Root/` di una mod attiva o in
`Overwrite/Root/`. Senza il loader esegue `Fallout4.exe`, e la riga di stato dice
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Se non trova nessuno dei due file, esegue il lanciatore così com'è e lo dice
anche in quel caso. Con `eidos play fallout4 -- %command%` come opzione di avvio,
la CLI fa lo scambio solo se il loader esiste, altrimenti esegue
`Fallout4Launcher.exe`.

C'è una seconda ragione per evitare il lanciatore: riscansiona `Data` e riscrive
`plugins.txt`, disfacendo l'ordine di caricamento appena distribuito.

Quando apri tu la finestra e premi Run, Eidos chiede a Steam di avviare il gioco
(`steam -applaunch 377160`), e Steam avvia Eidos tramite l'opzione di avvio qui
sopra. Se `f4se_loader.exe` non è accanto all'eseguibile del gioco nell'istanza,
non lo fa, e la riga di stato ti chiede di installare prima lo script extender.
Fallout 4 VR funziona allo stesso modo, con `Fallout4VRLauncher.exe` e
`f4sevr_loader.exe`.

## Di cosa si occupa Eidos al posto tuo

| Area | Cosa fa Eidos |
|---|---|
| Invalidazione degli archivi | `Fallout4Custom.ini` riceve `[Archive]` `bInvalidateOlderFiles=1` e un `sResourceDataDirsFinal=` vuoto, le due chiavi che permettono ai file sciolti fuori da `Data` di essere visti del tutto. Scritto nel profilo, non nella cartella del gioco. |
| INI | `Fallout4.ini`, `Fallout4Prefs.ini` e `Fallout4Custom.ini` appartengono al profilo. BethINI Pie eseguito tramite Eidos modifica le copie del profilo, e Backups -> INIs rimette la copia che Eidos tiene prima di ogni sua esecuzione ([tools.md](tools.md#bethini-pie)). |
| Ordine di caricamento | `plugins.txt` nel formato con asterisco usato da Fallout 4 (`*` indica attivo), con `Fallout4.ccc` rispettato per i plugin impliciti del Creation Club |
| LOOT | L'ordinamento funziona come per Skyrim - `eidos sort <instance>` scarica la masterlist `fallout4` |
| Salvataggi | I salvataggi `.fos` e i loro cosave `.f4se` sono elencati, copiati e tenuti per profilo; il pannello dei dettagli legge la tabella dei plugin del salvataggio stesso, così un salvataggio che richiede un plugin disattivato lo dice prima che tu lo carichi. Disattivare o rimuovere una mod il cui plugin è usato da uno dei tre salvataggi più recenti del profilo chiede prima conferma. |
| Mod root | Tutto ciò che una mod porta accanto all'eseguibile (F4SE stesso, ENB, un `dxvk.conf`) finisce lì attraverso lo stesso meccanismo `Root/` usato da Skyrim |

## La questione delle versioni

Fallout 4 non è più il gioco congelato che era tra il 2019 e il 2024. Ad agosto 2026
ci sono tre rami vivi, e una DLL di mod costruita per uno non si carica su un altro:

| Ramo | Versione | F4SE |
|---|---|---|
| Classico ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | da 1.11.137 a 1.11.240 | da 0.7.4 a 0.7.9 |

Due conseguenze da conoscere prima di costruire una lista di mod:

- Le cartelle `Creations/` e `Mods/` nella radice del gioco significano che sei
  sulla linea 1.11.x. Il pannello dei dettagli di un salvataggio in Eidos mostra
  anche la build che l'ha scritto: Fallout la incide nel salvataggio, ed Eidos la
  espone come "Game build".
- F4SE di solito arriva entro un giorno o due da un aggiornamento Bethesda, ma
  *Address Library for F4SE Plugins* - attraverso cui la maggior parte delle mod
  DLL risolve i propri offset - segue un calendario suo. Nel mezzo, le mod DLL non
  si caricano. Le mod senza DLL (texture, mesh, plugin) non ne risentono, quindi
  una patch appena uscita è un brutto giorno per cominciare una lista di mod.

Quando il tuo assetto funziona, congela la versione del gioco nell'istanza: Freeze
this version nella scheda Game files della scheda Diagnostics, oppure
`eidos stock-game fallout4 freeze` per vedere la dimensione e
`eidos stock-game fallout4 freeze --yes` per copiare. Eidos copia l'installazione
dello store in `<instance>/Stock Game` e avvia il gioco da quella copia, sempre
lanciato da Steam. Una patch successiva cambia l'installazione dello store e non
la copia, quindi le DLL che hai installato mantengono la loro versione del gioco.
Su btrfs o XFS la copia condivide i blocchi dello store e occupa quasi nulla finché
Steam non cambia un file; su ext4 è una copia completa grande quanto la cartella
del gioco. Eidos rifiuta se Steam segnala il gioco occupato o non del tutto
installato. I dettagli sono in
[usage.md](usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia).

Senza una copia, disattiva gli aggiornamenti automatici di Steam per Fallout 4
(Proprietà, Aggiornamenti, "Aggiorna questo gioco solo quando lo avvio"),
altrimenti la prossima patch romperà ogni DLL installata.

## Nota hardware: i detriti delle armi mandano in crash su NVIDIA

L'effetto detriti delle armi di Fallout 4 gira su NVIDIA FleX, un derivato di PhysX
che NVIDIA ha smesso di supportare dopo la generazione Pascal. Su qualsiasi scheda
Turing o più recente - GTX 16, RTX 20 fino a RTX 50 - manda il gioco in crash. È un
bug del gioco, nulla a che vedere con Linux, Proton o Eidos.

Due rimedi, va bene l'uno o l'altro: disattiva "Weapon Debris" nelle impostazioni
del gioco, oppure installa *Weapon Debris Crash Fix* (Nexus 48078), che disabilita
la collisione dei frammenti invece dell'effetto.

## Schede Health per Fallout 4

La scheda Diagnostics mostra una scheda Health per ciascuno di questi casi:

- I plugin F4SE sono installati in `F4SE/Plugins`, ma `f4se_loader.exe` non è in
  nessuna mod attiva né nella cartella del gioco. Il gioco parte senza di essi.
- `f4se_loader.exe` è installato, ma manca la `f4se_<version>.dll` per la versione
  del gioco in esecuzione (`f4se_1_10_984.dll` per la 1.10.984). Il loader si
  ferma con un errore; di solito succede dopo un aggiornamento del gioco.
- L'eseguibile del gioco o la sua build Steam sono cambiati dall'ultimo avvio
  tramite Eidos terminato senza errore. I plugin F4SE costruiti per la versione
  vecchia possono smettere di caricarsi.
- Una mod con priorità più alta di F4SE ha copie sciolte `.pex` degli script di
  F4SE stesso. Una copia vecchia rompe le funzioni F4SE che dichiara, e il gioco
  non mostra alcun errore.
- Il `Fallout4Custom.ini` del profilo ha righe da `sTestFile1` a `sTestFile10` in
  `[General]`. Finché una è impostata, il gioco carica quei plugin e ignora
  `plugins.txt`. Fix le rimuove e tiene il file precedente come
  `Fallout4Custom.ini.pre-fix`.
- L'archivio BA2 di una mod contiene audio `.wav` o `.xwm` compresso. Il gioco può
  andare in crash quando riproduce quel suono. Reimpacchetta l'archivio con l'audio
  non compresso. Fallout 4 VR riceve la stessa scheda.
- Un crash logger ha scritto un registro durante l'ultimo avvio: il
  `crash-*.log` di Buffout 4 in `Documents/My Games/Fallout4/F4SE`, oppure un
  `.log` in `F4SE/Crashlogs`. La scheda nomina l'eccezione e le DLL e i plugin del
  registro che le tue mod installano.
- La scheda Display mostra la dimensione e la modalità della finestra impostate nel
  `Fallout4Prefs.ini` del profilo, e Apply scrive lì `iSize W`, `iSize H`,
  `bFull Screen` e `bBorderless`.
- Un requisito di Nexus nomina un DLC. Eidos cerca un plugin con il nome del DLC, e
  i plugin dei DLC di Fallout 4 hanno altri nomi (`DLCCoast.esm` per Far Harbor),
  quindi questa scheda è un consiglio: controlla a mano, poi premi Ignore.

Le due schede del loader hanno un pulsante Open the download page per
<https://f4se.silverlock.org/> e un pulsante Install the downloaded archive, che
mette il loader nel `Root/` della mod. `eidos check fallout4` stampa gli stessi
riscontri, tranne la scheda `.pex`, la scheda Display e le schede dei requisiti di
Nexus, che solo la finestra calcola.

## Se qualcosa sembra sbagliato

La lista generale è in [troubleshooting.md](troubleshooting.md); la prima domanda
specifica di Fallout è sempre *quale eseguibile è partito davvero*. Quando la
finestra avvia il gioco, scrive il comando d'avvio completo nel registro
d'esecuzione dell'istanza, quindi:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Se nomina `f4se_loader.exe`, lo scambio è avvenuto. Se nomina `Fallout4.exe`, F4SE
non è installato dove Eidos possa trovarlo - va accanto all'eseguibile del gioco,
il che in un assetto gestito significa la cartella `Root/` di una mod (o la cartella
del gioco stessa, installato a mano). Se nomina `Fallout4Launcher.exe`, Eidos non
ha trovato né il loader né `Fallout4.exe` nella vista root.

Quando parte l'eseguibile giusto e il gioco va comunque in crash, Test run e Find
the mod behind a problem restringono il campo a una sola mod senza toccare i
salvataggi del profilo ([usage.md](usage.md#esecuzioni-di-prova)).
