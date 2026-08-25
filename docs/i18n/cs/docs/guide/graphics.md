<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS a generování snímků

Community Shaders 1.4+ přináší vlastní škálování (DLSS 4 / FSR 3.1 / XeSS, přes
samostatný balíček „Upscaling - Community Shaders") a generování snímků FSR 3.1.
Všechno to funguje přes Eidos na Linuxu - CS i jeho balíčky se instalují jako běžné
módy a sjednocení podává jejich DLL jako cokoli jiného - ale tři věci **nelze**
odhalit zevnitř hry a každá z nich způsobí, že funkce tiše nedělá nic. Tato stránka
je jejich seznam, získaný natvrdo na skutečné sestavě.

## Parametr spuštění, který DLSS potřebuje

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton vypíná svou vrstvu NVIDIA NVAPI (dxvk-nvapi), pokud hra není na seznamu
povolených od Valve - a Skyrim na něm není. Bez ní CS nedokáže inicializovat DLSS a
spadne zpět na škálování FSR - potichu, aniž by cokoli na obrazovce řeklo proč.
Nastavit proměnnou nic nestojí na strojích bez NVIDIE, takže bezpečným parametrem
spuštění je prostě řádek výše. Samotné generování snímků je FSR 3.1 a NVAPI
nepotřebuje; potřebuje ho jen upscaler DLSS.

## Generování snímků vyžaduje okno bez okrajů

Generování snímků v CS běží na prezentačním proxy D3D12 a výhradní celou obrazovku
rovnou odmítá. `bFull Screen=1` v `SkyrimPrefs.ini` znamená, že se nikdy nezapojí -
žádná chyba, žádná zpráva, jen základní snímková frekvence. Robustní řešení je SSE
Display Tweaks, který vynutí režim na úrovni enginu, ať INI říkají cokoli:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Okno vypadá stejně (bez okrajů, v nativním rozlišení); mění se jen to, čemu věří
engine - a čemu věří engine, to CS kontroluje.

Další dvě podmínky zapnutí, se stejným tichým selháním:

- **Obnovovací frekvence 120 Hz nebo vyšší**, nebo nastavte
  `frameGenerationForceEnable` v nastavení škálování CS. Generování snímků zdvojuje
  prezentovanou frekvenci, takže CS odmítá je natáhnout na displejích, které
  výsledek nedokážou zobrazit.
- **Nainstalovaný balíček Upscaling** (jeho strom `Data/Shaders/Upscaling/`
  obsahuje DLL Streamline a FidelityFX). CS bez něj ukáže položky nabídky a nic
  nezapne.

## Limit snímků v Reflexu dokáže výstup uškrtit

Nastavení Reflexu v CS nese vlastní strop FPS (`reflexFPSLimit` spolu s
`reflexUseFPSLimit`). Strop ponechaný na nějaké dřívější hodnotě - u nás 79 ze
starého ladění - sedí za generováním snímků a ořeže přesně ty snímky, které vyrobí:
základních 60 zdvojených na 120, oříznutých zpět na 79, se čte jako „generování
snímků nic nedělá". Na displeji se 144 Hz je obvyklý strop Reflexu ~138. Zkontrolujte
jej pokaždé, když se zdá, že generovaný výstup chybí; je to druhý tichý zabiják po
výhradní celé obrazovce.

## Známá souhra: černá obrazovka se SSE Display Tweaks

Kombinace FG + Display Tweaks + DXVK má známé selhání s černou obrazovkou. Náprava,
popořadě:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Pokud to nestačí, `dxvk.conf` vedle spustitelného souboru hry (adresář `Root/`
   nějakého módu ho tam umístí) s
   `dxvk.enableGraphicsPipelineLibrary = False`

## Jak potom číst čísla

Generované snímky existují jen na straně prezentace: engine dál simuluje základní
frekvencí, Havok dál tiká základní frekvencí, a všechno, co počítá snímky *enginu*
(včetně vlastních čítačů CS), hlásí dál ~60, zatímco displej ukazuje ~120. To je
správné chování, ne rozbitý čítač - a právě proto je generování snímků bezpečné pro
fyziku tam, kde zvyšování vlastní frekvence enginu není. `DXVK_HUD=fps` v
parametrech spuštění zobrazí čítač, pokud jej chcete mít na obrazovce.

Jedno pravidlo: interpolace na úrovni ovladače (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) a generování snímků v CS jsou konkurenční
technologie. Používejte jedno nebo druhé, nikdy obojí.
