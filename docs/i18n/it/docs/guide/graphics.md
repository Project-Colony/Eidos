<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS e generazione di fotogrammi

Community Shaders 1.4+ porta con sé il proprio upscaling (DLSS 4 / FSR 3.1 / XeSS,
tramite il pacchetto separato "Upscaling - Community Shaders") e la generazione di
fotogrammi FSR 3.1. Tutto funziona attraverso Eidos su Linux - CS e i suoi
pacchetti si installano come mod ordinarie e l'unione serve le loro DLL come
qualsiasi altro file - ma tre impostazioni non sono visibili da dentro il gioco, e
ciascuna fa sì che la funzione non faccia nulla, senza un errore: la variabile
NVAPI, lo schermo intero esclusivo e il tetto FPS di Reflex.

La scheda Diagnostics controlla le prime due, e la riga ENB più sotto, per un gioco
che ha un prefisso Wine (il prefisso Proton di Steam, o quello che ha creato
Heroic). Il tetto di Reflex non viene controllato. Quando l'istanza avvia il gioco
da una copia come `Stock Game`, i controlli leggono i file della copia, non
l'installazione dello store. `eidos check <instance>` stampa le stesse schede in un
terminale, senza i pulsanti Fix.

- La scheda NVAPI compare per una copia Steam di Skyrim SE o Enderal SE quando è
  caricato un driver NVIDIA (esiste `/proc/driver/nvidia/version`), il `Data`
  unito ha qualcosa in `Shaders/Upscaling`, la build di Proton del gioco lascia
  NVAPI disattivato per impostazione predefinita, e né l'opzione di avvio di Steam,
  né l'ambiente proprio di Eidos, né il `launch-env.ini` del profilo impostano
  `PROTON_ENABLE_NVAPI=1`.
- La scheda dello schermo intero esclusivo compare solo con il pacchetto Upscaling
  installato, quando il `SkyrimPrefs.ini` del profilo ha `bFull Screen=1` e
  `SKSE/Plugins/SSEDisplayTweaks.ini` non imposta insieme `Fullscreen=false` e
  `Borderless=true`. Il suo pulsante Fix scrive `bFull Screen=0` e `bBorderless=1`
  nel `SkyrimPrefs.ini` del profilo. Il controllo legge `SkyrimPrefs.ini` anche per
  Enderal SE, e un profilo Enderal SE contiene `EnderalPrefs.ini`, quindi su
  Enderal SE usa invece la scheda Display qui sotto.
- La scheda ENB compare per ogni gioco il cui `enblocal.ini` vincente nella vista
  root non ha `LinuxVersion=true` in `[GLOBAL]`. Ha anch'essa un pulsante Fix.

## L'opzione di avvio di cui DLSS ha bisogno

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Usa il percorso assoluto del tuo `eidos-gui` o `eidos-linux`, come in
[install.md](install.md). La scheda dà la riga da impostare: l'opzione di avvio
attuale del gioco con `PROTON_ENABLE_NVAPI=1` davanti, oppure, quando non ce n'è
una, la riga costruita con il percorso dell'Eidos in esecuzione.

Senza lo strato NVIDIA NVAPI (dxvk-nvapi), CS non riesce a inizializzare DLSS e
ripiega sull'upscaling FSR, senza nulla a schermo che dica perché. Se una build di
Proton attivi lo strato da sola è scritto nel suo script `proton`, ed Eidos legge
lì la riga `use_nvapi`. Un interruttore di esclusione (`disablenvapi`) significa
attivo per impostazione predefinita: GE-Proton11-9 e Proton 11.0 hanno questa
forma (verificato il 2026-11-09). Un interruttore di adesione (`enablenvapi`)
significa disattivato, e solo quella build ha bisogno della variabile. Impostarla
non costa nulla su macchine non NVIDIA. La generazione di fotogrammi in sé è FSR
3.1 e non richiede NVAPI; serve solo all'upscaler DLSS.

Per impostarla solo per un profilo, metti `env=PROTON_ENABLE_NVAPI=1` nel
`launch-env.ini` di quel profilo (View -> INI editor, ultima scheda).

## La generazione di fotogrammi richiede la finestra senza bordi

La generazione di fotogrammi di CS gira su un proxy di presentazione D3D12 e
rifiuta senz'altro lo schermo intero esclusivo. Con `bFull Screen=1` in
`SkyrimPrefs.ini` non si innesta mai: nessun errore, nessun messaggio, e il gioco
resta al frame rate di base. SSE Display Tweaks imposta la modalità a livello di
motore, qualunque cosa dicano gli INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

La finestra appare identica (senza bordi, a risoluzione nativa). Cambia soltanto la
modalità che il motore dichiara, ed è quella che CS controlla.

La scheda Display in Diagnostics imposta dimensione e modalità in un solo passo,
per Skyrim SE, Enderal SE e Fallout 4. Mostra la dimensione dello schermo connesso
più grande. Con Borderless window selezionato, Apply scrive `iSize W`, `iSize H`,
`bFull Screen=0` e `bBorderless=1` nel `SkyrimPrefs.ini` del profilo
(`EnderalPrefs.ini` per Enderal SE, `Fallout4Prefs.ini` per Fallout 4). Quando è
SSE Display Tweaks a impostare la finestra, Apply scrive anche `Resolution`,
`Fullscreen=false` e `Borderless=true` in una copia del suo INI nell'Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), e il file della mod resta com'è. Con
Start the game at once when a store opens Eidos attivo (Preferences, General,
Running a game), la finestra si apre comunque quando il profilo non ha ancora una
dimensione della finestra e SSE Display Tweaks non imposta la finestra, così la
dimensione può essere impostata prima del primo avvio.

Altre due condizioni di attivazione, con lo stesso fallimento silenzioso:

- Il display si aggiorna a 120 Hz o più, oppure `frameGenerationForceEnable` è
  impostato nelle impostazioni di upscaling di CS. La generazione di fotogrammi
  raddoppia la cadenza presentata, quindi CS si rifiuta di armarla su display che
  non possono mostrarne il risultato.
- Il pacchetto Upscaling è installato (il suo albero `Data/Shaders/Upscaling/`
  contiene le DLL di Streamline e FidelityFX). CS senza di esso mostra le voci di
  menu e non riesce ad abilitare nulla.

## Il limite di frame rate di Reflex può strozzare l'uscita

Le impostazioni Reflex di CS portano un proprio tetto di FPS (`reflexFPSLimit`, con
`reflexUseFPSLimit`). Un tetto rimasto a un valore precedente - il nostro era 79 da
una vecchia taratura - sta a valle della generazione di fotogrammi e taglia
esattamente i fotogrammi che essa produce: 60 di base raddoppiati a 120, ritagliati
a 79, si legge come "la generazione di fotogrammi non fa nulla". Su un display a
144 Hz il tetto Reflex consueto è ~138. Controllalo ogni volta che i fotogrammi
generati sembrano mancare.

## Interazione nota: schermo nero con SSE Display Tweaks

La combinazione FG + Display Tweaks + DXVK ha un noto guasto a schermo nero.
Rimedio, in ordine:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Se non basta, un `dxvk.conf` accanto all'eseguibile del gioco (la cartella
   `Root/` di una mod ne colloca uno lì) con
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mod di texture misti: PGPatcher

Community Shaders sa rendere parallax, complex material e PBR, ma solo dove una
mesh è predisposta. Storicamente questo voleva dire installare mesh già patchate e
poi una mod per rispegnere l'effetto dove le texture mancavano: copertura limitata
alle mesh che si avevano, e CPU spesa a runtime per disfare una scelta fatta al
momento dell'installazione.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) la
capovolge. Installi le texture che vuoi, di qualunque tipo di shader, e lui
riscrive mesh e plugin perché ogni superficie usi quello giusto - compresi i
record di texture alternative nei plugin, che il vecchio metodo lasciava scoperti.
Senza di esso, una texture che richiede uno shader che la sua mesh non chiede si
carica ma non viene resa come previsto, cosa che succede più spesso in
un'installazione che mescola tipi di texture.

Due cose da sapere prima di lanciarlo su Linux:

- gli serve l'argomento `--ignore-mo2vfscheck` o esce subito lamentandosi di MO2.
  Eidos lo fornisce - vedi
  [Strumenti](tools.md#perché-pgpatcher-ha-bisogno-di---ignore-mo2vfscheck) per cosa sia il
  flag e perché il controllo qui fallisca;
- modifica mesh, quindi gira dopo l'installazione di tutte le mod di texture e
  prima di DynDOLOD, che deve vedere le mesh finali. Cambiare le texture in
  seguito significa rilanciare entrambi, in quest'ordine.

Il suo output non finisce nell'Overwrite. PGPatcher scrive solo nella Output
location impostata nel suo launcher, e rifiuta una cartella dentro `Data`. Punta
quella posizione su una cartella tutta sua, per esempio una nuova cartella
`PGPatcher Output` sotto `mods/` dell'istanza. Tieni quella mod disattivata mentre
PGPatcher gira, perché PGPatcher svuota la cartella prima di riscriverla ed Eidos
si aspetta che le mod attive non cambino finché la vista è montata. Poi attivala
con una priorità alta: è fatta per vincere.

Non dare alla voce di PGPatcher una mod in Capture output into per poi usare quella
mod come Output location. Eidos sposta in quella mod le cartelle `cfg/` e `log/`
proprie di PGPatcher dopo ogni esecuzione, e PGPatcher allora si ferma con
"Output directory has non-PGPatcher related files".

## ENB con Proton: LinuxVersion

Jackify scrive `LinuxVersion=true` nella sezione `[GLOBAL]` di `enblocal.ini` per
ogni ENB che installa con Proton. La documentazione di ENB non descrive questa
chiave. Una scheda Health nella scheda Diagnostics segnala un `enblocal.ini` senza
la riga, e il suo pulsante Fix scrive in `overwrite/Root/` una copia
dell'`enblocal.ini` vincente con la riga impostata, così la mod ENB mantiene il
proprio file.

Un'altra scheda avvisa quando un ENB si trova nella cartella del gioco (un
`enblocal.ini`, un `enbseries.ini` o una cartella `enbseries/`) mentre una mod
attiva porta Community Shaders (`SKSE/Plugins/CommunityShaders.dll`). I due possono
girare insieme. Se l'immagine sembra sbagliata, disattiva uno dei due nel suo INI.

## Leggere i numeri dopo

I fotogrammi generati esistono solo sul lato presentazione: il motore continua a
simulare alla cadenza di base, Havok continua a battere alla cadenza di base, e
tutto ciò che conta i fotogrammi *del motore* (compresi i contatori di CS) continua
a riportare ~60 mentre il display ne mostra ~120. Il contatore è giusto, ed è il
motivo per cui la generazione di fotogrammi è sicura per la fisica là dove alzare
la cadenza del motore non lo è. `DXVK_HUD=fps` nelle opzioni di avvio mostra un
contatore se ne vuoi uno a schermo.

Una regola: l'interpolazione a livello di driver (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) e la generazione di fotogrammi di CS sono
tecnologie concorrenti. Usane una o l'altra, mai entrambe.
