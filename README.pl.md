<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
[usage.pl.md](docs/guide/usage.pl.md#instancje-globalne-i-przenośne).

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
przez wiersz poleceń: **[docs/guide/install.pl.md](docs/guide/install.pl.md)**.

## Opcje uruchamiania Steam

Podstawowa linia wystarcza większości konfiguracji:

```
~/.local/bin/eidos-gui %command%
```

Wszystko inne to zmienne środowiskowe ustawiane przed nią i łączą się dowolnie:

| Chcesz... | Wstaw przed |
|---|---|
| DLSS z Community Shaders | `PROTON_ENABLE_NVAPI=1` - bez niej DLSS po cichu nigdy się nie inicjalizuje; pełna lista kontrolna jest w [guide/graphics.pl.md](docs/guide/graphics.pl.md) |
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
żyją w [guide/troubleshooting.pl.md](docs/guide/troubleshooting.pl.md).

## Dokąd dalej

| Jeśli chcesz... | |
|---|---|
| zainstalować go | [guide/install.pl.md](docs/guide/install.pl.md) |
| poznać wiersz poleceń i GUI | [guide/usage.pl.md](docs/guide/usage.pl.md) |
| skonfigurować xEdit, BodySlide albo DynDOLOD | [guide/tools.pl.md](docs/guide/tools.pl.md) |
| grać w Fallouta 4 (F4SE, wersje, awaria z gruzem na NVIDII) | [guide/fallout4.pl.md](docs/guide/fallout4.pl.md) |
| uruchomić DLSS / generację klatek (Community Shaders) | [guide/graphics.pl.md](docs/guide/graphics.pl.md) |
| naprawić coś, co wygląda źle | [guide/troubleshooting.pl.md](docs/guide/troubleshooting.pl.md) |
| wiedzieć, dlaczego jest szybki, i sprawdzić to samodzielnie | [internals/performance.md](docs/internals/performance.md) |
| zrozumieć, jak działa w środku | [internals/architecture.md](docs/internals/architecture.md) |
| zbudować go, przetestować, wnieść wkład | [internals/contributing.md](docs/internals/contributing.md) |
| wiedzieć, po co w ogóle istnieje | [project/landscape.md](docs/project/landscape.md) |

Cały indeks jest w [docs/README.pl.md](docs/README.pl.md); polityka
bezpieczeństwa i sposób zgłaszania podatności w [SECURITY.md](SECURITY.md).

## Język

Strony, których potrzebuje gracz, są przetłumaczone. **Angielski jest wersją
wzorcową**: gdy tłumaczenie się z nim nie zgadza, rację ma plik angielski.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Cała reszta jest po angielsku celowo, nie przez przeoczenie.**
`docs/internals/` i `docs/project/` czytają ludzie, którzy czytają też Rusta, a
`CHANGELOG.md` jest generowany. Tłumaczenie ich to 17 678 dodatkowych słów do
utrzymania w zgodzie z prawdą dla odbiorców, którzy ich nie potrzebują.

Każde tłumaczenie niesie skrót pliku angielskiego, z którego powstało, a CI
zawodzi, gdy angielski pójdzie do przodu - zob.
[`scripts/i18n-check.sh`](scripts/i18n-check.sh). Tłumaczenie, którego nie da się
zaktualizować, jest **usuwane**, a nie zostawiane na miejscu: nieaktualna strona
wciąż wygląda wiarygodnie i podaje polecenia sprzed miesiąca, co dla czytelnika
jest gorsze niż odesłanie do angielskiego.

Dodanie języka to cztery pliki i wiersz w tej tabeli; kroki opisuje
[`docs/internals/contributing.md`](docs/internals/contributing.md).

## Obsługiwane gry

**Skyrim SE/AE** - sprawdzony w realnej grze. **Fallout 4** też jest podłączony
od początku do końca (F4SE podstawiany automatycznie, unieważnianie archiwów,
kolejność wczytywania z gwiazdką, LOOT, zapisy `.fos`) - zob.
[guide/fallout4.pl.md](docs/guide/fallout4.pl.md). Podłączone według wspólnego
deskryptora gry i szukające testerów: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion i Morrowind (te dwa
ostatnie montują się i zarządzają modami; ich listy wtyczek uporządkowane
znacznikami czasu nie są jeszcze zarządzane).

Dodanie rodziny to jeden wiersz deskryptora:
[internals/adding-games.md](docs/internals/adding-games.md).

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
