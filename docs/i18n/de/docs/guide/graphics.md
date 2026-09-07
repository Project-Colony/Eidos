<!-- eidos-i18n: source=docs/guide/graphics.md sha=ee3cee732aafbf91d8085d03a74f7d4e0cfa224d -->

# Community Shaders, DLSS und Frame Generation

Community Shaders 1.4+ bringt eigenes Upscaling mit (DLSS 4 / FSR 3.1 / XeSS über
das separate Paket "Upscaling - Community Shaders") sowie FSR-3.1-Frame-Generation.
All das funktioniert unter Linux durch Eidos - CS und seine Pakete installieren
sich als gewöhnliche Mods und die Union liefert ihre DLLs wie alles andere - aber
drei Dinge sind **nicht** aus dem Spiel heraus erkennbar, und jedes davon lässt die
Funktion stillschweigend nichts tun. Diese Seite ist die Liste davon, auf einem
echten System auf die harte Tour gelernt.

## Die Startoption, die DLSS braucht

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton schaltet seine NVIDIA-NVAPI-Schicht (dxvk-nvapi) ab, sofern das Spiel nicht
auf Valves Erlaubnisliste steht - und Skyrim steht nicht darauf. Ohne sie kann CS
DLSS nicht initialisieren und fällt auf FSR-Upscaling zurück - leise, ohne dass
irgendetwas auf dem Bildschirm sagt, warum. Die Variable zu setzen kostet auf
Nicht-NVIDIA-Rechnern nichts, also ist die gefahrlose Startoption schlicht die
Zeile oben. Frame Generation selbst ist FSR 3.1 und braucht kein NVAPI; nur der
DLSS-Upscaler tut es.

## Frame Generation verlangt randloses Fenster

Die Frame Generation von CS läuft über einen D3D12-Präsentations-Proxy und lehnt
exklusiven Vollbildmodus rundheraus ab. `bFull Screen=1` in `SkyrimPrefs.ini`
bedeutet, dass sie nie greift - kein Fehler, keine Meldung, nur die Basisbildrate.
Der belastbare Weg ist SSE Display Tweaks, das den Modus auf Engine-Ebene
erzwingt, was auch immer die INIs sagen:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Das Fenster sieht identisch aus (randlos in nativer Auflösung); nur was die Engine
glaubt, ändert sich - und was die Engine glaubt, ist das, was CS prüft.

Zwei weitere Aktivierungsbedingungen, mit demselben stillen Versagen:

- **Bildwiederholrate 120 Hz oder mehr**, oder setzen Sie
  `frameGenerationForceEnable` in den Upscaling-Einstellungen von CS. Frame
  Generation verdoppelt die dargestellte Rate, also weigert sich CS, sie auf
  Displays scharfzuschalten, die das Ergebnis nicht zeigen können.
- **Das Upscaling-Paket installiert** (sein `Data/Shaders/Upscaling/`-Baum enthält
  die Streamline- und FidelityFX-DLLs). CS ohne es zeigt die Menüeinträge und kann
  nichts aktivieren.

## Das Reflex-Bildratenlimit kann die Ausgabe abwürgen

Die Reflex-Einstellungen von CS führen ein eigenes FPS-Limit (`reflexFPSLimit`,
mit `reflexUseFPSLimit`). Ein Limit, das auf einem früheren Wert stehen geblieben
ist - unseres war 79 aus einem alten Abstimmungsdurchgang -, sitzt hinter der
Frame Generation und schneidet genau die Bilder ab, die sie erzeugt: Basis 60 auf
120 verdoppelt, zurück auf 79 gekappt, liest sich als "Frame Generation tut
nichts". Auf einem 144-Hz-Display liegt das übliche Reflex-Limit bei ~138. Prüfen
Sie es, sobald erzeugte Ausgabe zu fehlen scheint; es ist der zweite stille Killer
nach dem exklusiven Vollbild.

## Bekannte Wechselwirkung: schwarzer Bildschirm mit SSE Display Tweaks

Die Kombination FG + Display Tweaks + DXVK hat einen bekannten Schwarzbildfehler.
Abhilfe, der Reihe nach:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Reicht das nicht, eine `dxvk.conf` neben der Spiel-EXE (das `Root/`-Verzeichnis
   einer Mod legt eine dorthin) mit
   `dxvk.enableGraphicsPipelineLibrary = False`

## Gemischte Textur-Mods: PGPatcher

Community Shaders kann Parallax, Complex Material und PBR darstellen, aber nur
dort, wo ein Mesh dafuer eingerichtet ist. Frueher hiess das: vorgepatchte
Meshes installieren und danach einen Mod, der den Effekt ueberall dort wieder
abschaltet, wo die Texturen fehlten. Die Abdeckung blieb auf die vorhandenen
Meshes beschraenkt, und zur Laufzeit wurde CPU-Zeit dafuer verbraucht, eine
Entscheidung rueckgaengig zu machen, die nie haette fallen duerfen.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) dreht
das um. Sie installieren beliebige Texturen jeden Shader-Typs, und es schreibt
Meshes und Plugins so um, dass jede Oberflaeche den richtigen benutzt -
einschliesslich der Alternate-Texture-Records in Plugins, die die alte Methode
nicht abdeckte. Das ist der Unterschied zwischen einem Textur-Paket, das
dargestellt wird, und einem, das nur geladen wird, und er zaehlt am meisten bei
einem gemischten Setup, also bei jeder echten Ladereihenfolge.

Zwei Dinge vorab unter Linux:

- es braucht das Argument **`--ignore-mo2vfscheck`**, sonst beendet es sich
  sofort mit einer Beschwerde ueber MO2. Eidos liefert es mit - siehe
  [Werkzeuge](tools.md#warum-pgpatcher---ignore-mo2vfscheck-braucht) dazu, was der Schalter ist und warum die
  Pruefung hier fehlschlaegt;
- es veraendert Meshes, laeuft also **nach** allen Textur-Mods und **vor**
  DynDOLOD, das die endgueltigen Meshes sehen muss. Aendern Sie spaeter Ihre
  Texturen, laufen beide erneut, in dieser Reihenfolge.

Seine Ausgabe landet wie bei jedem Werkzeug im Overwrite, wo ein Klick daraus
einen Mod macht. Geben Sie diesem Mod hohe Prioritaet: er soll gewinnen.

## Die Zahlen danach lesen

Erzeugte Bilder betreffen nur die Präsentation: die Engine simuliert weiter mit der
Basisrate, Havok tickt weiter mit der Basisrate, und alles, was *Engine*-Bilder
zählt (die Zähler von CS eingeschlossen), meldet weiter ~60, während das Display
~120 zeigt. Das ist korrektes Verhalten, kein kaputter Zähler - und genau deshalb
ist Frame Generation physiksicher, wo das Anheben der Engine-Bildrate es nicht ist.
`DXVK_HUD=fps` in den Startoptionen zeigt einen Zähler, wenn Sie einen auf dem
Bildschirm wollen.

Eine Regel: Interpolation auf Treiberebene (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) und die Frame Generation von CS sind
konkurrierende Techniken. Nutzen Sie das eine oder das andere, niemals beides.
