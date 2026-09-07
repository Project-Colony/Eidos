<!-- eidos-i18n: source=docs/guide/graphics.md sha=ee3cee732aafbf91d8085d03a74f7d4e0cfa224d -->

# Community Shaders, DLSS e generazione di fotogrammi

Community Shaders 1.4+ porta con sé il proprio upscaling (DLSS 4 / FSR 3.1 / XeSS,
tramite il pacchetto separato "Upscaling - Community Shaders") e la generazione di
fotogrammi FSR 3.1. Tutto funziona attraverso Eidos su Linux - CS e i suoi
pacchetti si installano come mod ordinarie e l'unione serve le loro DLL come
qualsiasi altra cosa - ma tre cose **non** sono scopribili da dentro il gioco, e
ciascuna fa sì che la funzione non faccia nulla, in silenzio. Questa pagina è
l'elenco, imparato a caro prezzo su un'installazione reale.

## L'opzione di avvio di cui DLSS ha bisogno

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton disabilita il proprio strato NVIDIA NVAPI (dxvk-nvapi) a meno che il gioco
non sia nella lista di Valve, e Skyrim non c'è. Senza di essa CS non riesce a
inizializzare DLSS e ripiega sull'upscaling FSR - in sordina, senza nulla a schermo
che dica perché. Impostare la variabile non costa nulla su macchine non NVIDIA,
quindi l'opzione di avvio sicura è semplicemente la riga qui sopra. La generazione
di fotogrammi in sé è FSR 3.1 e non richiede NVAPI; serve solo all'upscaler DLSS.

## La generazione di fotogrammi richiede la finestra senza bordi

La generazione di fotogrammi di CS gira su un proxy di presentazione D3D12 e
rifiuta senz'altro lo schermo intero esclusivo. `bFull Screen=1` in
`SkyrimPrefs.ini` significa che non si innesta mai - nessun errore, nessun
messaggio, solo il frame rate di base. Il rimedio solido è SSE Display Tweaks, che
impone la modalità a livello di motore qualunque cosa dicano gli INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

La finestra appare identica (senza bordi, a risoluzione nativa); cambia soltanto
ciò che il motore crede - e ciò che il motore crede è ciò che CS controlla.

Altre due condizioni di attivazione, con lo stesso fallimento silenzioso:

- **Aggiornamento dello schermo a 120 Hz o più**, oppure imposta
  `frameGenerationForceEnable` nelle impostazioni di upscaling di CS. La
  generazione di fotogrammi raddoppia la cadenza presentata, quindi CS si rifiuta
  di armarla su display che non possono mostrarne il risultato.
- **Il pacchetto Upscaling installato** (il suo albero `Data/Shaders/Upscaling/`
  contiene le DLL di Streamline e FidelityFX). CS senza di esso mostra le voci di
  menu e non riesce ad abilitare nulla.

## Il limite di frame rate di Reflex può strozzare l'uscita

Le impostazioni Reflex di CS portano un proprio tetto di FPS (`reflexFPSLimit`, con
`reflexUseFPSLimit`). Un tetto rimasto a un valore precedente - il nostro era 79 da
una vecchia taratura - sta a valle della generazione di fotogrammi e taglia
esattamente i fotogrammi che essa produce: 60 di base raddoppiati a 120, ritagliati
a 79, si legge come "la generazione di fotogrammi non fa nulla". Su un display a
144 Hz il tetto Reflex consueto è ~138. Controllalo ogni volta che l'uscita
generata sembra mancare; è il secondo killer silenzioso dopo lo schermo intero
esclusivo.

## Interazione nota: schermo nero con SSE Display Tweaks

La combinazione FG + Display Tweaks + DXVK ha un noto guasto a schermo nero.
Rimedio, in ordine:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Se non basta, un `dxvk.conf` accanto all'eseguibile del gioco (la cartella
   `Root/` di una mod ne colloca uno lì) con
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mod di texture misti: PGPatcher

Community Shaders sa rendere parallax, complex material e PBR, ma solo dove una
mesh e predisposta. Storicamente questo voleva dire installare mesh gia
patchate e poi un mod per rispegnere l'effetto dove le texture mancavano:
copertura limitata alle mesh che si avevano, e CPU spesa a runtime per disfare
una decisione che non andava presa.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946)
capovolge la cosa. Installi le texture che vuoi, di qualunque tipo di shader, e
lui riscrive mesh e plugin perche ogni superficie usi quello giusto - compresi i
record di texture alternative nei plugin, che il vecchio metodo lasciava
scoperti. E la differenza tra un pacchetto di texture che si vede e uno che si
limita a caricarsi, e conta soprattutto su un'installazione mista, cioe su
qualunque ordine di caricamento reale.

Due cose da sapere prima di lanciarlo su Linux:

- gli serve l'argomento **`--ignore-mo2vfscheck`** o esce subito lamentandosi di
  MO2. Eidos lo fornisce - vedi [Strumenti](tools.md#perche-pgpatcher-ha-bisogno-di---ignore-mo2vfscheck) per cosa sia il
  flag e perche il controllo qui fallisca;
- modifica mesh, quindi gira **dopo** l'installazione di tutti i mod di texture
  e **prima** di DynDOLOD, che deve vedere le mesh finali. Cambiare le texture
  in seguito significa rilanciare entrambi, in quest'ordine.

Il suo output finisce nell'Overwrite come quello di ogni strumento, dove un clic
lo trasforma in un mod. Dai a quel mod una priorita alta: e fatto per vincere.

## Leggere i numeri dopo

I fotogrammi generati esistono solo sul lato presentazione: il motore continua a
simulare alla cadenza di base, Havok continua a battere alla cadenza di base, e
tutto ciò che conta i fotogrammi *del motore* (compresi i contatori di CS) continua
a riportare ~60 mentre il display ne mostra ~120. È il comportamento corretto, non
un contatore rotto - ed è il motivo per cui la generazione di fotogrammi è sicura
per la fisica là dove alzare la cadenza del motore non lo è. `DXVK_HUD=fps` nelle
opzioni di avvio mostra un contatore se ne vuoi uno a schermo.

Una regola: l'interpolazione a livello di driver (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) e la generazione di fotogrammi di CS sono
tecnologie concorrenti. Usane una o l'altra, mai entrambe.
