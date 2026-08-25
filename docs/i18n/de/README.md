<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
[usage.de.md](docs/guide/usage.md#instanzen-global-und-portabel).

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
**[docs/guide/install.de.md](docs/guide/install.md)**.

## Steam-Startoptionen

Die Grundzeile ist alles, was die meisten Konfigurationen brauchen:

```
~/.local/bin/eidos-gui %command%
```

Alles Weitere sind Umgebungsvariablen, die davor gestapelt werden, und sie
lassen sich frei kombinieren:

| Sie wollen... | Davor setzen |
|---|---|
| DLSS mit Community Shaders | `PROTON_ENABLE_NVAPI=1` - ohne sie initialisiert sich DLSS stillschweigend nie; die vollständige Checkliste ist [guide/graphics.de.md](docs/guide/graphics.md) |
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
[guide/troubleshooting.de.md](docs/guide/troubleshooting.md).

## Wohin als Nächstes

| Wenn Sie... wollen | |
|---|---|
| es installieren | [guide/install.de.md](docs/guide/install.md) |
| die Kommandozeile und die GUI lernen | [guide/usage.de.md](docs/guide/usage.md) |
| xEdit, BodySlide oder DynDOLOD einrichten | [guide/tools.de.md](docs/guide/tools.md) |
| Fallout 4 spielen (F4SE, Versionen, der NVIDIA-Debris-Absturz) | [guide/fallout4.de.md](docs/guide/fallout4.md) |
| DLSS / Frame Generation zum Laufen bringen (Community Shaders) | [guide/graphics.de.md](docs/guide/graphics.md) |
| etwas reparieren, das falsch aussieht | [guide/troubleshooting.de.md](docs/guide/troubleshooting.md) |
| wissen, warum es schnell ist, und es selbst nachprüfen | [internals/performance.md](../../internals/performance.md) |
| verstehen, wie es innen funktioniert | [internals/architecture.md](../../internals/architecture.md) |
| es bauen, testen, dazu beitragen | [internals/contributing.md](../../internals/contributing.md) |
| wissen, warum es überhaupt existiert | [project/landscape.md](../../project/landscape.md) |

Eine Sprache ist ein Verzeichnis: `docs/i18n/de/` spiegelt die Wurzel des
Repositorys, weshalb ein Link zwischen zwei übersetzten Seiten dieselbe
Zeichenkette ist wie der Link zwischen ihren englischen Originalen.

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
17.678 weitere Wörter, die ehrlich gehalten werden müssten, für ein Publikum,
das sie nicht braucht.

Jede Übersetzung trägt den Hash der englischen Datei, aus der sie gemacht wurde,
und die CI schlägt fehl, wenn das Englische vorauszieht - siehe
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Eine Übersetzung, die nicht
wieder auf den aktuellen Stand gebracht werden kann, wird **gelöscht**, nicht
stehen gelassen: Eine veraltete Seite sieht immer noch maßgeblich aus und gibt
die Befehle vom letzten Monat heraus, was für den Leser schlimmer ist, als auf
das Englische verwiesen zu werden.

Eine Sprache hinzuzufügen sind vier Dateien und eine Zeile in dieser Tabelle;
[`docs/internals/contributing.md`](../../internals/contributing.md) hat die
Schritte.

## Unterstützte Spiele

**Skyrim SE/AE** - im echten Spiel erprobt. **Fallout 4** ist ebenfalls
durchgängig verdrahtet (F4SE wird automatisch eingesetzt, Archivinvalidierung,
Ladereihenfolge mit Sternchen, LOOT, `.fos`-Spielstände) - siehe
[guide/fallout4.de.md](docs/guide/fallout4.md). Gemäß dem gemeinsamen
Spieldeskriptor verdrahtet und auf der Suche nach Testern: Skyrim LE, Skyrim VR,
Enderal SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion und
Morrowind (die letzten beiden mounten und verwalten Mods; ihre nach Zeitstempel
geordneten Plugin-Listen werden noch nicht verwaltet).

Eine Familie hinzuzufügen ist eine Deskriptorzeile:
[internals/adding-games.md](../../internals/adding-games.md).

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
