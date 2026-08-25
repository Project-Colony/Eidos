<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Eidos verwenden

Das praktische Handbuch: die Kommandozeile, die GUI, die Steam-Startoption, das
Bauen aus dem Quelltext und das Proof-of-Concept-Skript. Was zu tun ist, wenn
etwas falsch aussieht, steht in [troubleshooting.de.md](troubleshooting.de.md).

## Benutzung (CLI)

```sh
eidos games                       # hier installierte unterstützte Spiele (wie MO2s Liste)
eidos init skyrimse               # eine Modding-Instanz anlegen
# ...jede Mod als Ordner nach <instance>/mods/ legen (die globale Instanz liegt
#    unter ~/.local/share/eidos/skyrimse; `eidos init` nennt Ihre)...
eidos install skyrimse mod.7z     # oder ein heruntergeladenes Archiv installieren (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # Reihenfolge + Plugin-Zustand eines vorhandenen MO2-Profils übernehmen
eidos sort skyrimse               # die Plugin-Ladereihenfolge mit LOOT sortieren
eidos play skyrimse               # zeigen, was gemountet würde
eidos play skyrimse -- <command>  # <command> mit den über das Spiel gemounteten Mods starten
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` und `eidos export`
vervollständigen den Satz; führen Sie `eidos` ohne Argumente aus für die
vollständige Liste.

### Instanzen: global und portabel

Jeder Befehl oben spricht eine Instanz an. `skyrimse` benennt die **globale** -
zentral unter `~/.local/share/eidos/skyrimse` abgelegt, von Eidos verwaltet. Die
andere Art ist **portabel**: ein in sich geschlossener Ordner, wo immer Sie ihn
haben wollen (eine zweite Platte, eine Spielepartition), verschiebbar und
isoliert, genau wie MO2s portable Instanzen. Wo ein Befehl eine Spiel-ID annimmt,
nimmt er auch den Ordner einer portablen Instanz:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # dort eine portable Instanz anlegen
eidos install /mnt/games/EidosSkyrim mod.7z  # jeder Befehl akzeptiert den Ordner
eidos play /mnt/games/EidosSkyrim -- %command%
```

Der Ordner beschreibt sich selbst (seine `eidos-instance.ini` nennt das Spiel),
mehr braucht es also nicht - und `EIDOS_INSTANCE=<folder>` in der Umgebung leitet
eine Spiel-ID auf diesen Ordner um, was in Steam-Startoptionen praktisch ist.
Portable Instanzen, die Sie angelegt oder geöffnet haben, werden (zuletzt
benutzte zuerst) in `~/.config/Colony/Eidos/instances.ini` gemerkt; der
Willkommensbildschirm der GUI listet sie zum Öffnen mit einem Klick, der
Steam-Start landet auf der zuletzt gespielten, und der `nxm://`-Handler lädt in
sie herunter. Zwei Vorbehalte, die man kennen sollte: das Verschieben eines
portablen Ordners behält alles außer den Tool-Einträgen, die Sie mit absoluten
Pfaden in den alten Ort registriert haben (die legen Sie neu an), und der
gemeinsame Runtime-Cache (`~/.local/share/Colony/Eidos/runtimes/`) bleibt bewusst
maschinenweit - ein 78 MB großer .NET-Host gehört nicht in jede Instanz.

Eidos hält seine eigenen Dateien unter `Colony/Eidos`, dem Aufbau, den jedes
Programm der Colony-Familie verwendet: `~/.config/Colony/Eidos/` für das, was Sie
gewählt haben (Einstellungen, Ihre Nexus-Sitzung, Ihre Instanzliste, die Spiel-
und Add-on-Definitionen, die Sie geschrieben haben),
`~/.local/state/Colony/Eidos/logs/` für Sitzungsprotokolle und
`~/.local/share/Colony/Eidos/` für das, was Eidos heruntergeladen hat. Ein
älteres Eidos hielt diese in `~/.config/eidos/` und `~/.local/state/eidos/`; der
erste Start nach dem Upgrade **kopiert** sie herüber und schreibt das ins Log.
Die alten Verzeichnisse bleiben genau so, wie sie waren - nichts wird gelöscht,
ein missglücktes Upgrade kann Sie also keine Anmeldung kosten - und Sie können
sie selbst entfernen, sobald Sie zufrieden sind.

Ihre Mods gehören nicht dazu. Eine globale Instanz liegt weiterhin unter
`~/.local/share/eidos/<game>/`, eine portable dort, wo Sie sie hingelegt haben,
weil diese Pfade in Ihrer Instanzliste stehen und womöglich in einer
Steam-Startoption: sie zu verschieben würde eine Verbindung brechen, von der
Eidos nicht beide Enden besitzt.

Ein Ort wird rundheraus abgelehnt: **im Installationsordner eines Spiels** (der
Reflex des MO2-Veteranen). Steam besitzt diesen Baum - ein Update, ein "verify
integrity" oder eine Deinstallation kann ihn überschreiben oder löschen und nimmt
Ihre ganze Einrichtung mit - und Eidos mountet über die Spielwurzel, eine Instanz
darin säße also innerhalb ihres eigenen Mount-Ziels. Der Assistent, `eidos init`
und `eidos play` sagen alle nein; legen Sie den Ordner stattdessen NEBEN das
Spiel (ein Geschwisterordner auf derselben Platte bietet dieselbe
Bequemlichkeit).

`play` mountet die Mods der Instanz über das `Data`-Verzeichnis des Spiels (über
einen Bind-Stash, damit der Daemon weiterhin die unberührten Dateien liest) in
einem privaten Namensraum und führt den Befehl dann durch diesen Blick aus.
Schreibvorgänge (Spielstände, neu erzeugte Konfigurationen) landen in der
`overwrite/`-Schicht der Instanz; die Spielinstallation und jede Mod-Quelle
bleiben Byte für Byte unberührt.

### Kein privilegierter Schritt nötig

Eidos läuft vollständig rootless. Es mountet in einem privaten User- und
Mount-Namensraum, also kein setuid-Helfer, kein Daemon und nichts zu gewähren.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` ist **optional** und schaltet
genau eines frei: den Kernel-FUSE-Passthrough, der standardmäßig aus ist, weil er
das Spiel kaputtmacht (siehe unten). Mit der Capability nimmt Eidos einen
einfachen Mount-Namensraum statt eines User-Namensraums; Mods werden so oder so
identisch ausgebracht.


Warum der alte `setcap`-Rat verschwunden ist - und warum FUSE-Passthrough
ausgeschaltet ausgeliefert wird - erklärt
[troubleshooting.de.md](troubleshooting.de.md#warum-passthrough-standardmäßig-aus-ist).

## GUI

```sh
cargo run -p eidos-gui
```

Ein MO2-artiger Assistent beim ersten Start im Colony-Look aus Pergament und
Burgunderrot: Willkommen -> Instanztyp (portabel / global) -> Spiel -> Name & Ort
-> Zusammenfassung -> Anlegen -> Hauptbildschirm. Der Willkommensbildschirm
listet außerdem jede bekannte vorhandene Instanz (global und portabel, zuletzt
benutzte zuerst) zum Öffnen mit einem Klick - er ist zugleich der
Instanzumschalter - und den Assistenten auf einen Ordner zu zeigen, der bereits
eine Instanz enthält, ÜBERNIMMT sie, wie sie ist, statt eine neue darüber
anzulegen (und lehnt rundheraus ab, wenn der Ordner zu einem anderen Spiel
gehört).

Das zweigeteilte Hauptfenster ist ebenfalls gebaut: eine Profilauswahl (wechseln
oder ein neues durch Kopieren des aktuellen anlegen), eine Mod-Liste, die Sie
filtern, auswählen, umsortieren, mit Trennern gruppieren, nach Kategorie
eingrenzen und für Aktionen mit Rechtsklick anfassen, dazu die Reiter Data /
Plugins / Conflicts / Overwrite / Saves / Downloads / Diagnostics und ein
Run-Knopf mit einer Auswahl des Startziels.

Umsortieren heißt nicht nur nach ganz oben oder ganz unten: MO2s gezielte
Verschiebungen gibt es hier auch - über die erste kollidierende Mod, unter die
letzte, auf eine ausdrückliche Priorität oder in die Gruppe eines Trenners. Sie
laufen alle durch denselben gemeinsamen Verschiebe-Helfer, sodass der Off-by-one,
der vom Entfernen der Zeilen vor dem Wiedereinfügen kommt, an einer Stelle
existiert statt an fünf.

### Spalten, Sortierung und Gruppierung

Die Liste zeichnet von Haus aus vier Spalten und bietet acht an: Category,
Content, Version, Author, Installed, Nexus id, Game, Flags. Haken Sie sie im
View-Menü an. Dass nicht alle acht voreingestellt sind, hat einen Grund - einer
Liste, in der jede Spalte zu sehen ist, bleibt kein Platz mehr für den NAMEN, und
das ist die Spalte, die Sie tatsächlich lesen.

Klicken Sie auf eine Überschrift, um danach zu sortieren. Nochmaliges Klicken
kehrt um, ein dritter Klick führt zurück zur **Ladereihenfolge**, was mehr
bedeutet, als es klingt: die Ladereihenfolge ist die einzige Reihenfolge, in der
die Liste gezogen werden kann, denn eine Einfügelücke spricht die echte Liste an,
während eine sortierte Zeile ganz woanders steht. Solange eine Sortierung aktiv
ist, werden die Einfügestreifen nicht gezeichnet und ein Ziehen wird abgelehnt,
statt irgendwo zu landen, wo niemand hinzielte - dasselbe, was MO2 tut, und aus
demselben Grund. Das View-Menü sagt das und bietet den Weg zurück.

Das View-Menü kann die ganze Liste auch **gruppieren**, nach Kategorie oder nach
Herkunft (von Nexus oder von Hand installiert). Gruppenköpfe sind keine Trenner:
hinter ihnen steht nichts, das umbenannt, eingefärbt oder verschoben werden
könnte, sie klappen zu, und die Anzahl bleibt am Kopf, wenn er zugeklappt ist.
Trenner verlassen die Liste unter einer Sortierung oder einer Gruppierung - ein
Trenner steht den Zeilen vor, die ihm in der Ladereihenfolge folgen, und beide
haben diese verschoben.

### Maus und Tastatur

Doppelklick auf eine Mod öffnet Information, Strg+Doppelklick ihren Ordner,
Umschalt+Doppelklick ihre Nexus-Seite. Strg+F setzt den Cursor ins Filterfeld.
Einen Buchstaben zu tippen springt zur nächsten Mod, die damit beginnt, und ihn
erneut zu drücken geht die übrigen durch, statt an der ersten zu kleben. Keine
davon kann auf einer Zeile landen, die der Filter, ein zugeklappter Trenner oder
eine zugeklappte Gruppe verbirgt - eine Hervorhebung zu bewegen, die Sie nicht
sehen, ist der Weg, auf dem die nächste Leertaste eine Mod umschaltet, die Sie
gar nicht ansahen.

"Collapse others" im Menü eines Trenners klappt jede Gruppe außer dieser zu.
Während eines Ziehens öffnet das Verweilen auf einer zugeklappten Gruppe sie,
sodass eine Mod hineingelegt werden kann, ohne das Ziehen vorher abzubrechen -
Verweilen, nicht Vorbeistreifen.

### Was die Liste Ihnen über eine Mod sagt

Zwei beratende Kennzeichen, beide ein Glyph mit der Erklärung beim Überfahren.
**No valid game data** bedeutet, dass oben in der Mod nichts danach aussieht, als
würde dieses Spiel es laden; womöglich müssen ihre Ordner eine Ebene hochgezogen
werden, oder es ist keine Mod für dieses Spiel. **Another game** bedeutet, dass
die `meta.ini` der Mod ein anderes nennt. Keines blockiert etwas - die Mod wird
weiterhin ausgebracht - und "Mark as valid" im Zeilenmenü bringt beide zum
Schweigen, über MO2s eigenen `validated=`-Schlüssel, sodass eine Mod, für die Sie
in einem Manager gebürgt haben, im anderen still ankommt.

Die Layout-Prüfung ist bewusst großzügig: ein `Root/`-Baum zählt, ein unlesbarer
Ordner zählt, ein leerer zählt. Eine falsche Warnung in einer Liste mit
fünfhundert Zeilen ist schlimmer als eine fehlende.

### Eine Mod sichern, bevor Sie sie anfassen

"Back up this mod" kopiert ihren Ordner beiseite als `<name>_backup` (dann
`_backup2` und so weiter - eine Sicherung ersetzt nie die vorherige). Die Kopie
ist **inert**: sie ist keine Mod, ihr Häkchen tut nichts, und sie trägt nichts
zum zusammengeführten Blick bei, denn sie anzuhaken würde zwei Kopien einer Mod
übereinander ausbringen. "Restore this backup over the mod" stellt sie in zwei
Klicks zurück; der aktuelle Inhalt wird zuerst beiseitegelegt und erst verworfen,
wenn die Kopie gelungen ist.

**Data** ist ein echter Baum des zusammengeführten Blicks, jeweils um eine Ebene
ausgeklappt, sodass das Öffnen eines Knotens ein Verzeichnis-Lesen pro Schicht
kostet, die ihn hat, statt eines rekursiven Durchlaufs jeder aktivierten Mod. Er
wird von DEMSELBEN Schichtenstapel beantwortet, aus dem das Mount bedient wird,
sodass Whiteouts und versteckte Dateien beachtet werden und der Reiter dem, was
das Spiel sehen wird, nicht widersprechen kann. Filtern Sie ihn nach Namen,
grenzen Sie ihn auf umstrittene Dateien ein, klären Sie mit den Spalten Size und
Modified, was wo liegt, und zeigen Sie jede Zeile per Reveal im Dateimanager.
**Plugins** ist die ESP/ESM/ESL-Ladereihenfolge (umschalten, von Hand umsortieren
oder mit LOOT sortieren und den Bericht danach lesen, dessen Ratschlag-Links sich
im Browser öffnen). **Conflicts** erklärt die Gewinner und Verlierer je Datei.
**Overwrite** macht in einem Schritt eine echte Mod aus dem, was das Spiel
geschrieben hat. **Saves** liest den Kopf jedes Spielstands - Charakter, Stufe,
Ort, Spielzeit - und vergleicht die darin eingebackene Plugin-Liste mit Ihrer
aktuellen, mit einem Knopf, der die benötigten Mods aktiviert, denn sie zu
benennen und Sie damit sitzenzulassen ist die langweilige Hälfte.

"Information..." öffnet einen Dialog je Mod: Allgemeines, Konflikte, Dateibaum,
INI-Anpassungen, Notizen. Aus dem Dateibaum (und aus dem Data-Baum) kann jede
Datei **versteckt** werden - umbenannt zu `<name>.mohidden`, was sie aus dem
virtuellen Blick nimmt, ohne sie zu löschen, sodass die drei verirrten Meshes
einer Mod unterdrückt werden können, ohne Prioritäten anzufassen. Der Dateibaum
beherrscht auch die gewöhnlichen Dateioperationen: neuer Ordner, umbenennen,
löschen, öffnen. Sie alle laufen durch einen Resolver, der alles ablehnt, was
kein einfacher Pfad innerhalb dieser Mod ist - kein `..`, kein absoluter Pfad und
keine Komponente, die ein Symlink ist, denn einem zu folgen würde ein Löschen
ganz aus dem Mod-Ordner hinausführen. Umbenennen ersetzt nur die letzte
Komponente, kann also nie zu einem Verschieben werden, und es lehnt einen bereits
vergebenen Namen ab, statt jene Datei stillschweigend zu ersetzen. Löschen
braucht zwei Klicks; es ist die eine Aktion hier, die ein weiterer Klick nicht
rückgängig machen kann.

**View** auf einer beliebigen Zeile im Dateibaum oder im Data-Baum zeigt eine
Vorschau der Datei: Bilder und Text. Nicht DDS oder NIF - die brauchen einen
Block-Decoder und einen Renderer, die dieser Baum nicht hat - aber sie sagen das,
statt ein leeres Feld zu zeigen, und verweisen auf Reveal. Text wird bis 64 KB
gelesen und sagt, wo er aufgehört hat, denn eine Vorschau ist ein Blick und ein
Papyrus-Log kann hundert Megabyte groß sein. **INI Tweaks** listet die Fragmente,
die eine Mod in ihrem `INI Tweaks/`-Ordner mitliefert; die aktivierten werden
beim Start in der Prioritätsreihenfolge in die Spiel-INI des Profils eingemischt
und wieder entfernt, wenn die INIs des Laufs erfasst werden - sonst würde eine
Anpassung stillschweigend zur Einstellung, und sie abzuschalten täte nichts.

Ein Download kann **aus der Downloads-Liste auf eine Position in der Mod-Liste
gezogen** werden, um ihn mit dieser Priorität zu installieren, und Archive oder
Ordner, die aus einem Dateimanager auf das Fenster fallen gelassen werden,
installieren ebenfalls (diese Hälfte braucht eine X11- oder XWayland-Sitzung -
winit setzt Datei-Drops nur für X11 um). Downloads selbst lassen sich anhalten
und fortsetzen: Anhalten stoppt die Übertragung und behält das Teilstück, und
Resume löst einen frischen Link neu auf und macht dort weiter, wo es aufgehört
hat.

Der Downloads-Reiter ist eine Archiv-**Bibliothek**, keine
Übertragungswarteschlange. Filtern Sie ihn nach Namen (auch nach dem freundlichen
Mod-Namen, sodass "skyui" `SkyUI_5_2_SE-12604-5-2SE.7z` findet), sortieren Sie
nach neuestem, Namen, Größe oder Zustand, und **verbergen** Sie ein Archiv, mit
dem Sie fertig sind - was die Datei behält und nur die Zeile fallen lässt, denn
ein Buch wegzustellen heißt nicht, es zu verbrennen. "Show hidden" holt sie
zurück, und derselbe Knopf hebt das Verbergen wieder auf. "Remove N installed"
löscht in zwei Klicks die Archive der Mods, die Sie bereits installiert haben,
und nur die **auf dem Bildschirm**: der Filter ist Ihre Art zu sagen, welche Sie
meinten.

### Nexus-Collections

Fügen Sie einen Collection-Link ein - oder klicken Sie auf der Website auf
einen - und Eidos listet die Mitglieder der Revision, jedes gegen diese Instanz
abgeglichen: installiert, heruntergeladen oder fehlend. Es **liest** eine
Collection; es installiert keine, und die Leiste sagt das. Vier Dinge machen
einen Installer hier unehrlich statt bloß schwierig: die Mitglieder sind
gewöhnliche Nexus-Dateien, die einen Schlüssel je Datei brauchen, den außerhalb
des seiteneigenen Knopfes nur ein Premium-Konto prägen kann; eine vollständige
Installation sind drei API-Aufrufe je Mitglied gegen ein Budget, das dieser
Client nicht überziehen will; die Phasen, Regeln und wiedergegebenen
FOMOD-Antworten des Manifests ließen sich nicht gegen eine echte veröffentlichte
Bethesda-Collection prüfen, und Raten erzeugt eine Ladereihenfolge, die richtig
aussieht und es nicht ist. Lesen kostet eine Anfrage und ist exakt.

Eine Collection kann nur gegen **ihr eigenes Spiel** gelesen werden. Öffnen Sie
eine Skyrim-Collection bei geladener Fallout-4-Instanz, und sie lehnt namentlich
ab, statt die Mitglieder gegen die falsche Mod-Liste abzugleichen, wo jedes
"installiert" und jedes "fehlend" Rauschen wäre, das die Form einer Antwort
trägt.

### Offline-Modus

**Settings -> Nexus -> Offline** hält Eidos davon ab, Nexus überhaupt zu
kontaktieren. Update-Prüfungen, Anmeldung, Downloads und Collections sagen das,
statt mit einem Verbindungsfehler zu scheitern. Es ist aus, solange Sie es nicht
einschalten - eine von einem älteren Eidos geschriebene Einstellungsdatei hat
keinen solchen Schlüssel, und einen fehlenden als "an" zu lesen würde jedem, der
aktualisiert, das Netz abschneiden.

**Preferred servers** ordnet die CDN-Knoten, die ein Download bevorzugt, den
besten zuerst. Nur einem Premium-Konto wird je mehr als ein Spiegel zur Auswahl
gereicht, für alle anderen wählt also Nexus und dies ändert nichts. Es ist eine
Reihenfolge, kein Filter: ist heute nichts von dem im Angebot, was Sie genannt
haben, findet der Download trotzdem statt, von welchem Knoten auch immer Nexus
zuerst angeboten hat.

**Categories** sind bearbeitbar, nicht nur angezeigt: weisen Sie sie einer Mod
oder einer ganzen Auswahl zu, bearbeiten Sie den Katalog selbst aus demselben
Dialog, und holen Sie die offizielle Kategorienliste des Spiels von Nexus. Beide
Katalogdateien sind MO2s eigene (`categories.dat` und `nexuscatmap.dat`), sodass
eine geteilte Instanz einen Katalog behält.

**View -> INI editor** bearbeitet die Spiel-INIs des Profils - die Kopie, die
bleibt, statt der im Proton-Präfix vergrabenen, die bei jedem Start überschrieben
wird. **View -> Log** liest die Sitzungsprotokolle. **View -> Extensions** listet
Ihre eigenen Add-ons; siehe [extensions.de.md](extensions.de.md).

Das Installieren nimmt alles an: die Simple- und FOMOD-Wege, dazu Wrye Bash
**BAIN**-Pakete (haken Sie die Unterpakete an, die der Reihe nach eingemischt
werden) und eine **manuelle** Auswahl, die den Archivbaum zeigt und Sie auf die
Datenwurzel zeigen lässt, wenn keine Heuristik das Layout erkennt. Kein Archiv
wird abgelehnt.

**Diagnostics** führt Zustandsprüfungen live aus: vor allem die Startfähigkeit,
fehlende Master (der mit Abstand verlässlichste Absturzvorbote), Archive, die
kein aktives Plugin laden wird, ob die Mod-Liste noch zum mods-Ordner passt, und
- nach einem Lauf - was das Log des Script Extenders selbst über jede seiner
Plugin-DLLs sagt, was aus "sind meine SKSE-Plugins geladen?" statt einer
Vermutung einen Beleg macht.

Um das Spiel durch die GUI zu starten, setzen Sie die Steam-Startoption des
Spiels auf den absoluten Pfad der Binärdatei (Steam sieht `~/.cargo/bin` im PATH
nicht):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos öffnet sich auf der Instanz des Spiels - der zuletzt benutzten, sodass eine
portable Instanz genauso wiedergefunden wird wie die globale; klicken Sie auf
Run, um es durch den zusammengeführten Blick zu starten. (Der Run-Knopf zeigt
genau diese Zeile, mit dem echten Pfad der laufenden Binärdatei, wenn Sie ihn
außerhalb von Steam drücken.)

Steams `%command%` zeigt bei den Bethesda-Titeln meist auf `<Game>Launcher.exe`.
Eidos führt ihn nie aus: der Launcher ist eine eigene Einstellungs-App, die
`Data` neu einliest und `plugins.txt` neu schreibt und damit die eben
ausgebrachte Ladereihenfolge rückgängig macht. Es setzt stattdessen den Loader
des Script Extenders ein, falls einer installiert ist, sonst die
Spiel-Binärdatei, und sagt es, wenn es zurückfallen muss - ein Spiel, das mit
jeder SKSE-Mod untätig startet, ist schlimmer als eines, das nicht startet.

Ältere Anweisungen erzwangen hier `WINEDLLOVERRIDES="d3dcompiler_47=n"`. Das ist
nicht mehr nötig und war nie ganz richtig: eine Überschreibung auf *native* hilft
nur, wenn eine echte `d3dcompiler_47.dll` bereits im Präfix liegt. Eidos
untersucht jetzt die DLL-Importe der aktivierten Mods, bringt die echte
Microsoft-DLL selbst aus und setzt erst dann die Überschreibung.

## Den Proof of Concept ausprobieren

Kein Spiel nötig. Er beweist Union + Copy-on-Write + Zero-Touch +
Namensraum-Geltungsbereich allein mit unprivilegiertem OverlayFS in einem
User-Namensraum (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Tools

xEdit, BodySlide, DynDOLOD und Konsorten laufen durch den zusammengeführten Blick
innerhalb des Proton-Präfixes des Spiels:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # was die registrierten Tools brauchen, und deren Zustand
eidos prereqs skyrimse --install  # holen, was fehlt
```

Eines sollten Sie wissen, bevor Sie ein Tool benennen: **der Titel entscheidet,
welche Runtime-DLLs Eidos dafür bereitstellt** - `BodySlide` bekommt seine
DirectX-Bibliotheken, `BS` bekommt nichts. In der GUI zeigt der
Executables-Dialog den echten Zustand jeder Voraussetzung unter dem Feld, und die
fehlenden sind Knöpfe.

Die Tabelle, die drei Stufen von Voraussetzungen, warum DynDOLOD eine
.NET-Runtime braucht, die winetricks nicht installieren kann, und warum ein als
Mod installiertes Tool aus dem zusammengeführten Pfad statt aus seinem eigenen
Ordner gestartet wird, stehen in [tools.de.md](tools.de.md).

Das Bauen aus dem Quelltext und der Aufbau des Repositorys stehen in
[../internals/contributing.md](../internals/contributing.md).

## Erweiterungen

Eidos lässt sich erweitern, ohne neu gebaut zu werden: ein TOML-Manifest in
`~/.config/Colony/Eidos/addons/` fügt der Extensions-Liste ein Tool oder dem
Health-Reiter eine Prüfung hinzu. Nichts wird in Eidos geladen - eine Erweiterung
ist ein Programm, das es ausführt. Siehe [extensions.de.md](extensions.de.md).
