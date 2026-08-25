<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Der native Linux-Mod-Manager, der Ihr Spiel nie anfasst.**

</div>

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

**Instanzen.** Global - zentral unter `~/.local/share/eidos` verwaltet - oder
portabel: ein eigenständiger Ordner, wo Sie wollen (eine zweite Platte, eine
Spielepartition), verschiebbar und isoliert, wie die von MO2. Portable Instanzen
werden über Sitzungen hinweg gemerkt; die GUI, der Steam-Start und jeder
CLI-Befehl folgen der zuletzt verwendeten, und jeder Befehl nimmt den Ordner
überall dort, wo er eine Spiel-ID nimmt. Einzelheiten in
[usage.de.md](docs/guide/usage.de.md#instanzen-global-und-portabel).

**Profile.** Mod-Reihenfolge, Plugin-Zustand, INIs und Spielstände pro Profil.
Spielstände werden analysiert, gegen Ihre aktuellen Plugins abgeglichen - mit
einem Knopf, der aktiviert, was ein Spielstand braucht - und nach jeder Sitzung
für Steam Cloud zurücksynchronisiert.

**Nexus.** Verbinden Sie ein Konto, und der "Mod Manager Download"-Knopf der
Website landet direkt in Ihrer Instanz, mit Aktualisierungsprüfungen gegen das,
was Sie installiert haben, wer jeden Mod gemacht hat und einem Link zu seinem
Profil. Ein **Collection**-Link listet ihre Mitglieder, abgeglichen mit Ihrer
Instanz - installiert, heruntergeladen, fehlend -, was das Lesen einer
Collection ist statt ihrer Installation, und der Bereich sagt warum. Der
Downloads-Tab ist eine Archivbibliothek: filtern, sortieren, ausblenden ohne zu
löschen, und die bereits installierten bereinigen. Ein **Offline**-Schalter
stoppt das alles.

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

**Wo es seine eigenen Dateien ablegt.** `~/.config/Colony/Eidos/` für das, was
Sie gewählt haben - Einstellungen, Ihre Nexus-Sitzung, Ihre Instanzliste, die
Spiel- und Add-on-Definitionen, die Sie geschrieben haben - mit Logs unter
`~/.local/state/Colony/Eidos/`. Die Anordnung, die jedes Programm der
Colony-Familie verwendet. Ein älteres Eidos legte diese in `~/.config/eidos/`
ab; der erste Start nach der Aktualisierung kopiert sie herüber, sagt es im Log
und lässt das alte Verzeichnis exakt so, wie es war.

## Wie es sich vergleicht

| | Eidos | MO2 über Wine | Fluorine-Manager | Limo / Link-Deployer |
|---|---|---|---|---|
| Manager läuft nativ | ✅ | ❌ Windows-App in Wine | ✅ (Qt-Portierung) | ✅ |
| Spielverzeichnis unangetastet | ✅ immer | ✅ | ✅ | ❌ Links werden hineingeschrieben |
| Mount sichtbar für | nur das Spiel | nur das Spiel | **das ganze System** | entfällt |
| Aufräumen nach Absturz nötig | keines, konstruktionsbedingt | keines | Wiederherstellung toter Mounts | manuelles Zurücknehmen |
| Root-Mods (ENB, Preloader) | ✅ nativ | Plugin nötig | Plugin nötig | teilweise |
| Benötigte Privilegien | keine | keine | `/etc/fuse.conf` ändern | keine |

## Wie schnell es ist

| | vorher | jetzt |
|---|---|---|
| einen Spielstand laden | ~20 Sekunden | **6-7 Sekunden** |
| Verzeichnislesevorgänge in einer Sitzung | 5,6 Millionen | 465 Tausend |

Zellenwechsel sind sofort. Der Gewinn kam davon, Ihren Mods weniger Fragen zu
stellen: Eine Datei zu finden befragte früher alle fünfzig der Reihe nach, und
einen Ordner aufzulisten tat es fünfzig Mal. Beides tut es nicht mehr. Gemessen
an einer echten Instanz, normal gespielt, nicht an einem Benchmark.

## Loslegen

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Setzen Sie dann die Steam-Startoption Ihres Spiels auf
`~/.local/bin/eidos-gui %command%` und drücken Sie Spielen.

Arch-Pakete und Release-Archive, was Sie vorher installiert haben müssen, und
der Weg über die Kommandozeile:
**[docs/guide/install.de.md](docs/guide/install.de.md)**.

## Steam-Startoptionen

Die Grundzeile ist alles, was die meisten Konfigurationen brauchen:

```
~/.local/bin/eidos-gui %command%
```

Alles Weitere sind Umgebungsvariablen, die davor gestapelt werden, und sie
lassen sich frei kombinieren:

| Sie wollen... | Davor setzen |
|---|---|
| DLSS mit Community Shaders | `PROTON_ENABLE_NVAPI=1` - ohne sie initialisiert sich DLSS stillschweigend nie; die vollständige Checkliste ist [guide/graphics.de.md](docs/guide/graphics.de.md) |
| einen FPS-Zähler auf dem Bildschirm | `DXVK_HUD=fps` |
| Frame-Interpolation auf Treiberebene, keine Mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - niemals zusammen mit der eigenen Frame Generation von Community Shaders |
| ausführliche Logs für einen Fehlerbericht | `EIDOS_LOG=debug` (Sitzungs-Logs landen in `~/.local/state/Colony/Eidos/logs/`) |
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
[guide/troubleshooting.de.md](docs/guide/troubleshooting.de.md).

## Wohin als Nächstes

| Wenn Sie... wollen | |
|---|---|
| es installieren | [guide/install.de.md](docs/guide/install.de.md) |
| die Kommandozeile und die GUI lernen | [guide/usage.de.md](docs/guide/usage.de.md) |
| xEdit, BodySlide oder DynDOLOD einrichten | [guide/tools.de.md](docs/guide/tools.de.md) |
| Fallout 4 spielen (F4SE, Versionen, der NVIDIA-Debris-Absturz) | [guide/fallout4.de.md](docs/guide/fallout4.de.md) |
| DLSS / Frame Generation zum Laufen bringen (Community Shaders) | [guide/graphics.de.md](docs/guide/graphics.de.md) |
| etwas reparieren, das falsch aussieht | [guide/troubleshooting.de.md](docs/guide/troubleshooting.de.md) |
| wissen, warum es schnell ist, und es selbst nachprüfen | [internals/performance.md](docs/internals/performance.md) |
| verstehen, wie es innen funktioniert | [internals/architecture.md](docs/internals/architecture.md) |
| es bauen, testen, dazu beitragen | [internals/contributing.md](docs/internals/contributing.md) |
| wissen, warum es überhaupt existiert | [project/landscape.md](docs/project/landscape.md) |

Der vollständige Index steht in [docs/README.de.md](docs/README.de.md); die
Sicherheitsrichtlinie und wie man eine Schwachstelle meldet in
[SECURITY.md](SECURITY.md).

## Sprache

Die Seiten, die ein Spieler braucht, sind übersetzt. **Englisch ist
maßgeblich**: Wenn eine Übersetzung ihm widerspricht, hat die englische Datei
recht.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Alles andere ist absichtlich auf Englisch, nicht aus Versäumnis.**
`docs/internals/` und `docs/project/` werden von Leuten gelesen, die auch den
Rust-Code lesen, und `CHANGELOG.md` wird generiert. Sie zu übersetzen wären
17.678 weitere Wörter, die ehrlich gehalten werden müssten, für ein Publikum,
das sie nicht braucht.

Jede Übersetzung trägt den Hash der englischen Datei, aus der sie gemacht wurde,
und die CI schlägt fehl, wenn das Englische vorauszieht - siehe
[`scripts/i18n-check.sh`](scripts/i18n-check.sh). Eine Übersetzung, die nicht
wieder auf den aktuellen Stand gebracht werden kann, wird **gelöscht**, nicht
stehen gelassen: Eine veraltete Seite sieht immer noch maßgeblich aus und gibt
die Befehle vom letzten Monat heraus, was für den Leser schlimmer ist, als auf
das Englische verwiesen zu werden.

Eine Sprache hinzuzufügen sind vier Dateien und eine Zeile in dieser Tabelle;
[`docs/internals/contributing.md`](docs/internals/contributing.md) hat die
Schritte.

## Unterstützte Spiele

**Skyrim SE/AE** - im echten Spiel erprobt. **Fallout 4** ist ebenfalls
durchgängig verdrahtet (F4SE wird automatisch eingesetzt, Archivinvalidierung,
Ladereihenfolge mit Sternchen, LOOT, `.fos`-Spielstände) - siehe
[guide/fallout4.de.md](docs/guide/fallout4.de.md). Gemäß dem gemeinsamen
Spieldeskriptor verdrahtet und auf der Suche nach Testern: Skyrim LE, Skyrim VR,
Enderal SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion und
Morrowind (die letzten beiden mounten und verwalten Mods; ihre nach Zeitstempel
geordneten Plugin-Listen werden noch nicht verwaltet).

Eine Familie hinzuzufügen ist eine Deskriptorzeile:
[internals/adding-games.md](docs/internals/adding-games.md).

## Vorarbeiten und Dank

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) und
  [usvfs](https://github.com/ModOrganizer2/usvfs) - die Semantik, die Eidos
  nachbildet, und die Codebasis, an der seine Parität studiert wurde
- [LOOT](https://loot.github.io/) - die Sortier-Engine, über libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) und die anderen Linux-Manager - der
  Beweis, dass es eine Gemeinschaft gibt, die das gelöst haben will

## Lizenz

GPL-3.0-or-later. Mod-Verwaltung gehört allen.
