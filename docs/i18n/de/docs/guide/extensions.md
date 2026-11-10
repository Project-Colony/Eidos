<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Erweiterungen

Eine Erweiterung fügt Eidos einen Eintrag hinzu, ohne Teil von Eidos zu sein. Sie
ist ein TOML-Manifest, das ein Programm benennt, plus - höchstens - dieses
Programm.

Manifeste liegen in `~/.config/Colony/Eidos/addons/`, eine `.toml` je
Erweiterung. Öffnen Sie den Ordner über View -> Extensions -> Open folder und
drücken Sie Reload - kein Neustart.

`id`, `kind` und `exec` sind Pflicht; `name` ist ohne Angabe die id. `exec`
benennt das Programm. Ein Name ohne `/` wird im `PATH` gesucht. Fehlt das
Programm, überspringt Eidos die Erweiterung, und ihre Zeile unter View ->
Extensions sagt warum (`<exec> is not on PATH` oder `<exec> is not there`).
`games` beschränkt die Erweiterung auf diese Spiel-IDs; ohne die Angabe gilt sie
für jedes Spiel, und bei einem anderen Spiel steht in ihrer Zeile „not for this
game“.

Ein Manifest, das Eidos ablehnt, wird zuerst aufgelistet, mit dem Grund: eine
Datei, die sich nicht lesen lässt, kein gültiges TOML oder ein fehlendes
Pflichtfeld, eine leere `id` oder eine mit Leerzeichen, ein leeres `exec`, eine
`id`, die ein früheres Manifest schon belegt hat (Dateien werden in
Namensreihenfolge gelesen), ein `kind`, den Eidos nicht kennt, oder ein
Installer-, Vorschau- oder Spielstandinfo-Kind ohne `protocol = 1` und den
Dateinamen `extensions`.

## Warum nichts in Eidos geladen wird

Mod Organizer 2 lädt Plugins als gemeinsame Bibliotheken und beherbergt die in
Python geschriebenen über Qt. Keines von beidem lässt sich übertragen. Rust hat
kein stabiles ABI, also ist eine gemeinsame Bibliothek, die mit einem anderen
Compiler gebaut wurde - oder mit einem anderen Optimierungsflag oder einem anderen
Funktionsumfang einer gemeinsamen Abhängigkeit - undefiniertes Verhalten, und
nichts meldet es als Versionsabweichung. Und die Widgets von Eidos sind zur
Übersetzungszeit generisch, sodass eine Bibliothek nicht einmal eines zum
Zurückgeben bauen könnte, selbst wenn das ABI stabil wäre.

Eine Erweiterung ist ein Programm, das Eidos mit den Dateisystemrechten Ihres Kontos ausführt. Installieren Sie nur Hilfsprogramme, denen Sie vertrauen. Prozessisolierung und geprüfte Antworten bieten keine Sandbox des Betriebssystems.

## Ein Werkzeug

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # weglassen für jedes Spiel
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Es erscheint unter View -> Extensions mit einem Run-Knopf und startet
abgekoppelt - Eidos wartet nicht darauf, und die Statuszeile nennt seine pid.
Seine Ausgabe wird verworfen. Ohne `workdir` startet es in dem Ordner, aus dem
Eidos selbst gestartet wurde.

## Eine Prüfung

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Sie läuft bei jeder Aktualisierung, im Instanzstamm, sofern `workdir` nichts
anderes sagt, und gibt einen Befund je Zeile aus:

```
level<TAB>title<TAB>detail
```

wobei `level` `problem`, `advice` oder `ok` ist. Das Detail ist optional; eine
Zeile ohne Titel wird verworfen. Alles, was nicht mit einer bekannten Stufe
beginnt, wird ignoriert, sodass Fortschrittsausgaben und versprengte Warnungen
keine Zeile erzeugen können, die wie eine von Eidos' eigenen Prüfungen aussieht.
Befunde erscheinen im Tab Diagnostics als `<name> - <title>`, nach den
Präfix- und Nexus-Karten und vor den übrigen Health-Karten.

Eine Prüfung bekommt drei Sekunden. Eine, die überzieht, wird gestoppt und als
Problem gegen sich selbst gemeldet - sie läuft bei derselben Aktualisierung, die
jedem Klick folgt, eine hängende würde also das Fenster einfrieren. Die Uhr läuft,
bis die Prüfung beendet ist und ihre Ausgabe-Pipes geschlossen sind; ein
Hintergrund-Kindprozess, der sie noch hält, zählt also gegen die drei Sekunden.
Eine Prüfung, die mit einem Fehler endet und nichts ausgibt, wird auf dieselbe
Weise gemeldet, mit der ersten Zeile ihrer Fehlerausgabe, ebenso eine, die auf
einem der beiden Ströme mehr als 1 MiB ausgibt. In beiden Fällen führt Eidos sie
erst wieder aus, wenn Sie Reload drücken, und ihre Zeile sagt „It will not be
retried until you press Reload.“ Eine Prüfung liest nichts von ihrer
Standardeingabe.

## Platzhalter

Sowohl `args` als auch `workdir` setzen diese ein:

| Platzhalter     | Was es ist                                   |
| --------------- | -------------------------------------------- |
| `{instance}`    | die Wurzel der Instanz                       |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | der Name des aktiven Profils                 |
| `{profile_dir}` | das Verzeichnis des aktiven Profils          |
| `{game}`        | die Spiel-ID, z. B. `skyrimse`               |
| `{game_name}`   | der Anzeigename des Spiels                   |
| `{install}`     | der Spielordner, den die Instanz startet     |
| `{data}`        | das `Data`-Verzeichnis dieses Ordners        |

`{install}` ist die Store-Installation oder die Spielkopie, wenn die Instanz eine
nutzt (ein eingefrorenes `Stock Game` oder der eigene Spielordner einer Liste;
siehe [usage.md](usage.md#spieldateien-die-store-installation-oder-eine-kopie)).
`{data}` folgt ihm.

Eine Prüfung bekommt außerdem `{addon_dir}`, den Ordner, der ihr Manifest
enthält, und die Erweiterungen mit Protokoll 1 weiter unten ebenso. Ein Werkzeug
nicht.

Ein unbekannter Platzhalter wird nie geleert, was `--out {typo}` in
`--out --next-flag` verwandeln würde. Eidos weigert sich, ein Werkzeug oder eine
Prüfung auszuführen, deren Platzhalter sich nicht alle auflösen lassen, und sagt,
welche fehlen: in der Statuszeile bei einem Werkzeug, in der Zeile der Prüfung im
Tab Diagnostics bei einer Prüfung.

## Was eine Erweiterung nicht kann

Eine Erweiterung kann Eidos nicht zurückrufen, sich nicht aus einem Archiv registrieren und keine eigenen Widgets zeichnen. Strukturierte Installer mit Protokoll 1 können geprüfte Dateipläne und Auswahldialoge zurückgeben; Prüfung, Vorbereitung im temporären Verzeichnis und Veröffentlichung übernimmt Eidos. FOMOD und BAIN bleiben native Installer. Vorschau- und Spielstandinfo-Erweiterungen verwenden denselben Host mit begrenzten Antworten. Die [Protokollspezifikation mit ausführbaren Beispielen](../../../../guide/examples/extensions/README.md) erläutert die Zuordnungspriorität, gespeicherte CLI-Antworten, Grenzen und Identitätsprüfungen beim Wiederholen.
