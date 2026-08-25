<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Narzędzia: xEdit, BodySlide, DynDOLOD, FNIS

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
- **folderze narzędzi** ustawionym w Ustawieniach (Tools -> Tools folder), dla
  katalogu współdzielonego między instancjami - `/mnt/Games/Tools` i podobnych.

Lista jest osobna dla każdej gry, więc instancja Skyrima nigdy nie dostanie
edytora Fallouta. Szukanie zatrzymuje się cztery poziomy w głąb, bo pula modów to
setki tysięcy plików, a dzieje się to za każdym razem, gdy budowana jest lista
narzędzi, i nie podąża za dowiązaniami symbolicznymi. Narzędzie znalezione w ten
sposób jest skonfigurowane dokładnie tak samo jak wpisane ręcznie: jego runtime'y
biorą się z nazwy, według tej samej reguły co wszystko poniżej.

Jeśli narzędzie jest gdzie indziej albo chcesz innych argumentów, dodaj je
ręcznie - wpis użytkownika o tym samym tytule nadpisuje cokolwiek znalezionego
automatycznie.

## Dodawanie narzędzia

W GUI: **Tools -> Executables**, potem Add. Z wiersza poleceń:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # wypisać, co jest zarejestrowane
eidos tool skyrimse run BodySlide         # uruchomić przez scalony widok
eidos tool skyrimse run BodySlide --print # pokazać polecenie bez uruchamiania
```

Script extender, plik wykonywalny gry i launcher są wykrywane automatycznie;
rejestracji wymagają tylko dodatkowe narzędzia.

### Wskaż prawdziwy plik, gdziekolwiek jest

Zarejestruj plik wykonywalny tam, gdzie faktycznie leży. Jeśli narzędzie zostało
zainstalowane jako mod, to wewnątrz folderu moda:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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

## O tym, jakich DLL-i potrzebuje narzędzie, decyduje jego NAZWA

To zaskakująca część, więc warto powiedzieć wprost: **tytuł, który nadasz
narzędziu, decyduje o tym, jakie zależności uruchomieniowe Eidos mu przygotuje.**
Dopasowanie to podciąg tytułu, bez rozróżniania wielkości liter.

| Jeśli tytuł zawiera | Eidos żąda |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
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

### Trzy rodzaje zależności

**Poziom 1 - dołączone DLL-e** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
je dostarcza i kopiuje do prefiksu przy uruchomieniu. Nic do zrobienia, żadnej
sieci.

**Poziom 2 - czasowniki winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Zapisują klucze rejestru, GAC i hosty CLR, więc nie da
się ich skopiować jako plików. **Pobierają się od Microsoftu**.

**Poziom 3 - runtime'y** (`dotnet10`). Nowoczesny runtime .NET to 193 pliki,
które leżą we własnym katalogu i są odnajdywane przez `DOTNET_ROOT`: nigdy
nierejestrowane, nigdy w ogóle nieinstalowane do prefiksu, więc żaden z
pozostałych poziomów nie może go przenieść. Eidos pobiera go sam, sprawdza z sumą
kontrolną wbudowaną w plik wykonywalny i buforuje w
`~/.local/share/Colony/Eidos/runtimes/` - **poza jakąkolwiek instancją**, bo
78 MB nie jest per gra ani per profil.

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

## Narzędzie, które jest własną aplikacją Steam

Creation Kit jest osobną aplikacją Steam i chce własnego AppID; kilka innych
narzędzi moderskich rozprowadzanych przez Steama zachowuje się tak samo. Ustaw
**Steam AppID** we wpisie, a Eidos uruchomi je pod tym id zamiast pod id gry.

Na Windowsie oznacza to inny launcher. Tutaj to dwie zmienne środowiskowe w
uruchomieniu, które i tak było już budowane - `SteamAppId` i `SteamGameId`, obie,
bo Proton czyta jedną, a własne biblioteki Steama drugą, i narzędzie widzące ich
niezgodność zawodzi dziwnie, a nie wyraźnie. `eidos tool ... --print` pokazuje
dokładnie to, co dostałoby prawdziwe uruchomienie.

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
