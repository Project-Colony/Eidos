<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Natywny linuksowy menedżer modów, który nigdy nie dotyka twojej gry.**

</div>

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

**Instancje.** Globalne - zarządzane centralnie pod `~/.local/share/eidos` - albo
przenośne: samodzielny folder gdziekolwiek chcesz (drugi dysk, partycja z
grami), przenoszalny i odizolowany, jak w MO2. Instancje przenośne są pamiętane
między sesjami; GUI, uruchomienie ze Steama i każde polecenie wiersza poleceń
podążają za ostatnio używaną, a każde polecenie przyjmuje folder wszędzie tam,
gdzie przyjmuje identyfikator gry. Szczegóły w
[usage.pl.md](docs/guide/usage.md#instancje-globalne-i-przenośne).

**Profile.** Kolejność modów, stan wtyczek, pliki INI i zapisy na profil. Zapisy
są parsowane, porównywane z twoimi obecnymi wtyczkami - z przyciskiem
włączającym to, czego zapis potrzebuje - i po każdej sesji synchronizowane z
powrotem dla Steam Cloud.

**Nexus.** Podłącz konto, a przycisk „Mod Manager Download" na stronie ląduje
prosto w twojej instancji, wraz ze sprawdzaniem aktualizacji względem tego, co
masz zainstalowane, autorem każdego moda i odnośnikiem do jego profilu. Odnośnik
do **kolekcji** wypisuje jej elementy zestawione z twoją instancją -
zainstalowane, pobrane, brakujące - co jest czytaniem kolekcji, a nie jej
instalowaniem, i panel mówi dlaczego. Zakładka Downloads to biblioteka archiwów:
filtruj, sortuj, ukrywaj bez usuwania i wyczyść te już zainstalowane.
Przełącznik **offline** zatrzymuje to wszystko.

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

**Gdzie trzyma własne pliki.** `~/.config/Colony/Eidos/` na to, co wybrałeś -
preferencje, twoja sesja Nexusa, twoja lista instancji, napisane przez ciebie
definicje gier i dodatków - z logami pod `~/.local/state/Colony/Eidos/`. Układ,
którego używa każdy program z rodziny Colony. Starszy Eidos trzymał to w
`~/.config/eidos/`; pierwsze uruchomienie po aktualizacji kopiuje je, mówi o tym
w logu i zostawia stary folder dokładnie takim, jaki był.

## Jak wypada na tle innych

| | Eidos | MO2 przez Wine | Fluorine-Manager | Limo / deployery dowiązań |
|---|---|---|---|---|
| Menedżer działa natywnie | ✅ | ❌ aplikacja Windows w Wine | ✅ (port Qt) | ✅ |
| Folder gry nietknięty | ✅ zawsze | ✅ | ✅ | ❌ zapisywane są w nim dowiązania |
| Montowanie widoczne dla | tylko gry | tylko gry | **całego systemu** | nie dotyczy |
| Sprzątanie po awarii | żadne, z założenia | żadne | odzyskiwanie martwego montowania | ręczne wycofanie |
| Mody root (ENB, preloadery) | ✅ natywnie | wymagana wtyczka | wymagana wtyczka | częściowo |
| Wymagane uprawnienia | żadne | żadne | edycja `/etc/fuse.conf` | żadne |

## Jak szybko działa

| | przedtem | teraz |
|---|---|---|
| wczytanie zapisu | ~20 sekund | **6-7 sekund** |
| odczyty folderów w jednej sesji | 5,6 miliona | 465 tysięcy |

Zmiany komórek są natychmiastowe. Zysk wziął się z zadawania twoim modom mniej
pytań: znalezienie jednego pliku odpytywało wcześniej wszystkie pięćdziesiąt po
kolei, a wypisanie jednego folderu robiło to pięćdziesiąt razy. Ani jedno, ani
drugie już tego nie robi. Mierzone na prawdziwej instancji granej normalnie, nie
na benchmarku.

## Pierwsze kroki

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Następnie ustaw opcję uruchamiania gry w Steamie na
`~/.local/bin/eidos-gui %command%` i naciśnij Graj.

Pakiety Arch i archiwa wydań, co trzeba mieć zainstalowane najpierw oraz droga
przez wiersz poleceń: **[docs/guide/install.pl.md](docs/guide/install.md)**.

## Opcje uruchamiania Steam

Podstawowa linia wystarcza większości konfiguracji:

```
~/.local/bin/eidos-gui %command%
```

Wszystko inne to zmienne środowiskowe ustawiane przed nią i łączą się dowolnie:

| Chcesz... | Wstaw przed |
|---|---|
| DLSS z Community Shaders | `PROTON_ENABLE_NVAPI=1` - bez niej DLSS po cichu nigdy się nie inicjalizuje; pełna lista kontrolna jest w [guide/graphics.pl.md](docs/guide/graphics.md) |
| licznik FPS na ekranie | `DXVK_HUD=fps` |
| interpolację klatek na poziomie sterownika, zero modów (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nigdy razem z własną generacją klatek Community Shaders |
| szczegółowe logi do zgłoszenia błędu | `EIDOS_LOG=debug` (logi sesji lądują w `~/.local/state/Colony/Eidos/logs/`) |
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
żyją w [guide/troubleshooting.pl.md](docs/guide/troubleshooting.md).

## Dokąd dalej

| Jeśli chcesz... | |
|---|---|
| zainstalować go | [guide/install.pl.md](docs/guide/install.md) |
| poznać wiersz poleceń i GUI | [guide/usage.pl.md](docs/guide/usage.md) |
| skonfigurować xEdit, BodySlide albo DynDOLOD | [guide/tools.pl.md](docs/guide/tools.md) |
| grać w Fallouta 4 (F4SE, wersje, awaria z gruzem na NVIDII) | [guide/fallout4.pl.md](docs/guide/fallout4.md) |
| uruchomić DLSS / generację klatek (Community Shaders) | [guide/graphics.pl.md](docs/guide/graphics.md) |
| naprawić coś, co wygląda źle | [guide/troubleshooting.pl.md](docs/guide/troubleshooting.md) |
| wiedzieć, dlaczego jest szybki, i sprawdzić to samodzielnie | [internals/performance.md](../../internals/performance.md) |
| zrozumieć, jak działa w środku | [internals/architecture.md](../../internals/architecture.md) |
| zbudować go, przetestować, wnieść wkład | [internals/contributing.md](../../internals/contributing.md) |
| wiedzieć, po co w ogóle istnieje | [project/landscape.md](../../project/landscape.md) |

Język to jeden katalog: `docs/i18n/pl/` odwzorowuje korzeń repozytorium, dzięki
czemu odnośnik między dwiema przetłumaczonymi stronami jest tym samym ciągiem co
odnośnik między ich angielskimi oryginałami.

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

Dodanie języka to cztery pliki i wiersz w tej tabeli; kroki opisuje
[`docs/internals/contributing.md`](../../internals/contributing.md).

## Obsługiwane gry

**Skyrim SE/AE** - sprawdzony w realnej grze. **Fallout 4** też jest podłączony
od początku do końca (F4SE podstawiany automatycznie, unieważnianie archiwów,
kolejność wczytywania z gwiazdką, LOOT, zapisy `.fos`) - zob.
[guide/fallout4.pl.md](docs/guide/fallout4.md). Podłączone według wspólnego
deskryptora gry i szukające testerów: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion i Morrowind (te dwa
ostatnie montują się i zarządzają modami; ich listy wtyczek uporządkowane
znacznikami czasu nie są jeszcze zarządzane).

Dodanie rodziny to jeden wiersz deskryptora:
[internals/adding-games.md](../../internals/adding-games.md).

## Wcześniejsze prace i podziękowania

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) i
  [usvfs](https://github.com/ModOrganizer2/usvfs) - semantyka, którą Eidos
  odtwarza, i baza kodu, wobec której badano jego zgodność
- [LOOT](https://loot.github.io/) - silnik sortowania, przez libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) i inne linuksowe menedżery - dowód, że
  jest społeczność, która chce ten problem rozwiązany

## Licencja

GPL-3.0-or-later. Zarządzanie modami należy do wszystkich.
