<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

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
diesen Daemon vollständig umgehen. Es ist ein Durchsatzgewinn, der hier
Korrektheit kostet. A/B gemessen auf Skyrim SE 1.6.1170, proton-cachyos 11.0,
Kernel 7.1.4, dieselbe Ladereihenfolge mit 82 Plugins, einzige Variable war, ob das
Binary die Capability trug:

| Passthrough | `NtCreateFile`-Fehlschläge mit `STATUS_ACCESS_VIOLATION` |
|-------------|----------------------------------------------------------|
| an          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`          |
| aus         | 0                                                        |

Mit eingeschaltetem Passthrough öffnet das Spiel keines seiner eigenen Archive oder
Plugins, was sich im Spiel als schlicht nicht vorhandene Mods zeigt - kein Fehler,
keine Logzeile. Ausgeschaltet erreicht dieselbe Ladereihenfolge das Spielgeschehen
mit lebendigen Plugins, Archiven und Papyrus-Skripten.

Der Fehlschlag ist von innerhalb des Daemons unsichtbar, was ihn teuer zu finden
machte: unser eigenes `open` gelingt jedes Mal, und der Kernel weist nie eine
Hintergrunddatei ab (über eine ganze fehlschlagende Sitzung mit
`EIDOS_FUSE_TRACE=open` geprüft: null `open FAILED`, null `passthrough refused`).
Der Fehler entsteht, nachdem der Daemon `opened_passthrough` geantwortet hat, also
kann ihn keine daemonseitige Protokollierung sehen. Er ist auch nicht
endungsspezifisch - er trifft Archive und Plugins gleichermaßen, also die Dateien,
die das Spiel über seine gesamte Laufzeit offen hält.

`EIDOS_FUSE_PASSTHROUGH=1` schaltet es wieder ein, um zu messen, was es bringt, oder
um den Mechanismus erneut zu prüfen. Die Capability-Warnungen im Starter und im
Diagnostics-Reiter erscheinen nur, wenn Sie danach gefragt haben.

Um das Spiel selbst durch Eidos zu starten, setzen Sie seine Steam-Startoption auf:

```
eidos play skyrimse -- %command%
```

Stellen Sie `WINEDLLOVERRIDES="d3dcompiler_47=n"` davor, falls Proton den nativen
d3dcompiler für die Shader-Übersetzung braucht; Eidos führt das mit allen
DLL-Overrides zusammen, die eine Mod mitbringt (ENB/ReShade/`.asi`-Loader).

### Wird der Schichtindex tatsächlich benutzt?

Der Index ist alles oder nichts und wird schweigend aufgebaut: `LayerStack::new`
bekommt entweder eine vollständige Karte der Nur-Lese-Schichten oder `None`, wonach
jede Abfrage sie genau wie vorher durchläuft. Nichts in einem Sitzungsprotokoll
unterscheidet die beiden, sodass ein Stapel, der still zurückgefallen ist,
identisch zu einem funktionierenden aussieht - während er die alten Kosten zahlt.

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
Reflex-FPS-Limit. Die ganze Prüfliste steht in [graphics.de.md](graphics.de.md).

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

Es gibt keine API für "ist dieses Dateisystem case-insensitiv", also schnüffelt
Wines `get_dir_case_sensitivity` nach dem Marker, den CIOPFS in den von ihm
bedienten Verzeichnissen hinterlässt. Fehlt er, nimmt Wine case-SENSITIV an, und
jede Suche, deren Schreibweise nicht Byte für Byte passt, fällt darauf zurück, das
GANZE Verzeichnis zu lesen, um eine Übereinstimmung ohne Rücksicht auf Groß- und
Kleinschreibung zu finden. Bethesda-Spiele fragen nach `data/ccbgssse001-fish.bsa`,
während die Datei `ccBGSSSE001-Fish.bsa` heißt, also feuerte das bei nahezu jedem
Asset: 4471 Marker-Abfragen und 2236 vollständige Verzeichnis-Neulesungen in acht
Sekunden, und 195796 Aufzählungen von `Data` in neunzig. Skyrim SE erreichte nie
sein Hauptmenü - es saß bei 240 MB resident, während der Daemon 92 % eines Kerns
verbrannte.

Eidos faltete Groß- und Kleinschreibung in `resolve_read` von Anfang an. Die ganzen
Kosten entstanden nur daraus, es nie zu sagen. `lookup` antwortet nun `.ciopfs`;
`readdir` listet es weiterhin nicht.

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
Null-Dereferenzierung abstürzen, ohne einen einzigen installierten Mod; die anderen
drei kernelseitigen Caches wurden einzeln herausbisektiert, und nur dieser war von
Bedeutung. Sein Verlust wurde damals als kostenlos gemessen, aber diese Messung
entstand mit aktivem `FUSE_PASSTHROUGH`, wo der Daemon *null* Lesevorgänge bedient
(`EIDOS_FUSE_STATS` meldete `read 0` für einen vollständigen Ladevorgang) und der
Kernel diese Seiten bereits gegen die Hintergrunddatei cachte. Passthrough ist
inzwischen standardmäßig aus (siehe oben), also gilt das Argument nicht mehr und die
echten Kosten sind ungemessen - der Absturz allein ist Grund genug, es aus zu
lassen. Mit `EIDOS_FUSE_KEEP_CACHE=1` zum Untersuchen wieder einschalten; die beiden
Flags sind nicht mehr verwoben, es lässt sich also nun für sich testen.

### FUSE-Passthrough verhindert, dass das Spiel Mod-Inhalte lädt

Behoben, indem es abgeschaltet wurde; `EIDOS_FUSE_PASSTHROUGH=1` holt es zurück. Mit
eingeschaltetem Passthrough scheitert Skyrim SE daran, 152 seiner eigenen Dateien zu
öffnen (75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`) mit `STATUS_ACCESS_VIOLATION`,
gegenüber 0 ohne, auf Kernel 7.1.4 - also lädt stillschweigend kein Mod-Inhalt. Der
Kernel meldet den Fehler, nachdem der Daemon `opened_passthrough` geantwortet hat,
sodass die Logs des Daemons einen sauberen Lauf zeigen (null fehlgeschlagene
Öffnungen, null abgewiesene Hintergrunddateien). Die Grundursache im Kernelpfad ist
nicht geklärt; der Schalter bleibt, damit sich das erneut prüfen lässt und damit
Passthrough auf DLLs eingegrenzt werden könnte, falls sich zeigt, dass das
Image-Mapping ihn braucht.
