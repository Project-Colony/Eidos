<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Ein durch Eidos gestartetes Tool sieht die zusammengeführte Sicht, im Wine-Präfix
des Spiels (Steams Proton-Präfix oder das Präfix, das Heroic eingerichtet hat). Es
liest, was das Spiel lesen wird (jede aktivierte Mod, in Prioritätsreihenfolge), und
was immer es schreibt, landet im Overwrite, wo ein Klick daraus eine echte Mod macht.
Ein Tool mit `runner=native` läuft ohne Proton; siehe
[unten](#umgebungsvariablen-und-tools-ohne-proton).

## Die, die Eidos von selbst findet

Manche Tools heißen eindeutig genug, um gefunden statt eingetragen zu werden, und
xEdit ist der offensichtliche Fall: `FO4Edit.exe` für Fallout 4, `SSEEdit.exe` für
Skyrim SE, `TES5Edit.exe` für das Original und so weiter. Jedes kommt mit einem
QuickAutoClean-Eintrag, der die dirty edits bereinigt, vor denen LOOT warnt.
`PGPatcher.exe` wird auf jedem Bethesda-Spiel außer Morrowind auf dieselbe Weise
gefunden, und es kommt mit einem Argument, ohne das es nicht arbeiten kann (siehe
[unten](#warum-pgpatcher---ignore-mo2vfscheck-braucht)). Eidos sucht sie nach
Dateiname in:

- `Overwrite/Root/`, dem Spielordner, aus dem die Instanz läuft (die
  Store-Installation oder ihre Spielkopie), und den `Root/`-Bäumen aktivierter Mods;
- dem `mods/` dieser Instanz, wo MO2-Nutzer ihre Tools installieren;
- dem Tools-Ordner, den Sie in den Preferences festlegen (General -> Tools), für ein
  zwischen Instanzen geteiltes Verzeichnis wie `/mnt/Games/Tools`.

Die gängigen Generatoren werden auf dieselbe Weise gefunden, jeweils bei den Spielen,
die sie unterstützen: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen und
Synthesis. Ebenso BethINI Pie (`Bethini.exe`), bei denselben Spielen wie PGPatcher;
siehe [BethINI Pie](#bethini-pie). Bei Stardew Valley wird SMAPI
(`StardewModdingAPI.exe`) auf dieselbe Weise gefunden. Es ist auch der Script-Extender-Loader
dieses Spiels und startet das Spiel deshalb über den Store, wie in
[Ein Tool erreichen](#an-ein-tool-herankommen-verstecken-anheften-und-eine-desktop-verknüpfung)
beschrieben.

Die Liste gilt pro Spiel, einer Skyrim-Instanz wird also nie Fallouts Editor
angeboten. Die Suche hört vier Ebenen tief auf, weil ein Mod-Bestand Hunderttausende
Dateien umfasst und das bei jedem Aufbau der Tool-Liste läuft, und sie folgt keinen
Symlinks. Ein so gefundenes Tool ist genauso konfiguriert wie ein selbst eingetragenes:
seine Voraussetzungen, seine Argumente (PGPatchers `--ignore-mo2vfscheck`) und seine
Capture-Mod ergeben sich aus seinem Titel, nach den Regeln unten.

Liegt ein Tool woanders, oder wollen Sie andere Argumente, tragen Sie es von Hand ein.
Ein eigener Eintrag mit demselben Titel überschreibt alles automatisch Gefundene.

## Ein Plugin bereinigen, das LOOT als verschmutzt meldet

Nach einer LOOT-Sortierung listet der Tab Diagnostics die Plugins auf, die LOOT als
verschmutzt (dirty) meldet, für die Spiele, die sowohl ein xEdit als auch
LOOT-Bereinigungsdaten haben (Skyrim LE, SE und VR, Enderal SE, Fallout 3, New Vegas,
Fallout 4 und 4 VR, Oblivion, Starfield). Findet Eidos xEdit, eingetragen unter
Executables oder an einer der Stellen der Instanz, an denen es nach Tools sucht, hat
die Karte Clean all, und jedes verschmutzte Plugin hat Clean with QuickAutoClean im
Menü seiner Zeile; andernfalls sagt die Karte, dass Sie xEdit unter Executables
hinzufügen sollen. Eidos lädt xEdit nicht herunter. Es startet den
QuickAutoClean-Modus von xEdit für jeweils ein Plugin, über die zusammengeführte
Sicht, mit `-qac -autoexit -autoload`. xEdit speichert das bereinigte Plugin über das
geladene, was in der zusammengeführten Sicht eine neue Datei im Overwrite ist; Eidos
verschiebt sie in eine Mod namens `Cleaned plugins`, die aktiviert am Ende der Liste
angelegt wird, wo sie jeden Konflikt gewinnt. Das Plugin in seiner eigenen Mod oder im
Data-Ordner des Spiels wird nie geschrieben. Deaktivieren Sie `Cleaned plugins` oder
blenden Sie eine Datei darin aus, um eine Bereinigung rückgängig zu machen. Ein Plugin,
das im Overwrite liegt, wird nicht bereinigt, weil xEdit diese Datei selbst
überschreiben würde; machen Sie das Overwrite zuerst zu einer Mod.

Backups, Cache und Log von xEdit landen in `<instance>/logs/qac-<time>/`, außerhalb
der zusammengeführten Sicht, und die Statuszeile meldet die Zahlen aus diesem Log.
Eidos löscht diesen Ordner nach einer gelungenen Bereinigung und behält ihn nach einer
fehlgeschlagenen. Eidos vermerkt Größe und Datum des Plugins, aus dem jede Kopie
entstand, in `Cleaned plugins/meta.ini`. Ändert sich dieses Plugin, zeigt der Tab
Diagnostics "Out-of-date copy of <plugin>", mit einem Knopf, der die alte Kopie
verbirgt.

## Eines hinzufügen

In der GUI: Tools in der Menüleiste oder Executables in der Werkzeugleiste, dann Add.
Von der Kommandozeile:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` fängt die Ausgabe dieses Laufs in `mods/<mod>` ab, unabhängig
von der eigenen Einstellung des Tools. `rm` entfernt nur Einträge, die Sie hinzugefügt
haben; eine Voreinstellung pro Spiel lässt sich verbergen, nicht entfernen.

Eine mit `--exe` gestartete exe, deren Dateiname Eidos für das Spiel kennt, ohne
Rücksicht auf Groß- und Kleinschreibung, bekommt den Titel dieses Tools und damit die
Voraussetzungen, Argumente und die Capture-Mod unten. `GenerateFNISforUsers.exe` ist
FNIS, die `--exe`-Zeile oben fordert also `dotnet48` an und fängt in `FNIS Output`
ab. Jede andere exe wird nach ihrem Dateinamen betitelt. Im Tab Data hat eine
`.exe`-Zeile Add tool, das den Dialog Executables mit ausgefülltem Eintrag öffnet, und
Run, das die Datei einmal startet wie `--exe`. Beide betiteln die Datei gleich.

Für einen Befehl, der kein Windows-Tool ist, öffnet `eidos shell skyrimse` Ihre
`$SHELL` (`/bin/sh`, wenn sie nicht gesetzt ist) im Spielordner, mit eingehängter
zusammengeführter Sicht. Was die Shell schreibt, landet im Overwrite, und das Verlassen
der Shell hängt die Sicht aus. `eidos shell skyrimse -- <command>` führt stattdessen
einen einzigen Befehl aus. Strg+C in der interaktiven Shell stoppt den eingetippten
Befehl und lässt die Shell offen. Die Shell läuft ohne Proton; ein Windows-Programm
geht über `eidos tool`.

Im Fenster beendet Stop einen Tool-Lauf, der hängt: Drücken Sie es zweimal, und Eidos
beendet die Programme des Laufs und behandelt dann seine Dateien wie nach jedem
Beenden. Ist Notify the desktop when a long job ends an (Preferences, General, Running
a game), sendet ein Tool-Lauf, der endet, während das Fenster im Hintergrund ist, eine
Desktop-Benachrichtigung.

Der Script Extender, die Spielbinärdatei und der Launcher werden automatisch erkannt;
nur zusätzliche Tools müssen eingetragen werden. Bei einer Instanz, die aus einer
Spielkopie läuft, werden sie in der Kopie erkannt; siehe
[Tools auf einer Spielkopie](#tools-auf-einer-spielkopie).

### Zeigen Sie auf die echte Datei, wo immer sie liegt

Tragen Sie die ausführbare Datei dort ein, wo sie tatsächlich liegt. Wurde das Tool als
Mod installiert, ist das im Mod-Ordner:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

Das ist der Pfad der globalen Instanz. Für eine portable Instanz gilt dieselbe Regel
unter ihrem eigenen Ordner, `<instance>/mods/...`. Ein absoluter Pfad wie dieser ist das
Einzige, was das spätere Verschieben eines portablen Ordners nicht übersteht.

Eidos schreibt diesen Pfad vor dem Start auf den zusammengeführten um, sodass das Tool
aus `<game>/Data/CalienteTools/BodySlide/` läuft und dort auch die Dateien jeder anderen
Mod sieht. BodySlide liefert ein leeres `SliderSets`-Verzeichnis mit, und jeder Körper,
den es bauen kann, stammt aus CBBE und den Outfit-Mods. Aus seinem eigenen Mod-Ordner
gestartet findet es nichts und wirkt kaputt.

MO2 schreibt genauso um, aus demselben Grund: sein eigener Kommentar nennt FNIS.

Ein Tool in einer deaktivierten Mod kann nicht umgeschrieben werden, weil seine Dateien
auch nicht in der Sicht liegen. Eidos sagt das und startet es aus seinem eigenen Ordner.

## Werkzeuge aus einer Mod-Organizer-2-Liste

Eine Wabbajack-Liste oder eine andere MO2-Instanz, die als Eidos-Instanz geöffnet wird,
behält ihre `ModOrganizer.ini`, und die dort eingerichteten Programme lassen sich in
`tools.ini` übernehmen:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

Im Fenster hat der Dialog Executables die Schaltfläche Import from ModOrganizer.ini,
mit einem Kästchen zum Abwählen für jeden Eintrag. Add selected übernimmt die
angehakten Einträge in die Liste, und gespeichert wird erst, wenn Sie Save drücken. Nur
eine `ModOrganizer.ini`, die Eidos nicht geschrieben hat, zählt; die, die Eidos
schreibt, steht unter [PGPatcher und PGLite](#pgpatcher-und-pglite-im-modus-mod-organizer-2).

Pfade auf `Z:` sind Linux-Pfade und bleiben unverändert. Ein Pfad auf einem anderen
Laufwerksbuchstaben wird unter den Ordner der Liste verschoben, wenn er diesen Ordner
nennt (`D:\Lists\MyList\tools\SSEEdit.exe` wird zu `<list>/tools/SSEEdit.exe`), und zwar
in der ausführbaren Datei, im Arbeitsverzeichnis und in den Argumenten. Der Loader des
Script Extenders, der Launcher und die Spielbinärdatei werden zu schlichten Namen und
laufen so aus dem Spielordner, über den Eidos die Sicht legt, wie die eingebauten
Einträge. Das `hide` von MO2 wird zu Hide from picker und `steamAppID` zu Steam AppID.
Einträge, die woanders hinzeigen (ein Windows-Systemprogramm, ein Tool außerhalb der
Liste), erscheinen mit Begründung als übersprungen, und ein Titel, den `tools.ini` schon
enthält, wird nie ersetzt.

Ein importierter Eintrag behält den Titel und die Argumente von MO2, und Eidos füllt aus
seinem Titel nichts auf: keine Voraussetzungen, kein zusätzliches Argument und keine
Capture-Mod. Setzen Sie sie im Dialog Executables. Einem PGPatcher-Eintrag aus einer
unter Windows erstellten Liste fehlt meist `--ignore-mo2vfscheck`; fügen Sie es hinzu
(siehe [unten](#warum-pgpatcher---ignore-mo2vfscheck-braucht)).

Ein importierter Pfad, der in den Ordner `Stock Game` der Liste zeigt, wird nur dann auf
die Store-Installation verschoben, wenn die Instanz aus dieser Kopie läuft; siehe den
nächsten Abschnitt.

## Tools auf einer Spielkopie

Läuft die Instanz aus einer Spielkopie (der Ordner `Stock Game` oder `Game Root` einer
Liste oder eine eingefrorene Kopie; siehe
[Spieldateien](usage.md#spieldateien-die-store-installation-oder-eine-kopie)), sieht ein
Tool die Dateien der Kopie, nicht die der Store-Installation. Die zusammengeführte Sicht
ist weiterhin auf die Store-Installation gemountet: Die Root-Sicht hat die Kopie ganz
unten, und die Data-Sicht liest das `Data` der Kopie. Eidos sucht den Script Extender,
den Launcher, die Spielbinärdatei und die bekannten Tools in der Kopie statt in der
Store-Installation.

Ein Tool-Pfad, ein Argument oder ein Arbeitsverzeichnis, das in die Kopie zeigt, wird vor
dem Lauf auf die Store-Installation verschoben. Es kann als Linux-Pfad oder als Wine-Pfad
`Z:` geschrieben sein: `-D:"Z:\...\Stock Game\Data"` wird zum selben Pfad unter der
Store-Installation. Aus der `ModOrganizer.ini` einer Liste importierte Einträge brauchen
das. Läuft die Instanz aus der Store-Installation, wird nichts verschoben, sodass ein
solcher Eintrag die nackte Kopie ohne Ihre Mods liest.

Der `installed path` des Spiels in der Präfix-Registry und das `gamePath` in der
`ModOrganizer.ini`, die Eidos schreibt, nennen weiterhin die Store-Installation, wo die
zusammengeführte Sicht liegt. Fehlt der Kopie-Ordner oder enthält er keine
Spielbinärdatei, bricht ein Tool-Lauf ab, wie ein Spielstart, und die Meldung nennt
`eidos stock-game <instance> store`, um zur Store-Installation zurückzukehren.

`eidos stock-game <instance>` zeigt, welche Spieldateien die Instanz startet und zu
welchen Kopien sie wechseln könnte. `use <folder>` und `store` wechseln sie, und
`freeze --yes` kopiert die Store-Installation in den Ordner `Stock Game` der Instanz und
startet daraus. Die Karte Game files im Tab Diagnostics tut dasselbe; siehe
[Spieldateien](usage.md#spieldateien-die-store-installation-oder-eine-kopie).

## Die Ausgabe eines Tools in eine eigene Mod schicken

Ein Generator (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) schreibt Hunderte Dateien.
Standardmäßig landen sie mit allem anderen im Overwrite. Setzen Sie Capture output into
im Executables-Editor, und die Ausgabe dieses Laufs geht stattdessen in jene Mod:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide und Synthesis kommen mit bereits eingetragener
Capture-Mod: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` und
`Synthesis Output`. DynDOLOD, TexGen und xLODGen kommen ohne, weil sie in den
Ausgabeordner schreiben, den Sie ihnen angeben.

Die Mod wird angelegt, falls sie nicht existiert. Nur die Dateien, die dieser Lauf
erzeugt hat, werden verschoben; was schon vorher im Overwrite lag, bleibt dort, sodass
zwei Tools mit Capture-Zielen sich nicht gegenseitig die Ausgabe wegnehmen. Ein Lauf, der
nichts geschrieben hat, hinterlässt keine leere Mod.

Eidos verschiebt die Dateien nach dem Lauf. MO2 zeigt stattdessen mit der Schreibschicht
auf die Mod, was diese Mod für den ganzen Lauf auf höchste Priorität hebt: Jeder
Konflikt, in dem sie steckt, kippt und kippt danach zurück. Die Schreibvorgänge gehen
außerdem ohne Copy-up direkt durch die eigenen Dateien der Mod. Das Capture erreicht
denselben Endzustand ohne beides.

Ist die Ziel-Mod deaktiviert, wird die Ausgabe zwar geschrieben, aber das Spiel sieht
sie nicht, das Tool würde also beim nächsten Lauf dieselben Dateien neu erzeugen. Eidos
warnt, wenn das der Fall ist.

### Wenn seine Ausgabe veraltet ist

Eidos merkt sich, aus welchen Mods, welcher Plugin-Reihenfolge und welchem Profil die
Ausgabe eines Tools entstanden ist, dazu Größe und Datum der ausführbaren Datei des Tools
und Pfad und Argumente des Eintrags in `tools.ini`. Ändert sich eines davon, zeigt der Tab
Diagnostics "Generated output inputs changed", und die Karte hat einen Knopf Run <tool>
again, der das Tool mit der jetzigen Konfiguration startet.

Für einen Generator, dessen veraltete Ausgabe das Spiel kaputtmacht, etwa Pandora oder
Nemesis, tragen Sie `check_before_play=1` in seinen Eintrag in `tools.ini` ein oder
schalten im Dialog Executables Ask before playing when its output is out of date dafür
ein. Run fragt dann zuerst: das Tool starten oder trotzdem spielen. Play anyway gilt, bis
sich Mods, Plugins oder Profil wieder ändern oder das Fenster geschlossen wird. Eidos
startet das Tool nie von selbst. Ist Start the game at once when a store opens Eidos an,
hält eine solche Frage das Fenster offen, statt das Spiel zu starten.

## Voraussetzungen und Argumente ergeben sich aus dem Titel

Der Titel, den Sie einem Tool geben, entscheidet, welche Laufzeit-Voraussetzungen Eidos
dafür bereitstellt. Verglichen wird als Teilzeichenkette des Titels, ohne Rücksicht auf
Groß- und Kleinschreibung.

| Wenn der Titel enthält | fordert Eidos an |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| alles andere | nichts |

Ein als `BodySlide` eingetragenes Tool bekommt also seine DirectX-DLLs; dieselbe
ausführbare Datei als `BS` eingetragen bekommt nichts und startet womöglich nicht, mit
einem Fehler, der nichts über DLLs sagt. Benennen Sie Tools nach dem Programm.

Die Liste steht in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), und das Feld
`Prereqs` im Executables-Dialog ist editierbar: Der Titel füllt nur die Vorgabe aus.

Der Titel entscheidet genauso über Argumente, für das eine Tool, das eines braucht, das
niemand erraten könnte: Ein Titel mit `pgpatcher` (oder `parallaxgen`, dem früheren
Namen) erhält `--ignore-mo2vfscheck`. Warum, und warum es nicht optional ist, steht
[unten](#warum-pgpatcher---ignore-mo2vfscheck-braucht). Die Capture-Mod ergibt sich
ebenfalls aus dem Titel (siehe
[oben](#die-ausgabe-eines-tools-in-eine-eigene-mod-schicken)).

### Drei Arten von Voraussetzung

Stufe 1 sind die mitgelieferten DLLs: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43` und
`d3dx11_43`. Eidos trägt sie in seinen Binärdateien und kopiert sie beim Start ins
Präfix. In einem 64-Bit-Präfix, Protons Standard, kommt die 64-Bit-Fassung nach
`system32` und die 32-Bit-Fassung nach `syswow64`. Nichts zu tun, kein Netz. Jede
Fassung ist gzip-komprimiert gespeichert, sodass die acht Dateien 7,9 MB jeder
Binärdatei belegen statt 17,9 MB, und nur die, die gerade kopiert wird, wird entpackt
(höchstens 4,7 MB).

Stufe 2 sind die Installer: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` und `xact_x64`. Diese schreiben
Registry-Schlüssel, den GAC und CLR-Hosts, lassen sich also nicht per Dateikopie
erledigen. Drei davon installiert Eidos ohne winetricks: `vcrun2022` startet Microsofts
`VC_redist.x86.exe` und `VC_redist.x64.exe`, und `dotnet8` und `dotnetdesktop8` starten
den Installer der .NET 8.0.12 Desktop Runtime, der die .NET-Runtime mitbringt. Das
Release-Bundle (`eidos-linux-bundle.tar.gz`) liefert diese Installer in seinem Ordner
`runtimes/` mit, und `install.sh` kopiert sie für den Benutzer, der es ausführt, nach
`~/.local/share/Colony/Eidos/runtimes/`, auch mit `--system`. Eidos sucht jede Datei
zuerst dort, dann in einem Ordner `runtimes/` neben der Binärdatei `eidos` (ein
entpacktes Bundle, an Ort und Stelle gestartet). Ohne eine der beiden Kopien (die
einzelne Datei `eidos-linux`, ein Build aus dem Quelltext oder ein anderer Benutzer
einer Systeminstallation) lädt Eidos sie von Microsoft herunter
(`download.visualstudio.microsoft.com`), nach `~/.local/share/Colony/Eidos/runtimes/`,
und behält sie dort. In beiden Fällen wird eine Datei abgelehnt, wenn ihre SHA-256 nicht
zu der in Eidos eingebauten passt, und sie läuft ohne Fenster
(`/install /quiet /norestart`) im Präfix des Spiels, mit dem Proton- oder Wine-Build, der
die Tools des Spiels startet. Wie winetricks setzt `vcrun2022` zuerst die
Visual-C++-DLLs in den DLL-Overrides des Präfixes auf `native,builtin`. Danach prüft
Eidos bei einem Proton-Build, dass die C++-Runtime des Präfixes nicht halb von Microsoft
und halb von Proton stammt. Ist das der Fall, scheitert die Installation: Die Meldung
nennt die DLL-Paare, die aus verschiedenen Builds stammen, und sagt, stattdessen
`vcrun2022` mit winetricks zu installieren, das `msvcp140.dll` selbst ersetzt. Installiert
wird nur das 64-Bit-.NET 8; ein 32-Bit-Tool für .NET 8 braucht weiterhin
`winetricks dotnetdesktop8`.

Die anderen sechs Verben laufen über das `winetricks` des Systems, das im `PATH` liegen
muss. Eidos führt es mit dem `wine` des Proton- oder Wine-Builds aus, der die Tools des
Spiels startet, gegen das Spiel-Präfix, was Steams pressure-vessel-Container und die
Unstimmigkeit zwischen protontricks und Proton-GE umgeht. Manche winetricks-Verben
brauchen `cabextract`, und Eidos warnt, wenn es fehlt. Ein Tool, das ein nicht
installiertes Stufe-2-Verb deklariert, startet trotzdem, mit einer Warnung, die das Verb
und den Befehl zur Behebung nennt, weil Sie es vielleicht anderswoher haben.

Ein Stufe-2-Verb gilt als erledigt, wenn die `winetricks.log` des Präfixes es listet,
gleich wer winetricks oder protontricks ausgeführt hat, oder wenn Eidos es in dasselbe
Präfix installiert hat. Eidos hält seine eigenen Installationen in `prereqs.done` in der
Instanz fest, gebunden an Pfad und Inode des Präfixes, sodass ein neu aufgebautes Präfix
seine Verben wieder als ausstehend zeigt. Rebuild prefix, eine Diagnostics-Aktion für eine
Steam-Kopie, benennt `compatdata/<appid>` in `<appid>.eidos-backup-<time>` um und löscht
nichts; Steam legt beim nächsten Spielstart ein neues Präfix an.

Stufe 3 sind die Runtimes `dotnet10` und `dotnetsdk10`. Eine moderne .NET-Runtime besteht
aus 193 Dateien, die in ihrem eigenen Verzeichnis liegen und über `DOTNET_ROOT` gefunden
werden. Sie wird nie registriert und nie ins Präfix installiert, sodass keine der anderen
Stufen sie tragen kann. Eidos hält sie außerhalb jeder Instanz, weil 78 MB weder pro Spiel
noch pro Profil anfallen: Jede Runtime wird nach
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` entpackt (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos sucht das Zip der Runtime unter seinem Download-Namen in
denselben zwei Ordnern wie die Stufe-2-Installer und entpackt die Kopie, die das
Release-Bundle mitliefert (`dotnet-runtime-10.0.10-win-x64.zip`, nur für `dotnet10`), oder
lädt sie herunter. Das `dotnet-sdk-10.0.403-win-x64.zip` des SDK liefert das Bundle nicht
mit, aber eine von Hand dorthin gelegte Kopie wird genutzt und erspart den Download. Eidos
lehnt beide Dateien ab, wenn ihre SHA-256 nicht zu der in Eidos eingebauten passt. Es
entpackt das Zip mit `7z`, demselben 7-Zip, das es für Mod-Archive nutzt.

`dotnetsdk10` ist das .NET-10-SDK, ein Download von 301 MB, der auf 808 MB entpackt wird.
Synthesis braucht es, um seine Patcher zu kompilieren, und ruft zum Bauen und Starten ein
nacktes `dotnet` auf. Ein Tool, das `dotnetsdk10` angibt, bekommt deshalb auch den
SDK-Ordner in `WINEPATH` sowie `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` und `DOTNET_CLI_TELEMETRY_OPTOUT=1`. Ein
Patcher-Build ließe MSBuild-Knoten und den VBCSCompiler-Server weiterlaufen, und unter
Wine halten sie den wineserver des Präfixes am Leben, deshalb bekommt das Tool auch
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` und
`UseSharedCompilation=false`. Mit `DOTNET_ROLL_FORWARD=Major` läuft ein für .NET 8 gebauter
Patcher (Mutagen vor 0.54) auf der .NET-10-Runtime des SDK, wenn keine .NET-8-Runtime
gefunden wird. Ein Synthesis-Eintrag in `tools.ini`, der noch den alten Standard `dotnet8`,
`vcrun2022` enthält, bekommt beim Einlesen durch Eidos `dotnetsdk10`, `vcrun2022`.

NuGet prüft die Signatur jedes Pakets, das es wiederherstellt, gegen den
Windows-Stammzertifikatspeicher, und einem Wine-Präfix fehlen die Stammzertifikate für
Codesignatur und Zeitstempel, die das SDK als `sdk/10.0.403/trustedroots/codesignctl.pem`
und `timestampctl.pem` mitbringt. Ohne sie scheitert die Wiederherstellung von
`Mutagen.Bethesda.Synthesis` 0.36.6 mit `NU3018` und `NU3028`. Deshalb fügt Eidos, bevor
ein Tool mit `dotnetsdk10` startet und sobald das SDK heruntergeladen ist, die 385
Zertifikate beider Dateien dem Stammzertifikatspeicher des aktuellen Benutzers im Präfix
des Spiels hinzu, im selben `regedit`-Import, der den Spielpfad schreibt (siehe unten).
Fehlt später eines, importiert Eidos sie erneut. Jedes Windows-Programm in diesem Präfix
vertraut diesen Stammzertifikaten dann. Mit ihnen gelingen in einem frischen Präfix mit
leerem NuGet-Cache dieselbe Wiederherstellung und der Build, mit dem System-Wine 11.21 und
mit GE-Proton11-9.

Eidos installiert nichts aus Stufe 2 oder 3, bevor Sie darum bitten:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

In der GUI stehen dieselben Zustände unter dem Prereqs-Feld, und die fehlenden sind
Knöpfe. Ein Verb, das weder mitgeliefert noch eine Runtime noch ein Stufe-2-Verb ist,
wird als wahrscheinlicher Tippfehler gemeldet statt als Download angeboten.

`eidos prereqs <instance> --install` verweigert, solange das Spiel, Steam oder ein Tool
das Präfix noch hält. Stufe-3-Runtimes installieren sich auch, bevor das Präfix existiert.
Die Installer, die Eidos selbst ausführt, brauchen ein vorhandenes Präfix: Bevor der Store
das Spiel einmal gestartet hat, brechen sie mit "start the game once through its store,
then install again" ab. Stufe-2-Verben werden eines nach dem anderen installiert, und
jeder Erfolg wird festgehalten, sodass derselbe Befehl nach einem Fehlschlag
weitermacht. Bei einer GOG- oder Epic-Kopie ohne Heroic-Einstellungen kopiert `--install`
die mitgelieferten DLLs und hält dann vor den Stufe-2-Verben an: Es sagt, das eigene
winetricks dieses Runners zu verwenden, und liest die `winetricks.log` des Präfixes bei
der nächsten Prüfung.

### Warum DynDOLOD `dotnet10` braucht

DynDOLOD baut Object LOD nicht selbst: es ruft LODGen auf, und es liefert drei davon mit.
`LODGenx64.exe` zielt auf .NET Framework 4.8, das unter Proton auf Wines Mono umgeleitet
wird, und der `System.Uri`-Initialisierer von Mono ruft eine Methode auf, die Mono nicht
implementiert. LODGen stirbt vor seiner ersten Zeile Arbeit und hinterlässt ein Log mit
einem Versionsbanner und sonst nichts, und einen DynDOLOD-Dialog, der nur "failed for one
or more worlds" sagt.

Das echte .NET Framework zu installieren behebt es nicht: Proton ersetzt `mscoree.dll`,
den Loader, der es finden würde, durch einen Symlink in seinen eigenen Baum und macht das
bei jedem Präfix-Update erneut.

Der Build, der funktioniert, ist `LODGenx64Win10.exe`, der auf modernes .NET zielt und
`mscoree` nie anfasst. Zeigen Sie `DOTNET_ROOT` auf eine .NET-10-Runtime, und er läuft.
Genau das stellt `dotnet10` bereit, und Eidos setzt die Variable beim Start jedes Tools,
das sie deklariert.

### Warum Pandora `dotnet10` braucht

Pandora ist eine Avalonia-App, keine WPF-App, und zielt seit Pandora 4.1.0 auf `net10.0`,
sodass die .NET-10-Runtime allein es startet. Findet es keine Runtime, sagt es, die .NET
Desktop Runtime zu installieren. Das ist der allgemeine Text, den der .NET-Launcher für
jedes GUI-Programm zeigt, das keine Runtime findet, und `dotnetdesktop8` behebt es nicht.

Eidos 1.21.0 und älter gaben Pandora `dotnetdesktop8`, und ein Eintrag behält seine Zeile
`prereqs=`. Liest Eidos einen Pandora-Eintrag, der noch nur `dotnetdesktop8` auflistet,
und nennt die `.runtimeconfig.json` neben seiner ausführbaren Datei `net10.0`, bekommt der
Eintrag stattdessen `dotnet10`. Pandora 2.x zielt auf .NET 8 und behält, was es hat.

### Warum PGPatcher `--ignore-mo2vfscheck` braucht

PGPatcher schreibt Meshes und Plugins so um, dass die vorhandenen Textur-Mods (vanilla,
Parallax, Complex Material, PBR) auf jeder Oberfläche den richtigen Shader verwenden. Es
weigert sich, außerhalb eines Mod-Managers zu laufen, weil das Patchen eines nackten
Spielordners zerstörerisch wäre.

PGPatcher sucht nach usvfs, der DLL-Hooking-Schicht, die MO2 unter Windows ins Spiel
injiziert. Eidos hat keine: Die zusammengeführte Sicht ist ein FUSE-Mount, in das Wine
läuft, ohne davon zu wissen. Die Prüfung findet also nichts, und das Tool beendet sich
sofort mit

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

und nennt dabei ein Programm, das der Nutzer gar nicht ausführt, noch vor der ersten
Arbeit. Die Antwort des Autors auf die beiden Linux-Meldungen ([#716][pg716] und
[#725][pg725], beide von Fluorine, MO2s Linux-Fork, der aus demselben Grund FUSE nutzt)
ist ein Schalter, der die Prüfung überspringt.

Eidos setzt ihn daher vor. Ein im Mod-Pool gefundener PGPatcher kommt mit dem Argument
bereits gesetzt, und `PGPatcher` als Titel im Executables-Dialog füllt das Arguments-Feld
genauso. Was Sie tippen, gewinnt immer: Vorbelegt wird nur ein Feld, das Sie leer gelassen
haben, genau wie bei den Voraussetzungen oben.

Die Liste steht in `default_args` (`crates/eidos-instance/src/tools.rs`), neben
`default_prereqs`.

Zur Laufzeit braucht es nichts, ungewöhnlich für ein .NET-Tool: Es bringt seine eigene mit
(`dotnetlib/`, mit einer `runtimeconfig.json`, die `includedFrameworks` deklariert) und
liefert die Microsoft-CRT-DLLs neben der ausführbaren Datei. Lassen Sie Prereqs leer.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher und PGLite im Modus Mod Organizer 2

Mit Mod Manager auf `None` sieht PGPatcher einen einzigen zusammengeführten `Data`-Ordner
und kann nicht erkennen, aus welcher Mod ein Mesh oder eine Textur stammt. Es hat also
keine Konflikte anzuzeigen und keine Mod-Reihenfolge, der es folgen könnte. Sein Modus
`Mod Organizer 2` liest die Instanz selbst: die `modlist.txt` des Profils, die
aktivierten Mods in ihrer Prioritätsreihenfolge und die Dateien im Ordner jeder Mod.
PGLite, die Version im Spiel, liest die Instanz auf dieselbe Weise. Eine Eidos-Instanz hat
bereits das Layout von MO2 (`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), daher
funktioniert dieser Modus mit ihr.

Im Launcher von PGPatcher:

1. Stellen Sie Mod Manager auf `Mod Organizer 2`.
2. Stellen Sie MO2 Options -> Instance Location auf den Ordner der Eidos-Instanz,
   geschrieben als Windows-Pfad auf Wines Laufwerk `Z:`, das `/` ist. Eine portable
   Instanz unter `/mnt/Games/Eidos-Skyrim` wird zu `Z:\mnt\Games\Eidos-Skyrim`; eine
   globale wird zu `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Behalten Sie `--ignore-mo2vfscheck` in den Argumenten des Tools. In diesem Modus sucht
   PGPatcher nach usvfs und bricht ab, wenn es fehlt.

PGPatcher akzeptiert den Ordner nur, wenn er eine `modorganizer.ini` enthält. Deshalb
schreibt Eidos bei jedem Start des Spiels oder eines Tools eine `ModOrganizer.ini` in das
Wurzelverzeichnis der Instanz, für das gestartete Profil. Die Datei wird nur neu
geschrieben, wenn sich ihr Inhalt ändert. Sie enthält:

- in `[General]`: `gameName` (der Name des Spiels in MO2, also `New Vegas` für Fallout:
  New Vegas), `gamePath` (den Installationsordner des Spiels im Store als `Z:`-Pfad, auch
  bei einer Instanz, die aus einer Spielkopie läuft) und `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` und `overwrite_directory` als
  absolute `Z:`-Pfade, dazu `download_directory`, wenn die Instanz einen Ordner
  `downloads/` hat.

Ihre erste Zeile ist ein Kommentar, der sagt, dass Eidos sie geschrieben hat. Eine
`ModOrganizer.ini` ohne diese Zeile, in welcher Schreibweise des Namens auch immer, wird
nie geändert oder gelöscht: Eine MO2-Instanz, die Sie in Eidos geöffnet haben, behält ihre
eigene, und PGPatcher liest diese Datei so, wie sie ist. MO2 verwirft Kommentare, wenn es
seine Einstellungen speichert, also gehört eine Datei, über die MO2 gespeichert hat, von
da an MO2. Eine leere Datei gilt als die von Eidos.

Eine solche Datei nennt das Profil, das MO2 zuletzt ausgewählt hat, und PGPatcher
verwendet dieses Profil, nicht das, das Eidos startet. Wenn sich die beiden unterscheiden,
protokolliert Eidos beim Start eine Warnung. Wählen Sie in MO2 dasselbe Profil, oder
benennen Sie die Datei um oder entfernen Sie sie, damit Eidos seine eigene schreibt.

Tools im Modus Mod Organizer 2 lesen das Spiel außerdem aus dem `gamePath` dieser Datei.
Nennt er weder den Spielordner, aus dem die Instanz läuft, noch die Store-Installation,
zeigt der Tab Diagnostics "ModOrganizer.ini points tools at another game folder", mit
diesem Pfad und dem Spielordner, den diese Instanz startet. Ändern Sie `gamePath` in der
Datei oder entfernen Sie die Datei, damit Eidos seine eigene schreibt.

Eidos schreibt seine Datei nur, wenn es etwas startet. Zwischen zwei Starts nennt sie also
das Profil des letzten. Ein Tool, das Eidos startet, findet dort sein eigenes Profil.

PGPatcher bricht mit einem Fehler ab, wenn das Profil keine aktivierte Mod hat und wenn
sein Ausgabeordner der Ordner einer aktivierten Mod ist, so wie unter MO2.

## Wines eigene Programme im Prefix des Spiels

Das Menü File startet vier Programme von Wine im Prefix des Spiels: winecfg, regedit, den
Wine-Dateimanager und die Deinstallation. Sie starten wie jedes Tool, mit dem Proton- oder
Wine-Build, der die Tools des Spiels ausführt, bei eingehängten Mods, und das Fenster
wartet auf sie. Prefix drive_c öffnet das Laufwerk `C:` des Prefix in Ihrem Dateimanager.
Eine Zeile bleibt grau, bis das Prefix das Programm enthält, was geschieht, wenn der Store
das Spiel zum ersten Mal startet. Von einem Terminal aus tut `eidos tool <instance> run
winecfg` dasselbe, ebenso `regedit`, `explorer` und `uninstaller`, wenn kein Tool in der
Liste diesen Titel hat.

Eine in regedit vorgenommene Änderung am `installed path` des Spiels wird beim nächsten
Tool-Lauf zurückgesetzt; siehe den nächsten Abschnitt.

## Der Spielpfad im Präfix

Windows-Tools finden ihr Spiel, indem sie `HKLM\Software\Bethesda Softworks\<game>`
`installed path` lesen, einen Schlüssel, den der Installer des Spiels selbst schreibt.
Steam unter Proton führt diesen Installer nie aus. Ohne den Schlüssel öffnen xEdit, Wrye
Bash und DynDOLOD auf einem leeren Pfad.

Vor einem Tool-Lauf schreibt Eidos den Schlüssel mit einem `regedit`-Import im Präfix des
Spiels, außer ein früherer Import hat denselben Pfad schon geschrieben und das Präfix hält
ihn noch. Der Schlüssel nennt die Store-Installation, auch bei einer Instanz, die aus einer
Spielkopie läuft, weil die zusammengeführte Sicht dort gemountet ist. Für ein Tool, das
`dotnetsdk10` deklariert, fügt derselbe Import die NuGet-Signatur-Stammzertifikate hinzu
(siehe [oben](#drei-arten-von-voraussetzung)). Ein Tool mit `runner=native` überspringt
diesen Schritt. Existiert das Präfix noch nicht oder hält ein anderes Programm es, läuft
das Tool trotzdem, und Eidos gibt eine Warnung aus, die den Store nennt, aus dem das Spiel
einmal zu starten ist, oder das Programm, das zu schließen ist. Nennt der Schlüssel einen
anderen Ordner als die Store-Installation, zeigt der Tab Diagnostics "Windows tools cannot
find <game>", bis der nächste Tool-Lauf ihn erneut schreibt.

## An ein Tool herankommen: verstecken, anheften und eine Desktop-Verknüpfung

Die Voreinstellungen eines Spiels enthalten Tools, die Sie vielleicht nie benutzen. Im
Executables-Dialog:

- Pin to top setzt einen Eintrag an den Anfang der Run-Liste.
- Hide from picker nimmt einen heraus, ohne ihn zu löschen.
- Desktop shortcut schreibt eine `.desktop` nach `~/.local/share/applications`, sodass sie
  in Ihrem Anwendungsmenü und in einer Suche auftaucht, nicht auf dem Desktop. Sie führt
  `eidos tool <instance> run <title>` direkt aus, sodass das Tool durch die
  zusammengeführte Sicht mit dem Profil dieser Instanz hochkommt, ohne dass das
  Eidos-Fenster offen ist.

Verstecken und Anheften ändern nur, wie ein Tool erreicht wird, sie gelten also für die
Voreinstellungen pro Spiel ebenso wie für Ihre eigenen Einträge.

Der Loader des Script Extenders, der Launcher und die Spielbinärdatei haben keine
Schaltfläche Desktop shortcut. Sie starten das Spiel, und das Spiel startet immer über
seinen Store, was die Verknüpfung des Stores schon erledigt. Eine Verknüpfung, die Sie für
einen davon mit Eidos 1.22 oder älter angelegt haben, funktioniert weiter: Wie
`eidos tool <instance> run <title>` in einem Terminal bittet sie den Store, das Spiel zu
starten, so wie Run in einem Fenster, das kein Store geöffnet hat. Fehlt die Einstellung im
Store, startet der Befehl nichts und gibt aus, was zu setzen ist. Eine Verknüpfung zeigt
keine Meldung; führen Sie denselben Befehl in einem Terminal aus, um sie zu lesen.

Eidos übergibt jeden Eintrag auf diese Weise an den Store, wenn seine ausführbare Datei den
Dateinamen des Script-Extender-Loaders, des Launchers oder der Spielbinärdatei hat, ohne
Rücksicht auf Groß- und Kleinschreibung: eine Voreinstellung, ein selbst hinzugefügter
Eintrag, ein aus MO2 importierter oder ein `--exe`-Lauf. Es markiert zuerst diese Instanz
als die zuletzt verwendete, sodass das Fenster, das der Store öffnet, diese Instanz zeigt,
und führt dann `steam -applaunch <id>`, für Heroic
`xdg-open heroic://launch?appName=<app>&runner=<runner>` oder `legendary launch <app>`
aus. Mit `--print` gibt es diesen Store-Befehl aus und startet nichts. Der Befehl `steam`
muss im `PATH` liegen: Das Flatpak-Steam legt ihn dort nicht ab, sodass der Start scheitert
und die Meldung sagt, das Spiel aus Steam zu starten.

Der Eintrag läuft direkt durch die zusammengeführte Sicht, wie jedes Tool, wenn ein Store
den Prozess gestartet hat: Eine von `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`,
`HEROIC_APP_RUNNER` oder `EIDOS_STORE_STARTED` ist gesetzt. Ohne eine davon startet Eidos
das Spiel nicht selbst mit Proton. Hat die Instanz keinen Script-Extender-Loader (im
Spielordner, aus dem sie läuft, im `Root/` einer aktivierten Mod oder in
`Overwrite/Root/`), startet nichts, und die Zeile sagt, zuerst den Script Extender zu
installieren. Bei einer GOG-Kopie außerhalb von Heroic oder einer Store-Id, die Eidos nicht
weitergeben kann, sagt sie, das Spiel aus diesem Store zu starten.

Ein Profil kann eine eigene Verknüpfung für das Spiel haben. Klicken Sie mit der rechten
Maustaste auf das Profil und wählen Sie Desktop shortcut: Der Eintrag führt
`eidos tool <instance> run "<game>" --profile <name>` aus, was dieses Profil zum aktiven
macht, wie es die Auswahl im Fenster tut, und bittet dann den Store, das Spiel zu starten.
Das Eidos-Fenster, das der Store öffnet, zeigt dieses Profil. `--profile` funktioniert mit
jedem `eidos tool ... run` und wählt das Profil aus, bevor das Tool startet; mit `--print`
wählt es nichts aus.

## Ein Tool, das eine eigene Steam-App ist

Das Creation Kit ist eine eigene Steam-Anwendung und will seine eigene AppID; ein paar
andere über Steam vertriebene Modding-Tools sind genauso. Setzen Sie Steam AppID am
Eintrag, und Eidos startet ihn unter dieser Id statt unter der des Spiels.

Unter Windows bedeutet das einen anderen Launcher. Hier sind es zwei Umgebungsvariablen an
demselben Lauf, `SteamAppId` und `SteamGameId`. Eidos setzt beide, weil Proton die eine
liest und Steams eigene Bibliotheken die andere, und ein Tool, das sie uneins sieht,
scheitert ohne klaren Fehler. `eidos tool ... --print` zeigt genau, was der echte Lauf
bekäme.

## Umgebungsvariablen und Tools ohne Proton

Drei Schlüssel in `<instance>/tools.ini` haben im Executables-Dialog noch kein Feld;
tragen Sie sie von Hand in den Abschnitt des Tools ein. Jede Zeile `env=NAME=value` setzt
eine Variable nur für die Läufe dieses Tools:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos wendet diese Zeilen nach allem an, was es selbst setzt, und nach den Variablen in
der `launch-env.ini` des Profils, also ersetzt ein Wert hier beide. Das gilt auch für das
`WINEDLLOVERRIDES`, das Eidos aus den DLLs baut, die Ihre Mods mitbringen: Ein Tool, das es
setzt, muss darin jede DLL aufführen, die es braucht. Die Wrapper des Profils umgeben den
Befehl eines Tools wie den des Spiels; siehe
[Variablen und Wrapper pro Profil](usage.md#variablen-und-wrapper-pro-profil). Ein Name
besteht aus Buchstaben, Ziffern und `_` und beginnt nicht mit einer Ziffer. Eine Zeile mit
einem anderen Namen wird ignoriert.

`runner=native` startet `exe` selbst, ohne Proton. Die zusammengeführte Sicht wird wie für
jedes Tool eingehängt, und die Zeilen `arg0=`, `arg1=` sind seine Argumente. Nutzen Sie das
für ein Linux-Programm oder für ein Tool zu einer Kopie, die Steam nicht verwaltet und für
die Heroic keinen Runner festhält: `exe` ist das Wine dieser Kopie, das Windows-Programm
ist das erste Argument, und das Präfix kommt aus `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` muss ein absoluter Pfad sein. Das Arbeitsverzeichnis ist standardmäßig der Ordner, in
dem `exe` liegt, hier der des Runners, also setzen Sie `workdir=`. Nur `exe` wird auf seinen
zusammengeführten Pfad umgeschrieben: Für ein Windows-Programm in einer Mod geben Sie
seinen Pfad unter dem `Data`-Ordner des Spiels an, wo die zusammengeführte Sicht es zeigt.
Ein natives Tool überspringt die Proton-Schritte: den Registry-Schlüssel von oben, Steams
Shader-Cache, die leeren .NET-Wurzeln und das Entfernen von Steams Overlay aus
`LD_PRELOAD`.

Bei einer GOG- oder Epic-Kopie, die mit Heroic installiert wurde, brauchen Tools keinen
solchen Eintrag, wenn die Heroic-Einstellungen des Spiels (`GamesConfig/<appName>.json`)
einen Wine- oder Proton-Build nennen: Eidos startet sie mit diesem Build im Heroic-Präfix
des Spiels, und `eidos prereqs <instance> --install` nutzt ihn für die Installer und für
winetricks. Eine nur mit Legendary installierte Kopie hat keine Heroic-Einstellungen.

`slr=1` startet das Tool und den Registry-Import davor im Container der Steam Linux
Runtime, die der Proton-Build des Tools nennt (`require_tool_appid` in seiner
`toolmanifest.vdf`), so wie Steam das Spiel startet. Diese Runtime muss in einer
Steam-Bibliothek installiert sein; sonst bricht der Lauf ab und nennt die Steam-App, die zu
installieren ist. `eidos prereqs` führt seine Installer und winetricks weiterhin auf dem
Host aus, und ein Tool mit `runner=native` ignoriert den Schlüssel. Der Container sieht
Ihren Home-Ordner und die Steam-Bibliothek des Spiels. Ein Tool, eine Mod oder eine Runtime
an anderer Stelle, etwa auf einem anderen Laufwerk, braucht
`env=STEAM_COMPAT_MOUNTS=/dieser/pfad` (mehrere Pfade durch `:` getrennt). Ein Proton-Build
unter `/usr/share/steam/compatibilitytools.d` ist mit `slr=1` vermutlich nicht nutzbar,
weil `/usr` im Container das der Runtime ist. Ein Tool-Lauf im Container wurde noch nicht
getestet.

`eidos tool <instance> run <title> --print` zeigt den endgültigen Befehl und jede Variable,
die der Lauf über der geerbten Umgebung setzt, diese Zeilen eingeschlossen. Bei einem
Eintrag, der das Spiel startet, gibt es stattdessen den Store-Befehl aus (siehe
[Ein Tool erreichen](#an-ein-tool-herankommen-verstecken-anheften-und-eine-desktop-verknüpfung)).

## BethINI Pie

Eidos liefert BethINI Pie nicht mit. Legen Sie die Version von der Nexus-Seite, die Sie
heruntergeladen haben, in den Ordner `mods/` dieser Instanz oder in Ihren Tools-Ordner,
dann erscheint `Bethini.exe` in der Tool-Liste, bei denselben Spielen wie PGPatcher. Über
Eidos gestartet, ändert es die Kopie der Spiel-INIs im Profil. Vor jedem Lauf eines Tools,
dessen Titel `bethini` enthält, legt Eidos einen Wiederherstellungspunkt dieser INIs an;
Backups in der Werkzeugleiste, dann INIs, spielt einen zurück. Eine INI, die bei der
Sicherung noch nicht existierte, bleibt unverändert. Der Dialog behält die 10 neuesten
Wiederherstellungspunkte jeder Art, und das Zurückspielen eines Punkts sichert zuerst die
INIs, wie sie jetzt sind, sodass sich eine falsche Wahl rückgängig machen lässt.

## Die eigenen Einstellungen eines Tools bleiben seine eigenen

Eidos prüft die meisten Tool-Einstellungen nicht; eine falsche scheitert meist ohne
Fehler.

BodySlides Game Data Path (Settings) muss auf das `Data`-Verzeichnis des Spiels zeigen,
nicht auf den Spielordner darüber. Eine Ebene zu hoch gesetzt, meldet ein Batch Build "All
sets processed successfully" und schreibt 1439 Meshes dorthin, wo das Spiel nie nach ihnen
suchen wird. Eidos fängt sie in `Overwrite/Root/` ab, Ihre Spielinstallation bleibt also
unberührt, und das einzige Zeichen im Spiel ist, dass Ihre Körper nicht gebaut sind.

Vor einem Lauf von BodySlide oder Outfit Studio warnt die Statuszeile, wenn `GameDataPath`
oder `OutputDataPath` in der gewinnenden `CalienteTools/BodySlide/Config.xml` während des
Laufs nicht zum `Data`-Ordner des Spiels führt, und der Tab Diagnostics zeigt dieselbe
Warnung als Karte mit dem Wert, den Sie setzen müssen. `eidos check <instance>` meldet sie
ebenfalls. Ein `S:`-Pfad wird so gelesen, wie Eidos `S:` für ein Tool zuordnet: auf den
Ordner der Steam-Bibliothek. Ein `OutputDataPath` im Ordner `mods/` der Instanz, für eine
Ausgabe-Mod, wird akzeptiert. Diagnostics zeigt außerdem eine Karte, wenn `Overwrite/Root/`
einen Ordner `meshes` oder `textures` enthält.

Tool-Ausgaben gehören ins Overwrite. Wenn ein Lauf etwas Erhaltenswertes erzeugt, macht
Overwrite -> Create mod... daraus eine gewöhnliche Mod, die wie jede andere einsortiert,
deaktiviert und entfernt werden kann.
