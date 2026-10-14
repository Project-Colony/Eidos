<!-- eidos-i18n: source=docs/guide/extensions.md sha=5f31f1cbc3dcbfb9655f3570f01e5ada1377268d -->

# Estensioni

Un'estensione aggiunge una voce a Eidos senza far parte di Eidos. È un manifesto
TOML che nomina un programma e, al massimo, quel programma.

I manifesti vivono in `~/.config/Colony/Eidos/addons/`, un `.toml` per estensione.
Apri la cartella da **View -> Extensions -> Open folder** e premi **Reload** -
niente riavvio.

## Perché non viene caricato nulla dentro Eidos

Mod Organizer 2 carica i plugin come librerie condivise e ospita quelli in Python
tramite Qt. Nessuna delle due cose si trasferisce. Rust non ha un'ABI stabile,
quindi una libreria condivisa costruita con un compilatore diverso - o con un
diverso flag di ottimizzazione, o con un diverso insieme di funzionalità di una
dipendenza comune - è comportamento indefinito, non una discrepanza di versione. E
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

Compare in **View -> Extensions** con un pulsante Run e parte staccato - Eidos non
lo aspetta.

## Un controllo

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Viene eseguito a ogni aggiornamento e stampa un riscontro per riga:

```
level<TAB>title<TAB>detail
```

dove `level` è `problem`, `advice` oppure `ok`. Il dettaglio è facoltativo. Tutto
ciò che non inizia con un livello noto viene ignorato, così l'output di
avanzamento e gli avvisi sparsi non possono generare una riga che sembri uno dei
controlli propri di Eidos. I riscontri compaiono nella scheda **Health**,
preceduti dal nome dell'estensione.

Un controllo ha tre secondi. Quello che sfora viene fermato e segnalato come un
problema contro sé stesso - gira allo stesso aggiornamento che segue ogni clic,
quindi uno bloccato congelerebbe la finestra.

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
| `{install}`     | la cartella d'installazione del gioco        |
| `{data}`        | la cartella `Data` del gioco                 |

Un segnaposto sconosciuto resta esattamente com'è scritto anziché essere
svuotato, così un errore fallisce in modo visibile invece di trasformare
`--out {typo}` in `--out --next-flag`. Eseguire uno strumento i cui segnaposto non
si risolvono tutti viene rifiutato, ed Eidos dice quali mancano.

## Cosa un'estensione non può fare

Un'estensione non può richiamare Eidos, registrarsi da un archivio o disegnare i propri widget. Gli installatori strutturati del protocollo 1 possono restituire piani dei file verificati e richieste di scelta; Eidos gestisce la revisione, la preparazione temporanea e la pubblicazione. FOMOD e BAIN restano installatori nativi. Le estensioni di anteprima e informazioni sui salvataggi usano lo stesso host con risposte limitate. La [specifica del protocollo e gli esempi eseguibili](../../../../examples/extensions/README.md) descrivono la priorità delle corrispondenze, le risposte CLI registrate, i limiti e i controlli d'identità durante la ripetizione.
