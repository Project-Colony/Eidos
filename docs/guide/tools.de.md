<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Tools: xEdit, BodySlide, DynDOLOD, FNIS

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
- dem **Tools folder**, den Sie in den Settings festlegen (Tools -> Tools
  folder), für das zwischen Instanzen geteilte Verzeichnis - `/mnt/Games/Tools`
  und dergleichen.

Die Liste gilt pro Spiel, einer Skyrim-Instanz wird also nie Fallouts Editor
angeboten. Die Suche hört vier Ebenen tief auf, weil ein Mod-Bestand
Hunderttausende Dateien umfasst und das bei jedem Aufbau der Tool-Liste läuft,
und sie folgt keinen Symlinks. Ein so gefundenes Tool ist genauso konfiguriert
wie ein selbst eingetragenes: seine Runtimes ergeben sich aus seinem Namen, nach
derselben Regel wie alles Weitere unten.

Liegt ein Tool woanders, oder wollen Sie andere Argumente, tragen Sie es von
Hand ein - ein eigener Eintrag mit demselben Titel überschreibt alles
automatisch Gefundene.

## Eines hinzufügen

In der GUI: **Tools -> Executables**, dann Add. Von der Kommandozeile:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

Der Script Extender, die Spielbinärdatei und der Launcher werden automatisch
erkannt; nur zusätzliche Tools müssen eingetragen werden.

### Zeigen Sie auf die echte Datei, wo immer sie liegt

Tragen Sie die ausführbare Datei dort ein, wo sie tatsächlich liegt. Wurde das
Tool als Mod installiert, ist das im Mod-Ordner:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Drei Arten von Voraussetzung

**Tier 1 - mitgelieferte DLLs** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`).
Eidos liefert sie mit und kopiert sie beim Start ins Präfix. Nichts zu tun, kein
Netz.

**Tier 2 - winetricks-Verben** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Diese schreiben Registry-Schlüssel, den GAC und
CLR-Hosts, lassen sich also nicht per Dateikopie erledigen. Sie **laden von
Microsoft herunter**.

**Tier 3 - Runtimes** (`dotnet10`). Eine moderne .NET-Runtime besteht aus 193
Dateien, die in ihrem eigenen Verzeichnis liegen und über `DOTNET_ROOT` gefunden
werden: nie registriert, überhaupt nie ins Präfix installiert, sodass keine der
anderen Stufen sie tragen kann. Eidos lädt sie selbst herunter, prüft sie gegen
eine in die Binärdatei eingebaute Prüfsumme und legt sie in
`~/.local/share/Colony/Eidos/runtimes/` ab - **außerhalb jeder Instanz**, weil
78 MB weder pro Spiel noch pro Profil anfallen.

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
