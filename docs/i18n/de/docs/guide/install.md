<!-- eidos-i18n: source=docs/guide/install.md sha=c47b7d58351d0b72f6e700bf9acb7e3d78af6a5c -->

# Eidos installieren

Drei Wege hinein. Alle liefern `eidos` (die Kommandozeile), `eidos-gui` und das Hilfsprogramm für statische NIF-Vorschauen sowie den `nxm://`-Handler, durch den der „Mod Manager Download“-Knopf auf Nexus in Ihrer Instanz landet.

## Was Sie vorher brauchen

| | |
|---|---|
| **Linux mit FUSE** | `fusermount3` im PATH. Jede aktuelle Distribution liefert es mit. |
| **Ein Proton-Spiel, einmal gestartet** | Steam legt das Wine-Präfix des Spiels erst beim ersten Start an, und Eidos arbeitet darin. |
| **`7z`** | Zum Installieren von Mod-Archiven. In den meisten Distributionen `p7zip`. |

Kein root, kein Daemon, keine Änderung an `/etc/fuse.conf` und nichts, was Ihren
Gruppen hinzugefügt werden müsste. Eidos mountet in einem privaten Namensraum,
der dem Spielprozess gehört.

## Arch

```bash
cd packaging && makepkg -si
```

## Ein Release-Archiv

```bash
./install.sh
```

Installiert standardmäßig nach `~/.local/bin`. `--system` legt es in
`/usr/local/bin`, `--bindir DIR` irgendwo anders hin. Erneutes Ausführen ist der
vorgesehene Weg zum Aktualisieren.

## Aus dem Quelltext

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Zum Bauen aus dem Quellcode benötigen Sie Rust, `just`, CMake 3.20+, einen C++17-Compiler und Python 3 für die Tests des nativen Hilfsprogramms. Befehle ohne `just` finden Sie in der [Anleitung zum nativen Hilfsprogramm](../../../../../native/eidos-nif-preview/README.md). Bauen Sie Rust mit `cargo build --release --locked`, kopieren Sie das Hilfsprogramm nach `target/release` und führen Sie anschließend `packaging/install.sh --from target/release` aus.

## Dann: Steam darauf zeigen lassen

Eidos läuft *als* Startbefehl Ihres Spiels - so kommt es dazu, den Blick zu
mounten, bevor das Spiel startet. In Steam: Rechtsklick auf das Spiel ->
Eigenschaften -> Startoptionen:

```
~/.local/bin/eidos-gui %command%
```

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
Instanz - siehe [usage.de.md](usage.md). Die vollständige Führung steht dort.

## Optional: FUSE-Passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` vergibt die optionale Capability, aktiviert aber kein Passthrough.
`EIDOS_FUSE_PASSTHROUGH=1` ist der separate Schalter zur Laufzeit. Er ist **standardmäßig aus, und Sie wollen ihn mit
ziemlicher Sicherheit so lassen**: auf Skyrim SE gemessen hindert er das Spiel
daran, seine eigenen Archive und Plugins zu öffnen, sodass Mods stillschweigend
nicht laden. Der Schalter existiert, um den Mechanismus erneut zu testen, nicht
weil er empfohlen wäre.

Einzelheiten und die Messungen hinter dieser Entscheidung in
[troubleshooting.de.md](troubleshooting.md).

## Schon jetzt etwas falsch?

[troubleshooting.de.md](troubleshooting.md) behandelt die
Umgebungsschalter, das Lesen der Operationszähler und jedes Problem, das bisher
jemanden erwischt hat.
