<!-- eidos-i18n: source=docs/guide/extensions.md sha=5f31f1cbc3dcbfb9655f3570f01e5ada1377268d -->

# Erweiterungen

Eine Erweiterung fügt Eidos einen Eintrag hinzu, ohne Teil von Eidos zu sein. Sie
ist ein TOML-Manifest, das ein Programm benennt, plus - höchstens - dieses
Programm.

Manifeste liegen in `~/.config/Colony/Eidos/addons/`, eine `.toml` je
Erweiterung. Öffnen Sie den Ordner über **View -> Extensions -> Open folder** und
drücken Sie **Reload** - kein Neustart.

## Warum nichts in Eidos geladen wird

Mod Organizer 2 lädt Plugins als gemeinsame Bibliotheken und beherbergt die in
Python geschriebenen über Qt. Keines von beidem lässt sich übertragen. Rust hat
kein stabiles ABI, also ist eine gemeinsame Bibliothek, die mit einem anderen
Compiler gebaut wurde - oder mit einem anderen Optimierungsflag oder einem anderen
Funktionsumfang einer gemeinsamen Abhängigkeit - undefiniertes Verhalten und keine
Versionsabweichung. Und die Widgets von Eidos sind zur Übersetzungszeit generisch,
sodass eine Bibliothek nicht einmal eines zum Zurückgeben bauen könnte, selbst
wenn das ABI stabil wäre.

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

Es erscheint unter **View -> Extensions** mit einem Run-Knopf und startet
abgekoppelt - Eidos wartet nicht darauf.

## Eine Prüfung

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Sie läuft bei jeder Aktualisierung und gibt einen Befund je Zeile aus:

```
level<TAB>title<TAB>detail
```

wobei `level` `problem`, `advice` oder `ok` ist. Das Detail ist optional. Alles,
was nicht mit einer bekannten Stufe beginnt, wird ignoriert, sodass
Fortschrittsausgaben und versprengte Warnungen keine Zeile erzeugen können, die
wie eine von Eidos' eigenen Prüfungen aussieht. Befunde erscheinen im Reiter
**Health**, dem Namen der Erweiterung vorangestellt.

Eine Prüfung bekommt drei Sekunden. Eine, die überzieht, wird gestoppt und als
Problem gegen sich selbst gemeldet - sie läuft bei derselben Aktualisierung, die
jedem Klick folgt, eine hängende würde also das Fenster einfrieren.

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
| `{install}`     | das Installationsverzeichnis des Spiels      |
| `{data}`        | das `Data`-Verzeichnis des Spiels            |

Ein unbekannter Platzhalter bleibt genau so stehen, wie er geschrieben wurde,
statt geleert zu werden, damit ein Fehler sichtbar scheitert, anstatt
`--out {typo}` in `--out --next-flag` zu verwandeln. Ein Werkzeug zu starten,
dessen Platzhalter sich nicht alle auflösen lassen, wird verweigert, und Eidos
sagt, welche fehlen.

## Was eine Erweiterung nicht kann

Eine Erweiterung kann Eidos nicht zurückrufen, sich nicht aus einem Archiv registrieren und keine eigenen Widgets zeichnen. Strukturierte Installer mit Protokoll 1 können geprüfte Dateipläne und Auswahldialoge zurückgeben; Prüfung, Vorbereitung im temporären Verzeichnis und Veröffentlichung übernimmt Eidos. FOMOD und BAIN bleiben native Installer. Vorschau- und Spielstandinfo-Erweiterungen verwenden denselben Host mit begrenzten Antworten. Die [Protokollspezifikation mit ausführbaren Beispielen](../../../../examples/extensions/README.md) erläutert die Zuordnungspriorität, gespeicherte CLI-Antworten, Grenzen und Identitätsprüfungen beim Wiederholen.
