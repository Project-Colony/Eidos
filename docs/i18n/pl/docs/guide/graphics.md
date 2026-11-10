<!-- eidos-i18n: source=docs/guide/graphics.md sha=444791ab81e0f8bdaf91f5afdd30368fb1e166e8 -->

# Community Shaders, DLSS i generowanie klatek

Community Shaders 1.4+ dostarcza własne skalowanie (DLSS 4 / FSR 3.1 / XeSS, przez
osobny pakiet „Upscaling - Community Shaders") oraz generowanie klatek FSR 3.1.
Wszystko to działa przez Eidos na Linuksie - CS i jego pakiety instalują się jak
zwykłe mody, a unia podaje ich biblioteki DLL jak wszystko inne - ale trzech rzeczy
**nie da się** odkryć od wewnątrz gry, a każda z nich sprawia, że funkcja po cichu
nic nie robi. Ta strona to ich lista, zdobyta boleśnie na prawdziwej instalacji.

Karta Stan sprawdza za Ciebie trzy z nich: zmienną NVAPI, gdy jest sterownik NVIDIA i pakiet Upscaling, a build Protona gry domyślnie wyłącza NVAPI (GE-Proton11-9 i Proton 11.0 go włączają), pełny ekran wyłączny w `SkyrimPrefs.ini` profilu oraz opisaną niżej linię ENB. Przy ostatnich dwóch jest przycisk Napraw, a karta Ekran w Diagnostics ustawia okno bez ramki razem z rozmiarem.

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

Aby ustawić ją tylko dla jednego profilu, wpisz `env=PROTON_ENABLE_NVAPI=1` w `launch-env.ini` tego profilu (View -> INI editor, ostatnia karta). Karta Health też liczy ten wiersz.

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

## Mieszane mody tekstur: PGPatcher

Community Shaders potrafi renderowac parallax, complex material i PBR, ale tylko
tam, gdzie siatka jest do tego przygotowana. Kiedys oznaczalo to instalowanie
wczesniej zalatanych siatek, a potem moda wylaczajacego efekt wszedzie tam,
gdzie brakowalo tekstur: pokrycie ograniczone do posiadanych siatek i czas
procesora tracony w trakcie gry na cofanie decyzji, ktorej nigdy nie nalezalo
podjac.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) odwraca
to. Instalujesz dowolne tekstury, dowolnego typu shadera, a on przepisuje siatki
i wtyczki tak, by kazda powierzchnia uzywala wlasciwego - lacznie z rekordami
alternatywnych tekstur we wtyczkach, ktorych stara metoda nie obejmowala. To
roznica miedzy paczka tekstur, ktora widac, a taka, ktora tylko sie wczytuje, i
liczy sie najbardziej przy mieszanej instalacji, czyli przy kazdej prawdziwej
kolejnosci wczytywania.

Dwie rzeczy przed uruchomieniem na Linuksie:

- potrzebuje argumentu **`--ignore-mo2vfscheck`**, inaczej natychmiast konczy
  prace, narzekajac na MO2. Eidos go dostarcza - zobacz
  [Narzedzia](tools.md#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck), czym jest ta flaga i dlaczego sprawdzenie tu
  zawodzi;
- modyfikuje siatki, wiec dziala **po** zainstalowaniu wszystkich modow tekstur
  i **przed** DynDOLOD, ktory musi zobaczyc koncowe siatki. Pozniejsza zmiana
  tekstur oznacza ponowne uruchomienie obu, w tej kolejnosci.

Wynik nie trafia do Overwrite. PGPatcher zapisuje tylko do Output location
ustawionej w swoim launcherze i odrzuca folder wewnątrz `Data`. Skieruj tę
lokalizację do osobnego folderu, na przykład nowego folderu `PGPatcher Output` w
`mods/` instancji. Trzymaj ten mod wyłączony, dopóki PGPatcher działa, bo
PGPatcher opróżnia folder przed ponownym zapisem, a Eidos zakłada, że włączone
mody nie zmieniają się, dopóki widok jest zamontowany. Potem włącz go z wysokim
priorytetem: ma wygrywać.

Nie używaj jako Output location moda wybranego w **Capture output into**. Eidos
po każdym uruchomieniu przenosi do tego moda własne `cfg/` i `log/` PGPatchera,
a PGPatcher zatrzymuje się wtedy z komunikatem
„Output directory has non-PGPatcher related files".

## ENB pod Protonem: LinuxVersion

Jackify zapisuje `LinuxVersion=true` w sekcji `[GLOBAL]` pliku `enblocal.ini` dla każdego ENB, który instaluje pod Protonem. Własna dokumentacja ENB nie opisuje tego klucza. Karta Stan zgłasza `enblocal.ini` bez tej linii, a przycisk Napraw zapisuje w `overwrite/Root/` kopię zwycięskiego `enblocal.ini` z ustawioną linią, więc mod ENB zachowuje swój plik.

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
