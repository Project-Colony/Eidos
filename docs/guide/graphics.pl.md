<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS i generowanie klatek

Community Shaders 1.4+ dostarcza własne skalowanie (DLSS 4 / FSR 3.1 / XeSS, przez
osobny pakiet „Upscaling - Community Shaders") oraz generowanie klatek FSR 3.1.
Wszystko to działa przez Eidos na Linuksie - CS i jego pakiety instalują się jak
zwykłe mody, a unia podaje ich biblioteki DLL jak wszystko inne - ale trzech rzeczy
**nie da się** odkryć od wewnątrz gry, a każda z nich sprawia, że funkcja po cichu
nic nie robi. Ta strona to ich lista, zdobyta boleśnie na prawdziwej instalacji.

## Opcja uruchamiania, której potrzebuje DLSS

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton wyłącza swoją warstwę NVIDIA NVAPI (dxvk-nvapi), o ile gra nie znajduje się
na białej liście Valve - a Skyrim się na niej nie znajduje. Bez niej CS nie
zainicjuje DLSS i cofa się do skalowania FSR - po cichu, bez niczego na ekranie, co
tłumaczyłoby dlaczego. Ustawienie zmiennej nic nie kosztuje na maszynach bez
NVIDII, więc bezpieczną opcją uruchamiania jest po prostu powyższy wiersz. Samo
generowanie klatek to FSR 3.1 i nie potrzebuje NVAPI; potrzebuje go tylko upscaler
DLSS.

## Generowanie klatek wymaga okna bez ramki

Generowanie klatek w CS działa na pośredniku prezentacji D3D12 i wprost odmawia
pełnego ekranu wyłącznego. `bFull Screen=1` w `SkyrimPrefs.ini` oznacza, że nigdy
się nie włączy - bez błędu, bez komunikatu, po prostu bazowa liczba klatek.
Solidnym rozwiązaniem jest SSE Display Tweaks, które wymusza tryb na poziomie
silnika, cokolwiek mówią pliki INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Okno wygląda identycznie (bez ramki, w natywnej rozdzielczości); zmienia się tylko
to, w co wierzy silnik - a to, w co wierzy silnik, jest tym, co sprawdza CS.

Dwa dalsze warunki włączenia, z tą samą cichą awarią:

- **Odświeżanie ekranu 120 Hz lub wyższe**, albo ustaw
  `frameGenerationForceEnable` w ustawieniach skalowania CS. Generowanie klatek
  podwaja prezentowaną częstotliwość, więc CS odmawia uzbrojenia go na ekranach,
  które nie pokażą wyniku.
- **Zainstalowany pakiet Upscaling** (jego drzewo `Data/Shaders/Upscaling/` zawiera
  biblioteki Streamline i FidelityFX). CS bez niego pokazuje pozycje menu i nie
  potrafi niczego włączyć.

## Limit klatek Reflex potrafi zadusić wyjście

Ustawienia Reflex w CS niosą własny limit FPS (`reflexFPSLimit`, wraz z
`reflexUseFPSLimit`). Limit pozostawiony na jakiejś dawnej wartości - u nas było 79
ze starego strojenia - siedzi za generowaniem klatek i ścina dokładnie te klatki,
które ono wytwarza: bazowe 60 podwojone do 120, obcięte z powrotem do 79, czyta się
jako „generowanie klatek nic nie robi". Na ekranie 144 Hz typowy limit Reflex to
około 138. Sprawdzaj go zawsze, gdy wygenerowane wyjście wydaje się nie
istnieć; to drugi cichy zabójca po pełnym ekranie wyłącznym.

## Znana interakcja: czarny ekran z SSE Display Tweaks

Połączenie FG + Display Tweaks + DXVK ma znaną awarię z czarnym ekranem. Naprawa, w
kolejności:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Jeśli to nie wystarczy, `dxvk.conf` obok pliku wykonywalnego gry (katalog
   `Root/` moda umieszcza go tam) z
   `dxvk.enableGraphicsPipelineLibrary = False`

## Odczytywanie liczb potem

Wygenerowane klatki istnieją wyłącznie po stronie prezentacji: silnik nadal
symuluje w tempie bazowym, Havok nadal tyka w tempie bazowym, a wszystko, co liczy
klatki *silnika* (w tym własne liczniki CS), dalej pokazuje ~60, podczas gdy ekran
wyświetla ~120. To zachowanie poprawne, a nie zepsuty licznik - i dlatego właśnie
generowanie klatek jest bezpieczne dla fizyki tam, gdzie podnoszenie własnej
częstotliwości silnika nie jest. `DXVK_HUD=fps` w opcjach uruchamiania pokaże
licznik, jeśli chcesz mieć go na ekranie.

Jedna zasada: interpolacja na poziomie sterownika (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) i generowanie klatek CS to technologie
konkurencyjne. Używaj jednej albo drugiej, nigdy obu.
