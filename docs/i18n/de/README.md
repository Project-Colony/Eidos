<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Ein nativer Linux-Mod-Manager, der Ihren Spielordner nie anfasst.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

Eidos gibt Bethesda-Spielen unter Linux das, was Mod Organizer 2 ihnen unter
Windows gibt: eine zusammengeführte Sicht auf Ihre Mods, die bei jedem Start neu
aufgebaut wird. Statt Windows-API-Hooking nutzt es Linux-Mount-Namensräume und
FUSE, und der Manager läuft ohne Wine. Nichts wird in den Spielordner kopiert,
also gibt es hinterher nichts aufzuräumen. Eidos hat außerdem Definitionen für
Stellar Blade, Stardew Valley, 7 Days to Die und Crimson Desert.

Steam-Kopien starten Eidos über die Startoption des Spiels. GOG- und
Epic-Kopien in Heroic oder Legendary starten es als Wrapper. Das Spiel startet
immer über den Store, dem die Kopie gehört; Eidos startet es nie selbst mit
Proton.

```
Steam ──> eidos-gui %command% ──> [ privater Namensraum ]
                                  │  mods ⊕ Spiel  ──> was das Spiel sieht
                                  └─ stirbt mit dem Spiel; die Installation bleibt unberührt
```

> **Status:** Skyrim SE wird täglich mit Eidos gespielt, mit SKSE,
> Script-Extender-Preloadern, Creation Club, mit LOOT sortierten
> Ladereihenfolgen und Spielständen pro Profil. Eine Spielfamilie ist bisher im
> echten Spiel erprobt. Zehn weitere Bethesda-Spiele und vier andere sind
> verdrahtet und warten auf Tester.

## Warum Eidos

Die zusammengeführte Sicht wird in einem privaten Mount-Namensraum eingehängt,
den nur der Prozessbaum des Spiels sehen kann. Andere Programme, auch ein
zweites Spiel, sehen ihn nie und brauchen dafür keine Berechtigung. Wenn das
Spiel endet oder beendet wird oder der Rechner den Strom verliert, verschwindet
der Namensraum mit seinen Prozessen, und die Installation ist wie vorher.

Jedes Profil besitzt seine Mod-Liste, Plugin-Reihenfolge, INIs und Spielstände.
Beim Start werden die Plugin-Dateien und das Spielstandverzeichnis per Bind-Mount
über die eigenen Pfade des Spiels gelegt, sodass auch das, was das Spiel dort
schreibt, im Profil landet. Ein Profilwechsel wechselt alles.

Eidos braucht keinen setuid-Helfer, keinen Systemdienst, kein `sudo setcap` und
keine Änderung an `/etc/fuse.conf`. Der FUSE-Daemon läuft für einen Start als Ihr
Benutzer und endet mit dem Spiel. Sie installieren ein Programm und setzen eine
Steam-Startoption.

Vor jedem Lauf legt Eidos einen Schnappschuss der Plugin-Liste an. Beschädigt
ein Absturz die Liste, vergleicht Diagnostics sie mit diesem Schnappschuss, und
Restore the pre-session order stellt die alte Reihenfolge wieder her. Eine
Übernahme, die Ihre Ladereihenfolge löschen würde, wird verweigert, mit Angabe
des Grundes.

Test run startet das Spiel mit den Mods des Profils, ohne Mods oder nur mit den
ausgewählten Mods, die eingeschaltet sind, und deren Mastern. Die Spielstände, INI-Änderungen,
Plugin-Zustände, die Overwrite-Ausgabe und die übrigen My-Games-Dateien dieses
Laufs, Logs eingeschlossen, gehen nach `<instance>/test-run/`. Wenn das Spiel
endet, listet Eidos diese fünf Gruppen auf, und Sie behalten oder verwerfen jede
einzeln. Andere Schreibzugriffe, etwa auf die Wine-Registry, gehen dorthin, wo
sie immer hingehen. Find the mod behind a problem in Diagnostics führt Testläufe
mit jeweils der Hälfte der verdächtigen Mods plus den Mastern aus, die sie
brauchen. Nach jedem Lauf beantworten Sie, ob das Problem aufgetreten ist, und
Eidos grenzt die Verdächtigen ein, bis es die Mod nennt, oder die Mods, die
gegenseitig ihre Master brauchen; 200 Verdächtige brauchen etwa 8 Läufe. Eingeschaltete Mods,
die Sie vor dem Start in der Liste auswählen, bleiben in jedem Schritt
eingeschaltet. Einzelheiten in [Testläufe](docs/guide/usage.md#testläufe) und
[Die Mod hinter einem Problem finden](docs/guide/usage.md#die-mod-hinter-einem-problem-finden).

### Wie es sich vergleicht

| | Eidos | MO2 über Wine | Fluorine-Manager | Limo / Link-Deployer |
|---|---|---|---|---|
| Manager läuft nativ | ja | nein, eine Windows-App in Wine | ja (Qt-Portierung) | ja |
| Spielverzeichnis unangetastet | ja, immer | ja | ja | nein, Links werden hineingeschrieben |
| Mount sichtbar für | nur das Spiel | nur das Spiel | das ganze System | entfällt |
| Aufräumen nach Absturz nötig | keines | keines | Wiederherstellung toter Mounts | manuelles Zurücknehmen |
| Root-Mods (ENB, Preloader) | ja, nativ | Plugin nötig | Plugin nötig | teilweise |
| Benötigte Privilegien | keine | keine | `/etc/fuse.conf` ändern | keine |

### Wie schnell es ist

| | vorher | jetzt |
|---|---|---|
| einen Spielstand laden | ~20 Sekunden | 6-7 Sekunden |
| Verzeichnislesevorgänge in einer Sitzung | 5,6 Millionen | 335 Tausend |

Zellenwechsel sind sofort. Eine Datei zu finden befragte früher nacheinander
alle fünfzig Mods, und einen Ordner aufzulisten tat es fünfzigmal. Die
schreibgeschützten Ebenen werden jetzt beim Mount einmal indiziert, sodass das
Finden einer Datei ein einziger Hash-Zugriff ist und auch die zusammengeführte
Auflistung eines Ordners beim Mount einmal berechnet wird. Die Zahlen stammen
von einer echten Instanz, normal gespielt, nicht von einem Benchmark:
[internals/performance.md](../../internals/performance.md).

### Vorarbeiten und Dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) und
  [usvfs](https://github.com/ModOrganizer2/usvfs): die Semantik, die Eidos
  nachbildet, und die Codebasis, an der seine Parität studiert wurde
- [LOOT](https://loot.github.io/): die Sortier-Engine, über libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) und die anderen Linux-Manager, die
  zeigen, dass es eine Gemeinschaft gibt, die das gelöst haben will

## Was es tut

### Mods

Eidos installiert einfache Archive, FOMOD-Assistenten, BAIN-Pakete von Wrye
Bash, OMOD-Container mit unterstützten OBMM-Skripten und alles andere über eine
manuelle Auswahl. Root-Mods (Script-Extender-Preloader, ENB, Engine Fixes)
funktionieren nativ, ohne Root-Builder-Plugin und ohne dass etwas in Ihre
Installation kopiert wird. Sie können einzelne Dateien ausblenden, Mods mit
Separatoren gruppieren, eine Auswahl an ein Ziel verschieben, Notizen und
Kategorien pro Mod führen und ein MO2-Profil importieren.

Die Liste arbeitet wie die von MO2: acht optionale Spalten und eine Sortierung
nach jeder davon, Gruppierung nach Kategorie oder nach Quelle, Gesten per
Doppelklick, Tippen zum Springen, Backups pro Mod, die nichts tun, bis Sie sie
wiederherstellen, und Flaggen für eine Mod, deren Aufbau dieses Spiel nicht
laden wird oder die für ein anderes Spiel heruntergeladen wurde. Filter wählen
Mods nach Nexus-Zustand (bewertet, verfolgt, Update verfügbar), Notizen,
versteckten Dateien und Inhaltstyp (Plugins, Archive, Texturen, Meshes, Skripte,
Sound und mehr). Der Dateibaum legt Ordner an, benennt um, löscht und öffnet
Dateien und zeigt Bilder und Texte in der Vorschau, ohne etwas zu starten.

Mit Strg+Z und Strg+Umschalt+Z machen Sie Änderungen an der Mod-Liste und der
Ladereihenfolge rückgängig und stellen sie wieder her. Wer eine Mod
deaktiviert oder entfernt, die einer der drei neuesten Spielstände des Profils
braucht, wird vorher gefragt, und die Plugins werden genannt (Spielstände von
Skyrim und Fallout 4). Eine FOMOD-Mod neu zu installieren oder zu aktualisieren
öffnet den Assistenten mit den Antworten der letzten Installation. Die
Informationen einer Mod listen die Mods auf, die sie braucht, und die Mods, die
sie brauchen; sie werden aus den Mastern der aktivierten Plugins gelesen.

### Plugins

Die Ladereihenfolge hat eine eingebaute LOOT-Sortierung, Mod-Indizes so, wie das
Spiel sie berechnet, Warnungen vor fehlenden Mastern, und Ihre DLC- und
Creation-Club-Inhalte erscheinen als nicht verwaltete Zeilen. Die Kopfzeile
zählt die belegten Slots in jedem Indexbereich (full, medium, light). Fährt man
über ein Plugin, zeigt es Autor, Beschreibung und Header-Version. Bei den
Skyrim-Engines ist ein Plugin markiert, dessen gleichnamige INI das Spiel lädt.

Plugins, die LOOT als dreckig markiert, bereinigt xEdits QuickAutoClean in eine
Mod namens `Cleaned plugins`, nie an Ort und Stelle. Flag as light, into a new
mod schreibt eine Light-Kopie eines Plugins und verweigert das, wenn einer der
neuesten Spielstände dieses Plugin als volles Plugin lädt (Spielstände von
Skyrim und Fallout 4).

### Instanzen

Eine Instanz ist global, unter `~/.local/share/Colony/Eidos/instances/`
verwaltet, oder portabel: ein eigenständiger Ordner, wo Sie wollen (eine zweite
Platte, eine Spielepartition), verschiebbar und isoliert wie die von MO2.
Portable Instanzen werden über Sitzungen hinweg gemerkt. Die GUI, der Steam-Start
und jeder CLI-Befehl folgen der zuletzt verwendeten, und jeder Befehl nimmt den
Ordner überall dort, wo er eine Spiel-ID nimmt. Einzelheiten in
[usage.md](docs/guide/usage.md#instanzen-global-und-portabel).

Eine Instanz kann das Spiel auch aus einer Kopie des Spielordners innerhalb der
Instanz starten. Wenn Eidos zum ersten Mal eine Wabbajack-Liste öffnet, nutzt es
die eigene Kopie der Liste: den Ordner, den ihre `ModOrganizer.ini` nennt,
sonst einen Ordner `Stock Game` oder `Game Root`. Freeze this version auf der
Karte Game files in Diagnostics kopiert die Store-Installation nach
`<instance>/Stock Game`, sodass ein Store-Update die Dateien, mit denen das Spiel
läuft, nicht mehr ändert. Liegen beide Ordner auf einem Dateisystem, das Blöcke
teilen kann, etwa btrfs oder XFS, teilt die Kopie ihre Blöcke mit der
Store-Installation (Reflink). Auf ext4 oder über zwei Dateisysteme hinweg ist es
eine vollständige Kopie, und Größe und freier Platz werden vorher angezeigt. Der
Store startet das Spiel weiterhin. Return to the store version und Delete the
copy machen es rückgängig, und
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
tut dasselbe im Terminal. Morrowind und Oblivion mit `bUseMyGamesDirectory=0`
können nicht aus einer Kopie laufen. Einzelheiten in
[usage.md](docs/guide/usage.md#spieldateien-die-store-installation-oder-eine-kopie).

### Profile

Jedes Profil hat seine eigene Mod-Reihenfolge, seinen Plugin-Zustand, seine INIs
und seine Spielstände. Spielstände werden analysiert und mit Ihren aktuellen
Plugins abgeglichen, mit einem Knopf, der aktiviert, was ein Spielstand
braucht, und nach jeder Sitzung für Steam Cloud ins Präfix zurückkopiert.
Spielstände, die Steam Cloud von einem anderen Gerät mitgebracht hat oder die ein
Start ohne Eidos im Präfix hinterlassen hat, stehen im Tab Saves. Import kopiert
sie ins Profil, und ein Spielstand, dessen Namen das Profil schon verwendet,
kommt unter einem neuen Namen mit `orphan-` herein, statt den vorhandenen zu
ersetzen.

Ein Profil kann Startvariablen und Wrapper tragen (`env=NAME=value` und
`wrapper=command`, je eine pro Zeile, im INI-Editor) für die Läufe seines Spiels
und seiner Tools. Für Skyrim SE, Enderal SE und Fallout 4 setzt eine Karte
Display in Diagnostics Fenstergröße und -modus in den INI-Kopien des Profils.
Backups hält Wiederherstellungspunkte der INIs des Profils, und Eidos legt vor
jedem Lauf von BethINI Pie einen an. Die Desktop-Verknüpfung eines Profils
startet das Spiel über seinen Store. `eidos sessions` listet die aufgezeichneten
Läufe und vergleicht einen mit dem aktuellen Zustand.

### Nexus

Verbinden Sie ein Konto, um „Mod Manager Download“-Links zu empfangen, Updates zu
prüfen und Autoren mit ihren Profilen zu sehen. Jede Mod wird auf der
Nexus-Domain geprüft, bewertet und verlinkt, aus der ihre Datei kam; eine
Skyrim-SE-Mod in einer Enderal-SE-Instanz bleibt also auf der Skyrim-SE-Seite.
Ein Update folgt der Nachfolgerkette, die der Autor auf Nexus gesetzt hat,
ersetzt den Ordner der Mod durch die neue Datei (ein Backup der alten wird
angeboten), behält Priorität und Aktivierungszustand und spielt die bei der
Installation gespeicherten FOMOD-Antworten erneut ab.
`eidos nexus update <instance> --apply` tut dasselbe im Terminal mit einem
Premium-Konto und gibt bei einem kostenlosen Konto die Dateiseiten aus. Track
und Endorse stehen im Mod-Menü, und jede Update-Prüfung gleicht sie mit Ihrem
Nexus-Konto ab. Die Informationen einer Mod haben einen Tab Nexus page mit
Beschreibung und Changelog als reinem Text, und ihr Tab General endet mit dem,
was das letzte Update geändert hat: die hinzugefügten, entfernten und in der
Größe geänderten Dateien sowie die hinzugekommenen oder entfernten Master.
Install downloads when they finish (Preferences, Nexus, standardmäßig aus)
installiert einen Download, sobald er fertig ist, solange das Fenster offen ist.

Kollektionen werden anhand ihrer Rezepte installiert, mit fortsetzbarem
Fortschritt und ausdrücklichen Berichten über nicht unterstützte Schritte. Der
Abgleich prüft die genauen Quelldateien; unvollständige ältere Metadaten bleiben
ungeprüft. Der Kollektionsbereich durchsucht die für ein Spiel veröffentlichten
Kollektionen, zeigt an, wenn eine installierte Kollektion eine neuere Revision
hat, und führt ein kostenloses Konto Seite für Seite durch die fehlenden
Dateien. Bricht ein Ziehen die Reihenfolgeregeln einer Kollektion, sagt
Diagnostics das, und Restore collection order bringt ihre Mods zurück. Der
Downloads-Tab filtert, sortiert, verbirgt und entfernt Archive. Der Offline-Modus
(Preferences, Nexus) unterbindet den Nexus-Zugriff und die tägliche
Release-Prüfung.

### Werkzeuge

xEdit, BodySlide, DynDOLOD und andere Modding-Werkzeuge laufen durch die
zusammengeführte Sicht im Wine-Präfix des Spiels (Steams Proton-Präfix oder das,
das Heroic eingerichtet hat). Sie sehen Ihre Mods, ihre Ausgabe landet in
Overwrite, und ein Klick macht daraus eine echte Mod. Für Skyrim SE findet Eidos
SSEEdit, SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide,
DynDOLOD, TexGen, xLODGen, Synthesis und BethINI Pie im Spielordner, in einer Mod
oder in dem Werkzeugordner, den Sie neben Ihren Spielen halten, jeweils mit den
Runtimes, die es braucht. Die anderen Spiele haben ihre eigene Liste. Pandora,
Nemesis, FNIS, BodySlide und Synthesis schreiben ihre Ausgabe in eine benannte
Mod (`Nemesis Output` und so weiter).

Das Release-Bundle enthält die Installer für Microsofts Visual C++ 2015-2022 und
die .NET 8 Desktop Runtime sowie die .NET-10-Runtime. Eidos installiert
`vcrun2022` und .NET 8 mit diesen Installern, ohne winetricks, und ohne das
Bundle lädt es dieselben Dateien von Microsoft herunter, jede gegen die in
Eidos eingebaute SHA-256 geprüft. Synthesis bekommt das .NET-10-SDK, einen
Download von 300 MB, den das Bundle nicht mitliefert. Die übrigen Voraussetzungen
(.NET Framework 4.8, XACT und ältere .NET-Versionen) laufen weiter über
winetricks. Es wird nichts heruntergeladen, bevor Sie die Voraussetzungen eines
Tools installieren lassen.

Heften Sie die Tools an, die Sie nutzen, blenden Sie die aus, die Sie nicht
nutzen, geben Sie einem Tool seine eigene Steam-AppID, wenn es eine eigene
Steam-App ist, und schreiben Sie eine `.desktop`-Verknüpfung, die es durch die
zusammengeführte Sicht startet, ohne Eidos zu öffnen. `eidos tool <game> import-mo2`
listet die Programme in der `ModOrganizer.ini` des Ordners einer Liste auf, und
`--write` fügt sie hinzu. Eine exe im Tab Data lässt sich einmal ausführen oder
als Tool hinzufügen, und `eidos tool <instance> run --exe <path>` führt eine im
Terminal aus. Mit Ask before playing fragt Run zuerst nach, wenn die Ausgabe
eines Generators veraltet ist.

Stop beendet ein laufendes Spiel oder Tool. Das Menü File startet winecfg,
regedit, den Wine-Dateiexplorer und den Windows-Deinstallierer im Präfix des
Spiels. Rebuild prefix in Diagnostics benennt das Präfix um, damit Steam ein
neues anlegt, und löscht nichts. `eidos shell <instance>` öffnet eine Shell in
der zusammengeführten Sicht oder führt den Befehl nach `--` aus, und was sie
schreibt, landet in Overwrite.

Ist das Fenster im Hintergrund, kann Eidos eine Desktop-Benachrichtigung senden,
wenn ein Tool-Lauf, eine Kollektionsinstallation, ein Packen oder Entpacken oder
der letzte laufende Download endet und wenn eine LOOT-Sortierung fehlschlägt.
Notify the desktop when a long job ends in Preferences schaltet das ein; es ist
standardmäßig aus.

### Diagnose

Der Tab Diagnostics listet fehlende Master, verwaiste Archive, Abweichungen in
der Mod-Liste und beschädigte Plugin-Sätze und nach einem Lauf, was das eigene
Log des Script Extenders als geladen ausweist. Er meldet außerdem:

- einen Script Extender, der fehlt oder nicht zur Spielversion passt
- ein Spiel, das sich seit dem letzten sauberen Start geändert hat
- eine Steam-Startoption, die Eidos nicht startet oder hinter der `%command%` fehlt
- ein Proton, das älter ist als das, das das Präfix zuletzt eingerichtet hat,
  oder eines aus dem Flatpak-Steam
- `PROTON_ENABLE_NVAPI`, das für DLSS fehlt, und exklusiven Vollbildmodus, der
  die Frame Generation von Community Shaders stoppt
- eine `enblocal.ini` ohne `LinuxVersion=true`
- das Absturzlog der letzten Sitzung und die Mods, die es nennt
- Nexus-Anforderungen, die fehlen oder deaktiviert sind
- einen `nxm://`-Handler, der ein anderes Programm oder eine fehlende Datei ist
- `sTestFile`-Zeilen, die das Spiel `plugins.txt` ignorieren lassen
- aktivierte `.esp`-Plugins in Starfield sowie ungültige oder ungepaarte
  Blueprint-Plugins
- einen Patch, dessen lose Dateien gegen die Mod verlieren, die er patcht
- eine Mod, deren Kopien der Papyrus-Skripte des Script Extenders die des
  Extenders selbst überstimmen
- ein niedriges `vm.max_map_count` oder Limit offener Dateien
- Spieldateien auf einem Windows-Dateisystem
- BodySlide-Datenpfade, die nicht der Data-Ordner des Spiels sind, und Ordner in
  `Overwrite/Root`, die nach Data gehören
- eine `ModOrganizer.ini` einer Liste, die Tools auf einen anderen Spielordner
  zeigen lässt
- eine Spielkopie, die fehlt oder keine Spiel-Exe hat

`eidos check <instance> [--json]` führt die Prüfungen aus, die kein Fenster
brauchen, und endet mit 1, wenn es ein Problem findet. Copy report kopiert einen
Bericht, in dem Ihr Home-Ordner als `~` und Ihr Benutzername als `<user>`
geschrieben sind, und Save tree im Tab Data schreibt den zusammengeführten
Data-Baum mit der Quelle jeder Datei, auf dieselbe Weise geschwärzt.

Siehe [Integritätsprüfungen und generierte Dateien](../../internals/integrity.md)
für den Schutz beim Ersetzen, BSA/BA2-Dateikonflikte, statische SKSE-Diagnosen
und Generierungsnachweise, einschließlich ihrer Grenzen.

### Wo es seine eigenen Dateien ablegt

Eidos nutzt die Anordnung, die jedes Programm der Colony-Familie verwendet:
`~/.config/Colony/Eidos/` für das, was Sie gewählt haben (Einstellungen, Ihre
Nexus-Sitzung, Ihre Instanzliste, die Spiel- und Add-on-Definitionen, die Sie
geschrieben haben), `~/.local/share/Colony/Eidos/` für globale Instanzen,
Tool-Runtimes (heruntergeladen oder von `install.sh` aus dem Bundle kopiert) und
Sitzungs-Logs und `~/.cache/Colony/Eidos/` für LOOT-Masterlisten. Innerhalb einer
Instanz enthält `Stock Game` eine eingefrorene Kopie des Spiels, und `test-run/`
enthält einen Testlauf, der auf seine Durchsicht wartet.

Ein älteres Eidos legte seine Dateien in `~/.config/eidos/`,
`~/.local/share/eidos/` und `~/.local/state/` ab. Der erste Start nach der
Aktualisierung überträgt sie, sagt es im Log und lässt die alten Verzeichnisse
so, wie sie waren. Solange ein Umzug nicht abgeschlossen ist, verwendet Eidos
weiter die alten Dateien. Details in [internals/paths.md](../../internals/paths.md).

### Unterstützte Spiele

Skyrim SE/AE ist im echten Spiel erprobt. Fallout 4 ist ebenfalls durchgängig
verdrahtet (F4SE wird automatisch eingesetzt, Archivinvalidierung,
Ladereihenfolge mit Sternchen, LOOT, `.fos`-Spielstände); siehe
[guide/fallout4.md](docs/guide/fallout4.md). Verdrahtet und auf der Suche nach
Testern: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV,
Starfield, Oblivion und Morrowind. Fallout 3, Fallout NV, Oblivion und Morrowind
ordnen Plugins nach dem Datum der Dateien: Eidos führt pro Profil eine
Reihenfolge und zeigt diese Daten im Mount an, ohne die Dateien auf der Platte zu
ändern.

Vier weitere Spiele sind verdrahtet und suchen ebenfalls Tester: Stellar Blade
(Unreal-`.pak`-Mods in `~mods`), Stardew Valley (SMAPI-Mods, SMAPI getrennt
installiert), 7 Days to Die (`Mods/`-Ordner neben dem Spiel) und Crimson Desert.
Crimson Desert nimmt nur Code-Mods: ASI-Plugins, ReShade und Lader-DLLs in
`bin64`. Seine Daten-Mods, die in PAZ-Gruppen liegen, werden nicht unterstützt.

Ein Spiel ist eine TOML-Datei, und Ihre eigene kann ohne Neubau in
`~/.config/Colony/Eidos/games/` liegen:
[internals/adding-games.md](../../internals/adding-games.md).

## Installation

### Über Colony (empfohlen)

Suchen Sie in [Colony](https://github.com/Project-Colony/Colony) nach **Eidos**
und installieren Sie es. Updates kommen über den Launcher. Colony installiert
eine einzige Datei, `eidos-linux`: das Fenster und zugleich die Kommandozeile
`eidos`, wenn sie über einen Link namens `eidos` aufgerufen wird. Diese Datei
enthält weder das Hilfsprogramm für die NIF-Modellvorschau noch die
Microsoft-Runtimes des Bundles unten. Ohne das Hilfsprogramm sagt die Vorschau
das, und alles andere funktioniert; und Eidos lädt eine Runtime von Microsoft
herunter, wenn Sie die Voraussetzungen eines Tools installieren. Was nach der
Installation zu tun ist und wie Sie Steam darauf zeigen lassen:
[docs/guide/install.md](docs/guide/install.md#über-colony).

### Direkter Download der Binärdateien

Holen Sie sich ein Asset aus dem [neuesten Release](https://github.com/Project-Colony/Eidos/releases/latest):

| Asset | Was es ist |
|---|---|
| `eidos-linux` | das ganze Programm in einer Datei, so wie Colony es installiert |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, das NIF-Vorschau-Hilfsprogramm, die Installer für Visual C++ 2015-2022 und die .NET 8 Desktop Runtime sowie die .NET-10-Runtime, Icons, Lizenzhinweise und ein Installer, der den Desktop-Eintrag und den Nexus-Download-Handler registriert |

Prüfen Sie den Download zuerst, wie die
[Richtlinie zur Codesignierung](#richtlinie-zur-codesignierung) zeigt, dann:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Der Installer legt die Binärdateien ohne sudo nach `~/.local/bin` und kopiert das
`runtimes/` des Bundles nach `~/.local/share/Colony/Eidos/runtimes`. `--system`
und `--bindir DIR` legen die Binärdateien an einen anderen Ort, und `--from DIR`
nimmt sie aus einem anderen Ordner. `--cap` bittet sudo um `CAP_SYS_ADMIN`, das
nur das experimentelle FUSE-Passthrough braucht; Passthrough bleibt aus, und
`--no-cap` ist die Voreinstellung. Beide Assets brauchen x86_64-Linux mit glibc
2.39 oder neuer, FUSE (`fusermount3`) und 7-Zip für Mod-Archive.

Eidos aktualisiert sich nicht selbst. Einmal am Tag fragt das Fenster beim Start
GitHub nach dem neuesten Release, und wenn ein neueres erschienen ist, zeigt es
nach Help in der Menüleiste einen Link zu dessen Release Notes. Preferences
schaltet das ab, und eine Kopie unter `/usr` außerhalb von `/usr/local`, die ein
Paketmanager aktualisiert, fragt nie.

### Aus dem Quelltext bauen

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Benötigt Rust 1.92 oder neuer (die `rust-version`, die die CI prüft; CI und
Releases bauen mit 1.99.1), dazu `just`, CMake 3.20+ und einen C++17-Compiler
für das NIF-Vorschau-Hilfsprogramm. `just build` nutzt das Release-Profil: eine
Codegen-Einheit und Fat LTO. Ein Kaltbau von `eidos` und `eidos-gui` mit 3
Build-Jobs dauerte so 334 Sekunden und mit Thin LTO 243 Sekunden, und Fat LTO
macht `eidos` um 1,2 MB und `eidos-gui` um 3,8 MB kleiner (Messwerte in
`Cargo.toml`). `just profile=dev build` kompiliert schneller. Ein Arch-Paket wird
aus `packaging/PKGBUILD` gebaut. Voraussetzungen und die Befehle ohne `just`:
[docs/guide/install.md](docs/guide/install.md).

### Steam-Startoptionen

Eidos läuft *als* Startbefehl des Spiels, und so mountet es, bevor das Spiel
startet. Die Grundzeile ist alles, was die meisten Konfigurationen brauchen, mit
dem absoluten Pfad Ihrer Installation (Steam liest Ihren `PATH` nicht):

```
~/.local/bin/eidos-gui %command%
```

Aus Colony lautet dieser Pfad `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Drücken Sie dann Spielen: Eidos öffnet sich auf der Instanz dieses Spiels, und
Run startet das Spiel durch die zusammengeführte Sicht.

In einem Fenster, das Sie selbst geöffnet haben, bittet Run Steam, das Spiel zu
starten (`steam -applaunch <appid>`), sodass die Startoption weiter gilt, und Sie
drücken Run in dem Fenster, das Steam öffnet. Heroic bekommt einen
`heroic://launch`-Link, und Legendary führt `legendary launch <app>` aus, sobald
Eidos dort als Wrapper des Spiels gesetzt ist. Fehlt die Einstellung des Stores,
startet Run nichts und sagt, was zu setzen ist:
[guide/usage.md](docs/guide/usage.md#das-spiel-starten). Mit Start the game at
once when a store opens Eidos (Preferences, standardmäßig aus) führt Eidos die
Prüfungen aus Diagnostics aus und startet das Spiel ohne sein Fenster, wenn
nichts Aufmerksamkeit braucht.

Alles Weitere sind Umgebungsvariablen, die davor gestapelt werden, und sie
lassen sich frei kombinieren:

| Sie wollen... | Davor setzen |
|---|---|
| DLSS mit Community Shaders | `PROTON_ENABLE_NVAPI=1`. Ohne sie initialisiert sich DLSS nie und sagt nichts; die vollständige Checkliste ist [guide/graphics.md](docs/guide/graphics.md) |
| einen FPS-Zähler auf dem Bildschirm | `DXVK_HUD=fps` |
| Frame-Interpolation auf Treiberebene, keine Mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, niemals zusammen mit der eigenen Frame Generation von Community Shaders |
| ausführliche Logs für einen Fehlerbericht | `EIDOS_LOG=debug` (Sitzungs-Logs landen in `~/.local/share/Colony/Eidos/logs/`) |
| einen I/O-Bericht pro Sitzung vom Mount | `EIDOS_FUSE_STATS=1` |
| eine andere Anzahl an FUSE-Workern | `EIDOS_FUSE_THREADS=8` (Standard 4; `1` ist das Erste, was man bei der Jagd auf einen Nebenläufigkeitsfehler versucht) |
| diesen Start an eine portable Instanz binden | `EIDOS_INSTANCE=/path/to/folder`. Ohne sie öffnet Eidos die zuletzt verwendete Instanz |
| das Fenster für einen Start, solange Start the game at once an ist | `EIDOS_WINDOW=1` |

Die Zeile, die man für eine moderne gemoddete Konfiguration behält (Community
Shaders, DLSS, Frame Generation), ist der endgültige Befehl, kein Beispiel:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Setzen Sie `DXVK_HUD=fps` davor, während Sie prüfen, ob die Konfiguration
funktioniert, und lassen Sie es weg, sobald sie es tut.

Dieselben Variablen können statt in der Steam-Zeile auch in den Startvariablen
des Profils stehen, was sie pro Profil hält und auch auf Tool-Läufe anwendet:
[guide/usage.md](docs/guide/usage.md#variablen-und-wrapper-pro-profil).

Die tieferen Diagnoseschalter (`EIDOS_FUSE_TRACE`, die Bisektionsschalter für
Cache und Index, warum `EIDOS_FUSE_PASSTHROUGH` standardmäßig aus ist) stehen in
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Dokumentation

Alles steht in [docs/](../../../docs/), geordnet danach, wer liest, unter
[docs/README.md](docs/README.md): die Benutzeranleitung (Installation, die
Kommandozeile und die GUI, Tools wie xEdit und DynDOLOD, Grafik, Fehlersuche),
wie Eidos innen funktioniert und warum es existiert. Sicherheitsrichtlinie und
wie Sie eine Schwachstelle melden: [SECURITY.md](../../../SECURITY.md).

## Sprache

Die Seiten, die ein Spieler braucht, sind übersetzt. Englisch ist maßgeblich:
Wenn eine Übersetzung ihm widerspricht, hat die englische Datei recht.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Alles andere ist auf Englisch. `docs/internals/` und `docs/project/` werden von
Leuten gelesen, die auch den Rust-Code lesen, `CHANGELOG.md` wird generiert, und
auch `SECURITY.md` ist nicht übersetzt. Die ersten beiden zu übersetzen, würde
mehr als 28.000 Wörter hinzufügen, die synchron gehalten werden müssten, für
Leser, die sie nicht brauchen.

Jede Übersetzung trägt den Hash der englischen Datei, aus der sie gemacht wurde,
und die CI schlägt fehl, solange dieser Hash nicht mehr zum Englischen passt;
siehe [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Die Übersetzung
wird dann aktualisiert und mit `scripts/i18n-check.sh --fix` neu gestempelt. Eine
veraltete Übersetzung wird aktualisiert, nicht entfernt.

Eine Sprache ist ein Verzeichnis. `docs/i18n/<lang>/` spiegelt die Wurzel des
Repositorys, sodass `docs/i18n/de/docs/guide/install.md` das deutsche
`docs/guide/install.md` ist. Deshalb ist ein Link zwischen zwei übersetzten
Seiten dieselbe Zeichenkette wie der Link zwischen ihren englischen Originalen,
und deshalb ist das Zurückziehen einer Sprache ein einziges `rm -r`.

## Richtlinie zur Codesignierung

Jedes Release-Asset wird von der Organisation Project Colony in der CI signiert,
niemals auf dem Rechner eines Entwicklers. Neben jedem Asset auf der
Release-Seite:

| Datei | Was sie ist |
|---|---|
| `<asset>.sig` | eine ed25519-Signatur über das Asset, erstellt mit dem Release-Schlüssel der Organisation |
| `<asset>.meta` | drei Zeilen, die das Asset an seinen Dateinamen, seinen sha256 und die Release-Version binden |
| `<asset>.meta.sig` | eine ed25519-Signatur über die `.meta` |

Der private Schlüssel ist ein Geheimnis der Organisation und wird nur vom
gemeinsamen
[Sign-and-publish-Workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
verwendet, in einem Job, der nichts baut; die Jobs, die Eidos kompilieren,
bekommen ihn nie zu sehen. Colony prüft alle drei Dateien, bevor es Eidos
installiert oder aktualisiert, und verweigert ein Release, das älter ist als das
installierte. So prüfen Sie einen Download selbst mit OpenSSL 3 (dieselben
Befehle funktionieren für jedes Asset):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Wie Releases gebaut, signiert und veröffentlicht werden:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Datenschutz

Eidos sendet keine Telemetrie, keine Analysedaten und keine Absturzberichte. Es
verbindet sich nur mit den Servern in dieser Tabelle und nur für die dort
genannte Aktion. Der Offline-Modus (Preferences, Nexus) unterbindet jede
Verbindung zu Nexus und die tägliche Release-Prüfung.

| Daten | Gespeichert oder gesendet | Wo, und warum |
|---|---|---|
| Preferences, Instanzliste, Spiel- und Add-on-Definitionen | gespeichert | `~/.config/Colony/Eidos/` |
| Instanzen, Tool-Runtimes, Sitzungs-Logs | gespeichert | `~/.local/share/Colony/Eidos/` oder der Ordner, den Sie für eine portable Instanz gewählt haben |
| LOOT-Masterlisten | gespeichert | `~/.cache/Colony/Eidos/loot/` |
| Release-Prüfung | gesendet | Beim Start, höchstens einmal am Tag, eine Anfrage an `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos liest nur das Release-Tag und lädt nichts herunter. Sie ist standardmäßig an und in Preferences abschaltbar, und sie entfällt im Offline-Modus und bei einer Kopie, die unter `/usr` außerhalb von `/usr/local` installiert ist und die ein Paketmanager aktualisiert. [Datenschutzerklärung von GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus-Anmeldung | gesendet | Die Anmeldung öffnet `users.nexusmods.com` in Ihrem Browser, der das Ergebnis an einen Listener zurückgibt, den Eidos nur für diese Anmeldung auf `127.0.0.1` öffnet. Eidos tauscht es dann unter `users.nexusmods.com/oauth/token` gegen Tokens ein und holt die öffentlichen Schlüssel, mit denen Nexus sie signiert. [Datenschutzrichtlinie von Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus-Zugangsdaten | gespeichert | Das Access- und das Refresh-Token, in `~/.config/Colony/Eidos/nexus.ini` mit Modus 0600. Eine `api_key=`-Zeile, die ein älteres Eidos dort geschrieben hat, bleibt erhalten, wird aber nie gelesen; löschen Sie sie, wenn Sie möchten. |
| Nexus-Anfragen | gesendet | `api.nexusmods.com`, wenn Sie eine Nexus-Funktion verwenden: Ihr Kontoname und Premium-Status, Mod- und Dateidetails, Update-Prüfungen, Kollektionen und Download-Links. Ihre Bewertungen und verfolgten Mods werden bei jeder Update-Prüfung gelesen und geschrieben, wenn Sie Endorse oder Track drücken. Mod-Anforderungen, Beschreibungen und Changelogs sowie die Kollektionssuche gehen an `api.nexusmods.com/v2/graphql`. Jede Anfrage trägt Ihr Access-Token und nennt Eidos und seine Version. |
| Nexus-Downloads | gesendet | Die Dateien, die Sie anfordern, vom Nexus-CDN-Host, den der Download-Link nennt, gespeichert im Downloads-Ordner der Instanz mit einer `.meta` im MO2-Format neben jeder Datei. |
| `nxm://`-Links | gespeichert | `eidos nxm --register`, das `install.sh` für Sie ausführt, schreibt `~/.local/share/applications/eidos-nxm.desktop` und macht es über `xdg-mime` zum `nxm://`-Handler, sodass der „Mod Manager Download“-Knopf der Website seinen Link an Eidos übergibt. Das Arch-Paket installiert denselben Eintrag systemweit. |
| LOOT-Masterlisten | gesendet | `raw.githubusercontent.com/loot/...`, bei der ersten LOOT-Sortierung eines Spiels und immer dann, wenn Sie ein Update anfordern. Nichts von Ihnen wird gesendet. [Datenschutzerklärung von GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Tool-Runtimes | gesendet | Nur wenn Sie die Voraussetzungen eines Tools installieren, und nur für das, was das Release-Bundle nicht mitgeliefert hat (die einzelne Datei `eidos-linux` und ein Build aus den Quellen liefern nichts mit): die .NET-10-Runtime von `builds.dotnet.microsoft.com` und die Installer für Visual C++ und .NET 8 von `download.visualstudio.microsoft.com`, jeweils abgelehnt, wenn sie nicht zu der in Eidos eingebauten SHA-256 passen; das .NET-10-SDK, immer von `builds.dotnet.microsoft.com` und auf dieselbe Weise geprüft; und `winetricks` für die übrigen Voraussetzungen (.NET Framework 4.8, XACT, .NET 6 und 7), das herunterlädt, was es installiert, von Microsoft und den anderen Hosts, die winetricks verwendet. [Datenschutzerklärung von Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Sitzungs-Logs | gespeichert | `~/.local/share/Colony/Eidos/logs/`, Modus 0600: zehn Sitzungen pro Instanz, nach 30 Tagen bereinigt. Ihr Home-Ordner wird als `~` geschrieben und Ihr Benutzername als `<user>`, sodass ein Log unverändert in ein Issue eingefügt werden kann. Sie verlassen Ihren Rechner nie, es sei denn, Sie schicken sie ab. |
| Laufprotokolle | gespeichert | `<instance>/logs/sessions/`: für jeden Spiel- oder Tool-Lauf das Profil, die aktivierten Mods mit ihren Versionen und Nexus-Datei-IDs, die Plugin-Reihenfolge, eine Prüfsumme jeder Profil-INI, die Spiel- und Eidos-Versionen, der Exit-Code und die Spielstände, die der Lauf angelegt hat. Die 50 neuesten bleiben erhalten, `eidos sessions` liest sie, und `eidos pack` lässt sie weg. Sie bleiben auf Ihrem Rechner. |
| Update-Berichte | gespeichert | `<instance>/logs/updates/<mod>.json`: was das letzte Update oder Replace im Ordner dieser Mod geändert hat. `eidos pack` lässt sie weg. |
| Verknüpfungen | gespeichert | `install.sh` schreibt `eidos.desktop` und seine Icons unter `~/.local/share`; eine Tool- oder Profil-Verknüpfung, die Sie anlegen, kommt nach `~/.local/share/applications`. |
| Desktop-Benachrichtigungen | weder noch | Standardmäßig aus. Wenn an, gehen sie über D-Bus (`org.freedesktop.Notifications`) an den Benachrichtigungsdienst Ihres Desktops und nennen den Job, der geendet hat. |
| Links, die Sie öffnen | weder noch | Mod-Seiten und andere Links öffnen sich über `xdg-open` in Ihrem eigenen Browser. |
| Add-ons | weder noch | Erweiterungen, die Sie installieren, laufen als eigene Programme auf Ihrem Rechner; was sie tun, ist ihre Sache. |

## Lizenz

Der ursprüngliche Code von Eidos steht unter GPL-3.0-or-later; siehe
[LICENSE](../../../LICENSE). Diese Teile stehen unter GPL-3.0 ohne die Option
„or later“:

- die OMOD-Skriptübersetzungen in `eidos-install`; siehe die
  [Danksagungen an die ursprünglichen Autoren](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- das Hilfsprogramm `eidos-nif-preview`, gebaut auf der mitgelieferten
  nifly-Bibliothek; siehe seine
  [Komponentenlizenzen und Danksagungen](../../../crates/eidos-nif-preview/UPSTREAM.md);
- die Crates `esplugin` und `libloadorder`, die der Code für die
  Ladereihenfolge linkt und die `GPL-3.0` deklarieren.

Die anderen Rust-Crates in den Binärdateien behalten ihre eigenen Lizenzen (MIT,
Apache-2.0, BSD und ähnliche). Das Release-Bundle enthält ihre Hinweise in
`licenses/crates/`, mit einem Verzeichnis aller Crates und ihrer Lizenzen.

Die Binärdateien enthalten außerdem acht echte redistributierbare
Microsoft-DirectX-DLLs (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` und
`d3dx11_43`, jeweils 32- und 64-Bit), gzip-komprimiert gespeichert (7,9 MB statt
17,9 MB), die Eidos entpackt und in das Präfix des Spiels legt, wenn eine Mod oder
ein Tool sie braucht. Sie bleiben unter den Redistributionsbedingungen von
Microsoft; siehe ihre
[Herkunft](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Das Release-Bundle enthält außerdem unter `runtimes/` die Installer für
Microsofts Visual C++ 2015-2022, den Installer der .NET 8 Desktop Runtime und das
Zip der .NET-10-Runtime, so wie Microsoft sie veröffentlicht. Ihre Bedingungen
stehen in `licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
