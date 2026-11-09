<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=8f0ea64ea39bdda653b7726249aa2b3377b772dc -->

# Fehlersuche und Diagnose

Alles für den Tag, an dem das Spiel etwas sieht, dem das Dateisystem nicht
zustimmt: die Umgebungsschalter, das Lesen der Operationszähler, die bekannten
Probleme samt ihrer Geschichte und die Passthrough-Geschichte.

### Das VFS diagnostizieren

Für den Fall, dass das Spiel etwas sieht, dem das Dateisystem nicht zustimmt, gibt
es zwei Umgebungsvariablen:

```sh
EIDOS_FUSE_STATS=1                  # Operationszähler, beim Aushängen ausgegeben
EIDOS_FUSE_NO_CACHE=1               # jeder kernelseitige Cache aus
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # oder einzeln benennen
```

Die feingranulare Form hat den weiter unten beschriebenen Absturz gefunden: alle
vier abzuschalten beantwortet "liegt es am Caching?", und erst das Benennen
beantwortet "an welchem". Die Zähler beantworten die andere Hälfte - ein Ladevorgang
mit `read 0` ist einer, in dem `FUSE_PASSTHROUGH` jedes Byte im Kernel geliefert
hat, also ist alles, was Sie am Lesepfad optimieren wollten, bereits umsonst.

## Eine Union von Hand einhängen

Die erste `--layer` gewinnt beim Konflikt; die letzte sind Ihre unberührten
Spieldaten. Das Einhängen braucht nur `/dev/fuse` und `fusermount3` (kein
overlayfs, kein Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... über /mnt/point lesen und schreiben ...
fusermount3 -u /mnt/point
```

Schreibvorgänge landen in `--overwrite <dir>` (ohne Angabe ein temporäres
Verzeichnis), sodass die Schichten selbst auch hier unberührt bleiben.

#### Warum Passthrough standardmäßig aus ist

Passthrough übergibt dem Kernel die echte Hintergrunddatei, sodass Lesevorgänge
diesen Daemon vollständig umgehen. Das A/B unten wurde mit einer früheren
Version des Daemons gemessen, vor der Änderung, die weiter unten im bekannten
Problem „FUSE-Passthrough verhindert, dass das Spiel Mod-Inhalte lädt“
beschrieben ist. Diese Änderung vermeidet die wahrscheinlichste Ursache, wurde
aber noch nicht erneut in einem Spiel gemessen, deshalb bleibt Passthrough
standardmäßig aus, bis das geschehen ist. A/B gemessen auf Skyrim SE 1.6.1170,
proton-cachyos 11.0, Kernel 7.1.4, dieselbe Ladereihenfolge mit 82 Plugins,
einzige Variable war, ob das Binary die Capability trug:

| Passthrough | `NtCreateFile`-Fehlschläge mit `STATUS_ACCESS_VIOLATION` |
|-------------|----------------------------------------------------------|
| an          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`          |
| aus         | 0                                                        |

Mit eingeschaltetem Passthrough öffnete das Spiel keines seiner eigenen Archive
oder Plugins, was sich im Spiel als nicht vorhandene Mods zeigte - kein Fehler,
keine Logzeile. Ausgeschaltet erreichte dieselbe Ladereihenfolge das
Spielgeschehen mit lebendigen Plugins, Archiven und Papyrus-Skripten.

Der Fehlschlag war von innerhalb des Daemons unsichtbar, was ihn teuer zu finden
machte: sein eigenes `open` gelang jedes Mal, und der Kernel wies nie eine
Hintergrunddatei ab (über eine ganze fehlschlagende Sitzung mit
`EIDOS_FUSE_TRACE=open` geprüft: null `open FAILED`, null
`passthrough refused`). Der Fehler entstand, nachdem der Daemon
`opened_passthrough` geantwortet hatte, also konnte ihn keine daemonseitige
Protokollierung sehen. Er war auch nicht endungsspezifisch - er traf Archive und
Plugins gleichermaßen, also die Dateien, die das Spiel über seine gesamte
Laufzeit offen hält.

`EIDOS_FUSE_PASSTHROUGH=1` schaltet es wieder ein, um zu messen, was es bringt, oder
um den Mechanismus erneut zu prüfen. Die Capability-Warnungen im Starter und im
Diagnostics-Reiter erscheinen nur, wenn Sie danach gefragt haben.

Um das Spiel selbst durch Eidos zu starten, setzen Sie seine Steam-Startoption auf:

```
eidos play skyrimse -- %command%
```

Ein vorangestelltes `WINEDLLOVERRIDES` ist nicht nötig. Wenn eine aktivierte Mod
`d3dcompiler_47` importiert, legt Eidos seine mitgelieferte Kopie ins Präfix,
erzwingt sie als native und führt das mit allen DLL-Overrides zusammen, die eine
Mod mitbringt (ENB/ReShade/`.asi`-Loader).

### Wird der Schichtindex tatsächlich benutzt?

`LayerStack::new` baut entweder eine vollständige Karte der Nur-Lese-Schichten
oder gar keine. Ohne Karte durchläuft jede Abfrage die Schichten wie vorher: Die
Antworten sind dieselben, ein Spielstand lädt etwa dreimal so lange, und sonst
sieht nichts anders aus.

Ist der Index aus, nennt der Mount den Grund, einmal, beim Start:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

Der Grund nennt ein unlesbares Verzeichnis, eine Schicht mit mehr als 64
Verzeichnisebenen, mehr als 4.000.000 Einträge oder `EIDOS_NO_INDEX`. Die Zeile
landet im Protokoll des Laufs: im `logs/run-*.log` der Instanz bei einem Start aus
dem Fenster, in der Steam-Konsole bei einem Start über Steam. Mit
`EIDOS_FUSE_STATS=1` enthält die Ausgabe beim Aushängen außerdem eine Zeile
`layer index:`, und ein `unindexed`-Zähler ungleich null bei einem normalen Mount
heißt, dass der Index aus war.

Die Beispiele unten prüfen dasselbe offline, an den Ordnern einer Instanz, ohne
Mount:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` löst echte Pfade mit und ohne Index auf und vergleicht die
Verzeichnis-Scans. `index_agrees` prüft, ob beide DASSELBE antworten, auf jedem Pfad
und jeder Auflistung einer echten Instanz. `listing_cost` misst, was die Karte der
zusammengeführten Kinder bei `readdir` einspart.

`EIDOS_NO_INDEX=1` erzwingt den Durchlauf, für den Fall, dass gerade der Unterschied
zwischen beiden Antworten das Untersuchte ist.

## Bekannte Probleme

### DLSS oder Frame Generation tut stillschweigend nichts

Drei getrennte Ursachen, jede ohne jede Fehlermeldung: NVAPI nicht in den
Startoptionen aktiviert, exklusiver Vollbildmodus, oder ein veraltetes
Reflex-FPS-Limit. Die ganze Prüfliste steht in [graphics.de.md](graphics.md).

**Eine Mod, die ein Verzeichnis auf zwei Arten schreibt, verlor alles unter der
zweiten.** Behoben. ext4 hält `meshes/` und `Meshes/` auseinander; die
zusammengeführte Sicht darf das nicht, und echte Mods liefern beides - XP32 Maximum
Skeleton hat seine Animationen und seine FNIS-Verhaltensdatei unter der großen
Schreibweise, seine `character assets` unter der anderen.

Der Resolver nahm für jede Pfadkomponente die exakte Groß-/Kleinschreibung und legte
sich darauf fest: er betrat `meshes/`, fand den Rest des Pfades dort nicht und gab
DIE GANZE SCHICHT auf. Jede Datei unter der anderen Schreibweise war für das Spiel
unsichtbar - kein Fehler, kein Log, nichts in irgendeiner Diagnose. Auf einer echten
Instanz mit 50 Schichten waren das 74 Dateien.

Eine passende Komponente ist jetzt ein Kandidat, keine Entscheidung; die exakte
Schreibweise wird weiterhin zuerst versucht, und nur wenn der Rest darunter
fehlschlägt, sucht der Scan nach faltungsgleichen Geschwistern. Auflistungen hatten
denselben Fehler ein Verzeichnis höher und lesen nun je Schicht jedes
faltungsgleiche Verzeichnis.

**DynDOLODs LODGen stirbt und hinterlässt ein leeres Log.** Behoben durch
`dotnet10`; siehe [tools.md](tools.md). Das Symptom ist unverkennbar:
`LODGen_SSE_<world>_log.txt` mit einem Versionsbanner, einer Zeile `.NET Version:`
und sonst nichts, für jede Welt, sowie ein Dialog, der nur "failed to generate
object LOD for one or more worlds" sagt. Ursache ist Wines Mono, das für .NET
Framework antwortet, und keine Menge installierter .NET Frameworks behebt es -
Proton ersetzt `mscoree.dll` bei jeder Prefix-Aktualisierung durch einen Symlink in
seinen eigenen Baum.

**Wine konnte nicht erkennen, dass das Mount Groß-/Kleinschreibung faltet.**
Behoben, und das war der, auf den es ankam.

Wines `get_dir_case_sensitivity` fragt das Verzeichnis zuerst per
`FS_IOC_GETFLAGS`, dann schnüffelt es nach dem Marker, den CIOPFS in den von ihm
bedienten Verzeichnissen hinterlässt. Fehlt beides, nimmt Wine case-SENSITIV an,
und jede Suche, deren Schreibweise nicht Byte für Byte passt, fällt darauf
zurück, das GANZE Verzeichnis zu lesen, um eine Übereinstimmung ohne Rücksicht
auf Groß- und Kleinschreibung zu finden. Bethesda-Spiele fragen nach
`data/ccbgssse001-fish.bsa`, während die Datei `ccBGSSSE001-Fish.bsa` heißt,
also feuerte das bei nahezu jedem Asset: 4471 Marker-Abfragen und 2236
vollständige Verzeichnis-Neulesungen in acht Sekunden, und 195796 Aufzählungen
von `Data` in neunzig. Skyrim SE erreichte nie sein Hauptmenü - es saß bei
240 MB resident, während der Daemon 92 % eines Kerns verbrannte.

Eidos faltete Groß- und Kleinschreibung in `resolve_read` von Anfang an. Die
ganzen Kosten entstanden nur daraus, es nie zu sagen. `lookup` antwortet nun
`.ciopfs`; `readdir` listet es weiterhin nicht. `ioctl` beantwortet außerdem
`FS_IOC_GETFLAGS` auf Verzeichnissen mit dem Casefold-Flag, das aktuelles Wine
vor der Suche nach dem Marker abfragt.

Zwei Dinge machten es tödlich statt bloß langsam. Die Kosten skalieren mit der
Verzeichnisgröße, also brachte das Installieren der Anniversary-Inhalte (`Data` von
37 auf 177 Dateien) das Fass zum Überlaufen. Und `opendir` baute die
zusammengeführte Auflistung eifrig auf, was reine Verschwendung ist, wenn Wine ein
Verzeichnis nur öffnet, um darin diesen Marker zu `stat`en - der Schnappschuss wird
jetzt beim ersten `readdir` genommen.

Danach: das Hauptmenü, 2,1 GB resident, Daemon bei 0 % CPU.

`EIDOS_FUSE_TRACE=opendir` hat es gefunden und wird mitgeliefert. Die
Operationszähler sagen, wie viele; 195796 Aufzählungen eines Verzeichnisses sind in
einer Summe unsichtbar.

**Dass das Spiel `plugins.txt` leer überschrieb**, war sehr wahrscheinlich dasselbe -
ein `Data`, das es in keiner vernünftigen Zeit aufzählen konnte, woraus es schloss,
dort sei nichts, und das speicherte. Nicht bewiesen und eine erneute Prüfung wert.
So oder so bedeutet die Capture-Sicherung (eine Erfassung, die den aktiven Satz
vollständig leert, wird in jeder Größe abgelehnt), dass es das Profil nicht mehr
beschädigen kann.

**`FOPEN_KEEP_CACHE` ist aus.** Behoben, und es lohnt zu wissen, warum. Es ließ
Skyrim SE Sekunden nach dem Hauptmenü deterministisch an einer
Null-Dereferenzierung abstürzen, ohne einen einzigen installierten Mod; die
anderen drei kernelseitigen Caches wurden einzeln herausbisektiert, und nur
dieser war von Bedeutung. Diese Bisektion lief mit eingeschaltetem
`FUSE_PASSTHROUGH`, und der Kernel lässt ein Passthrough-Öffnen, das
`FOPEN_KEEP_CACHE` trägt, mit EIO scheitern, also scheiterte mit dem Flag jedes
Passthrough-Öffnen. Vielleicht waren es diese gescheiterten Öffnungen, die das
Spiel abstürzen ließen, und das Flag allein, mit ausgeschaltetem Passthrough,
wurde nie getestet. Passthrough-Antworten tragen es nicht mehr. Sein Verlust
wurde damals als kostenlos gemessen, aber mit eingeschaltetem Passthrough
bedient der Daemon *null* Lesevorgänge (`EIDOS_FUSE_STATS` meldete `read 0` für
einen vollständigen Ladevorgang), also sind die echten Kosten bei standardmäßig
ausgeschaltetem Passthrough (siehe unten) ungemessen. Mit
`EIDOS_FUSE_KEEP_CACHE=1` wieder einschalten, um es für sich zu testen.

### FUSE-Passthrough verhindert, dass das Spiel Mod-Inhalte lädt

Der Daemon vermeidet jetzt die wahrscheinlichste Ursache (unten), aber das wurde
noch nicht erneut in einem Spiel gemessen, deshalb bleibt Passthrough
standardmäßig aus; `EIDOS_FUSE_PASSTHROUGH=1` schaltet es ein. Vor der Änderung
scheiterte Skyrim SE mit eingeschaltetem Passthrough daran, 152 seiner eigenen
Dateien zu öffnen (75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`), mit
`STATUS_ACCESS_VIOLATION`, gegenüber 0 ohne, auf Kernel 7.1.4 - also lud
stillschweigend kein Mod-Inhalt. Der Kernel meldete den Fehler, nachdem der
Daemon `opened_passthrough` geantwortet hatte, sodass die Logs des Daemons einen
sauberen Lauf zeigten (null fehlgeschlagene Öffnungen, null abgewiesene
Hintergrunddateien).

Der Kernel bindet eine einzige Hintergrunddatei an jeden Inode
(`fs/fuse/iomode.c`). Ein Öffnen, das für einen Inode, der schon eine hat, eine
zweite Hintergrunddatei-ID mitbringt, scheitert mit EIO, ebenso ein normales
Öffnen (ohne `FOPEN_PASSTHROUGH`) eines Inodes, der eine Hintergrunddatei hat.
Ein Passthrough-Öffnen, das `FOPEN_KEEP_CACHE` trägt, scheitert ebenfalls mit
EIO. Der Daemon registrierte früher bei jedem Öffnen eine neue ID, also
scheiterte das zweite Öffnen einer Datei, die das Spiel schon offen hielt. Wine
meldet EIO als `STATUS_ACCESS_VIOLATION`, den Fehler hinter den 152
gescheiterten Öffnungen. Diese Regel stammt aus dem Kernel-Quelltext und passt
zu diesen 152 Fehlschlägen, was sie zur wahrscheinlichsten Ursache macht.

Der Daemon registriert jetzt eine Hintergrunddatei pro Inode, die sich alle
Passthrough-Öffnungen dieses Inodes teilen, und setzt `FOPEN_KEEP_CACHE` nie bei
einem Passthrough-Öffnen. Passthrough gilt nur für schreibgeschützte Öffnungen
von Dateien aus den Spiel- und Mod-Schichten, die 1 MiB oder größer sind oder
auf `.dll`, `.exe` oder `.asi` enden. Sobald für eine Datei eine
Hintergrunddatei registriert ist, nutzt jedes schreibgeschützte Öffnen diese,
und solange die Datei normale Öffnungen hat, bekommen neue Leser ebenfalls
normale Öffnungen. Eine Datei im Overwrite geht nie über Passthrough, und ein
Mount, der Plugin-Zeitstempel projiziert, nutzt es gar nicht. Ein Schreiber, der
eine Datei öffnet, während Passthrough-Leser sie halten, bekommt ETXTBSY (in
Wine: `STATUS_SHARING_VIOLATION`), bevor irgendein Copy-up läuft, sodass ein
Öffnen mit `O_TRUNC` keine leere Overwrite-Kopie hinterlassen kann, die die
Datei des Mods verdeckt.
