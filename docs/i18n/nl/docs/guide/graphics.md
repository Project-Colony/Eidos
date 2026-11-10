<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS en frame generation

Community Shaders 1.4+ levert eigen upscaling mee (DLSS 4 / FSR 3.1 / XeSS, via het
aparte pakket "Upscaling - Community Shaders") en FSR 3.1-frame generation. Dat
alles werkt onder Linux via Eidos - CS en zijn pakketten installeren als gewone mods
en de samenvoeging serveert hun DLL's als elk ander bestand - maar drie instellingen
zijn niet vanuit het spel te zien, en elk ervan zorgt dat de functie niets doet
zonder een fout te melden: de NVAPI-variabele, exclusief volledig scherm en het
Reflex-FPS-plafond.

Het tabblad Diagnostics controleert de eerste twee, en de ENB-regel verderop, voor
een spel met een Wine-prefix (de Proton-prefix van Steam, of die welke Heroic heeft
ingericht). Het Reflex-plafond wordt niet gecontroleerd. Draait de instantie het
spel vanuit een kopie zoals `Stock Game`, dan lezen de controles de bestanden van
de kopie, niet van de winkelinstallatie. `eidos check <instance>` drukt dezelfde
kaarten af in een terminal, zonder de Fix-knoppen.

- De NVAPI-kaart verschijnt bij een Steam-kopie van Skyrim SE of Enderal SE wanneer
  een NVIDIA-stuurprogramma geladen is (`/proc/driver/nvidia/version` bestaat), de
  samengevoegde `Data` iets in `Shaders/Upscaling` heeft, de Proton-build van het
  spel NVAPI standaard uit laat, en noch Steams opstartoptie, noch de eigen
  omgeving van Eidos, noch de `launch-env.ini` van het profiel
  `PROTON_ENABLE_NVAPI=1` instelt.
- De kaart voor exclusief volledig scherm verschijnt alleen met het Upscaling-pakket
  geïnstalleerd, wanneer de `SkyrimPrefs.ini` van het profiel `bFull Screen=1` heeft
  en `SKSE/Plugins/SSEDisplayTweaks.ini` niet zowel `Fullscreen=false` als
  `Borderless=true` instelt. De Fix-knop schrijft `bFull Screen=0` en
  `bBorderless=1` in de `SkyrimPrefs.ini` van het profiel. De controle leest ook voor
  Enderal SE de `SkyrimPrefs.ini`, terwijl een Enderal SE-profiel `EnderalPrefs.ini`
  bevat, dus gebruik bij Enderal SE de Display-kaart hieronder.
- De ENB-kaart verschijnt bij elk spel waarvan de winnende `enblocal.ini` in de
  rootweergave geen `LinuxVersion=true` in `[GLOBAL]` heeft. Ook die heeft een
  Fix-knop.

## De opstartoptie die DLSS nodig heeft

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Gebruik het absolute pad van je eigen `eidos-gui` of `eidos-linux`, zoals in
[install.md](install.md). De kaart geeft de regel die je moet instellen: de huidige
opstartoptie van het spel met `PROTON_ENABLE_NVAPI=1` ervoor, of, als er geen is, de
regel opgebouwd uit het pad van het draaiende Eidos.

Zonder de NVIDIA-NVAPI-laag (dxvk-nvapi) kan CS DLSS niet initialiseren en valt het
terug op FSR-upscaling, zonder dat iets op het scherm zegt waarom. Of een
Proton-build de laag zelf aanzet staat in zijn `proton`-script, en Eidos leest daar
de regel `use_nvapi`. Een uitschakelschakelaar (`disablenvapi`) betekent standaard
aan: GE-Proton11-9 en Proton 11.0 leveren die vorm (gecontroleerd op 2026-11-09).
Een inschakelschakelaar (`enablenvapi`) betekent uit, en alleen die build heeft de
variabele nodig. De variabele zetten kost niets op niet-NVIDIA-machines. Frame
generation zelf is FSR 3.1 en heeft geen NVAPI nodig; alleen de DLSS-upscaler wel.

Om hem voor één profiel in te stellen, zet je `env=PROTON_ENABLE_NVAPI=1` in de
`launch-env.ini` van dat profiel (View -> INI editor, laatste tabblad).

## Frame generation vereist een randloos venster

De frame generation van CS draait op een D3D12-presentatieproxy en weigert
exclusief volledig scherm ronduit. Met `bFull Screen=1` in `SkyrimPrefs.ini` grijpt
ze nooit aan: geen fout, geen bericht, en het spel blijft op zijn basisbeeldsnelheid.
SSE Display Tweaks stelt de modus op engineniveau in, wat de INI's ook zeggen:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Het venster ziet er identiek uit (randloos op de eigen resolutie). Alleen de modus
die de engine meldt verandert, en die modus is wat CS controleert.

De kaart Display in Diagnostics stelt de grootte en de modus in één keer in, voor
Skyrim SE, Enderal SE en Fallout 4. Ze toont de grootte van het grootste
aangesloten scherm. Met Borderless window geselecteerd schrijft Apply `iSize W`,
`iSize H`, `bFull Screen=0` en `bBorderless=1` in de `SkyrimPrefs.ini` van het
profiel (`EnderalPrefs.ini` voor Enderal SE, `Fallout4Prefs.ini` voor Fallout 4).
Stelt SSE Display Tweaks het venster in, dan schrijft Apply ook `Resolution`,
`Fullscreen=false` en `Borderless=true` in een kopie van zijn INI in de Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), en het eigen bestand van de mod blijft zoals
het is. Staat **Start the game at once when a store opens Eidos** aan (Preferences,
General, Running a game), dan opent het venster toch wanneer het profiel nog geen
venstergrootte heeft en SSE Display Tweaks het venster niet instelt, zodat de grootte
voor de eerste start ingesteld kan worden.

Nog twee activeringsvoorwaarden, met hetzelfde stille falen:

- Het scherm ververst op 120 Hz of hoger, of `frameGenerationForceEnable` is
  ingesteld in de upscaling-instellingen van CS. Frame generation verdubbelt de
  gepresenteerde snelheid, dus CS weigert haar scherp te stellen op schermen die het
  resultaat niet kunnen tonen.
- Het Upscaling-pakket is geïnstalleerd (zijn `Data/Shaders/Upscaling/`-boom bevat
  de Streamline- en FidelityFX-DLL's). CS zonder dat toont de menu-items en kan
  niets inschakelen.

## De Reflex-beeldsnelheidslimiet kan de uitvoer wurgen

De Reflex-instellingen van CS dragen een eigen FPS-plafond (`reflexFPSLimit`, met
`reflexUseFPSLimit`). Een plafond dat op een eerdere waarde is blijven staan - het
onze stond op 79 uit een oude afstelronde - zit stroomafwaarts van frame generation
en knipt precies de beelden weg die zij maakt: basis 60 verdubbeld naar 120,
teruggeknipt naar 79, leest als "frame generation doet niets". Op een 144 Hz-scherm
ligt het gebruikelijke Reflex-plafond rond 138. Controleer het telkens wanneer
gegenereerde beelden lijken te ontbreken.

## Bekende wisselwerking: zwart scherm met SSE Display Tweaks

De combinatie FG + Display Tweaks + DXVK heeft een bekend zwart-schermprobleem.
Oplossing, op volgorde:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Als dat niet genoeg is, een `dxvk.conf` naast het uitvoerbare bestand van het
   spel (de `Root/`-map van een mod zet er een neer) met
   `dxvk.enableGraphicsPipelineLibrary = False`

## Gemengde texture-mods: PGPatcher

Community Shaders kan parallax, complex material en PBR weergeven, maar alleen
waar een mesh daarvoor is ingericht. Vroeger betekende dat: voorgepatchte meshes
installeren en daarna een mod om het effect weer uit te zetten waar de textures
ontbraken. De dekking bleef beperkt tot de meshes die je toevallig had, en er
ging CPU-tijd op aan het terugdraaien van een keuze die bij de installatie was
gemaakt.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) draait
het om. Jij installeert welke textures je wilt, van elk shader-type, en het
herschrijft meshes en plugins zodat elk oppervlak de juiste gebruikt - inclusief
de alternate-texture-records in plugins, die de oude methode niet dekte. Zonder
PGPatcher laadt een texture die een shader nodig heeft die haar mesh niet aanvraagt
wel, maar wordt ze niet weergegeven zoals bedoeld, en dat gebeurt het vaakst bij een
opstelling die texture-typen mengt.

Twee dingen om te weten voor je het onder Linux draait:

- het heeft het argument `--ignore-mo2vfscheck` nodig, anders stopt het
  meteen met een klacht over MO2. Eidos levert het mee - zie
  [Gereedschap](tools.md#waarom-pgpatcher---ignore-mo2vfscheck-nodig-heeft) voor wat de vlag is en waarom de controle
  hier faalt;
- het bewerkt meshes, dus het draait na alle texture-mods en voor
  DynDOLOD, dat de uiteindelijke meshes moet zien. Verander je later je
  textures, dan draaien ze allebei opnieuw, in die volgorde.

De uitvoer belandt niet in de Overwrite. PGPatcher schrijft alleen naar de
Output location die in zijn launcher is ingesteld, en weigert een map binnen
`Data`. Laat die locatie naar een eigen map wijzen, bijvoorbeeld een nieuwe map
`PGPatcher Output` onder `mods/` van de instantie. Houd die mod uitgeschakeld
zolang PGPatcher draait, want PGPatcher maakt de map leeg voordat het hem
opnieuw schrijft en Eidos verwacht dat de ingeschakelde mods niet veranderen
zolang de weergave gemount is. Schakel hem daarna in met een hoge prioriteit:
hij hoort te winnen.

Geef het item van PGPatcher geen mod bij **Capture output into** om die mod daarna
als Output location te gebruiken. Eidos verplaatst na elke draai de eigen `cfg/` en `log/` van PGPatcher naar die mod,
en PGPatcher stopt dan met "Output directory has non-PGPatcher related files".

## ENB onder Proton: LinuxVersion

Jackify schrijft `LinuxVersion=true` in de sectie `[GLOBAL]` van `enblocal.ini` voor
elke ENB die het onder Proton installeert. De eigen documentatie van ENB beschrijft
deze sleutel niet. Een Health-kaart in het tabblad Diagnostics meldt een
`enblocal.ini` zonder die regel, en de Fix-knop schrijft een kopie van de winnende
`enblocal.ini` met de regel erin naar `overwrite/Root/`, zodat de ENB-mod zijn eigen
bestand houdt.

Een andere kaart zegt wanneer er een ENB in de spelmap staat (een `enblocal.ini`, een
`enbseries.ini` of een map `enbseries/`) terwijl een ingeschakelde mod Community
Shaders levert (`SKSE/Plugins/CommunityShaders.dll`). De twee kunnen samen draaien.
Ziet het beeld er verkeerd uit, zet dan een van beide uit in zijn INI.

## De getallen achteraf lezen

Gegenereerde beelden bestaan alleen aan de presentatiekant: de engine simuleert nog
steeds op de basissnelheid, Havok tikt nog steeds op de basissnelheid, en alles wat
*engine*-beelden telt (de tellers van CS incluis) blijft ~60 melden terwijl het
scherm ~120 toont. De teller klopt, en dit is de reden dat frame generation veilig is
voor de fysica waar het verhogen van de eigen beeldsnelheid van de engine dat niet
is. `DXVK_HUD=fps` in de opstartopties toont
een teller als je er een op het scherm wilt.

Eén regel: interpolatie op stuurprogrammaniveau (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) en de frame generation van CS zijn concurrerende
technieken. Gebruik de een of de ander, nooit allebei.
