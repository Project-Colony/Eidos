<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS und Frame Generation

Community Shaders 1.4+ bringt eigenes Upscaling mit (DLSS 4 / FSR 3.1 / XeSS über
das separate Paket "Upscaling - Community Shaders") sowie FSR-3.1-Frame-Generation.
All das funktioniert unter Linux durch Eidos - CS und seine Pakete installieren
sich als gewöhnliche Mods, und die Union liefert ihre DLLs wie jede andere Datei -,
aber drei Einstellungen sind aus dem Spiel heraus nicht erkennbar, und jede lässt
die Funktion ohne Fehlermeldung nichts tun: die NVAPI-Variable, exklusiver
Vollbildmodus und das Reflex-FPS-Limit.

Der Tab Diagnostics prüft die ersten beiden und die ENB-Zeile weiter unten für ein
Spiel, das ein Wine-Präfix hat (Steams Proton-Präfix oder das, das Heroic
eingerichtet hat). Das Reflex-Limit wird nicht geprüft. Startet die Instanz das
Spiel aus einer Kopie wie `Stock Game`, lesen die Prüfungen die Dateien der Kopie,
nicht die der Store-Installation. `eidos check <instance>` gibt dieselben Karten im
Terminal aus, ohne die Fix-Knöpfe.

- Die NVAPI-Karte erscheint für eine Steam-Kopie von Skyrim SE oder Enderal SE,
  wenn ein NVIDIA-Treiber geladen ist (`/proc/driver/nvidia/version` existiert),
  das zusammengeführte `Data` etwas in `Shaders/Upscaling` hat, der Proton-Build des
  Spiels NVAPI standardmäßig aus lässt und weder Steams Startoption noch die eigene
  Umgebung von Eidos noch die `launch-env.ini` des Profils `PROTON_ENABLE_NVAPI=1`
  setzt.
- Die Karte für exklusiven Vollbildmodus erscheint nur bei installiertem
  Upscaling-Paket, wenn die `SkyrimPrefs.ini` des Profils `bFull Screen=1` hat und
  `SKSE/Plugins/SSEDisplayTweaks.ini` nicht zugleich `Fullscreen=false` und
  `Borderless=true` setzt. Ihr Fix-Knopf schreibt `bFull Screen=0` und
  `bBorderless=1` in die `SkyrimPrefs.ini` des Profils. Die Prüfung liest auch für
  Enderal SE die `SkyrimPrefs.ini`, und ein Enderal-SE-Profil enthält
  `EnderalPrefs.ini`; bei Enderal SE nutzen Sie daher stattdessen die Karte Display
  weiter unten.
- Die ENB-Karte erscheint für jedes Spiel, dessen gewinnende `enblocal.ini` in der
  Root-Sicht kein `LinuxVersion=true` in `[GLOBAL]` hat. Sie hat ebenfalls einen
  Fix-Knopf.

## Die Startoption, die DLSS braucht

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Nehmen Sie den absoluten Pfad Ihres eigenen `eidos-gui` oder `eidos-linux`, wie in
[install.md](install.md). Die Karte nennt die Zeile, die zu setzen ist: die
aktuelle Startoption des Spiels mit `PROTON_ENABLE_NVAPI=1` davor oder, wenn es
keine gibt, die Zeile aus dem Pfad des laufenden Eidos.

Ohne die NVIDIA-NVAPI-Schicht (dxvk-nvapi) kann CS DLSS nicht initialisieren und
fällt auf FSR-Upscaling zurück, ohne dass etwas auf dem Bildschirm sagt, warum. Ob
ein Proton-Build die Schicht von selbst einschaltet, steht in seinem
`proton`-Skript, und Eidos liest dort die Zeile `use_nvapi`. Ein Opt-out-Schalter
(`disablenvapi`) bedeutet standardmäßig an: GE-Proton11-9 und Proton 11.0 haben
diese Form (geprüft am 2026-11-09). Ein Opt-in-Schalter (`enablenvapi`) bedeutet
aus, und nur dieser Build braucht die Variable. Sie zu setzen kostet auf
Nicht-NVIDIA-Rechnern nichts. Frame Generation selbst ist FSR 3.1 und braucht kein
NVAPI; nur der DLSS-Upscaler tut es.

Um sie nur für ein Profil zu setzen, tragen Sie `env=PROTON_ENABLE_NVAPI=1` in die
`launch-env.ini` dieses Profils ein (View -> INI editor, letzter Tab).

## Frame Generation verlangt randloses Fenster

Die Frame Generation von CS läuft über einen D3D12-Präsentations-Proxy und lehnt
exklusiven Vollbildmodus rundheraus ab. Mit `bFull Screen=1` in `SkyrimPrefs.ini`
greift sie nie: kein Fehler, keine Meldung, und das Spiel bleibt bei seiner
Basisbildrate. SSE Display Tweaks setzt den Modus auf Engine-Ebene, was auch immer
die INIs sagen:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Das Fenster sieht identisch aus (randlos in nativer Auflösung). Nur der Modus, den
die Engine meldet, ändert sich, und dieser Modus ist es, was CS prüft.

Die Karte Display in Diagnostics setzt Größe und Modus in einem Schritt, für
Skyrim SE, Enderal SE und Fallout 4. Sie zeigt die Größe des größten angeschlossenen
Bildschirms. Ist Borderless window gewählt, schreibt Apply `iSize W`, `iSize H`,
`bFull Screen=0` und `bBorderless=1` in die `SkyrimPrefs.ini` des Profils
(`EnderalPrefs.ini` bei Enderal SE, `Fallout4Prefs.ini` bei Fallout 4). Setzt SSE
Display Tweaks das Fenster, schreibt Apply außerdem `Resolution`,
`Fullscreen=false` und `Borderless=true` in eine Kopie seiner INI im Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), und die eigene Datei der Mod bleibt, wie sie
ist. Ist Start the game at once when a store opens Eidos an (Preferences, General,
Running a game), öffnet sich das Fenster trotzdem, solange das Profil noch keine
Fenstergröße hat und SSE Display Tweaks das Fenster nicht setzt, damit die Größe
vor dem ersten Start eingestellt werden kann.

Zwei weitere Aktivierungsbedingungen, mit demselben stillen Versagen:

- Die Bildwiederholrate des Displays beträgt 120 Hz oder mehr, oder
  `frameGenerationForceEnable` ist in den Upscaling-Einstellungen von CS gesetzt.
  Frame Generation verdoppelt die dargestellte Rate, also weigert sich CS, sie auf
  Displays scharfzuschalten, die das Ergebnis nicht zeigen können.
- Das Upscaling-Paket ist installiert (sein `Data/Shaders/Upscaling/`-Baum enthält
  die Streamline- und FidelityFX-DLLs). CS ohne es zeigt die Menüeinträge und kann
  nichts aktivieren.

## Das Reflex-Bildratenlimit kann die Ausgabe abwürgen

Die Reflex-Einstellungen von CS führen ein eigenes FPS-Limit (`reflexFPSLimit`,
mit `reflexUseFPSLimit`). Ein Limit, das auf einem früheren Wert stehen geblieben
ist - unseres war 79 aus einem alten Abstimmungsdurchgang -, sitzt hinter der
Frame Generation und schneidet genau die Bilder ab, die sie erzeugt: Basis 60 auf
120 verdoppelt, zurück auf 79 gekappt, liest sich als "Frame Generation tut
nichts". Auf einem 144-Hz-Display liegt das übliche Reflex-Limit bei ~138. Prüfen
Sie es, sobald erzeugte Bilder zu fehlen scheinen.

## Bekannte Wechselwirkung: schwarzer Bildschirm mit SSE Display Tweaks

Die Kombination FG + Display Tweaks + DXVK hat einen bekannten Schwarzbildfehler.
Abhilfe, der Reihe nach:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Reicht das nicht, eine `dxvk.conf` neben der Spiel-EXE (das `Root/`-Verzeichnis
   einer Mod legt eine dorthin) mit
   `dxvk.enableGraphicsPipelineLibrary = False`

## Gemischte Textur-Mods: PGPatcher

Community Shaders kann Parallax, Complex Material und PBR darstellen, aber nur
dort, wo ein Mesh dafür eingerichtet ist. Früher hieß das: vorgepatchte Meshes
installieren und danach eine Mod, die den Effekt überall dort wieder abschaltet,
wo die Texturen fehlten. Die Abdeckung blieb auf die vorhandenen Meshes
beschränkt, und zur Laufzeit wurde CPU-Zeit dafür verbraucht, eine Entscheidung
rückgängig zu machen, die zur Installationszeit getroffen worden war.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) dreht das
um. Sie installieren beliebige Texturen jeden Shader-Typs, und es schreibt Meshes
und Plugins so um, dass jede Oberfläche den richtigen nutzt - einschließlich der
Alternate-Texture-Records in Plugins, die die alte Methode nicht abdeckte. Ohne es
lädt eine Textur, die einen Shader braucht, den ihr Mesh nicht anfordert, wird
aber nicht wie gedacht dargestellt, was am häufigsten bei einem Setup vorkommt, das
Texturtypen mischt.

Zwei Dinge vorab unter Linux:

- es braucht das Argument `--ignore-mo2vfscheck`, sonst beendet es sich sofort mit
  einer Beschwerde über MO2. Eidos liefert es mit; siehe
  [Tools](tools.md#warum-pgpatcher---ignore-mo2vfscheck-braucht) dazu, was der
  Schalter ist und warum die Prüfung hier fehlschlägt;
- es verändert Meshes, läuft also nach allen Textur-Mods und vor DynDOLOD, das die
  endgültigen Meshes sehen muss. Ändern Sie später Ihre Texturen, laufen beide
  erneut, in dieser Reihenfolge.

Seine Ausgabe landet nicht im Overwrite. PGPatcher schreibt nur in die Output
location, die in seinem Launcher eingestellt ist, und lehnt einen Ordner innerhalb
von `Data` ab. Tragen Sie dort einen eigenen Ordner ein, etwa einen neuen Ordner
`PGPatcher Output` unter `mods/` der Instanz. Lassen Sie diese Mod deaktiviert,
solange PGPatcher läuft, denn PGPatcher leert den Ordner, bevor es ihn neu
schreibt, und Eidos erwartet, dass die aktivierten Mods unverändert bleiben,
solange die Ansicht gemountet ist. Aktivieren Sie sie danach mit hoher Priorität:
sie soll gewinnen.

Geben Sie dem Eintrag von PGPatcher keine Mod bei Capture output into und nutzen
Sie diese Mod dann als Output location. Eidos verschiebt nach jedem Lauf
PGPatchers eigene Ordner `cfg/` und `log/` in diese Mod, und PGPatcher bricht dann
mit "Output directory has non-PGPatcher related files" ab.

## ENB unter Proton: LinuxVersion

Jackify schreibt für jedes ENB, das es unter Proton installiert, `LinuxVersion=true`
in den Abschnitt `[GLOBAL]` der `enblocal.ini`. Die eigene Dokumentation von ENB
beschreibt diesen Schlüssel nicht. Eine Health-Karte im Tab Diagnostics meldet eine
`enblocal.ini` ohne diese Zeile, und ihr Fix-Knopf schreibt eine Kopie der
gewinnenden `enblocal.ini` mit gesetzter Zeile nach `overwrite/Root/`, sodass die
ENB-Mod ihre Datei behält.

Eine weitere Karte sagt Bescheid, wenn ein ENB im Spielordner liegt (eine
`enblocal.ini`, eine `enbseries.ini` oder ein Ordner `enbseries/`), während eine
aktivierte Mod Community Shaders mitbringt (`SKSE/Plugins/CommunityShaders.dll`).
Beide können zusammen laufen. Sieht das Bild falsch aus, schalten Sie eines von
beiden in seiner INI ab.

## Die Zahlen danach lesen

Erzeugte Bilder betreffen nur die Präsentation: die Engine simuliert weiter mit der
Basisrate, Havok tickt weiter mit der Basisrate, und alles, was *Engine*-Bilder
zählt (die Zähler von CS eingeschlossen), meldet weiter ~60, während das Display
~120 zeigt. Der Zähler stimmt, und genau deshalb ist Frame Generation physiksicher,
wo das Anheben der Engine-Bildrate es nicht ist.
`DXVK_HUD=fps` in den Startoptionen zeigt einen Zähler, wenn Sie einen auf dem
Bildschirm wollen.

Eine Regel: Interpolation auf Treiberebene (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) und die Frame Generation von CS sind
konkurrierende Techniken. Nutzen Sie das eine oder das andere, niemals beides.
