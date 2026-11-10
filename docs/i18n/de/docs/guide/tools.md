<!-- eidos-i18n: source=docs/guide/tools.md sha=4cb3ab1260fe32bf41a1a9fdf03479f0c1d74b50 -->

# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Ein durch Eidos gestartetes Tool sieht **den zusammengeführten Blick**, im
Proton-Präfix des Spiels selbst. Es liest, was das Spiel lesen wird - jeden
aktivierten Mod, in Prioritätsreihenfolge - und was immer es schreibt, landet im
Overwrite, wo ein Klick daraus einen echten Mod macht.

## Die, die Eidos von selbst findet

Manche Tools heißen eindeutig genug, um gefunden statt eingetragen zu werden,
und xEdit ist der offensichtliche Fall: `FO4Edit.exe` für Fallout 4,
`SSEEdit.exe` für Skyrim SE, `TES5Edit.exe` für das Original und so weiter -
dazu jeweils der **QuickAutoClean**-Zwilling, der der Knopf für die dirty edits
ist, vor denen LOOT ständig warnt. Eidos sucht danach, nach Dateiname, in:

- dem Installationsordner des Spiels und den `Root/`-Bäumen aktivierter Mods;
- **dem `mods/` dieser Instanz**, wo MO2-Nutzer ihre Tools installieren;
- dem **Tools folder**, den Sie in den Preferences festlegen (General -> Tools),
  für das zwischen Instanzen geteilte Verzeichnis - `/mnt/Games/Tools`
  und dergleichen.

`PGPatcher.exe` wird auf den Creation-Engine-Spielen genauso gefunden, und es kommt mit einem Argument, ohne das es nicht arbeiten kann - siehe [unten](#warum-pgpatcher---ignore-mo2vfscheck-braucht).

Die Liste gilt pro Spiel, einer Skyrim-Instanz wird also nie Fallouts Editor
angeboten. Die Suche hört vier Ebenen tief auf, weil ein Mod-Bestand
Hunderttausende Dateien umfasst und das bei jedem Aufbau der Tool-Liste läuft,
und sie folgt keinen Symlinks. Ein so gefundenes Tool ist genauso konfiguriert
wie ein selbst eingetragenes: seine Runtimes ergeben sich aus seinem Namen, nach
derselben Regel wie alles Weitere unten.

Liegt ein Tool woanders, oder wollen Sie andere Argumente, tragen Sie es von
Hand ein - ein eigener Eintrag mit demselben Titel überschreibt alles
automatisch Gefundene.

## Ein Plugin bereinigen, das LOOT als verschmutzt meldet

Nach einer LOOT-Sortierung listet der Tab Diagnostics die Plugins auf, die LOOT
als verschmutzt (dirty) meldet, für die Spiele, die sowohl ein xEdit als auch
LOOT-Bereinigungsdaten haben (Skyrim LE, SE und VR, Enderal SE, Fallout 3, New
Vegas, Fallout 4 und 4 VR, Oblivion, Starfield). Findet Eidos xEdit, eingetragen
unter Executables oder an einer der Stellen der Instanz, an denen es nach Tools
sucht, hat die Karte Clean all, und jedes verschmutzte Plugin hat Clean with
QuickAutoClean im Menü seiner Zeile; andernfalls sagt die Karte, dass Sie xEdit
unter Executables hinzufügen sollen. Eidos lädt xEdit nicht herunter. Es startet
den QuickAutoClean-Modus von xEdit für jeweils ein Plugin, über den
zusammengeführten Blick, mit `-qac -autoexit -autoload`. xEdit speichert das
bereinigte Plugin über das geladene, was im zusammengeführten Blick eine neue
Datei im Overwrite ist; Eidos verschiebt sie in einen Mod namens `Cleaned
plugins`, der aktiviert am Ende der Liste angelegt wird, wo er jeden Konflikt
gewinnt. Das Plugin in seinem eigenen Mod oder im Data-Ordner des Spiels wird
nie geschrieben. Deaktivieren Sie `Cleaned plugins` oder blenden Sie eine Datei
darin aus, um eine Bereinigung rückgängig zu machen.

Backups, Cache und Log von xEdit landen in `<instance>/logs/qac-<time>/`,
außerhalb des zusammengeführten Blicks, und die Statuszeile meldet die Zahlen
aus diesem Log. Eidos löscht diesen Ordner nach einer gelungenen Bereinigung und
behält ihn nach einer fehlgeschlagenen. Eidos vermerkt Größe und Datum des
Plugins, aus dem jede Kopie entstand, in `Cleaned plugins/meta.ini`. Ändert sich dieses Plugin, meldet der Health-Tab die Kopie als veraltet und bietet an, sie auszublenden.

## Eines hinzufügen

In der GUI: **Tools -> Executables**, dann Add. Von der Kommandozeile:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` fängt die Ausgabe dieses Laufs in `mods/<mod>` ab, unabhängig von der eigenen Einstellung des Tools.

Der Script Extender, die Spielbinärdatei und der Launcher werden automatisch
erkannt; nur zusätzliche Tools müssen eingetragen werden.

### Zeigen Sie auf die echte Datei, wo immer sie liegt

Tragen Sie die ausführbare Datei dort ein, wo sie tatsächlich liegt. Wurde das
Tool als Mod installiert, ist das im Mod-Ordner:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(das ist der Pfad der globalen Instanz - für eine portable Instanz gilt dieselbe
Regel unter ihrem eigenen Ordner, `<instance>/mods/...`; beachten Sie, dass ein
absoluter Pfad wie dieser das Einzige ist, was das spätere VERSCHIEBEN eines
portablen Ordners nicht übersteht).

Eidos schreibt diesen Pfad vor dem Start auf den zusammengeführten um, sodass
das Tool aus `<game>/Data/CalienteTools/BodySlide/` läuft und dort auch die
Dateien jedes anderen Mods sieht. Das wiegt schwerer, als es klingt: BodySlide
liefert ein **leeres** `SliderSets`-Verzeichnis mit, und jeder Körper, den es
bauen kann, stammt aus CBBE und den Outfit-Mods. Aus seinem eigenen Mod-Ordner
gestartet findet es nichts und wirkt kaputt.

MO2 schreibt genauso um, aus demselben Grund - sein eigener Kommentar nennt
FNIS.

Ein Tool in einem **deaktivierten** Mod kann nicht umgeschrieben werden, weil
seine Dateien auch nicht im Blick liegen. Eidos sagt das und startet es aus
seinem eigenen Ordner, statt so zu tun als ob.

## Werkzeuge aus einer Mod-Organizer-2-Liste

Eine Wabbajack-Liste oder eine andere MO2-Instanz, die als Eidos-Instanz geöffnet wird, behält ihre `ModOrganizer.ini`. Die dort eingerichteten Programme lassen sich in `tools.ini` übernehmen:

```sh
eidos tool /path/to/list import-mo2           # zeigt, was hinzukäme
eidos tool /path/to/list import-mo2 --write   # fügt es hinzu
```

Im Fenster hat der Dialog Programme die Schaltfläche **Aus ModOrganizer.ini importieren**, mit einem Kästchen zum Abwählen für jeden Eintrag. Gespeichert wird erst, wenn Sie auf Speichern klicken.

Pfade auf `Z:` sind Linux-Pfade und bleiben unverändert. Ein Pfad auf einem anderen Laufwerksbuchstaben wird unter den Ordner der Liste verschoben, wenn er diesen Ordner nennt (`D:\Lists\MyList\tools\SSEEdit.exe` wird zu `<list>/tools/SSEEdit.exe`), und zwar in der ausführbaren Datei, im Arbeitsverzeichnis und in den Argumenten. Der Loader des Script Extenders und die Spieldatei werden zu schlichten Namen und laufen so aus dem Spielordner, über den Eidos die Ansicht legt, wie die eingebauten Einträge. Einträge, die woanders hinzeigen (ein Windows-Systemprogramm, ein Werkzeug außerhalb der Liste), erscheinen mit Begründung als übersprungen, und ein Titel, den `tools.ini` schon enthält, wird nie ersetzt.

## Die Ausgabe eines Tools in einen eigenen Mod schicken

Ein Generator - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - schreibt
Hunderte Dateien. Standardmäßig landen sie mit allem anderen im Overwrite.
Setzen Sie **Capture output into** im Executables-Editor, und die Ausgabe dieses
Laufs geht stattdessen in jenen Mod:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Der Mod wird angelegt, falls er nicht existiert. Nur die Dateien, die DIESER
Lauf erzeugt hat, werden verschoben; was schon vorher im Overwrite lag, bleibt
dort, sodass zwei Tools mit Capture-Zielen sich nicht gegenseitig die Ausgabe
klauen. Ein Lauf, der nichts geschrieben hat, hinterlässt keinen leeren Mod.

Das geschieht nach dem Lauf, statt die Schreibschicht auf den Mod zu zeigen, wie
MO2 es macht. Die Schreibschicht auf einen Mod zu zeigen würde ihn für den
ganzen Lauf auf höchste Priorität heben - jeden Konflikt, in dem er steckt,
umkippen und danach wieder zurück - und würde ohne Copy-up direkt durch die
eigenen Dateien des Mods schreiben. Das Capture erreicht denselben Endzustand
ohne beides.

Ist der Ziel-Mod deaktiviert, wird die Ausgabe zwar geschrieben, aber das Spiel
sieht sie nicht, das Tool würde also beim nächsten Lauf dieselben Dateien neu
erzeugen. Eidos warnt, wenn das der Fall ist.

### Wenn seine Ausgabe veraltet ist

Eidos merkt sich, aus welchen Mods, welcher Plugin-Reihenfolge, welchem Profil und welcher Programmdatei die Ausgabe eines Tools entstanden ist. Ändert sich eines davon, meldet das der Reiter Diagnostics, und die Karte hat einen Knopf **Run <tool> again**, der das Tool mit der jetzigen Konfiguration startet.

Für einen Generator, dessen veraltete Ausgabe das Spiel kaputtmacht, etwa Pandora oder Nemesis, tragen Sie `check_before_play=1` in seinen Eintrag in `tools.ini` ein oder schalten im Dialog Executables **Ask before playing when its output is out of date** ein. Run fragt dann zuerst: das Tool starten oder trotzdem spielen. Die Entscheidung, trotzdem zu spielen, gilt, bis sich Mods, Plugins oder Profil wieder ändern oder das Fenster geschlossen wird. Eidos startet das Tool nie von selbst.

## Die DLLs, die ein Tool braucht, werden über seinen NAMEN gewählt

Das ist der überraschende Teil, deshalb sei es klar gesagt: **Der Titel, den Sie
einem Tool geben, entscheidet, welche Laufzeit-Voraussetzungen Eidos dafür
bereitstellt.** Verglichen wird als Teilzeichenkette des Titels, ohne Rücksicht
auf Groß- und Kleinschreibung.

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

Ein als **`BodySlide`** eingetragenes Tool bekommt also seine DirectX-DLLs;
dieselbe ausführbare Datei als **`BS`** eingetragen bekommt nichts und startet
womöglich nicht, mit einem Fehler, der nichts über DLLs sagt. Benennen Sie Tools
nach dem Programm.

Die Liste steht in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), und
das Feld `Prereqs` im Executables-Dialog ist editierbar - die Erkennung ist eine
Vorgabe, keine Regel.

Der Titel entscheidet genauso ueber **Argumente**, fuer das eine Werkzeug, das
eines braucht, das niemand erraten koennte: ein Titel mit `pgpatcher` (oder
`parallaxgen`, dem frueheren Namen) erhaelt `--ignore-mo2vfscheck`. Warum, und
warum es nicht optional ist, steht [unten](#warum-pgpatcher---ignore-mo2vfscheck-braucht).

### Drei Arten von Voraussetzung

**Tier 1 - mitgelieferte DLLs** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos liefert sie mit und kopiert sie beim Start ins Präfix. Nichts zu tun, kein
Netz.

**Tier 2 - winetricks-Verben** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Diese schreiben Registry-Schlüssel, den GAC und
CLR-Hosts, lassen sich also nicht per Dateikopie erledigen. Sie **laden von
Microsoft herunter**.

**Tier 3 - Runtimes** (`dotnet10`, `dotnetsdk10`). Eine moderne .NET-Runtime besteht aus 193
Dateien, die in ihrem eigenen Verzeichnis liegen und über `DOTNET_ROOT` gefunden
werden: nie registriert, überhaupt nie ins Präfix installiert, sodass keine der
anderen Stufen sie tragen kann. Eidos lädt sie selbst herunter, prüft sie gegen
eine in die Binärdatei eingebaute Prüfsumme und legt sie in
`~/.local/share/Colony/Eidos/runtimes/` ab - **außerhalb jeder Instanz**, weil
78 MB weder pro Spiel noch pro Profil anfallen.

`dotnetsdk10` ist das .NET-10-SDK: 301 MB Download, entpackt 808 MB. Synthesis
braucht es, um seine Patcher zu kompilieren, und ruft zum Bauen und Starten ein
nacktes `dotnet` auf. Ein Tool, das `dotnetsdk10` angibt, bekommt deshalb auch
den SDK-Ordner in `WINEPATH` sowie `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` und `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Ein Patcher-Build ließe MSBuild-Knoten und den VBCSCompiler-Server weiterlaufen,
und unter Wine halten sie den wineserver des Präfixes am Leben, deshalb bekommt
das Tool auch `MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` und
`UseSharedCompilation=false`. Mit `DOTNET_ROLL_FORWARD=Major` läuft ein für .NET
8 gebauter Patcher (Mutagen vor 0.54) auf der .NET-10-Runtime des SDK, wenn
keine .NET-8-Runtime gefunden wird.
Ein Synthesis-Eintrag in `tools.ini`, der noch den alten Standard `dotnet8`,
`vcrun2022` enthält, bekommt beim Einlesen durch Eidos `dotnetsdk10`,
`vcrun2022`.

NuGet prüft die Signatur jedes Pakets, das es wiederherstellt, gegen den
Windows-Stammzertifikatspeicher, und einem Wine-Präfix fehlen die
Stammzertifikate für Codesignatur und Zeitstempel, die das SDK als
`sdk/10.0.403/trustedroots/codesignctl.pem` und `timestampctl.pem` mitbringt.
Ohne sie scheitert die Wiederherstellung von `Mutagen.Bethesda.Synthesis` 0.36.6
mit `NU3018` und `NU3028`. Deshalb fügt Eidos, bevor ein Tool mit `dotnetsdk10`
startet und sobald das SDK heruntergeladen ist, die 385 Zertifikate beider
Dateien dem Stammzertifikatspeicher des aktuellen Benutzers im Präfix des Spiels
hinzu, im selben `regedit`-Import, der den Spielpfad schreibt (siehe unten).
Fehlt später eines, importiert Eidos sie erneut. Jedes Windows-Programm in
diesem Präfix vertraut diesen Stammzertifikaten dann. Mit ihnen gelingen in
einem frischen Präfix mit leerem NuGet-Cache dieselbe Wiederherstellung und der
Build, mit dem System-Wine 11.21 und mit GE-Proton11-9.

Nichts in Tier 2 oder 3 läuft stillschweigend:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

In der GUI stehen dieselben Zustände unter dem Prereqs-Feld, und die fehlenden
sind Knöpfe. Ein Verb, das weder mitgeliefert noch eine Runtime noch ein
bekanntes winetricks-Verb ist, wird als wahrscheinlicher Tippfehler gemeldet
statt als Download angeboten.

### Warum DynDOLOD `dotnet10` braucht

DynDOLOD baut Object LOD nicht selbst: es ruft LODGen auf, und es liefert drei
davon mit. `LODGenx64.exe` zielt auf .NET Framework 4.8, das unter Proton auf
Wines Mono umgeleitet wird - dessen `System.Uri`-Initialisierer eine Methode
aufruft, die Mono nicht implementiert. Es stirbt vor seiner ersten Zeile Arbeit
und hinterlässt ein Log mit einem Versionsbanner und sonst nichts, und einen
DynDOLOD-Dialog, der nur "failed for one or more worlds" sagt.

Das echte .NET Framework zu installieren behebt es nicht: Proton ersetzt
`mscoree.dll` - den Loader, der es finden würde - durch einen Symlink in seinen
eigenen Baum, und macht das bei jedem Präfix-Update erneut.

Der Build, der funktioniert, ist `LODGenx64Win10.exe`, der auf modernes .NET
zielt und `mscoree` nie anfasst. Zeigen Sie `DOTNET_ROOT` auf eine
.NET-10-Runtime, und er läuft. Genau das stellt `dotnet10` bereit, und Eidos
setzt die Variable beim Start jedes Tools, das sie deklariert.

Eidos führt das `winetricks` des Systems gegen Protons eigenes `wine` und das
Spiel-Präfix aus, was Steams pressure-vessel-Container und die Unstimmigkeit
zwischen protontricks und Proton-GE umgeht. Ein Tool, das ein nicht
installiertes Tier-2-Verb deklariert, startet trotzdem, mit einer Warnung, die
das Verb und den Befehl zur Behebung nennt - der Nutzer hat es vielleicht
anderswoher.

### Warum PGPatcher `--ignore-mo2vfscheck` braucht

PGPatcher schreibt Meshes und Plugins so um, dass die vorhandenen Textur-Mods -
vanilla, Parallax, Complex Material, PBR - auf jeder Oberflaeche den richtigen
Shader verwenden. Es weigert sich zu Recht, ausserhalb eines Mod-Managers zu
laufen: einen nackten Spielordner zu patchen waere zerstoererisch.

Das Problem ist, WIE es das prueft: es sucht **usvfs**, die DLL-Hooking-Schicht,
die MO2 unter Windows ins Spiel injiziert. Eidos hat keine und wird nie eine
haben - die zusammengefuehrte Ansicht ist ein FUSE-Mount, in das Wine laeuft,
ohne davon zu wissen - also findet die Pruefung nichts und das Werkzeug beendet
sich sofort mit

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

und nennt dabei ein Programm, das die Nutzerin gar nicht ausfuehrt, noch vor der
ersten Arbeit. Die Antwort des Autors auf die beiden Linux-Meldungen
([#716][pg716] und [#725][pg725], beide von Fluorine, MO2s Linux-Fork, der aus
demselben Grund FUSE nutzt) ist ein Schalter, der die Pruefung ueberspringt.

Eidos setzt ihn daher vor. Ein im Mod-Pool gefundener PGPatcher kommt mit dem
Argument bereits gesetzt, und `PGPatcher` als Titel im Executables-Dialog fuellt
das Arguments-Feld genauso. Was Sie tippen, gewinnt immer: vorbelegt wird nur
ein Feld, das Sie leer gelassen haben, genau wie bei den Voraussetzungen oben.

Die Liste steht in `default_args` (`crates/eidos-instance/src/tools.rs`), neben
`default_prereqs`. Sie hat einen Eintrag, und einer genuegt fuer eine eigene
Funktion: einen Schalter kann niemand erraten, und der verhinderte Fehler nennt
das falsche Programm.

Zur Laufzeit braucht es nichts, ungewoehnlich fuer ein .NET-Werkzeug: es bringt
seine eigene mit (`dotnetlib/`, mit einer `runtimeconfig.json`, die
`includedFrameworks` deklariert) und liefert die Microsoft-CRT-DLLs neben der
ausfuehrbaren Datei. Lassen Sie Prereqs leer.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher und PGLite im Modus Mod Organizer 2

Mit Mod Manager auf `None` sieht PGPatcher einen einzigen zusammengeführten
`Data`-Ordner und kann nicht erkennen, aus welchem Mod ein Mesh oder eine Textur
stammt. Es hat also keine Konflikte anzuzeigen und keine Mod-Reihenfolge, der es
folgen könnte. Sein Modus `Mod Organizer 2` liest die Instanz selbst: die
`modlist.txt` des Profils, die aktivierten Mods in ihrer Prioritätsreihenfolge
und die Dateien im Ordner jedes Mods. PGLite, die Version im Spiel, liest die
Instanz auf dieselbe Weise. Eine Eidos-Instanz hat bereits das Layout von MO2
(`mods/`, `profiles/<name>/modlist.txt`, `overwrite/`), daher funktioniert
dieser Modus mit ihr.

Im Launcher von PGPatcher:

1. Stellen Sie **Mod Manager** auf `Mod Organizer 2`.
2. Stellen Sie **MO2 Options -> Instance Location** auf den Ordner der
   Eidos-Instanz, geschrieben als Windows-Pfad auf Wines Laufwerk `Z:`, das `/`
   ist. Eine portable Instanz unter `/mnt/Games/Eidos-Skyrim` wird zu
   `Z:\mnt\Games\Eidos-Skyrim`; eine globale wird zu
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Behalten Sie `--ignore-mo2vfscheck` in den Argumenten des Tools. In diesem
   Modus sucht PGPatcher nach usvfs und bricht ab, wenn es fehlt.

PGPatcher akzeptiert den Ordner nur, wenn er eine `modorganizer.ini` enthält.
Deshalb schreibt Eidos bei jedem Start des Spiels oder eines Tools eine
`ModOrganizer.ini` in das Wurzelverzeichnis der Instanz, für das gestartete
Profil. Die Datei wird nur neu geschrieben, wenn sich ihr Inhalt ändert. Sie
enthält:

- in `[General]`: `gameName` (der Name des Spiels in MO2, also `New Vegas` für Fallout: New Vegas), `gamePath` (den Installationsordner des Spiels als
  `Z:`-Pfad) und `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` und
  `overwrite_directory` als absolute `Z:`-Pfade, dazu `download_directory`, wenn
  die Instanz einen Ordner `downloads/` hat.

Ihre erste Zeile ist ein Kommentar, der sagt, dass Eidos sie geschrieben hat.
Eine `ModOrganizer.ini` ohne diese Zeile, in welcher Schreibweise des Namens
auch immer, wird nie geändert oder gelöscht: Eine MO2-Instanz, die Sie in Eidos
geöffnet haben, behält ihre eigene, und PGPatcher liest diese Datei so, wie sie
ist. MO2 verwirft Kommentare, wenn es seine Einstellungen speichert, also gehört
eine Datei, über die MO2 gespeichert hat, von da an MO2. Eine leere Datei gilt als die von Eidos.

Eine solche Datei nennt das Profil, das MO2 zuletzt ausgewählt hat, und
PGPatcher verwendet dieses Profil, nicht das, das Eidos startet. Wenn sich die
beiden unterscheiden, protokolliert Eidos beim Start eine Warnung. Wählen Sie in
MO2 dasselbe Profil, oder benennen Sie die Datei um oder entfernen Sie sie,
damit Eidos seine eigene schreibt.

Eidos schreibt seine Datei nur, wenn es etwas startet. Zwischen zwei Starts nennt
sie also das Profil des letzten. Ein Tool, das Eidos startet, findet dort
sein eigenes Profil.

PGPatcher bricht mit einem Fehler ab, wenn das Profil keinen aktivierten Mod hat
und wenn sein Ausgabeordner der Ordner eines aktivierten Mods ist, so wie unter
MO2.

## Wines eigene Programme im Prefix des Spiels

Das Menü Datei startet vier Programme von Wine im Prefix des Spiels: winecfg, regedit, den Wine-Dateimanager und die Deinstallation. Sie starten wie jedes Werkzeug, mit dem Proton- oder Wine-Build, der die Werkzeuge des Spiels ausführt, bei eingehängten Mods, und das Fenster wartet auf sie. **Prefix drive_c** öffnet das Laufwerk `C:` des Prefix in Ihrem Dateimanager. Eine Zeile bleibt grau, bis das Prefix das Programm enthält. Das geschieht, wenn der Store das Spiel zum ersten Mal startet.

Eidos schreibt selbst einen einzigen Registrywert, den `installed path` des Spiels unter `HKLM\Software\Bethesda Softworks\<game>`, vor jedem Werkzeuglauf, wenn er falsch ist. Eine Änderung daran in regedit wird beim nächsten Werkzeuglauf zurückgesetzt.

## Der Spielpfad im Präfix

Windows-Tools finden ihr Spiel, indem sie
`HKLM\Software\Bethesda Softworks\<game>` `installed path` lesen, einen
Schlüssel, den der Installer des Spiels selbst schreibt - und den Steam unter
Proton nie ausführt. Ohne ihn öffnen xEdit, Wrye Bash und DynDOLOD auf einem
leeren Pfad. Eidos schreibt ihn, bevor es ein Tool startet: idempotent, additiv,
und übersprungen, wenn das Präfix nicht initialisiert ist oder gerade benutzt
wird.

## An ein Tool herankommen: verstecken, anheften und eine Desktop-Verknüpfung

Die Voreinstellungen eines Spiels enthalten Tools, die Sie vielleicht nie
benutzen, und ein Auswahlmenü, das acht Einträge auflistet, um an den zweiten zu
kommen, ist ein Auswahlmenü, das niemand liest. Im Executables-Dialog:

- **Pin to top** setzt einen Eintrag an den Anfang der Run-Liste.
- **Hide from picker** nimmt einen heraus, ohne ihn zu löschen.
- **Desktop shortcut** schreibt eine `.desktop` nach
  `~/.local/share/applications` - wo ein Starter auf einem freedesktop-System
  hingehört, sodass er in Ihrem Anwendungsmenü und in einer Suche auftaucht
  statt auf dem Desktop. Sie führt `eidos tool <instance> run <title>` direkt
  aus, was heißt, dass das Tool **durch den zusammengeführten Blick mit dem
  Profil dieser Instanz** hochkommt, ohne dass das Eidos-Fenster überhaupt offen
  ist.

Verstecken und Anheften betreffen, wie ein Tool *erreicht* wird, nicht was es
startet, sie gelten also für die Voreinstellungen pro Spiel ebenso wie für Ihre
eigenen Einträge.

Der Loader des Script Extenders, der Launcher und die Spielbinärdatei haben
keine Schaltfläche Desktop shortcut. Sie starten das Spiel, und das Spiel
startet immer über seinen Store, was die Verknüpfung des Stores schon erledigt.
Eine Verknüpfung, die Sie für einen davon mit Eidos 1.22 oder älter angelegt
haben, funktioniert weiter: Wie `eidos tool <instance> run <title>` in einem
Terminal bittet sie den Store, das Spiel zu starten, so wie Run in einem
Fenster, das kein Store geöffnet hat. Fehlt die Einstellung im Store, startet
der Befehl nichts und gibt aus, was zu setzen ist. Eine Verknüpfung zeigt keine
Meldung; führen Sie denselben Befehl in einem Terminal aus, um sie zu lesen.

Ein Profil kann eine eigene Verknüpfung für das Spiel haben. Klicken Sie mit der
rechten Maustaste auf das Profil und wählen Sie **Desktop shortcut**: Der
Eintrag führt `eidos tool <instance> run "<game>" --profile <name>` aus, was
dieses Profil zum aktiven macht, wie es die Auswahl im Fenster tut, und bittet
dann den Store, das Spiel zu starten. Das Eidos-Fenster, das der Store öffnet,
zeigt dieses Profil. `--profile` funktioniert mit jedem `eidos tool ... run` und
wählt das Profil aus, bevor das Tool startet; mit `--print` wählt es nichts aus.

## Ein Tool, das eine eigene Steam-App ist

Das Creation Kit ist eine eigene Steam-Anwendung und will seine eigene AppID;
ein paar andere über Steam vertriebene Modding-Tools sind genauso. Setzen Sie
**Steam AppID** am Eintrag, und Eidos startet ihn unter dieser Id statt unter
der des Spiels.

Unter Windows bedeutet das einen anderen Launcher. Hier sind es zwei
Umgebungsvariablen an dem Lauf, der ohnehin schon gebaut wurde - `SteamAppId`
und `SteamGameId`, beide, weil Proton die eine liest und Steams eigene
Bibliotheken die andere, und ein Tool, das sie uneins sieht, scheitert
eigenartig statt deutlich. `eidos tool ... --print` zeigt genau, was der echte
Lauf bekäme.

## Umgebungsvariablen und Tools ohne Proton

Drei Schlüssel in `<instance>/tools.ini` haben im Executables-Dialog noch kein
Feld; tragen Sie sie von Hand in den Abschnitt des Tools ein. Jede Zeile
`env=NAME=value` setzt eine Variable nur für die Läufe dieses Tools:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos wendet diese Zeilen nach allem an, was es selbst setzt, also ersetzt ein
Wert hier den eigenen. Das gilt auch für das `WINEDLLOVERRIDES`, das Eidos aus
den DLLs baut, die Ihre Mods mitbringen: Ein Tool, das es setzt, muss darin
jede DLL aufführen, die es braucht. Ein Name besteht aus Buchstaben, Ziffern und
`_` und beginnt nicht mit einer Ziffer. Eine Zeile mit einem anderen Namen wird
ignoriert.

`runner=native` startet `exe` selbst, ohne Proton. Der zusammengeführte Blick
wird wie für jedes Tool eingehängt, und die Zeilen `arg0=`, `arg1=` sind seine
Argumente. Nutzen Sie das für ein Linux-Programm oder für ein Tool zu einer
Kopie, die Steam nicht verwaltet und für die Heroic keinen Runner festhält:
`exe` ist das Wine dieser Kopie, das Windows-Programm ist das erste Argument,
und das Präfix kommt aus `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` muss ein absoluter Pfad sein. Das Arbeitsverzeichnis ist standardmäßig
der Ordner, in dem `exe` liegt, hier der des Runners, also setzen Sie
`workdir=`. Nur `exe` wird auf seinen zusammengeführten Pfad umgeschrieben: Für
ein Windows-Programm in einem Mod geben Sie seinen Pfad unter dem `Data`-Ordner
des Spiels an, wo der zusammengeführte Blick es zeigt. Ein natives Tool
überspringt die Proton-Schritte: den Registry-Schlüssel von oben, Steams
Shader-Cache, die leeren .NET-Wurzeln und das Entfernen von Steams Overlay aus
`LD_PRELOAD`.

Bei einer GOG- oder Epic-Kopie, die mit Heroic installiert wurde, brauchen
Tools keinen solchen Eintrag, wenn die Heroic-Einstellungen des Spiels
(`GamesConfig/<appName>.json`) einen Wine- oder Proton-Build nennen: Eidos
startet sie mit diesem Build im Heroic-Präfix des Spiels, und
`eidos prereqs <instance> --install` gibt ihn an winetricks weiter. Eine nur
mit Legendary installierte Kopie hat keine Heroic-Einstellungen.

`slr=1` startet das Tool und den Registry-Import davor im Container der Steam
Linux Runtime, die der Proton-Build des Tools nennt (`require_tool_appid` in
seiner `toolmanifest.vdf`), so wie Steam das Spiel startet. Diese Runtime muss
in einer Steam-Bibliothek installiert sein; sonst bricht der Lauf ab und nennt
die Steam-App, die zu installieren ist. `eidos prereqs` führt winetricks
weiterhin auf dem Host aus, und ein Tool mit `runner=native` ignoriert den
Schlüssel. Der Container sieht Ihren Home-Ordner und die Steam-Bibliothek des
Spiels. Ein Tool, ein Mod oder eine Runtime an anderer Stelle, etwa auf einem
anderen Laufwerk, braucht `env=STEAM_COMPAT_MOUNTS=/dieser/pfad` (mehrere Pfade
durch `:` getrennt). Ein Proton-Build unter
`/usr/share/steam/compatibilitytools.d` ist mit `slr=1` vermutlich nicht
nutzbar, weil `/usr` im Container das der Runtime ist. Ein Tool-Lauf im
Container wurde noch nicht getestet.

`eidos tool <instance> run <title> --print` zeigt den endgültigen Befehl und
jede Variable, die der Lauf über der geerbten Umgebung setzt, diese Zeilen
eingeschlossen.

## Die eigenen Einstellungen eines Tools bleiben seine eigenen

Eidos setzt ein Tool an die richtige Stelle, mit den richtigen DLLs. Was das
Tool dann mit seiner Konfiguration macht, ist eine Sache zwischen Ihnen und dem
Tool, und das Scheitern ist meist stillschweigend.

Das durchgerechnete Beispiel, weil es sonst eine Stunde kostet: BodySlides
**Game Data Path** (Settings) muss auf das `Data`-Verzeichnis des Spiels zeigen,
nicht auf den Spielordner darüber. Eine Ebene zu hoch gesetzt, meldet ein Batch
Build "All sets processed successfully" und schreibt 1439 Meshes dorthin, wo das
Spiel nie nach ihnen suchen wird. Eidos fängt sie ab - sie landen in
`Overwrite/Root/` statt in Ihrer Installation - aber aus Sicht des Spiels ist
nichts falsch, außer dass Ihre Körper nicht gebaut sind.

Tool-Ausgaben gehören ins Overwrite. Wenn ein Lauf etwas Erhaltenswertes
erzeugt, macht **Overwrite -> Create mod...** daraus einen gewöhnlichen Mod, der
wie jeder andere einsortiert, deaktiviert und entfernt werden kann.
