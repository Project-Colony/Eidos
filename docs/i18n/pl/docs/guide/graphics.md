<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS i generowanie klatek

Community Shaders 1.4+ dostarcza własne skalowanie (DLSS 4 / FSR 3.1 / XeSS, przez
osobny pakiet „Upscaling - Community Shaders") oraz generowanie klatek FSR 3.1.
Wszystko to działa przez Eidos na Linuksie - CS i jego pakiety instalują się jak
zwykłe mody, a unia podaje ich biblioteki DLL jak każdy inny plik - ale trzech
ustawień nie da się zobaczyć od wewnątrz gry, a każde z nich sprawia, że funkcja
nie działa i nie zgłasza błędu: zmienna NVAPI, pełny ekran wyłączny i limit FPS
Reflex.

Zakładka Diagnostics sprawdza dwa pierwsze oraz linię ENB opisaną niżej, dla gry,
która ma prefiks Wine (prefiks Protona ze Steama albo ten, który założył
Heroic). Limit Reflex nie jest sprawdzany. Gdy instancja uruchamia grę z kopii,
takiej jak `Stock Game`, kontrole czytają pliki kopii, a nie instalacji ze
sklepu. `eidos check <instance>` wypisuje te same karty w terminalu, bez
przycisków Fix.

- Karta NVAPI pojawia się dla kopii Skyrim SE lub Enderal SE ze Steama, gdy
  załadowany jest sterownik NVIDIA (istnieje `/proc/driver/nvidia/version`),
  scalone `Data` ma coś w `Shaders/Upscaling`, build Protona gry domyślnie
  wyłącza NVAPI, a ani opcja uruchamiania Steama, ani własne środowisko Eidosa,
  ani `launch-env.ini` profilu nie ustawia `PROTON_ENABLE_NVAPI=1`.
- Karta pełnego ekranu wyłącznego pojawia się tylko przy zainstalowanym pakiecie
  Upscaling, gdy `SkyrimPrefs.ini` profilu ma `bFull Screen=1`, a
  `SKSE/Plugins/SSEDisplayTweaks.ini` nie ustawia jednocześnie `Fullscreen=false`
  i `Borderless=true`. Jej przycisk Fix zapisuje `bFull Screen=0` i
  `bBorderless=1` w `SkyrimPrefs.ini` profilu. Kontrola czyta `SkyrimPrefs.ini`
  także dla Enderal SE, a profil Enderal SE ma `EnderalPrefs.ini`, więc w
  Enderal SE użyj zamiast tego karty Display opisanej niżej.
- Karta ENB pojawia się dla każdej gry, w której zwycięski `enblocal.ini` w
  widoku root nie ma `LinuxVersion=true` w `[GLOBAL]`. Ma również przycisk Fix.

## Opcja uruchamiania, której potrzebuje DLSS

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Użyj bezwzględnej ścieżki własnego `eidos-gui` lub `eidos-linux`, jak w
[install.md](install.md). Karta podaje wiersz do ustawienia: bieżącą opcję
uruchamiania gry z `PROTON_ENABLE_NVAPI=1` z przodu, a gdy żadnej nie ma, wiersz
zbudowany ze ścieżki działającego Eidosa.

Bez warstwy NVIDIA NVAPI (dxvk-nvapi) CS nie zainicjuje DLSS i cofa się do
skalowania FSR, bez niczego na ekranie, co tłumaczyłoby dlaczego. To, czy build
Protona włącza tę warstwę sam, jest zapisane w jego skrypcie `proton`, a Eidos
czyta tam wiersz `use_nvapi`. Przełącznik wyłączający (`disablenvapi`) oznacza
domyślne włączenie: GE-Proton11-9 i Proton 11.0 mają taką postać (sprawdzone
2026-11-09). Przełącznik włączający (`enablenvapi`) oznacza wyłączenie i tylko
taki build potrzebuje zmiennej. Ustawienie jej nic nie kosztuje na maszynach bez
NVIDII. Samo generowanie klatek to FSR 3.1 i nie potrzebuje NVAPI; potrzebuje go
tylko upscaler DLSS.

Aby ustawić ją tylko dla jednego profilu, wpisz `env=PROTON_ENABLE_NVAPI=1` w
`launch-env.ini` tego profilu (View -> INI editor, ostatnia karta).

## Generowanie klatek wymaga okna bez ramki

Generowanie klatek w CS działa na pośredniku prezentacji D3D12 i wprost odmawia
pełnego ekranu wyłącznego. Przy `bFull Screen=1` w `SkyrimPrefs.ini` nigdy się
nie włączy: bez błędu, bez komunikatu, a gra zostaje przy bazowej liczbie klatek.
SSE Display Tweaks ustawia tryb na poziomie silnika, cokolwiek mówią pliki INI:

```ini
[Render]
Fullscreen=false
Borderless=true
```

Okno wygląda identycznie (bez ramki, w natywnej rozdzielczości). Zmienia się
tylko tryb, który zgłasza silnik, a to właśnie ten tryb sprawdza CS.

Karta Display w Diagnostics ustawia rozmiar i tryb jednym krokiem, dla Skyrim SE,
Enderal SE i Fallouta 4. Pokazuje rozmiar największego podłączonego ekranu. Przy
wybranym Borderless window przycisk Apply zapisuje `iSize W`, `iSize H`,
`bFull Screen=0` i `bBorderless=1` w `SkyrimPrefs.ini` profilu
(`EnderalPrefs.ini` dla Enderal SE, `Fallout4Prefs.ini` dla Fallouta 4). Gdy
okno ustawia SSE Display Tweaks, Apply zapisuje też `Resolution`,
`Fullscreen=false` i `Borderless=true` w kopii jego INI w Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), a własny plik moda zostaje bez zmian. Przy
włączonym **Start the game at once when a store opens Eidos** (Preferences,
General, Running a game) okno nadal się otwiera, gdy profil nie ma jeszcze
rozmiaru okna, a SSE Display Tweaks nie ustawia okna, żeby można było ustawić
rozmiar przed pierwszym startem.

Dwa dalsze warunki włączenia, z tą samą cichą awarią:

- Ekran odświeża się z częstotliwością 120 Hz lub wyższą albo w ustawieniach
  skalowania CS jest ustawione `frameGenerationForceEnable`. Generowanie klatek
  podwaja prezentowaną częstotliwość, więc CS odmawia uzbrojenia go na ekranach,
  które nie pokażą wyniku.
- Pakiet Upscaling jest zainstalowany (jego drzewo `Data/Shaders/Upscaling/`
  zawiera biblioteki Streamline i FidelityFX). CS bez niego pokazuje pozycje menu
  i nie potrafi niczego włączyć.

## Limit klatek Reflex potrafi zadusić wyjście

Ustawienia Reflex w CS niosą własny limit FPS (`reflexFPSLimit`, wraz z
`reflexUseFPSLimit`). Limit pozostawiony na wcześniejszej wartości - u nas było 79
ze starego strojenia - siedzi za generowaniem klatek i ścina dokładnie te klatki,
które ono wytwarza: bazowe 60 podwojone do 120, obcięte z powrotem do 79, czyta się
jako „generowanie klatek nic nie robi". Na ekranie 144 Hz typowy limit Reflex to
około 138. Sprawdzaj go zawsze, gdy wygenerowanych klatek zdaje się brakować.

## Znana interakcja: czarny ekran z SSE Display Tweaks

Połączenie FG + Display Tweaks + DXVK ma znaną awarię z czarnym ekranem. Naprawa, w
kolejności:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. Jeśli to nie wystarczy, `dxvk.conf` obok pliku wykonywalnego gry (katalog
   `Root/` moda umieszcza go tam) z
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mieszane mody tekstur: PGPatcher

Community Shaders potrafi renderować parallax, complex material i PBR, ale tylko
tam, gdzie siatka jest do tego przygotowana. Kiedyś oznaczało to instalowanie
wcześniej załatanych siatek, a potem moda wyłączającego efekt wszędzie tam,
gdzie brakowało tekstur: pokrycie ograniczone do posiadanych siatek i czas
procesora tracony w trakcie gry na cofanie wyboru dokonanego przy instalacji.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) odwraca
to. Instalujesz dowolne tekstury, dowolnego typu shadera, a on przepisuje siatki
i wtyczki tak, by każda powierzchnia używała właściwego - łącznie z rekordami
alternatywnych tekstur we wtyczkach, których stara metoda nie obejmowała. Bez
niego tekstura wymagająca shadera, o który jej siatka nie prosi, wczytuje się,
ale nie renderuje tak, jak miała, co zdarza się najczęściej przy instalacji
mieszającej typy tekstur.

Dwie rzeczy przed uruchomieniem na Linuksie:

- potrzebuje argumentu `--ignore-mo2vfscheck`, inaczej natychmiast kończy
  pracę, narzekając na MO2. Eidos go dostarcza - zobacz
  [Narzędzia](tools.md#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck), czym
  jest ta flaga i dlaczego sprawdzenie tu zawodzi;
- modyfikuje siatki, więc działa po zainstalowaniu wszystkich modów tekstur
  i przed DynDOLOD, który musi zobaczyć końcowe siatki. Późniejsza zmiana
  tekstur oznacza ponowne uruchomienie obu, w tej kolejności.

Wynik nie trafia do Overwrite. PGPatcher zapisuje tylko do Output location
ustawionej w swoim launcherze i odrzuca folder wewnątrz `Data`. Skieruj tę
lokalizację do osobnego folderu, na przykład nowego folderu `PGPatcher Output` w
`mods/` instancji. Trzymaj ten mod wyłączony, dopóki PGPatcher działa, bo
PGPatcher opróżnia folder przed ponownym zapisem, a Eidos zakłada, że włączone
mody nie zmieniają się, dopóki widok jest zamontowany. Potem włącz go z wysokim
priorytetem: ma wygrywać.

Nie dawaj wpisowi PGPatchera moda w **Capture output into**, a potem nie używaj
tego moda jako Output location. Eidos po każdym uruchomieniu przenosi do tego
moda własne `cfg/` i `log/` PGPatchera, a PGPatcher zatrzymuje się wtedy z
komunikatem „Output directory has non-PGPatcher related files".

## ENB pod Protonem: LinuxVersion

Jackify zapisuje `LinuxVersion=true` w sekcji `[GLOBAL]` pliku `enblocal.ini` dla
każdego ENB, który instaluje pod Protonem. Własna dokumentacja ENB nie opisuje
tego klucza. Karta Health w zakładce Diagnostics zgłasza `enblocal.ini` bez tej
linii, a jej przycisk Fix zapisuje w `overwrite/Root/` kopię zwycięskiego
`enblocal.ini` z ustawioną linią, więc mod ENB zachowuje swój plik.

Inna karta mówi, gdy ENB leży w folderze gry (`enblocal.ini`, `enbseries.ini`
albo folder `enbseries/`), a włączony mod dostarcza Community Shaders
(`SKSE/Plugins/CommunityShaders.dll`). Oba mogą działać razem. Jeśli obraz
wygląda źle, wyłącz jeden z nich w jego INI.

## Odczytywanie liczb potem

Wygenerowane klatki istnieją wyłącznie po stronie prezentacji: silnik nadal
symuluje w tempie bazowym, Havok nadal tyka w tempie bazowym, a wszystko, co liczy
klatki *silnika* (w tym własne liczniki CS), dalej pokazuje ~60, podczas gdy ekran
wyświetla ~120. Licznik jest poprawny i dlatego właśnie
generowanie klatek jest bezpieczne dla fizyki tam, gdzie podnoszenie własnej
częstotliwości silnika nie jest. `DXVK_HUD=fps` w opcjach uruchamiania pokaże
licznik, jeśli chcesz mieć go na ekranie.

Jedna zasada: interpolacja na poziomie sterownika (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) i generowanie klatek CS to technologie
konkurencyjne. Używaj jednej albo drugiej, nigdy obu.
