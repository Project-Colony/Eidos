<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS en frame generation

Community Shaders 1.4+ levert eigen upscaling mee (DLSS 4 / FSR 3.1 / XeSS, via het
aparte pakket "Upscaling - Community Shaders") en FSR 3.1-frame generation. Dat
alles werkt onder Linux via Eidos - CS en zijn pakketten installeren als gewone mods
en de samenvoeging serveert hun DLL's als al het andere - maar drie dingen zijn
**niet** vanuit het spel te ontdekken, en elk ervan zorgt dat de functie
stilzwijgend niets doet. Deze pagina is die lijst, op een echte opstelling met
schade en schande geleerd.

## De opstartoptie die DLSS nodig heeft

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton schakelt zijn NVIDIA-NVAPI-laag (dxvk-nvapi) uit tenzij het spel op Valves
toelatingslijst staat, en Skyrim staat er niet op. Zonder haar kan CS DLSS niet
initialiseren en valt het terug op FSR-upscaling - stilletjes, zonder dat iets op
het scherm zegt waarom. De variabele zetten kost niets op niet-NVIDIA-machines, dus
de veilige opstartoptie is simpelweg de regel hierboven. Frame generation zelf is
FSR 3.1 en heeft geen NVAPI nodig; alleen de DLSS-upscaler wel.

## Frame generation vereist een randloos venster

De frame generation van CS draait op een D3D12-presentatieproxy en weigert
exclusief volledig scherm ronduit. `bFull Screen=1` in `SkyrimPrefs.ini` betekent
dat ze nooit aangrijpt - geen fout, geen bericht, alleen de basisbeeldsnelheid. De
robuuste oplossing is SSE Display Tweaks, dat de modus op engineniveau afdwingt wat
de INI's ook zeggen:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Het venster ziet er identiek uit (randloos op de eigen resolutie); alleen wat de
engine gelooft verandert - en wat de engine gelooft is wat CS controleert.

Nog twee activeringsvoorwaarden, met hetzelfde stille falen:

- **Verversingssnelheid van 120 Hz of hoger**, of zet `frameGenerationForceEnable`
  in de upscaling-instellingen van CS. Frame generation verdubbelt de gepresenteerde
  snelheid, dus CS weigert haar scherp te stellen op schermen die het resultaat niet
  kunnen tonen.
- **Het Upscaling-pakket geïnstalleerd** (zijn `Data/Shaders/Upscaling/`-boom bevat
  de Streamline- en FidelityFX-DLL's). CS zonder dat toont de menu-items en kan
  niets inschakelen.

## De Reflex-beeldsnelheidslimiet kan de uitvoer wurgen

De Reflex-instellingen van CS dragen een eigen FPS-plafond (`reflexFPSLimit`, met
`reflexUseFPSLimit`). Een plafond dat op een eerdere waarde is blijven staan - het
onze stond op 79 uit een oude afstelronde - zit stroomafwaarts van frame generation
en knipt precies de beelden weg die zij maakt: basis 60 verdubbeld naar 120,
teruggeknipt naar 79, leest als "frame generation doet niets". Op een 144 Hz-scherm
ligt het gebruikelijke Reflex-plafond rond 138. Controleer het telkens wanneer
gegenereerde uitvoer lijkt te ontbreken; het is de tweede stille moordenaar na
exclusief volledig scherm.

## Bekende wisselwerking: zwart scherm met SSE Display Tweaks

De combinatie FG + Display Tweaks + DXVK heeft een bekend zwart-schermprobleem.
Oplossing, op volgorde:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Als dat niet genoeg is, een `dxvk.conf` naast het uitvoerbare bestand van het
   spel (de `Root/`-map van een mod zet er een neer) met
   `dxvk.enableGraphicsPipelineLibrary = False`

## De getallen achteraf lezen

Gegenereerde beelden bestaan alleen aan de presentatiekant: de engine simuleert nog
steeds op de basissnelheid, Havok tikt nog steeds op de basissnelheid, en alles wat
*engine*-beelden telt (de tellers van CS incluis) blijft ~60 melden terwijl het
scherm ~120 toont. Dat is correct gedrag, geen kapotte teller - en het is de reden
dat frame generation veilig is voor de fysica waar het verhogen van de eigen
beeldsnelheid van de engine dat niet is. `DXVK_HUD=fps` in de opstartopties toont
een teller als je er een op het scherm wilt.

Eén regel: interpolatie op stuurprogrammaniveau (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) en de frame generation van CS zijn concurrerende
technieken. Gebruik de een of de ander, nooit allebei.
