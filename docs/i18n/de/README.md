<!-- eidos-i18n: source=README.md sha=c4073ef7daa93b14edfa3d1e00794a2cdbdf8066 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Der native Linux-Mod-Manager, der Ihr Spiel nie anfasst.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

Eidos gibt Bethesda-Spielen unter Linux das, was Mod Organizer 2 ihnen unter
Windows gibt - einen virtuellen, bei jedem Start neu zusammengeführten Blick auf
Ihre Mods - gebaut aus Linux-Primitiven statt aus Windows-API-Hooking. Kein Wine
für den Manager. Keine Dateien, die ins Spielverzeichnis kopiert werden. Kein
Aufräumweg, weil es nichts aufzuräumen gibt.

```
Steam ──> eidos-gui %command% ──> [ privater Namensraum ]
                                  │  mods ⊕ Spiel  ──> was das Spiel sieht
                                  └─ stirbt mit dem Spiel; die Installation bleibt unberührt
```

> **Status:** Skyrim SE wird täglich durch Eidos gespielt - SKSE,
> Script-Extender-Preloader, Creation Club, mit LOOT sortierte
> Ladereihenfolgen, Spielstände pro Profil, alles. Eine Spielfamilie bisher im
> echten Spiel erprobt; zehn weitere sind verdrahtet und warten auf Tester.

## Warum Eidos

- 🔒 **Ein Mount, das nur Ihr Spiel sieht.** Der zusammengeführte Blick lebt in
  einem privaten Mount-Namensraum: Ihr Dateimanager, Ihr Backup-Lauf, ein
  zweites Spiel - keiner von ihnen sieht ihn, keiner braucht eine Berechtigung
  dafür. Töten Sie das Spiel, ziehen Sie den Stecker: Der Namensraum stirbt mit
  dem Prozessbaum, und Ihre Installation ist exakt wie zuvor. Es gibt *von der
  Konstruktion her* keine Rückstände.
- 🧾 **Eine einzige Kopie der Wahrheit.** Ihr Profil besitzt seine Mod-Liste,
  seine Plugin-Reihenfolge, seine INIs und seine Spielstände. Die Plugin-Dateien
  und das Spielstandverzeichnis werden beim Start per Bind-Mount über die eigenen
  Pfade des Spiels gelegt, sodass selbst die Schreibvorgänge des Spiels in Ihrem
  Profil landen. Ein Profilwechsel wechselt alles.
- 🐧 **Vollständig rootlos.** Kein setuid-Helfer, kein Daemon, kein
  `sudo setcap`, keine Änderungen an `/etc/fuse.conf`. Eine Binärdatei, eine
  Steam-Startoption.
- 🛡️ **Schutzmechanismen, die ihre Belege zeigen.** Ein Absturz, der Ihre
  Plugin-Liste beschädigt, wird gegen einen vor der Sitzung genommenen
  Schnappschuss gemeldet, mit Wiederherstellung per Klick. Eine Übernahme, die
  Ihre Ladereihenfolge löschen würde, wird verweigert und sagt warum.

### Wie es sich vergleicht

| | Eidos | MO2 über Wine | Fluorine-Manager | Limo / Link-Deployer |
|---|---|---|---|---|
| Manager läuft nativ | ✅ | ❌ Windows-App in Wine | ✅ (Qt-Portierung) | ✅ |
| Spielverzeichnis unangetastet | ✅ immer | ✅ | ✅ | ❌ Links werden hineingeschrieben |
| Mount sichtbar für | nur das Spiel | nur das Spiel | **das ganze System** | entfällt |
| Aufräumen nach Absturz nötig | keines, konstruktionsbedingt | keines | Wiederherstellung toter Mounts | manuelles Zurücknehmen |
| Root-Mods (ENB, Preloader) | ✅ nativ | Plugin nötig | Plugin nötig | teilweise |
| Benötigte Privilegien | keine | keine | `/etc/fuse.conf` ändern | keine |

### Wie schnell es ist

| | vorher | jetzt |
|---|---|---|
| einen Spielstand laden | ~20 Sekunden | **6-7 Sekunden** |
| Verzeichnislesevorgänge in einer Sitzung | 5,6 Millionen | 465 Tausend |

Zellenwechsel sind sofort. Der Gewinn kam davon, Ihren Mods weniger Fragen zu
stellen: Eine Datei zu finden befragte früher alle fünfzig der Reihe nach, und
einen Ordner aufzulisten tat es fünfzig Mal. Beides tut es nicht mehr. Gemessen
an einer echten Instanz, normal gespielt, nicht an einem Benchmark.

### Vorarbeiten und Dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) und
  [usvfs](https://github.com/ModOrganizer2/usvfs) - die Semantik, die Eidos
  nachbildet, und die Codebasis, an der seine Parität studiert wurde
- [LOOT](https://loot.github.io/) - die Sortier-Engine, über libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) und die anderen Linux-Manager - der
  Beweis, dass es eine Gemeinschaft gibt, die das gelöst haben will

## Was es tut

**Mods.** Einfache Archive, FOMOD-Assistenten, BAIN-Pakete von Wrye Bash, eine
manuelle Auswahl für den Rest - und **Root-Mods nativ**
(Script-Extender-Preloader, ENB, Engine Fixes), ohne Root-Builder-Plugin und
ohne dass etwas in Ihre Installation kopiert wird. Blenden Sie einzelne Dateien
aus, gruppieren Sie mit Separatoren, gezielte Verschiebungen, Notizen und
Kategorien pro Mod, und ein Importeur für MO2-Profile.

Die Liste ist die von MO2, mit ihren Gewohnheiten: acht optionale Spalten und
eine Sortierung nach jeder davon, Gruppierung nach Kategorie oder nach Quelle,
Gesten per Doppelklick, Tippen zum Springen, Backups pro Mod, die untätig
bleiben, bis Sie sie wiederherstellen, und Hinweisflaggen für einen Mod, dessen
Aufbau dieses Spiel nicht laden wird oder der für ein anderes heruntergeladen
wurde. Sein Dateibaum erledigt die gewöhnlichen Operationen - neuer Ordner,
umbenennen, löschen, öffnen - und zeigt Bilder und Texte in der Vorschau, ohne
etwas zu starten.

**Plugins.** Die Ladereihenfolge mit eingebauter LOOT-Sortierung, Mod-Indizes
so, wie das Spiel sie berechnet, Warnungen vor fehlenden Mastern, und Ihre DLC-
und Creation-Club-Inhalte angezeigt als die nicht verwalteten Zeilen, die sie
sind.

**Instanzen.** Global - zentral unter `~/.local/share/Colony/Eidos/instances/` verwaltet - oder
portabel: ein eigenständiger Ordner, wo Sie wollen (eine zweite Platte, eine
Spielepartition), verschiebbar und isoliert, wie die von MO2. Portable Instanzen
werden über Sitzungen hinweg gemerkt; die GUI, der Steam-Start und jeder
CLI-Befehl folgen der zuletzt verwendeten, und jeder Befehl nimmt den Ordner
überall dort, wo er eine Spiel-ID nimmt. Einzelheiten in
[usage.md](docs/guide/usage.md#instanzen-global-und-portabel).

**Profile.** Mod-Reihenfolge, Plugin-Zustand, INIs und Spielstände pro Profil.
Spielstände werden analysiert, gegen Ihre aktuellen Plugins abgeglichen - mit
einem Knopf, der aktiviert, was ein Spielstand braucht - und nach jeder Sitzung
für Steam Cloud zurücksynchronisiert.

**Nexus.** Verbinden Sie ein Konto, um „Mod Manager Download“-Links zu empfangen, Updates zu prüfen und Autoren samt Profilen anzuzeigen. Kollektionen werden anhand ihrer Rezepte installiert, mit fortsetzbarem Fortschritt und ausdrücklichen Berichten über nicht unterstützte Schritte. Der Abgleich prüft die genauen Quelldateien; unvollständige ältere Metadaten bleiben ungeprüft. Der Downloads-Tab filtert, sortiert, verbirgt und entfernt Archive. Ein **Offline**-Schalter unterbindet den Nexus-Zugriff.

**Werkzeuge.** xEdit, BodySlide, DynDOLOD und Konsorten laufen *durch den
zusammengeführten Blick* im Proton-Präfix des Spiels - sie sehen Ihre Mods, ihre
Ausgabe landet in Overwrite, und ein Klick macht daraus einen echten Mod. Welche
Runtime jedes einzelne braucht, wird auf Anfrage geholt, sodass eine fehlende
DLL ein Knopf ist statt eines Nachmittags. xEdit und sein
QuickAutoClean-Zwilling werden für Sie gefunden - im Spielordner, in einem Mod
oder in dem Werkzeugverzeichnis, das Sie neben Ihren Spielen halten - mit den
richtigen Runtimes bereits ausgewählt. Heften Sie die an, die Sie verwenden,
blenden Sie die aus, die Sie nicht verwenden, geben Sie einem Werkzeug seine
eigene Steam-AppID, wenn es seine eigene Steam-App ist, und schreiben Sie eine
`.desktop`-Verknüpfung, die es durch den zusammengeführten Blick startet, ohne
Eidos überhaupt zu öffnen.

**Diagnose.** Fehlende Master, verwaiste Archive, Abweichungen in der Mod-Liste,
beschädigte Plugin-Sätze - und, nach einem Lauf, was das eigene Log des Script
Extenders als tatsächlich geladen ausweist.

**Wo es seine eigenen Dateien ablegt.** Die Anordnung, die jedes Programm der
Colony-Familie verwendet: `~/.config/Colony/Eidos/` für das, was Sie gewählt
haben - Einstellungen, Ihre Nexus-Sitzung, Ihre Instanzliste, die Spiel- und
Add-on-Definitionen, die Sie geschrieben haben - `~/.local/share/Colony/Eidos/`
für globale Instanzen, heruntergeladene Laufzeitumgebungen und Sitzungs-Logs,
und `~/.cache/Colony/Eidos/` für LOOT-Masterlisten. Ein älteres Eidos legte
diese in `~/.config/eidos/`, `~/.local/share/eidos/` und `~/.local/state/` ab;
der erste Start nach der Aktualisierung überträgt sie, sagt es im Log und lässt
die alten Verzeichnisse exakt so, wie sie waren. Solange ein Umzug nicht
abgeschlossen ist, verwendet Eidos weiter die alten Dateien. Details in
[internals/paths.md](../../internals/paths.md).

Siehe [Integritätsprüfungen und generierte Dateien](../../internals/integrity.md) für den Schutz beim Ersetzen, BSA/BA2-Dateikonflikte, statische SKSE-Diagnosen und Generierungsnachweise, einschließlich ihrer Grenzen.

### Unterstützte Spiele

**Skyrim SE/AE** - im echten Spiel erprobt. **Fallout 4** ist ebenfalls
durchgängig verdrahtet (F4SE wird automatisch eingesetzt, Archivinvalidierung,
Ladereihenfolge mit Sternchen, LOOT, `.fos`-Spielstände) - siehe
[guide/fallout4.md](docs/guide/fallout4.md). Gemäß dem gemeinsamen
Spieldeskriptor verdrahtet und auf der Suche nach Testern: Skyrim LE, Skyrim VR,
Enderal SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion und
Morrowind (die letzten beiden mounten und verwalten Mods; ihre nach Zeitstempel
geordneten Plugin-Listen werden noch nicht verwaltet).

Eine Familie hinzuzufügen ist eine Deskriptorzeile:
[internals/adding-games.md](../../internals/adding-games.md).

## Installation

### Über Colony (empfohlen)

Suchen Sie in [Colony](https://github.com/Project-Colony/Colony) nach **Eidos**
und installieren Sie es. Updates kommen über den Launcher. Colony installiert
eine einzige Datei, `eidos-linux`: das Fenster und zugleich die Kommandozeile
`eidos`, wenn sie über einen Link namens `eidos` aufgerufen wird. Die
NIF-Modellvorschau braucht ein Hilfsprogramm, das nur das Bundle unten
mitbringt; ohne es sagt die Vorschau das, und alles andere funktioniert. Was
nach der Installation zu tun ist und wie Sie Steam darauf zeigen lassen:
[docs/guide/install.md](docs/guide/install.md#über-colony).

### Direkter Download der Binärdateien

Holen Sie sich ein Asset aus dem [neuesten Release](https://github.com/Project-Colony/Eidos/releases/latest):

| Asset | Was es ist |
|---|---|
| `eidos-linux` | das ganze Programm in einer Datei, so wie Colony es installiert |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, das NIF-Vorschau-Hilfsprogramm, Icons und ein Installer, der den Desktop-Eintrag und den Nexus-Download-Handler registriert |

Prüfen Sie den Download zuerst, wie die
[Richtlinie zur Codesignierung](#richtlinie-zur-codesignierung) zeigt, dann:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Der Installer legt alles ohne sudo nach `~/.local/bin`; `--system` und
`--bindir DIR` wählen einen anderen Ort. Beide Assets brauchen x86_64-Linux mit
glibc 2.39 oder neuer, FUSE (`fusermount3`) und 7-Zip für Mod-Archive.

### Aus dem Quelltext bauen

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Benötigt Rust 1.89 oder neuer (die `rust-version`, die die CI prüft; CI und
Releases bauen mit 1.99.0), dazu `just`, CMake 3.20+ und einen C++17-Compiler
für das NIF-Vorschau-Hilfsprogramm. Ein Arch-Paket wird aus
`packaging/PKGBUILD` gebaut. Voraussetzungen und die Befehle ohne `just`:
[docs/guide/install.md](docs/guide/install.md).

### Steam-Startoptionen

Eidos läuft *als* Startbefehl des Spiels - so mountet es, bevor das Spiel
startet. Die Grundzeile ist alles, was die meisten Konfigurationen brauchen, mit
dem absoluten Pfad Ihrer Installation (Steam liest Ihren `PATH` nicht):

```
~/.local/bin/eidos-gui %command%
```

Aus Colony lautet dieser Pfad `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Drücken Sie dann Spielen: Eidos öffnet sich auf der Instanz dieses Spiels, und
Run startet das Spiel durch den zusammengeführten Blick.

Alles Weitere sind Umgebungsvariablen, die davor gestapelt werden, und sie
lassen sich frei kombinieren:

| Sie wollen... | Davor setzen |
|---|---|
| DLSS mit Community Shaders | `PROTON_ENABLE_NVAPI=1` - ohne sie initialisiert sich DLSS stillschweigend nie; die vollständige Checkliste ist [guide/graphics.md](docs/guide/graphics.md) |
| einen FPS-Zähler auf dem Bildschirm | `DXVK_HUD=fps` |
| Frame-Interpolation auf Treiberebene, keine Mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - niemals zusammen mit der eigenen Frame Generation von Community Shaders |
| ausführliche Logs für einen Fehlerbericht | `EIDOS_LOG=debug` (Sitzungs-Logs landen in `~/.local/share/Colony/Eidos/logs/`) |
| einen I/O-Bericht pro Sitzung vom Mount | `EIDOS_FUSE_STATS=1` |
| eine andere Anzahl an FUSE-Workern | `EIDOS_FUSE_THREADS=8` (Standard 4; `1` ist das Erste, was man bei der Jagd auf einen Nebenläufigkeitsfehler versucht) |
| diesen Start an eine portable Instanz binden | `EIDOS_INSTANCE=/path/to/folder` - ohne sie öffnet Eidos die zuletzt verwendete Instanz, was meistens das ist, was Sie wollen |

Die Zeile, die man für eine moderne gemoddete Konfiguration behält (Community
Shaders, DLSS, Frame Generation) - das ist der endgültige Befehl, kein Beispiel:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Setzen Sie `DXVK_HUD=fps` davor, während Sie prüfen, ob die Konfiguration
funktioniert, und lassen Sie es weg, sobald sie es tut.

Die tieferen Diagnoseschalter (`EIDOS_FUSE_TRACE`, die Bisektionsschalter für
Cache und Index, warum `EIDOS_FUSE_PASSTHROUGH` standardmäßig aus ist) leben in
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Dokumentation

Alles steht in [docs/](../../../docs/), geordnet danach, wer liest, unter
[docs/README.md](docs/README.md): die Benutzeranleitung (Installation, die
Kommandozeile und die GUI, Tools wie xEdit und DynDOLOD, Grafik, Fehlersuche),
wie Eidos innen funktioniert und warum es existiert. Sicherheitsrichtlinie und
wie Sie eine Schwachstelle melden: [SECURITY.md](../../../SECURITY.md).

## Sprache

Die Seiten, die ein Spieler braucht, sind übersetzt. **Englisch ist
maßgeblich**: Wenn eine Übersetzung ihm widerspricht, hat die englische Datei
recht.

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

**Alles andere ist absichtlich auf Englisch, nicht aus Versäumnis.**
`docs/internals/` und `docs/project/` werden von Leuten gelesen, die auch den
Rust-Code lesen, und `CHANGELOG.md` wird generiert. Sie zu übersetzen wären
18.536 weitere Wörter, die ehrlich gehalten werden müssten, für ein Publikum,
das sie nicht braucht.

Jede Übersetzung trägt den Hash der englischen Datei, aus der sie gemacht wurde,
und die CI schlägt fehl, wenn das Englische vorauszieht - siehe
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Eine Übersetzung, die nicht
wieder auf den aktuellen Stand gebracht werden kann, wird **gelöscht**, nicht
stehen gelassen: Eine veraltete Seite sieht immer noch maßgeblich aus und gibt
die Befehle vom letzten Monat heraus, was für den Leser schlimmer ist, als auf
das Englische verwiesen zu werden.

Eine Sprache ist ein Verzeichnis. `docs/i18n/<lang>/` spiegelt die Wurzel des
Repositorys, sodass `docs/i18n/de/docs/guide/install.md` das deutsche
`docs/guide/install.md` ist - deshalb ist ein Link zwischen zwei übersetzten
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
verbindet sich mit keinem Server außer denen in dieser Tabelle, und nur für die
dort genannte Aktion. Der **Offline**-Modus (Preferences -> Nexus) unterbindet
jede Verbindung zu Nexus.

| Daten | Gespeichert oder gesendet | Wo, und warum |
|---|---|---|
| Preferences, Instanzliste, Spiel- und Add-on-Definitionen | gespeichert | `~/.config/Colony/Eidos/` |
| Instanzen, heruntergeladene Laufzeitumgebungen, Sitzungs-Logs | gespeichert | `~/.local/share/Colony/Eidos/` oder der Ordner, den Sie für eine portable Instanz gewählt haben |
| LOOT-Masterlisten | gespeichert | `~/.cache/Colony/Eidos/loot/` |
| Nexus-Anmeldung | gesendet | Die Anmeldung öffnet `users.nexusmods.com` in Ihrem Browser, der das Ergebnis an einen Listener zurückgibt, den Eidos nur für diese Anmeldung auf `127.0.0.1` öffnet. Eidos tauscht es dann unter `users.nexusmods.com/oauth/token` gegen Tokens ein und holt die öffentlichen Schlüssel, mit denen Nexus sie signiert. [Datenschutzrichtlinie von Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus-Zugangsdaten | gespeichert | Das Access- und das Refresh-Token, in `~/.config/Colony/Eidos/nexus.ini` mit Modus 0600. Eine `api_key=`-Zeile, die ein älteres Eidos dort geschrieben hat, bleibt erhalten, wird aber nie gelesen; löschen Sie sie, wenn Sie möchten. |
| Nexus-Anfragen | gesendet | `api.nexusmods.com`, wenn Sie eine Nexus-Funktion verwenden: Ihr Kontoname und Premium-Status, Mod- und Dateidetails, Update-Prüfungen, Kollektionen und Download-Links. Jede Anfrage trägt Ihr Access-Token und nennt Eidos und seine Version. |
| Nexus-Downloads | gesendet | Die Dateien, die Sie anfordern, vom Nexus-CDN-Host, den der Download-Link nennt, gespeichert im Downloads-Ordner der Instanz mit einer `.meta` im MO2-Format neben jeder Datei. |
| `nxm://`-Links | gespeichert | `eidos nxm --register`, das `install.sh` für Sie ausführt, schreibt `~/.local/share/applications/eidos-nxm.desktop` und macht es über `xdg-mime` zum `nxm://`-Handler, sodass der „Mod Manager Download“-Knopf der Website seinen Link an Eidos übergibt. Das Arch-Paket installiert denselben Eintrag systemweit. |
| LOOT-Masterlisten | gesendet | `raw.githubusercontent.com/loot/...`, bei der ersten LOOT-Sortierung eines Spiels und immer dann, wenn Sie ein Update anfordern. Nichts von Ihnen wird gesendet. [Datenschutzerklärung von GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Laufzeitumgebungen für Tools | gesendet | Nur wenn Sie eine Tool-Voraussetzung installieren oder eine Mod eine braucht: die .NET-Runtime von `builds.dotnet.microsoft.com`, die abgelehnt wird, wenn sie nicht zu der in Eidos eingebauten SHA-256 passt, und `winetricks`, das herunterlädt, was es in das Proton-Präfix des Spiels installiert (Visual-C++- und .NET-Redistributables), von Microsoft und den anderen Hosts, die winetricks verwendet. [Datenschutzerklärung von Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Sitzungs-Logs | gespeichert | `~/.local/share/Colony/Eidos/logs/`, Modus 0600: zehn Sitzungen pro Instanz, nach 30 Tagen bereinigt. Ihr Home-Ordner wird als `~` geschrieben und Ihr Benutzername als `<user>`, sodass ein Log unverändert in ein Issue eingefügt werden kann. Sie verlassen Ihren Rechner nie, es sei denn, Sie schicken sie ab. |
| Verknüpfungen | gespeichert | `install.sh` schreibt `eidos.desktop` und seine Icons unter `~/.local/share`; eine Tool-Verknüpfung, die Sie anlegen, kommt nach `~/.local/share/applications`. |
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
`d3dx11_43`, jeweils 32- und 64-Bit), die Eidos in das Präfix des Spiels legt,
wenn eine Mod oder ein Tool sie braucht. Sie bleiben unter den
Redistributionsbedingungen von Microsoft; siehe ihre
[Herkunft](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Mod-Verwaltung gehört allen.
