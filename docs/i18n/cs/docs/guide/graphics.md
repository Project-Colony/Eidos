<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS a generování snímků

Community Shaders 1.4+ přináší vlastní škálování (DLSS 4 / FSR 3.1 / XeSS, přes
samostatný balíček „Upscaling - Community Shaders") a generování snímků FSR 3.1.
Všechno to funguje přes Eidos na Linuxu - CS i jeho balíčky se instalují jako běžné
módy a sjednocení podává jejich DLL jako cokoli jiného - ale tři nastavení nejsou
zevnitř hry vidět a každé z nich způsobí, že funkce bez chyby nedělá nic: proměnná
NVAPI, exkluzivní celá obrazovka a strop FPS v Reflexu.

Záložka Diagnostics kontroluje první dvě a řádek ENB níže u hry, která má prefix
Wine (prefix Proton ze Steamu, nebo ten, který nastavil Heroic). Strop Reflexu se
nekontroluje. Když instance spouští hru z kopie, třeba `Stock Game`, kontroly čtou
soubory kopie, ne instalace z obchodu. `eidos check <instance>` vypíše stejné karty
v terminálu, bez tlačítek Fix.

- Karta NVAPI se objeví u kopie Skyrimu SE nebo Enderalu SE ze Steamu, když je
  načtený ovladač NVIDIA (existuje `/proc/driver/nvidia/version`), sloučený `Data` obsahuje něco v `Shaders/Upscaling`,
  sestavení Protonu hry nechává NVAPI ve výchozím stavu vypnuté a proměnnou
  `PROTON_ENABLE_NVAPI=1` nenastavuje ani parametr spuštění Steamu, ani vlastní
  prostředí Eidosu, ani `launch-env.ini` profilu.
- Karta exkluzivní celé obrazovky se objeví jen s nainstalovaným balíčkem
  Upscaling, když `SkyrimPrefs.ini` profilu má `bFull Screen=1` a
  `SKSE/Plugins/SSEDisplayTweaks.ini` nenastavuje zároveň `Fullscreen=false` a
  `Borderless=true`. Její tlačítko Fix zapíše `bFull Screen=0` a `bBorderless=1`
  do `SkyrimPrefs.ini` profilu. Kontrola čte `SkyrimPrefs.ini` i pro Enderal SE,
  a profil Enderalu SE obsahuje `EnderalPrefs.ini`, takže u Enderalu SE použijte
  místo toho kartu Display níže.
- Karta ENB se objeví u každé hry, jejíž vítězný `enblocal.ini` v kořenovém
  pohledu nemá v `[GLOBAL]` řádek `LinuxVersion=true`. Má také tlačítko Fix.

## Parametr spuštění, který DLSS potřebuje

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Použijte absolutní cestu k vlastnímu `eidos-gui` nebo `eidos-linux`, jako v
[install.md](install.md). Karta uvede řádek, který nastavit: současný parametr
spuštění hry s `PROTON_ENABLE_NVAPI=1` na začátku, nebo, když žádný není, řádek
sestavený z cesty běžícího Eidosu.

Bez vrstvy NVIDIA NVAPI (dxvk-nvapi) CS nedokáže inicializovat DLSS a spadne
zpět na škálování FSR, aniž by cokoli na obrazovce řeklo proč. Zda sestavení Protonu
vrstvu zapíná samo, je zapsáno v jeho skriptu `proton` a Eidos tam čte řádek
`use_nvapi`. Přepínač pro vypnutí (`disablenvapi`) znamená zapnuto ve výchozím
stavu: GE-Proton11-9 a Proton 11.0 mají tuto podobu (ověřeno 2026-11-09).
Přepínač pro zapnutí (`enablenvapi`) znamená vypnuto a jen takové sestavení
proměnnou potřebuje. Nastavit ji nic nestojí na strojích bez NVIDIE. Samotné
generování snímků je FSR 3.1 a NVAPI nepotřebuje; potřebuje ho jen upscaler DLSS.

Chcete-li ji nastavit jen pro jeden profil, vložte `env=PROTON_ENABLE_NVAPI=1` do
`launch-env.ini` tohoto profilu (View -> INI editor, poslední karta).

## Generování snímků vyžaduje okno bez okrajů

Generování snímků v CS běží na prezentačním proxy D3D12 a exkluzivní celou
obrazovku rovnou odmítá. S `bFull Screen=1` v `SkyrimPrefs.ini` se nikdy nezapojí:
žádná chyba, žádná zpráva, hra zůstane na základní snímkové frekvenci. SSE Display
Tweaks nastaví režim na úrovni enginu, ať INI říkají cokoli:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Okno vypadá stejně (bez okrajů, v nativním rozlišení). Mění se jen režim, který
engine hlásí, a ten CS kontroluje.

Karta Display v Diagnostics nastaví velikost a režim jedním krokem, pro Skyrim SE,
Enderal SE a Fallout 4. Ukazuje velikost největší připojené obrazovky. Při vybraném
Borderless window Apply zapíše `iSize W`, `iSize H`, `bFull Screen=0` a
`bBorderless=1` do `SkyrimPrefs.ini` profilu (`EnderalPrefs.ini` u Enderalu SE,
`Fallout4Prefs.ini` u Falloutu 4). Když okno nastavuje SSE Display Tweaks, Apply
zapíše také `Resolution`, `Fullscreen=false` a `Borderless=true` do kopie jeho INI
v Overwrite (`SKSE/Plugins/SSEDisplayTweaks.ini`) a soubor samotného módu zůstane
beze změny. Při zapnutém **Start the game at once when a store opens Eidos**
(Preferences, General, Running a game) se okno přesto otevře, když profil zatím
nemá velikost okna a SSE Display Tweaks okno nenastavuje, takže velikost jde
nastavit před prvním startem.

Další dvě podmínky zapnutí, se stejným tichým selháním:

- Displej se obnovuje na 120 Hz nebo výš, nebo je v nastavení škálování CS
  nastaveno `frameGenerationForceEnable`. Generování snímků zdvojuje prezentovanou
  frekvenci, takže CS odmítá je zapnout na displejích, které výsledek nedokážou
  zobrazit.
- Je nainstalovaný balíček Upscaling (jeho strom `Data/Shaders/Upscaling/`
  obsahuje DLL Streamline a FidelityFX). CS bez něj ukáže položky nabídky a nic
  nezapne.

## Limit snímků v Reflexu dokáže výstup uškrtit

Nastavení Reflexu v CS nese vlastní strop FPS (`reflexFPSLimit` spolu s
`reflexUseFPSLimit`). Strop ponechaný na nějaké dřívější hodnotě - u nás 79 ze
starého ladění - sedí za generováním snímků a ořeže přesně ty snímky, které vyrobí:
základních 60 zdvojených na 120, oříznutých zpět na 79, se čte jako „generování
snímků nic nedělá". Na displeji se 144 Hz je obvyklý strop Reflexu ~138. Zkontrolujte
jej pokaždé, když se zdá, že generované snímky chybí.

## Známá souhra: černá obrazovka se SSE Display Tweaks

Kombinace FG + Display Tweaks + DXVK má známé selhání s černou obrazovkou. Náprava,
popořadě:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Pokud to nestačí, `dxvk.conf` vedle spustitelného souboru hry (adresář `Root/`
   nějakého módu ho tam umístí) s
   `dxvk.enableGraphicsPipelineLibrary = False`

## Smíšené módy textur: PGPatcher

Community Shaders umí vykreslit parallax, complex material i PBR, ale jen tam, kde
je síť pro to připravená. Dříve to znamenalo nainstalovat předem zaplatované sítě
a pak mód, který efekt zase vypne všude, kde textury chyběly: pokrytí omezené na
sítě, které jste měli, a čas procesoru utracený za běhu rušením rozhodnutí
učiněného při instalaci.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) to
obrací. Nainstalujete libovolné textury libovolného typu shaderu a on přepíše sítě
i pluginy tak, aby každý povrch použil ten správný, včetně záznamů alternativních
textur v pluginech, které stará metoda nepokrývala. Bez něj se textura, která
potřebuje shader, o který její síť nežádá, sice načte, ale nevykreslí se tak, jak
má, což se děje nejvíc u sestavy, která míchá typy textur.

Dvě věci před spuštěním na Linuxu:

- potřebuje argument `--ignore-mo2vfscheck`, jinak okamžitě skončí se stížností na
  MO2. Eidos ho dodává - co ten přepínač je a proč tu kontrola selhává, viz
  [Tools](tools.md#proč-pgpatcher-potřebuje---ignore-mo2vfscheck);
- upravuje sítě, takže běží po instalaci všech módů textur a před DynDOLOD, který
  musí vidět konečné sítě. Pozdější změna textur znamená spustit oba znovu, v tomto
  pořadí.

Výstup nemíří do Overwrite. PGPatcher zapisuje jen do složky zadané v poli
Output location jeho spouštěče a složku uvnitř `Data` odmítne. Nasměrujte toto
pole na vlastní složku, třeba na novou složku `PGPatcher Output` v `mods/`
instance. Dokud PGPatcher běží, nechte tento mód vypnutý, protože PGPatcher
složku před novým zápisem vyprázdní a Eidos počítá s tím, že se zapnuté módy
nemění, dokud je pohled připojený. Potom ho zapněte s vysokou prioritou: má
vyhrávat.

Nedávejte záznamu PGPatcheru **Capture output into** a pak nepoužívejte tento mód
jako Output location. Eidos po každém běhu přesune vlastní `cfg/` a `log/`
PGPatcheru do tohoto módu a PGPatcher se pak zastaví s hláškou
„Output directory has non-PGPatcher related files".

## ENB pod Protonem: LinuxVersion

Jackify zapisuje `LinuxVersion=true` do sekce `[GLOBAL]` souboru `enblocal.ini` pro
každé ENB, které pod Protonem instaluje. Vlastní dokumentace ENB tento klíč
nepopisuje. Karta Health v záložce Diagnostics upozorní na `enblocal.ini` bez tohoto
řádku a její tlačítko Fix zapíše kopii vítězného `enblocal.ini` do
`overwrite/Root/` s nastaveným řádkem, takže mód ENB si svůj soubor ponechá.

Jiná karta řekne, když ENB leží ve složce hry (`enblocal.ini`, `enbseries.ini` nebo
složka `enbseries/`) a zároveň zapnutý mód přináší Community Shaders
(`SKSE/Plugins/CommunityShaders.dll`). Oba mohou běžet spolu. Pokud obraz vypadá
špatně, vypněte jeden z nich v jeho INI.

## Jak potom číst čísla

Generované snímky existují jen na straně prezentace: engine dál simuluje základní
frekvencí, Havok dál tiká základní frekvencí, a všechno, co počítá snímky *enginu*
(včetně vlastních čítačů CS), hlásí dál ~60, zatímco displej ukazuje ~120. Čítač
má pravdu a právě proto je generování snímků bezpečné pro fyziku tam, kde zvyšování
vlastní frekvence enginu není. `DXVK_HUD=fps` v parametrech spuštění zobrazí čítač,
pokud jej chcete mít na obrazovce.

Jedno pravidlo: interpolace na úrovni ovladače (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) a generování snímků v CS jsou konkurenční
technologie. Používejte jedno nebo druhé, nikdy obojí.
