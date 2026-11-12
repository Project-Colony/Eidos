<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Eidos verwenden

So benutzen Sie Eidos: Instanzen und Spielkopien, die Kommandozeile, Testläufe, das
Fenster, Nexus-Collections und Updates, den Umzug einer Instanz auf einen anderen
Rechner, Tools und Erweiterungen. Bei Problemen siehe
[troubleshooting.md](troubleshooting.md); zum Bauen aus dem Quelltext und zum Aufbau
des Repositorys siehe [../internals/contributing.md](../../../../internals/contributing.md).

## Instanzen: global und portabel

Befehle, die eine Instanz annehmen, akzeptieren eine Spiel-ID oder den Ordner einer
portablen Instanz. `skyrimse` benennt die globale Instanz, die Eidos unter
`~/.local/share/Colony/Eidos/instances/skyrimse` ablegt. Eine portable Instanz ist ein
in sich geschlossener Ordner, wo immer Sie ihn haben wollen (eine zweite Platte, eine
Spielepartition), und Sie können ihn verschieben, wie die portablen Instanzen von MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # dort eine portable Instanz anlegen
eidos install /mnt/games/EidosSkyrim mod.7z  # jeder Befehl akzeptiert den Ordner
eidos play /mnt/games/EidosSkyrim -- %command%
```

Die `eidos-instance.ini` des Ordners nennt das Spiel, der Ordner braucht also nichts
weiter. `EIDOS_INSTANCE=<folder>` in der Umgebung leitet eine Spiel-ID auf diesen
Ordner um, zum Beispiel in einer Steam-Startoption. Portable Instanzen, die Sie angelegt
oder geöffnet haben, werden (zuletzt benutzte zuerst) in
`~/.config/Colony/Eidos/instances.ini` gemerkt: Der Willkommensbildschirm des Fensters
listet sie, der Steam-Start landet auf der zuletzt gespielten, und der
`nxm://`-Handler lädt in sie herunter.

Einen portablen Ordner von Hand zu verschieben behält alles außer den Tool-Einträgen, die
Sie mit absoluten Pfaden in den alten Ort registriert haben. `eidos pack` und
`eidos unpack` ([unten](#eine-instanz-auf-einen-anderen-rechner-umziehen)) korrigieren
die; ein bloßes `mv` nicht. Der Runtime-Cache in
`~/.local/share/Colony/Eidos/runtimes/` wird von jeder Instanz auf dem Rechner geteilt,
ein 78 MB großer .NET-Host wird also nicht in jede kopiert.

Ein Ort wird abgelehnt: der Installationsordner eines Spiels, wo MO2-Nutzer ihre Instanz
oft ablegen. Steam besitzt diesen Baum, und ein Update, ein "verify integrity" oder eine
Deinstallation kann ihn samt Ihrer Einrichtung überschreiben oder löschen. Eidos mountet
außerdem über die Spielwurzel, eine Instanz darin säße also innerhalb ihres eigenen
Mount-Ziels. Der Assistent, `eidos init`, `eidos play` und `eidos shell` lehnen das ab.
Legen Sie den Ordner stattdessen neben das Spiel; ein Geschwisterordner auf derselben
Platte ist genauso bequem.

### Wo Eidos seine Dateien ablegt

Eidos hält seine eigenen Dateien unter `Colony/Eidos`, dem Aufbau, den jedes Programm der
Colony-Familie verwendet: `~/.config/Colony/Eidos/` für das, was Sie gewählt haben
(Einstellungen, Ihre Nexus-Sitzung, Ihre Instanzliste, die Spiel- und Add-on-Definitionen,
die Sie geschrieben haben), `~/.local/share/Colony/Eidos/` für globale Instanzen
(`instances/`), heruntergeladene Runtimes und Sitzungsprotokolle (`logs/`) und
`~/.cache/Colony/Eidos/` für die LOOT-Masterlisten, die erneut heruntergeladen werden,
sobald sie fehlen.

Findet es Dateien im Aufbau eines älteren Eidos (`~/.config/eidos/`,
`~/.local/share/eidos/` und `~/.local/state/`), kopiert der erste Start sie herüber und
schreibt das ins Log. Eine globale Instanz wird mit Hardlinks statt einer Byte-Kopie
verschoben, sodass ihre Mods keinen zusätzlichen Platz belegen, und ihre Tool-Einträge
werden auf den neuen Ordner umgestellt. In den alten Verzeichnissen wird nichts geändert
oder gelöscht; entfernen Sie sie selbst, sobald der neue Aufbau funktioniert. Lässt sich
eine Kopie nicht anlegen (ein volles oder schreibgeschütztes Laufwerk, ein alter
Instanzordner auf einem anderen Laufwerk, ein Spiel, das noch daraus läuft), verwendet
Eidos weiter den alten Ort und versucht es beim nächsten Start erneut. Eine
Steam-Startoption, die noch den alten Instanzordner nennt, öffnet die verschobene Instanz.

Die Hardlinks sparen nur anfangs Platz. Eine Mod oder ein Download, den Sie danach
entfernen oder neu installieren, behält über den alten Instanzordner seine alte Kopie auf
dem Laufwerk, und das Löschen von `~/.local/share/eidos/<game>/` gibt diesen Platz wieder
frei.

Eine portable Instanz wird nie verschoben: Sie bleibt dort, wo Sie sie hingelegt haben.

## Die Kommandozeile

```sh
eidos games                                # hier installierte unterstützte Spiele (wie MO2s Liste)
eidos init skyrimse [folder]               # eine Instanz anlegen; mit Ordner ist sie portabel
eidos init skyrimse --game-path <folder>   # die in <folder> installierte Kopie nutzen, wenn mehrere installiert sind
eidos install skyrimse mod.7z [name]       # ein Archiv installieren (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # über die Mod neu installieren (--merge installiert über ihre Dateien)
eidos import skyrimse <mo2-profile>        # Mod-Reihenfolge und Plugin-Zustand eines MO2-Profils übernehmen
eidos sort skyrimse [--dry-run]            # die Ladereihenfolge mit LOOT sortieren (--update-masterlist aktualisiert sie vorher)
eidos play skyrimse                        # zeigen, was gemountet würde
eidos play skyrimse -- <command>           # <command> mit den über das Spiel gemounteten Mods ausführen
eidos shell skyrimse                       # eine Shell in der zusammengeführten Sicht; Schreibzugriffe landen im Overwrite
eidos check skyrimse [--json]              # Health-Prüfungen ohne Fenster
eidos sessions skyrimse                    # die aufgezeichneten Spiel- und Tool-Läufe, neueste zuerst
eidos sessions skyrimse diff [<id>]        # was sich seit dem letzten Lauf mit Exit-Code 0 geändert hat
eidos stock-game skyrimse                  # welche Spieldateien die Instanz nutzt; eine Kopie einfrieren oder wechseln
eidos tool skyrimse                        # die Tools auflisten (add, rm und run: siehe Tools unten)
eidos tool skyrimse run --exe <path>       # eine exe einmal durch die zusammengeführte Sicht ausführen
eidos tool skyrimse import-mo2 [--write]   # die Programme einer ModOrganizer.ini; --write fügt sie hinzu
eidos prereqs skyrimse [--install]         # was die Tools brauchen; --install holt, was fehlt
eidos collection skyrimse <link>           # eine Nexus-Collection installieren (--dry-run, --no-optional)
eidos nexus status                         # die gespeicherte Nexus-Anmeldung prüfen
eidos nexus update skyrimse [--apply]      # nach Mod-Updates suchen; --apply installiert sie
eidos nxm <link>                           # einen Mod-Manager-Download-Link herunterladen
eidos nxm --register                       # den Knopf Mod Manager Download der Website Eidos öffnen lassen
eidos export skyrimse -o mods.csv --active # MO2s CSV-Format; --active listet nur aktivierte Mods
eidos pack skyrimse backup.eidos           # die ganze Instanz in einer Datei
eidos unpack backup.eidos [folder]         # ohne Ordner kommt eine globale Instanz an ihren zentralen Ort zurück
eidos --version                            # die Version ausgeben
```

`eidos` ohne Argumente gibt die Liste der Befehle aus. Die meisten Befehle geben ihre
eigenen Optionen aus, wenn ein Argument fehlt (`eidos init` und `eidos import` geben
stattdessen die vollständige Liste aus), und `eidos tool --help` listet die
Unterbefehle von tool auf. `eidos pack` nimmt `--dry-run`, `--level`, `--no-downloads`,
`--no-recovery` und `--force`, und `eidos unpack` nimmt `--info` und `--force`; beide sind
[unten](#eine-instanz-auf-einen-anderen-rechner-umziehen) beschrieben. `eidos stock-game`
steht unter [Spieldateien](#spieldateien-die-store-installation-oder-eine-kopie).

### Ein Spiel durch die zusammengeführte Sicht starten

`eidos play` mountet die Mods der Instanz über das eigene `Data`-Verzeichnis des Spiels,
in einem privaten Namensraum, und führt den Befehl dann durch diese Sicht aus. Die eigenen
Dateien des Spiels werden zuerst per Bind-Stash weggelegt, damit der Daemon sie weiterhin
unverändert liest. Schreibvorgänge (Spielstände, neu erzeugte Konfigurationen) landen in
der `overwrite/`-Schicht der Instanz; die Spielinstallation und jeder Mod-Ordner bleiben
Byte für Byte unberührt.

Eidos braucht kein root. Es mountet in einem privaten User- und Mount-Namensraum, ohne
setuid-Helfer, ohne Systemdienst und ohne etwas zu gewähren. Eine optionale Capability auf
der Binärdatei `eidos` erlaubt FUSE-Passthrough, das aus bleibt, solange nicht
`EIDOS_FUSE_PASSTHROUGH=1` gesetzt ist; siehe
[install.md](install.md#optional-fuse-passthrough). Mit der Capability nimmt Eidos einen
einfachen Mount-Namensraum statt eines User-Namensraums, und die Mods werden in beiden
gleich ausgebracht.

### Eine Shell in der zusammengeführten Sicht

`eidos shell <instance>` startet Ihre `$SHELL` (oder `/bin/sh`) im Spielordner, so wie das Spiel ihn sehen würde: jede aktivierte Mod über `Data` gemountet, Root-Mods über der Spielwurzel, und alles, was dort geschrieben wird, landet im Overwrite. `eidos shell <instance> -- <command>` führt stattdessen einen einzelnen Befehl aus, zum Beispiel `eidos shell skyrimse -- ls Data/meshes` oder ein natives Tool wie NifSkope. Der Lauf hält die Instanzsperre wie eine Spielsitzung, das Fenster kann also bis zum Beenden keine Änderungen speichern, und er steht in `eidos sessions` unter dem Namen `shell`, nie als Spielstart. Strg+C bei einem Befehl beendet den Lauf wie Stop: Eidos beendet den Befehl, erfasst die Sitzung und hängt die Sicht aus. In der interaktiven Shell gehört Strg+C wie gewohnt der Shell; geben Sie `exit` ein, um den Lauf zu beenden.

### Zustandsprüfungen im Terminal

`eidos check <instance>` führt die Health-Prüfungen aus, die kein Fenster brauchen, sodass
sie über SSH oder im Game Mode laufen und in einen Fehlerbericht wandern können:

- das Proton-Präfix und seine Registry;
- Plugin-Diagnosen und fehlende Master;
- der Script Extender und seine Plugins;
- ein Spiel-Update seit dem letzten sauberen Start;
- die Steam-Startoption;
- Grafikeinstellungen (NVAPI, Vollbild, ENB);
- das Absturzlog des letzten Laufs;
- `sTestFile`-Zeilen in den Profil-INIs;
- `.esp`- und Blueprint-Plugins bei Starfield;
- `vm.max_map_count` und das Limit offener Dateien;
- die Spielkopie, die die Instanz startet, und ein `gamePath` in der `ModOrganizer.ini`
  einer Liste, der einen anderen Spielordner nennt;
- Reihenfolgeregeln einer Collection, die nicht mehr gelten;
- Spieldateien auf NTFS, exFAT oder FAT;
- BodySlide-Datenpfade;
- `Data`-Ordner, die in `Overwrite/Root` liegen geblieben sind;
- Schriftbibliotheken, die `Interface/fontconfig.txt` nennt und keine Mod liefert;
- komprimiertes Audio in den BA2-Archiven von Mods für Fallout 4 und Fallout 4 VR.

Einige Karten berechnet nur das Fenster, im Tab Diagnostics: Patch-Reihenfolge,
Extender-Skripte, die eine Mod ersetzt, Archive, die nichts lädt, Nexus-Anforderungen,
Plugins, die LOOT als verschmutzt markiert, der Proton-Build, der das Spiel startet, die
Karte Display und Tool-Ausgabe, die älter ist als ihre Eingaben.

Der Befehl gibt dieselben Systemzeilen aus wie Copy report, dann eine Zeile pro Befund,
wobei Ihr Home-Ordner als `~` und Ihr Anmeldename als `<user>` erscheint. Der Exit-Code ist
0, wenn kein Problem gefunden wurde (reine Hinweise ergeben weiterhin 0), 1, wenn
mindestens ein Problem gefunden wurde, und 2, wenn Eidos nicht prüfen konnte: falsche
Argumente, eine Instanz, die es nicht öffnen kann, oder ein Spiel, das es nicht erkennt.

`eidos check <instance> --json` gibt stattdessen ein einziges JSON-Objekt aus. Seine Felder
der obersten Ebene sind `eidos` (die Version), `game`, `instance`, `system` (`proton`,
`flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`,
`max_map_count`, `open_files_hard`) und `findings`. `max_map_count` und `open_files_hard`
sind Zahlen, oder `null`, wenn Eidos sie nicht lesen konnte. Jeder Befund besteht aus einem
`code`, einem `level` (`problem` oder `advice`) und seinen `args`; der Tab Diagnostics zeigt
den Satz zu jedem Code.

### Was sich seit dem letzten guten Lauf geändert hat

Jeder Lauf über `eidos play`, `eidos shell` oder `eidos tool run`, ob aus dem Fenster, einem Terminal oder Steam, schreibt einen Eintrag nach `<instance>/logs/sessions/`: das Profil, jede aktivierte Mod mit Version und Nexus-Datei-IDs, die Plugin-Reihenfolge, eine Prüfsumme jeder Profil-INI, die Version der Spiel-Exe, die Eidos-Version, den Exit-Code und die Spielstände, die der Lauf angelegt hat. Die 50 neuesten bleiben erhalten, und `logs/` kommt nicht in eine gepackte Instanz. `eidos sessions <instance> diff` vergleicht den neuesten Spiellauf mit Exit-Code 0 mit dem aktuellen Stand; `diff <id>` wählt einen anderen Eintrag aus der Liste. Die Liste führt einen [Testlauf](#testläufe) als `test`, und ein Testlauf wird nie als letzter guter Lauf gewählt. Ein Lauf, den Sie gestoppt haben, zeichnet den Exit-Code 143 auf und wird deshalb ebenfalls nie gewählt. Eidos erkennt hier noch keine Abstürze, und ein unter Proton abgestürztes Spiel kann trotzdem mit Code 0 enden: Gehört der gewählte Lauf dazu, nennen Sie mit `diff <id>` einen früheren.

## Die Mod-Liste

Wie in MO2 hat jedes Profil seine eigene `modlist.txt` (welche Mods aktiviert sind, mit
welcher Priorität), während die Mod-Ordner im gemeinsamen `mods/` der Instanz liegen. Ein
Ordner, den Sie von Hand nach `mods/` legen, ist noch nicht aufgeführt: Er erscheint
deaktiviert mit der höchsten Priorität, am Ende der Liste im Fenster wie in MO2, sodass er
seine Konflikte gewinnt, sobald Sie ihn aktivieren.

Weil die Ordner gemeinsam sind, ändert das Entfernen oder Umbenennen einer Mod jedes
Profil: Ihre Zeile wird aus der `modlist.txt` jedes Profils entfernt oder darin
umbenannt, und im Übrigen behält jedes Profil seinen eigenen Zustand und seine eigene
Reihenfolge. Ein Profil, dem keine Mods bleiben, hat gar keine `modlist.txt`.

Das Fenster arbeitet mit seiner eigenen Kopie der Mod-Liste und der Ladereihenfolge.
Schreibt ein anderer Eidos-Prozess eine der beiden, während es offen ist (`eidos install`
oder `eidos sort` in einem Terminal, eine Collection, eine von Steam gestartete Sitzung,
ein zweites Fenster), wird die nächste Änderung im Fenster abgelehnt, statt diese Arbeit
zu überschreiben. Es lädt neu von der Platte und sagt *Not saved: another Eidos process
changed the mod list. It has been reloaded - redo the change.* (oder dasselbe für die
Ladereihenfolge); nur dieser eine Klick geht verloren, machen Sie ihn also noch einmal.
Rename, New mod, Add separator, Remove und das Umwandeln des Overwrite in eine Mod prüfen,
bevor sie `mods/` anfassen, sodass eine abgelehnte Änderung auf der Platte nichts ändert.
Ebenso lehnt der INI-Editor es ab, über eine Datei zu speichern, die sich geändert hat,
seit er sie geöffnet hat.

## Spielstände gehören zum Profil

Jedes Profil hält seine eigenen Spielstände in `profiles/<name>/saves/`, und für die Dauer
eines Laufs wird dieser Ordner über den Spielstand-Ordner des Spiels im Proton-Präfix
gemountet. Der erste Start eines Profils kopiert die bereits im Präfix liegenden
Spielstände hinein, einmalig, sodass ein Spielstand, den Sie später aus dem Profil
löschen, gelöscht bleibt. Nach jedem Lauf werden die neuesten Spielstände des Profils für
Steam Cloud zurück ins Präfix kopiert; ein Spielstand im Präfix, den Eidos nicht dort
hingelegt hat, wird zuerst als `orphan-<time>-<name>` im Profil aufbewahrt, nie
überschrieben.

Ein Spielstand, der nach dieser ersten Kopie im Präfix landet, ohne durch Eidos zu gehen
(ein Steam-Cloud-Download von einem Steam Deck, ein Start ohne Eidos), bleibt verborgen,
solange die Spielstände des Profils gemountet sind. Nichts löscht ihn: Der Start nennt ihn
im Log des Laufs (das `logs/run-*.log` der Instanz bei einem Start aus dem Fenster, die
Konsole von Steam bei einem Steam-Start), jeweils einmal, wenn sich dieser Satz von Dateien
ändert, und der Tab Saves zählt diese Spielstände in der Zeile unter seiner Überschrift. Um
einen zu spielen, drücken Sie Import im Tab Saves. Import kopiert jeden mit seinem Co-Save
ins Profil und ersetzt nie einen Spielstand des Profils: Ein Spielstand, dessen Namen das
Profil schon verwendet, wird als `orphan-<time>-<name>` importiert.

## Testläufe

Die Schaltfläche Test run neben Run startet das Spiel wie Run, aber alles, was das Spiel schreibt, landet in `<instance>/test-run/` statt in Ihrem Profil, im Overwrite und im Präfix: neue und geänderte Spielstände, INI-Änderungen, `plugins.txt`, Dateien, die in Data oder in den Spielordner geschrieben werden, und die Logs in `Documents/My Games/<game>`. Sie können mit den Mods des Profils testen, ganz ohne Mod oder nur mit den in der Liste ausgewählten Mods, die aktiviert sind, plus den Mods, deren Plugins sie als Master brauchen.

Wenn das Spiel endet, zeigt Eidos diese Dateien in fünf Gruppen (Spielstände, INI-Änderungen, Plugin-Zustand, Overwrite-Ausgabe, Logs) und fragt bei jeder Gruppe, ob Sie sie behalten oder verwerfen. Apply wartet, bis jede Gruppe, die Dateien enthält, eine Antwort hat. Der Plugin-Zustand wird nicht angeboten, wenn der Lauf einen anderen Satz von Mods als den des Profils benutzt hat. Das Behalten der Spielstände löscht nie einen Spielstand aus dem Profil, auch keinen, den das Spiel während des Laufs gelöscht hat. Das Behalten der Overwrite-Ausgabe entfernt außerdem die Dateien aus dem Overwrite, die das Spiel gelöscht hat, und die Liste zählt sie vorher. Etwas zu verwerfen braucht einen zweiten Klick. Decide later schließt die Liste und lässt die Dateien in `test-run/`, und die Liste öffnet sich wieder, sobald Sie die Instanz das nächste Mal öffnen oder F5 drücken. Hält Eidos oder der Rechner während eines Testlaufs an, bleiben die Dateien ebenfalls dort, und die Liste öffnet sich auf dieselbe Weise.

Wurde das Fenster nicht vom Store des Spiels gestartet, übergibt Test run das Spiel an diesen Store (Steam, Heroic oder Legendary), wie Run es tut, und der Lauf, den der Store startet, ist der Testlauf. Startet der Store das Spiel nicht, bleibt die Anforderung wartend: Der nächste Spiellauf dieses Profils, aus Run oder aus dem Store, ist der Testlauf, und Cancel the waiting test run verwirft sie. Ein Testlauf deckt nur die oben genannten Ordner ab: andere Schreibzugriffe des Spiels, etwa in die Wine-Registry, gehen dorthin, wo sie immer hingehen. Spielstände aus einem Testlauf erreichen Steam Cloud erst, nachdem Sie sie behalten und einen normalen Lauf gespielt haben.

### Die Mod hinter einem Problem finden

Find the mod behind a problem im Tab Diagnostics sucht die Mod, die ein Problem verursacht, das Sie nachstellen können, etwa einen Absturz an einer bestimmten Stelle. Jeder Schritt ist ein Testlauf mit der Hälfte der verbleibenden Verdächtigen, den Mastern, die sie brauchen, und den Mods, die Sie immer aktiv lassen: den in der Liste ausgewählten, aktivierten Mods beim Start und der Mod, die den Script Extender liefert. Nach jedem Lauf beantworten Sie, ob das Problem aufgetreten ist. Not sure teilt die Verdächtigen anders auf, und Run it again wiederholt den Schritt. Eine Antwort oder Run it again verwirft die Dateien, die der Schritt geschrieben hat, nach einem zweiten Klick. Um einige davon zu behalten, drücken Sie vor der Antwort See the files: Es öffnet dieselbe Liste wie nach einem Testlauf. Ist das Spiel abgestürzt und hat ein Crash-Logger ein Log geschrieben, oder endete das Spiel mit einem Fehlercode, markiert Eidos Yes als wahrscheinliche Antwort. 200 Verdächtige brauchen etwa 8 Läufe.

Die Suche wird in `<instance>/bisect.json` gespeichert, Sie können Eidos also zwischen den Schritten schließen, und sie gehört zu dem Profil, in dem sie begonnen hat. Stop the search verwirft sie nach einem zweiten Klick. Das Ergebnis nennt die Mod oder die Gruppe von Mods, die gegenseitig ihre Master brauchen, und wird in `<instance>/logs/bisect-<time>.txt` aufbewahrt. Generierte Mods (eine Nemesis- oder DynDOLOD-Ausgabe, ein Bashed Patch) sind für die vollständige Liste gebaut und können mit der Hälfte davon scheitern: Lassen Sie sie immer aktiv oder während der ganzen Suche aus.

## Wabbajack-Listen

Eidos hat keinen Wabbajack-Installer. Installieren Sie die Liste mit CLF3 oder Jackify außerhalb von Eidos und wählen Sie den Installationsordner im Assistenten als portable Instanz. Eidos liest den Ordner so, wie MO2 ihn anlegt: `mods/`, `profiles/<name>/modlist.txt` und die Plugin-Listen. Die vom Installer geschriebene `ModOrganizer.ini` bleibt erhalten und wird nie überschrieben, sodass die eigenen Programme der Liste registriert bleiben.

Viele Listen legen einen Ordner `Stock Game` oder `Game Root` an, der eine angepasste Kopie des Spiels enthält, und MO2 startet das Spiel von dort. Übernimmt der Assistent oder `eidos init` einen Listenordner zum ersten Mal und enthält dieser Ordner die ausführbare Datei des Spiels, startet auch die Instanz das Spiel aus diesem Ordner. Das Spiel startet weiterhin über Steam oder den Store, dem es gehört, sodass nichts verschoben werden muss. Eidos findet den Ordner über den `gamePath` in der `ModOrganizer.ini` der Liste oder an diesen beiden Namen. Eine Instanz, die mit einer früheren Eidos-Version übernommen wurde, behält die Store-Installation, bis Sie umschalten (siehe nächster Abschnitt). Nennt dieser `gamePath` einen Spielordner, den die Instanz nicht startet, sagt Diagnostics das, weil Tools im Modus Mod Organizer 2, etwa PGPatcher, das Spiel von dort lesen. CLF3 baut BSA- und DDS-Dateien mit eigenen Tools neu, deshalb kann das Ergebnis von einer Wabbajack-Installation abweichen, und die Listenautoren unterstützen es nicht.

## Spieldateien: die Store-Installation oder eine Kopie

Standardmäßig läuft das Spiel aus seiner Store-Installation, dem Ordner, in dem Steam, Heroic oder Legendary es ablegt. Eine Instanz kann stattdessen aus einer Kopie dieses Ordners innerhalb der Instanz laufen. Die Mods liegen über der Kopie, und die Store-Installation ist nur noch der Ort, an dem Eidos das Ergebnis einhängt. Der Store startet das Spiel weiterhin, daher behält Steam sein Overlay, seinen Shader-Cache und seine Cloud-Spielstände.

Die Karte Game files oben im Tab Diagnostics zeigt, welche Dateien die Instanz nutzt, mit der Spielversion der Kopie und der Store-Installation, und schaltet zwischen ihnen um. In einem Terminal gibt `eidos stock-game <instance>` dieselben Fakten aus, und `use <folder>`, `store`, `freeze` und `delete --yes` tun, was die Knöpfe der Karte tun. `freeze` gibt die Größe und die Art der Kopie aus, und `freeze --yes` kopiert.

**Freeze this version** kopiert die Store-Installation nach `<instance>/Stock Game` und startet das Spiel von dort, sodass ein Store-Update keine Script-Extender-Plugins mehr bricht: Das Update ändert die Store-Installation, nicht die Kopie. Der erste Klick zeigt die Größe und was das Dateisystem damit macht, der zweite kopiert. Auf einem Dateisystem, das Blöcke teilen kann (Reflink), etwa btrfs oder XFS, belegt die Kopie fast keinen Platz, bis der Store eine Datei ändert. Auf ext4, oder wenn Spiel und Instanz auf verschiedenen Dateisystemen liegen, ist es eine vollständige Kopie, so groß wie der Spielordner. Eidos lehnt ab, solange Steam das Spiel als beschäftigt oder nicht vollständig installiert meldet, solange ein Spiel oder Tool der Instanz läuft, wenn die Instanz schon die Spielkopie einer Liste, eine eingefrorene oder eine unfertige Kopie enthält und wenn das Dateisystem weniger freien Platz hat, als eine vollständige Kopie braucht. Bei Heroic- und Legendary-Kopien wird nicht auf ein laufendes Update geprüft. Stop, ein Fehler oder ein Steam-Update des Spiels während des Kopierens hinterlässt `Stock Game.eidos-tmp`, das die Karte zum Löschen anbietet; dieser Ordner wird nie benutzt.

Eine Kopie verbirgt jede Datei, die der Store nach dem Kopieren im Spielordner hinzufügt, ändert oder entfernt: neue Creation-Club-Dateien, eine neue ausführbare Datei, neue DLLs. Das Spiel, die Plugin-Liste, LOOT und die Health-Prüfungen lesen alle die Kopie. Was außerhalb des Spielordners passiert, verbirgt sie nicht: Ein Update kann beim nächsten Start weiterhin Runtimes in das Proton-Präfix installieren, und Steam kann das Spiel auf ein anderes Proton umstellen.

**Return to the store version** nimmt die Kopie nach einem zweiten Klick aus dem nächsten Start. Die Kopie bleibt auf der Platte, und **Delete the copy** entfernt sie, ebenfalls nach einem zweiten Klick. Delete wird abgelehnt, solange die Instanz noch die Kopie nutzt. Eidos löscht nie den eigenen Ordner `Stock Game` oder `Game Root` einer Liste. Fehlt der Ordner der Kopie oder enthält er die ausführbare Datei des Spiels nicht mehr, werden Starts abgelehnt und Diagnostics nennt den Ordner, sodass nie unerwartet die Store-Version läuft.

Morrowind sowie Oblivion mit `bUseMyGamesDirectory=0` halten ihre Plugin-Liste im Spielordner, und Eidos startet sie noch nicht aus einer Kopie.

## Eine Nexus-Collection installieren

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # sie lesen und sagen, was geschehen würde
eidos collection skyrimse rqhcxy --no-optional  # die Mitglieder überspringen, die sie als optional markiert
```

Im Fenster fügen Sie den Link unter **File -> Open a Nexus collection...** ein oder
klicken auf der Website auf eine. Eidos listet die Mitglieder der Revision, jedes gegen
diese Instanz abgeglichen: installiert, heruntergeladen oder fehlend. *Install this
collection* führt das ganze Rezept auf einem Worker-Thread aus, mit demselben
Fortschrittsbalken, den das Packen und das Auspacken benutzen, und der Balken wird zum
Bericht, wenn er fertig ist. Das Fenster und `eidos collection` führen dieselbe
Installation aus.

Ohne Link öffnet sich **File -> Open a Nexus collection...** auf dem Tab **Browse**. Er
listet die Collections, die für das Spiel der geöffneten Instanz veröffentlicht sind, 20
auf einmal, nach den meisten Downloads, der letzten Aktualisierung oder den neuesten, und
das Suchfeld grenzt die Liste auf Collections ein, deren Name oder Text passt. Jede Zeile
nennt den Autor, die neueste Revision, die Zahl der Mods, die Downloads und ein Datum.
**View** öffnet die neueste Revision dieser Collection auf dem Tab **Link**, genau wie das
Einfügen ihres Links. Als Erwachseneninhalt markierte Collections werden ausgelassen,
sofern Ihr Nexus-Konto keine Erwachseneninhalte anzeigt, Collections ohne Einstufung werden
immer ausgelassen, und die Zeile unter der Liste zählt beide. Zum Durchsuchen brauchen Sie
ein angemeldetes Nexus-Konto, wie für den Rest des Bereichs.

Eine Collection öffnet sich nur gegen eine Instanz ihres eigenen Spiels. Eine
Skyrim-Collection, die bei geladener Fallout-4-Instanz geöffnet wird, wird namentlich
abgelehnt, statt ihre Mitglieder gegen die falsche Mod-Liste abzugleichen.

### Was Eidos aus einer Collection liest

Fast nichts von einer Collection ist über die Nexus-API sichtbar. Die
Installationsreihenfolge, die Antworten, die der Autor dem skriptgesteuerten Installer
jeder Mod gegeben hat, welche Mod einen Dateikonflikt gewinnt, welche Plugins wo laden und
die Binärpatches stecken alle in einer `collection.json` im Archiv der Collection selbst.
Dieselben Mods herunterzuladen und sie mit ihren Standardoptionen zu installieren ergibt
ein anderes Spiel.

Eidos lädt das Archiv herunter, liest `collection.json` und wendet es an:

| Die Collection sagt | Eidos tut |
| --- | --- |
| `phase` | installiert in dieser Reihenfolge, optionale Mitglieder zuletzt |
| `choices` | spielt die Antworten des Autors zu jedem FOMOD nach und sagt es, wenn es das nicht kann |
| `modRules` | sortiert die Mod-Liste um, damit die richtige Mod jede Datei gewinnt |
| `plugins` / `pluginRules` | mischt die LOOT-Regeln in Ihre userlist und sortiert |
| `INI Tweaks/` | installiert sie als eigene Mod |
| `tools` | sagt Ihnen, welche Tools es erwartet; legt nichts an |

Mitglieder werden aktiviert, während sie installiert werden, am Ende der Mod-Liste in
Installationsreihenfolge, bevor `modRules` sie verschiebt. Ein Mitglied, das nicht in der
Mod-Liste stünde, würde nicht laden, und der Bericht würde es nicht zeigen.

Der Zustand wird nach jedem Mitglied in
`<instance>/collections/<slug>-<revision>.state.json` geschrieben. Er liegt neben dem
Ordner dieser Revision, nie darin, sodass eine Collection keine eigene Zustandsdatei
mitliefern kann, und eine bei Mitglied 147 von 200 abgebrochene Installation macht bei 147
weiter. Er wird unter einem temporären Namen geschrieben und an seinen Platz umbenannt,
sodass ein Abbruch die alte Aufzeichnung oder die neue hinterlässt und nie die Hälfte von
einer der beiden. Lässt er sich einmal nicht lesen, hält Eidos an und sagt es, statt die
ganze Collection von vorn zu beginnen. Führen Sie denselben Befehl noch einmal aus.

Eine neuere Revision einer Collection zu installieren, die Sie bereits haben, übernimmt die
Ordner der vorherigen Revision: Ein unverändertes Mitglied wird an Ort und Stelle geprüft,
ein Mitglied, dessen Datei der Autor aktualisiert hat, wird in seinem alten Ordner ersetzt,
und ein Mitglied, das die neue Revision fallen gelassen hat, bleibt installiert und wird im
Bericht benannt, damit Sie es deaktivieren oder entfernen. Ein Ordner, den ein anderes
Profil aktiviert, wird nie übernommen, denn der mods-Ordner ist gemeinsam und jenes Profil
würde sich unter Ihnen ändern: Die neue Revision installiert stattdessen ihre eigene Kopie
daneben, und der Bericht bittet Sie, die alte in diesem Profil zu deaktivieren, nie, sie zu
entfernen. Die Rückkehr zu einer älteren Revision holt deren Ordner auf dieselbe Weise
zurück oder installiert frische Kopien derer, die sie nicht nehmen kann. Wird ein Mitglied
so ersetzt und haben Sie in seinem Ordner Dateien geändert oder hinzugefügt, wird der alte
Ordner vorher als `<name>_backup` daneben aufbewahrt, der nie ins Spiel geladen wird, und
der Bericht nennt ihn.

Ein Überbleibsel einer früheren Revision, das aktiviert bleibt, verhindert, dass die
Installation als die zählt, die der Autor gebaut hat; der Hinweis verschwindet, sobald Sie
es deaktivieren. Ein Mitglied, das Sie mit `--no-optional` übersprungen haben, wird nicht
übernommen und auf dieselbe Weise benannt. Schlägt die Übernahme selbst fehl, hält der Lauf
mit *previous revision: its folders could not be taken over* und dem Grund an; beheben Sie
das und führen Sie den Befehl noch einmal aus.

### Kostenlose Konten und Mitglieder, die Sie selbst holen

Ein kostenloses Nexus-Konto kann die Mitglieds-Mods nicht holen. Nexus gibt kostenlosen
Konten über die API keine Download-Links: Jede Datei braucht den seiteneigenen Knopf "Mod
Manager Download". Das Archiv der Collection lädt mit einem kostenlosen Konto herunter, Sie
können also das ganze Rezept lesen, aber jedes Mitglied wird mit seiner genauen Seiten-URL
gemeldet, damit Sie es selbst holen, und Wiederholen ändert daran nichts. Holen Sie sie
über den seiteneigenen Knopf und führen Sie den Befehl noch einmal aus: Nach diesen
Mitgliedern wird bei jedem Lauf in `downloads/` gesucht, sodass die Collection in dem Maße
fertig wird, wie Sie sie beliefern.

Im Fenster bekommt ein kostenloses Konto im Collection-Bereich statt des Sammelabrufs den
Knopf *Fetch the N missing required files, one by one*. Er öffnet die Nexus-Seite der
ersten fehlenden Pflichtdatei, wartet, bis diese Datei in `downloads/` ankommt, und öffnet
dann die nächste. Den Download-Knopf der Seite drücken Sie weiterhin bei jeder Datei selbst.
Der Bereich zeigt den Stand (*3 of 12*) mit *Skip* und *Stop*. Kommt stattdessen eine andere
Datei derselben Mod an, nennt der Bereich beide Datei-IDs und wartet weiter auf die
vorgegebene. Optionale Mitglieder gehören nicht dazu und behalten ihren eigenen Knopf
*Open*.

Eidos wendet außerdem Binärpatches an (jeder wird gegen seine aufgezeichnete CRC geprüft),
Datei-Überschreibungen (die genannten Dateien werden wie in MO2 mit `.mohidden`
ausgeblendet) und Mitglieder, deren Dateien im Archiv der Collection selbst mitreisen
(`bundle`). Mitglieder, die die Collection von Ihnen von Hand geholt haben will (`browse`,
`manual`), tragen die eigenen Anweisungen des Autors bis in den Bericht. Ein Mitglied,
dessen Name schon eine Ihrer Mods ist, wird neben ihr unter einem freien Namen installiert,
nie über sie, und der Bericht sagt, welches.

### Der Bericht

Ein Mitglied, bei dem sich nicht alle Installer-Antworten nachspielen ließen, bekommt eine
eigene Zeile, *installed, but not the way the collection asks*, getrennt von den
Mitgliedern, die so installiert wurden, wie der Autor sie gebaut hat, weil die Dateien auf
der Platte von denen des Autors abweichen. Der Bericht listet außerdem Sortierregeln, die
nichts in der Collection benannt haben, Mods, die in einer Schleife widersprüchlicher
Regeln gefangen sind, LOOT-Gruppen, die es nicht gibt, und jeden Teil des Manifests, den
diese Eidos-Version nicht versteht.

### Die Collection-Reihenfolge beibehalten

Eidos behält die Regeln jeder installierten Collection nach der Installation. Wenn Sie
später zwei ihrer Mods gegen eine Reihenfolgeregel (vor oder nach) verschieben, zeigt der
Tab Diagnostics eine Karte für diese Collection, die das erste solche Paar nennt und die
Regeln zählt, die nicht mehr gelten. Nur Mods, die im aktuellen Profil aktiviert sind,
zählen, und Regeln in einer Schleife werden übersprungen, weil keine Reihenfolge sie
erfüllen kann. **Restore collection order** bringt die Mods der Collection zurück in eine
Reihenfolge, die jede Regel erfüllt, behält Ihre eigene Reihenfolge überall, wo die Regeln
es zulassen, und lässt die Mods, die nicht zur Collection gehören, an ihrem Platz. Strg+Z
macht es rückgängig. `eidos check` listet denselben Befund als
`collection_order_broken`. Die Prüfung liest das Rezept der Revision in `collections/`,
daher wird eine Collection, deren zwischengespeichertes Rezept entfernt wurde, nicht
geprüft.

## Eine Instanz auf einen anderen Rechner umziehen

`eidos pack` schreibt eine vollständige Instanz (jede Mod, die Ladereihenfolge, alle
Profile, das Overwrite mit Ihren Spielständen darin und die Archive, aus denen alles
installiert wurde) in eine einzige Datei. `eidos unpack` packt sie wieder aus:

```sh
eidos pack skyrimse ~/backup.eidos                  # oder einen Ordner angeben: benannt nach Spiel und Datum
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # auf dem anderen Rechner
```

`eidos pack --dry-run` listet genau auf, was hineinkäme, was nicht und warum, und wie viel
Platz es braucht, ohne irgendetwas zu schreiben. `eidos unpack --info` tut dasselbe für eine
Datei, die Sie bereits haben.

Falls 7-Zip etwas nicht lesen konnte oder der Durchlauf etwas überspringen musste, das Sie in
einer Sicherung erwarten würden (einen symbolischen Link, einen Ordner, den er nicht lesen
konnte, einen Namen, der sich 7-Zip nicht sicher übergeben lässt), wird das Archiv trotzdem
unter dem gewählten Namen geschrieben, aber `eidos pack` endet mit 1 und sagt, was fehlt,
sodass ein Skript, das `eidos pack ... && ...` ausführt, dort anhält.

Eine `.eidos`-Datei ist ein 7-Zip-Archiv mit eigener Endung. 7-Zip wird ohnehin gebraucht, um
eine Mod zu installieren, das fügt also keine Abhängigkeit hinzu, und die Sicherung öffnet
sich ohne Eidos in jedem Archivprogramm, das 7-Zip liest. Sie ist mit LZMA2 auf `-mx1`
komprimiert, auf rund 43 % des Originals bei einer texturlastigen Liste; `-mx9` braucht viel
länger und spart ein paar Punkte mehr. Texturen und Meshes sind bereits komprimierte
Formate, daher findet ein größeres Wörterbuch sehr wenig, was sich noch komprimieren ließe.
`--level` (0, 1, 3, 5, 7 oder 9) wählt eine andere Stufe, und `--no-downloads` lässt
`downloads/` weg.

Auf der Standardstufe 1 landen die Dateien in soliden Blöcken von höchstens 16 MB. Eine
einzelne Datei aus einer 70 GB großen Sicherung zu ziehen dekodiert höchstens den Block, in
dem sie liegt, nie alles davor, und das Packen von Spielinhalten läuft 2- bis 5-mal
schneller als mit jeder Datei einzeln komprimiert (das Entpacken 2- bis 4-mal). Der Preis
zeigt sich bei Schäden: Ein einziges defektes Byte kann jede Datei in seinem Block
mitnehmen, bis zu 16 MB benachbarter Dateien. Die weiter unten beschriebenen
Wiederherstellungsdaten decken das ab. Alle anderen Stufen bleiben nicht solide: Jede Datei
ist ein eigener Datenstrom, und ein defektes Byte kostet nur diese eine Datei. Stufe 0
speichert die Dateien unkomprimiert. Ab Stufe 3 arbeitet LZMA2 ohnehin in Stücken von 16 MB
oder mehr, und solide Blöcke waren dort in der Messung langsamer: Auf Stufe 5 dauerte das
Packen von Meshes 1,4-mal und das Entpacken von Texturen 6,8-mal so lange.

Das Packen nimmt die Sperre der Instanz, es kann also nicht laufen, während das Spiel oder
ein anderes Eidos in diese Instanz schreibt, und nichts ändert die Instanz, während sie
gelesen wird. Die Sicherung wird unter einem `.part`-Namen geschrieben, auf die Platte
gebracht und mit `7z t` vollständig zurückgelesen, bevor sie den Namen bekommt, den Sie
gewählt haben. Diese Prüfung dekodiert jeden Block und vergleicht jede Prüfsumme, also
dieselbe Arbeit, die ein Entpacken auf dem anderen Rechner leistet. Sie zeigt, was die Platte
direkt nach dem Schreiben zurückgegeben hat, nicht, dass das Laufwerk es behält; dafür sind
die Wiederherstellungsdaten weiter unten da. Schlägt sie fehl, wird die `.part`-Datei
gelöscht, unter Ihrem Namen wird nichts geschrieben, und `eidos pack` zitiert die Zeile, in
der 7-Zip den Schaden gefunden hat. Ein abgebrochenes oder fehlgeschlagenes Packen
hinterlässt eine `.part`-Datei oder nichts, nie eine `.eidos`-Datei, die vollständig aussieht
und es nicht ist.

Ist par2cmdline installiert (`sudo pacman -S par2cmdline` unter Arch,
`sudo apt install par2` unter Debian und Ubuntu, `sudo dnf install par2cmdline` unter
Fedora), schreibt `eidos pack` zusätzlich PAR2-Wiederherstellungsdaten neben das Archiv:
`backup.eidos.par2` und einige `backup.eidos.volNN+MM.par2`-Dateien, zusammen 5 % des
Archivs plus ein kleiner Index. Lassen Sie sie unter den Namen, mit denen sie geschrieben
wurden, im selben Ordner wie die `.eidos`-Datei und kopieren Sie sie mit ihr: Eine Sicherung,
die ohne sie umzieht, hat nichts, woraus sie repariert werden könnte, und die
Wiederherstellungsdaten halten den Namen des Archivs fest, sodass eine umbenannte Sicherung
nicht mehr zu ihnen passt. `eidos unpack` findet sie über den Namen und prüft das Archiv
damit, bevor irgendetwas entpackt wird, auch eine Sicherung, deren Ende so abgeschnitten ist,
dass 7-Zip sie nicht öffnen kann. Sind ein paar Bytes kaputt, repariert par2 das Archiv
zuerst und behält die beschädigte Kopie als `backup.eidos.1`, wofür in ihrem Ordner so viel
freier Platz nötig ist, wie das Archiv groß ist; ein Schaden, der über die 5 % hinausgeht,
stoppt das Entpacken, bevor eine einzige Datei geschrieben ist. Wiederherstellungsdateien, die
nicht zum Archiv passen, etwa die einer älteren Sicherung gleichen Namens, werden nicht
verwendet, wenn 7-Zip das Archiv liest und es intakt findet, und das Entpacken sagt das. Auf
einem Rechner ohne par2 läuft das Entpacken trotzdem, mit einer Warnung, dass die
Wiederherstellungsdaten nicht genutzt wurden. `--no-recovery` packt ohne sie, und
`eidos pack --dry-run` sagt, ob sie geschrieben werden. `--force` ersetzt eine vorhandene
Datei gleichen Namens.

### Was nicht darin ist, und warum

| Weggelassen | Warum |
| --- | --- |
| Das Proton-Präfix | Es ist auf die Maschine zugeschnitten (Laufwerkszuordnungen, ein `Z:`-Blick auf *dieses* Dateisystem) und Eidos baut es mit `eidos prereqs <instance> --install` in einer Minute neu auf. Es mitzunehmen würde das Archiv ungefähr verdoppeln, um etwas zu liefern, das der andere Rechner ohnehin neu erzeugen muss. |
| Das Spiel | Eine Instanz moddet ein Spiel, das sie nicht enthält. Installieren Sie es drüben aus Steam. |
| Tools außerhalb der Instanz | xEdit, DynDOLOD und andere Tools sind fremde Programme mit eigenen Installern. Sie werden im Archiv benannt, sodass das Auspacken Ihnen genau sagt, welche auf dem neuen Rechner fehlen. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Aufzeichnungen der Maschine, die die Sicherung erstellt hat. `prereqs.done` würde insbesondere behaupten, die Runtime-Bibliotheken seien bereits in einem Präfix installiert, das gar nicht da ist. |
| `loot/`, außer `userlist.yaml` | Ein Masterlist-Cache, den Eidos bei Bedarf neu holt. Ihre eigenen LOOT-Regeln in `userlist.yaml` lassen sich nicht neu holen, also kommt diese eine Datei mit. |
| `.base/`, `.base-root/` | Leere Mountpunkte, an denen die eigenen Dateien des Spiels während einer Sitzung verstaut werden. |
| `test-run/`, `bisect.json` | Ein [Testlauf](#testläufe), der auf seine Durchsicht wartet, und eine Suche nach der Mod hinter einem Problem zwischen zwei Schritten. Beides gehört zu Läufen auf diesem Rechner. |
| Halb geschriebene Dateien | Ein angehaltener Download (`*.unfinished`), ein atomares Schreiben im Flug (`*.eidos-tmp*`). |
| Symbolische Links | 7-Zip würde einem folgen und kopieren, worauf er zeigt, was bei einem absoluten Link heißt, einen fremden Baum in Ihre Sicherung zu ziehen. Sie werden stattdessen gemeldet, und die Sicherung gilt als unvollständig: Ein auf ein anderes Laufwerk verlinktes `mods/` oder `downloads/` wird nicht gepackt. Um bewusst ohne ein verlinktes `downloads/` zu sichern, geben Sie `--no-downloads` an (oder entfernen Sie im Fenster den Haken bei *Include downloads/*); ein bewusstes Weglassen zählt nicht als fehlend. |

Eine Spielkopie in der Instanz (`Stock Game`) wird wie jeder andere Ordner gepackt, daher ist
das Backup einer eingefrorenen Instanz etwa um die Größe des Spiels größer. Eine unfertige
Kopie (`Stock Game.eidos-tmp`) bleibt draußen.

Jeder dieser Punkte landet mit seinem Grund in der `eidos-backup.ini` in der Wurzel des
Archivs, sodass das Lesen dieser Datei zeigt, was weggelassen wurde. Dieselbe Datei hält
fest, wo die Instanz früher lag, welches Profil aktiv war, wie groß sie ausgepackt wird und
welche Tools der neue Rechner brauchen wird.

Download-Aufzeichnungen (`downloads/*.meta`) kommen mit geleerter `url=`-Zeile hinein. Diese
URL ist ein signierter Nexus-Link, der binnen Stunden aufhört zu funktionieren und die
`user_id` des Kontos trägt, das die Datei heruntergeladen hat, und eine Sicherung wird
vielleicht an jemand anderen weitergegeben. Die Mod-ID, die Datei-ID und die Version bleiben,
sodass sich der Download wiederfinden lässt.

### Im Fenster

Beides steckt im Menü **File**: *Pack this instance...* und *Unpack a backup...*. Das
Auspacken liegt außerdem auf dem Willkommensbildschirm, unter der Liste der Instanzen,
sodass ein Rechner ohne jede Instanz eine auspacken kann.

Bevor irgendetwas geschrieben wird, zeigt der Pack-Dialog, wie viele Dateien, wie groß, wie
viel davon `downloads/` ist, worauf das Archiv ungefähr hinausläuft und welche Tools benannt,
aber nicht mitgenommen werden. Sein Kästchen *Write recovery data* ist angehakt, wenn par2
installiert ist, und ausgegraut mit dem zu installierenden Paket, wenn nicht. Der
Unpack-Dialog liest das Manifest der Sicherung, sobald Sie die Datei auswählen, und kann
Ihnen daher sagen, welches Spiel sie enthält, wann sie erstellt wurde, wo sie früher lag und
welche ihrer Tools hier fehlen, bevor Sie einen Ordner wählen.

Beide laufen auf einem Worker-Thread mit Fortschrittsbalken, und der Balken wird zum
Bericht, wenn sie fertig sind.

### Was das Auspacken repariert

Eine bloße Kopie des Ordners würde jeden Tool-Knopf auf einen Pfad auf dem alten Rechner
zeigen lassen. `eidos unpack` schreibt die Werte um, die die alte Instanzwurzel nennen, und
nur diese: `exe`, `workdir` und die nummerierten `arg`-Schlüssel in `tools.ini` sowie
`installationFile` in der `meta.ini` jeder Mod. Ein Tool, das nie innerhalb der Instanz lag
(`/opt/xedit/SSEEdit.exe`), bleibt genau so, wie es war, und wird als etwas aufgeführt, das
neu zu installieren ist; Eidos rät nicht, wo dieses Tool auf dem neuen Rechner liegt. Es
korrigiert außerdem, ob die Instanz sich selbst zentral oder portabel nennt, damit spätere
Befehle sie dort suchen, wo sie jetzt ist.

Das Auspacken lehnt einen Ordner ab, der nicht leer ist (geben Sie `--force` an, wenn Sie ihn
wirklich überschreiben wollen), eine Platte, die schon für die Angabe des Manifests selbst zu
klein ist, ein Archiv, das ein neueres Eidos als Ihres erstellt hat, und jedes Archiv mit
einem Eintrag, der außerhalb des von Ihnen gewählten Ordners geschrieben würde. Ohne Ordner
kommt die Sicherung einer globalen Instanz an den zentralen Ort für ihr Spiel zurück; eine
portable braucht einen Ordner.

Danach:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # das Proton-Präfix neu aufbauen
eidos play /mnt/games/EidosSkyrim
```

## Das Fenster

Starten Sie `eidos-gui` oder die Datei `eidos-linux`, die Colony installiert.

Der erste Start öffnet einen Assistenten wie den von MO2: Willkommen, Instanztyp (portabel
oder global), Spiel, Name und Ort, Zusammenfassung, dann das Hauptfenster. Der
Willkommensbildschirm listet außerdem jede bekannte Instanz, global und portabel, zuletzt
benutzte zuerst, zum Öffnen mit einem Klick, ist also zugleich der Instanzumschalter. Zeigt
man den Assistenten auf einen Ordner, der bereits eine Instanz enthält, öffnet er diese
Instanz, wie sie ist, statt eine neue darüber anzulegen, und ein Ordner, der zu einem anderen
Spiel gehört, wird abgelehnt.

Das Hauptfenster hat zwei Bereiche: die Mod-Liste und die Tabs Data, Plugins, Conflicts,
Overwrite, Archives, Saves, Downloads und Diagnostics, mit einem Run-Knopf, einem Test-run-Knopf
([Testläufe](#testläufe)) und einer Auswahl des Startziels. Sie filtern die Mod-Liste,
wählen aus, sortieren um, gruppieren mit Trennern, grenzen nach Kategorie ein und klicken
für Aktionen mit der rechten Maustaste. Neben "nach ganz oben" und "nach ganz unten" hat das
Mod-Menü MOs gezielte Verschiebungen: über die erste kollidierende Mod, unter die letzte,
auf eine bestimmte Priorität oder in die Gruppe eines Trenners.

Die Profilauswahl wechselt Profile. Rechtsklick auf ein Profil bietet Rename, Copy to new...,
Desktop shortcut, Import from MO2... und Löschen, das zwei Klicks braucht und das aktive
oder das letzte Profil ablehnt. Import from MO2... übernimmt die Mod-Reihenfolge und die
Ladereihenfolge eines MO2-Profils. Desktop shortcut schreibt einen Desktop-Eintrag, der
`eidos tool <instance> run "<game>" --profile <name>` ausführt: Er macht dieses Profil zum
aktiven und bittet den Store, das Spiel zu starten (Einzelheiten in
[tools.md](tools.md#an-ein-tool-herankommen-verstecken-anheften-und-eine-desktop-verknüpfung)).

Das Menü **File** startet außerdem winecfg, regedit, den Wine-Dateiexplorer und die
Deinstallation im Präfix des Spiels, und Prefix drive_c öffnet das Laufwerk `C:` des Präfix;
siehe [tools.md](tools.md#wines-eigene-programme-im-prefix-des-spiels). *Instances...* listet
die Instanzen auf, die Eidos kennt; Forget nimmt eine aus dieser Liste und fasst auf der
Platte nichts an. *Export the mod list...* schreibt die Liste im CSV-Format von MO2, wie
`eidos export`.

### Preferences

Klicken Sie oben links auf **Eidos**, um die Preferences anstelle der beiden Bereiche zu
öffnen, und klicken Sie erneut darauf (oder auf **Close**), um zurückzukehren. General,
Appearance und Accessibility kommen zuerst, wie in jedem Colony-Programm, danach Mod list,
Nexus und About. Jede Änderung wird sofort angewendet und gespeichert; es gibt keinen
Save-Knopf.

- General: das Spiel, das Eidos öffnet, die Fenstergröße, die Sprache der Oberfläche, der
  Tools-Ordner und wie ein Spiel ausgeführt wird. Lock the window while a game runs ist
  standardmäßig an. Start the game at once when a store opens Eidos und die
  Desktop-Benachrichtigung, die meldet, wann eine lange Aufgabe endet, während das Fenster im
  Hintergrund ist, sind standardmäßig aus. Das Fenster spricht Englisch und Französisch;
  System folgt `LC_ALL`, `LC_MESSAGES` oder `LANG`, und die Änderung gilt sofort. Die
  Kommandozeile `eidos` antwortet immer auf Englisch.
- Appearance: das Theme (26 Familien, 59 Paletten, standardmäßig das Pergament von Eidos), die
  Akzentfarbe, die Textgröße, Effekte und eine Live-Vorschau.
- Accessibility: hoher Kontrast, eine legasthenikerfreundliche Schrift, reduzierte Bewegung
  und eine Lesetextgröße, die sich mit der aus Appearance multipliziert.
- Mod list: die Auto-Scroll-Geschwindigkeit, während eine Mod an den oberen oder unteren
  Rand der Liste gezogen wird.
- Nexus: Anmeldung, der Downloads-Ordner, der `nxm://`-Handler und sein Knopf Register,
  bevorzugte Server, Install downloads when they finish und Offline mode.
- About: wo die Dateien liegen, die Tastenkürzel, die Version und die Release-Prüfung.

Das Aussehen und die Sprache der Oberfläche liegen in
`~/.config/Colony/Eidos/preferences.json`. Die übrigen Einstellungen, die auch die
Kommandozeile liest (das Standardspiel, der Tools-Ordner, die Nexus-Server, der Offline-Modus
und der Rest), liegen in `settings.ini` daneben, und die Nexus-Sitzung in `nexus.ini`. Das
Theme, das ein älteres Eidos in `settings.ini` gespeichert hat, wird beim ersten Mal
übernommen.

**About -> Version** enthält die Release-Prüfung, standardmäßig an. Beim Start, höchstens
einmal am Tag, fragt Eidos GitHub nach dem neuesten Release von `Project-Colony/Eidos`. Ist es
neuer als die laufende Kopie, zeigt die Menüleiste nach Help `Eidos <version> is available:
release notes`, und der Link öffnet die Seite dieses Releases mit seinem Changelog. Eidos lädt
sich nie selbst herunter und ersetzt sich nie selbst. Im Offline-Modus entfällt die Prüfung.
Für eine Kopie, die ein Paketmanager unter `/usr` installiert hat, ist sie aus, denn der
Paketmanager aktualisiert sie; `/usr/local`, wohin `install.sh --system` Eidos legt, gilt als
Ihre eigene Installation.

### Spalten, Sortierung und Gruppierung

Die Liste zeigt von Haus aus vier Spalten (Category, Content, Version und Flags, die, die MO2
zeigt) und bietet acht an: Category, Content, Version, Author, Installed, Nexus id, Game,
Flags. Haken Sie sie im View-Menü an. Sind alle acht an, bleibt für die Namensspalte wenig
Platz.

Klicken Sie auf eine Überschrift, um danach zu sortieren, erneut, um umzukehren, und ein
drittes Mal, um zur Ladereihenfolge zurückzukehren. Die Liste lässt sich nur in der
Ladereihenfolge ziehen: Eine Ablageposition spricht die echte Liste an, und unter einer
Sortierung ist der Platz einer Zeile auf dem Bildschirm nicht ihr Platz in dieser Liste.
Solange eine Sortierung aktiv ist, werden die Einfügestreifen nicht gezeichnet und ein Ziehen
wird abgelehnt, wie in MO2. Das View-Menü sagt das und bietet den Weg zurück.

Das View-Menü kann die ganze Liste auch gruppieren, nach Kategorie oder nach Herkunft (von
Nexus oder von Hand installiert). Gruppenköpfe sind keine Trenner: Sie lassen sich nicht
umbenennen, einfärben oder verschieben. Sie klappen zu, und ein zugeklappter Kopf behält
seine Anzahl. Trenner sind unter einer Sortierung oder Gruppierung ausgeblendet, weil ein
Trenner den Zeilen vorsteht, die ihm in der Ladereihenfolge folgen, und beide diese Reihenfolge
auf dem Bildschirm ändern.

### Maus und Tastatur

Doppelklick auf eine Mod öffnet Information, Strg+Doppelklick ihren Ordner,
Umschalt+Doppelklick ihre Nexus-Seite. Strg+F setzt den Cursor ins Filterfeld. Einen
Buchstaben zu tippen springt zur nächsten Mod, die damit beginnt, und ihn erneut zu drücken
geht die übrigen durch, statt an der ersten zu kleben. Keine davon landet auf einer Zeile,
die der Filter, ein zugeklappter Trenner oder eine zugeklappte Gruppe verbirgt, sodass die
nächste Leertaste nie eine Mod umschaltet, die Sie nicht sehen.

"Collapse others" im Menü eines Trenners klappt jede Gruppe außer dieser zu. Während eines
Ziehens öffnet das Verweilen auf einer zugeklappten Gruppe sie, sodass die Mod hineingelegt
werden kann; Vorbeistreifen öffnet sie nicht.

Strg+Z macht die letzte Änderung an der Mod-Liste oder der Ladereihenfolge rückgängig: ein
Häkchen, ein Verschieben, ein Sammel-Deaktivieren, eine LOOT-Sortierung. Strg+Umschalt+Z
stellt sie wieder her. Das Fenster behält 50 Schritte und vergisst sie bei F5, am Ende eines
Spiel- oder Tool-Laufs, beim Profilwechsel und wenn ein anderer Eidos-Prozess die Liste
ändert. Installieren, Umbenennen oder Entfernen einer Mod ändert die Ordner auf der Platte,
daher wird ein Rückgängigmachen über einen solchen Schritt hinaus abgelehnt.

### Die Liste filtern

Der Knopf Filters öffnet eine Zeile je Kriterium: aktiv, im Konflikt, Update verfügbar, ohne
Kategorie, auf Nexus bewertet (endorsed), verfolgt (tracked), mit Nexus-ID, mit versteckten
Dateien, ohne gültige Spieldaten, für ein anderes Spiel, mit Notizen, und eine Zeile je Art
von Inhalt (Plugins, BSA- oder BA2-Archive, Texturen, Meshes, Skripte,
Script-Extender-Plugins, Interface-Dateien, Sound, FaceGen). Ein Klick schaltet eine Zeile
reihum zwischen aus, "only" und "except". Die oberste Zeile wechselt zwischen Mods, die alle
eingeschalteten Kriterien erfüllen, und Mods, die mindestens eines davon erfüllen. Das
Namensfeld und die Kategorienliste schränken das Ergebnis immer zusätzlich ein. Die Zeile für
versteckte Dateien sieht nur aktivierte Mods an.

### Was die Liste Ihnen über eine Mod sagt

Zwei beratende Kennzeichen, beide ein Glyph mit der Erklärung beim Überfahren. No valid game
data bedeutet, dass oben in der Mod nichts danach aussieht, als würde dieses Spiel es laden;
womöglich müssen ihre Ordner eine Ebene hochgezogen werden, oder es ist keine Mod für dieses
Spiel. Another game bedeutet, dass die `meta.ini` der Mod ein anderes nennt. Keines blockiert
etwas: Die Mod wird weiterhin ausgebracht. "Mark as valid" im Zeilenmenü bringt beide über
MO2s eigenen `validated=`-Schlüssel zum Schweigen, sodass eine Mod, die in einem Manager als
gültig markiert wurde, im anderen nicht markiert wird.

Die Layout-Prüfung markiert eine Mod nur, wenn sie sicher ist: ein `Root/`-Baum, ein
unlesbarer Ordner und ein leerer Ordner zählen alle als gültige Spieldaten.

### Eine Mod deaktivieren, die Ihre Spielstände oder andere Mods brauchen

Bevor eine Mod deaktiviert oder entfernt wird, liest Eidos die Plugin-Listen der drei
neuesten Spielstände des Profils. Nutzt einer davon ein Plugin aus dieser Mod, nennt eine
Leiste über der Liste die Plugins und wartet auf Disable anyway oder Cancel (Esc bricht
ebenfalls ab). Remove fügt dieselben Namen seiner Meldung zum erneuten Klicken hinzu. Gelesen
werden nur Spielstände von Skyrim und Fallout 4; andere Spiele deaktivieren ohne Rückfrage.

Die Informationen einer Mod, Tab General, listen die Mods auf, die sie braucht, und die
Mods, die sie brauchen, gelesen aus den Mastern der aktivierten Plugins. Wer eine Mod
deaktiviert, die andere brauchen, sieht dieselbe Leiste mit Disable them too, was auch jede
Mod deaktiviert, die sie braucht, und die Mods, die diese brauchen, oder Keep them enabled.
Die Karte für fehlende Master im Tab Diagnostics hat Enable what provides them: Sie aktiviert
die Mod, die jeden fehlenden Master enthält, und hakt ein Master-Plugin an, das vorhanden,
aber nicht angehakt ist.

### Eine Mod sichern, bevor Sie sie anfassen

"Back up this mod" kopiert ihren Ordner beiseite als `<name>_backup` (dann `_backup2` und so
weiter; eine Sicherung ersetzt nie die vorherige). Die Kopie ist keine Mod: Ihr Häkchen tut
nichts, und sie trägt nichts zur zusammengeführten Sicht bei, denn sie anzuhaken würde zwei
Kopien einer Mod übereinander ausbringen. "Restore this backup over the mod" stellt sie in
zwei Klicks zurück; der aktuelle Inhalt wird zuerst beiseitegelegt und erst verworfen, wenn
die Kopie gelungen ist.

**Backups** in der Werkzeugleiste hält Wiederherstellungspunkte der Mod-Liste des Profils,
seiner Ladereihenfolge (`plugins.txt` und `loadorder.txt` zusammen) und seiner Spiel-INIs,
je 10, in `<profile>/backups/`. Back up now fügt einen hinzu, und Restore sichert zuerst den
aktuellen Zustand, sodass sich eine falsche Wahl rückgängig machen lässt. Eidos legt auch vor
jedem Lauf von BethINI einen INI-Wiederherstellungspunkt an (siehe
[tools.md](tools.md#bethini-pie)).

### Die Tabs

**Data** ist ein Baum der zusammengeführten Sicht, jeweils um eine Ebene ausgeklappt: Das
Öffnen eines Ordners liest diesen Ordner in jeder Schicht, die ihn hat, nie jede aktivierte
Mod auf einmal. Er wird aus demselben Schichtenstapel gebaut, den das Mount bedient, sodass
Whiteouts und versteckte Dateien beachtet werden und der Tab zeigt, was das Spiel sehen wird.
Filtern Sie ihn nach Namen, grenzen Sie ihn auf umstrittene Dateien ein, klären Sie mit den
Spalten Size und Modified, was wo liegt, und zeigen Sie jede Zeile per Reveal im
Dateimanager. Fährt man über eine Datei, listet er die anderen Mods auf, die sie mitbringen.
Die `meta.ini`, `readme.txt`, `fomod/` und `Root/` auf der obersten Ebene einer Mod fehlen in
Data, in diesem Tab wie im Mount: Die ersten drei sind für den Mod-Manager, und `Root/` kommt
stattdessen in den Spielordner. Eine Datei, die das Spiel oder ein Tool unter einem dieser
Namen schreibt, landet im Overwrite und bleibt sichtbar.

Eine `.exe`-Zeile in Data hat Add tool, das den Executables-Editor mit dieser Datei
vorausgefüllt öffnet (gespeichert wird erst mit Save), und Run, das die Datei einmal durch die
zusammengeführte Sicht startet, wie ein registriertes Tool ([Tools](#tools)). Save tree schreibt
den gesamten zusammengeführten Data-Baum in eine Textdatei, eine Zeile pro Datei mit der Mod,
die sie liefert, ohne Ihren Home-Ordner und Benutzernamen, für einen Fehlerbericht.

**Plugins** ist die ESP/ESM/ESL-Ladereihenfolge: umschalten, von Hand umsortieren oder mit LOOT
sortieren und den Bericht danach lesen, dessen Ratschlag-Links sich im Browser öffnen. Seine
Kopfzeile zählt die aktivierten Plugins je Indexbereich gegen das Limit des Spiels, zum
Beispiel `full 231/254, light 512/4096`, und wird orange, sobald einer voll ist. Das Filterfeld
grenzt die Liste nach Namen ein. Leertaste, Strg+A, Umschalt-Bereiche und Activate all shown
und Deactivate all shown im Zeilenmenü wirken dann nur auf die angezeigten Zeilen, und ein
Ziehen wird abgelehnt, bis der Filter geleert ist. Wählt man ein Plugin aus, färbt es seine
Master in der Plugin-Liste und die Mod, die es mitbringt, in der Mod-Liste ein. Plugins, die
LOOT als verschmutzt markiert, bekommen eine Diagnostics-Karte, die sie mit xEdit bereinigen
kann ([tools.md](tools.md#ein-plugin-bereinigen-das-loot-als-verschmutzt-meldet)).

Nach einer LOOT-Sortierung sagt ein vollständiges Plugin, dessen neue Datensätze alle in den
Light-Bereich passen, das beim Überfahren, und sein Zeilenmenü enthält Flag as light. Eidos
schreibt eine Kopie mit Light-Flag in eine neue Mod namens `<plugin> (light)` am Ende der
Mod-Liste (höchste Priorität) und lässt das Original unverändert, sodass das Deaktivieren
dieser Mod den Schritt rückgängig macht. Es lehnt ab, wenn einer der drei neuesten
Spielstände des Profils das Plugin als vollständiges Plugin lädt. Gelesen werden nur
Spielstände von Skyrim und Fallout 4, unter Starfield wird also kein Spielstand geprüft. LOOT
prüft nur Datensatzbereiche: Sehen Sie sich ein Plugin, das Zellen oder Worldspaces
hinzufügt, vor dem Markieren in xEdit an.

Beim Überfahren eines Plugins erscheinen seine Herkunft, sein Autor, die Header- und die
Formularversion, die ersten 300 Zeichen der Beschreibung, seine Master und nach einer
LOOT-Sortierung die Meldungen von LOOT. Bei Skyrim, Skyrim SE, Skyrim VR und Enderal SE liest
das Spiel zusätzlich eine INI, die nach einem geladenen Plugin benannt ist; die Spalte INI
markiert diese Plugins, und ein Klick öffnet die Datei schreibgeschützt.

**Conflicts** erklärt die Gewinner und Verlierer je Datei. **Overwrite** macht in einem Schritt
eine echte Mod aus dem, was das Spiel geschrieben hat.

**Archives** listet jedes BSA oder BA2, das die aktivierten Mods mitbringen, die Mod, aus der
es stammt, und warum es geladen wird: ein aktives Plugin gleichen Namens oder die INI
registriert es. Eines, das nichts lädt, wird als toter Ballast markiert. Gewinnt eine andere
Mod, die ein Archiv gleichen Namens mitbringt, den Konflikt, nennt die Zeile diese Mod.
Extract entpackt ein Archiv in eine neue Mod, die nach ihm ohne seine Endung benannt wird
(`Foo.ba2` ergibt `Foo (extracted)`), und lässt das Archiv in seiner Mod; lose Dateien gewinnen
gegen Dateien in jedem Archiv, was immer die Mod-Reihenfolge sagt.

**Saves** liest den Kopf jedes Spielstands (Charakter, Stufe, Ort, Spielzeit), vergleicht die
darin gespeicherte Plugin-Liste mit der aktuellen und hat einen Knopf, der die benötigten Mods
aktiviert. Spielstände, die ohne Eidos ins Präfix gelangt sind, stehen unter
[Spielstände gehören zum Profil](#spielstände-gehören-zum-profil).

**Downloads** und **Diagnostics** haben weiter unten ihre eigenen Abschnitte.

### Mod-Informationen

"Information..." öffnet einen Dialog je Mod mit den Tabs General, Conflicts, Filetree, INI
Tweaks, Notes, Nexus files und Nexus page. Aus dem Filetree (und aus dem Data-Baum) kann jede
Datei versteckt werden: Sie wird zu `<name>.mohidden` umbenannt, was sie aus der
zusammengeführten Sicht nimmt, ohne sie zu löschen, sodass ein paar verirrte Dateien einer Mod
unterdrückt werden können, ohne Prioritäten zu ändern. Der Filetree beherrscht auch die
gewöhnlichen Dateioperationen: neuer Ordner, umbenennen, löschen, öffnen. Jede lehnt einen Pfad
ab, der kein einfacher Pfad innerhalb dieser Mod ist: kein `..`, kein absoluter Pfad und kein
Symlink auf dem Weg, da einem zu folgen eine Datei außerhalb des Mod-Ordners löschen könnte.
Umbenennen ändert nur den letzten Teil des Pfades, kann also nie eine Datei verschieben, und
lehnt einen bereits vergebenen Namen ab. Löschen braucht zwei Klicks und lässt sich nicht
rückgängig machen.

**View** auf einer beliebigen Zeile im Filetree oder im Data-Baum zeigt eine Vorschau der Datei:
Bilder, Text, DDS-Texturen (wählen Sie Mip, Layer, Face und Kanal) und statische NIF-Modelle
mit ihren Diffuse-Texturen. Die NIF-Vorschau braucht `eidos-nif-preview` neben Eidos oder im
`PATH`; das Release-Bundle, das Arch-Paket und Builds aus dem Quelltext haben es, `eidos-linux`
nicht. Text wird bis 64 KB gelesen und sagt, wo er aufgehört hat.

**INI Tweaks** listet die Fragmente, die eine Mod in ihrem Ordner `INI Tweaks/` mitliefert; die
aktivierten werden beim Start in der Prioritätsreihenfolge in die Spiel-INI des Profils
eingemischt und wieder entfernt, wenn die INIs des Laufs erfasst werden, sodass das
Deaktivieren einer Anpassung ihre Einstellungen entfernt.

Der Tab **Nexus files** listet die Dateien der Mod auf Nexus, die neueste zuerst, und markiert
die hier installierte und die, die sie ersetzt. Diese zweite Zeile hat einen Knopf
**Update**; siehe [Mod-Updates](#mod-updates).

Der Tab **Nexus page** zeigt die Beschreibung der Mod und, die neueste zuerst, das Changelog
jeder Dateiversion auf ihrer Seite. Eidos liest beides beim ersten Öffnen des Tabs über die
GraphQL-API von Nexus, die nicht auf das stündliche Anfragekontingent angerechnet wird, und
behält es, bis Eidos geschlossen wird oder Sie **Refresh** drücken. Beides erscheint als
reiner Text: BBCode- und HTML-Formatierung wird entfernt, die Adresse eines Links steht hinter
seinem Text, und Bilder und Videos fallen weg. **Endorse** und **Track** über dem Text tun
dasselbe wie das Rechtsklickmenü der Mod. Die Seite einer Mod für Erwachsene bleibt verborgen,
solange Ihr Nexus-Konto keine Inhalte für Erwachsene anzeigt, und eine Seite ohne Einstufung
bleibt in jedem Fall verborgen.

### Downloads

Ein Download kann aus der Downloads-Liste auf eine Position in der Mod-Liste gezogen werden,
um ihn mit dieser Priorität zu installieren, und Archive oder Ordner, die aus einem
Dateimanager auf das Fenster fallen gelassen werden, installieren ebenfalls. Datei-Drops
brauchen eine X11- oder XWayland-Sitzung: winit setzt sie nur für X11 um. Downloads lassen
sich anhalten und fortsetzen: Anhalten stoppt die Übertragung und behält die Teildatei, und
Resume fragt Nexus nach einem frischen Link und macht dort weiter, wo es aufgehört hat.

Der Tab Downloads hält die Archive, die Sie heruntergeladen haben. Filtern Sie ihn nach Namen
(auch nach dem freundlichen Mod-Namen, sodass "skyui" `SkyUI_5_2_SE-12604-5-2SE.7z` findet),
sortieren Sie nach neuestem, Namen, Größe oder Zustand und verbergen Sie ein Archiv, mit dem
Sie fertig sind: Die Datei bleibt, und nur die Zeile fällt weg. "Show hidden" holt sie zurück,
und derselbe Knopf hebt das Verbergen wieder auf. "Remove N installed" löscht in zwei Klicks
die Archive der Mods, die Sie bereits installiert haben, und nur die, die der Filter zeigt.

**Preferences -> Nexus -> Install downloads when they finish** ist standardmäßig aus. Ist die
Option an, wird ein Nexus-Download, der bei geöffnetem Fenster fertig wird, ohne Umweg über
den Tab Downloads installiert. Es ist die übliche Installation: Ein FOMOD öffnet seinen
Installer, und ein Name, der schon in der Mod-Liste steht, fragt nach. Archive, die vor dem
Öffnen des Fensters oder vor dem Einschalten der Option in `downloads/` lagen, bleiben
unberührt, ebenso ein Download, dessen Installation Sie abgebrochen haben. Solange im
Collection-Bereich eine Collection offen ist, installiert sich kein Download von selbst, und
einer, der in der Zwischenzeit fertig wird, installiert sich auch später nicht von selbst; die
Collection installiert ihre Mitglieder. Die Datei, auf die ein wartendes Update wartet, wird
vom Update installiert. Auf diese Weise werden nur Nexus-Downloads installiert, und einer, den
der Downloads-Filter verbirgt, wartet, bis der Filter geleert ist. Solange ein Ziehen läuft
oder das Spiel oder ein anderes Eidos die Instanz hält, wartet die Installation.

### Mod-Updates

Der Update-Knopf nahe dem rechten Ende der Werkzeugleiste prüft jede Mod, die eine Nexus-ID
hat. Zeigt die Mod-Seite die Version, die Sie installiert haben, wird nichts weiter gefragt.
Andernfalls liest Eidos die Dateiliste der Mod (eine Anfrage mehr, für den Rest der Prüfung
behalten) und folgt der Update-Kette ab der Datei-ID, die die `meta.ini` der Mod festhält:
Autoren markieren einen neuen Upload als Nachfolger eines älteren, und MO2 liest dieselbe
Kette. Eine neuere Datei in dieser Kette ist das Update. Nennt die Kette keine und ist die
installierte Datei auf der Seite noch aufgeführt, gilt die Mod als aktuell, auch wenn die Seite
eine andere Version zeigt. Die Version der Seite entscheidet nur, wenn keine Datei-ID
festgehalten ist, wenn die installierte Datei entfernt oder archiviert wurde oder wenn Nexus
die Dateiliste nicht geschickt hat.

Jede Mod nutzt die Nexus-Domain des Spiels, das ihre eigene `meta.ini` nennt (`gameName`), nicht
die der Instanz: Die Update-Prüfung, Endorse, Track und Visit on Nexus gehen alle dorthin.
Enderal SE akzeptiert außerdem Mods für `skyrimspecialedition`, sodass eine in einer
Enderal-SE-Instanz installierte Skyrim-SE-Mod auf ihrer Skyrim-SE-Seite geprüft wird. Ein
Mod-Manager-Download-Link geht an die Instanz, die Sie zuletzt benutzt haben, wenn deren Spiel
die Domain des Links akzeptiert. Sonst geht er an eine Instanz des Spiels, dessen eigene Domain
es ist, vor der eines Spiels, das sie nur akzeptiert. `EIDOS_INSTANCE=<folder>` überschreibt
beides.

Track und Untrack im Rechtsklickmenü einer Mod fügen die Mod Ihren verfolgten Mods auf Nexus
hinzu oder entfernen sie daraus, damit die Benachrichtigungen der Website zu dem passen, was
Sie hier verfolgen. Endorse und Abstain im selben Menü bewerten die Mod auf Nexus oder ziehen
die Bewertung zurück; der Endorse-Knopf neben dem Update-Knopf bewertet die ausgewählte Mod.
Jede Update-Prüfung liest außerdem Ihre Bewertungen und verfolgten Mods von Nexus (zwei
Anfragen) und setzt die Markierungen Endorsed und Tracked jeder Mod passend dazu, auf der
eigenen Nexus-Domain der Mod. Wenn sich beide unterscheiden, gilt Nexus. Kann eine der beiden
Listen nicht gelesen werden, meldet die Prüfung das und lässt diese Markierungen, wie sie
waren.

Der Knopf **Update** im Tab Nexus files von "Information..." installiert die neuere Datei. Mit
einem Premium-Konto lädt Eidos die Datei selbst herunter. Mit einem kostenlosen Konto öffnet es
die Seite der Datei, wo Mod Manager Download die Datei zurückschickt, und die Installation
beginnt, sobald das Archiv in `downloads/` ankommt. Danach ist es die gewohnte Installation: Ein
FOMOD öffnet sich mit den Antworten, mit denen die Mod installiert wurde, dann bietet die
Abfrage Replace mit angehaktem "Keep a backup of the existing mod" an, egal was Sie bei der
letzten Abfrage gewählt haben. Replace behält die Priorität der Mod und ob sie aktiviert ist.

Bevor ein Replace den Ordner einer Mod austauscht, vergleicht Eidos den alten mit dem neuen
Ordner nach Dateiname und Größe und liest die Köpfe der Plugins in beiden. Das gilt für Update,
ein Collection-Update und für Replace, das in der Installationsabfrage gewählt wird. Die
Statuszeile zählt danach die hinzugekommenen, entfernten und in der Größe geänderten Dateien und
nennt jeden Master, den die neue Version braucht und die alte nicht brauchte. Der Tab
**General** von "Information..." endet mit dem Abschnitt **Last update**. Er listet die
Versionen, die hinzugekommenen oder entfernten Plugins, die hinzugekommenen oder entfernten
Master, die Plugins, deren ESM-, ESL- oder ESH-Kennzeichen sich geändert hat, die Mods, mit denen
diese nun Dateien teilt oder nicht mehr teilt, und die Dateien. Eine Datei, die mit gleicher
Größe neu geschrieben wurde, steht nicht darin. Der Bericht liegt in `logs/updates/<mod>.json`
in der Instanz, die `eidos pack` auslässt; eine frische Installation oder ein Merge in den Ordner
löscht ihn. `eidos install --replace` gibt dieselben Zahlen in einer Zeile `changes:` aus.

Ein Ordner mit mehreren Nexus-Dateien, etwa einer Hauptdatei mit einer hineingemischten
optionalen Datei, wird abgelehnt: Ersetzt durch die neue Hauptdatei, gingen die anderen
verloren. Aktualisieren Sie diese von Hand.

`eidos nexus update skyrimse --apply` führt dieselbe Prüfung aus und lädt dann bei einem
Premium-Konto jede neue Datei herunter und installiert sie mit `--replace --backup` über ihren
Ordner. Bei einem kostenlosen Konto gibt es stattdessen für jede Dateiseite `OPEN <url>` aus.
Ordner, die mehrere Dateien zusammenführen, werden mit Begründung übersprungen. Eine FOMOD-Mod
ohne gespeicherte Antworten, etwa eine, die installiert wurde, bevor Eidos sie aufbewahrte,
wird nicht mit den Vorgaben des Installers neu installiert: Dieses Update schlägt fehl, mit dem
Hinweis, die Mod einmal aus dem Fenster zu aktualisieren, das die Antworten dann speichert. Ein
FOMOD, dessen Installer nichts zur Auswahl stellt, wird wie gewohnt installiert. Der Befehl
endet mit 1, wenn ein Update fehlgeschlagen ist.

### Collection-Updates

Wenn Sie Check for updates ausführen, fragt Eidos bei Nexus auch die neueste Revision jeder in dieser Instanz installierten Collection ab. Eine Zeile über der Mod-Liste nennt jede Collection mit neuerer Revision und zählt die hinzugekommenen (+), aktualisierten (~) und entfernten (-) Mods; die Namen erscheinen beim Darüberfahren. Die Zahlen fehlen, wenn das Rezept der installierten Revision nicht mehr im Cache liegt. **Update** öffnet die neuere Revision im Collection-Bereich, wo *Install this collection* sie ausführt. Die Liste wird bei jeder Prüfung neu aufgebaut und nicht gespeichert.

### Nexus-Einstellungen

**Preferences -> Nexus -> Offline** hält Eidos davon ab, Nexus überhaupt zu kontaktieren, und
lässt die tägliche Release-Prüfung auf GitHub aus. Update-Prüfungen, Anmeldung, Downloads und
Collections sagen das, statt mit einem Verbindungsfehler zu scheitern. Es ist standardmäßig aus.

**Preferred servers** ordnet die CDN-Knoten, die ein Download bevorzugt, den besten zuerst. Nur
einem Premium-Konto wird je mehr als ein Spiegel zur Auswahl gereicht, für alle anderen wählt
also Nexus und dies ändert nichts. Wird keiner der genannten Server angeboten, findet der
Download trotzdem statt, vom ersten Knoten, den Nexus anbietet.

### Kategorien

Kategorien lassen sich einer Mod oder einer ganzen Auswahl zuweisen, der Katalog lässt sich
aus demselben Dialog bearbeiten, und die offizielle Kategorienliste des Spiels lässt sich von
Nexus holen. Beide Katalogdateien sind MO2s eigene (`categories.dat` und `nexuscatmap.dat`),
sodass eine geteilte Instanz einen Katalog behält. Eine von Nexus heruntergeladene Mod wird bei
der Installation kategorisiert, sobald Fetch from Nexus den Katalog zugeordnet hat.

### INI-Editor, Log und Erweiterungen

**View -> INI editor** bearbeitet die Spiel-INIs des Profils, die Kopie, die Eidos behält; die
Kopie im Proton-Präfix wird bei jedem Start überschrieben. Sein letzter Tab ist die
`launch-env.ini` des Profils
([Variablen und Wrapper pro Profil](#variablen-und-wrapper-pro-profil)). **View -> Log** liest
die Sitzungsprotokolle. **View -> Extensions** listet Ihre eigenen Add-ons; siehe
[extensions.md](extensions.md).

Findet ein Start die Haupt-INI des Spiels (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...)
weder im Profil noch im Präfix, kopiert Eidos zuerst die Standarddatei aus dem Spielordner
(`Skyrim_Default.ini`, `Fallout4_Default.ini`...) ins Profil, wie MO2. Führt die Instanz eine
Spielkopie aus, stammt die Standarddatei aus der Kopie. Das Präfix und der Spielordner bleiben
unverändert.

### Mods installieren

Der Installer beherrscht Simple-Archive, FOMOD, BAIN-Pakete von Wrye Bash (haken Sie die
Unterpakete an, die der Reihe nach eingemischt werden) und OMOD-Pakete von Oblivion. Ein
OMOD-Skript läuft, wenn es OBMM-Befehle nutzt, die Eidos unterstützt, oder einer der drei
C#-Installer ist, die Eidos kennt; anderen C#- oder Visual-Basic-Code führt Eidos nicht aus und
sagt das. Erkennt keine Heuristik das Layout, zeigt eine manuelle Auswahl den Archivbaum und
lässt Sie auf die Datenwurzel zeigen.

Eine Neuinstallation oder ein Update einer FOMOD-Mod öffnet den Assistenten mit den angehakten
Antworten der letzten Installation, auf dem ersten Schritt, dessen Optionen sich seitdem
geändert haben, und listet die Änderungen auf. **Reinstall with new choices** beginnt
stattdessen mit den Vorgaben des Installers. `eidos install --replace` oder `--merge` verwendet
die gespeicherten Antworten erneut und bricht mit einer Liste der Änderungen ab, wenn sie nicht
mehr passen.

### Diagnostics

Der Tab Diagnostics führt Zustandsprüfungen live aus. Er meldet fehlende Master, Plugins, die LOOT als verschmutzt markiert, Archive, die kein aktives Plugin lädt, ob die Mod-Liste noch zum mods-Ordner passt, ob FUSE-Passthrough an ist und, wenn es angefordert ist, ob die dafür nötige Start-Capability da ist. Nach einem Lauf zeigt er, was das eigene Log des Script Extenders zu jeder seiner Plugin-DLLs sagt, sodass Sie sehen, ob Ihre SKSE-Plugins geladen wurden. Er meldet außerdem ein Spiel-Update seit dem letzten sauberen Start, einen fehlenden Script Extender oder einen, der nicht zur Spielversion passt (mit der Download-Seite des Extenders und einem Knopf, der das heruntergeladene Archiv installiert), eine Steam-Startoption, die Eidos nicht mit `%command%` startet, NVAPI, das für DLSS aus bleibt, exklusiven Vollbildmodus, der die Frame Generation verhindert, eine ENB-`enblocal.ini` ohne `LinuxVersion=true`, die Plugin-Deklarationen von F4SE- und SFSE-Plugins und das Absturzlog der letzten Sitzung mit den Mods, die es nennt.

Die Prüfungen für die Profil-INIs und Plugins suchen `sTestFile`-Zeilen in `[General]` der Custom-INIs von Fallout 4, Fallout 4 VR und Starfield, die das Spiel `plugins.txt` ignorieren lassen (Fix entfernt sie aus der Kopie des Profils und behält die vorherige Datei als `<ini>.pre-fix`), aktivierte `.esp`-Plugins bei Starfield und Blueprint-Plugins, denen das Blueprint-Flag, das Präfix `BlueprintShips-` oder die Endung `.esm` fehlt oder deren Hauptplugin nicht aktiviert ist. Die Systemprüfungen melden ein `vm.max_map_count` oder ein hartes Limit offener Dateien unter dem, was Proton braucht, und ein Spiel, Präfix oder eine Instanz auf NTFS, exFAT oder FAT. Die Mod-Prüfungen nennen den Datenpfad von BodySlide, wenn er nicht auf das `Data` des Spiels zeigt, `Data`-Ordner, die ein Tool nach `Overwrite/Root` geschrieben hat, einen Patch, dessen lose Dateien gegen die Mod verlieren, die er patcht, und eine Mod, die die Papyrus-Skripte des Script Extenders ersetzt.

Check for updates liest außerdem die Nexus-Anforderungen jeder Mod. Diagnostics listet eine benötigte Mod auf, die fehlt oder deaktiviert ist, mit Ignore für eine Anforderung, die anders erfüllt ist.

Der Tab zeigt außerdem, welcher Proton-Build das Spiel startet, und warnt, bevor ein älteres Proton ein Präfix startet, das ein neueres eingerichtet hat; Rebuild stellt ein defektes Präfix beiseite, damit Steam ein neues anlegt ([troubleshooting.md](troubleshooting.md#welches-proton-das-spiel-ausführt-und-ein-defektes-präfix)). Weitere Karten melden ein Präfix, das nie eingerichtet wurde oder dessen Registry veraltet ist, ein anderes Programm als Eidos, das Mod-Manager-Download-Links erhält (Register behebt es), Tool-Ausgabe, die älter ist als ihre Eingaben (mit einem Knopf, der das Tool erneut startet), und eine bereinigte oder als Light markierte Kopie eines Plugins, dessen Original sich seitdem geändert hat. Jede Karte ist in [troubleshooting.md](troubleshooting.md#health-karten) erklärt. Copy report legt einen Bericht über das System und jeden Befund in die Zwischenablage, mit verborgenem Home-Ordner und Login-Namen, für einen Fehlerbericht.

Für Skyrim SE, Enderal SE und Fallout 4 hat Diagnostics außerdem eine Karte Display. Sie zeigt Fenstergröße und Fenstermodus aus der `SkyrimPrefs.ini` des Profils (`EnderalPrefs.ini`, `Fallout4Prefs.ini`) oder aus der Kopie im Präfix, solange das Profil noch keine hat. Legt keine der beiden eine Größe fest, übernehmen die Felder den größten angeschlossenen Bildschirm, der in `/sys/class/drm` aufgeführt ist, ein Steam Deck zeigt also 1280x800. Breite und Höhe sind ganze Zahlen von 320 bis 16384. Apply schreibt `iSize W`, `iSize H`, `bFull Screen` und `bBorderless` in die Kopie des Profils und übernimmt vorher die Kopie aus dem Präfix, wenn das Profil keine hat. Legt SSE Display Tweaks das Fenster fest, schreibt Apply dieselbe Größe und denselben Modus in dessen Kopie im Overwrite; die Datei der Mod selbst bleibt unverändert.

Die Karte Game files oben im Tab steht unter [Spieldateien](#spieldateien-die-store-installation-oder-eine-kopie), die Karte zur Collection-Reihenfolge unter [Die Collection-Reihenfolge beibehalten](#die-collection-reihenfolge-beibehalten) und Find the mod behind a problem unter [Testläufe](#die-mod-hinter-einem-problem-finden).

### Das Spiel starten

Um das Spiel durch das Fenster zu starten, setzen Sie die Steam-Startoption des Spiels auf den
absoluten Pfad des Programms, weil Steam den `PATH` Ihrer Shell nicht liest
([install.md](install.md#dann-steam-darauf-zeigen-lassen)):

```
~/.local/bin/eidos-gui %command%
```

Aus Colony lautet der Pfad `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos öffnet sich auf
der Instanz des Spiels, der zuletzt benutzten, sodass eine portable Instanz genauso
wiedergefunden wird wie die globale; klicken Sie auf Run, um es durch die zusammengeführte Sicht
zu starten.

Ist **Start the game at once when a store opens Eidos** an (Preferences, General, Running a
game; standardmäßig aus), spart Eidos diesen Klick: Es prüft Health und startet das Spiel, ohne
sein Fenster zu zeigen. Das Fenster öffnet sich wie gewohnt, wenn Health ein Problem meldet,
wenn die Mod-Liste Aufmerksamkeit braucht, wenn ein Tool, das vor dem Spielen laufen soll, eine
veraltete Ausgabe hat, oder bei Skyrim SE, Enderal SE und Fallout 4, wenn das Profil noch keine
Fenstergröße hat (das Fenster öffnet sich dann auf Diagnostics). Wenn Sie das Fenster für einen
einzelnen Start möchten, setzen Sie `EIDOS_WINDOW=1` vor den Befehl:
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

In einem Fenster, das kein Store geöffnet hat, startet Run das Spiel nicht selbst, und Eidos
startet ein Spiel nie direkt über Proton oder Wine. Run bittet den Store, dem die Kopie gehört,
sie zu starten, und der Store startet Eidos:

- Steam führt `steam -applaunch <appid>` aus, wenn die Startoption oben gesetzt ist und das Spiel
  ein Proton-Präfix hat, das Steam beim ersten Start des Spiels anlegt. Ist Steam geschlossen,
  startet es zuerst.
- Heroic (eine GOG- oder Epic-Kopie, nativ oder als Flatpak) bekommt einen
  `heroic://launch`-Link, wenn Eidos in den Einstellungen des Spiels in Heroic ein Wrapper ist,
  mit dem absoluten Pfad von `eidos-gui` (oder `eidos-linux`) als ausführbarer Datei und ohne
  Argumente. Ist Heroic geschlossen, startet es zuerst.
- Legendary allein führt `legendary launch <app>` aus, wenn `~/.config/legendary/config.ini` im
  Abschnitt `[<app>]` des Spiels oder unter `[default]` die Zeile `wrapper =` mit dem absoluten
  Pfad von `eidos-gui` (oder `eidos-linux`) enthält.

Der Loader des Script Extenders, der Launcher und die ausführbare Datei des Spiels in der
Run-Liste werden alle auf diese Weise an den Store übergeben, ebenso
`eidos tool <instance> run <entry>` für einen davon in einem Terminal. Manche Kopien kann Eidos
nicht an ihren Store übergeben, etwa eine GOG-Kopie außerhalb von Heroic: Run sagt dann, das
Spiel aus diesem Store zu starten.

Die Übergabe hält diese Instanz als die zuletzt benutzte fest, und der Store öffnet dann ein
zweites Eidos-Fenster darauf. Ist Start the game at once aus, klicken Sie dort auf Run; ist es
an, startet dieses Fenster das Spiel, ohne sich zu zeigen, es sei denn, einer der Fälle oben
öffnet es. Das erste Fenster verfolgt diesen Lauf nicht; drücken Sie darin nach dem Spielen F5,
um seine Listen neu zu laden. Fehlt die Einstellung im Store oder liegt der Loader des Script
Extenders nicht neben der ausführbaren Datei des Spiels, startet Run nichts und zeigt, was wo zu
setzen ist, mit dem echten Pfad der laufenden Binärdatei.

Solange ein Spiel oder Tool läuft, wird Run zu Stop. Zwei Klicks beenden jedes Programm, das der
Lauf gestartet hat, und Eidos erfasst dann die Sitzung und hängt aus wie nach jedem Beenden; ein
gestoppter Lauf zählt nie als sauberer Start
([troubleshooting.md](troubleshooting.md#ein-spiel-oder-tool-das-sich-nicht-beendet)).

Steams `%command%` zeigt bei den Bethesda-Titeln meist auf `<Game>Launcher.exe`. Eidos führt ihn
nie aus: Der Launcher ist eine eigene Einstellungs-App, die `Data` neu einliest und
`plugins.txt` neu schreibt und damit die eben ausgebrachte Ladereihenfolge rückgängig macht.
Eidos führt stattdessen den Loader des Script Extenders aus, falls einer installiert ist, sonst
die Spielbinärdatei, und sagt es, wenn es auf die Spielbinärdatei zurückfallen muss.

Eidos durchsucht die DLLs der aktivierten Mods nach einem Import von `d3dcompiler_47.dll`, kopiert
Microsofts DLL ins Präfix und setzt erst dann `WINEDLLOVERRIDES=d3dcompiler_47=n,b`; Sie müssen
es nicht selbst setzen.

### Variablen und Wrapper pro Profil

Ein Profil kann eigene Umgebungsvariablen und Wrapper-Befehle in `profiles/<name>/launch-env.ini` enthalten, ein Eintrag pro Zeile:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Sie gelten für jedes `eidos play` dieses Profils (das Spiel, das Steams `%command%` startet, oder Run in einem Fenster, das ein Store geöffnet hat) und für jeden Tool-Lauf. `eidos shell` bekommt die Variablen, aber keinen Wrapper. Die erste `wrapper=`-Zeile ist die äußerste. Enthält eine `wrapper=`-Zeile `%command%`, steht der Befehl an dieser Stelle, wie bei Steams Startoptionen; ohne `%command%` hängt er am Ende. Wrapper laufen im privaten Namensraum von Eidos und sehen deshalb die Mods.

Eine Variable hier ersetzt den Wert, den Eidos für denselben Namen setzt, und die eigene `env=`-Zeile eines Tools in `tools.ini` ersetzt beides. `WINEDLLOVERRIDES` wird dagegen hinter die Overrides gesetzt, die Eidos für ENB, ReShade und die mitgelieferten DLLs setzt: Eine DLL, die in beiden steht, übernimmt die Einstellung des Profils, die übrigen behalten ihre.

View -> INI editor öffnet die Datei als letzten Tab, für jedes Spiel. Leere Zeilen und Zeilen, die mit `#` oder `;` beginnen, werden übersprungen. Beim Speichern nennt Eidos die Zeilennummern, die es ignoriert: Zeilen, die weder `env=NAME=value` noch `wrapper=command` sind, und eine `wrapper=`-Zeile, deren erstes Wort `NAME=value` ist, wie Steams Startoptionen es erlauben; setzen Sie diese Variable stattdessen in eine `env=`-Zeile. `eidos play <instance>` ohne Befehl listet die Variablen, die ein Lauf bekommt, und die Wrapper. Copy report in Diagnostics nennt die Variablen und die Wrapper-Programme, ohne ihre Werte.

## Tools

xEdit, BodySlide, DynDOLOD und andere Tools laufen durch die zusammengeführte Sicht im Wine-Präfix
des Spiels (Steams Proton-Präfix oder das, das Heroic für eine GOG- oder Epic-Kopie eingerichtet
hat):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # eine exe einmal ausführen, ohne sie zu registrieren
eidos prereqs skyrimse            # was die registrierten Tools brauchen, und ihr Zustand
eidos prereqs skyrimse --install  # holen, was fehlt
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen und Synthesis werden gefunden, ohne
sie zu registrieren. Pandora, Nemesis, FNIS, BodySlide und Synthesis kommen mit bereits
eingetragener Capture-Mod, sodass ihre Ausgabe in eine eigene Mod geht (Einzelheiten in
[tools.md](tools.md#die-ausgabe-eines-tools-in-eine-eigene-mod-schicken)).

Der Titel, den Sie einem Tool geben, entscheidet, welche Runtimes und DLLs Eidos dafür bereitstellt:
`BodySlide` bekommt seine DirectX-Bibliotheken, `BS` bekommt nichts. Im Fenster zeigt der
Executables-Dialog den echten Zustand jeder Voraussetzung unter dem Feld, und die fehlenden sind
Knöpfe.

Das Release-Bundle enthält in `runtimes/` die vcrun2022-Installer (x86 und x64), die .NET 8 Desktop
Runtime 8.0.12 und die .NET-10-Runtime 10.0.10, und `install.sh` kopiert sie nach
`~/.local/share/Colony/Eidos/runtimes/`, sodass `eidos prereqs <instance> --install` sie ohne
Download installiert. Eidos führt die Installer von Microsoft für vcrun2022 und .NET 8 still im Präfix
des Spiels aus, ohne winetricks. `eidos-linux` und Builds aus dem Quelltext haben keinen Ordner
`runtimes/` und laden deshalb dieselben festgelegten Dateien von Microsoft herunter. Das .NET-10-SDK,
das Synthesis braucht (etwa 300 MB), ist immer ein Download.

Die Tabelle, die drei Stufen von Voraussetzungen, warum DynDOLOD eine .NET-Runtime braucht, die
winetricks nicht installieren kann, und warum ein als Mod installiertes Tool aus dem zusammengeführten
Pfad statt aus seinem eigenen Ordner gestartet wird, stehen in [tools.md](tools.md).

## Erweiterungen

Eidos lässt sich erweitern, ohne neu gebaut zu werden: Ein TOML-Manifest in
`~/.config/Colony/Eidos/addons/` fügt der Extensions-Liste ein Tool oder dem Tab Diagnostics eine
Prüfung hinzu. Nichts wird in Eidos geladen: Eine Erweiterung ist ein Programm, das es ausführt. Siehe
[extensions.md](extensions.md).

## Den Proof of Concept ausprobieren

Das Skript braucht kein Spiel. Es zeigt die zusammengeführte Sicht, Copy-on-Write, einen
unberührten Spielordner und eine Zusammenführung, die nur innerhalb ihres Namensraums existiert,
allein mit unprivilegiertem OverlayFS in einem User-Namensraum (Linux 5.11 oder neuer):

```sh
./scripts/poc-overlay.sh
```
