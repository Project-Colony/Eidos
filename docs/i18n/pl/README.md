<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Natywny linuksowy menedżer modów, który nigdy nie dotyka folderu twojej gry.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalacja)

Eidos daje grom Bethesdy pod Linuksem to, co Mod Organizer 2 daje im pod
Windowsem: scalony widok twoich modów, budowany przy każdym uruchomieniu. Zamiast
przechwytywania API Windows używa linuksowych przestrzeni nazw montowań i FUSE, a
sam menedżer działa bez Wine. Nic nie jest kopiowane do folderu gry, więc potem
nie ma czego czyścić. Eidos ma też definicje dla Stellar Blade, Stardew Valley,
7 Days to Die i Crimson Desert.

Kopie ze Steama uruchamiają Eidos z opcji uruchamiania gry. Kopie GOG i Epic w
Heroic lub Legendary uruchamiają go jako wrapper. Gra zawsze startuje przez
sklep, do którego należy kopia; Eidos nigdy nie uruchamia jej sam przez Proton.

```
Steam ──> eidos-gui %command% ──> [ prywatna przestrzeń nazw ]
                                  │  mody ⊕ gra  ──> to, co widzi gra
                                  └─ ginie razem z grą; instalacja zostaje nietknięta
```

> **Stan:** w Skyrim SE gra się przez Eidos codziennie, z SKSE, preloaderami
> script extendera, Creation Club, kolejnościami wczytywania posortowanymi
> LOOT-em i zapisami na profil. Jak dotąd jedna rodzina gier jest sprawdzona w
> realnej grze. Dziesięć kolejnych gier Bethesdy i cztery inne są podłączone i
> czekają na testerów.

## Dlaczego Eidos

Scalony widok jest montowany w prywatnej przestrzeni nazw montowań, którą widzi
tylko drzewo procesów gry. Inne programy, także druga gra, nigdy go nie widzą i
nie potrzebują do niego uprawnień. Gdy gra się kończy lub zostaje ubita albo
maszynie zabraknie prądu, przestrzeń nazw ginie razem ze swoimi procesami, a
instalacja jest taka, jaka była.

Każdy profil ma własną listę modów, kolejność wtyczek, pliki INI i zapisy. Przy
uruchomieniu pliki wtyczek i folder zapisów są montowane przez bind na ścieżkach
samej gry, więc to, co gra tam zapisuje, ląduje w profilu. Zmiana profilu zmienia
to wszystko.

Eidos nie potrzebuje pomocnika setuid, demona systemowego, `sudo setcap` ani
edycji `/etc/fuse.conf`. Demon FUSE działa jako twój użytkownik przez jedno
uruchomienie i kończy się razem z grą. Instalujesz jeden program i ustawiasz
jedną opcję uruchamiania Steama.

Przed każdym uruchomieniem Eidos zachowuje migawkę listy wtyczek. Jeśli awaria
uszkodzi listę, Diagnostics porównuje ją z tą migawką, a Restore the pre-session
order przywraca starą kolejność. Przechwycenie, które wymazałoby twoją kolejność
wczytywania, jest odrzucane, z podaniem powodu.

Test run uruchamia grę z modami profilu, bez modów albo tylko z zaznaczonymi
modami, które są włączone, plus ich masterami. Zapisy, zmiany INI, stan wtyczek, wynik w Overwrite i
inne pliki My Games z tego uruchomienia, razem z logami, trafiają do
`<instance>/test-run/`. Gdy gra się kończy, Eidos wypisuje te pięć grup, a ty
zachowujesz lub wyrzucasz każdą z nich. Inne zapisy, na przykład rejestr Wine,
idą tam, gdzie zawsze. Find the mod behind a problem w Diagnostics uruchamia
testowe uruchomienia z połową podejrzanych modów naraz plus potrzebnymi im
masterami. Po każdym uruchomieniu odpowiadasz, czy problem wystąpił, a Eidos
zawęża podejrzanych, aż wskaże moda albo mody, które potrzebują nawzajem swoich
masterów; 200 podejrzanych to około 8 uruchomień. Włączone mody zaznaczone na liście przed
startem zostają włączone w każdym kroku. Szczegóły w
[Uruchomienia testowe](docs/guide/usage.md#uruchomienia-testowe) i
[Szukanie moda stojącego za problemem](docs/guide/usage.md#szukanie-moda-stojącego-za-problemem).

### Jak wypada na tle innych

| | Eidos | MO2 przez Wine | Fluorine-Manager | Limo / deployery dowiązań |
|---|---|---|---|---|
| Menedżer działa natywnie | tak | nie, aplikacja Windows w Wine | tak (port Qt) | tak |
| Folder gry nietknięty | tak, zawsze | tak | tak | nie, zapisywane są w nim dowiązania |
| Montowanie widoczne dla | tylko gry | tylko gry | całego systemu | nie dotyczy |
| Sprzątanie po awarii | żadne | żadne | odzyskiwanie martwego montowania | ręczne wycofanie |
| Mody root (ENB, preloadery) | tak, natywnie | wymagana wtyczka | wymagana wtyczka | częściowo |
| Wymagane uprawnienia | żadne | żadne | edycja `/etc/fuse.conf` | żadne |

### Jak szybko działa

| | przedtem | teraz |
|---|---|---|
| wczytanie zapisu | ~20 sekund | 6-7 sekund |
| skanowania folderów w jednej sesji | 5,6 miliona | 335 tysięcy |

Zmiany komórek są natychmiastowe. Znalezienie jednego pliku odpytywało wcześniej
po kolei każdy z pięćdziesięciu modów, a wypisanie jednego folderu robiło to
pięćdziesiąt razy. Warstwy tylko do odczytu są teraz indeksowane raz przy
montowaniu, więc znalezienie pliku to jedno wyszukanie w tablicy haszującej, a
scalony spis folderu też jest liczony raz przy montowaniu. Liczby pochodzą z
prawdziwej instancji granej normalnie, nie z benchmarku:
[internals/performance.md](../../internals/performance.md) (po angielsku).

### Wcześniejsze prace i podziękowania

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) i
  [usvfs](https://github.com/ModOrganizer2/usvfs): semantyka, którą Eidos
  odtwarza, i baza kodu, wobec której badano jego zgodność
- [LOOT](https://loot.github.io/): silnik sortowania, przez libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) i inne linuksowe menedżery, które
  pokazują, że jest społeczność chcąca to rozwiązać

## Co robi

### Mody

Eidos instaluje proste archiwa, kreatory FOMOD, paczki BAIN Wrye Basha,
kontenery OMOD z obsługiwanymi skryptami OBMM oraz wszystko inne przez ręczny
wybór. Mody root (preloadery script extendera, ENB, Engine Fixes) działają
natywnie, bez wtyczki Root Builder i bez kopiowania czegokolwiek do twojej
instalacji. Możesz ukrywać pojedyncze pliki, grupować mody separatorami,
przenosić zaznaczenie do celu, trzymać notatki i kategorie dla każdego moda oraz
importować profil MO2.

Lista działa jak ta z MO2: osiem opcjonalnych kolumn i sortowanie po każdej z
nich, grupowanie według kategorii lub źródła, gesty podwójnego kliknięcia, skok
do nazwy przez pisanie, kopie zapasowe pojedynczych modów, które nic nie robią,
dopóki ich nie przywrócisz, oraz flagi dla moda, którego układ ta gra nie
wczyta albo który został pobrany do innej gry. Filtry wybierają mody według
stanu na Nexusie (polubiony, śledzony, dostępna aktualizacja), notatek, ukrytych
plików i rodzaju zawartości (wtyczki, archiwa, tekstury, siatki, skrypty, dźwięk
i inne). Drzewo plików tworzy foldery, zmienia nazwy, usuwa i otwiera pliki oraz
podgląda obrazy i teksty, nic nie uruchamiając.

Ctrl+Z i Ctrl+Shift+Z cofają i ponawiają zmiany listy modów i kolejności
wczytywania. Wyłączenie lub usunięcie moda, którego wtyczek używa jeden z trzech
najnowszych zapisów profilu, najpierw pyta i wymienia wtyczki (zapisy Skyrim i
Fallouta 4). Ponowna instalacja lub aktualizacja moda FOMOD otwiera kreator z
odpowiedziami z jego ostatniej instalacji. Informacje o modzie wymieniają mody,
których wymaga, i mody, które wymagają jego, odczytane z masterów włączonych
wtyczek.

### Wtyczki

Kolejność wczytywania ma wbudowane sortowanie LOOT-em, indeksy modów takie, jakie
wylicza gra, ostrzeżenia o brakujących masterach oraz twoje DLC i zawartość
Creation Club pokazane jako niezarządzane wiersze. Nagłówek liczy zajęte sloty w
każdej przestrzeni indeksów (full, medium, light). Najechanie na wtyczkę pokazuje
jej autora, opis i wersję z nagłówka. W silnikach Skyrim oznaczona jest wtyczka,
której INI o tej samej nazwie gra wczytuje.

Wtyczki, które LOOT oznacza jako brudne, są czyszczone przez QuickAutoClean z
xEdit do moda `Cleaned plugins`, nigdy w miejscu. Flag as light, into a new mod
zapisuje lekką kopię wtyczki i odmawia, gdy jeden z najnowszych zapisów wczytuje
tę wtyczkę jako pełną (zapisy Skyrim i Fallouta 4).

### Instancje

Instancja jest globalna, trzymana pod `~/.local/share/Colony/Eidos/instances/`,
albo przenośna: samodzielny folder gdziekolwiek chcesz (drugi dysk, partycja z
grami), przenoszalny i odizolowany jak w MO2. Instancje przenośne są pamiętane
między sesjami. GUI, uruchomienie ze Steama i każde polecenie wiersza poleceń
podążają za ostatnio używaną, a każde polecenie przyjmuje folder wszędzie tam,
gdzie przyjmuje identyfikator gry. Szczegóły w
[usage.md](docs/guide/usage.md#instancje-globalne-i-przenośne).

Instancja może też uruchamiać grę z kopii folderu gry wewnątrz instancji. Gdy
Eidos po raz pierwszy otwiera listę Wabbajack, uruchamia jej własną kopię:
folder wskazany w jej `ModOrganizer.ini`, a w przeciwnym razie folder `Stock
Game` lub `Game Root`. Freeze this version, na karcie Game files w Diagnostics,
kopiuje instalację ze sklepu do `<instance>/Stock Game`, więc aktualizacja
sklepu nie zmienia już plików, z których działa gra. Gdy oba foldery leżą na
jednym systemie plików, który potrafi współdzielić bloki, takim jak btrfs lub
XFS, kopia dzieli je z instalacją ze sklepu (reflink). Na ext4 albo między
dwoma systemami plików to pełna kopia, a rozmiar i wolne miejsce są pokazywane
wcześniej. Grę nadal uruchamia sklep. Return to the store version i Delete cofają
to, a `eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
robi to samo w terminalu. Morrowind oraz Oblivion z `bUseMyGamesDirectory=0` nie
mogą działać z kopii. Szczegóły w
[usage.md](docs/guide/usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia).

### Profile

Każdy profil ma własną kolejność modów, stan wtyczek, pliki INI i zapisy. Zapisy
są parsowane i porównywane z twoimi obecnymi wtyczkami, z przyciskiem
włączającym to, czego zapis potrzebuje, i kopiowane z powrotem do prefiksu dla
Steam Cloud po każdej sesji. Zapisy, które Steam Cloud przyniósł z innego
urządzenia albo które uruchomienie bez Eidosa zostawiło w prefiksie, są
wymienione na karcie Saves. Import kopiuje je do profilu, a zapis, którego
nazwy profil już używa, trafia pod nową nazwą z `orphan-` zamiast go zastąpić.

Profil może nieść zmienne uruchomieniowe i wrappery (`env=NAME=value` i
`wrapper=command`, po jednym w wierszu, w edytorze INI) dla uruchomień swojej
gry i narzędzi. Dla Skyrim SE, Enderal SE i Fallouta 4 karta Display w
Diagnostics ustawia rozmiar i tryb okna w kopiach INI profilu. Backups trzyma
punkty przywracania INI profilu, a Eidos robi jeden przed każdym uruchomieniem
BethINI Pie. Skrót pulpitu profilu uruchamia grę przez jej sklep. `eidos
sessions` wypisuje zapisane uruchomienia i porównuje jedno z bieżącym stanem.

### Nexus

Połącz konto, aby odbierać odnośniki Mod Manager Download, sprawdzać
aktualizacje oraz przeglądać autorów i ich profile. Każdy mod jest sprawdzany,
polubiany i linkowany w domenie Nexusa, z której pochodzi jego plik, więc mod
Skyrim SE w instancji Enderal SE zostaje na stronie Skyrim SE. Aktualizacja
podąża za łańcuchem następców ustawionym przez autora na Nexusie, zastępuje
folder moda nowym plikiem (z ofertą kopii zapasowej starego), zachowuje jego
priorytet i stan włączenia oraz odtwarza odpowiedzi FOMOD zapisane przy
instalacji. `eidos nexus update <instance> --apply` robi to samo w terminalu na
koncie premium, a na darmowym wypisuje strony plików. Track i Endorse są w menu
moda, a każde sprawdzenie aktualizacji synchronizuje je z twoim kontem Nexus.
Informacje o modzie mają kartę Nexus page z jego opisem i listą zmian jako zwykły
tekst, a jej karta General kończy się tym, co zmieniła ostatnia aktualizacja:
pliki dodane, usunięte i zmienione rozmiarem oraz mastery dodane lub usunięte.
Install downloads when they finish (Preferences, Nexus, domyślnie wyłączone)
instaluje pobranie, gdy tylko się skończy, dopóki okno jest otwarte.

Kolekcje są instalowane według swoich przepisów, z możliwością wznowienia
postępu i wyraźnymi raportami o nieobsługiwanych krokach. Dopasowanie sprawdza
dokładne pliki źródłowe; niepełne starsze metadane pozostają niezweryfikowane.
Panel kolekcji przegląda i przeszukuje kolekcje opublikowane dla gry, pokazuje,
gdy zainstalowana kolekcja ma nowszą rewizję, i prowadzi darmowe konto przez
brakujące pliki, po jednej stronie naraz. Gdy przeciągnięcie łamie reguły
kolejności kolekcji, Diagnostics to mówi, a Restore collection order przywraca
jej mody na miejsce. Karta Downloads filtruje, sortuje, ukrywa i usuwa archiwa.
Tryb Offline (Preferences, Nexus) zatrzymuje dostęp do Nexusa i codzienne
sprawdzanie wydań.

### Narzędzia

xEdit, BodySlide, DynDOLOD i inne narzędzia do modowania działają przez scalony
widok wewnątrz prefiksu Wine gry (prefiksu Protona ze Steama albo tego, który
założył Heroic). Widzą twoje mody, ich wynik ląduje w Overwrite, a jedno
kliknięcie zamienia ten wynik w prawdziwego moda. Dla Skyrim SE Eidos znajduje
SSEEdit, SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide,
DynDOLOD, TexGen, xLODGen, Synthesis i BethINI Pie w folderze gry, wewnątrz moda
albo w folderze narzędzi trzymanym obok gier, każde z potrzebnymi runtime'ami.
Pozostałe gry mają własną listę. Pandora, Nemesis, FNIS, BodySlide i Synthesis
zapisują swój wynik do nazwanego moda (`Nemesis Output` i tak dalej).

Pakiet wydania zawiera instalatory Visual C++ 2015-2022 i .NET 8 Desktop Runtime
Microsoftu oraz runtime .NET 10. Eidos instaluje `vcrun2022` i .NET 8 tymi
instalatorami, bez winetricks, a bez pakietu pobiera te same pliki od Microsoftu,
każdy sprawdzony względem SHA-256 wbudowanego w Eidosa. Synthesis dostaje SDK
.NET 10, pobranie o rozmiarze 300 MB, którego pakiet nie zawiera. Pozostałe
wymagania (.NET Framework 4.8, XACT i starsze wersje .NET) nadal idą przez
winetricks. Nic nie jest pobierane, dopóki nie poprosisz o instalację wymagań
narzędzia.

Przypnij narzędzia, których używasz, ukryj te, których nie, nadaj narzędziu
własny Steam AppID, gdy jest osobną aplikacją Steama, i zapisz skrót `.desktop`,
który uruchamia je przez scalony widok, bez otwierania Eidosa. `eidos tool <game>
import-mo2` wypisuje pliki wykonywalne z `ModOrganizer.ini` folderu listy, a
`--write` je dodaje. Plik exe na karcie Data można uruchomić jednorazowo albo
dodać jako narzędzie, a `eidos tool <instance> run --exe <path>` uruchamia go z
terminala. Przy Ask before playing Run pyta najpierw, gdy wynik generatora jest
nieaktualny.

Stop kończy działającą grę lub narzędzie. Menu File uruchamia winecfg, regedit,
eksplorator plików Wine i deinstalator Windows w prefiksie gry. Rebuild prefix w
Diagnostics zmienia nazwę prefiksu, żeby Steam założył nowy, i niczego nie
usuwa. `eidos shell <instance>` otwiera powłokę w scalonym widoku albo uruchamia
polecenie podane po `--`, a to, co zapisuje, ląduje w Overwrite.

Gdy okno jest w tle, Eidos może wysłać powiadomienie pulpitu, gdy kończy się
uruchomienie narzędzia, instalacja kolekcji, pakowanie lub rozpakowanie albo
ostatnie działające pobieranie, oraz gdy sortowanie LOOT-em się nie powiedzie.
Notify the desktop when a long job ends, w Preferences, to włącza; domyślnie jest
wyłączone.

### Diagnostics

Zakładka Diagnostics wymienia brakujące mastery, osierocone archiwa, dryf listy
modów i uszkodzone zestawy wtyczek, a po uruchomieniu to, co według własnego
logu script extendera zostało wczytane. Zgłasza też:

- script extender, którego brakuje albo który nie pasuje do wersji gry
- grę, która zmieniła się od ostatniego czystego uruchomienia
- opcję uruchamiania Steama, która nie uruchamia Eidosa albo nie ma po nim
  `%command%`
- Proton starszy od tego, który ostatnio skonfigurował prefiks, albo pochodzący
  z Flatpak Steam
- brak `PROTON_ENABLE_NVAPI` dla DLSS oraz pełny ekran wyłączny, który
  zatrzymuje generowanie klatek Community Shaders
- `enblocal.ini` bez `LinuxVersion=true`
- log awarii z ostatniej sesji i mody, które wymienia
- wymagania z Nexusa, których brakuje albo które są wyłączone
- obsługę `nxm://`, którą jest inny program albo brakujący plik
- wiersze `sTestFile`, przez które gra ignoruje `plugins.txt`
- włączone wtyczki `.esp` w Starfield oraz nieprawidłowe lub niesparowane
  wtyczki blueprint
- łatkę, której luźne pliki przegrywają z modem, który łata
- mod, którego kopie skryptów Papyrus script extendera wygrywają z własnymi
  skryptami extendera
- niski `vm.max_map_count` lub limit otwartych plików
- pliki gry na systemie plików Windows
- ścieżki danych BodySlide, które nie są folderem Data gry, oraz foldery w
  `Overwrite/Root`, które należą do Data
- `ModOrganizer.ini` listy, który kieruje narzędzia na inny folder gry
- kopię gry, której brakuje albo która nie ma pliku wykonywalnego gry

`eidos check <instance> [--json]` uruchamia kontrole, które nie potrzebują okna,
i kończy się kodem 1, gdy znajdzie problem. Copy report kopiuje raport z twoim
folderem domowym zapisanym jako `~` i nazwą użytkownika jako `<user>`, a Save
tree na karcie Data zapisuje scalone drzewo Data ze źródłem każdego pliku,
zredagowane tak samo.

[Kontrole integralności i wygenerowane pliki](../../internals/integrity.md) (po angielsku) opisuje ochronę przy zastępowaniu modów, konflikty plików w BSA/BA2, statyczną diagnostykę SKSE i zapisy generowania wraz z ich ograniczeniami.

### Gdzie trzyma własne pliki

Eidos używa układu, którego używa każdy program z rodziny Colony:
`~/.config/Colony/Eidos/` na to, co wybrałeś (preferencje, twoja sesja Nexusa,
twoja lista instancji, napisane przez ciebie definicje gier i dodatków),
`~/.local/share/Colony/Eidos/` na instancje globalne, runtime'y narzędzi
(pobrane albo skopiowane z pakietu przez `install.sh`) i logi sesji, a
`~/.cache/Colony/Eidos/` na masterlisty LOOT. Wewnątrz instancji `Stock Game`
trzyma zamrożoną kopię gry, a `test-run/` trzyma test run czekający na przegląd.

Starszy Eidos trzymał pliki w `~/.config/eidos/`, `~/.local/share/eidos/` i
`~/.local/state/`. Pierwsze uruchomienie po aktualizacji przenosi je, mówi o tym
w logu i zostawia stare foldery dokładnie takimi, jakie były. Dopóki przenosiny
się nie zakończą, Eidos dalej używa starych plików. Szczegóły w
[internals/paths.md](../../internals/paths.md) (po angielsku).

### Obsługiwane gry

Skyrim SE/AE jest sprawdzony w realnej grze. Fallout 4 też jest podłączony od
początku do końca (F4SE podstawiany automatycznie, unieważnianie archiwów,
kolejność wczytywania z gwiazdką, LOOT, zapisy `.fos`); zob.
[guide/fallout4.md](docs/guide/fallout4.md). Podłączone i szukające testerów:
Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV,
Starfield, Oblivion i Morrowind. Fallout 3, Fallout NV, Oblivion i Morrowind
ustalają kolejność wtyczek według dat plików: Eidos trzyma jedną kolejność na
profil i pokazuje te daty wewnątrz montowania, nie zmieniając plików na dysku.

Cztery inne gry są podłączone i też szukają testerów: Stellar Blade (mody Unreal
`.pak` w `~mods`), Stardew Valley (mody SMAPI, z osobno zainstalowanym SMAPI),
7 Days to Die (foldery `Mods/` obok gry) i Crimson Desert. Crimson Desert
przyjmuje tylko mody z kodem: wtyczki ASI, ReShade i biblioteki loadera w
`bin64`. Jego mody danych, które żyją w grupach PAZ, nie są obsługiwane.

Gra to jeden plik TOML, a twoje możesz umieścić w `~/.config/Colony/Eidos/games/`
bez przebudowy: [internals/adding-games.md](../../internals/adding-games.md)
(po angielsku).

## Instalacja

### Przez Colony (zalecane)

Wyszukaj **Eidos** w [Colony](https://github.com/Project-Colony/Colony) i
zainstaluj go. Aktualizacje przychodzą przez launcher. Colony instaluje jeden
plik, `eidos-linux`: okno, a zarazem wiersz poleceń `eidos`, gdy jest wywołany
przez dowiązanie o nazwie `eidos`. Ten plik nie zawiera ani programu
pomocniczego podglądu modeli NIF, ani runtime'ów Microsoftu z opisanego niżej
pakietu. Bez programu pomocniczego podgląd to zgłasza, a wszystko inne działa, a
Eidos pobiera runtime od Microsoftu, gdy instalujesz wymagania narzędzia. Co
zrobić po instalacji i jak skierować na niego Steama:
[docs/guide/install.md](docs/guide/install.md#przez-colony).

### Bezpośrednie pobranie pliku wykonywalnego

Pobierz plik z [najnowszego wydania](https://github.com/Project-Colony/Eidos/releases/latest):

| Plik | Co to jest |
|---|---|
| `eidos-linux` | cały program w jednym pliku, tak jak instaluje go Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, program pomocniczy podglądu NIF, instalatory Visual C++ 2015-2022 i .NET 8 Desktop Runtime oraz runtime .NET 10, ikony, noty licencyjne i instalator, który rejestruje wpis pulpitu i obsługę pobierania z Nexusa |

Najpierw sprawdź pobrany plik, jak pokazuje
[Polityka podpisywania kodu](#polityka-podpisywania-kodu), a potem:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Instalator umieszcza pliki wykonywalne w `~/.local/bin` bez sudo i kopiuje
`runtimes/` z pakietu do `~/.local/share/Colony/Eidos/runtimes`. `--system` i
`--bindir DIR` umieszczają pliki wykonywalne gdzie indziej, a `--from DIR` bierze
je z innego folderu. `--cap` prosi sudo o `CAP_SYS_ADMIN`, którego potrzebuje
tylko eksperymentalny passthrough FUSE, a passthrough zostaje wyłączony;
`--no-cap` jest domyślne. Oba pliki wymagają Linuksa x86_64 z glibc 2.39 lub
nowszą, FUSE (`fusermount3`) i 7-Zip do archiwów modów.

Eidos nie aktualizuje się sam. Raz dziennie przy starcie okno pyta GitHub o
najnowsze wydanie, a gdy jest nowsze, pokazuje link do jego notatek wydania po
Help na pasku menu. Preferences to wyłącza, a kopia w `/usr` poza `/usr/local`,
którą aktualizuje menedżer pakietów, nigdy nie pyta.

### Budowanie ze źródeł

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Wymaga Rusta 1.92 lub nowszego (`rust-version` sprawdzane przez CI; CI i
wydania budują wersją 1.99.1), a także `just`, CMake 3.20+ i kompilatora C++17
dla programu pomocniczego podglądu NIF. `just build` używa profilu release: jedna
jednostka generowania kodu i pełne LTO. Zimna kompilacja `eidos` i `eidos-gui`
przy 3 zadaniach budowania trwała tak 334 sekundy, a z cienkim LTO 243 sekundy,
a pełne LTO zmniejsza `eidos` o 1,2 MB i `eidos-gui` o 3,8 MB (pomiary w
`Cargo.toml`). `just profile=dev build` kompiluje szybciej. Pakiet Arch buduje
się z `packaging/PKGBUILD`. Wymagania wstępne i polecenia bez `just`:
[docs/guide/install.md](docs/guide/install.md).

### Opcje uruchamiania Steam

Eidos działa *jako* polecenie uruchamiające grę i dzięki temu montuje się,
zanim gra wystartuje. Podstawowa linia wystarcza większości konfiguracji, z
bezwzględną ścieżką twojej instalacji (Steam nie czyta twojej zmiennej `PATH`):

```
~/.local/bin/eidos-gui %command%
```

Przy instalacji z Colony ta ścieżka to
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Następnie naciśnij Graj: Eidos
otwiera się na instancji tej gry, a Run uruchamia grę przez scalony widok.

W oknie otwartym przez ciebie Run prosi Steama o uruchomienie gry
(`steam -applaunch <appid>`), więc opcja uruchamiania nadal obowiązuje, a ty
naciskasz Run w oknie, które otwiera Steam. Heroic dostaje odnośnik
`heroic://launch`, a Legendary uruchamia `legendary launch <app>`, gdy Eidos jest
tam ustawiony jako wrapper gry. Gdy brakuje ustawienia sklepu, Run niczego nie
uruchamia i mówi, co ustawić:
[guide/usage.md](docs/guide/usage.md#uruchamianie-gry). Przy Start the game at
once when a store opens Eidos (Preferences, domyślnie wyłączone) Eidos uruchamia
kontrole Diagnostics i startuje grę bez pokazywania okna, gdy nic nie wymaga
uwagi.

Cała reszta to zmienne środowiskowe ustawiane przed nią i łączą się dowolnie:

| Chcesz... | Wstaw przed |
|---|---|
| DLSS z Community Shaders | `PROTON_ENABLE_NVAPI=1`. Bez niej DLSS się nie inicjalizuje i nic o tym nie mówi; pełna lista kontrolna jest w [guide/graphics.md](docs/guide/graphics.md) |
| licznik FPS na ekranie | `DXVK_HUD=fps` |
| interpolację klatek na poziomie sterownika, zero modów (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, nigdy razem z własną generacją klatek Community Shaders |
| szczegółowe logi do zgłoszenia błędu | `EIDOS_LOG=debug` (logi sesji lądują w `~/.local/share/Colony/Eidos/logs/`) |
| raport we/wy z montowania dla każdej sesji | `EIDOS_FUSE_STATS=1` |
| inną liczbę wątków roboczych FUSE | `EIDOS_FUSE_THREADS=8` (domyślnie 4; `1` to pierwsza rzecz do wypróbowania przy tropieniu błędu współbieżności) |
| przypiąć to uruchomienie do jednej instancji przenośnej | `EIDOS_INSTANCE=/path/to/folder`. Bez niej Eidos otwiera ostatnio używaną instancję |
| okno na jedno uruchomienie przy włączonym Start the game at once | `EIDOS_WINDOW=1` |

Linia do zachowania przy nowoczesnej moddowanej konfiguracji (Community Shaders,
DLSS, generacja klatek) to polecenie ostateczne, nie przykład:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Dodaj z przodu `DXVK_HUD=fps` na czas sprawdzania, czy konfiguracja działa, i
usuń go, gdy już działa.

Te same zmienne mogą trafić do zmiennych uruchomieniowych profilu zamiast do
linii Steama, co trzyma je per profil i stosuje też do uruchomień narzędzi:
[guide/usage.md](docs/guide/usage.md#zmienne-i-wrappery-per-profil).

Głębsze przełączniki diagnostyczne (`EIDOS_FUSE_TRACE`, przełączniki bisekcji
cache'u i indeksu, dlaczego `EIDOS_FUSE_PASSTHROUGH` jest domyślnie wyłączony)
są w [guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Dokumentacja

Wszystko jest w [docs/](docs/), z indeksem ułożonym według tego, kto czyta, w
[docs/README.md](docs/README.md): przewodnik użytkownika (instalacja, wiersz
poleceń i GUI, narzędzia takie jak xEdit i DynDOLOD, grafika, rozwiązywanie
problemów), jak Eidos działa w środku i po co istnieje. Polityka
bezpieczeństwa i sposób zgłaszania podatności:
[SECURITY.md](../../../SECURITY.md).

## Język

Strony, których potrzebuje gracz, są przetłumaczone. Angielski jest wersją
wzorcową: gdy tłumaczenie się z nim nie zgadza, rację ma plik angielski.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Cała reszta jest po angielsku. `docs/internals/` i `docs/project/` czytają ludzie,
którzy czytają też Rusta, `CHANGELOG.md` jest generowany, a `SECURITY.md` też nie
jest tłumaczony. Tłumaczenie dwóch pierwszych dodałoby ponad 28 000 słów do
utrzymania w zgodzie z prawdą dla czytelników, którzy ich nie potrzebują.

Każde tłumaczenie niesie skrót pliku angielskiego, z którego powstało, a CI
zawodzi, dopóki ten skrót nie zgadza się z angielskim; zob.
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Tłumaczenie jest
wtedy aktualizowane i ponownie stemplowane przez `scripts/i18n-check.sh --fix`.
Nieaktualne tłumaczenie jest aktualizowane, a nie usuwane.

Język to jeden katalog. `docs/i18n/<lang>/` odwzorowuje korzeń repozytorium,
więc `docs/i18n/de/docs/guide/install.md` to niemiecki `docs/guide/install.md`.
Dlatego odnośnik między dwiema przetłumaczonymi stronami jest tym samym ciągiem
co odnośnik między ich angielskimi oryginałami, a wycofanie języka to jedno
`rm -r`.

## Polityka podpisywania kodu

Każdy plik wydania jest podpisywany przez organizację Project Colony w CI,
nigdy na komputerze programisty. Obok każdego pliku na stronie wydania:

| Plik | Co to jest |
|---|---|
| `<asset>.sig` | podpis ed25519 pliku, złożony kluczem wydań organizacji |
| `<asset>.meta` | trzy linie wiążące plik z jego nazwą, jego sha256 i wersją wydania |
| `<asset>.meta.sig` | podpis ed25519 pliku `.meta` |

Klucz prywatny jest sekretem organizacji, używanym wyłącznie przez wspólny
[workflow sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
w zadaniu, które niczego nie buduje; zadania kompilujące Eidosa nigdy go nie
widzą. Colony sprawdza wszystkie trzy pliki, zanim zainstaluje lub zaktualizuje
Eidosa, i odrzuca wydanie starsze niż zainstalowane. Aby samodzielnie sprawdzić
pobrany plik za pomocą OpenSSL 3 (te same polecenia działają dla każdego
pliku):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Jak wydania są budowane, podpisywane i publikowane:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)
(po angielsku).

## Prywatność

Eidos nie wysyła żadnej telemetrii, żadnych danych analitycznych ani raportów
o awariach. Łączy się tylko z serwerami z tej tabeli i tylko w celu, który tam
podano. Tryb Offline (Preferences, Nexus) zatrzymuje każde połączenie z Nexusem i
codzienne sprawdzanie wydań.

| Dane | Przechowywane czy wysyłane | Gdzie i dlaczego |
|---|---|---|
| Preferencje, lista instancji, definicje gier i dodatków | przechowywane | `~/.config/Colony/Eidos/` |
| Instancje, runtime'y narzędzi, logi sesji | przechowywane | `~/.local/share/Colony/Eidos/` albo folder wybrany dla instancji przenośnej |
| Masterlisty LOOT | przechowywane | `~/.cache/Colony/Eidos/loot/` |
| Sprawdzanie wydań | wysyłane | Przy starcie, najwyżej raz dziennie, jedno żądanie do `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos czyta tylko tag wydania i niczego nie pobiera. Jest domyślnie włączone i wyłączane w Preferences, a pomijane w trybie Offline i dla kopii zainstalowanej w `/usr` poza `/usr/local`, którą aktualizuje menedżer pakietów. [Oświadczenie o prywatności GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Logowanie do Nexusa | wysyłane | Logowanie otwiera `users.nexusmods.com` w twojej przeglądarce, która przekazuje wynik z powrotem do nasłuchu, jaki Eidos otwiera na `127.0.0.1` tylko na czas tego logowania. Eidos wymienia go następnie na tokeny pod `users.nexusmods.com/oauth/token` i pobiera klucze publiczne, którymi Nexus je podpisuje. [Polityka prywatności Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Dane uwierzytelniające Nexusa | przechowywane | Tokeny dostępu i odświeżania, w `~/.config/Colony/Eidos/nexus.ini` z uprawnieniami 0600. Linia `api_key=` zapisana tam przez starszego Eidosa jest zachowywana, ale nigdy nieczytana; możesz ją usunąć. |
| Żądania do Nexusa | wysyłane | `api.nexusmods.com`, gdy używasz funkcji Nexusa: nazwa twojego konta i status premium, szczegóły modów i plików, sprawdzanie aktualizacji, kolekcje i odnośniki pobierania. Twoje polubienia i śledzone mody są czytane przy każdym sprawdzeniu aktualizacji i zapisywane, gdy naciskasz Endorse lub Track. Wymagania modów, opisy i listy zmian oraz wyszukiwanie kolekcji idą do `api.nexusmods.com/v2/graphql`. Każde żądanie niesie twój token dostępu i podaje Eidosa oraz jego wersję. |
| Pobrania z Nexusa | wysyłane | Pliki, o które prosisz, z hosta CDN Nexusa wskazanego w odnośniku pobierania, zapisywane w folderze pobrań instancji z plikiem `.meta` w formacie MO2 obok każdego z nich. |
| Odnośniki `nxm://` | przechowywane | `eidos nxm --register`, które `install.sh` uruchamia za ciebie, zapisuje `~/.local/share/applications/eidos-nxm.desktop` i ustawia go jako obsługę `nxm://` przez `xdg-mime`, więc przycisk Mod Manager Download na stronie przekazuje swój odnośnik Eidosowi. Pakiet Arch instaluje ten sam wpis dla całego systemu. |
| Masterlisty LOOT | wysyłane | `raw.githubusercontent.com/loot/...`, przy pierwszym sortowaniu LOOT danej gry i za każdym razem, gdy prosisz o aktualizację. Nic twojego nie jest wysyłane. [Oświadczenie o prywatności GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtime'y narzędzi | wysyłane | Tylko gdy instalujesz wymagania narzędzia i tylko dla tego, czego pakiet wydania nie dostarczył (pojedynczy plik `eidos-linux` i kompilacja ze źródeł nie dostarczają niczego): runtime .NET 10 z `builds.dotnet.microsoft.com` i instalatory Visual C++ i .NET 8 z `download.visualstudio.microsoft.com`, każde odrzucane, jeśli nie zgadza się z SHA-256 wbudowanym w Eidosa; SDK .NET 10, zawsze z `builds.dotnet.microsoft.com` i sprawdzane tak samo; oraz `winetricks` dla pozostałych wymagań (.NET Framework 4.8, XACT, .NET 6 i 7), który pobiera to, co instaluje, od Microsoftu i z innych hostów, których używa winetricks. [Oświadczenie o ochronie prywatności Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Logi sesji | przechowywane | `~/.local/share/Colony/Eidos/logs/`, uprawnienia 0600: dziesięć sesji na instancję, usuwane po 30 dniach. Twój folder domowy jest zapisywany jako `~`, a nazwa użytkownika jako `<user>`, więc log można wkleić do zgłoszenia bez zmian. Nigdy nie opuszczają twojego komputera, chyba że sam je wyślesz. |
| Zapisy uruchomień | przechowywane | `<instance>/logs/sessions/`: dla każdego uruchomienia gry lub narzędzia profil, włączone mody z ich wersjami i identyfikatorami plików Nexusa, kolejność wtyczek, suma kontrolna każdego INI profilu, wersje gry i Eidosa, kod wyjścia i zapisy, które uruchomienie utworzyło. Zachowuje się 50 najnowszych, czyta je `eidos sessions`, a `eidos pack` je pomija. Zostają na twoim komputerze. |
| Raporty aktualizacji | przechowywane | `<instance>/logs/updates/<mod>.json`: co ostatnia aktualizacja lub Replace zmieniła w folderze tego moda. `eidos pack` je pomija. |
| Skróty | przechowywane | `install.sh` zapisuje `eidos.desktop` i jego ikony pod `~/.local/share`; utworzony przez ciebie skrót do narzędzia lub profilu trafia do `~/.local/share/applications`. |
| Powiadomienia pulpitu | ani jedno, ani drugie | Domyślnie wyłączone. Po włączeniu idą do usługi powiadomień twojego pulpitu przez D-Bus (`org.freedesktop.Notifications`) i nazywają zadanie, które się skończyło. |
| Otwierane odnośniki | ani jedno, ani drugie | Strony modów i inne odnośniki otwierają się w twojej własnej przeglądarce przez `xdg-open`. |
| Dodatki | ani jedno, ani drugie | Zainstalowane rozszerzenia działają jako osobne programy na twoim komputerze; za to, co robią, odpowiadają one same. |

## Licencja

Oryginalny kod Eidosa jest objęty licencją GPL-3.0-or-later; zobacz
[LICENSE](../../../LICENSE). Te części są objęte licencją GPL-3.0 bez opcji
„or later”:

- tłumaczenia skryptów OMOD w `eidos-install`; zobacz
  [informacje o pierwotnych autorach](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- program pomocniczy `eidos-nif-preview`, zbudowany na dołączonej bibliotece
  nifly; zobacz
  [licencje i informacje o autorach jego składników](../../../crates/eidos-nif-preview/UPSTREAM.md);
- skrzynki `esplugin` i `libloadorder`, z którymi linkuje się kod kolejności
  wczytywania i które deklarują `GPL-3.0`.

Pozostałe skrzynki Rusta w plikach wykonywalnych zachowują własne licencje
(MIT, Apache-2.0, BSD i podobne). Pakiet wydania zawiera ich informacje
licencyjne w `licenses/crates/`, wraz z indeksem każdej skrzynki i jej licencji.

Pliki wykonywalne zawierają też osiem oryginalnych bibliotek DLL z pakietu
redystrybucyjnego Microsoft DirectX (`d3dcompiler_47`, `d3dcompiler_43`,
`d3dx9_43` i `d3dx11_43`, 32- i 64-bitowe), przechowywanych skompresowane gzipem
(7,9 MB zamiast 17,9 MB), które Eidos rozpakowuje i wdraża do prefiksu gry, gdy
potrzebuje ich mod lub narzędzie. Pozostają objęte warunkami redystrybucji
Microsoftu; zobacz ich [pochodzenie](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Pakiet wydania zawiera też w `runtimes/` instalatory Visual C++ 2015-2022,
instalator .NET 8 Desktop Runtime i zip runtime'u .NET 10, w postaci, w jakiej
publikuje je Microsoft. Ich warunki są w `licenses/microsoft-runtimes/`
(`vc_redist.txt`, `dotnet.txt`).

Zarządzanie modami należy do wszystkich.
