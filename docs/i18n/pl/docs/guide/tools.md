<!-- eidos-i18n: source=docs/guide/tools.md sha=88804497cb9e6fcf73c9a60ce56ae6f7e5f0b22b -->

# Narzędzia: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Narzędzie uruchomione przez Eidos widzi **scalony widok**, wewnątrz własnego
prefiksu Proton gry. Czyta to, co przeczyta gra - każdy włączony mod, w
kolejności priorytetów - a cokolwiek zapisze, ląduje w Overwrite, gdzie jedno
kliknięcie zmienia to w prawdziwego moda.

## Te, które Eidos znajduje sam

Niektóre narzędzia mają nazwy na tyle jednoznaczne, że da się je znaleźć zamiast
deklarować, a xEdit jest oczywistym przypadkiem: `FO4Edit.exe` dla Fallouta 4,
`SSEEdit.exe` dla Skyrim SE, `TES5Edit.exe` dla oryginału i tak dalej - wraz z
bliźniakiem **QuickAutoClean** każdego z nich, który jest przyciskiem od brudnych
edycji, przed którymi LOOT wciąż ostrzega. Eidos szuka ich, po nazwie pliku, w:

- folderze instalacyjnym gry oraz w drzewach `Root/` włączonych modów;
- **`mods/` tej instancji**, czyli tam, gdzie użytkownicy MO2 instalują narzędzia;
- **folderze narzędzi** ustawionym w Preferences (General -> Tools), dla
  katalogu współdzielonego między instancjami - `/mnt/Games/Tools` i podobnych.

`PGPatcher.exe` jest znajdowany tak samo w grach na Creation Engine i przychodzi z argumentem, bez ktorego nie zadziala - zobacz [ponizej](#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck).

Lista jest osobna dla każdej gry, więc instancja Skyrima nigdy nie dostanie
edytora Fallouta. Szukanie zatrzymuje się cztery poziomy w głąb, bo pula modów to
setki tysięcy plików, a dzieje się to za każdym razem, gdy budowana jest lista
narzędzi, i nie podąża za dowiązaniami symbolicznymi. Narzędzie znalezione w ten
sposób jest skonfigurowane dokładnie tak samo jak wpisane ręcznie: jego runtime'y
biorą się z nazwy, według tej samej reguły co wszystko poniżej.

Jeśli narzędzie jest gdzie indziej albo chcesz innych argumentów, dodaj je
ręcznie - wpis użytkownika o tym samym tytule nadpisuje cokolwiek znalezionego
automatycznie.

## Czyszczenie pluginu, który LOOT oznacza jako brudny

Po sortowaniu LOOT zakładka Diagnostics wymienia pluginy, które LOOT oznacza
jako brudne (dirty), w grach, które mają zarówno xEdit, jak i dane czyszczenia
LOOT (Skyrim LE, SE i VR, Enderal SE, Fallout 3, New Vegas, Fallout 4 i 4 VR,
Oblivion, Starfield). Gdy Eidos znajdzie xEdit, zarejestrowany w Executables
albo w dowolnym miejscu instancji, w którym szuka narzędzi, karta ma przycisk
Clean all, a każdy brudny plugin ma Clean with QuickAutoClean w menu swojego
wiersza; w przeciwnym razie karta mówi, żeby dodać xEdit w Executables. Eidos
nie pobiera xEdit. Uruchamia tryb QuickAutoClean programu xEdit na jednym
pluginie naraz, przez scalony widok, z `-qac -autoexit -autoload`. xEdit
zapisuje oczyszczony plugin w miejsce tego, który wczytał, co w scalonym widoku
jest nowym plikiem w Overwrite; Eidos przenosi go do moda o nazwie `Cleaned
plugins`, tworzonego jako włączony na dole listy, gdzie wygrywa każdy konflikt.
Plugin w jego własnym modzie ani w folderze Data gry nigdy nie jest zapisywany.
Aby cofnąć czyszczenie, wyłącz `Cleaned plugins` albo ukryj w nim jeden plik.

Kopie zapasowe, cache i log programu xEdit trafiają do
`<instance>/logs/qac-<time>/`, poza scalony widok, a pasek stanu podaje liczby z
tego logu. Eidos usuwa ten folder po udanym czyszczeniu i zostawia go po
nieudanym. Rozmiar i datę pluginu, z którego powstała każda kopia, Eidos
zapisuje w `Cleaned plugins/meta.ini`. Gdy ta wtyczka się zmieni, karta Health zgłasza kopię jako nieaktualną i proponuje jej ukrycie.

## Dodawanie narzędzia

W GUI: **Tools -> Executables**, potem Add. Z wiersza poleceń:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # wypisać, co jest zarejestrowane
eidos tool skyrimse run BodySlide         # uruchomić przez scalony widok
eidos tool skyrimse run BodySlide --print # pokazać polecenie bez uruchamiania
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` zapisuje wynik tego uruchomienia w `mods/<mod>`, niezależnie od własnego ustawienia narzędzia.

Script extender, plik wykonywalny gry i launcher są wykrywane automatycznie;
rejestracji wymagają tylko dodatkowe narzędzia.

### Wskaż prawdziwy plik, gdziekolwiek jest

Zarejestruj plik wykonywalny tam, gdzie faktycznie leży. Jeśli narzędzie zostało
zainstalowane jako mod, to wewnątrz folderu moda:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(to ścieżka instancji globalnej - dla instancji przenośnej ta sama reguła
obowiązuje w jej własnym folderze, `<instance>/mods/...`; zwróć uwagę, że ścieżka
bezwzględna taka jak ta jest jedyną rzeczą, która nie przeżywa PRZENIESIENIA
przenośnego folderu w przyszłości).

Eidos przepisuje tę ścieżkę na scaloną przed uruchomieniem, więc narzędzie działa
z `<game>/Data/CalienteTools/BodySlide/` i widzi tam także pliki wszystkich
innych modów. Znaczy to więcej, niż brzmi: BodySlide dostarcza **pusty** katalog
`SliderSets`, a każde ciało, które potrafi zbudować, pochodzi z CBBE i z modów z
ubiorami. Uruchomiony ze swojego własnego folderu moda nie znajduje nic i wygląda
na zepsuty.

MO2 przepisuje ścieżkę tak samo i z tego samego powodu - jego własny komentarz
wymienia FNIS.

Narzędzia w **wyłączonym** modzie nie da się przepisać, bo jego plików też nie ma
w widoku. Eidos to mówi i uruchamia je z jego własnego folderu, zamiast udawać.

## Narzędzia z listy Mod Organizer 2

Lista Wabbajack albo inna instancja MO2 otwarta jako instancja Eidos zachowuje swój `ModOrganizer.ini`, a skonfigurowane tam pliki wykonywalne można skopiować do `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # pokazuje, co zostałoby dodane
eidos tool /path/to/list import-mo2 --write   # dodaje je
```

W oknie okno Pliki wykonywalne ma przycisk **Importuj z ModOrganizer.ini**, z polem do odznaczenia przy każdym wpisie. Nic nie jest zapisywane, dopóki nie klikniesz Zapisz.

Ścieżki na `Z:` są ścieżkami Linuksa i zostają bez zmian. Ścieżka na innej literze dysku jest przenoszona do folderu listy, jeśli zawiera nazwę tego folderu (`D:\Lists\MyList\tools\SSEEdit.exe` staje się `<list>/tools/SSEEdit.exe`), w pliku wykonywalnym, katalogu roboczym i argumentach. Loader rozszerzenia skryptów i plik gry stają się zwykłymi nazwami, więc uruchamiają się z folderu gry, na który Eidos nakłada widok, tak jak wpisy wbudowane. Wpisy wskazujące gdzie indziej (systemowy program Windows, narzędzie spoza listy) są pokazane jako pominięte, z powodem, a tytuł, który już jest w `tools.ini`, nigdy nie jest zastępowany.

## Kierowanie wyjścia narzędzia do własnego moda

Generator - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - zapisuje setki
plików. Domyślnie lądują w Overwrite razem ze wszystkim innym. Ustaw **Capture
output into** w edytorze Executables, a wyjście tego uruchomienia trafi zamiast
tego do tamtego moda:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Mod jest tworzony, jeśli nie istnieje. Przenoszone są tylko pliki wytworzone
przez TO uruchomienie; cokolwiek już było w Overwrite, tam zostaje, więc dwa
narzędzia z celami przechwytywania nie kradną sobie nawzajem wyjścia.
Uruchomienie, które nic nie zapisało, nie zostawia po sobie pustego moda.

Dzieje się to po uruchomieniu, a nie przez wycelowanie warstwy zapisu w moda, jak
robi to MO2. Wycelowanie warstwy zapisu w moda awansowałoby go na najwyższy
priorytet na czas całego uruchomienia - odwracając każdy konflikt, w którym
bierze udział, i odwracając je z powrotem potem - i zapisywałoby prosto przez
własne pliki moda, bez copy-up. Przechwytywanie dochodzi do tego samego stanu
końcowego bez jednego i drugiego.

Jeśli docelowy mod jest wyłączony, wyjście i tak zostaje zapisane, ale gra go nie
zobaczy, więc narzędzie wygenerowałoby te same pliki przy następnym uruchomieniu.
Eidos ostrzega, gdy tak jest.

### Gdy jego wynik jest nieaktualny

Eidos zapisuje, z jakich modów, kolejności pluginów, profilu i pliku wykonywalnego powstał wynik narzędzia. Gdy któryś z nich się zmieni, mówi o tym karta Diagnostics, a na karcie jest przycisk **Run <tool> again**, który uruchamia narzędzie z obecną konfiguracją.

Dla generatora, którego nieaktualny wynik psuje grę, jak Pandora czy Nemesis, dodaj `check_before_play=1` do jego wpisu w `tools.ini` albo włącz **Ask before playing when its output is out of date** w oknie Executables. Run pyta wtedy najpierw: uruchomić narzędzie czy grać mimo to. Wybór „grać mimo to” obowiązuje, dopóki mody, pluginy lub profil znów się nie zmienią albo okno się nie zamknie. Eidos nigdy nie uruchamia narzędzia samodzielnie.

## O tym, jakich DLL-i potrzebuje narzędzie, decyduje jego NAZWA

To zaskakująca część, więc warto powiedzieć wprost: **tytuł, który nadasz
narzędziu, decyduje o tym, jakie zależności uruchomieniowe Eidos mu przygotuje.**
Dopasowanie to podciąg tytułu, bez rozróżniania wielkości liter.

| Jeśli tytuł zawiera | Eidos żąda |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| cokolwiek innego | nic |

Więc narzędzie zarejestrowane jako **`BodySlide`** dostaje swoje DLL-e DirectX;
ten sam plik wykonywalny zarejestrowany jako **`BS`** nie dostaje nic i może nie
wystartować, z błędem, który nie mówi nic o DLL-ach. Nazywaj narzędzia po
programie.

Lista jest w `default_prereqs` (`crates/eidos-instance/src/tools.rs`), a pole
`Prereqs` w oknie Executables jest edytowalne - wykrywanie to wartość domyślna, a
nie reguła.

Tytul decyduje o **argumentach** tak samo, dla jedynego narzedzia, ktore
potrzebuje takiego, jakiego nikt by nie zgadl: tytul zawierajacy `pgpatcher`
(albo `parallaxgen`, dawna nazwe) dostaje `--ignore-mo2vfscheck`. Dlaczego i
dlaczego nie jest opcjonalny - [ponizej](#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck).

### Trzy rodzaje zależności

**Poziom 1 - dołączone DLL-e** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos je dostarcza i kopiuje do prefiksu przy uruchomieniu. Nic do
zrobienia, żadnej sieci.

**Poziom 2 - czasowniki winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Zapisują klucze rejestru, GAC i hosty CLR, więc nie da
się ich skopiować jako plików. **Pobierają się od Microsoftu**.

**Poziom 3 - runtime'y** (`dotnet10`, `dotnetsdk10`). Nowoczesny runtime .NET to 193 pliki,
które leżą we własnym katalogu i są odnajdywane przez `DOTNET_ROOT`: nigdy
nierejestrowane, nigdy w ogóle nieinstalowane do prefiksu, więc żaden z
pozostałych poziomów nie może go przenieść. Eidos pobiera go sam, sprawdza z sumą
kontrolną wbudowaną w plik wykonywalny i buforuje w
`~/.local/share/Colony/Eidos/runtimes/` - **poza jakąkolwiek instancją**, bo
78 MB nie jest per gra ani per profil.

`dotnetsdk10` to .NET 10 SDK: 301 MB do pobrania, 808 MB po rozpakowaniu.
Synthesis potrzebuje go do kompilowania swoich patcherów, a do ich budowania i
uruchamiania wywołuje gołe `dotnet`, więc narzędzie, które deklaruje
`dotnetsdk10`, dostaje też folder SDK w `WINEPATH`, a do tego
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
i `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Budowanie patchera zostawiłoby działające węzły MSBuild i serwer VBCSCompiler,
które pod Wine utrzymują przy życiu wineserver prefiksu, więc narzędzie dostaje
też `MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` i
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` pozwala patcherowi
zbudowanemu dla .NET 8 (Mutagen przed 0.54) działać na runtime .NET 10 z SDK,
gdy nie zostanie znaleziony żaden runtime .NET 8.
Wpis Synthesis w `tools.ini`, który wciąż ma
starą wartość domyślną `dotnet8`, `vcrun2022`, dostaje `dotnetsdk10`,
`vcrun2022`, gdy Eidos go wczytuje.

NuGet sprawdza podpis każdego przywracanego pakietu względem magazynu
certyfikatów głównych Windows, a w prefiksie Wine brakuje certyfikatów głównych
do podpisywania kodu i znaczników czasu, które SDK dostarcza jako
`sdk/10.0.403/trustedroots/codesignctl.pem` i `timestampctl.pem`. Bez nich
przywracanie `Mutagen.Bethesda.Synthesis` 0.36.6 kończy się błędami `NU3018` i
`NU3028`. Dlatego zanim uruchomi narzędzie, które deklaruje `dotnetsdk10`, i gdy
SDK jest już pobrane, Eidos dodaje 385 certyfikatów z obu plików do magazynu
certyfikatów głównych bieżącego użytkownika w prefiksie gry, w tym samym
imporcie przez `regedit`, który zapisuje ścieżkę gry (patrz niżej). Jeśli
któregoś zabraknie, importuje je ponownie. Od tej chwili każdy program dla
Windows w tym prefiksie ufa tym certyfikatom. Z nimi, w nowym prefiksie i z
pustą pamięcią podręczną NuGet, to samo przywracanie i kompilacja udają się z
systemowym Wine 11.21 i z GE-Proton11-9.

Nic z poziomu 2 ani 3 nie dzieje się po cichu:

```sh
eidos prereqs skyrimse            # pokazać, czego potrzebują zarejestrowane narzędzia, i ich stan
eidos prereqs skyrimse --install  # pobrać to, czego brakuje (pobieranie)
```

W GUI te same stany siedzą pod polem Prereqs, a brakujące są przyciskami.
Czasownik, który nie jest ani dołączony, ani runtime'em, ani znanym czasownikiem
winetricks, jest zgłaszany jako prawdopodobna literówka, a nie oferowany do
pobrania.

### Dlaczego DynDOLOD potrzebuje `dotnet10`

DynDOLOD nie buduje object LOD sam: wywołuje LODGen, a dostarcza trzy jego
wersje. `LODGenx64.exe` celuje w .NET Framework 4.8, który pod Protonem jest
kierowany do Mono z Wine - a jego inicjalizator `System.Uri` wywołuje metodę,
której Mono nie implementuje. Umiera przed pierwszą linijką pracy, zostawiając
log z banerem wersji i niczym więcej oraz okno DynDOLOD-a, które mówi tylko
„failed for one or more worlds".

Zainstalowanie prawdziwego .NET Framework tego nie naprawia: Proton zastępuje
`mscoree.dll` - loader, który by go znalazł - dowiązaniem symbolicznym do
własnego drzewa i robi to ponownie przy każdej aktualizacji prefiksu.

Wersją, która działa, jest `LODGenx64Win10.exe`, celująca w nowoczesny .NET i
nigdy niedotykająca `mscoree`. Wskaż `DOTNET_ROOT` na runtime .NET 10, a
zadziała. To właśnie przygotowuje `dotnet10`, a Eidos ustawia tę zmienną przy
uruchamianiu każdego narzędzia, które ją deklaruje.

Eidos uruchamia systemowy `winetricks` przeciwko własnemu `wine` Protona i
prefiksowi gry, co omija kontener pressure-vessel Steama oraz niezgodność
protontricks + Proton-GE. Narzędzie deklarujące niezainstalowany czasownik
Poziomu 2 i tak się uruchamia, z ostrzeżeniem wymieniającym czasownik i polecenie
naprawcze - użytkownik może go mieć skądinąd.

### Dlaczego PGPatcher potrzebuje `--ignore-mo2vfscheck`

PGPatcher przepisuje siatki i wtyczki tak, aby posiadane mody tekstur -
vanilla, parallax, complex material, PBR - uzywaly wlasciwego shadera na kazdej
powierzchni. Slusznie odmawia pracy poza menedzerem modow: latanie golego
katalogu gry byloby destrukcyjne.

Problem tkwi w SPOSOBIE sprawdzania: szuka **usvfs**, warstwy przechwytywania
DLL, ktora MO2 wstrzykuje do gry w Windows. Eidos jej nie ma i nigdy miec nie
bedzie - scalony widok to montowanie FUSE, w ktore Wine wchodzi, nie wiedzac, ze
istnieje - wiec sprawdzenie nic nie znajduje i narzedzie natychmiast konczy sie
komunikatem

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

wymieniajac program, ktorego uzytkownik wcale nie uruchomil, jeszcze przed
jakakolwiek praca. Odpowiedzia samego autora na dwa zgloszenia z Linuksa
([#716][pg716] i [#725][pg725], oba z Fluorine, linuksowego forka MO2, ktory z
tego samego powodu opiera sie na FUSE) jest flaga pomijajaca sprawdzenie.

Dlatego Eidos ja ustawia. PGPatcher znaleziony w puli modow przychodzi z juz
ustawionym argumentem, a wpisanie `PGPatcher` jako tytulu w oknie Executables
wypelnia pole Arguments tak samo. To, co wpiszesz, zawsze wygrywa: uzupelniane
jest wylacznie pole pozostawione puste, dokladnie jak przy wymaganiach powyzej.

Lista znajduje sie w `default_args` (`crates/eidos-instance/src/tools.rs`), obok
`default_prereqs`. Ma jeden wpis i jeden wystarczy, by uzasadnic funkcje: flagi
nikt nie zgadnie, a blad, ktoremu zapobiega, wymienia niewlasciwy program.

Do dzialania nie wymaga niczego, co rzadkie u narzedzia .NET: niesie wlasne
srodowisko (`dotnetlib/`, z `runtimeconfig.json` deklarujacym
`includedFrameworks`) i dostarcza biblioteki CRT Microsoftu obok pliku
wykonywalnego. Zostaw Prereqs puste.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher i PGLite w trybie Mod Organizer 2

Gdy Mod Manager ma wartość `None`, PGPatcher widzi jeden scalony folder `Data` i
nie wie, z którego moda pochodzi dana siatka lub tekstura, więc nie ma konfliktów
do pokazania ani kolejności modów do zachowania. Tryb `Mod Organizer 2` czyta
samą instancję: plik `modlist.txt` profilu, włączone mody w kolejności
priorytetu oraz pliki w folderze każdego moda. PGLite, wersja działająca w grze,
czyta instancję tak samo. Instancja Eidos ma już układ MO2 (`mods/`,
`profiles/<name>/modlist.txt`, `overwrite/`), więc ten tryb z nią działa.

W launcherze PGPatchera:

1. Ustaw **Mod Manager** na `Mod Organizer 2`.
2. Ustaw **MO2 Options -> Instance Location** na folder instancji Eidos,
   zapisany jako ścieżka Windows na dysku `Z:` Wine, którym jest `/`. Instancja
   przenośna w `/mnt/Games/Eidos-Skyrim` to `Z:\mnt\Games\Eidos-Skyrim`;
   globalna to
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Zostaw `--ignore-mo2vfscheck` w argumentach narzędzia. W tym trybie
   PGPatcher szuka usvfs i przerywa pracę, gdy go nie znajdzie.

PGPatcher przyjmuje folder tylko wtedy, gdy zawiera on `modorganizer.ini`, więc
Eidos zapisuje `ModOrganizer.ini` w katalogu głównym instancji przy każdym
uruchomieniu gry lub narzędzia, dla uruchamianego profilu. Plik jest zapisywany
ponownie tylko wtedy, gdy zmienia się jego zawartość. Zawiera:

- w `[General]`: `gameName` (nazwa gry w MO2, czyli `New Vegas` dla Fallout: New Vegas), `gamePath` (folder instalacji gry jako ścieżkę
  `Z:`) i `selected_profile`;
- w `[Settings]`: `mod_directory`, `profiles_directory` i
  `overwrite_directory` jako bezwzględne ścieżki `Z:`, a także
  `download_directory`, gdy instancja ma folder `downloads/`.

Jego pierwszy wiersz to komentarz mówiący, że zapisał go Eidos. Plik
`ModOrganizer.ini` bez tego wiersza, przy dowolnej wielkości liter w nazwie,
nigdy nie jest zmieniany ani usuwany: instancja MO2 otwarta w Eidos zachowuje
swój plik, a PGPatcher czyta go w niezmienionej postaci. MO2 usuwa komentarze
przy zapisie ustawień, więc plik nadpisany przez MO2 od tej chwili należy do MO2. Pusty plik liczy się jako plik Eidos.

Taki plik wskazuje profil wybrany ostatnio w MO2 i PGPatcher używa tego profilu,
a nie tego, który uruchamia Eidos. Gdy się różnią, Eidos przy uruchomieniu
zapisuje ostrzeżenie w dzienniku. Należy wybrać ten sam profil w MO2 albo
zmienić nazwę pliku lub go usunąć, aby Eidos zapisał własny.

Eidos zapisuje swój plik tylko przy uruchamianiu czegoś, więc między uruchomieniami
wskazuje on profil ostatniego. Narzędzie uruchomione przez Eidos znajduje
w nim swój profil.

PGPatcher kończy pracę z błędem, gdy profil nie ma żadnego włączonego moda, oraz
gdy jego folder wyjściowy jest folderem włączonego moda, tak samo jak pod MO2.

## Ścieżka gry w prefiksie

Narzędzia windowsowe znajdują swoją grę, czytając
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, klucz zapisywany
przez własny instalator gry - którego Steam pod Protonem nigdy nie uruchamia. Bez
niego xEdit, Wrye Bash i DynDOLOD otwierają się na pustej ścieżce. Eidos zapisuje
go przed uruchomieniem narzędzia: idempotentnie, addytywnie i z pominięciem,
jeśli prefiks jest niezainicjalizowany albo w użyciu.

## Dotarcie do narzędzia: ukrywanie, przypinanie i skrót na pulpicie

Domyślne wpisy gry obejmują narzędzia, których możesz nigdy nie użyć, a lista
wyboru wypisująca osiem pozycji, by dojść do drugiej, to lista, której nikt nie
czyta. W oknie Executables:

- **Pin to top** stawia wpis na czele listy Run.
- **Hide from picker** wyjmuje wpis bez usuwania go.
- **Desktop shortcut** zapisuje plik `.desktop` do
  `~/.local/share/applications` - tam, gdzie na systemie freedesktop należy się
  launcher, więc pojawia się w twoim menu aplikacji i w wyszukiwaniu, a nie na
  pulpicie. Uruchamia bezpośrednio `eidos tool <instance> run <title>`, co
  znaczy, że narzędzie wstaje **przez scalony widok, z profilem tej instancji**,
  bez otwierania okna Eidos w ogóle.

Ukrywanie i przypinanie dotyczą tego, jak się do narzędzia *dociera*, a nie tego,
co ono uruchamia, więc stosują się do domyślnych wpisów gry tak samo jak do
twoich własnych.

Loader script extendera, launcher i plik wykonywalny gry nie mają przycisku
Desktop shortcut. Uruchamiają grę, a gra zawsze startuje przez swój sklep, co
robi już skrót samego sklepu. Skrót utworzony dla któregoś z nich w Eidos 1.22
lub starszym dalej działa: podobnie jak `eidos tool <instance> run <title>`
wpisane w terminalu prosi sklep o uruchomienie gry, tak samo jak Run w oknie,
którego nie otworzył żaden sklep. Gdy brakuje ustawienia w sklepie, polecenie
niczego nie uruchamia i wypisuje, co ustawić. Skrót nie pokazuje żadnego
komunikatu, więc uruchom to samo polecenie w terminalu, żeby go przeczytać.

## Narzędzie, które jest własną aplikacją Steam

Creation Kit jest osobną aplikacją Steam i chce własnego AppID; kilka innych
narzędzi moderskich rozprowadzanych przez Steama zachowuje się tak samo. Ustaw
**Steam AppID** we wpisie, a Eidos uruchomi je pod tym id zamiast pod id gry.

Na Windowsie oznacza to inny launcher. Tutaj to dwie zmienne środowiskowe w
uruchomieniu, które i tak było już budowane - `SteamAppId` i `SteamGameId`, obie,
bo Proton czyta jedną, a własne biblioteki Steama drugą, i narzędzie widzące ich
niezgodność zawodzi dziwnie, a nie wyraźnie. `eidos tool ... --print` pokazuje
dokładnie to, co dostałoby prawdziwe uruchomienie.

## Zmienne środowiskowe i narzędzia uruchamiane bez Protona

Trzy klucze w `<instance>/tools.ini` nie mają jeszcze pola w oknie Executables,
więc dopisz je ręcznie w sekcji narzędzia. Każdy wiersz `env=NAME=value` ustawia
jedną zmienną tylko dla uruchomień tego narzędzia:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos stosuje te wiersze po wszystkim, co ustawia sam, więc wartość podana tutaj
zastępuje jego własną. Dotyczy to też `WINEDLLOVERRIDES`, który Eidos składa z
bibliotek DLL dostarczanych przez twoje mody: narzędzie, które go ustawia, musi
wymienić w nim każdą DLL, jakiej potrzebuje. Nazwa składa się z liter, cyfr i
`_` i nie zaczyna się od cyfry. Wiersz z inną nazwą jest pomijany.

`runner=native` uruchamia samo `exe`, bez Protona. Scalony widok jest montowany
jak dla każdego narzędzia, a wiersze `arg0=`, `arg1=` są jego argumentami. Użyj
tego dla programu linuksowego albo dla narzędzia do kopii, którą Steam nie
zarządza i dla której Heroic nie zapisuje żadnego runnera: `exe` to Wine tej
kopii, program dla Windows jest pierwszym argumentem, a prefiks pochodzi
z `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` musi być ścieżką bezwzględną. Katalog roboczy to domyślnie folder, w
którym leży `exe`, tutaj folder runnera, więc ustaw `workdir=`. Na scaloną
ścieżkę przepisywane jest tylko `exe`: dla programu Windows wewnątrz moda podaj
jego ścieżkę w folderze `Data` gry, gdzie pokazuje go scalony widok. Narzędzie
natywne pomija kroki Protona: klucz rejestru opisany wyżej, pamięć podręczną
shaderów Steama, puste katalogi główne .NET i usunięcie nakładki Steama z
`LD_PRELOAD`.

W kopii z GOG lub Epic zainstalowanej przez Heroic narzędzia nie potrzebują
takiego wpisu, jeśli ustawienia gry w Heroic (`GamesConfig/<appName>.json`)
wskazują build Wine lub Protona: Eidos uruchamia je tym buildem w prefiksie
Heroic tej gry, a `eidos prereqs <instance> --install` przekazuje go do
winetricks. Kopia zainstalowana samym Legendary nie ma ustawień Heroic.

`slr=1` uruchamia narzędzie, a także poprzedzający je import do rejestru,
wewnątrz kontenera Steam Linux Runtime, który wskazuje Proton narzędzia
(`require_tool_appid` w jego `toolmanifest.vdf`), tak jak Steam uruchamia grę.
Ten runtime musi być zainstalowany w którejś bibliotece Steam; jeśli nie jest,
uruchomienie się zatrzymuje i podaje aplikację Steam do zainstalowania. `eidos
prereqs` nadal uruchamia winetricks na hoście, a narzędzie z `runner=native`
ignoruje ten klucz. Kontener widzi twój folder domowy i bibliotekę Steam z grą.
Narzędzie, mod lub runtime w innym miejscu, na przykład na innym dysku, wymaga
`env=STEAM_COMPAT_MOUNTS=/ta/ścieżka` (kilka ścieżek oddziel `:`). Proton
zainstalowany w `/usr/share/steam/compatibilitytools.d` prawdopodobnie nie
zadziała z `slr=1`, bo `/usr` w kontenerze należy do runtime'u. Uruchomienie
narzędzia w kontenerze nie było jeszcze testowane.

`eidos tool <instance> run <title> --print` pokazuje końcowe polecenie i każdą
zmienną, którą uruchomienie ustawia ponad odziedziczonym środowiskiem, łącznie
z tymi wierszami.

## Własne ustawienia narzędzia pozostają jego własne

Eidos stawia narzędzie we właściwym miejscu z właściwymi DLL-ami. Co narzędzie
potem robi ze swoją konfiguracją, jest sprawą między tobą a nim, a awaria jest
zwykle cicha.

Przykład rozpisany, bo inaczej kosztuje godzinę: **Game Data Path** BodySlide'a
(Settings) musi wskazywać katalog `Data` gry, a nie folder gry powyżej niego.
Ustawiony o poziom za wysoko sprawia, że batch build zgłasza „All sets processed
successfully" i zapisuje 1439 siatek tam, gdzie gra nigdy ich nie poszuka. Eidos
je łapie - lądują w `Overwrite/Root/`, a nie w twojej instalacji - ale z punktu
widzenia gry nic nie jest nie tak poza tym, że twoje ciała nie są zbudowane.

Wyjście narzędzia należy do Overwrite. Gdy uruchomienie wytworzy coś wartego
zachowania, **Overwrite -> Create mod...** zmienia to w zwykłego moda, którego
można ustawiać w kolejności, wyłączać i usuwać jak każdego innego.
