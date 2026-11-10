<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Estensioni

Un'estensione aggiunge una voce a Eidos senza far parte di Eidos. È un manifesto
TOML che nomina un programma e, al massimo, quel programma.

I manifesti vivono in `~/.config/Colony/Eidos/addons/`, un `.toml` per estensione.
Apri la cartella da View -> Extensions -> Open folder e premi Reload - niente
riavvio.

`id`, `kind` ed `exec` sono obbligatori; `name` vale come predefinito l'id.
`exec` indica il programma. Un nome senza `/` viene cercato in `PATH`. Se il
programma manca, Eidos salta l'estensione e la sua riga in View -> Extensions
dice perché (`<exec> is not on PATH` oppure `<exec> is not there`). `games`
limita l'estensione agli id di gioco indicati; senza `games`, l'estensione vale
per ogni gioco, e su un altro gioco la sua riga dice "not for this game".

Un manifesto che Eidos rifiuta compare per primo, con il motivo: un file che non
riesce a leggere, un TOML non valido o un campo obbligatorio mancante, un `id`
vuoto o con uno spazio, un `exec` vuoto, un `id` già preso da un manifesto
precedente (i file vengono letti in ordine di nome), un `kind` che Eidos non
conosce, oppure un tipo installer, preview o save-info senza `protocol = 1` e
nome file `extensions`.

## Perché non viene caricato nulla dentro Eidos

Mod Organizer 2 carica i plugin come librerie condivise e ospita quelli in Python
tramite Qt. Nessuna delle due cose si trasferisce. Rust non ha un'ABI stabile,
quindi una libreria condivisa costruita con un compilatore diverso - o con un
diverso flag di ottimizzazione, o con un diverso insieme di funzionalità di una
dipendenza comune - è comportamento indefinito, e nulla lo segnala come discrepanza di versione. E
i widget di Eidos sono generici a tempo di compilazione, per cui una libreria non
potrebbe costruirne uno da restituire nemmeno se l'ABI fosse stabile.

Un'estensione è un programma che Eidos esegue con i diritti di accesso al file system del tuo account. Installa solo programmi ausiliari di cui ti fidi. L'isolamento dei processi e la verifica delle risposte non costituiscono una sandbox del sistema operativo.

## Uno strumento

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # ometti per ogni gioco
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Compare in View -> Extensions con un pulsante Run e parte staccato - Eidos non lo
aspetta, e la riga di stato ne indica il pid. Il suo output viene scartato. Senza
`workdir` parte dalla cartella da cui è stato avviato Eidos.

## Un controllo

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Viene eseguito a ogni aggiornamento, nella radice dell'istanza se `workdir` non
dice altro, e stampa un riscontro per riga:

```
level<TAB>title<TAB>detail
```

dove `level` è `problem`, `advice` oppure `ok`. Il dettaglio è facoltativo; una
riga senza titolo viene scartata. Tutto ciò che non inizia con un livello noto
viene ignorato, così l'output di avanzamento e gli avvisi sparsi non possono
generare una riga che sembri uno dei controlli propri di Eidos. I riscontri
compaiono nella scheda Diagnostics come `<name> - <title>`, dopo le schede del
prefisso e di Nexus e prima delle altre schede Health.

Un controllo ha tre secondi. Quello che sfora viene fermato e segnalato come un
problema contro sé stesso - gira allo stesso aggiornamento che segue ogni clic,
quindi uno bloccato congelerebbe la finestra. Il tempo corre finché il controllo
non è terminato e le sue pipe di output non sono chiuse, quindi un processo figlio
in background che le tiene aperte conta nei tre secondi. Un controllo che termina
con un errore senza stampare nulla viene segnalato allo stesso modo, con la prima
riga del suo output di errore, e lo stesso vale per uno che stampa più di 1 MiB su
uno dei due flussi. In entrambi i casi Eidos non lo esegue di nuovo finché non
premi Reload, e la sua riga dice "It will not be retried until you press Reload."
Un controllo non legge nulla dal suo standard input.

## Segnaposto

Sia `args` sia `workdir` espandono questi:

| Segnaposto      | Che cos'è                                    |
| --------------- | -------------------------------------------- |
| `{instance}`    | la radice dell'istanza                       |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | il nome del profilo attivo                   |
| `{profile_dir}` | la cartella del profilo attivo               |
| `{game}`        | l'identificatore del gioco, es. `skyrimse`   |
| `{game_name}`   | il nome visualizzato del gioco               |
| `{install}`     | la cartella del gioco che l'istanza usa      |
| `{data}`        | la cartella `Data` di quella cartella        |

`{install}` è l'installazione dello store, oppure la copia del gioco quando
l'istanza ne usa una (uno `Stock Game` congelato o la cartella di gioco di una
lista; vedi [usage.md](usage.md#file-di-gioco-linstallazione-dello-store-o-una-copia)). `{data}`
la segue.

Un controllo riceve anche `{addon_dir}`, la cartella che contiene il suo
manifesto, e lo stesso vale per le estensioni del protocollo 1 qui sotto. Uno
strumento no.

Un segnaposto sconosciuto non viene mai svuotato, perché trasformerebbe
`--out {typo}` in `--out --next-flag`. Eidos rifiuta di eseguire uno strumento o
un controllo i cui segnaposto non si risolvono tutti, e dice quali mancano: nella
riga di stato per uno strumento, nella riga del controllo nella scheda
Diagnostics per un controllo.

## Cosa un'estensione non può fare

Un'estensione non può richiamare Eidos, registrarsi da un archivio o disegnare i propri widget. Gli installatori strutturati del protocollo 1 possono restituire piani dei file verificati e richieste di scelta; Eidos gestisce la revisione, la preparazione temporanea e la pubblicazione. FOMOD e BAIN restano installatori nativi. Le estensioni di anteprima e informazioni sui salvataggi usano lo stesso host con risposte limitate. La [specifica del protocollo e gli esempi eseguibili](../../../../guide/examples/extensions/README.md) descrivono la priorità delle corrispondenze, le risposte CLI registrate, i limiti e i controlli d'identità durante la ripetizione.
