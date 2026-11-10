<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# Rozwiązywanie problemów i diagnostyka

Przełączniki środowiskowe, odczyt liczników operacji, karty Health w zakładce
Diagnostics, szukanie moda stojącego za awarią, co dołączyć do zgłoszenia błędu
oraz znane problemy wraz z ich historią.

## Diagnozowanie VFS

Dwie zmienne środowiskowe obejmują większość przypadków, w których gra widzi coś,
z czym system plików się nie zgadza:

```sh
EIDOS_FUSE_STATS=1                  # liczniki operacji, zrzucane przy odmontowaniu
EIDOS_FUSE_NO_CACHE=1               # każdy cache po stronie jądra wyłączony
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # albo nazwij je pojedynczo
```

To właśnie postać szczegółowa znalazła awarię `FOPEN_KEEP_CACHE` opisaną w
Znanych problemach: wyłączenie wszystkich czterech odpowiada na pytanie „czy to
cache?", a dopiero nazwanie ich odpowiada „który". Liczniki odpowiadają na drugą
połowę. Przy włączonym passthrough (domyślnie jest wyłączony) wczytywanie
pokazujące `read 0` to takie, w którym `FUSE_PASSTHROUGH` podał każdy bajt w
jądrze, więc wszystko, co zamierzałeś stroić na ścieżce odczytu, jest już
darmowe.

Kilka dalszych przełączników zawęża usterkę jeszcze bardziej:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

Przy uruchomieniu ze Steama wstaw je przed poleceniem w opcji uruchamiania, na
przykład `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. Wyjście trafia do
dziennika uruchomienia: do `logs/run-*.log` instancji przy uruchomieniu z okna, do
konsoli Steama przy uruchomieniu przez Steam.

Żeby obejrzeć sam scalony widok, `eidos shell <instance>` otwiera powłokę w
folderze gry takim, jakim widzi go gra, a
`eidos shell <instance> -- ls Data/meshes` uruchamia tam jedno polecenie; to, co
zostanie zapisane, ląduje w Overwrite
([usage.md](usage.md#powłoka-w-scalonym-widoku)). Przycisk Save tree na karcie
Data zapisuje całe scalone drzewo `Data` do pliku tekstowego, po jednym pliku w
wierszu, z modem, z którego pochodzi, z ukrytym folderem domowym.

## Zamontować unię ręcznie

`eidos-fuse` jest w pakiecie Arch i w kompilacjach ze źródeł: `just build` go
buduje, a `just install` lub `install.sh` kopiuje go, gdy jest. Pakiet wydania i
`eidos-linux` go nie zawierają. Jego wiersz użycia:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

Pierwsza `--layer` wygrywa przy konflikcie; ostatnia to twoje nietknięte dane gry.
Montowanie potrzebuje tylko `/dev/fuse` i `fusermount3` (bez overlayfs, bez Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... czytaj i pisz przez /mnt/point ...
fusermount3 -u /mnt/point
```

Zapisy lądują w `--overwrite <dir>` (katalog tymczasowy, gdy pominięty), więc same
warstwy pozostają nietknięte nawet tutaj.

## Dlaczego passthrough jest domyślnie wyłączony

Passthrough przekazuje jądru prawdziwy plik bazowy, dzięki czemu odczyty
całkowicie omijają tego demona. Pomiar A/B poniżej wykonano z wcześniejszą
wersją demona, przed zmianą opisaną niżej w znanym problemie „Passthrough FUSE
nie pozwala grze wczytać żadnej zawartości modów”. Ta zmiana omija najbardziej
prawdopodobną przyczynę, ale nie zmierzono jej jeszcze ponownie w grze, więc
passthrough pozostaje domyślnie wyłączony, dopóki to nie nastąpi. Zmierzone A/B
na Skyrim SE 1.6.1170, proton-cachyos 11.0, jądro 7.1.4, ta sama kolejność
wczytywania 82 wtyczek, przy jedynej zmiennej: czy plik binarny nosił tę
zdolność:

| passthrough | niepowodzenia `NtCreateFile` z `STATUS_ACCESS_VIOLATION` |
|-------------|----------------------------------------------------------|
| włączony    | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`         |
| wyłączony   | 0                                                        |

Przy włączonym gra nie otwierała żadnego z własnych archiwów ani wtyczek, co w
grze objawiało się jako mody, których nie było - bez błędu, bez wiersza w
dzienniku. Przy wyłączonym ta sama kolejność wczytywania docierała do rozgrywki
z żywymi wtyczkami, archiwami i skryptami Papyrus.

Awaria była niewidoczna z wnętrza demona i to właśnie uczyniło jej znalezienie
kosztownym: jego własne `open` udawało się za każdym razem, a jądro nigdy nie
odmawiało pliku bazowego (sprawdzone na całej nieudanej sesji z
`EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero `passthrough refused`). Błąd
powstawał po tym, jak demon odpowiedział `opened_passthrough`, więc żadne
logowanie po stronie demona nie mogło go zobaczyć. Nie zależał też od
rozszerzenia - uderzał tak samo w archiwa i wtyczki, czyli w pliki, które gra
trzyma otwarte przez cały czas działania.

`EIDOS_FUSE_PASSTHROUGH=1` włącza go z powrotem, by zmierzyć, co daje, albo by
ponownie przetestować mechanizm. Ostrzeżenia o zdolności w launcherze i w zakładce
Diagnostics pojawiają się wyłącznie wtedy, gdy o nią poprosiłeś.

## Czy indeks warstw jest naprawdę używany?

`LayerStack::new` buduje albo kompletną mapę warstw tylko do odczytu, albo
żadnej. Bez mapy każde zapytanie przechodzi warstwy tak jak wcześniej: odpowiedzi
są te same, wczytanie zapisu trwa mniej więcej trzy razy dłużej, a poza tym nic
nie wygląda inaczej.

Gdy indeks jest wyłączony, montowanie podaje powód, raz, przy starcie:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

Powód wskazuje nieczytelny katalog, warstwę głębszą niż 64 katalogi, ponad
4 000 000 wpisów albo `EIDOS_NO_INDEX`. Wiersz trafia do dziennika uruchomienia:
do `logs/run-*.log` instancji przy uruchomieniu z okna, do konsoli Steama przy
uruchomieniu przez Steam. Z `EIDOS_FUSE_STATS=1` zrzut przy odmontowaniu ma też
wiersz `layer index:`, a niezerowy licznik `unindexed` na zwykłym montowaniu
oznacza, że indeks był wyłączony.

Poniższe przykłady sprawdzają to samo offline, na folderach instancji, bez
montowania:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` rozwiązuje prawdziwe ścieżki z indeksem i bez niego oraz porównuje
skanowania katalogów. `index_agrees` sprawdza, czy oba odpowiadają TO SAMO, na każdej
ścieżce i każdym listowaniu prawdziwej instancji. `listing_cost` mierzy, ile mapa
scalonych dzieci oszczędza przy `readdir`.

`EIDOS_NO_INDEX=1` wymusza przechodzenie - na wypadek, gdy to właśnie różnica między
dwiema odpowiedziami jest przedmiotem debugowania.

## Uruchamianie gry przez CLI

Aby uruchomić grę przez CLI `eidos`, a nie przez okno, ustaw jej opcję
uruchamiania w Steamie na bezwzględną ścieżkę `eidos`, na przykład:

```
~/.local/bin/eidos play skyrimse -- %command%
```

Prefiks `WINEDLLOVERRIDES` nie jest potrzebny. Gdy włączony mod importuje
`d3dcompiler_47`, Eidos wdraża swoją dołączoną kopię do prefiksu, wymusza ją jako
natywną i scala to z dowolnymi nadpisaniami DLL, które dostarcza mod (ładowarki
ENB/ReShade/`.asi`).

Eidos nigdy sam nie uruchamia gry przez Proton.
`eidos tool <instance> run <title>` z wpisem, który uruchamia grę (loader script
extendera, launcher albo plik wykonywalny gry), prosi sklep, do którego należy
kopia, o jej uruchomienie, tak jak Run w oknie, którego nie otworzył żaden sklep,
i wypisuje, co ustawić, gdy ustawienie sklepu nie uruchamia Eidosa. `--print`
wypisuje polecenie sklepu zamiast je wykonać.

## Gra lub narzędzie, które się nie zamyka

Gdy gra lub narzędzie działa, przycisk Run zmienia się w Stop, a karta blokady
też ma przycisk Stop. Naciśnij go dwa razy. Drugie naciśnięcie kończy każdy
program uruchomiony przez to uruchomienie: najpierw SIGTERM, a po 10 sekundach
SIGKILL dla tego, co zostało. To, czego gra nie zapisała, przepada. Potem Eidos
robi to samo co po każdym wyjściu: zmiany w INI i wtyczkach wracają do profilu,
wyjście narzędzi zostaje zapisane, a mody są odmontowywane. Uruchomienie jest
zapisywane z kodem wyjścia 143, cokolwiek zwrócił program, więc
`eidos sessions <instance> diff` nigdy nie bierze go za ostatnie dobre
uruchomienie. Unlock to co innego: oddaje okno i zostawia program uruchomiony. W
terminalu `kill <pid>` na procesie `eidos` (pid jest na karcie blokady) działa
tak samo jak Stop.

## Szukanie, co się zepsuło

Aby znaleźć zmianę, która zepsuła grę:

- `eidos sessions <instance> diff` wypisuje, co zmieniło się od ostatniego
  uruchomienia gry, które zakończyło się kodem 0: mody, wersje, kolejność
  wtyczek, INI i plik wykonywalny gry
  ([usage.md](usage.md#co-zmieniło-się-od-ostatniego-dobrego-uruchomienia)).
- Test run, obok Run, gra z modami profilu, bez modów albo z zaznaczonymi modami
  i potrzebnymi im masterami, a wszystko, co gra zapisuje, kieruje do
  `<instance>/test-run/`. Po uruchomieniu zachowujesz lub wyrzucasz każdą grupę
  plików: zapisy, zmiany INI, stan wtyczek, wynik w Overwrite i logi
  ([usage.md](usage.md#uruchomienia-testowe)).
- Find the mod behind a problem, w zakładce Diagnostics, uruchamia testowe
  uruchomienia z połową pozostałych podejrzanych za każdym razem i pyta, czy
  problem wystąpił. 200 podejrzanych to około 8 uruchomień, a wyszukiwanie
  przeżywa zamknięcie Eidosa
  ([usage.md](usage.md#szukanie-moda-stojącego-za-problemem)).

Zatrzymane uruchomienie nigdy nie wypełnia w tym wyszukiwaniu odpowiedzi Yes,
bo jest zapisane z kodem wyjścia 143 (zob. wyżej).

## Karty Health

Zakładka Diagnostics pokazuje kartę Health dla każdego znalezionego problemu, z
jego przyczyną i, dla niektórych, z przyciskiem, który go naprawia. Karty opcji
uruchamiania są opisane w [install.md](install.md), karty NVAPI, pełnego ekranu i
ENB w [graphics.md](graphics.md), a karty Protona i prefiksu w następnej sekcji.
`eidos check <instance>` wypisuje ustalenia, które nie potrzebują okna
([usage.md](usage.md#kontrole-health-z-terminala) je wymienia), w tym karty
prefiksu, rejestru i dysku `S:`. Tylko okno pokazuje kartę Proton z jej
wariantami Flatpak, nieznanego i starszego buildu, kolejność łatek, skrypty
extendera, archiwa, których żadna wtyczka nie wczytuje, wymagania z Nexusa,
nieaktualne kopie wtyczek i kartę Display.

### System

- „vm.max_map_count is <value>” pojawia się poniżej 1048576, wartości domyślnej
  w Arch Linux, Fedorze 39 i nowszych oraz Ubuntu 24.04 i nowszych. Niektóre gry
  pod Protonem mapują więcej regionów pamięci i wtedy wywalają się bez błędu.
  Wpisz `vm.max_map_count = 1048576` do pliku w `/etc/sysctl.d`, na przykład
  `80-gamecompatibility.conf`, a potem uruchom `sudo sysctl --system`.
- „The open files limit is <value>” pojawia się, gdy twardy limit jest poniżej
  524288, którego potrzebuje esync Protona; poniżej niego gry mogą się nie
  uruchomić albo wywalić. systemd 240 i nowszy go ustawia. W starszej
  konfiguracji ustaw `DefaultLimitNOFILE=524288` w `/etc/systemd/system.conf` i
  `/etc/systemd/user.conf`, a potem zaloguj się ponownie.
- „Game files on a Windows file system” wskazuje folder gry, prefiks albo
  katalog główny instancji, gdy leży na `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g,
  exfat-fuse), `exfat` lub `vfat`. Proton potrzebuje dowiązań symbolicznych,
  uprawnień uniksowych i nazw rozróżniających wielkość liter. Przenieś grę, jej
  prefiks i instancję na linuksowy system plików, taki jak ext4, btrfs lub xfs.

### Pliki gry

Karta Game files na górze zakładki pokazuje, czy instancja uruchamia instalację
ze sklepu, czy kopię gry wewnątrz instancji
([usage.md](usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia)). Uruchomienia są
odrzucane, z kartą mówiącą dlaczego, w trzech przypadkach:

- „The game copy '<folder>' is missing”: instancja uruchamia kopię, której już
  nie ma. Przywróć ją albo wróć do wersji ze sklepu.
- „'<folder>' has no <binary>”: folder, z którego działa instancja, nie zawiera
  pliku wykonywalnego gry, więc nie jest kopią gry.
- „<game> cannot run from a copy yet”: Morrowind oraz Oblivion z
  `bUseMyGamesDirectory=0` trzymają listę wtyczek w folderze gry.

„ModOrganizer.ini points tools at another game folder” pojawia się, gdy
własny `ModOrganizer.ini` listy, niezapisany przez Eidosa, ma `gamePath` inną niż
pliki gry, z których działa instancja. Narzędzia w trybie Mod Organizer 2, takie
jak PGPatcher, czytają grę stamtąd. Zmień `gamePath` w tym pliku albo usuń plik,
żeby Eidos zapisał własny.

### Kolejność wczytywania

- „sTestFile lines make the game ignore plugins.txt” pojawia się dla Fallouta 4,
  Fallouta 4 VR i Starfield, gdy `Fallout4Custom.ini` lub `StarfieldCustom.ini`
  profilu ma wiersze od `sTestFile1` do `sTestFile10` w `[General]`. Gdy któryś
  jest ustawiony, gra wczytuje te wtyczki, a kolejność wczytywania zapisana
  przez Eidosa nie ma żadnego skutku. Fix usuwa wiersze z kopii profilu i
  zachowuje poprzedni plik jako `<ini>.pre-fix`.
- Starfield: włączone wtyczki `.esp` bez flagi mastera dostają kartę, bo
  Starfield nie potrafi ich sortować wśród `.esm` i wtyczek z flagą mastera i
  trzyma część ich rekordów wczytaną przez całą sesję. Wydanie `.esm` od autora
  omija oba problemy.
- Starfield: „Invalid blueprint plugins” wymienia wtyczki, które mają część, ale
  nie wszystkie z flagi blueprint, nazwy zaczynającej się od `BlueprintShips-` i
  rozszerzenia `.esm`. „Blueprint plugins without their main plugin” wymienia te,
  których głównej wtyczki nie włączono: `BlueprintShips-Example.esm` potrzebuje
  `Example.esm`, `.esp` lub `.esl`.
- „<patch> loses <n> file(s) to <master>” pojawia się, gdy wtyczka w jednym modzie
  potrzebuje mastera z innego moda, a ten inny mod ma wyższy priorytet, więc jego
  kopie plików, które dostarczają oba, wygrywają. Łatka zwykle idzie poniżej moda,
  który łata, a przycisk Move na karcie ją tam umieszcza. Zostaw kolejność, jeśli
  łatka ma te pliki przegrywać.
- „Out-of-date copy of <plugin>” pojawia się, gdy mod zawiera oczyszczoną lub
  oznaczoną jako light kopię wtyczki, której oryginał zmienił się od tamtej pory
  (aktualizacja moda lub gry). Gra wczytuje starą kopię. Ukryj ją, a potem
  oczyść lub oznacz nową wtyczkę ponownie.

### Lista modów

- „Order rules of the collection <name> that no longer hold” pojawia się, gdy
  mody zainstalowanej kolekcji nie leżą już w kolejności, jakiej żądają jej
  reguły. Restore collection order przenosi z powrotem tylko mody tej kolekcji, a
  Ctrl+Z to cofa.
- „<mod> requires <mod>, which is not installed” (albo „which is disabled”)
  pochodzi z wymagań z Nexusa, które czyta Check for updates. Open on Nexus
  otwiera stronę wymaganego moda, Download main file pobiera go na koncie
  premium, a Ignore ukrywa wymaganie, które spełnia już inny mod. Wymaganie DLC
  jest tylko radą: Eidos szuka wtyczki o nazwie DLC.
- „fontconfig.txt names a font library that is missing” (silniki Skyrim) wskazuje
  czcionki, których nie ma ani w żadnej warstwie, ani w archiwum, oraz mod, którego
  `Interface/fontconfig.txt` wygrywa. Tekst menu może wyglądać jak kwadraciki,
  dopóki czcionka nie zostanie zainstalowana albo wiersz usunięty.
- „BA2 archive stores audio compressed” (Fallout 4 i Fallout 4 VR) liczy pliki
  `.wav` i `.xwm`, które BA2 moda przechowuje skompresowane. Gra może się
  wywalić, odtwarzając je. Spakuj archiwum ponownie z nieskompresowanym
  dźwiękiem.

### Script extender i aktualizacje gry

- „<loader> is not installed” pojawia się, gdy wtyczki script extendera są
  zainstalowane (w `SKSE/Plugins`, `F4SE/Plugins` i tak dalej), ale loadera nie
  ma w żadnym włączonym modzie ani w folderze gry. Gra startuje bez nich.
- „The script extender does not match game version <version>” pojawia się, gdy
  loader jest zainstalowany, ale nie ma DLL runtime dla wersji uruchamianej gry,
  na przykład `skse64_1_6_1170.dll`. Loader zatrzymuje się z błędem. Zwykle
  następuje to po aktualizacji gry. Ta karta i poprzednia mają przycisk Open the
  download page dla strony extendera oraz przycisk Install the downloaded
  archive, który umieszcza loader w folderze `Root/` moda.
- „<game> changed since the last clean launch” pojawia się, gdy plik wykonywalny
  gry, a dla kopii ze Steama jej build id, zmienił się od ostatniego uruchomienia
  przez Eidosa, które zakończyło się bez błędu. Aktualizacja Steama, która
  zostawia plik wykonywalny bez zmian, tego nie wywołuje. Wtyczki script extendera
  zbudowane pod starą wersję mogą przestać się wczytywać, a karty niżej wskazują
  te, które to deklarują. Żeby zachować wersję gry, pod którą zbudowano listę,
  zamroź ją w instancji przez Freeze this version na karcie Game files albo
  przez `eidos stock-game <instance> freeze --yes` po tym, jak samo `freeze`
  pokazało rozmiar: późniejsze aktualizacje sklepu zmieniają instalację ze
  sklepu, a nie kopię.
- „<mod> replaces <n> <extender> script(s)” pojawia się, gdy mod o wyższym
  priorytecie niż script extender ma luźne kopie `.pex` własnych skryptów
  extendera. Stara kopia psuje funkcje, które deklaruje, a gra nie pokazuje
  żadnego błędu. Przenieś extender poniżej tego moda (przycisk Move na karcie
  robi to, gdy oba są modami) albo ukryj te pliki w modzie. Gdy skrypty extendera
  leżą luzem w `Data` gry, zainstaluj extender jako mod, żeby dało się go
  ustawić w kolejności. Archiwa nie są sprawdzane, tylko luźne pliki.

### Wyjście narzędzi

- „Overwrite/Root holds meshes” (albo textures) znaczy, że narzędzie, którego
  ścieżka danych wskazywała folder gry, zapisało tam te foldery, gdzie gra nigdy
  ich nie czyta. Popraw ścieżkę narzędzia, usuń foldery i uruchom narzędzie
  ponownie. Przycisk Open folder otwiera `Overwrite/Root`.
- „BodySlide's GameDataPath is not the game's Data folder” (albo
  `OutputDataPath`): zob.
  [Narzędzia](tools.md#własne-ustawienia-narzędzia-pozostają-jego-własne).

### Awaria w ostatniej sesji

„The game crashed in the last session” pojawia się, gdy program rejestrujący
awarie zapisał log podczas ostatniego uruchomienia. Karta wskazuje wyjątek oraz
te DLL i wtyczki z logu, które instalują twoje mody lub Overwrite. Traktuj je jak
tropy: mod wymieniony w logu awarii jest często miejscem, gdzie awaria się
ujawniła, a nie jej przyczyną. Gdy log nie wskazuje żadnego pliku z twoich modów,
karta to mówi. Open the log go otwiera, a Copy the log (home path hidden) kopiuje
go z ukrytym folderem domowym.

### Rozmiar okna

Dla Skyrim SE, Enderal SE i Fallouta 4 karta Display pokazuje rozmiar i tryb
okna, z jakimi profil uruchamia grę, i mówi „No window size set for this
profile”, gdy nie ustawia ich ani INI Prefs profilu, ani prefiksu. Apply zapisuje
rozmiar i tryb w kopii profilu oraz w kopii INI SSE Display Tweaks w Overwrite,
gdy ten mod ustawia okno ([usage.md](usage.md#diagnostics)).

## Który Proton uruchamia grę, i zepsuty prefiks

Zakładka Diagnostics ma kartę Proton: build, którego Eidos i Steam używają dla
gry, własne ustawienie Steama i build, który ostatnio uruchomił prefiks. Gdy
prefiks ostatnio przygotował nowszy Proton niż wybrany, karta staje się poradą,
bo starszy Proton na nowszym prefiksie może zresetować jego pliki Wine albo się
nie uruchomić. Wybierz build co najmniej tak samo nowy we Właściwościach gry, w
Zgodności w Steamie, albo odbuduj prefiks.

Pozostałe karty Protona i prefiksu:

- „Proton comes from the Flatpak Steam install”: ten Proton trzyma swój runtime
  i biblioteki steamclient wewnątrz piaskownicy Flatpak, więc uruchomienie go z
  hosta może się nie udać. Eidos nie uruchamia go przez `flatpak run`, co
  umieściłoby grę w piaskownicy, poza scalonym widokiem. Zainstaluj Proton w
  `~/.steam/root/compatibilitytools.d/` i wybierz go dla gry.
- „Eidos cannot tell which Proton runs this game”: `config.vdf` Steama nie
  wskazuje dla niej żadnego narzędzia zgodności, a prefiks nie zapisuje żadnego
  zainstalowanego buildu. Narzędzia nie startują, dopóki nie wybierzesz jednego
  w Steamie (Właściwości gry, Zgodność).
- „<game> has no Proton prefix yet”: uruchom grę raz przez Steama, potem naciśnij
  F5. Jeśli Steam nie tworzy prefiksu, sprawdź, czy folder gry w
  `steamapps/compatdata` jest zapisywalny; pusty, należący do roota, blokuje to
  bez komunikatu.
- „Windows tools cannot find <game>”: zapis ścieżki instalacji w prefiksie jest
  błędny lub go brakuje, a czytają go xEdit, DynDOLOD, Wrye Bash i Creation Kit.
  Eidos zapisuje właściwą ścieżkę przy następnym uruchomieniu narzędzia.
- „The prefix's S: drive points outside the Steam library”: `S:` nie pasuje ani
  do korzenia biblioteki, na który Eidos kieruje go dla narzędzia, ani do
  `<library>/steamapps`, na który Steam kieruje go dla gry. Biblioteka
  prawdopodobnie się przeniosła albo prefiks pochodzi z innego komputera. Każda
  ścieżka względem `S:` zapisana w prefiksie nie prowadzi nigdzie, dopóki
  następne uruchomienie przez Eidosa lub Steama nie ustawi go ponownie.
- „<store> installation: started through its store” to karta kopii GOG lub Epic.
  Run prosi Heroic lub Legendary o uruchomienie gry, gdy Eidos jest tam
  wrapperem gry, a narzędzia działają z buildem Wine lub Protona, który Heroic
  zapisuje dla kopii.

Menu File uruchamia winecfg, regedit, eksplorator plików Wine i deinstalator w
prefiksie gry, z Protonem, który uruchamia narzędzia gry
([tools.md](tools.md#własne-programy-wine-w-prefiksie-gry)). Ich wiersze
pozostają wyszarzone, dopóki prefiks nie ma tych programów.

Przycisk Rebuild prefix na karcie Proton służy do prefiksu, który już nie działa.
Naciśnij go dwa razy. Eidos niczego nie usuwa: zmienia nazwę
`steamapps/compatdata/<appid>` na `<appid>.eidos-backup-<date>-<time>` w tym
samym folderze, a Steam tworzy nowy prefiks przy następnym uruchomieniu gry ze
Steama. Odmawia, dopóki gra, narzędzie albo dowolny program Wine używa prefiksu.
Dopóki nowy prefiks nie istnieje, Run w oknie nie może przekazać gry Steamowi i
pokazuje zamiast tego opcję uruchamiania, więc naciśnij Graj w Steamie. Zapisy i
ustawienia trzymane tylko w starym prefiksie zostają w folderze o zmienionej
nazwie: skopiuj z powrotem to, czego potrzebujesz, i sam usuń ten folder, gdy nie
będzie ci już potrzebny.

## Co dołączyć do zgłoszenia błędu

- Copy report, w zakładce Diagnostics, kopiuje wersję Eidosa, grę, build Protona,
  informację, czy Steam jest we Flatpaku, stan passthrough, typ sesji,
  dystrybucję, jądro, `vm.max_map_count`, limit otwartych plików, informację, czy
  są `/dev/fuse` i `fusermount3`, nazwy zmiennych uruchomieniowych i programów
  wrapperów profilu oraz każdą kartę Health. Twój folder domowy jest pokazany
  jako `~`, a login jako `<user>`. `eidos check <instance>` wypisuje ten sam
  nagłówek w terminalu, a `--json` daje go jako JSON.
- Dziennik uruchomienia, `<instance>/logs/run-*.log`, zaczyna się od dokładnego
  polecenia, które uruchomił Eidos. File, Eidos logs otwiera folder własnych
  logów Eidosa, `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), na karcie awarii, kopiuje log awarii.
- Save tree, na karcie Data, zapisuje scalone drzewo `Data` ze źródłem każdego
  pliku.

## Znane problemy

### DLSS albo generowanie klatek po cichu nic nie robi

Trzy odrębne przyczyny, każda bez jakiegokolwiek komunikatu błędu: niewłączone NVAPI
w opcjach uruchamiania, pełny ekran wyłączny albo przeterminowany limit FPS Reflexa.
Cała lista kontrolna jest w [graphics.md](graphics.md).

### Run nic nie robi z Flatpak Steam

W oknie, którego nie otworzył żaden sklep, Run prosi Steama o uruchomienie gry,
wykonując `steam -applaunch <appid>`. Flatpak Steam nie dodaje żadnego `steam` do
`PATH` hosta, więc wiersz stanu mówi „Could not start Steam” i podaje błąd.
Naciśnij zamiast tego Graj w Steamie.

### Zapis z innego urządzenia się nie pokazuje

Zapis, który trafia do prefiksu z pominięciem Eidosa, na przykład pobranie ze
Steam Cloud z Steam Decka, pozostaje ukryty, dopóki zapisy profilu są
zamontowane. Nic go nie usuwa. Karta Saves zlicza te zapisy pod swoim
nagłówkiem, a Import kopiuje je do profilu, nie zastępując żadnego zapisu profilu
([usage.md](usage.md#zapisy-należą-do-profilu)).

### Mod, który zapisuje jeden katalog na dwa sposoby, tracił wszystko pod drugim

Naprawione. ext4 rozróżnia `meshes/` i `Meshes/`; scalony widok rozróżniać nie może, a
prawdziwe mody dostarczają obie postaci - XP32 Maximum Skeleton ma swoje animacje i
plik zachowań FNIS pod wersją z wielką literą, a `character assets` pod drugą.

Resolver brał dokładne dopasowanie wielkości liter dla każdego składnika ścieżki i
trzymał się go: wchodził do `meshes/`, nie znajdował tam reszty ścieżki i porzucał
CAŁĄ WARSTWĘ. Każdy plik pod drugą pisownią był dla gry niewidzialny - bez błędu, bez
dziennika, bez niczego w jakiejkolwiek diagnostyce. Na prawdziwej instancji z 50
warstwami było to 74 pliki.

Pasujący składnik jest teraz kandydatem, a nie decyzją; dokładna wielkość liter wciąż
próbowana jest najpierw, a dopiero gdy reszta pod nią zawiedzie, skan szuka
rodzeństwa równego po złożeniu wielkości liter. Listowania miały tę samą wadę o
katalog wyżej i teraz czytają w każdej warstwie każdy taki równoważny katalog.

Indeks ścieżek nigdy nie miał tego błędu, bo przechodzi każdy katalog, jaki znajdzie.
Przez pewien czas indeks zwracał więc pliki, których nie potrafił znaleźć
mechanizm zapasowy, choć to właśnie zapasowy miał być odpowiedzią nigdy
niezawodzącą.

### LODGen z DynDOLOD-a umiera, zostawiając pusty dziennik

Naprawione przez `dotnet10`; zob. [tools.md](tools.md). Dla każdego świata
`LODGen_SSE_<world>_log.txt` zawiera baner wersji, wiersz `.NET Version:` i nic
więcej, a okno mówi tylko „failed to generate object LOD for one or more
worlds". Przyczyną jest Mono z Wine odpowiadające za .NET Framework i żadna
ilość zainstalowanego .NET Framework tego nie naprawia - Proton przy każdej
aktualizacji prefiksu zastępuje `mscoree.dll` dowiązaniem do własnego drzewa.
Pakiet wydania zawiera runtime .NET 10; pozostałe instalacje pobierają go raz.

### Wine nie potrafiło stwierdzić, że montowanie składa wielkość liter

Naprawione.

`get_dir_case_sensitivity` w Wine najpierw pyta katalog przez `FS_IOC_GETFLAGS`,
potem węszy za znacznikiem, który CIOPFS zostawia w obsługiwanych przez siebie
katalogach. Gdy brak obu, Wine zakłada WRAŻLIWOŚĆ na wielkość liter, a każde
wyszukanie, którego pisownia nie zgadza się bajt w bajt, cofa się do
przeczytania CAŁEGO katalogu w poszukiwaniu dopasowania bez względu na wielkość
liter. Gry Bethesdy proszą o `data/ccbgssse001-fish.bsa`, podczas gdy plik
nazywa się `ccBGSSSE001-Fish.bsa`, więc uruchamiało się to przy niemal każdym
zasobie: 4471 sondowań znacznika i 2236 pełnych ponownych odczytów katalogu w
osiem sekund oraz 195796 wyliczeń `Data` w dziewięćdziesiąt. Skyrim SE nigdy nie
dochodził do menu głównego - siedział na 240 MB rezydentnych, podczas gdy demon
spalał 92 % rdzenia.

Eidos składał wielkość liter w `resolve_read` od samego początku. Cały koszt
brał się z tego, że nigdy tego nie powiedział. `lookup` odpowiada teraz
`.ciopfs`; `readdir` nadal go nie wypisuje. `ioctl` odpowiada też na
`FS_IOC_GETFLAGS` dla katalogów flagą casefold, o którą obecny Wine pyta przed
szukaniem znacznika.

Dwie rzeczy uczyniły to śmiertelnym, a nie tylko powolnym. Koszt rośnie z rozmiarem
katalogu, więc zainstalowanie zawartości Anniversary (`Data` z 37 plików do 177)
przeważyło szalę. A `opendir` zachłannie budował scaloną listę, co jest czystym
marnotrawstwem, gdy Wine otwiera katalog tylko po to, by zrobić `stat` na tym
znaczniku w środku - migawka jest teraz robiona przy pierwszym `readdir`.

Po: menu główne, 2,1 GB rezydentnych, demon przy 0 % CPU.

`EIDOS_FUSE_TRACE=opendir` to właśnie to, co go znalazło, i jest dostarczane.
Liczniki operacji mówią ile; 195796 wyliczeń jednego katalogu jest w sumie
niewidoczne.

### Gra przepisała `plugins.txt` na pusty

Bardzo prawdopodobnie to samo co składanie wielkości liter wyżej - `Data`, którego
gra nie mogła wyliczyć w rozsądnym czasie, więc wywnioskowała, że nic tam nie ma,
i to zapisała. Nieudowodnione i warte ponownego sprawdzenia. Tak czy inaczej,
zabezpieczenie przechwytywania (przechwycenie, które całkowicie czyści zbiór
aktywnych, jest odrzucane przy każdym rozmiarze) sprawia, że nie może to już
uszkodzić profilu.

### `FOPEN_KEEP_CACHE` jest wyłączony

Naprawione. Wywalał Skyrim SE na wyłuskaniu wskaźnika zerowego kilka sekund po
menu głównym, deterministycznie, przy zerze zainstalowanych modów; pozostałe trzy
cache po stronie jądra zostały wyeliminowane pojedynczo przez bisekcję i tylko
ten miał znaczenie. Ta bisekcja szła przy włączonym `FUSE_PASSTHROUGH`, a jądro
kończy błędem EIO otwarcie przez passthrough, które niesie `FOPEN_KEEP_CACHE`,
więc z tą flagą każde otwarcie przez passthrough się nie udawało. Być może to te
nieudane otwarcia wywalały grę, a samej flagi, przy wyłączonym passthrough,
nigdy nie przetestowano. Odpowiedzi przez passthrough już jej nie niosą. Jego
utratę zmierzono wtedy jako darmową, ale przy włączonym passthrough demon
obsługuje *zero* odczytów (`EIDOS_FUSE_STATS` raportował `read 0` przy pełnym
wczytaniu), więc przy passthrough domyślnie wyłączonym rzeczywisty koszt
pozostaje niezmierzony. Włącz go z powrotem przez `EIDOS_FUSE_KEEP_CACHE=1`, by
przetestować go osobno.

### Passthrough FUSE nie pozwala grze wczytać żadnej zawartości modów

Demon omija teraz najbardziej prawdopodobną przyczynę (niżej), ale nie zmierzono
tego jeszcze ponownie w grze, więc passthrough pozostaje domyślnie wyłączony;
`EIDOS_FUSE_PASSTHROUGH=1` go włącza. Przed tą zmianą, przy włączonym
passthrough, Skyrim SE nie potrafił otworzyć 152 własnych plików z
`STATUS_ACCESS_VIOLATION`, więc żadna zawartość modów się nie wczytywała, po
cichu. Pomiar i powód, dla którego własne dzienniki demona pokazywały czysty
przebieg, są wyżej w „Dlaczego passthrough jest domyślnie wyłączony”.

Jądro wiąże z każdym i-węzłem jeden plik bazowy (`fs/fuse/iomode.c`). Otwarcie,
które dla i-węzła mającego już plik bazowy przynosi drugi identyfikator pliku
bazowego, kończy się błędem EIO, podobnie jak zwykłe otwarcie (bez
`FOPEN_PASSTHROUGH`) i-węzła, który ma plik bazowy. Otwarcie przez passthrough
niosące `FOPEN_KEEP_CACHE` również kończy się błędem EIO. Demon rejestrował
wcześniej nowy identyfikator przy każdym otwarciu, więc drugie otwarcie pliku,
który gra już trzymała otwarty, się nie udawało. Wine zgłasza EIO jako
`STATUS_ACCESS_VIOLATION`, czyli błąd stojący za 152 nieudanymi otwarciami. Ta
reguła pochodzi z kodu źródłowego jądra i zgadza się z tymi 152 niepowodzeniami,
co czyni ją najbardziej prawdopodobną przyczyną.

Demon rejestruje teraz jeden plik bazowy na i-węzeł, wspólny dla wszystkich jego
otwarć przez passthrough, i nigdy nie ustawia `FOPEN_KEEP_CACHE` przy otwarciu
przez passthrough. Passthrough obejmuje tylko otwarcia tylko do odczytu plików z
warstw gry i modów, które mają 1 MiB lub więcej albo kończą się na `.dll`,
`.exe` lub `.asi`. Gdy ten plik bazowy jest już zarejestrowany, każde otwarcie
pliku tylko do odczytu go używa, a dopóki plik ma zwykłe otwarcia, nowi
czytelnicy też dostają zwykłe otwarcia. Plik z Overwrite nigdy nie idzie przez
passthrough, a montowanie, które rzutuje znaczniki czasu wtyczek, w ogóle go nie
używa. Otwarcie do zapisu pliku, który trzymają czytelnicy przez passthrough,
dostaje ETXTBSY (w Wine: `STATUS_SHARING_VIOLATION`) przed jakimkolwiek copy-up,
więc otwarcie z `O_TRUNC` nie może zostawić w Overwrite pustej kopii, która
ukryłaby plik moda.

### Przycisk Mod Manager Download nic nie robi

Preferences, Nexus, Downloads pokazuje, który program odbiera linki `nxm://`.
Karta Health pojawia się, gdy tym programem nie jest Eidos, gdy go brakuje albo
gdy żaden nie jest ustawiony. Przycisk Register zapisuje `eidos-nxm.desktop` i
ustawia go jako domyślną obsługę `x-scheme-handler/nxm`. W terminalu to samo robi
`eidos nxm --register`.

Jeśli stan już wskazuje Eidos, a przycisk nadal nic nie robi, przeglądarka może
używać wcześniejszego wyboru. Usuń zapisany wybór dla linków nxm w ustawieniach
przeglądarki, żeby zapytała ponownie. Przeglądarka we Flatpaku nie może
uruchamiać programów hosta i przekazuje link przez portal pulpitu, który czyta
obsługę zarejestrowaną na hoście.
