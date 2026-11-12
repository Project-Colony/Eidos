<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=c365b34dbc69a832b1c9305498c94eb223b567a1 -->

# Fehlersuche und Diagnose

Die Umgebungsschalter, das Lesen der Operationszähler, die Health-Karten des Tabs
Diagnostics, wie man die Mod hinter einem Absturz findet, was man einem Fehlerbericht
beilegt und die bekannten Probleme samt ihrer Geschichte.

## Das VFS diagnostizieren

Zwei Umgebungsvariablen decken die meisten Fälle ab, in denen das Spiel etwas
sieht, dem das Dateisystem nicht zustimmt:

```sh
EIDOS_FUSE_STATS=1                  # Operationszähler, beim Aushängen ausgegeben
EIDOS_FUSE_NO_CACHE=1               # jeder kernelseitige Cache aus
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # oder einzeln benennen
```

Die feingranulare Form hat den Absturz mit `FOPEN_KEEP_CACHE` unter Bekannte
Probleme gefunden: alle vier abzuschalten beantwortet "liegt es am Caching?", und
erst das Benennen beantwortet "an welchem". Die Zähler beantworten die andere
Hälfte. Bei eingeschaltetem Passthrough (es ist standardmäßig aus) ist ein
Ladevorgang mit `read 0` einer, in dem `FUSE_PASSTHROUGH` jedes Byte im Kernel
geliefert hat, also ist alles, was Sie am Lesepfad optimieren wollten, bereits
umsonst.

Ein paar weitere Schalter grenzen einen Fehler weiter ein:

```sh
EIDOS_FUSE_TRACE=open,opendir       # jedes open / opendir protokollieren (1 = jeder Kanal)
EIDOS_FUSE_THREADS=1                # ein Daemon-Thread (Standard: CPU-Anzahl, höchstens 4)
EIDOS_FUSE_OPENDIR=1                # opendir-Handles behalten, auch wenn der Kernel sie weglassen kann
EIDOS_NO_INDEX=1                    # die Schichten durchlaufen, statt den Schichtindex zu nutzen
EIDOS_NO_OW_INDEX=1                 # dasselbe für den Mitgliedschaftsindex des Overwrite
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE wieder an, siehe Bekannte Probleme
```

Bei einem Steam-Start setzen Sie sie in der Startoption vor den Befehl, zum Beispiel
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. Die Ausgabe geht ins Log des
Laufs: das `logs/run-*.log` der Instanz bei einem Start aus dem Fenster, die
Steam-Konsole bei einem Steam-Start.

Um die zusammengeführte Sicht selbst anzusehen, öffnet `eidos shell <instance>` eine
Shell im Spielordner, so wie das Spiel ihn sieht, und
`eidos shell <instance> -- ls Data/meshes` führt dort einen Befehl aus; alles
Geschriebene landet im Overwrite
([usage.md](usage.md#eine-shell-in-der-zusammengeführten-sicht)). Der Knopf Save
tree im Tab Data schreibt den ganzen zusammengeführten `Data`-Baum in eine
Textdatei, eine Datei pro Zeile mit der Mod, aus der sie stammt, mit verborgenem
Home-Ordner.

## Eine Union von Hand einhängen

`eidos-fuse` kommt mit dem Arch-Paket und mit Builds aus dem Quelltext: `just build`
baut es, und `just install` oder `install.sh` kopiert es, wenn es da ist. Das
Release-Bundle und `eidos-linux` enthalten es nicht. Seine Aufrufzeile:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

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

## Warum Passthrough standardmäßig aus ist

Passthrough übergibt dem Kernel die echte Hintergrunddatei, sodass Lesevorgänge
diesen Daemon vollständig umgehen. Das A/B unten wurde mit einem früheren Daemon
gemessen, vor der Änderung, die im bekannten Problem „FUSE-Passthrough verhindert,
dass das Spiel Mod-Inhalte lädt“ weiter unten beschrieben ist. Diese Änderung
vermeidet die wahrscheinlichste Ursache, wurde aber noch nicht erneut in einem Spiel
gemessen, deshalb bleibt Passthrough standardmäßig aus, bis das geschehen ist.
A/B gemessen auf Skyrim SE 1.6.1170, proton-cachyos 11.0, Kernel 7.1.4, dieselbe
Ladereihenfolge mit 82 Plugins, einzige Variable war, ob die Binärdatei die
Capability trug:

| Passthrough | `NtCreateFile`-Fehlschläge mit `STATUS_ACCESS_VIOLATION` |
|-------------|----------------------------------------------------------|
| an          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`          |
| aus         | 0                                                        |

Mit eingeschaltetem Passthrough öffnete das Spiel keines seiner eigenen Archive
oder Plugins, was sich im Spiel als nicht vorhandene Mods zeigte - kein Fehler,
keine Logzeile. Ausgeschaltet erreichte dieselbe Ladereihenfolge das
Spielgeschehen mit lebendigen Plugins, Archiven und Papyrus-Skripten.

Der Fehlschlag war von innerhalb des Daemons unsichtbar: sein eigenes `open` gelang
jedes Mal, und der Kernel wies nie eine Hintergrunddatei ab (über eine ganze
fehlschlagende Sitzung mit `EIDOS_FUSE_TRACE=open` geprüft: null `open FAILED`, null
`passthrough refused`). Der Fehler entstand, nachdem der Daemon
`opened_passthrough` geantwortet hatte, also konnte ihn keine daemonseitige
Protokollierung sehen. Er war auch nicht endungsspezifisch - er traf Archive und
Plugins gleichermaßen, also die Dateien, die das Spiel über seine gesamte
Laufzeit offen hält.

`EIDOS_FUSE_PASSTHROUGH=1` schaltet es wieder ein, um zu messen, was es bringt, oder
um den Mechanismus erneut zu prüfen. Die Capability-Warnungen im Starter und im Tab
Diagnostics erscheinen nur, wenn Sie danach gefragt haben.

## Wird der Schichtindex tatsächlich benutzt?

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

## Das Spiel mit der CLI starten

Um das Spiel über die CLI `eidos` statt über das Fenster zu starten, setzen Sie seine
Steam-Startoption auf den absoluten Pfad von `eidos`, zum Beispiel:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Ein vorangestelltes `WINEDLLOVERRIDES` ist nicht nötig. Wenn eine aktivierte Mod
`d3dcompiler_47` importiert, legt Eidos seine mitgelieferte Kopie ins Präfix,
erzwingt sie als native und führt das mit allen DLL-Overrides zusammen, die eine
Mod mitbringt (ENB/ReShade/`.asi`-Loader).

Eidos startet das Spiel nie selbst über Proton. `eidos tool <instance> run <title>`
mit einem Eintrag, der das Spiel startet (der Loader des Script Extenders, der
Launcher oder die Spiel-Binärdatei), bittet den Store, dem die Kopie gehört, es zu
starten, so wie Run in einem Fenster, das kein Store geöffnet hat, und gibt aus, was
zu setzen ist, wenn die Einstellung des Stores Eidos nicht startet. `--print` gibt
den Store-Befehl aus, statt ihn auszuführen.

## Ein Spiel oder Tool, das sich nicht beendet

Während ein Spiel oder Tool läuft, wird der Run-Knopf zu Stop, und die Sperrkarte
hat ebenfalls einen Stop-Knopf. Drücken Sie ihn zweimal. Der zweite Druck beendet
jedes Programm, das der Start gestartet hat: zuerst SIGTERM, dann nach 10 Sekunden
SIGKILL für alles, was noch übrig ist. Was das Spiel nicht gespeichert hatte, geht
verloren. Danach tut Eidos, was es nach jedem Beenden tut: INI- und
Plugin-Änderungen gehen zurück ins Profil, die Ausgabe der Tools wird erfasst, und
die Mods werden ausgehängt. Der Lauf wird mit Exit-Code 143 aufgezeichnet, was
immer das Programm zurückgegeben hat, sodass `eidos sessions <instance> diff` ihn
nie als letzten guten Lauf nimmt. Unlock ist etwas anderes: Es gibt das Fenster
frei und lässt das Programm weiterlaufen. In einem Terminal bewirkt `kill <pid>`
auf den `eidos`-Prozess (die PID steht auf der Sperrkarte) dasselbe wie Stop.

## Finden, was kaputtgegangen ist

Um herauszufinden, welche Änderung das Spiel kaputtgemacht hat:

- `eidos sessions <instance> diff` listet auf, was sich seit dem letzten
  Spiellauf geändert hat, der mit Code 0 endete: Mods, Versionen,
  Plugin-Reihenfolge, INIs und die ausführbare Datei des Spiels
  ([usage.md](usage.md#was-sich-seit-dem-letzten-guten-lauf-geändert-hat)).
- Test run, neben Run, spielt mit den Mods des Profils, ohne Mods oder mit den
  ausgewählten Mods, die eingeschaltet sind, und den Mastern, die sie brauchen, und leitet alles, was das
  Spiel schreibt, nach `<instance>/test-run/`. Nach dem Lauf behalten oder
  verwerfen Sie jede Gruppe von Dateien: Spielstände, INI-Änderungen,
  Plugin-Zustand, Overwrite-Ausgabe und Logs ([usage.md](usage.md#testläufe)).
- Find the mod behind a problem im Tab Diagnostics führt Testläufe mit jeweils der
  Hälfte der verbleibenden Verdächtigen aus und fragt, ob das Problem aufgetreten
  ist. 200 Verdächtige brauchen etwa 8 Läufe, und die Suche überlebt das Schließen
  von Eidos ([usage.md](usage.md#die-mod-hinter-einem-problem-finden)).

Ein gestoppter Lauf füllt in dieser Suche nie ein Yes vor, weil er mit Exit-Code 143
aufgezeichnet wird (siehe oben).

## Health-Karten

Der Tab Diagnostics zeigt für jedes gefundene Problem eine Health-Karte, mit seiner
Ursache und bei manchen einem Knopf, der es behebt. Die Karten zur Startoption sind
in [install.md](install.md) beschrieben, die Karten zu NVAPI, Vollbild und ENB in
[graphics.md](graphics.md) und die Proton- und Präfix-Karten im nächsten Abschnitt.
`eidos check <instance>` gibt die Befunde aus, die kein Fenster brauchen
([usage.md](usage.md#zustandsprüfungen-im-terminal) listet sie auf), darunter die
Karten zu Präfix, Registry und `S:`-Laufwerk. Nur das Fenster zeigt die Proton-Karte
mit ihren Varianten für Flatpak, unbekannten Build und älteren Build, die
Patch-Reihenfolge, Extender-Skripte, Archive, die kein Plugin lädt,
Nexus-Anforderungen, veraltete Plugin-Kopien und die Karte Display.

### Das System

- "vm.max_map_count is <value>" erscheint unter 1048576, dem Standard unter Arch
  Linux, Fedora 39 und neuer und Ubuntu 24.04 und neuer. Manche Spiele unter Proton
  mappen mehr Speicherregionen und stürzen dann ohne Fehler ab. Tragen Sie
  `vm.max_map_count = 1048576` in eine Datei unter `/etc/sysctl.d` ein, zum Beispiel
  `80-gamecompatibility.conf`, und führen Sie `sudo sysctl --system` aus.
- "The open files limit is <value>" erscheint, wenn das harte Limit unter 524288
  liegt, was Protons esync braucht; darunter können Spiele nicht starten oder
  abstürzen. systemd 240 und neuer setzen es. Bei einem älteren Aufbau setzen Sie
  `DefaultLimitNOFILE=524288` in `/etc/systemd/system.conf` und
  `/etc/systemd/user.conf` und melden sich neu an.
- "Game files on a Windows file system" nennt den Spielordner, das Präfix oder die
  Instanzwurzel, wenn sie auf `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse),
  `exfat` oder `vfat` liegt. Proton braucht Symlinks, Unix-Rechte und
  Groß-/Kleinschreibung unterscheidende Namen. Verschieben Sie das Spiel, sein Präfix
  und die Instanz auf ein Linux-Dateisystem wie ext4, btrfs oder xfs.

### Die Spieldateien

Die Karte Game files oben im Tab zeigt, ob die Instanz die Store-Installation oder
eine Kopie des Spiels innerhalb der Instanz startet
([usage.md](usage.md#spieldateien-die-store-installation-oder-eine-kopie)). Starts
werden in drei Fällen verweigert, mit einer Karte, die sagt warum:

- "The game copy '<folder>' is missing": die Instanz startet eine Kopie, die
  verschwunden ist. Legen Sie sie zurück oder kehren Sie zur Store-Version zurück.
- "'<folder>' has no <binary>": der Ordner, den die Instanz startet, enthält keine
  ausführbare Spieldatei, ist also keine Kopie des Spiels.
- "<game> cannot run from a copy yet": Morrowind und Oblivion mit
  `bUseMyGamesDirectory=0` halten ihre Plugin-Liste im Spielordner.

"ModOrganizer.ini points tools at another game folder" erscheint, wenn die eigene
`ModOrganizer.ini` einer Liste, nicht von Eidos geschrieben, ein `gamePath` hat, das
nicht zu den Spieldateien passt, die die Instanz startet. Tools im Modus Mod
Organizer 2, etwa PGPatcher, lesen das Spiel von dort. Ändern Sie `gamePath` in
dieser Datei oder entfernen Sie die Datei, damit Eidos seine eigene schreibt.

### Die Ladereihenfolge

- "sTestFile lines make the game ignore plugins.txt" erscheint für Fallout 4,
  Fallout 4 VR und Starfield, wenn die `Fallout4Custom.ini` oder `StarfieldCustom.ini`
  des Profils Zeilen `sTestFile1` bis `sTestFile10` in `[General]` hat. Solange eine
  gesetzt ist, lädt das Spiel diese Plugins, und die Ladereihenfolge, die Eidos
  schreibt, bleibt wirkungslos. Fix entfernt die Zeilen aus der Kopie des Profils und
  behält die vorherige Datei als `<ini>.pre-fix`.
- Starfield: aktivierte `.esp`-Plugins ohne Master-Flag bekommen eine Karte, weil
  Starfield sie nicht zwischen `.esm` und Plugins mit Master-Flag einsortieren kann
  und einige ihrer Records die ganze Sitzung geladen hält. Eine `.esm`-Veröffentlichung
  des Autors vermeidet beides.
- Starfield: "Invalid blueprint plugins" listet Plugins auf, die einige, aber nicht
  alle drei Merkmale von Blueprint-Plugins haben: das Blueprint-Flag, einen Namen,
  der mit `BlueprintShips-` beginnt, und die Endung `.esm`. "Blueprint plugins
  without their main plugin" listet die auf, deren Hauptplugin nicht aktiviert ist:
  `BlueprintShips-Example.esm` braucht `Example.esm`, `.esp` oder `.esl`.
- "<patch> loses <n> file(s) to <master>" erscheint, wenn ein Plugin in einer Mod
  einen Master aus einer anderen Mod braucht und diese andere Mod die höhere Priorität
  hat, sodass ihre Kopien der Dateien gewinnen, die beide mitbringen. Ein Patch gehört
  meist unter die Mod, die er patcht, und der Knopf Move der Karte legt ihn dorthin.
  Behalten Sie die Reihenfolge, wenn der Patch diese Dateien verlieren soll.
- "Out-of-date copy of <plugin>" erscheint, wenn eine Mod eine bereinigte oder als
  Light markierte Kopie eines Plugins enthält, dessen Original sich seitdem geändert
  hat (ein Mod- oder Spiel-Update). Das Spiel lädt die alte Kopie. Verbergen Sie sie
  und bereinigen oder markieren Sie das neue Plugin erneut.

### Die Mod-Liste

- "Order rules of the collection <name> that no longer hold" erscheint, wenn die Mods
  einer installierten Kollektion nicht mehr in der Reihenfolge stehen, die ihre Regeln
  verlangen. Restore collection order verschiebt nur die Mods dieser Kollektion
  zurück, und Strg+Z macht es rückgängig.
- "<mod> requires <mod>, which is not installed" (oder "which is disabled") stammt aus
  den Nexus-Anforderungen, die Check for updates liest. Open on Nexus geht zur Seite
  der benötigten Mod, Download main file lädt sie mit einem Premium-Konto herunter,
  und Ignore verbirgt eine Anforderung, die eine andere Mod bereits erfüllt. Eine
  DLC-Anforderung ist nur ein Hinweis: Eidos sucht ein Plugin mit dem Namen des DLC.
- "fontconfig.txt names a font library that is missing" (Skyrim-Engines) nennt die
  Schriften, die keine Schicht und kein Archiv enthält, und die Mod, deren
  `Interface/fontconfig.txt` gewinnt. Menütext kann als Kästchen erscheinen, bis die
  Schrift installiert oder die Zeile entfernt ist.
- "BA2 archive stores audio compressed" (Fallout 4 und Fallout 4 VR) zählt die
  `.wav`- und `.xwm`-Dateien, die das BA2 einer Mod komprimiert speichert. Das Spiel
  kann abstürzen, wenn es sie abspielt. Packen Sie das Archiv mit unkomprimiertem
  Audio neu.

### Der Script Extender und Spiel-Updates

- "<loader> is not installed" erscheint, wenn Script-Extender-Plugins installiert sind
  (in `SKSE/Plugins`, `F4SE/Plugins` und so weiter), der Loader aber in keiner
  aktivierten Mod und nicht im Spielordner liegt. Das Spiel startet ohne sie.
- "The script extender does not match game version <version>" erscheint, wenn der
  Loader installiert ist, aber die Runtime-DLL für die laufende Spielversion nicht,
  zum Beispiel `skse64_1_6_1170.dll`. Der Loader bricht mit einem Fehler ab. Das
  folgt meist auf ein Spiel-Update. Diese Karte und die darüber haben einen Knopf
  Open the download page für die Seite des Extenders und einen Knopf Install the
  downloaded archive, der den Loader in den Ordner `Root/` der Mod legt.
- "<game> changed since the last clean launch" erscheint, wenn sich die ausführbare
  Datei des Spiels oder bei einer Steam-Kopie seine Build-ID seit dem letzten Start
  über Eidos geändert hat, der ohne Fehler endete. Ein Steam-Update, das die
  ausführbare Datei unverändert lässt, löst sie nicht aus. Script-Extender-Plugins,
  die für die alte Version gebaut wurden, laden womöglich nicht mehr, und die Karten
  darunter nennen die, die das erklären. Um die Spielversion zu behalten, auf der eine
  Liste aufgebaut wurde, frieren Sie sie in der Instanz ein, mit Freeze this version
  auf der Karte Game files oder mit `eidos stock-game <instance> freeze --yes`,
  nachdem `freeze` allein die Größe gezeigt hat: spätere Store-Updates ändern die
  Store-Installation, nicht die Kopie.
- "<mod> replaces <n> <extender> script(s)" erscheint, wenn eine Mod mit höherer
  Priorität als der Script Extender lose `.pex`-Kopien der eigenen Skripte des
  Extenders hat. Eine alte Kopie zerstört die Funktionen, die sie deklariert, und das
  Spiel zeigt keinen Fehler. Schieben Sie den Extender unter diese Mod (der Knopf Move
  der Karte tut das, wenn beide Mods sind) oder verbergen Sie diese Dateien in der Mod.
  Liegen die Skripte des Extenders lose im `Data` des Spiels, installieren Sie den
  Extender als Mod, damit er sich ordnen lässt. Archive werden nicht geprüft, nur lose
  Dateien.

### Tool-Ausgabe

- "Overwrite/Root holds meshes" (oder textures) heißt, dass ein Tool, dessen
  Datenpfad auf den Spielordner zeigte, diese Ordner dorthin geschrieben hat, wo das
  Spiel sie nie liest. Korrigieren Sie den Pfad des Tools, löschen Sie die Ordner und
  führen Sie das Tool erneut aus. Der Knopf Open folder öffnet `Overwrite/Root`.
- "BodySlide's GameDataPath is not the game's Data folder" (oder `OutputDataPath`):
  siehe [Tools](tools.md#die-eigenen-einstellungen-eines-tools-bleiben-seine-eigenen).

### Ein Absturz in der letzten Sitzung

"The game crashed in the last session" erscheint, wenn ein Crash-Logger beim letzten
Start ein Log geschrieben hat. Die Karte nennt die Ausnahme und die DLLs und Plugins
im Log, die Ihre Mods oder das Overwrite installieren. Nehmen Sie diese als Spuren:
Eine in einem Crash-Log genannte Mod ist oft der Ort, an dem sich der Absturz zeigte,
nicht seine Ursache. Nennt das Log keine Datei aus Ihren Mods, sagt die Karte das.
Open the log öffnet es, und Copy the log (home path hidden) kopiert es mit
verborgenem Home-Ordner.

### Die Fenstergröße

Für Skyrim SE, Enderal SE und Fallout 4 zeigt eine Karte Display die Fenstergröße
und den Modus, mit denen das Profil das Spiel startet, und sagt "No window size set
for this profile", wenn weder die Prefs-INI des Profils noch die des Präfix eine
setzt. Apply schreibt Größe und Modus in die Kopie des Profils und, wenn diese Mod
das Fenster setzt, in eine Kopie der INI von SSE Display Tweaks im Overwrite
([usage.md](usage.md#diagnostics)).

## Welches Proton das Spiel ausführt, und ein defektes Präfix

Der Tab Diagnostics hat eine Proton-Karte: den Build, den Eidos und Steam für das
Spiel verwenden, Steams eigene Einstellung und den Build, der das Präfix zuletzt
ausgeführt hat. Wenn ein neueres Proton als das ausgewählte das Präfix zuletzt
eingerichtet hat, wird die Karte zu einem Hinweis, denn ein älteres Proton auf einem
neueren Präfix kann dessen Wine-Dateien zurücksetzen oder gar nicht starten. Wählen
Sie in Steam unter Eigenschaften, Kompatibilität des Spiels einen mindestens ebenso
neuen Build oder bauen Sie das Präfix neu auf.

Die übrigen Proton- und Präfix-Karten:

- "Proton comes from the Flatpak Steam install": dieses Proton hält seine Runtime und
  die steamclient-Bibliotheken innerhalb der Flatpak-Sandbox, sodass es vom Host aus
  womöglich nicht läuft. Eidos startet es nicht über `flatpak run`, was das Spiel in
  die Sandbox setzen würde, außerhalb der zusammengeführten Sicht. Installieren Sie
  ein Proton in `~/.steam/root/compatibilitytools.d/` und wählen Sie es für das Spiel.
- "Eidos cannot tell which Proton runs this game": Steams `config.vdf` nennt dafür
  kein Kompatibilitätswerkzeug, und das Präfix verzeichnet keinen installierten
  Build. Tools starten nicht, bis in Steam eines gewählt ist (Eigenschaften des
  Spiels, Kompatibilität).
- "<game> has no Proton prefix yet": starten Sie das Spiel einmal über Steam und
  drücken Sie dann F5. Legt Steam das Präfix nicht an, prüfen Sie, ob der Ordner des
  Spiels unter `steamapps/compatdata` beschreibbar ist; ein leerer, der root gehört,
  blockiert es ohne Meldung.
- "Windows tools cannot find <game>": der Eintrag des Präfix zum Installationspfad ist
  falsch oder fehlt, und xEdit, DynDOLOD, Wrye Bash und das Creation Kit lesen ihn.
  Eidos schreibt den richtigen Pfad, wenn Sie das nächste Mal ein Tool starten.
- "The prefix's S: drive points outside the Steam library": `S:` passt weder zur
  Bibliothekswurzel, wohin Eidos es für ein Tool zeigen lässt, noch zu
  `<library>/steamapps`, wohin Steam es für das Spiel zeigen lässt. Die Bibliothek
  ist wohl umgezogen, oder das Präfix stammt von einem anderen Rechner. Jeder im
  Präfix gespeicherte, auf `S:` bezogene Pfad führt ins Leere, bis der nächste Start
  über Eidos oder Steam es wieder setzt.
- "<store> installation: started through its store" ist die Karte einer GOG- oder
  Epic-Kopie. Run bittet Heroic oder Legendary, das Spiel zu starten, wenn Eidos dort
  der Wrapper des Spiels ist, und Tools laufen mit dem Wine- oder Proton-Build, den
  Heroic für die Kopie verzeichnet.

Das Menü File startet winecfg, regedit, den Wine-Dateiexplorer und den
Deinstallierer im Präfix des Spiels, mit dem Proton, das die Tools des Spiels
ausführt ([tools.md](tools.md#wines-eigene-programme-im-prefix-des-spiels)). Ihre
Zeilen bleiben ausgegraut, bis das Präfix diese Programme hat.

Der Knopf Rebuild prefix auf der Proton-Karte ist für ein Präfix, das nicht mehr
funktioniert. Drücken Sie ihn zweimal. Eidos löscht nichts: Es benennt
`steamapps/compatdata/<appid>` im selben Ordner in
`<appid>.eidos-backup-<date>-<time>` um, und Steam legt ein neues Präfix an, wenn
Sie das Spiel das nächste Mal aus Steam starten. Solange ein Spiel, ein Tool oder
irgendein Wine-Programm das Präfix benutzt, verweigert es das. Bis das neue Präfix
existiert, kann Run im Fenster das Spiel nicht an Steam übergeben und zeigt stattdessen
die Startoption; drücken Sie also Spielen in Steam. Spielstände und Einstellungen,
die nur im alten Präfix liegen, bleiben im umbenannten Ordner: Kopieren Sie zurück,
was Sie brauchen, und löschen Sie diesen Ordner selbst, wenn Sie ihn nicht mehr
wollen.

## Was man einem Fehlerbericht beilegt

- Copy report im Tab Diagnostics kopiert die Eidos-Version, das Spiel, den
  Proton-Build, ob Steam das Flatpak ist, den Passthrough-Zustand, den Sitzungstyp,
  die Distribution, den Kernel, `vm.max_map_count`, das Limit offener Dateien, ob
  `/dev/fuse` und `fusermount3` da sind, die Namen der Startvariablen und
  Wrapper-Programme des Profils und jede Health-Karte. Ihr Home-Ordner erscheint als
  `~` und Ihr Login-Name als `<user>`. `eidos check <instance>` gibt denselben Kopf
  im Terminal aus, und `--json` liefert ihn als JSON.
- Das Log des Laufs, `<instance>/logs/run-*.log`, beginnt mit dem genauen Befehl, den
  Eidos ausgeführt hat. File, Eidos logs öffnet den Ordner mit den eigenen Logs von
  Eidos, `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden) auf der Absturzkarte kopiert das Absturzlog.
- Save tree im Tab Data speichert den zusammengeführten `Data`-Baum mit der Quelle
  jeder Datei.

## Bekannte Probleme

### DLSS oder Frame Generation tut stillschweigend nichts

Drei getrennte Ursachen, jede ohne jede Fehlermeldung: NVAPI nicht in den
Startoptionen aktiviert, exklusiver Vollbildmodus oder ein veraltetes
Reflex-FPS-Limit. Die ganze Prüfliste steht in [graphics.md](graphics.md).

### Run tut nichts mit dem Flatpak-Steam

In einem Fenster, das kein Store geöffnet hat, bittet Run Steam, das Spiel zu
starten, indem es `steam -applaunch <appid>` ausführt. Das Flatpak-Steam legt kein
`steam` in den `PATH` des Hosts, sodass die Statuszeile "Could not start Steam" mit
dem Fehler sagt. Drücken Sie stattdessen Spielen in Steam.

### Ein Spielstand von einem anderen Gerät erscheint nicht

Ein Spielstand, der ohne Umweg über Eidos ins Präfix gelangt, etwa ein Steam-Cloud-Download
von einem Steam Deck, bleibt verborgen, solange die Spielstände des Profils gemountet
sind. Nichts löscht ihn. Der Tab Saves zählt diese Spielstände unter seiner
Überschrift, und Import kopiert sie ins Profil, ohne einen Spielstand des Profils zu
ersetzen ([usage.md](usage.md#spielstände-gehören-zum-profil)).

### Eine Mod, die ein Verzeichnis auf zwei Arten schreibt, verlor alles unter der zweiten

Behoben. ext4 hält `meshes/` und `Meshes/` auseinander; die zusammengeführte Sicht
darf das nicht, und echte Mods liefern beides - XP32 Maximum Skeleton hat seine
Animationen und seine FNIS-Verhaltensdatei unter der großen Schreibweise, seine
`character assets` unter der anderen.

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

Der Pfadindex hatte diesen Fehler nie, weil er jedes Verzeichnis durchläuft, das er
findet. Eine Zeit lang lieferte der Index also Dateien, die der Rückfallweg nicht
fand, obwohl der Rückfallweg die Antwort sein soll, die nie falsch ist.

### DynDOLODs LODGen stirbt und hinterlässt ein leeres Log

Behoben durch `dotnet10`; siehe [tools.md](tools.md). Das
`LODGen_SSE_<world>_log.txt` jeder Welt enthält ein Versionsbanner, eine Zeile
`.NET Version:` und sonst nichts, und ein Dialog sagt nur "failed to generate object
LOD for one or more worlds". Ursache ist Wines Mono, das für .NET Framework
antwortet, und keine Menge installierter .NET Frameworks behebt es - Proton ersetzt
`mscoree.dll` bei jeder Präfix-Aktualisierung durch einen Symlink in seinen eigenen
Baum. Das Release-Bundle enthält die .NET-10-Runtime; andere Installationen laden sie
einmal herunter.

### Wine konnte nicht erkennen, dass das Mount Groß-/Kleinschreibung faltet

Behoben.

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

### Das Spiel überschrieb `plugins.txt` leer

Sehr wahrscheinlich dasselbe wie die Groß-/Kleinschreibungsfaltung oben - ein `Data`,
das es in keiner vernünftigen Zeit aufzählen konnte, woraus es schloss, dort sei
nichts, und das speicherte. Nicht bewiesen und eine erneute Prüfung wert. So oder so
bedeutet die Capture-Sicherung (eine Erfassung, die den aktiven Satz vollständig
leert, wird in jeder Größe abgelehnt), dass es das Profil nicht mehr beschädigen
kann.

### `FOPEN_KEEP_CACHE` ist aus

Behoben. Es ließ Skyrim SE Sekunden nach dem Hauptmenü deterministisch an einer
Null-Dereferenzierung abstürzen, ohne eine einzige installierte Mod; die anderen drei
kernelseitigen Caches wurden einzeln herausbisektiert, und nur dieser war von
Bedeutung. Diese Bisektion lief mit eingeschaltetem `FUSE_PASSTHROUGH`, und der
Kernel lässt ein Passthrough-Öffnen, das `FOPEN_KEEP_CACHE` trägt, mit EIO
scheitern, also scheiterte mit dem Flag jedes Passthrough-Öffnen. Vielleicht waren es
diese gescheiterten Öffnungen, die das Spiel abstürzen ließen, und das Flag allein,
mit ausgeschaltetem Passthrough, wurde nie getestet. Passthrough-Antworten tragen es
nicht mehr. Sein Verlust wurde damals als kostenlos gemessen, aber mit
eingeschaltetem Passthrough bedient der Daemon *null* Lesevorgänge
(`EIDOS_FUSE_STATS` meldete `read 0` für einen vollständigen Ladevorgang), also sind
die echten Kosten bei standardmäßig ausgeschaltetem Passthrough ungemessen. Mit
`EIDOS_FUSE_KEEP_CACHE=1` wieder einschalten, um es für sich zu testen.

### FUSE-Passthrough verhindert, dass das Spiel Mod-Inhalte lädt

Der Daemon vermeidet jetzt die wahrscheinlichste Ursache (unten), aber das wurde
noch nicht erneut in einem Spiel gemessen, deshalb bleibt Passthrough
standardmäßig aus; `EIDOS_FUSE_PASSTHROUGH=1` schaltet es ein. Vor der Änderung
scheiterte Skyrim SE mit eingeschaltetem Passthrough daran, 152 seiner eigenen
Dateien zu öffnen, mit `STATUS_ACCESS_VIOLATION`, also lud still kein Mod-Inhalt. Die
Messung und warum die eigenen Logs des Daemons einen sauberen Lauf zeigten, stehen
oben unter "Warum Passthrough standardmäßig aus ist".

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
Datei der Mod verdeckt.

### Der Button Mod Manager Download tut nichts

Preferences, Nexus, Downloads zeigt, welches Programm `nxm://`-Links erhält. Eine
Health-Karte erscheint, wenn dieses Programm nicht Eidos ist, fehlt oder keines
gesetzt ist. Der Knopf Register schreibt `eidos-nxm.desktop` und macht es zum
Standard-Handler für `x-scheme-handler/nxm`. Im Terminal erledigt
`eidos nxm --register` dasselbe.

Nennt der Status bereits Eidos und der Button tut trotzdem nichts, verwendet der
Browser womöglich eine frühere Wahl. Löschen Sie in den Browsereinstellungen die
gespeicherte Wahl für nxm-Links, damit er erneut fragt. Ein Flatpak-Browser kann
keine Programme des Hosts selbst starten und reicht den Link über das
Desktop-Portal weiter, das den auf dem Host registrierten Handler liest.
