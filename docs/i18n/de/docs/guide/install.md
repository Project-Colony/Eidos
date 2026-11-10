<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Eidos installieren

Vier Wege hinein. Colony installiert eine einzige Datei, `eidos-linux`, die das
ganze Programm ist. Das Release-Bundle, das Arch-Paket und ein Build aus dem
Quelltext liefern `eidos` (die Kommandozeile), `eidos-gui` und das statische
NIF-Vorschau-Hilfsprogramm `eidos-nif-preview` nebeneinander. Das Arch-Paket und
ein Build aus dem Quelltext installieren außerdem `eidos-fuse` und
`eidos-launch`, die beiden tiefer liegenden Kommandozeilenprogramme. Jeder Weg kann
den `nxm://`-Handler registrieren, durch den der „Mod Manager Download“-Knopf auf
Nexus in Ihrer Instanz landet.

## Was Sie vorher brauchen

| Voraussetzung | Warum |
|---|---|
| Linux mit FUSE | `fusermount3` im PATH. Jede aktuelle Distribution liefert es mit. |
| Ein Proton-Spiel, einmal gestartet | Steam legt das Wine-Präfix des Spiels erst beim ersten Start an, und Eidos arbeitet darin. Eidos erkennt auch die GOG- und Epic-Kopien von Skyrim Special Edition, die Epic-Kopien von Fallout 3 und Fallout: New Vegas und die GOG-Kopie von Stardew Valley. Diese laufen über Heroic oder Legendary, mit Eidos als Wrapper des Spiels dort: siehe [usage.md](usage.md#das-spiel-starten). |
| Ein 7-Zip-Programm: `7z`, `7zz` oder `7za` | Zum Installieren von Mod-Archiven. `7zip` unter Arch, `p7zip` in den meisten anderen Distributionen. |
| `par2` (optional) | PAR2-Wiederherstellungsdaten für Sicherungen mit `eidos pack`. `par2cmdline` unter Arch und Fedora, `par2` unter Debian und Ubuntu. |

Kein root, kein Daemon, keine Änderung an `/etc/fuse.conf` und nichts, was Ihren
Gruppen hinzugefügt werden müsste. Eidos mountet in einem privaten Namensraum,
der dem Spielprozess gehört.

Manche Systemeinstellungen zeigen sich nur als Abstürze. Der Tab Diagnostics
prüft `vm.max_map_count` (1048576 oder mehr), das harte Limit offener Dateien
(524288 oder mehr) und ob das Spiel, sein Präfix oder die Instanz auf NTFS, exFAT
oder FAT liegt; [troubleshooting.md](troubleshooting.md#das-system) nennt die
Abhilfe für jedes.

## Über Colony

Suchen Sie in [Colony](https://github.com/Project-Colony/Colony) nach **Eidos**
und installieren Sie es; Updates kommen über den Launcher. Colony hält eine
einzige Datei vor, `~/.local/share/Colony/apps/Eidos/eidos-linux`. Starten Sie
sie, und das Fenster öffnet sich. Über einen Link namens `eidos` aufgerufen, ist
dieselbe Datei die Kommandozeile. Legen Sie einen in Ihren `PATH` und
registrieren Sie den Nexus-Download-Handler:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

Das Fenster braucht keinen Link: Wann immer es ein Spiel oder ein Tool startet,
führt es seine eigene Datei als Kommandozeile aus. Die NIF-Modellvorschau braucht
`eidos-nif-preview`, das nur das Release-Bundle, das Arch-Paket und Builds aus dem
Quelltext mitbringen; ohne es sagt die Vorschau das, und sonst ist nichts
betroffen. `eidos-linux` enthält außerdem keine der Microsoft-Runtimes, die das
Bundle mitliefert ([unten](#die-microsoft-runtimes-im-bundle)): Braucht ein Tool
eine, lädt Eidos sie von Microsoft herunter, sobald Sie die Installation der
Voraussetzungen starten. Lassen Sie Steam auf die Datei selbst zeigen, wie unten
beschrieben.

## Ein Release-Bundle

Laden Sie `eidos-linux-bundle.tar.gz` sowie die zugehörigen `.sig`, `.meta` und
`.meta.sig` aus dem [neuesten Release](https://github.com/Project-Colony/Eidos/releases/latest)
herunter und prüfen Sie sie, wie die
[Richtlinie zur Codesignierung](../../README.md#richtlinie-zur-codesignierung) im
README zeigt. Das Bundle enthält auf oberster Ebene `eidos`, `eidos-gui` und
`eidos-nif-preview`, dazu `install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, die
Icons in `assets/brand/png`, die Hinweise zu Drittanbietern in `licenses/` (nifly,
omod, microsoft-directx, microsoft-runtimes und die Rust-Crates) und den Ordner
`runtimes/` weiter unten. `eidos-fuse` und `eidos-launch` enthält es nicht. Dann:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` kopiert die Binärdateien nach `~/.local/bin` oder, wenn gesetzt, nach
`$XDG_BIN_HOME`. `--system` legt sie über sudo nach `/usr/local/bin`, `--bindir DIR`
irgendwo anders hin, und `--from DIR` nimmt die Binärdateien aus einem anderen
Verzeichnis. Das Skript außerdem:

- registriert den `nxm://`-Handler mit `eidos nxm --register`, sodass der
  Registrierungsschritt im Abschnitt zu Colony nicht nötig ist;
- schreibt `~/.local/share/applications/eidos.desktop` und die Eidos-Icons unter
  `~/.local/share/icons/hicolor`;
- kopiert die Lizenzhinweise nach `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` mit `--system`);
- kopiert `runtimes/` nach `~/.local/share/Colony/Eidos/runtimes`, auch mit
  `--system`, sodass die Runtimes dem Benutzer gehören, der das Skript ausführt;
- gibt zum Schluss die Steam-Startoption mit dem absoluten Pfad des von ihm
  installierten `eidos-gui` aus.

`--cap` bittet sudo, dem installierten `eidos` `CAP_SYS_ADMIN` zu geben (siehe
FUSE-Passthrough unten). `--no-cap` ist die Voreinstellung und lässt es rootlos.
Erneutes Ausführen des Skripts ist der vorgesehene Weg zum Aktualisieren. Das
Bundle und `eidos-linux` brauchen x86_64-Linux mit glibc 2.39 oder neuer.

### Die Microsoft-Runtimes im Bundle

Der Ordner `runtimes/` des Bundles enthält die Microsoft-Dateien, die Tools
anfordern: `VC_redist.x86.exe` und `VC_redist.x64.exe` (Visual C++ 2015-2022, die
Voraussetzung `vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (die .NET 8
Desktop Runtime, für `dotnet8` und `dotnetdesktop8`) und
`dotnet-runtime-10.0.10-win-x64.zip` (die .NET-10-Runtime, für `dotnet10`). Eidos
führt die Installer für Visual C++ und .NET 8 selbst im Präfix des Spiels aus, ohne
winetricks, und entpackt das .NET-10-Zip einmal nach
`~/.local/share/Colony/Eidos/runtimes`, wo jede Instanz es mitnutzt (78 MB
entpackt). Jede Datei wird nur verwendet, wenn ihre SHA-256 mit der in Eidos
festgelegten übereinstimmt. Das .NET-10-SDK, das Synthesis braucht (`dotnetsdk10`,
etwa 300 MB), ist nicht im Bundle und wird immer heruntergeladen.

Ein entpacktes Bundle läuft auch an Ort und Stelle: Eidos sucht nach dem oben
genannten Datenordner `runtimes/` neben seiner eigenen Binärdatei.

## Arch

```bash
cd packaging && makepkg -si
```

Das PKGBUILD baut den aktuellen Stand von `main` aus dem Quelltext. Es installiert
`eidos`, `eidos-gui`, `eidos-nif-preview`, `eidos-fuse` und `eidos-launch` nach
`/usr/bin`, mit einem systemweiten `nxm://`-Handler, einem Desktop-Eintrag und den
Icons. Es hängt von `fuse3`, `7zip` und `xdg-utils` ab. Es liefert keinen Ordner
`runtimes/` mit, sodass die Microsoft-Runtimes heruntergeladen werden, wenn ein
Tool sie braucht. `winetricks` ist optional: Eidos nutzt es weiterhin für die
Voraussetzungen, die es nicht selbst installiert, etwa `dotnet48` und `xact`.

## Aus dem Quelltext

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Zum Bauen aus dem Quelltext benötigen Sie Rust 1.92 oder neuer, `just`, CMake 3.20+,
einen C++17-Compiler und Python 3 für die Tests des nativen Hilfsprogramms. `just
build` baut den ganzen Workspace, also werden auch `eidos-fuse` und `eidos-launch`
gebaut, und `just install` führt `packaging/install.sh` auf der Build-Ausgabe aus,
das sie mit dem Rest kopiert. Befehle ohne `just` finden Sie in der
[Anleitung zum nativen Hilfsprogramm](../../../../../crates/eidos-nif-preview/README.md).
Bauen Sie Rust mit `cargo build --release --locked`, kopieren Sie das Hilfsprogramm
nach `target/release` und führen Sie anschließend
`packaging/install.sh --from target/release` aus. Auch ein Build aus dem
Quelltext hat keinen Ordner `runtimes/`, sodass die Microsoft-Runtimes
heruntergeladen werden, wenn ein Tool sie braucht.

Das Release-Profil nutzt eine Codegen-Einheit, Fat LTO und entfernte Symbole. Mit
rustc 1.99.1 und 3 Build-Jobs dauerte ein Kaltbau von `eidos` und `eidos-gui`
334 s, gegenüber 243 s mit Thin LTO, und ergab ein `eidos` von 20,9 MB und ein
`eidos-gui` von 49,7 MB. Jede Binärdatei bettet acht DirectX-DLLs gzip-komprimiert
ein: 7,9 MB statt 17,9 MB. `just profile=dev build` baut ohne Optimierung.

## Dann: Steam darauf zeigen lassen

Eidos läuft *als* Startbefehl Ihres Spiels - so kommt es dazu, den Blick zu
mounten, bevor das Spiel startet. In Steam: Rechtsklick auf das Spiel ->
Eigenschaften -> Startoptionen, und tragen Sie den absoluten Pfad des
Eidos-Fensters gefolgt von `%command%` ein:

```
~/.local/bin/eidos-gui %command%
```

Verwenden Sie den absoluten Pfad - Steam liest den `PATH` Ihrer Shell nicht. Der
Pfad hängt davon ab, wie Sie Eidos installiert haben, und `install.sh` gibt die
Zeile für das Verzeichnis aus, das es genutzt hat:

| Installation | Startoption |
|---|---|
| Release-Bundle oder `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch-Paket | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Drücken Sie Spielen. Eidos öffnet sich auf der Instanz dieses Spiels; Mods
installieren, mit LOOT sortieren, auf Run klicken. Beim Beenden verschwindet das
Mount mit dem Spiel, und Ihre Installation ist exakt wie zuvor.

Das Spiel startet immer über seinen Store. In einem Fenster, das Sie selbst
geöffnet haben, führt Run `steam -applaunch <appid>` aus, Steam startet Eidos über
die Startoption, und es öffnet sich ein zweites Eidos-Fenster, in dem Sie Run
drücken. Solange die Startoption Eidos nicht startet, startet Run nichts, und die
Statuszeile zeigt die Zeile, die zu setzen ist. Mit Start the game at once when a
store opens Eidos (Preferences, General, Running a game; standardmäßig aus) startet
Spielen in Steam das Spiel ohne das Fenster, wenn Health kein Problem auflistet.
Heroic und Legendary funktionieren genauso über ihre Wrapper-Einstellung;
[usage.md](usage.md#das-spiel-starten) hat die Einzelheiten.

Der Tab Diagnostics prüft die Zeile. Eine Health-Karte zeigt die Zeile, die zu
setzen ist, wenn die Startoption leer ist, keine Eidos-Binärdatei nennt, auf eine
nicht vorhandene Datei zeigt, kein `%command%` hinter Eidos hat oder eine andere
Kopie von Eidos startet als die laufende.

### Wenn Sie das Terminal bevorzugen

```sh
eidos init skyrimse               # eine Instanz anlegen (mit Ordner wird sie portabel)
eidos install skyrimse mod.7z     # Simple- / FOMOD- / BAIN- / Root-Mods
eidos sort skyrimse               # die Ladereihenfolge mit LOOT sortieren
eidos check skyrimse              # die Health-Prüfungen, die kein Fenster brauchen
eidos play skyrimse -- %command%  # irgendetwas durch den zusammengeführten Blick starten
eidos shell skyrimse              # eine Shell im zusammengeführten Blick, Schreibzugriffe landen in Overwrite
```

Jeder Befehl, der eine Spiel-ID annimmt, nimmt auch den Ordner einer portablen
Instanz - siehe [usage.md](usage.md#instanzen-global-und-portabel). Die
vollständige Führung steht in [usage.md](usage.md).

## Aktualisieren

Colony aktualisiert `eidos-linux` über seinen Launcher. Beim Bundle führen Sie das
`install.sh` des neuen Bundles aus; beim Arch-Paket führen Sie `makepkg -si` erneut
aus; bei einem Build aus dem Quelltext holen Sie den neuen Stand und führen
`just install` aus. Das Fenster fragt beim Start höchstens einmal am Tag GitHub
nach dem neuesten Release, und wenn ein neueres erschienen ist, zeigt es nach Help
in der Menüleiste einen Link zu dessen Release Notes. Es lädt sich nie selbst
herunter und ersetzt sich nie. Preferences, About, Version schaltet die Prüfung ab,
der Offline-Modus überspringt sie, und eine Kopie unter `/usr` (das Arch-Paket)
fragt nie, weil der Paketmanager sie aktualisiert.

## Optional: FUSE-Passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` vergibt die optionale
Capability, aktiviert aber kein Passthrough. `./install.sh --cap` und
`just setcap` vergeben sie an das `eidos`, das sie installieren oder bauen. Auf
einem `nosuid`-Mount ignoriert der Kernel Datei-Capabilities, sodass `install.sh`
die Vergabe mit einer Warnung überspringt und `just setcap` mit einem Fehler
abbricht. Die Capability gehört auf die separate `eidos`-Binärdatei des Bundles,
des Arch-Pakets oder eines Builds aus dem Quelltext, niemals auf `eidos-linux`, das
zugleich das Fenster ist ([SECURITY.md](../../../../../SECURITY.md)). Eine neu
gebaute oder neu installierte Binärdatei ist eine neue Datei ohne die Capability;
vergeben Sie sie also nach jedem Upgrade erneut, wenn Sie sie nutzen.

`EIDOS_FUSE_PASSTHROUGH=1` ist der separate Schalter zur Laufzeit, und er ist
standardmäßig aus. Mit eingeschaltetem Passthrough hinderte ein früherer Daemon
Skyrim SE daran, Mod-Inhalte zu laden. Die wahrscheinlichste Ursache ist, dass er
bei jedem Öffnen eine neue Hintergrunddatei registrierte, während der Kernel nur
eine pro Inode akzeptiert. Der Daemon teilt jetzt eine Hintergrunddatei zwischen
allen Passthrough-Öffnungen einer Datei, aber das wurde noch nicht erneut in einem
Spiel gemessen. Bis dahin schalten Sie Passthrough nur zum Testen ein. Die
Messungen und die Ursache stehen in [troubleshooting.md](troubleshooting.md).

## Schon jetzt etwas falsch?

[troubleshooting.md](troubleshooting.md) behandelt die Umgebungsschalter, das
Lesen der Operationszähler, die Health-Karten und jedes Problem, das bisher jemanden
erwischt hat.
