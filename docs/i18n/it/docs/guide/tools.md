<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Strumenti: xEdit, BodySlide, DynDOLOD, FNIS

Uno strumento eseguito attraverso Eidos vede **la vista unita**, dentro il
prefisso Proton del gioco stesso. Legge ciò che leggerà il gioco - ogni mod
attiva, in ordine di priorità - e tutto ciò che scrive atterra nell'Overwrite,
dove un clic lo trasforma in una mod vera.

## Quelli che Eidos trova da sé

Alcuni strumenti hanno un nome abbastanza univoco da essere trovati invece che
dichiarati, e xEdit è il caso ovvio: `FO4Edit.exe` per Fallout 4, `SSEEdit.exe`
per Skyrim SE, `TES5Edit.exe` per l'originale, e così via - insieme al gemello
**QuickAutoClean** di ciascuno, che è il pulsante per le modifiche sporche di cui
LOOT continua ad avvertire. Eidos li cerca, per nome di file, in:

- la cartella d'installazione del gioco e gli alberi `Root/` delle mod attive;
- **la `mods/` di questa istanza**, dove gli utenti MO2 installano gli strumenti;
- la **cartella degli strumenti** che imposti in Settings (Tools -> Tools
  folder), per la directory condivisa tra istanze - `/mnt/Games/Tools` e simili.

L'elenco è per gioco, quindi a un'istanza di Skyrim non viene mai proposto
l'editor di Fallout. La ricerca si ferma a quattro livelli di profondità, perché
un insieme di mod sono centinaia di migliaia di file e questo gira ogni volta che
l'elenco degli strumenti viene costruito, e non segue i symlink. Uno strumento
trovato così è configurato esattamente come uno inserito a mano: i suoi runtime
derivano dal nome, secondo la stessa regola di tutto ciò che segue.

Se uno strumento sta altrove, o vuoi argomenti diversi, aggiungilo a mano - una
voce utente con lo stesso titolo prevale su qualunque cosa trovata
automaticamente.

## Aggiungerne uno

Nella GUI: **Tools -> Executables**, poi Add. Da riga di comando:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # elencare ciò che è registrato
eidos tool skyrimse run BodySlide         # eseguirlo attraverso la vista unita
eidos tool skyrimse run BodySlide --print # mostrare il comando senza eseguirlo
```

Lo script extender, l'eseguibile del gioco e il lanciatore vengono rilevati
automaticamente; solo gli strumenti aggiuntivi vanno registrati.

### Puntalo al file reale, ovunque sia

Registra l'eseguibile dove si trova davvero. Se lo strumento è stato installato
come mod, è dentro la cartella della mod:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(quello è il percorso dell'istanza globale - per un'istanza portatile vale la
stessa regola sotto la sua cartella, `<instance>/mods/...`; nota che un percorso
assoluto come questo è l'unica cosa che non sopravvive allo SPOSTAMENTO
successivo di una cartella portatile).

Eidos riscrive quel percorso in quello unito prima di avviare, così lo strumento
gira da `<game>/Data/CalienteTools/BodySlide/` e lì vede anche i file di ogni
altra mod. Conta più di quanto sembri: BodySlide porta con sé una directory
`SliderSets` **vuota**, e ogni corpo che può costruire viene da CBBE e dalle mod
di vestiario. Avviato dalla propria cartella della mod non trova nulla e sembra
rotto.

MO2 fa la stessa riscrittura, per la stessa ragione - il suo stesso commento
nomina FNIS.

Uno strumento dentro una mod **disattivata** non può essere riscritto, perché
nemmeno i suoi file sono nella vista. Eidos lo dice e lo esegue dalla sua
cartella invece di fingere.

## Mandare l'output di uno strumento in una mod sua

Un generatore - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - scrive centinaia
di file. Per impostazione predefinita atterrano nell'Overwrite insieme a tutto il
resto. Imposta **Capture output into** nell'editor Executables e l'output di
questa esecuzione finisce invece in quella mod:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

La mod viene creata se non esiste. Si spostano solo i file prodotti da QUESTA
esecuzione; tutto ciò che era già nell'Overwrite resta lì, così due strumenti con
una destinazione di cattura non si rubano l'output a vicenda. Un'esecuzione che
non ha scritto nulla non lascia dietro di sé una mod vuota.

Viene fatto dopo l'esecuzione invece che puntando il livello di scrittura sulla
mod, che è il modo di MO2. Puntare il livello di scrittura su una mod la
promuoverebbe alla priorità massima per tutta l'esecuzione - ribaltando ogni
conflitto in cui si trova e ribaltandoli di nuovo alla fine - e scriverebbe
dritto attraverso i file della mod stessa senza copy-up. La cattura arriva allo
stesso stato finale senza né l'uno né l'altro.

Se la mod di destinazione è disattivata, l'output viene comunque scritto ma il
gioco non lo vedrà, quindi lo strumento rigenererebbe gli stessi file alla
prossima esecuzione. Eidos avvisa quando è così.

## Le DLL di cui uno strumento ha bisogno sono scelte dal suo NOME

Questa è la parte sorprendente, quindi vale la pena dirla chiaramente: **il
titolo che dai a uno strumento decide quali prerequisiti di runtime Eidos gli
predispone.** La corrispondenza è una sottostringa del titolo, senza distinzione
tra maiuscole e minuscole.

| Se il titolo contiene | Eidos richiede |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| qualsiasi altra cosa | nulla |

Così uno strumento registrato come **`BodySlide`** ottiene le sue DLL DirectX; lo
stesso eseguibile registrato come **`BS`** non ottiene nulla e può non partire con
un errore che non dice niente sulle DLL. Dai agli strumenti il nome del
programma.

L'elenco è in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), e il
campo `Prereqs` nella finestra Executables è modificabile - il rilevamento è un
valore predefinito, non una regola.

### Tre tipi di prerequisito

**Livello 1 - DLL incluse** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos le
distribuisce e le copia nel prefisso all'avvio. Niente da fare, niente rete.

**Livello 2 - verbi winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Scrivono chiavi di registro, la GAC e gli host CLR,
quindi non possono essere copiati come file. **Scaricano da Microsoft**.

**Livello 3 - runtime** (`dotnet10`). Un runtime .NET moderno è fatto di 193 file
che vivono in una directory propria e vengono trovati attraverso `DOTNET_ROOT`:
mai registrati, mai installati nel prefisso, quindi nessuno degli altri due
livelli può trasportarlo. Eidos lo scarica da sé, lo verifica contro un checksum
incorporato nel binario, e lo mette in cache in
`~/.local/share/Colony/Eidos/runtimes/` - **fuori da ogni istanza**, perché 78 MB
non sono per gioco né per profilo.

Niente nei livelli 2 o 3 gira in sordina:

```sh
eidos prereqs skyrimse            # mostrare ciò di cui hanno bisogno gli strumenti registrati, e il loro stato
eidos prereqs skyrimse --install  # recuperare ciò che manca (download)
```

Nella GUI gli stessi stati stanno sotto il campo Prereqs, e quelli mancanti sono
pulsanti. Un verbo che non è né incluso, né un runtime, né un verbo winetricks
conosciuto viene segnalato come probabile errore di battitura invece che offerto
come download.

### Perché DynDOLOD ha bisogno di `dotnet10`

DynDOLOD non costruisce da sé il LOD degli oggetti: si appoggia a LODGen, e ne
distribuisce tre. `LODGenx64.exe` punta a .NET Framework 4.8, che sotto Proton
viene indirizzato al Mono di Wine - il cui inizializzatore di `System.Uri` chiama
un metodo che Mono non implementa. Muore prima della sua prima riga di lavoro,
lasciando un log che contiene un'intestazione di versione e nient'altro, e una
finestra DynDOLOD che dice soltanto "failed for one or more worlds".

Installare il vero .NET Framework non lo risolve: Proton sostituisce
`mscoree.dll` - il caricatore che lo troverebbe - con un symlink dentro il
proprio albero, e lo rifà a ogni aggiornamento del prefisso.

La build che funziona è `LODGenx64Win10.exe`, che punta a .NET moderno e non tocca
mai `mscoree`. Punta `DOTNET_ROOT` a un runtime .NET 10 e gira. È quello che
`dotnet10` predispone, ed Eidos imposta la variabile quando avvia qualsiasi
strumento che lo dichiari.

Eidos esegue il `winetricks` di sistema contro il `wine` di Proton e il prefisso
del gioco, il che aggira il contenitore pressure-vessel di Steam e la discordanza
protontricks + Proton-GE. Uno strumento che dichiara un verbo di Livello 2 non
installato parte comunque, con un avviso che nomina il verbo e il comando per
rimediare - l'utente potrebbe averlo da altrove.

## Il percorso del gioco nel prefisso

Gli strumenti Windows trovano il proprio gioco leggendo
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, una chiave che scrive
l'installer del gioco stesso - e che Steam sotto Proton non esegue mai. Senza di
essa xEdit, Wrye Bash e DynDOLOD si aprono su un percorso vuoto. Eidos la scrive
prima di eseguire uno strumento: idempotente, additiva, e saltata se il prefisso
non è inizializzato o è in uso.

## Raggiungere uno strumento: nascondere, fissare e una scorciatoia sul desktop

I valori predefiniti di un gioco includono strumenti che forse non userai mai, e
un selettore che elenca otto voci per arrivare alla seconda è un selettore che
nessuno legge. Nella finestra Executables:

- **Pin to top** mette una voce in cima all'elenco Run.
- **Hide from picker** ne toglie una senza cancellarla.
- **Desktop shortcut** scrive un `.desktop` in
  `~/.local/share/applications` - dove un lanciatore sta di casa su un sistema
  freedesktop, così spunta nel menu delle applicazioni e in una ricerca invece
  che sul desktop. Esegue direttamente `eidos tool <instance> run <title>`, il
  che significa che lo strumento si apre **attraverso la vista unita con il
  profilo di questa istanza** senza che la finestra di Eidos sia affatto aperta.

Nascondere e fissare riguardano il modo in cui uno strumento viene *raggiunto*
più che ciò che esegue, quindi valgono tanto per i valori predefiniti per gioco
quanto per le tue voci.

## Uno strumento che è un'app Steam a sé

Il Creation Kit è un'applicazione Steam separata e vuole il proprio AppID; qualche
altro strumento di modding distribuito su Steam è uguale. Imposta **Steam AppID**
sulla voce ed Eidos lo avvia sotto quell'id invece che sotto quello del gioco.

Su Windows questo significa un lanciatore diverso. Qui sono due variabili
d'ambiente sull'esecuzione che si stava già costruendo - `SteamAppId` e
`SteamGameId`, entrambe, perché Proton legge l'una e le librerie di Steam leggono
l'altra, e uno strumento che le vede in disaccordo fallisce in modo strano invece
che chiaro. `eidos tool ... --print` mostra esattamente ciò che riceverebbe
l'esecuzione vera.

## Le impostazioni di uno strumento restano affar suo

Eidos mette uno strumento nel posto giusto con le DLL giuste. Quello che poi lo
strumento fa con la propria configurazione è cosa tra te e lo strumento, e il
fallimento di solito è silenzioso.

L'esempio svolto, perché altrimenti costa un'ora: il **Game Data Path** di
BodySlide (Settings) deve puntare alla directory `Data` del gioco, non alla
cartella del gioco che sta sopra. Impostato un livello troppo in alto, una build
in blocco riporta "All sets processed successfully" e scrive 1439 mesh dove il
gioco non le cercherà mai. Eidos le intercetta - atterrano in `Overwrite/Root/`
invece che nella tua installazione - ma dal punto di vista del gioco non c'è
niente di sbagliato, tranne che i tuoi corpi non sono costruiti.

L'output degli strumenti sta di casa nell'Overwrite. Quando un'esecuzione produce
qualcosa che vale la pena tenere, **Overwrite -> Create mod...** lo trasforma in
una mod ordinaria che può essere ordinata, disattivata e rimossa come qualsiasi
altra.
