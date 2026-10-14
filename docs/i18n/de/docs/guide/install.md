<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Eidos installieren

Vier Wege hinein. Colony installiert eine einzige Datei, `eidos-linux`, die das ganze Programm ist; die anderen liefern `eidos` (die Kommandozeile), `eidos-gui` und das Hilfsprogramm für statische NIF-Vorschauen nebeneinander, dazu den `nxm://`-Handler, durch den der „Mod Manager Download“-Knopf auf Nexus in Ihrer Instanz landet.

## Was Sie vorher brauchen

| | |
|---|---|
| **Linux mit FUSE** | `fusermount3` im PATH. Jede aktuelle Distribution liefert es mit. |
| **Ein Proton-Spiel, einmal gestartet** | Steam legt das Wine-Präfix des Spiels erst beim ersten Start an, und Eidos arbeitet darin. |
| **`7z`** | Zum Installieren von Mod-Archiven. In den meisten Distributionen `p7zip`. |

Kein root, kein Daemon, keine Änderung an `/etc/fuse.conf` und nichts, was Ihren
Gruppen hinzugefügt werden müsste. Eidos mountet in einem privaten Namensraum,
der dem Spielprozess gehört.

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
`eidos-nif-preview`, das nur das Release-Bundle und Builds aus dem Quelltext
mitbringen; ohne es sagt die Vorschau das, und sonst ist nichts betroffen. Lassen
Sie Steam auf die Datei selbst zeigen, wie unten beschrieben.

## Ein Release-Bundle

Laden Sie `eidos-linux-bundle.tar.gz` sowie die zugehörigen `.sig`, `.meta` und
`.meta.sig` aus dem [neuesten Release](https://github.com/Project-Colony/Eidos/releases/latest)
herunter und prüfen Sie sie, wie die
[Richtlinie zur Codesignierung](../../README.md#richtlinie-zur-codesignierung) im
README zeigt. Dann:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Installiert standardmäßig nach `~/.local/bin`. `--system` legt es in
`/usr/local/bin`, `--bindir DIR` irgendwo anders hin. Erneutes Ausführen ist der
vorgesehene Weg zum Aktualisieren. Das Bundle und `eidos-linux` brauchen
x86_64-Linux mit glibc 2.39 oder neuer.

## Arch

```bash
cd packaging && makepkg -si
```

Das PKGBUILD baut den aktuellen Stand von `main` aus dem Quelltext.

## Aus dem Quelltext

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Zum Bauen aus dem Quellcode benötigen Sie Rust 1.89 oder neuer, `just`, CMake 3.20+, einen C++17-Compiler und Python 3 für die Tests des nativen Hilfsprogramms. Befehle ohne `just` finden Sie in der [Anleitung zum nativen Hilfsprogramm](../../../../../crates/eidos-nif-preview/README.md). Bauen Sie Rust mit `cargo build --release --locked`, kopieren Sie das Hilfsprogramm nach `target/release` und führen Sie anschließend `packaging/install.sh --from target/release` aus.

## Dann: Steam darauf zeigen lassen

Eidos läuft *als* Startbefehl Ihres Spiels - so kommt es dazu, den Blick zu
mounten, bevor das Spiel startet. In Steam: Rechtsklick auf das Spiel ->
Eigenschaften -> Startoptionen:

```
~/.local/bin/eidos-gui %command%
```

Aus Colony lautet der Pfad `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Drücken Sie Spielen. Eidos öffnet sich auf der Instanz dieses Spiels; Mods
installieren, mit LOOT sortieren, auf Run klicken. Beim Beenden verschwindet das
Mount mit dem Spiel, und Ihre Installation ist exakt wie zuvor.

Verwenden Sie den absoluten Pfad - Steam liest den `PATH` Ihrer Shell nicht.

### Wenn Sie das Terminal bevorzugen

```sh
eidos init skyrimse               # eine Instanz anlegen (mit Ordner wird sie portabel)
eidos install skyrimse mod.7z     # Simple- / FOMOD- / BAIN- / Root-Mods
eidos sort skyrimse               # die Ladereihenfolge mit LOOT sortieren
eidos play skyrimse -- %command%  # irgendetwas durch den zusammengeführten Blick starten
```

Jeder Befehl, der eine Spiel-ID annimmt, nimmt auch den Ordner einer portablen
Instanz - siehe [usage.md](usage.md#instanzen-global-und-portabel). Die
vollständige Führung steht in [usage.md](usage.md).

## Optional: FUSE-Passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` vergibt die optionale Capability, aktiviert aber kein Passthrough.
Sie gehört auf die separate `eidos`-Binärdatei des Bundles oder eines Builds aus
dem Quelltext, niemals auf `eidos-linux`, das zugleich das Fenster ist
([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` ist der separate Schalter zur Laufzeit. Er ist **standardmäßig aus, und Sie wollen ihn mit
ziemlicher Sicherheit so lassen**: auf Skyrim SE gemessen hindert er das Spiel
daran, seine eigenen Archive und Plugins zu öffnen, sodass Mods stillschweigend
nicht laden. Der Schalter existiert, um den Mechanismus erneut zu testen, nicht
weil er empfohlen wäre.

Einzelheiten und die Messungen hinter dieser Entscheidung in
[troubleshooting.md](troubleshooting.md).

## Schon jetzt etwas falsch?

[troubleshooting.md](troubleshooting.md) behandelt die
Umgebungsschalter, das Lesen der Operationszähler und jedes Problem, das bisher
jemanden erwischt hat.
