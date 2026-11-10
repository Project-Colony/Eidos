<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 durch Eidos

Mit Eidos braucht Fallout 4 keine besondere Startoption, keine umbenannte
ausführbare Datei und kein Wrapper-Skript. Es nimmt dieselbe Startoption wie
jedes andere Spiel.

## Die Startoption

```
~/.local/bin/eidos-gui %command%
```

Steams Startziel für Fallout 4 ist `Fallout4Launcher.exe`, nie `Fallout4.exe`; den
Script Extender überhaupt zum Laufen zu bringen ist also in Wahrheit die Frage "wie
bringe ich Steam dazu, ein anderes Programm zu starten". Die üblichen Antworten
schreiben `%command%` in bash um:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

oder kopieren `f4se_loader.exe` über `Fallout4Launcher.exe`, was Steam bei jedem
Spiel-Update wiederherstellt - danach spielen Sie ohne F4SE, und nichts sagt es
Ihnen.

Eidos nimmt den Tausch selbst vor, aus dem Spieldeskriptor. Wenn das Fenster das
Spiel startet, ersetzt es `Fallout4Launcher.exe` durch `f4se_loader.exe`, wenn der
Loader im Spielordner, im `Root/` einer aktivierten Mod oder in `Overwrite/Root/`
liegt. Ohne den Loader führt es `Fallout4.exe` aus, und die Statuszeile sagt
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Findet es dort keine der beiden Dateien, führt es den Launcher unverändert aus
und sagt auch das. Ist stattdessen `eidos play fallout4 -- %command%` die
Startoption, tauscht die CLI nur, wenn der Loader existiert, und führt sonst
`Fallout4Launcher.exe` aus.

Es gibt einen zweiten Grund, den Launcher zu meiden: er durchsucht `Data` erneut
und schreibt `plugins.txt` neu, wodurch die gerade ausgerollte Ladereihenfolge
zunichtegemacht wird.

Wenn Sie das Fenster selbst öffnen und Run drücken, bittet Eidos Steam, das Spiel
zu starten (`steam -applaunch 377160`), und Steam startet Eidos über die
Startoption oben. Liegt `f4se_loader.exe` nicht neben der ausführbaren Datei des
Spiels in der Instanz, tut es das nicht, und die Statuszeile bittet Sie, zuerst
den Script Extender zu installieren. Fallout 4 VR funktioniert genauso, mit
`Fallout4VRLauncher.exe` und `f4sevr_loader.exe`.

## Was Eidos für Sie erledigt

| Bereich | Was Eidos tut |
|---|---|
| Archiv-Invalidierung | `Fallout4Custom.ini` bekommt `[Archive]` `bInvalidateOlderFiles=1` und ein leeres `sResourceDataDirsFinal=` - die beiden Schlüssel, die lose Dateien außerhalb von `Data` überhaupt sichtbar machen. Ins Profil geschrieben, nicht in den Spielordner. |
| INIs | `Fallout4.ini`, `Fallout4Prefs.ini` und `Fallout4Custom.ini` gehören zum Profil. BethINI Pie, über Eidos ausgeführt, bearbeitet die Kopien des Profils, und Backups -> INIs stellt die Kopie wieder her, die Eidos vor jedem seiner Läufe anlegt ([tools.md](tools.md#bethini-pie)). |
| Ladereihenfolge | `plugins.txt` im Sternchen-Format, das Fallout 4 verwendet (`*` markiert aktiv), mit beachtetem `Fallout4.ccc` für die impliziten Creation-Club-Plugins |
| LOOT | Das Sortieren funktioniert wie bei Skyrim - `eidos sort <instance>` holt die `fallout4`-Masterlist |
| Spielstände | `.fos`-Spielstände und ihre `.f4se`-Cosaves werden aufgelistet, kopiert und je Profil gehalten; die Detailansicht liest die Plugin-Tabelle des Spielstands, sodass ein Spielstand, der ein von Ihnen deaktiviertes Plugin braucht, das sagt, bevor Sie ihn laden. Eine Mod zu deaktivieren oder zu entfernen, deren Plugin einer der drei neuesten Spielstände des Profils nutzt, fragt vorher nach. |
| Root-Mods | Alles, was eine Mod neben der ausführbaren Datei mitbringt (F4SE selbst, ENB, eine `dxvk.conf`), landet dort über denselben `Root/`-Mechanismus wie bei Skyrim |

## Die Versionsfrage

Fallout 4 ist nicht mehr das eingefrorene Spiel von 2019 bis 2024. Stand August 2026
gibt es drei lebende Zweige, und eine für einen davon gebaute Mod-DLL lädt auf einem
anderen nicht:

| Zweig | Version | F4SE |
|---|---|---|
| Klassisch ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 bis 1.11.240 | 0.7.4 bis 0.7.9 |

Zwei Folgerungen, die man vor dem Bauen einer Modliste kennen sollte:

- Ordner `Creations/` und `Mods/` im Spielverzeichnis bedeuten, dass Sie auf der
  1.11.x-Linie sind. Die Detailansicht eines Spielstands in Eidos zeigt außerdem
  den Build, der ihn geschrieben hat: Fallout schreibt das in den Spielstand, und
  Eidos zeigt es als "Game build".
- F4SE erscheint meist ein bis zwei Tage nach einem Bethesda-Update, aber *Address
  Library for F4SE Plugins* - über das die meisten DLL-Mods ihre Offsets
  auflösen - folgt seinem eigenen Zeitplan. Dazwischen laden DLL-Mods nicht. Mods
  ohne DLL (Texturen, Meshes, Plugins) sind nicht betroffen, ein frischer Patch
  ist also ein schlechter Tag, um eine Modliste anzufangen.

Sobald Ihr Aufbau läuft, frieren Sie die Spielversion in der Instanz ein: Freeze
this version auf der Karte Game files im Tab Diagnostics, oder
`eidos stock-game fallout4 freeze`, um die Größe zu sehen, und
`eidos stock-game fallout4 freeze --yes`, um zu kopieren. Eidos kopiert die
Store-Installation nach `<instance>/Stock Game` und startet das Spiel aus dieser
Kopie, weiterhin von Steam gestartet. Ein späterer Patch ändert die
Store-Installation und nicht die Kopie, sodass die DLLs, die Sie installiert
haben, ihre Spielversion behalten. Auf btrfs oder XFS teilt die Kopie die Blöcke
des Stores und braucht fast keinen Platz, bis Steam eine Datei ändert; auf ext4 ist
es eine vollständige Kopie so groß wie der Spielordner. Eidos lehnt ab, solange
Steam das Spiel als beschäftigt oder nicht vollständig installiert meldet. Die
Einzelheiten stehen in
[usage.md](usage.md#spieldateien-die-store-installation-oder-eine-kopie).

Ohne Kopie schalten Sie Steams automatische Updates für Fallout 4 ab
(Eigenschaften, Updates, "Dieses Spiel nur beim Start aktualisieren"), sonst
zerlegt der nächste Patch jede installierte DLL.

## Hardware-Hinweis: Waffentrümmer stürzen auf NVIDIA ab

Der Waffentrümmer-Effekt von Fallout 4 läuft auf NVIDIA FleX, einem
PhysX-Ableger, den NVIDIA nach der Pascal-Generation nicht mehr unterstützt hat. Auf
jeder Turing-Karte oder neuer - GTX 16, RTX 20 bis RTX 50 - stürzt das Spiel ab. Das
ist ein Spielfehler und hat nichts mit Linux, Proton oder Eidos zu tun.

Zwei Abhilfen, beide wirken: "Weapon Debris" in den Spieleinstellungen ausschalten,
oder *Weapon Debris Crash Fix* (Nexus 48078) installieren, das die Kollision der
Fragmente abschaltet statt den Effekt.

## Health-Karten für Fallout 4

Der Tab Diagnostics zeigt für jeden dieser Fälle eine Health-Karte:

- F4SE-Plugins liegen in `F4SE/Plugins`, aber `f4se_loader.exe` steckt in keiner
  aktivierten Mod und liegt nicht im Spielordner. Das Spiel startet ohne sie.
- `f4se_loader.exe` ist installiert, aber die `f4se_<version>.dll` für die
  laufende Spielversion (`f4se_1_10_984.dll` für 1.10.984) nicht. Der Loader bricht
  mit einem Fehler ab; das folgt meist auf ein Spiel-Update.
- Die ausführbare Datei des Spiels oder ihr Steam-Build hat sich seit dem letzten
  Start über Eidos geändert, der ohne Fehler endete. F4SE-Plugins, die für die
  alte Version gebaut wurden, laden womöglich nicht mehr.
- Eine Mod mit höherer Priorität als F4SE hat lose `.pex`-Kopien von F4SEs
  eigenen Skripten. Eine alte Kopie zerstört die F4SE-Funktionen, die sie
  deklariert, und das Spiel zeigt keinen Fehler.
- Die `Fallout4Custom.ini` des Profils hat Zeilen `sTestFile1` bis `sTestFile10`
  in `[General]`. Solange eine gesetzt ist, lädt das Spiel diese Plugins und
  ignoriert `plugins.txt`. Fix entfernt sie und behält die vorherige Datei als
  `Fallout4Custom.ini.pre-fix`.
- Das BA2-Archiv einer Mod speichert `.wav`- oder `.xwm`-Audio komprimiert. Das
  Spiel kann abstürzen, wenn es diesen Ton abspielt. Packen Sie das Archiv mit
  unkomprimiertem Audio neu. Fallout 4 VR bekommt dieselbe Karte.
- Ein Crash-Logger hat beim letzten Start ein Log geschrieben: `crash-*.log` von
  Buffout 4 in `Documents/My Games/Fallout4/F4SE` oder eine `.log` in
  `F4SE/Crashlogs`. Die Karte nennt die Ausnahme und die DLLs und Plugins im Log,
  die Ihre Mods installieren.
- Die Karte Display zeigt Fenstergröße und -modus aus der `Fallout4Prefs.ini` des
  Profils, und Apply schreibt dort `iSize W`, `iSize H`, `bFull Screen` und
  `bBorderless`.
- Eine Nexus-Anforderung nennt ein DLC. Eidos sucht ein Plugin mit dem Namen des
  DLC, und die DLC-Plugins von Fallout 4 heißen anders (`DLCCoast.esm` für Far
  Harbor), diese Karte ist also ein Hinweis: Prüfen Sie von Hand, dann drücken Sie
  Ignore.

Die beiden Loader-Karten haben einen Knopf Open the download page für
<https://f4se.silverlock.org/> und einen Knopf Install the downloaded archive, der
den Loader in das `Root/` der Mod legt. `eidos check fallout4` gibt dieselben
Befunde aus, außer der `.pex`-Karte, der Karte Display und den Karten zu
Nexus-Anforderungen, die nur das Fenster berechnet.

## Wenn etwas falsch aussieht

Die allgemeine Prüfliste steht in [troubleshooting.md](troubleshooting.md); die
Fallout-spezifische erste Frage ist immer *welche ausführbare Datei tatsächlich
gestartet ist*. Wenn das Fenster das Spiel startet, schreibt es den
vollständigen Startbefehl in das Laufprotokoll der Instanz, also:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Nennt es `f4se_loader.exe`, hat der Tausch stattgefunden. Nennt es
`Fallout4.exe`, ist F4SE nicht dort installiert, wo Eidos es finden kann - es
gehört neben die ausführbare Datei des Spiels, was bei einem verwalteten Aufbau
das `Root/`-Verzeichnis einer Mod bedeutet (oder den Spielordner selbst, von Hand
installiert). Nennt es `Fallout4Launcher.exe`, hat Eidos weder den Loader noch
`Fallout4.exe` in der Root-Sicht gefunden.

Startet die richtige ausführbare Datei und das Spiel stürzt trotzdem ab, grenzen
Test run und Find the mod behind a problem es auf eine Mod ein, ohne die
Spielstände des Profils anzutasten ([usage.md](usage.md#testläufe)).
