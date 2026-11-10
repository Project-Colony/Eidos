<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Strumenti: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Uno strumento eseguito attraverso Eidos vede la vista unita, dentro il prefisso
Wine del gioco (il prefisso Proton di Steam, o quello che ha creato Heroic).
Legge ciò che leggerà il gioco (ogni mod attiva, in ordine di priorità), e tutto
ciò che scrive atterra nell'Overwrite, dove un clic lo trasforma in una mod vera.
Uno strumento `runner=native` gira senza Proton; vedi
[più sotto](#variabili-dambiente-e-strumenti-avviati-senza-proton).

## Quelli che Eidos trova da sé

Alcuni strumenti hanno un nome abbastanza univoco da essere trovati invece che
dichiarati, e xEdit è il caso ovvio: `FO4Edit.exe` per Fallout 4, `SSEEdit.exe`
per Skyrim SE, `TES5Edit.exe` per l'originale, e così via. Ciascuno arriva con una
voce QuickAutoClean, che pulisce le modifiche sporche di cui LOOT avverte.
`PGPatcher.exe` viene trovato allo stesso modo su ogni gioco Bethesda tranne
Morrowind, e arriva con un argomento senza il quale non può funzionare (vedi
[più sotto](#perché-pgpatcher-ha-bisogno-di---ignore-mo2vfscheck)). Eidos li
cerca, per nome di file, in:

- `Overwrite/Root/`, la cartella del gioco da cui parte l'istanza (l'installazione
  dello store, o la sua copia del gioco), e gli alberi `Root/` delle mod attive;
- la `mods/` di questa istanza, dove gli utenti MO2 installano gli strumenti;
- la cartella degli strumenti che imposti in Preferences (General -> Tools), per
  una directory condivisa tra istanze, come `/mnt/Games/Tools`.

I generatori più usati si trovano allo stesso modo, ciascuno per i giochi che
supporta: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen e Synthesis.
Lo stesso vale per BethINI Pie (`Bethini.exe`), sugli stessi giochi di PGPatcher;
vedi [BethINI Pie](#bethini-pie). Su Stardew Valley, SMAPI
(`StardewModdingAPI.exe`) si trova allo stesso modo. È anche il loader dello script
extender di quel gioco, quindi avvia il gioco attraverso lo store, come descritto
in [Raggiungere uno strumento](#raggiungere-uno-strumento-nascondere-fissare-e-una-scorciatoia-sul-desktop).

L'elenco è per gioco, quindi a un'istanza di Skyrim non viene mai proposto
l'editor di Fallout. La ricerca si ferma a quattro livelli di profondità, perché
un insieme di mod sono centinaia di migliaia di file e questo gira ogni volta che
l'elenco degli strumenti viene costruito, e non segue i symlink. Uno strumento
trovato così è configurato esattamente come uno inserito a mano: i suoi
prerequisiti, i suoi argomenti (il `--ignore-mo2vfscheck` di PGPatcher) e la sua
mod di cattura derivano dal titolo, secondo le regole qui sotto.

Se uno strumento sta altrove, o vuoi argomenti diversi, aggiungilo a mano. Una
voce utente con lo stesso titolo prevale su qualunque cosa trovata
automaticamente.

## Pulire un plugin che LOOT segnala come sporco

Dopo un ordinamento LOOT, la scheda Diagnostics elenca i plugin che LOOT segnala
come sporchi, per i giochi che hanno sia un xEdit sia i dati di pulizia di LOOT
(Skyrim LE, SE e VR, Enderal SE, Fallout 3, New Vegas, Fallout 4 e 4 VR,
Oblivion, Starfield). Quando Eidos trova xEdit, registrato in Executables o in
qualunque punto dell'istanza in cui cerca gli strumenti, la scheda ha Clean all e
ogni plugin sporco ha Clean with QuickAutoClean nel menu della sua riga;
altrimenti la scheda dice di aggiungere xEdit in Executables. Eidos non scarica
xEdit. Esegue la modalità QuickAutoClean di xEdit su un plugin alla volta,
attraverso la vista unita, con `-qac -autoexit -autoload`. xEdit salva il plugin
pulito sopra quello che ha caricato, e nella vista unita questo è un file nuovo
nell'Overwrite; Eidos lo sposta in una mod chiamata `Cleaned plugins`, creata
attiva in fondo alla lista, dove vince ogni conflitto. Il plugin nella sua mod, o
nella cartella Data del gioco, non viene mai scritto. Disattiva `Cleaned plugins`,
o nascondi un suo file, per annullare una pulizia. Un plugin che sta
nell'Overwrite non viene pulito, perché xEdit riscriverebbe quel file da sé;
trasforma prima l'Overwrite in una mod.

I backup, la cache e il log di xEdit finiscono in `<instance>/logs/qac-<time>/`,
fuori dalla vista unita, e la riga di stato riporta i numeri di quel log. Eidos
cancella quella cartella dopo una pulizia riuscita e la tiene dopo una fallita.
Eidos annota in `Cleaned plugins/meta.ini` la dimensione e la data del plugin da
cui è stata fatta ogni copia. Quando quel plugin cambia, la scheda Diagnostics
mostra "Out-of-date copy of <plugin>", con un pulsante che nasconde la vecchia
copia.

## Aggiungerne uno

Nella GUI: Tools nella barra dei menu, oppure Executables nella barra degli
strumenti, poi Add. Da riga di comando:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # elencare ciò che è registrato
eidos tool skyrimse run BodySlide         # eseguirlo attraverso la vista unita
eidos tool skyrimse run BodySlide --print # mostrare il comando senza eseguirlo
eidos tool skyrimse rm BodySlide          # rimuovere una voce che hai aggiunto
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # avviare un exe una volta, senza registrarlo
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` cattura l'output di quell'esecuzione in `mods/<mod>`, qualunque
sia l'impostazione propria dello strumento. `rm` rimuove solo le voci che hai
aggiunto; un valore predefinito per gioco può essere nascosto, non rimosso.

Un exe avviato con `--exe` il cui nome di file è uno che Eidos conosce per il
gioco, senza distinzione tra maiuscole e minuscole, riceve il titolo di quello
strumento, e con esso i prerequisiti, gli argomenti e la mod di cattura qui sotto.
`GenerateFNISforUsers.exe` è FNIS, quindi la riga `--exe` qui sopra chiede
`dotnet48` e cattura in `FNIS Output`. Qualsiasi altro exe ha come titolo il suo
nome di file. Nella scheda Data, una riga `.exe` ha Add tool, che apre la finestra
Executables con la voce già compilata, e Run, che esegue il file una volta come fa
`--exe`. Entrambi danno al file lo stesso titolo.

Per un comando che non è uno strumento Windows, `eidos shell skyrimse` apre la tua
`$SHELL` (`/bin/sh` se non è impostata) nella cartella del gioco con la vista unita
montata. Ciò che la shell scrive finisce nell'Overwrite, e uscire dalla shell
smonta la vista. `eidos shell skyrimse -- <command>` esegue invece un solo
comando. Ctrl+C nella shell interattiva ferma il comando che hai digitato e lascia
la shell aperta. La shell gira senza Proton; un programma Windows passa da
`eidos tool`.

Nella finestra, Stop termina un'esecuzione di strumento che si blocca: premilo
due volte, ed Eidos termina i programmi dell'esecuzione, poi tratta i suoi file
come dopo qualsiasi uscita. Con Notify the desktop when a long job ends attivo
(Preferences, General, Running a game), un'esecuzione di strumento che finisce
mentre la finestra è in secondo piano invia una notifica desktop.

Lo script extender, l'eseguibile del gioco e il lanciatore vengono rilevati
automaticamente; solo gli strumenti aggiuntivi vanno registrati. In un'istanza che
parte da una copia del gioco vengono rilevati nella copia; vedi
[Strumenti su una copia del gioco](#strumenti-su-una-copia-del-gioco).

### Puntalo al file reale, ovunque sia

Registra l'eseguibile dove si trova davvero. Se lo strumento è stato installato
come mod, è dentro la cartella della mod:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Quello è il percorso dell'istanza globale. Per un'istanza portatile vale la stessa
regola sotto la sua cartella, `<instance>/mods/...`. Un percorso assoluto come
questo è l'unica cosa che non sopravvive allo spostamento successivo di una
cartella portatile.

Eidos riscrive quel percorso in quello unito prima di avviare, così lo strumento
gira da `<game>/Data/CalienteTools/BodySlide/` e lì vede anche i file di ogni
altra mod. BodySlide porta con sé una directory `SliderSets` vuota, e ogni corpo
che può costruire viene da CBBE e dalle mod di vestiario. Avviato dalla propria
cartella della mod non trova nulla e sembra rotto.

MO2 fa la stessa riscrittura, per la stessa ragione: il suo stesso commento nomina
FNIS.

Uno strumento dentro una mod disattivata non può essere riscritto, perché nemmeno
i suoi file sono nella vista. Eidos lo dice e lo esegue dalla sua cartella.

## Strumenti da una lista di Mod Organizer 2

Una lista Wabbajack o un'altra istanza MO2 aperta come istanza Eidos conserva il
suo `ModOrganizer.ini`, e gli eseguibili configurati lì possono essere copiati in
`tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # mostra cosa verrebbe aggiunto
eidos tool /path/to/list import-mo2 --write   # le aggiunge
```

Nella finestra, la finestra Executables ha Import from ModOrganizer.ini, con una
casella da deselezionare per ogni voce. Add selected mette le voci selezionate
nell'elenco, e non viene salvato nulla finché non premi Save. Conta solo un
`ModOrganizer.ini` che Eidos non ha scritto; per quello che scrive Eidos vedi
[PGPatcher e PGLite](#pgpatcher-e-pglite-in-modalità-mod-organizer-2).

I percorsi su `Z:` sono percorsi Linux e restano come sono. Un percorso su
un'altra lettera di unità viene spostato nella cartella della lista quando nomina
quella cartella (`D:\Lists\MyList\tools\SSEEdit.exe` diventa
`<list>/tools/SSEEdit.exe`), nell'eseguibile, nella cartella di lavoro e negli
argomenti. Il loader dello script extender, il lanciatore e il binario del gioco
diventano semplici nomi, quindi partono dalla cartella del gioco su cui Eidos
monta la vista, come le voci integrate. `hide` di MO2 diventa Hide from picker, e
`steamAppID` diventa Steam AppID. Le voci che puntano altrove (un programma di
sistema di Windows, uno strumento fuori dalla lista) sono elencate come saltate
con il motivo, e un titolo già presente in `tools.ini` non viene mai sostituito.

Una voce importata mantiene il titolo e gli argomenti di MO2, ed Eidos non compila
nulla a partire dal titolo: niente prerequisiti, niente argomento in più, niente
mod di cattura. Impostali nella finestra Executables. Una voce PGPatcher da una
lista fatta su Windows di solito non ha `--ignore-mo2vfscheck`; aggiungilo (vedi
[più sotto](#perché-pgpatcher-ha-bisogno-di---ignore-mo2vfscheck)).

Un percorso importato che punta nella cartella `Stock Game` della lista viene
spostato sull'installazione dello store solo al momento dell'esecuzione, e solo
quando l'istanza parte da quella copia; vedi la sezione successiva.

## Strumenti su una copia del gioco

Quando l'istanza parte da una copia del gioco (la cartella `Stock Game` o
`Game Root` di una lista, o una copia congelata; vedi
[File di gioco](usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia)),
uno strumento vede i file della copia, non quelli dell'installazione dello store.
La vista unita è comunque montata sull'installazione dello store: la vista root ha
la copia in fondo, e la vista Data legge il `Data` della copia. Eidos cerca lo
script extender, il lanciatore, il binario del gioco e gli strumenti noti nella
copia invece che nell'installazione dello store.

Un percorso di strumento, un argomento o una cartella di lavoro che punta nella
copia viene spostato sull'installazione dello store prima dell'esecuzione. Può
essere scritto come percorso Linux o come percorso Wine `Z:`:
`-D:"Z:\...\Stock Game\Data"` diventa lo stesso percorso sotto l'installazione
dello store. Le voci importate dal `ModOrganizer.ini` di una lista ne hanno
bisogno. Quando l'istanza parte dall'installazione dello store non viene spostato
nulla, quindi una voce simile legge la copia nuda senza le tue mod.

L'`installed path` del gioco nel registro del prefisso e il `gamePath` nel
`ModOrganizer.ini` che Eidos scrive nominano ancora l'installazione dello store,
dove sta la vista unita. Se la cartella della copia manca, o non contiene alcun
binario del gioco, l'esecuzione di uno strumento si ferma, come un avvio del
gioco, e il messaggio nomina `eidos stock-game <instance> store` per tornare
all'installazione dello store.

`eidos stock-game <instance>` mostra quali file di gioco l'istanza usa e a quali
copie potrebbe passare. `use <folder>` e `store` li cambiano, e `freeze --yes`
copia l'installazione dello store nella cartella `Stock Game` dell'istanza e parte
da lì. La scheda Game files nella scheda Diagnostics fa lo stesso; vedi
[File di gioco](usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia).

## Mandare l'output di uno strumento in una mod sua

Un generatore (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) scrive centinaia di
file. Per impostazione predefinita atterrano nell'Overwrite insieme a tutto il
resto. Imposta Capture output into nell'editor Executables e l'output di questa
esecuzione finisce invece in quella mod:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide e Synthesis arrivano con una mod di cattura già
impostata: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` e
`Synthesis Output`. DynDOLOD, TexGen e xLODGen arrivano senza, perché scrivono
nella cartella di output che indichi tu.

La mod viene creata se non esiste. Si spostano solo i file prodotti da questa
esecuzione; tutto ciò che era già nell'Overwrite resta lì, così due strumenti con
una destinazione di cattura non si rubano l'output a vicenda. Un'esecuzione che
non ha scritto nulla non lascia dietro di sé una mod vuota.

Eidos sposta i file dopo l'esecuzione. MO2 invece punta il livello di scrittura
sulla mod, che la promuove alla priorità massima per tutta l'esecuzione: ogni
conflitto in cui si trova si ribalta, e si ribalta di nuovo alla fine. Le
scritture passano inoltre dritte attraverso i file della mod stessa, senza
copy-up. La cattura arriva allo stesso stato finale senza né l'uno né l'altro.

Se la mod di destinazione è disattivata, l'output viene comunque scritto ma il
gioco non lo vedrà, quindi lo strumento rigenererebbe gli stessi file alla
prossima esecuzione. Eidos avvisa quando è così.

### Quando il suo output non è aggiornato

Eidos registra da quali mod, ordine dei plugin e profilo è stato prodotto
l'output di uno strumento, la dimensione e la data dell'eseguibile dello
strumento, e il percorso e gli argomenti della voce in `tools.ini`. Quando uno di
questi cambia, la scheda Diagnostics mostra "Generated output inputs changed", e
la scheda ha un pulsante Run <tool> again che esegue lo strumento con la
configurazione attuale.

Per un generatore il cui output non aggiornato rompe il gioco, come Pandora o
Nemesis, aggiungi `check_before_play=1` alla sua voce in `tools.ini`, oppure attiva
Ask before playing when its output is out of date per esso nella finestra
Executables. Run chiede allora prima: eseguire lo strumento o giocare comunque.
Play anyway viene ricordato finché mod, plugin o profilo non cambiano di nuovo, o
finché la finestra non si chiude. Eidos non avvia mai lo strumento da solo. Con
Start the game at once when a store opens Eidos attivo, una domanda simile tiene
aperta la finestra invece di avviare il gioco.

## Prerequisiti e argomenti vengono dal titolo

Il titolo che dai a uno strumento decide quali prerequisiti di runtime Eidos gli
predispone. La corrispondenza è una sottostringa del titolo, senza distinzione tra
maiuscole e minuscole.

| Se il titolo contiene | Eidos richiede |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| qualsiasi altra cosa | nulla |

Così uno strumento registrato come `BodySlide` ottiene le sue DLL DirectX; lo
stesso eseguibile registrato come `BS` non ottiene nulla e può non partire con un
errore che non dice niente sulle DLL. Dai agli strumenti il nome del programma.

L'elenco è in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), e il campo
`Prereqs` nella finestra Executables è modificabile: il titolo riempie solo il
valore predefinito.

Il titolo decide gli argomenti allo stesso modo, per l'unico strumento che ne
richiede uno che nessuno potrebbe indovinare: un titolo che contiene `pgpatcher` (o
`parallaxgen`, il suo vecchio nome) riceve `--ignore-mo2vfscheck`. Perché, e perché
non è opzionale, è spiegato
[più sotto](#perché-pgpatcher-ha-bisogno-di---ignore-mo2vfscheck). Anche la mod di
cattura viene dal titolo (vedi
[sopra](#mandare-loutput-di-uno-strumento-in-una-mod-sua)).

### Tre tipi di prerequisito

Il livello 1 sono le DLL incluse: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43` e
`d3dx11_43`. Eidos le porta dentro i propri binari e le copia nel prefisso
all'avvio. In un prefisso a 64 bit, l'impostazione predefinita di Proton, la build a
64 bit va in `system32` e quella a 32 bit in `syswow64`. Niente da fare, niente
rete. Ogni build è conservata compressa con gzip, quindi gli otto file occupano 7,9
MB di ciascun binario invece di 17,9 MB, e viene decompressa solo quella che viene
copiata (4,7 MB al massimo).

Il livello 2 sono gli installatori: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` e `xact_x64`. Scrivono
chiavi di registro, la GAC e gli host CLR, quindi non possono essere copiati come
file. Eidos ne installa tre senza winetricks: `vcrun2022` esegue
`VC_redist.x86.exe` e `VC_redist.x64.exe` di Microsoft, e `dotnet8` e
`dotnetdesktop8` eseguono l'installatore di .NET 8.0.12 Desktop Runtime, che
contiene anche il runtime .NET. Il pacchetto di rilascio
(`eidos-linux-bundle.tar.gz`) include questi installatori nella sua cartella
`runtimes/`, e `install.sh` li copia in `~/.local/share/Colony/Eidos/runtimes/` per
l'utente che lo esegue, anche con `--system`. Eidos cerca ogni file prima lì, poi in
una cartella `runtimes/` accanto al binario `eidos` (un pacchetto decompresso
eseguito sul posto). Senza nessuna delle due copie (il file singolo `eidos-linux`,
una build dai sorgenti, o un altro utente di un'installazione di sistema) Eidos li
scarica da Microsoft (`download.visualstudio.microsoft.com`) in
`~/.local/share/Colony/Eidos/runtimes/` e li tiene lì. In entrambi i casi un file
viene rifiutato se il suo SHA-256 non corrisponde a quello integrato in Eidos, e
viene eseguito senza finestra (`/install /quiet /norestart`) nel prefisso del
gioco, con la build di Proton o Wine che avvia gli strumenti del gioco. Come fa
winetricks, `vcrun2022` imposta prima le DLL di Visual C++ su `native,builtin`
negli override DLL del prefisso. Dopo, con una build di Proton, Eidos controlla che
il runtime C++ del prefisso non sia per metà di Microsoft e per metà di Proton.
Quando lo è, l'installazione fallisce: il messaggio nomina le coppie di DLL che
vengono da build diverse, e dice di installare invece `vcrun2022` con winetricks,
che sostituisce `msvcp140.dll` da sé. Viene installato solo il .NET 8 a 64 bit; uno
strumento .NET 8 a 32 bit richiede ancora `winetricks dotnetdesktop8`.

Gli altri sei verbi passano per il `winetricks` di sistema, che deve essere nel
`PATH`. Eidos lo esegue con il `wine` della build di Proton o Wine che avvia gli
strumenti del gioco, contro il prefisso del gioco, il che aggira il contenitore
pressure-vessel di Steam e la discordanza protontricks + Proton-GE. Alcuni verbi di
winetricks richiedono `cabextract`, ed Eidos avvisa quando manca. Uno strumento che
dichiara un verbo del livello 2 non installato parte comunque, con un avviso che
nomina il verbo e il comando per rimediare, perché potresti averlo installato in un
altro modo.

Un verbo del livello 2 conta come fatto quando il `winetricks.log` del prefisso lo
elenca, chiunque abbia eseguito winetricks o protontricks, oppure quando Eidos lo ha
installato in quello stesso prefisso. Eidos registra le proprie installazioni in
`prereqs.done` nell'istanza, legate al percorso e all'inode del prefisso, quindi un
prefisso ricostruito mostra di nuovo i suoi verbi come in sospeso. Rebuild prefix,
un'azione di Diagnostics per una copia Steam, rinomina `compatdata/<appid>` in
`<appid>.eidos-backup-<time>` e non elimina nulla; Steam crea un nuovo prefisso la
prossima volta che avvia il gioco.

Il livello 3 sono i runtime, `dotnet10` e `dotnetsdk10`. Un runtime .NET moderno è
fatto di 193 file che vivono in una directory propria e vengono trovati attraverso
`DOTNET_ROOT`. Non viene mai registrato e mai installato nel prefisso, quindi
nessuno degli altri due livelli può trasportarlo. Eidos lo tiene fuori da ogni
istanza, perché 78 MB non sono per gioco né per profilo: ogni runtime viene
decompresso in `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`
(`dotnet10-10.0.10`, `dotnetsdk10-10.0.403`). Eidos cerca lo zip del runtime con il
suo nome di download nelle stesse due cartelle degli installatori del livello 2, e
decomprime la copia che il pacchetto di rilascio include
(`dotnet-runtime-10.0.10-win-x64.zip`, solo per `dotnet10`) oppure la scarica. Il
pacchetto non include il `dotnet-sdk-10.0.403-win-x64.zip` dell'SDK, ma una copia
messa lì a mano viene usata e risparmia il download. Eidos rifiuta entrambi i file
se il loro SHA-256 non corrisponde a quello integrato in Eidos. Decomprime lo zip
con `7z`, lo stesso 7-Zip che usa per gli archivi delle mod.

`dotnetsdk10` è l'SDK di .NET 10, un download da 301 MB che occupa 808 MB una volta
decompresso. Synthesis ne ha bisogno per compilare i suoi patcher, e per costruirli
ed eseguirli chiama un `dotnet` senza percorso, quindi uno strumento che dichiara
`dotnetsdk10` riceve anche la cartella dell'SDK in `WINEPATH`, più
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false` e
`DOTNET_CLI_TELEMETRY_OPTOUT=1`. La compilazione di un patcher lascerebbe in
esecuzione nodi MSBuild e il server VBCSCompiler, che sotto Wine tengono in vita il
wineserver del prefisso, quindi lo strumento riceve anche
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` e
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` permette a un patcher
compilato per .NET 8 (Mutagen prima della 0.54) di girare sul runtime .NET 10
dell'SDK quando non si trova nessun runtime .NET 8. Una voce di Synthesis in
`tools.ini` che riporta ancora il vecchio valore predefinito, `dotnet8`,
`vcrun2022`, passa a `dotnetsdk10`, `vcrun2022` quando Eidos la legge.

NuGet verifica la firma di ogni pacchetto che ripristina rispetto all'archivio delle
radici di Windows, e a un prefisso Wine mancano le radici di firma del codice e di
marca temporale che l'SDK fornisce come
`sdk/10.0.403/trustedroots/codesignctl.pem` e `timestampctl.pem`. Senza di esse, il
ripristino di `Mutagen.Bethesda.Synthesis` 0.36.6 fallisce con `NU3018` e `NU3028`.
Per questo, prima di avviare uno strumento che dichiara `dotnetsdk10`, e una volta
scaricato l'SDK, Eidos aggiunge i 385 certificati dei due file all'archivio delle
radici dell'utente corrente nel prefisso del gioco, nella stessa importazione con
`regedit` che scrive il percorso del gioco (vedi sotto). Se uno manca, li importa di
nuovo. Da quel momento ogni programma Windows in quel prefisso si fida di queste
radici. Con esse, in un prefisso nuovo e con la cache di NuGet vuota, lo stesso
ripristino e la compilazione riescono con il Wine 11.21 di sistema e con
GE-Proton11-9.

Eidos non installa nulla dei livelli 2 o 3 finché non lo chiedi:

```sh
eidos prereqs skyrimse            # mostrare ciò di cui hanno bisogno gli strumenti registrati, e il loro stato
eidos prereqs skyrimse --install  # installare ciò che manca (prima le copie del pacchetto, poi i download)
```

Nella GUI gli stessi stati stanno sotto il campo Prereqs, e quelli mancanti sono
pulsanti. Un verbo che non è né incluso, né un runtime, né un verbo del livello 2
viene segnalato come probabile errore di battitura invece che offerto come
download.

`eidos prereqs <instance> --install` si rifiuta finché il gioco, Steam o uno
strumento tiene ancora il prefisso. I runtime del livello 3 si installano anche
prima che il prefisso esista. Gli installatori che Eidos esegue da sé richiedono un
prefisso che esista: prima che lo store abbia avviato il gioco una volta, si
fermano con "start the game once through its store, then install again". I verbi
del livello 2 vengono installati uno alla volta e ogni successo viene registrato,
quindi dopo un fallimento lo stesso comando riprende. Per una copia GOG o Epic
senza impostazioni di Heroic, `--install` copia le DLL incluse e poi si ferma prima
dei verbi del livello 2: dice di usare il winetricks di quel runner, e legge il
`winetricks.log` del prefisso al controllo successivo.

### Perché DynDOLOD ha bisogno di `dotnet10`

DynDOLOD non costruisce da sé il LOD degli oggetti: si appoggia a LODGen, e ne
distribuisce tre. `LODGenx64.exe` punta a .NET Framework 4.8, che sotto Proton viene
indirizzato al Mono di Wine, e l'inizializzatore di `System.Uri` di Mono chiama un
metodo che Mono non implementa. LODGen muore prima della sua prima riga di lavoro,
lasciando un log che contiene un'intestazione di versione e nient'altro, e una
finestra DynDOLOD che dice soltanto "failed for one or more worlds".

Installare il vero .NET Framework non lo risolve: Proton sostituisce `mscoree.dll`,
il caricatore che lo troverebbe, con un symlink dentro il proprio albero, e lo rifà
a ogni aggiornamento del prefisso.

La build che funziona è `LODGenx64Win10.exe`, che punta a .NET moderno e non tocca
mai `mscoree`. Punta `DOTNET_ROOT` a un runtime .NET 10 e gira. È quello che
`dotnet10` predispone, ed Eidos imposta la variabile quando avvia qualsiasi
strumento che lo dichiari.

### Perché Pandora ha bisogno di `dotnet10`

Pandora è un'app Avalonia, non WPF, e punta a `net10.0` dalla versione 4.1.0 di
Pandora, quindi il solo runtime .NET 10 basta a eseguirla. Quando non trova alcun
runtime dice di installare il .NET Desktop Runtime. È il testo generico che il
launcher di .NET mostra per qualsiasi programma con interfaccia grafica che non
trova un runtime, e `dotnetdesktop8` non lo risolve.

Eidos 1.21.0 e precedenti davano a Pandora `dotnetdesktop8`, e una voce conserva la
propria riga `prereqs=`. Quando Eidos legge una voce di Pandora che elenca ancora
solo `dotnetdesktop8`, e il `.runtimeconfig.json` accanto al suo eseguibile nomina
`net10.0`, la voce riceve invece `dotnet10`. Pandora 2.x punta a .NET 8 e mantiene
ciò che ha.

### Perché PGPatcher ha bisogno di `--ignore-mo2vfscheck`

PGPatcher riscrive mesh e plugin perché le mod di texture che hai (vanilla,
parallax, complex material, PBR) usino lo shader giusto su ogni superficie. Si
rifiuta di girare fuori da un gestore di mod, perché applicare patch a una cartella
di gioco nuda sarebbe distruttivo.

PGPatcher cerca usvfs, lo strato di hooking delle DLL che MO2 inietta nel gioco.
Eidos non ne ha uno: la vista unita è un mount FUSE in cui Wine entra senza sapere
che esiste. Quindi il controllo non trova nulla e lo strumento esce subito con

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

nominando un programma che l'utente non sta eseguendo, prima di qualsiasi lavoro. La
risposta del suo stesso autore alle due segnalazioni Linux di questo
([#716][pg716] e [#725][pg725], entrambe da Fluorine, il fork Linux di MO2, che usa
FUSE per la stessa ragione) è un flag che salta il controllo.

Quindi Eidos lo imposta da sé. Un PGPatcher trovato in un pool di mod arriva con
l'argomento già messo, e digitare `PGPatcher` come titolo nella finestra Executables
riempie il campo Arguments allo stesso modo. Quello che digiti vince sempre: il
precaricamento riempie solo un campo lasciato vuoto, esattamente come i prerequisiti
sopra.

L'elenco è in `default_args` (`crates/eidos-instance/src/tools.rs`), accanto a
`default_prereqs`.

A runtime non chiede nulla, cosa rara per uno strumento .NET: si porta il proprio
(`dotnetlib/`, con un `runtimeconfig.json` che dichiara `includedFrameworks`) e
consegna le DLL del CRT Microsoft accanto all'eseguibile. Lascia Prereqs vuoto.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher e PGLite in modalità Mod Organizer 2

Con Mod Manager impostato su `None`, PGPatcher vede un'unica cartella `Data` unita
e non può sapere da quale mod provengano una mesh o una texture, quindi non ha
conflitti da mostrare né un ordine delle mod da seguire. La sua modalità
`Mod Organizer 2` legge direttamente l'istanza: il `modlist.txt` del profilo, le
mod attive nel loro ordine di priorità e i file nella cartella di ogni mod. PGLite,
la versione in gioco, legge l'istanza allo stesso modo. Un'istanza Eidos ha già la
struttura di MO2 (`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), quindi
questa modalità funziona con essa.

Nel launcher di PGPatcher:

1. Imposta Mod Manager su `Mod Organizer 2`.
2. Imposta MO2 Options -> Instance Location sulla cartella dell'istanza Eidos,
   scritta come percorso Windows sull'unità `Z:` di Wine, che è `/`. Un'istanza
   portatile in `/mnt/Games/Eidos-Skyrim` diventa `Z:\mnt\Games\Eidos-Skyrim`; una
   globale diventa `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Lascia `--ignore-mo2vfscheck` negli argomenti dello strumento. In questa
   modalità PGPatcher cerca usvfs e si ferma se manca.

PGPatcher accetta la cartella solo se contiene un `modorganizer.ini`, perciò Eidos
scrive `ModOrganizer.ini` nella radice dell'istanza ogni volta che avvia il gioco o
uno strumento, per il profilo avviato. Il file viene riscritto solo quando il suo
contenuto cambia. Contiene:

- in `[General]`: `gameName` (il nome del gioco in MO2, quindi `New Vegas` per
  Fallout: New Vegas), `gamePath` (la cartella d'installazione del gioco nello
  store come percorso `Z:`, anche in un'istanza che parte da una copia del gioco) e
  `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` e `overwrite_directory`
  come percorsi `Z:` assoluti, più `download_directory` quando l'istanza ha una
  cartella `downloads/`.

La sua prima riga è un commento che dice che l'ha scritto Eidos. Un
`ModOrganizer.ini` senza quella riga, con qualsiasi grafia del nome, non viene mai
modificato né cancellato: un'istanza MO2 che hai aperto in Eidos conserva il
proprio, e PGPatcher legge quel file così com'è. MO2 elimina i commenti quando
salva le sue impostazioni, quindi un file su cui MO2 ha salvato è di MO2 da quel
momento. Un file vuoto conta come di Eidos.

Un file del genere nomina il profilo che MO2 ha selezionato per ultimo, e PGPatcher
usa quel profilo, non quello che avvia Eidos. Quando i due differiscono, Eidos
registra un avviso all'avvio. Seleziona lo stesso profilo in MO2, oppure rinomina o
rimuovi il file perché Eidos scriva il proprio.

Gli strumenti in modalità Mod Organizer 2 leggono il gioco anche dal `gamePath` di
quel file. Quando non nomina né la cartella del gioco da cui parte l'istanza né
l'installazione dello store, la scheda Diagnostics mostra "ModOrganizer.ini points
tools at another game folder", con quel percorso e la cartella del gioco che questa
istanza usa. Cambia `gamePath` nel file, oppure rimuovi il file perché Eidos scriva
il proprio.

Eidos scrive il suo file solo quando avvia qualcosa, quindi tra un avvio e l'altro
nomina il profilo dell'ultimo. Uno strumento avviato da Eidos vi trova il proprio
profilo.

PGPatcher si ferma con un errore quando il profilo non ha alcuna mod attiva e
quando la sua cartella di output è la cartella di una mod attiva, come fa sotto
MO2.

## I programmi di Wine nel prefisso del gioco

Il menu File avvia quattro programmi di Wine nel prefisso del gioco: winecfg,
regedit, il gestore file di Wine e il programma di disinstallazione. Partono come
qualsiasi strumento, con la build di Proton o Wine che esegue gli strumenti del
gioco, con le mod montate, e la finestra li aspetta. Prefix drive_c apre l'unità
`C:` del prefisso nel gestore file. Una riga resta grigia finché il prefisso non ha
quel programma, cosa che accade la prima volta che lo store avvia il gioco. Da un
terminale, `eidos tool <instance> run winecfg` fa lo stesso, e così `regedit`,
`explorer` e `uninstaller`, quando nessuno strumento nell'elenco ha quel titolo.

Una modifica fatta in regedit all'`installed path` del gioco viene rimessa a posto
alla successiva esecuzione di uno strumento; vedi la sezione successiva.

## Il percorso del gioco nel prefisso

Gli strumenti Windows trovano il proprio gioco leggendo
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, una chiave che scrive
l'installer del gioco stesso. Steam sotto Proton non esegue mai quell'installer.
Senza la chiave xEdit, Wrye Bash e DynDOLOD si aprono su un percorso vuoto.

Prima di un'esecuzione di uno strumento, Eidos scrive la chiave con
un'importazione `regedit` nel prefisso del gioco, a meno che un'importazione
precedente non abbia già scritto lo stesso percorso e il prefisso lo contenga
ancora. La chiave nomina l'installazione dello store, anche in un'istanza che parte
da una copia del gioco, perché la vista unita è montata lì. Per uno strumento che
dichiara `dotnetsdk10`, la stessa importazione aggiunge le radici di firma di NuGet
(vedi [sopra](#tre-tipi-di-prerequisito)). Uno strumento `runner=native` salta
questo passaggio. Quando il prefisso non esiste ancora, o un altro programma lo
tiene, lo strumento parte comunque, ed Eidos stampa un avviso che nomina lo store
da cui avviare il gioco una volta, o il programma da chiudere. Quando la chiave
nomina una cartella diversa dall'installazione dello store, la scheda Diagnostics
mostra "Windows tools cannot find <game>" finché la successiva esecuzione di uno
strumento non la riscrive.

## Raggiungere uno strumento: nascondere, fissare e una scorciatoia sul desktop

I valori predefiniti di un gioco includono strumenti che forse non userai mai.
Nella finestra Executables:

- Pin to top mette una voce in cima all'elenco Run.
- Hide from picker ne toglie una senza cancellarla.
- Desktop shortcut scrive un `.desktop` in `~/.local/share/applications`, così
  spunta nel menu delle applicazioni e in una ricerca, non sul desktop. Esegue
  direttamente `eidos tool <instance> run <title>`, quindi lo strumento si apre
  attraverso la vista unita con il profilo di questa istanza senza che la finestra
  di Eidos sia aperta.

Nascondere e fissare cambiano solo il modo in cui uno strumento viene raggiunto,
quindi valgono tanto per i valori predefiniti per gioco quanto per le tue voci.

Il loader dello script extender, il lanciatore e l'eseguibile del gioco non hanno
il pulsante Desktop shortcut. Avviano il gioco, e il gioco si avvia sempre tramite
il suo store, cosa che fa già il collegamento dello store stesso. Un collegamento
creato per uno di loro con Eidos 1.22 o precedente continua a funzionare: come
`eidos tool <instance> run <title>` digitato in un terminale, chiede allo store di
avviare il gioco, come fa Run in una finestra che nessuno store ha aperto. Quando
manca l'impostazione dello store, il comando non avvia nulla e stampa cosa
impostare. Un collegamento non mostra alcun messaggio, quindi esegui lo stesso
comando in un terminale per leggerlo.

Eidos passa allo store qualsiasi voce in questo modo quando il suo eseguibile ha il
nome di file del loader dello script extender, del lanciatore o del binario del
gioco, senza distinzione tra maiuscole e minuscole: una voce predefinita, una che
hai aggiunto, una importata da MO2, o un'esecuzione `--exe`. Prima segna questa
istanza come l'ultima usata, così la finestra che lo store apre mostra questa
istanza, poi esegue `steam -applaunch <id>`,
`xdg-open heroic://launch?appName=<app>&runner=<runner>` per Heroic, oppure
`legendary launch <app>`. Con `--print` stampa quel comando dello store e non avvia
nulla. Il comando `steam` deve essere nel `PATH`: Steam Flatpak non ce lo mette,
quindi l'avvio fallisce e il messaggio dice di avviare il gioco da Steam.

La voce gira direttamente attraverso la vista unita, come qualsiasi strumento,
quando uno store ha avviato il processo: una tra `SteamGameId`, `SteamAppId`,
`HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` o `EIDOS_STORE_STARTED` è impostata. Senza
nessuna di esse, Eidos non avvia il gioco con Proton da sé. Quando l'istanza non ha
alcun loader dello script extender (nella cartella del gioco da cui parte, nel
`Root/` di una mod attiva o in `Overwrite/Root/`), non parte nulla e la riga dice di
installare prima lo script extender. Per una copia GOG fuori da Heroic, o un id
dello store che Eidos non sa passare, dice di avviare il gioco da quello store.

Un profilo può avere un suo collegamento per il gioco. Fai clic destro sul profilo
e scegli Desktop shortcut: la voce esegue
`eidos tool <instance> run "<game>" --profile <name>`, che rende attivo quel
profilo, come quando lo scegli nella finestra, e poi chiede allo store di avviare
il gioco. La finestra di Eidos che lo store apre mostra quel profilo. `--profile`
funziona con qualsiasi `eidos tool ... run` e seleziona il profilo prima che lo
strumento parta; con `--print` non seleziona niente.

## Uno strumento che è un'app Steam a sé

Il Creation Kit è un'applicazione Steam separata e vuole il proprio AppID; qualche
altro strumento di modding distribuito su Steam è uguale. Imposta Steam AppID sulla
voce ed Eidos lo avvia sotto quell'id invece che sotto quello del gioco.

Su Windows questo significa un lanciatore diverso. Qui sono due variabili d'ambiente
sulla stessa esecuzione, `SteamAppId` e `SteamGameId`. Eidos le imposta entrambe,
perché Proton legge l'una e le librerie di Steam leggono l'altra, e uno strumento che
le vede in disaccordo fallisce senza un errore chiaro. `eidos tool ... --print`
mostra esattamente ciò che riceverebbe l'esecuzione vera.

## Variabili d'ambiente e strumenti avviati senza Proton

Tre chiavi di `<instance>/tools.ini` non hanno ancora un campo nella finestra
Executables, quindi aggiungile a mano nella sezione dello strumento. Ogni riga
`env=NAME=value` imposta una variabile solo per le esecuzioni di quello strumento:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos applica queste righe dopo tutto ciò che imposta da sé e dopo le variabili del
`launch-env.ini` del profilo, quindi un valore messo qui sostituisce entrambi. Vale
anche per il `WINEDLLOVERRIDES` che Eidos costruisce dalle DLL fornite dalle tue
mod: uno strumento che lo imposta deve elencarvi ogni DLL di cui ha bisogno. I
wrapper del profilo avvolgono il comando di uno strumento come avvolgono quello del
gioco; vedi
[Variabili e wrapper per profilo](usage.md#variabili-e-wrapper-per-profilo). Un nome
è fatto di lettere, cifre e `_`, e non comincia con una cifra. Una riga con
qualsiasi altro nome viene ignorata.

`runner=native` avvia `exe` direttamente, senza Proton. La vista unita viene
montata come per ogni strumento, e le righe `arg0=`, `arg1=` sono i suoi argomenti.
Usalo per un programma Linux, o per uno strumento su una copia che Steam non gestisce
e per cui Heroic non registra alcun runner: `exe` è il Wine di quella copia, il
programma Windows è il primo argomento e il prefisso arriva da `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` deve essere un percorso assoluto. La cartella di lavoro predefinita è quella
che contiene `exe`, qui quella del runner, quindi imposta `workdir=`. Solo `exe`
viene riscritto nel suo percorso unito: per un programma Windows dentro una mod,
indica il suo percorso sotto la cartella `Data` del gioco, dove la vista unita lo
mostra. Uno strumento nativo salta i passaggi di Proton: la chiave di registro
descritta sopra, la cache degli shader di Steam, le radici .NET vuote e la
rimozione dell'overlay di Steam da `LD_PRELOAD`.

Su una copia GOG o Epic installata con Heroic, gli strumenti non hanno bisogno di
questa voce se le impostazioni Heroic del gioco (`GamesConfig/<appName>.json`)
indicano una build di Wine o Proton: Eidos li avvia con quella build nel prefisso
Heroic del gioco, e `eidos prereqs <instance> --install` la usa per gli installatori
e per winetricks. Una copia installata solo con Legendary non ha impostazioni
Heroic.

`slr=1` avvia lo strumento, e l'importazione nel registro che lo precede, dentro il
contenitore Steam Linux Runtime indicato dalla build di Proton dello strumento
(`require_tool_appid` nel suo `toolmanifest.vdf`), come Steam avvia il gioco. Quel
runtime deve essere installato in una libreria di Steam; altrimenti l'esecuzione si
ferma e indica l'app di Steam da installare. `eidos prereqs` continua a eseguire i
suoi installatori e winetricks sull'host, e uno strumento con `runner=native`
ignora la chiave. Il contenitore vede la tua cartella home e la libreria di Steam
del gioco. Uno strumento, una mod o un runtime altrove, per esempio su un altro
disco, richiede `env=STEAM_COMPAT_MOUNTS=/quel/percorso` (più percorsi separati da
`:`). Un Proton installato in `/usr/share/steam/compatibilitytools.d` probabilmente
non è utilizzabile con `slr=1`, perché `/usr` dentro il contenitore è quello del
runtime. Nessuna esecuzione di uno strumento nel contenitore è stata ancora provata.

`eidos tool <instance> run <title> --print` mostra il comando finale e ogni
variabile che l'esecuzione imposta sopra l'ambiente ereditato, comprese queste
righe. Per una voce che avvia il gioco stampa invece il comando dello store (vedi
[Raggiungere uno strumento](#raggiungere-uno-strumento-nascondere-fissare-e-una-scorciatoia-sul-desktop)).

## BethINI Pie

Eidos non include BethINI Pie. Metti la versione scaricata dalla sua pagina Nexus
in `mods/` di questa istanza o nella tua cartella degli strumenti, e `Bethini.exe`
compare nell'elenco degli strumenti, sugli stessi giochi di PGPatcher. Avviato
tramite Eidos, modifica la copia degli INI del gioco che appartiene al profilo.
Prima di ogni avvio di uno strumento il cui titolo contiene `bethini`, Eidos
conserva un punto di ripristino di quegli INI; Backups nella barra degli strumenti,
poi INIs, ne rimette uno. Un INI che non esisteva al momento della copia resta
com'è. La finestra tiene i 10 punti di ripristino più recenti per ogni tipo, e
ripristinarne uno fa prima una copia degli INI così come sono, quindi una scelta
sbagliata si può annullare.

## Le impostazioni di uno strumento restano affar suo

Eidos non controlla la maggior parte delle impostazioni degli strumenti; una
impostazione sbagliata di solito fallisce senza errore.

Il Game Data Path di BodySlide (Settings) deve puntare alla directory `Data` del
gioco, non alla cartella del gioco che sta sopra. Impostato un livello troppo in
alto, una build in blocco riporta "All sets processed successfully" e scrive 1439
mesh dove il gioco non le cercherà mai. Eidos le intercetta in `Overwrite/Root/`,
quindi la tua installazione del gioco resta intatta, e l'unico segno nel gioco è
che i tuoi corpi non sono costruiti.

Prima di un'esecuzione di BodySlide o di Outfit Studio, la riga di stato avvisa
quando `GameDataPath` o `OutputDataPath` nel `CalienteTools/BodySlide/Config.xml`
vincente non porta alla cartella `Data` del gioco durante l'esecuzione, e la scheda
Diagnostics mostra lo stesso avviso come scheda con il valore da impostare.
Anche `eidos check <instance>` lo segnala. Un percorso su `S:` viene letto come
Eidos mappa `S:` per uno strumento, cioè sulla cartella della libreria di Steam. Un
`OutputDataPath` dentro la cartella `mods/` dell'istanza, per una mod di output, è
accettato. Diagnostics mostra anche una scheda quando `Overwrite/Root/` contiene una
cartella `meshes` o `textures`.

L'output degli strumenti sta di casa nell'Overwrite. Quando un'esecuzione produce
qualcosa che vale la pena tenere, Overwrite -> Create mod... lo trasforma in una
mod ordinaria che può essere ordinata, disattivata e rimossa come qualsiasi altra.
