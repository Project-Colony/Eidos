<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 durch Eidos

Fallout 4 braucht keine besondere Startoption, keine umbenannte ausführbare Datei
und kein Wrapper-Skript. Das ist es wert, klar gesagt zu werden, denn jede andere
Linux-Anleitung für F4SE behauptet das Gegenteil - und ihr Rat zerbricht beim
nächsten Steam-Update.

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
Spiel-Update stillschweigend wiederherstellt - danach spielen Sie ohne F4SE, und
nichts sagt es Ihnen.

Eidos nimmt den Tausch selbst vor, aus dem Spieldeskriptor: es ersetzt den Launcher
durch `f4se_loader.exe`, wenn einer installiert ist, fällt auf `Fallout4.exe`
zurück, wenn keiner da ist, und **sagt Ihnen**, wenn es zurückfallen musste. Ein
Spiel, das mit lauter toten F4SE-Mods startet, ist schlimmer als eines, das gar
nicht startet.

Es gibt einen zweiten Grund, den Launcher nie auszuführen: er durchsucht `Data`
erneut und schreibt `plugins.txt` neu, wodurch die gerade ausgerollte
Ladereihenfolge zunichtegemacht wird. Eidos führt ihn nie aus.

## Was Eidos für Sie erledigt

| | |
|---|---|
| Archiv-Invalidierung | `Fallout4Custom.ini` bekommt `[Archive]` `bInvalidateOlderFiles=1` und ein leeres `sResourceDataDirsFinal=` - die beiden Schlüssel, die lose Dateien außerhalb von `Data` überhaupt sichtbar machen. Ins Profil geschrieben, nicht in den Spielordner. |
| Ladereihenfolge | `plugins.txt` im Sternchen-Format, das Fallout 4 verwendet (`*` markiert aktiv), mit beachtetem `Fallout4.ccc` für die impliziten Creation-Club-Plugins |
| LOOT | Das Sortieren funktioniert wie bei Skyrim - `eidos sort <instance>` holt die `fallout4`-Masterlist |
| Spielstände | `.fos`-Spielstände und ihre `.f4se`-Cosaves werden aufgelistet, kopiert und je Profil gehalten; die Detailansicht liest die Plugin-Tabelle des Spielstands, sodass ein Spielstand, der ein von Ihnen deaktiviertes Plugin braucht, das sagt, bevor Sie ihn laden |
| Root-Mods | Alles, was eine Mod neben der ausführbaren Datei mitbringt (F4SE selbst, ENB, eine `dxvk.conf`), landet dort über denselben `Root/`-Mechanismus wie bei Skyrim |

## Die Versionsfrage

Fallout 4 ist nicht mehr das eingefrorene Spiel von 2019 bis 2024. Stand August 2026
gibt es drei lebende Zweige, und eine für einen davon gebaute Mod-DLL lädt auf einem
anderen nicht:

| Zweig | Version | F4SE |
|---|---|---|
| Klassisch ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Zwei Folgerungen, die man vor dem Bauen einer Modliste kennen sollte:

- **Prüfen Sie, was Sie tatsächlich haben.** Ordner `Creations/` und `Mods/` im
  Spielverzeichnis bedeuten, dass Sie auf der 1.11.x-Linie sind. Die Detailansicht
  eines Spielstands in Eidos zeigt außerdem den Build, der ihn geschrieben hat -
  Fallout schreibt das in den Spielstand, und Eidos zeigt es als "Game build".
- **Ein frischer Patch ist kein guter Tag zum Anfangen.** F4SE erscheint meist ein
  bis zwei Tage nach einem Bethesda-Update, aber *Address Library for F4SE Plugins*
  - über das die meisten DLL-Mods ihre Offsets auflösen - folgt seinem eigenen
  Zeitplan. Dazwischen liegt die DLL-Hälfte des Ökosystems am Boden. Mods ohne DLL
  (Texturen, Meshes, Plugins) sind nicht betroffen.

Sobald Ihr Aufbau läuft, schalten Sie Steams automatische Updates für Fallout 4 ab
(Eigenschaften → Updates → "Dieses Spiel nur beim Start aktualisieren"), sonst
zerlegt der nächste Patch jede installierte DLL.

## Hardware-Hinweis: Waffentrümmer stürzen auf NVIDIA ab

Der Waffentrümmer-Effekt von Fallout 4 läuft auf NVIDIA FleX, einem
PhysX-Ableger, den NVIDIA nach der Pascal-Generation nicht mehr unterstützt hat. Auf
jeder Turing-Karte oder neuer - GTX 16, RTX 20 bis RTX 50 - stürzt das Spiel ab. Das
ist ein Spielfehler und hat nichts mit Linux, Proton oder Eidos zu tun.

Zwei Abhilfen, beide wirken: "Weapon Debris" in den Spieleinstellungen ausschalten,
oder *Weapon Debris Crash Fix* (Nexus 48078) installieren, das die Kollision der
Fragmente abschaltet statt den Effekt.

## Wenn etwas falsch aussieht

Die allgemeine Prüfliste steht in
[troubleshooting.de.md](troubleshooting.md); die Fallout-spezifische erste Frage
ist immer *welche ausführbare Datei tatsächlich gestartet ist*. Eidos schreibt den
vollständigen Startbefehl in das Laufprotokoll der Instanz, also:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Nennt es `f4se_loader.exe`, hat der Tausch stattgefunden. Nennt es
`Fallout4Launcher.exe`, ist F4SE nicht dort installiert, wo Eidos es finden kann -
es gehört neben die ausführbare Datei des Spiels, was bei einem verwalteten Aufbau
das `Root/`-Verzeichnis einer Mod bedeutet (oder den Spielordner selbst, von Hand
installiert).
