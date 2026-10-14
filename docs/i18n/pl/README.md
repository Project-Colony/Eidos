<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Natywny linuksowy menedżer modów, który nigdy nie dotyka twojej gry.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#instalacja)

Eidos daje grom Bethesdy pod Linuksem to, co Mod Organizer 2 daje im pod
Windowsem - wirtualny, tworzony przy każdym uruchomieniu scalony widok twoich
modów - zbudowany z linuksowych prymitywów zamiast z przechwytywania API
Windows. Żadnego Wine dla menedżera. Żadnych plików kopiowanych do folderu gry.
Żadnej procedury czyszczenia, bo nie ma czego czyścić.

```
Steam ──> eidos-gui %command% ──> [ prywatna przestrzeń nazw ]
                                  │  mody ⊕ gra  ──> to, co widzi gra
                                  └─ ginie razem z grą; instalacja zostaje nietknięta
```

> **Stan:** w Skyrim SE gra się przez Eidos codziennie - SKSE, preloadery script
> extendera, Creation Club, kolejności wczytywania posortowane LOOT-em, zapisy
> na profil, wszystko. Jak dotąd jedna rodzina gier sprawdzona w realnej grze;
> dziesięć kolejnych jest podłączonych i czeka na testerów.

## Dlaczego Eidos

- 🔒 **Montowanie, które widzi tylko twoja gra.** Scalony widok żyje w prywatnej
  przestrzeni nazw montowań: twój menedżer plików, twoje zadanie kopii
  zapasowej, druga gra - żadne z nich go nie widzi, żadne nie potrzebuje do
  niego uprawnień. Ubij grę, odetnij prąd: przestrzeń nazw ginie razem z drzewem
  procesów, a twoja instalacja jest dokładnie taka, jaka była. Nie ma żadnych
  pozostałości *z konstrukcji*.
- 🧾 **Jedna kopia prawdy.** Twój profil ma własną listę modów, kolejność
  wtyczek, pliki INI i zapisy. Pliki wtyczek i folder zapisów są przy
  uruchomieniu montowane przez bind na ścieżkach samej gry, więc nawet zapisy
  samej gry lądują w twoim profilu. Zmiana profilu zmienia wszystko.
- 🐧 **Całkowicie bez roota.** Żadnego pomocnika setuid, żadnego demona, żadnego
  `sudo setcap`, żadnych edycji `/etc/fuse.conf`. Jeden plik wykonywalny, jedna
  opcja uruchamiania Steama.
- 🛡️ **Zabezpieczenia z dowodami.** Awaria, która niszczy twoją listę wtyczek,
  jest zgłaszana na tle migawki sprzed sesji, z przywracaniem jednym
  kliknięciem. Przechwycenie, które wymazałoby twoją kolejność wczytywania, jest
  odrzucane i mówi dlaczego.

### Jak wypada na tle innych

| | Eidos | MO2 przez Wine | Fluorine-Manager | Limo / deployery dowiązań |
|---|---|---|---|---|
| Menedżer działa natywnie | ✅ | ❌ aplikacja Windows w Wine | ✅ (port Qt) | ✅ |
| Folder gry nietknięty | ✅ zawsze | ✅ | ✅ | ❌ zapisywane są w nim dowiązania |
| Montowanie widoczne dla | tylko gry | tylko gry | **całego systemu** | nie dotyczy |
| Sprzątanie po awarii | żadne, z założenia | żadne | odzyskiwanie martwego montowania | ręczne wycofanie |
| Mody root (ENB, preloadery) | ✅ natywnie | wymagana wtyczka | wymagana wtyczka | częściowo |
| Wymagane uprawnienia | żadne | żadne | edycja `/etc/fuse.conf` | żadne |

### Jak szybko działa

| | przedtem | teraz |
|---|---|---|
| wczytanie zapisu | ~20 sekund | **6-7 sekund** |
| odczyty folderów w jednej sesji | 5,6 miliona | 465 tysięcy |

Zmiany komórek są natychmiastowe. Zysk wziął się z zadawania twoim modom mniej
pytań: znalezienie jednego pliku odpytywało wcześniej wszystkie pięćdziesiąt po
kolei, a wypisanie jednego folderu robiło to pięćdziesiąt razy. Ani jedno, ani
drugie już tego nie robi. Mierzone na prawdziwej instancji granej normalnie, nie
na benchmarku.

### Wcześniejsze prace i podziękowania

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) i
  [usvfs](https://github.com/ModOrganizer2/usvfs) - semantyka, którą Eidos
  odtwarza, i baza kodu, wobec której badano jego zgodność
- [LOOT](https://loot.github.io/) - silnik sortowania, przez libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) i inne linuksowe menedżery - dowód, że
  jest społeczność, która chce ten problem rozwiązany

## Co robi

**Mody.** Proste archiwa, kreatory FOMOD, paczki BAIN Wrye Basha, ręczny wybór
dla reszty - i **mody root natywnie** (preloadery script extendera, ENB, Engine
Fixes), bez wtyczki Root Builder i bez kopiowania czegokolwiek do twojej
instalacji. Ukrywaj pojedyncze pliki, grupuj separatorami, celowane
przenoszenie, notatki i kategorie dla każdego moda oraz importer profili MO2.

Lista jest listą MO2, z jej przyzwyczajeniami: osiem opcjonalnych kolumn i
sortowanie po każdej z nich, grupowanie według kategorii lub źródła, gesty
podwójnego kliknięcia, skok do nazwy przez pisanie, kopie zapasowe pojedynczych
modów bezczynne, dopóki ich nie przywrócisz, oraz flagi doradcze dla moda,
którego układ nie zostanie przez tę grę wczytany albo który został pobrany do
innej. Jego drzewo plików wykonuje zwykłe operacje - nowy folder, zmiana nazwy,
usunięcie, otwarcie - i podgląda obrazy oraz teksty, nic nie uruchamiając.

**Wtyczki.** Kolejność wczytywania z wbudowanym sortowaniem LOOT-em, indeksy
modów takie, jakie wylicza gra, ostrzeżenia o brakujących masterach oraz twoje
DLC i zawartość Creation Club pokazane jako niezarządzane wiersze, którymi są.

**Instancje.** Globalne - zarządzane centralnie pod `~/.local/share/Colony/Eidos/instances/` - albo
przenośne: samodzielny folder gdziekolwiek chcesz (drugi dysk, partycja z
grami), przenoszalny i odizolowany, jak w MO2. Instancje przenośne są pamiętane
między sesjami; GUI, uruchomienie ze Steama i każde polecenie wiersza poleceń
podążają za ostatnio używaną, a każde polecenie przyjmuje folder wszędzie tam,
gdzie przyjmuje identyfikator gry. Szczegóły w
[usage.md](docs/guide/usage.md#instancje-globalne-i-przenośne).

**Profile.** Kolejność modów, stan wtyczek, pliki INI i zapisy na profil. Zapisy
są parsowane, porównywane z twoimi obecnymi wtyczkami - z przyciskiem
włączającym to, czego zapis potrzebuje - i po każdej sesji synchronizowane z
powrotem dla Steam Cloud.

**Nexus.** Połącz konto, aby odbierać odnośniki „Mod Manager Download”, sprawdzać aktualizacje oraz przeglądać autorów i ich profile. Kolekcje są instalowane według swoich przepisów, z możliwością wznowienia postępu i wyraźnymi raportami o nieobsługiwanych krokach. Porównanie sprawdza dokładne pliki źródłowe; niepełne starsze metadane pozostają niezweryfikowane. Karta pobierania pozwala filtrować, sortować, ukrywać i usuwać archiwa. Przełącznik **offline** blokuje dostęp do Nexusa.

**Narzędzia.** xEdit, BodySlide, DynDOLOD i spółka działają *przez scalony widok*
wewnątrz prefiksu Proton gry - widzą twoje mody, ich wyniki lądują w Overwrite,
a jedno kliknięcie zamienia je w prawdziwego moda. Środowisko uruchomieniowe,
którego każde z nich potrzebuje, jest pobierane na żądanie, więc brakująca DLL
to przycisk, a nie popołudnie. xEdit i jego bliźniak QuickAutoClean są
znajdowane za ciebie - w folderze gry, wewnątrz moda albo w folderze narzędzi,
który trzymasz obok swoich gier - z już wybranymi właściwymi środowiskami
uruchomieniowymi. Przypnij te, których używasz, ukryj te, których nie, nadaj
narzędziu własne Steam AppID,
gdy jest osobną aplikacją Steama, i zapisz skrót `.desktop`, który uruchamia je
przez scalony widok, w ogóle nie otwierając Eidosa.

**Diagnostyka.** Brakujące mastery, osierocone archiwa, dryf listy modów,
uszkodzone zestawy wtyczek - a po uruchomieniu to, co własny log script
extendera mówi, że faktycznie się wczytało.

**Gdzie trzyma własne pliki.** Układ, którego używa każdy program z rodziny
Colony: `~/.config/Colony/Eidos/` na to, co wybrałeś - preferencje, twoja sesja
Nexusa, twoja lista instancji, napisane przez ciebie definicje gier i dodatków -
`~/.local/share/Colony/Eidos/` na instancje globalne, pobrane środowiska
uruchomieniowe i logi sesji, a `~/.cache/Colony/Eidos/` na masterlisty LOOT.
Starszy Eidos trzymał to w `~/.config/eidos/`, `~/.local/share/eidos/` i
`~/.local/state/`; pierwsze uruchomienie po aktualizacji przenosi je, mówi o tym
w logu i zostawia stare foldery dokładnie takimi, jakie były. Dopóki przenosiny
się nie zakończą, Eidos dalej używa starych plików. Szczegóły w
[internals/paths.md](../../internals/paths.md) (po angielsku).

[Kontrole integralności i wygenerowane pliki](../../../docs/internals/integrity.md) (po angielsku) opisuje ochronę przy zastępowaniu modów, konflikty plików w BSA/BA2, statyczną diagnostykę SKSE i zapisy generowania wraz z ich ograniczeniami.

### Obsługiwane gry

**Skyrim SE/AE** - sprawdzony w realnej grze. **Fallout 4** też jest podłączony
od początku do końca (F4SE podstawiany automatycznie, unieważnianie archiwów,
kolejność wczytywania z gwiazdką, LOOT, zapisy `.fos`) - zob.
[guide/fallout4.md](docs/guide/fallout4.md). Podłączone według wspólnego
deskryptora gry i szukające testerów: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion i Morrowind (te dwa
ostatnie montują się i zarządzają modami; ich listy wtyczek uporządkowane
znacznikami czasu nie są jeszcze zarządzane).

Dodanie rodziny to jeden wiersz deskryptora:
[internals/adding-games.md](../../internals/adding-games.md).


## Instalacja

### Przez Colony (zalecane)

Wyszukaj **Eidos** w [Colony](https://github.com/Project-Colony/Colony) i
zainstaluj go. Aktualizacje przychodzą przez launcher. Colony instaluje jeden
plik, `eidos-linux`: okno, a zarazem wiersz poleceń `eidos`, gdy jest wywołany
przez dowiązanie o nazwie `eidos`. Podgląd modeli NIF potrzebuje programu
pomocniczego, który ma tylko opisany niżej pakiet; bez niego podgląd to
zgłasza, a wszystko inne działa. Co zrobić po instalacji i jak skierować na
niego Steama: [docs/guide/install.md](docs/guide/install.md#przez-colony).

### Bezpośrednie pobranie pliku wykonywalnego

Pobierz plik z [najnowszego wydania](https://github.com/Project-Colony/Eidos/releases/latest):

| Plik | Co to jest |
|---|---|
| `eidos-linux` | cały program w jednym pliku, tak jak instaluje go Colony |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, program pomocniczy podglądu NIF, ikony i instalator, który rejestruje wpis pulpitu i obsługę pobierania z Nexusa |

Najpierw sprawdź pobrany plik, jak pokazuje
[Polityka podpisywania kodu](#polityka-podpisywania-kodu), a potem:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Instalator umieszcza wszystko w `~/.local/bin` bez sudo; `--system` i
`--bindir DIR` wybierają inne miejsce. Oba pliki wymagają Linuksa x86_64 z
glibc 2.39 lub nowszą, FUSE (`fusermount3`) i 7-Zip do archiwów modów.

### Budowanie ze źródeł

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Wymaga Rusta 1.89 lub nowszego (`rust-version` sprawdzane przez CI; CI i
wydania budują wersją 1.99.0), a także `just`, CMake 3.20+ i kompilatora C++17
dla programu pomocniczego podglądu NIF. Pakiet Arch buduje się z
`packaging/PKGBUILD`. Wymagania wstępne i polecenia bez `just`:
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

Wszystko inne to zmienne środowiskowe ustawiane przed nią i łączą się dowolnie:

| Chcesz... | Wstaw przed |
|---|---|
| DLSS z Community Shaders | `PROTON_ENABLE_NVAPI=1` - bez niej DLSS po cichu nigdy się nie inicjalizuje; pełna lista kontrolna jest w [guide/graphics.md](docs/guide/graphics.md) |
| licznik FPS na ekranie | `DXVK_HUD=fps` |
| interpolację klatek na poziomie sterownika, zero modów (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nigdy razem z własną generacją klatek Community Shaders |
| szczegółowe logi do zgłoszenia błędu | `EIDOS_LOG=debug` (logi sesji lądują w `~/.local/share/Colony/Eidos/logs/`) |
| raport we/wy z montowania dla każdej sesji | `EIDOS_FUSE_STATS=1` |
| inną liczbę wątków roboczych FUSE | `EIDOS_FUSE_THREADS=8` (domyślnie 4; `1` to pierwsza rzecz do wypróbowania przy tropieniu błędu współbieżności) |
| przypiąć to uruchomienie do jednej instancji przenośnej | `EIDOS_INSTANCE=/path/to/folder` - bez niej Eidos otwiera ostatnio używaną instancję, co zwykle jest tym, czego chcesz |

Linia do zachowania przy nowoczesnej moddowanej konfiguracji (Community Shaders,
DLSS, generacja klatek) - to jest ostateczne polecenie, nie przykład:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Dodaj z przodu `DXVK_HUD=fps` na czas sprawdzania, czy konfiguracja działa, i
usuń go, gdy już działa.

Głębsze przełączniki diagnostyczne (`EIDOS_FUSE_TRACE`, przełączniki bisekcji
cache'u i indeksu, dlaczego `EIDOS_FUSE_PASSTHROUGH` jest domyślnie wyłączony)
żyją w [guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Dokumentacja

Wszystko jest w [docs/](docs/), z indeksem ułożonym według tego, kto czyta, w
[docs/README.md](docs/README.md): przewodnik użytkownika (instalacja, wiersz
poleceń i GUI, narzędzia takie jak xEdit i DynDOLOD, grafika, rozwiązywanie
problemów), jak Eidos działa w środku i po co istnieje. Polityka
bezpieczeństwa i sposób zgłaszania podatności:
[SECURITY.md](../../../SECURITY.md).

## Język

Strony, których potrzebuje gracz, są przetłumaczone. **Angielski jest wersją
wzorcową**: gdy tłumaczenie się z nim nie zgadza, rację ma plik angielski.

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

**Cała reszta jest po angielsku celowo, nie przez przeoczenie.**
`docs/internals/` i `docs/project/` czytają ludzie, którzy czytają też Rusta, a
`CHANGELOG.md` jest generowany. Tłumaczenie ich to 17 678 dodatkowych słów do
utrzymania w zgodzie z prawdą dla odbiorców, którzy ich nie potrzebują.

Każde tłumaczenie niesie skrót pliku angielskiego, z którego powstało, a CI
zawodzi, gdy angielski pójdzie do przodu - zob.
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Tłumaczenie, którego nie da się
zaktualizować, jest **usuwane**, a nie zostawiane na miejscu: nieaktualna strona
wciąż wygląda wiarygodnie i podaje polecenia sprzed miesiąca, co dla czytelnika
jest gorsze niż odesłanie do angielskiego.

Język to jeden katalog. `docs/i18n/<lang>/` odwzorowuje korzeń repozytorium,
więc `docs/i18n/de/docs/guide/install.md` to niemiecki `docs/guide/install.md` -
dzięki temu odnośnik między dwiema przetłumaczonymi stronami jest TYM SAMYM
ciągiem co odnośnik między ich angielskimi oryginałami, a wycofanie języka to
jedno `rm -r`.

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
o awariach. Nie łączy się z żadnym serwerem poza wymienionymi w tej tabeli i
tylko w celu tam podanym. Tryb **offline** (Preferences -> Nexus) zatrzymuje
każde połączenie z Nexusem.

| Dane | Przechowywane czy wysyłane | Gdzie i dlaczego |
|---|---|---|
| Preferencje, lista instancji, definicje gier i dodatków | przechowywane | `~/.config/Colony/Eidos/` |
| Instancje, pobrane środowiska uruchomieniowe, logi sesji | przechowywane | `~/.local/share/Colony/Eidos/` albo folder wybrany dla instancji przenośnej |
| Masterlisty LOOT | przechowywane | `~/.cache/Colony/Eidos/loot/` |
| Logowanie do Nexusa | wysyłane | Logowanie otwiera `users.nexusmods.com` w twojej przeglądarce, która przekazuje wynik z powrotem do nasłuchu, jaki Eidos otwiera na `127.0.0.1` tylko na czas tego logowania. Eidos wymienia go następnie na tokeny pod `users.nexusmods.com/oauth/token` i pobiera klucze publiczne, którymi Nexus je podpisuje. [Polityka prywatności Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Dane uwierzytelniające Nexusa | przechowywane | Tokeny dostępu i odświeżania, w `~/.config/Colony/Eidos/nexus.ini` z uprawnieniami 0600. Linia `api_key=` zapisana tam przez starszego Eidosa jest zachowywana, ale nigdy nieczytana; możesz ją usunąć. |
| Żądania do Nexusa | wysyłane | `api.nexusmods.com`, gdy używasz funkcji Nexusa: nazwa twojego konta i status premium, szczegóły modów i plików, sprawdzanie aktualizacji, kolekcje i odnośniki pobierania. Każde żądanie niesie twój token dostępu i podaje Eidosa oraz jego wersję. |
| Pobrania z Nexusa | wysyłane | Pliki, o które prosisz, z hosta CDN Nexusa wskazanego w odnośniku pobierania, zapisywane w folderze pobrań instancji z plikiem `.meta` w formacie MO2 obok każdego z nich. |
| Odnośniki `nxm://` | przechowywane | `eidos nxm --register`, które `install.sh` uruchamia za ciebie, zapisuje `~/.local/share/applications/eidos-nxm.desktop` i ustawia go jako obsługę `nxm://` przez `xdg-mime`, więc przycisk Mod Manager Download na stronie przekazuje swój odnośnik Eidosowi. Pakiet Arch instaluje ten sam wpis dla całego systemu. |
| Masterlisty LOOT | wysyłane | `raw.githubusercontent.com/loot/...`, przy pierwszym sortowaniu LOOT danej gry i za każdym razem, gdy prosisz o aktualizację. Nic twojego nie jest wysyłane. [Oświadczenie o prywatności GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Środowiska uruchomieniowe narzędzi | wysyłane | Tylko gdy instalujesz wymaganie wstępne narzędzia albo potrzebuje go mod: środowisko .NET z `builds.dotnet.microsoft.com`, odrzucane, jeśli nie zgadza się z SHA-256 wbudowanym w Eidosa, oraz `winetricks`, który pobiera to, co instaluje w prefiksie Proton gry (pakiety redystrybucyjne Visual C++ i .NET), od Microsoftu i z innych hostów, których używa winetricks. [Oświadczenie o ochronie prywatności Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Logi sesji | przechowywane | `~/.local/share/Colony/Eidos/logs/`, uprawnienia 0600: dziesięć sesji na instancję, usuwane po 30 dniach. Twój folder domowy jest zapisywany jako `~`, a nazwa użytkownika jako `<user>`, więc log można wkleić do zgłoszenia bez zmian. Nigdy nie opuszczają twojego komputera, chyba że sam je wyślesz. |
| Skróty | przechowywane | `install.sh` zapisuje `eidos.desktop` i jego ikony pod `~/.local/share`; utworzony przez ciebie skrót do narzędzia trafia do `~/.local/share/applications`. |
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
`d3dx9_43` i `d3dx11_43`, 32- i 64-bitowe), które Eidos wdraża do prefiksu gry,
gdy potrzebuje ich mod lub narzędzie. Pozostają objęte warunkami redystrybucji
Microsoftu; zobacz ich [pochodzenie](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Zarządzanie modami należy do wszystkich.
