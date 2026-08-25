<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Używanie Eidos

Praktyczny podręcznik: wiersz poleceń, GUI, opcja uruchamiania w Steamie,
budowanie ze źródeł i skrypt proof of concept. Co robić, gdy coś wygląda źle,
opisuje [troubleshooting.pl.md](troubleshooting.md).

## Jak używać (wiersz poleceń)

```sh
eidos games                       # obsługiwane gry zainstalowane tutaj (jak lista MO2)
eidos init skyrimse               # utworzyć instancję do modowania
# ...każdy mod wrzucić jako folder do <instance>/mods/ (instancja globalna leży
#    w ~/.local/share/eidos/skyrimse; `eidos init` wypisze twoją)...
eidos install skyrimse mod.7z     # albo zainstalować pobrane archiwum (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # przejąć kolejność i stan wtyczek istniejącego profilu MO2
eidos sort skyrimse               # posortować kolejność wczytywania wtyczek LOOT-em
eidos play skyrimse               # pokazać, co zostałoby zamontowane
eidos play skyrimse -- <command>  # uruchomić <command> z modami zamontowanymi nad grą
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` i `eidos export`
dopełniają zestaw; uruchom `eidos` bez argumentów, by zobaczyć pełną listę.

### Instancje: globalne i przenośne

Każde powyższe polecenie adresuje instancję. `skyrimse` nazywa tę **globalną** -
przechowywaną centralnie w `~/.local/share/eidos/skyrimse`, zarządzaną przez
Eidos. Drugi rodzaj to instancja **przenośna**: samodzielny folder gdziekolwiek
chcesz (drugi dysk, partycja z grami), przenoszalna i odizolowana, dokładnie jak
instancje przenośne MO2. Wszędzie tam, gdzie polecenie przyjmuje identyfikator
gry, przyjmuje też folder instancji przenośnej:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # utworzyć tam instancję przenośną
eidos install /mnt/games/EidosSkyrim mod.7z  # każde polecenie przyjmuje ten folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

Folder sam się opisuje (jego `eidos-instance.ini` nazywa grę), więc nic więcej
nie jest potrzebne - a `EIDOS_INSTANCE=<folder>` w środowisku przekierowuje
identyfikator gry na ten folder, co przydaje się w opcjach uruchamiania Steama.
Instancje przenośne, które utworzyłeś lub otworzyłeś, są zapamiętywane (ostatnio
używane pierwsze) w `~/.config/Colony/Eidos/instances.ini`; ekran powitalny GUI
wypisuje je do otwarcia jednym kliknięciem, uruchomienie ze Steama trafia na tę,
w którą grałeś ostatnio, a program obsługujący `nxm://` pobiera do niej. Dwa
zastrzeżenia warte poznania: przeniesienie folderu przenośnego zachowuje
wszystko poza wpisami narzędzi zarejestrowanymi ścieżkami bezwzględnymi do
starej lokalizacji (te trzeba dodać ponownie), a współdzielony cache środowisk
uruchomieniowych (`~/.local/share/Colony/Eidos/runtimes/`) celowo pozostaje
globalny dla maszyny - host .NET ważący 78 MB nie jest per instancja.

Eidos trzyma własne pliki pod `Colony/Eidos`, w układzie używanym przez każdy
program z rodziny Colony: `~/.config/Colony/Eidos/` na to, co wybrałeś
(preferencje, twoja sesja Nexusa, twoja lista instancji, napisane przez ciebie
definicje gier i dodatków), `~/.local/state/Colony/Eidos/logs/` na logi sesji i
`~/.local/share/Colony/Eidos/` na to, co Eidos pobrał. Starszy Eidos trzymał je
w `~/.config/eidos/` i `~/.local/state/eidos/`; pierwsze uruchomienie po
aktualizacji **kopiuje** je i mówi o tym w logu. Stare katalogi zostają dokładnie
takie, jakie były - nic nie jest usuwane, więc nieudana aktualizacja nie może cię
kosztować zalogowania - i możesz je usunąć sam, gdy się upewnisz.

Twoje mody nie są tego częścią. Instancja globalna nadal leży w
`~/.local/share/eidos/<game>/`, a przenośna tam, gdzie ją umieściłeś, ponieważ te
ścieżki są wpisane w twoją listę instancji i być może w opcję uruchamiania
Steama: przeniesienie ich zerwałoby połączenie, którego Eidos nie trzyma z obu
stron.

Jedno miejsce jest odrzucane wprost: **wnętrze folderu instalacyjnego gry**
(odruch weterana MO2). To drzewo należy do Steama - aktualizacja, „sprawdzenie
spójności plików" albo odinstalowanie może je nadpisać lub skasować, zabierając
ze sobą całą twoją konfigurację - a Eidos montuje nad katalogiem głównym gry,
więc instancja w środku siedziałaby wewnątrz własnego celu montowania. Kreator,
`eidos init` i `eidos play` odmawiają; umieść folder OBOK gry (sąsiad na tym
samym dysku daje tę samą wygodę).

`play` montuje mody instancji nad własnym katalogiem `Data` gry (przez
bind-stash, dzięki czemu demon nadal czyta nietknięte pliki) wewnątrz prywatnej
przestrzeni nazw, a potem uruchamia polecenie przez ten widok. Zapisy (save'y,
wygenerowane na nowo konfiguracje) lądują w warstwie `overwrite/` instancji;
instalacja gry i każde źródło moda pozostają nietknięte co do bajtu.

### Żaden krok z uprawnieniami nie jest potrzebny

Eidos działa całkowicie bez roota. Montuje w prywatnej przestrzeni nazw
użytkownika i montowania, więc żadnego pomocnika setuid, żadnego demona i
niczego do nadania.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` jest **opcjonalne** i
otwiera dokładnie jedną rzecz: jądrowy passthrough FUSE, domyślnie wyłączony, bo
psuje grę (niżej). Z tą zdolnością Eidos bierze zwykłą przestrzeń nazw
montowania zamiast przestrzeni nazw użytkownika; mody wdrażają się tak samo w
obu przypadkach.


Dlaczego dawna rada z `setcap` zniknęła - i dlaczego passthrough FUSE jest
dostarczany wyłączony - wyjaśnia
[troubleshooting.pl.md](troubleshooting.md#dlaczego-passthrough-jest-domyślnie-wyłączony).

## GUI

```sh
cargo run -p eidos-gui
```

Kreator pierwszego uruchomienia w stylu MO2, w pergaminowo-burgundowym wyglądzie
Colony: powitanie -> typ instancji (przenośna / globalna) -> gra -> nazwa i
lokalizacja -> podsumowanie -> utworzenie -> ekran główny. Ekran powitalny
wypisuje też każdą znaną istniejącą instancję (globalną i przenośną, ostatnio
używane pierwsze) do otwarcia jednym kliknięciem - służy zarazem za przełącznik
instancji - a wskazanie kreatorowi folderu, w którym instancja już jest,
PRZEJMUJE ją taką, jaka jest, zamiast tworzyć na niej (odmawiając wprost, jeśli
folder należy do innej gry).

Dwupanelowe okno główne też jest zbudowane: wybór profilu (przełączyć albo
utworzyć nowy przez skopiowanie bieżącego), lista modów, którą filtrujesz,
zaznaczasz, przestawiasz, grupujesz separatorami, zawężasz po kategorii i
klikasz prawym po akcje, plus zakładki Data / Plugins / Conflicts / Overwrite /
Saves / Downloads / Diagnostics oraz przycisk Run z wyborem celu uruchomienia.

Przestawianie to nie tylko wysłanie na samą górę i na sam dół: celowane
przesunięcia MO2 też tu są - wyślij nad pierwszy konfliktujący mod, pod ostatni,
na wskazany priorytet albo do grupy separatora. Wszystkie idą przez jednego
wspólnego pomocnika przesuwania, więc błąd o jeden, biorący się z usuwania
wierszy przed ich ponownym wstawieniem, istnieje w jednym miejscu zamiast w
pięciu.

### Kolumny, sortowanie i grupowanie

Lista rysuje z pudełka cztery kolumny, a oferuje osiem: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Zaznaczasz je w menu View.
Domyślnie nie ma wszystkich ośmiu i to celowo - lista z każdą widoczną kolumną
nie ma już miejsca na NAZWĘ, czyli tę kolumnę, którą naprawdę czytasz.

Kliknięcie nagłówka sortuje po nim. Kolejne kliknięcie odwraca, a trzecie wraca
do **kolejności wczytywania**, co znaczy więcej, niż brzmi: kolejność wczytywania
to jedyny porządek, w którym listę da się przeciągać, bo szczelina wstawienia
adresuje prawdziwą listę, podczas gdy posortowany wiersz jest zupełnie gdzie
indziej. Gdy sortowanie jest włączone, paski wstawienia nie są rysowane, a
przeciągnięcie zostaje odrzucone, zamiast lądować tam, gdzie nikt nie celował -
dokładnie to samo robi MO2 i z tego samego powodu. Menu View mówi o tym i podaje
drogę powrotną.

Menu View potrafi też **pogrupować** całą listę, po kategorii albo po źródle (z
Nexusa albo zainstalowane ręcznie). Nagłówki grup nie są separatorami: nie stoi
za nimi nic do zmiany nazwy, koloru czy przesunięcia, zwijają się, a licznik
zostaje na nagłówku po zwinięciu. Separatory znikają z listy pod sortowaniem albo
grupowaniem - separator stoi na czele wierszy, które idą po nim w kolejności
wczytywania, a jedno i drugie je przestawiło.

### Mysz i klawiatura

Podwójne kliknięcie moda otwiera Information, Ctrl+podwójne kliknięcie jego
folder, Shift+podwójne kliknięcie jego stronę na Nexusie. Ctrl+F stawia kursor w
polu filtra. Napisanie litery skacze do następnego moda zaczynającego się od
niej, a ponowne jej naciśnięcie idzie przez resztę, zamiast tkwić na pierwszym.
Żadne z nich nie może wylądować na wierszu, który ukrywa filtr, zwinięty
separator albo zwinięta grupa - przesuwanie podświetlenia, którego nie widzisz,
to sposób, w jaki następna spacja przełącza moda, na którego nie patrzyłeś.

„Collapse others" w menu separatora zwija każdą grupę poza tą jedną. W trakcie
przeciągania zatrzymanie się nad zwiniętą grupą otwiera ją, więc mod da się
upuścić w środku bez porzucania przeciągania - zatrzymanie się, nie przemknięcie
obok.

### Co lista mówi ci o modzie

Dwie flagi doradcze, obie w postaci znaku z wyjaśnieniem po najechaniu. **No
valid game data** znaczy, że nic na szczycie moda nie wygląda na coś, co ta gra
wczytuje; może trzeba przenieść jego foldery o poziom wyżej, a może to nie jest
mod do tej gry. **Another game** znaczy, że własny `meta.ini` moda nazywa inną.
Żadna niczego nie blokuje - mod nadal się wdraża - a „Mark as valid" w menu
wiersza ucisza obie, przez własny klucz `validated=` MO2, więc mod, za którego
poręczyłeś w jednym menedżerze, przychodzi cichy w drugim.

Sprawdzenie układu jest celowo hojne: drzewo `Root/` się liczy, nieczytelny
folder się liczy, pusty się liczy. Błędne ostrzeżenie na liście pięciuset wierszy
jest gorsze niż brakujące.

### Kopia moda, zanim go dotkniesz

„Back up this mod" kopiuje jego folder na bok jako `<name>_backup` (potem
`_backup2` i tak dalej - kopia nigdy nie zastępuje poprzedniej). Kopia jest
**bezczynna**: nie jest modem, jej pole wyboru nic nie robi i nic nie wnosi do
scalonego widoku, bo zaznaczenie jej wdrożyłoby dwie kopie jednego moda jedna na
drugą. „Restore this backup over the mod" wstawia ją z powrotem, w dwóch
kliknięciach; bieżąca zawartość jest najpierw odsuwana na bok i odrzucana dopiero
wtedy, gdy kopiowanie się powiedzie.

**Data** to prawdziwe drzewo scalonego widoku, rozwijane po jednym poziomie, więc
otwarcie węzła kosztuje jeden odczyt katalogu na warstwę, która go ma, a nie
rekurencyjny przemarsz przez każdy włączony mod. Odpowiada na nie TEN SAM stos
warstw, z którego serwuje montowanie, więc whiteouty i pliki ukryte są
respektowane, a zakładka nie może być niezgodna z tym, co zobaczy gra. Filtruj
po nazwie, zawęź do samych plików spornych, rozeznaj, co jest gdzie, po kolumnach
Size i Modified, i pokaż dowolny wiersz w menedżerze plików przez Reveal.
**Plugins** to kolejność wczytywania ESP/ESM/ESL (przełączanie, ręczne
przestawianie albo sortowanie LOOT-em i lektura raportu po sortowaniu, którego
odnośniki z poradami otwierają się w twojej przeglądarce). **Conflicts**
wyjaśnia wygranych i przegranych plik po pliku. **Overwrite** zamienia to, co
gra zapisała, w prawdziwy mod jednym krokiem. **Saves** rozbiera nagłówek każdego
zapisu - postać, poziom, lokacja, czas gry - i porównuje wpieczoną w niego listę
wtyczek z twoją bieżącą, wraz z przyciskiem włączającym mody, których potrzebuje,
bo nazwanie ich i zostawienie cię z tym to ta nudna połowa.

„Information..." otwiera okno dla pojedynczego moda: ogólne, konflikty, drzewo
plików, poprawki INI, notatki. Z drzewa plików (i z drzewa Data) każdy plik można
**ukryć** - zmienić mu nazwę na `<name>.mohidden`, co wyrzuca go z widoku
wirtualnego bez kasowania, więc trzy zabłąkane siatki jednego moda da się
stłumić bez ruszania priorytetów. Drzewo plików wykonuje też zwykłe operacje na
plikach: nowy folder, zmiana nazwy, usunięcie, otwarcie. Wszystkie idą przez
jeden resolver, który odrzuca cokolwiek, co nie jest zwykłą ścieżką wewnątrz tego
moda - żadnych `..`, żadnej ścieżki bezwzględnej i żadnego składnika będącego
dowiązaniem symbolicznym, bo pójście za nim wyprowadziłoby usunięcie całkiem poza
folder moda. Zmiana nazwy podmienia tylko ostatni składnik, więc nigdy nie stanie
się przeniesieniem, i odrzuca nazwę już zajętą, zamiast po cichu zastąpić tamten
plik. Usunięcie zabiera dwa kliknięcia; to jedyna tutejsza akcja, której kolejne
kliknięcie nie cofnie.

**View** na dowolnym wierszu w drzewie plików albo w drzewie Data daje podgląd
pliku: obrazy i tekst. Nie DDS ani NIF - te potrzebują dekodera bloków i
renderera, których to drzewo nie ma - ale mówią o tym, zamiast pokazywać puste
pole, i wskazują na Reveal. Tekst jest czytany do 64 KB i mówi, gdzie się
zatrzymał, bo podgląd to rzut oka, a log Papyrusa może mieć sto megabajtów.
**INI Tweaks** wypisuje fragmenty, które mod dostarcza w swoim folderze
`INI Tweaks/`; włączone są scalane z plikiem INI gry z profilu przy uruchomieniu,
w kolejności priorytetów, i zdejmowane z powrotem, gdy pliki INI po przebiegu są
przechwytywane - inaczej poprawka po cichu staje się ustawieniem, a jej
wyłączenie nic by nie dało.

Pobrany plik można **przeciągnąć z listy Downloads na pozycję w liście modów**,
by zainstalować go z tym priorytetem, a archiwa i foldery upuszczone na okno z
menedżera plików też się instalują (ta połowa wymaga sesji X11 albo XWayland -
winit implementuje upuszczanie plików tylko dla X11). Same pobierania da się
wstrzymać i wznowić: wstrzymanie zatrzymuje transfer i zachowuje część, a Resume
rozwiązuje na nowo świeży odnośnik i kontynuuje od miejsca zatrzymania.

Zakładka Downloads to **biblioteka** archiwów, nie kolejka transferów. Filtruj po
nazwie (także po przyjaznej nazwie moda, więc „skyui" znajduje
`SkyUI_5_2_SE-12604-5-2SE.7z`), sortuj po najnowszych, nazwie, rozmiarze albo
stanie i **ukryj** archiwum, z którym skończyłeś - co zachowuje plik i zdejmuje
jedynie wiersz, bo odłożenie książki to nie spalenie jej. „Show hidden" przywraca
je, a ten sam przycisk odkrywa. „Remove N installed" usuwa archiwa modów, które
już zainstalowałeś, w dwóch kliknięciach, i tylko te **na ekranie**: filtrem
powiedziałeś, o które ci chodziło.

### Kolekcje Nexusa

Wklej odnośnik do kolekcji - albo kliknij taki na stronie - a Eidos wypisze
członków tej rewizji, każdego zestawionego z tą instancją: zainstalowany,
pobrany albo brakujący. **Czyta** kolekcję; nie instaluje jej i panel to mówi.
Cztery rzeczy czynią tu instalator nieuczciwym, a nie tylko trudnym: członkowie
to zwykłe pliki Nexusa wymagające klucza na plik, który poza własnym przyciskiem
serwisu potrafi wybić tylko konto premium; pełna instalacja to trzy wywołania API
na członka wobec budżetu, którego ten klient nie chce przekraczać; faz, reguł i
odtwarzanych odpowiedzi FOMOD z manifestu nie dało się zweryfikować wobec
prawdziwej opublikowanej kolekcji Bethesdy, a zgadywanie daje kolejność
wczytywania, która wygląda dobrze i dobra nie jest. Czytanie kosztuje jedno
żądanie i jest dokładne.

Kolekcję da się odczytać tylko wobec **jej własnej gry**. Otwórz kolekcję do
Skyrima przy wczytanej instancji Fallouta 4, a odmówi z nazwy, zamiast zestawiać
członków z niewłaściwą listą modów, gdzie każde „zainstalowany" i każde
„brakujący" byłoby szumem w kształcie odpowiedzi.

### Tryb offline

**Settings -> Nexus -> Offline** całkiem powstrzymuje Eidos przed kontaktem z
Nexusem. Sprawdzanie aktualizacji, logowanie, pobierania i kolekcje mówią o tym,
zamiast padać z błędem połączenia. Jest wyłączony, dopóki go nie włączysz - plik
ustawień zapisany przez starszy Eidos nie ma takiego klucza, a odczytanie
brakującego jako „włączony" odcięłoby sieć każdemu, kto aktualizuje.

**Preferred servers** porządkuje węzły CDN, które pobieranie przedkłada,
najlepsze pierwsze. Tylko konto premium dostaje kiedykolwiek więcej niż jedno
lustro do wyboru, więc dla wszystkich pozostałych wybiera Nexus i to nic nie
zmienia. To uporządkowanie, nie filtr: jeśli nic z wymienionych przez ciebie nie
jest dziś w ofercie, pobieranie i tak się odbywa, z tego węzła, który Nexus podał
pierwszy.

**Categories** są edytowalne, nie tylko wyświetlane: przypisz je do jednego moda
albo do całego zaznaczenia, edytuj sam katalog z tego samego okna i ściągnij
oficjalną listę kategorii gry z Nexusa. Oba pliki katalogu są własnymi plikami
MO2 (`categories.dat` i `nexuscatmap.dat`), więc współdzielona instancja trzyma
jeden katalog.

**View -> INI editor** edytuje pliki INI gry z profilu - tę kopię, która
przetrwa, a nie tę zakopaną w prefiksie Protona, nadpisywaną przy każdym
uruchomieniu. **View -> Log** czyta logi sesji. **View -> Extensions** wypisuje
twoje własne dodatki; zob. [extensions.pl.md](extensions.md).

Instalowanie przyjmuje wszystko: ścieżki Simple i FOMOD, plus paczki **BAIN**
Wrye Bash (zaznaczasz podpaczki, które scalają się po kolei) i **ręczny** wybór,
który pokazuje drzewo archiwum i pozwala wskazać katalog danych, gdy żadna
heurystyka nie rozpoznaje układu. Żadne archiwum nie jest odrzucane.

**Diagnostics** przeprowadza kontrole kondycji na żywo: przede wszystkim zdolność
do uruchomienia, brakujące pliki master (najbardziej niezawodny pojedynczy
zwiastun crasha), archiwa, których żadna aktywna wtyczka nie wczyta, czy lista
modów wciąż zgadza się z folderem modów i - po przebiegu - co własny log script
extendera mówi o każdej z jego wtyczek DLL, co zamienia „czy moje wtyczki SKSE
się wczytały?" z domysłu w dowód.

Żeby uruchomić grę przez GUI, ustaw opcję uruchamiania gry w Steamie na ścieżkę
bezwzględną pliku wykonywalnego (Steam nie widzi `~/.cargo/bin` w PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos otwiera się na instancji tej gry - tej, której użyłeś ostatnio, więc
instancja przenośna zostaje odnaleziona tak samo jak globalna; kliknij Run, by
uruchomić ją przez scalony widok. (Przycisk Run pokazuje dokładnie ten wiersz, z
prawdziwą ścieżką działającego pliku wykonywalnego, jeśli naciśniesz go poza
Steamem.)

`%command%` Steama dla tytułów Bethesdy zwykle wskazuje na
`<Game>Launcher.exe`. Eidos nigdy go nie uruchamia: launcher to osobna aplikacja
ustawień, która ponownie skanuje `Data` i przepisuje `plugins.txt`, cofając
dopiero co wdrożoną kolejność wczytywania. Podstawia loader script extendera,
jeśli jakiś jest zainstalowany, a w przeciwnym razie plik wykonywalny gry, i mówi
o tym, gdy musi sięgnąć po zapasowe wyjście - gra, która startuje z każdym modem
SKSE bezczynnym, jest gorsza niż taka, która nie startuje.

Dawniejsze instrukcje wymuszały tutaj `WINEDLLOVERRIDES="d3dcompiler_47=n"`. Nie
jest to już potrzebne i nigdy nie było całkiem trafne: nadpisanie na *native*
pomaga tylko wtedy, gdy prawdziwy `d3dcompiler_47.dll` już jest w prefiksie.
Eidos teraz skanuje importy DLL włączonych modów, sam wdraża prawdziwy plik DLL
Microsoftu i dopiero potem ustawia nadpisanie.

## Wypróbuj proof of concept

Gra nie jest potrzebna. Dowodzi unii + copy-on-write + zera dotknięć + zasięgu na
przestrzeń nazw, używając jedynie nieuprzywilejowanego OverlayFS w przestrzeni
nazw użytkownika (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Narzędzia

xEdit, BodySlide, DynDOLOD i spółka działają przez scalony widok wewnątrz
prefiksu Protona gry:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # czego potrzebują zarejestrowane narzędzia i jaki jest stan
eidos prereqs skyrimse --install  # pobrać to, czego brakuje
```

Jedna rzecz do wiedzenia przed nazwaniem narzędzia: **tytuł decyduje o tym, które
biblioteki DLL środowiska uruchomieniowego Eidos mu dostarcza** - `BodySlide`
dostaje swoje biblioteki DirectX, `BS` nie dostaje nic. W GUI okno Executables
pokazuje pod polem prawdziwy stan każdego wymagania, a brakujące są przyciskami.

Tabela, trzy poziomy wymagań, dlaczego DynDOLOD potrzebuje środowiska .NET,
którego winetricks nie potrafi zainstalować, i dlaczego narzędzie zainstalowane
jako mod uruchamia się ze scalonej ścieżki, a nie z własnego folderu, są w
[tools.pl.md](tools.md).

Budowanie ze źródeł i układ repozytorium są w
[../internals/contributing.md](../../../../internals/contributing.md).

## Rozszerzenia

Eidos da się rozszerzyć bez przebudowy: manifest TOML w
`~/.config/Colony/Eidos/addons/` dodaje narzędzie do listy Extensions albo
kontrolę do zakładki Health. Nic nie jest wczytywane do Eidos - rozszerzenie to
program, który on uruchamia. Zob. [extensions.pl.md](extensions.md).
