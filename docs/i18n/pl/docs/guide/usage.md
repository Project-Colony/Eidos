<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Używanie Eidos

Jak używać Eidosa: instancje i kopie gry, wiersz poleceń, uruchomienia testowe,
okno, kolekcje i aktualizacje z Nexusa, przenoszenie instancji na inny komputer,
narzędzia i rozszerzenia. Problemy opisuje
[troubleshooting.md](troubleshooting.md); budowanie ze źródeł i układ repozytorium
opisuje [../internals/contributing.md](../../../../internals/contributing.md).

## Instancje: globalne i przenośne

Polecenia, które przyjmują instancję, przyjmują identyfikator gry albo folder
instancji przenośnej. `skyrimse` nazywa instancję globalną, którą Eidos trzyma w
`~/.local/share/Colony/Eidos/instances/skyrimse`. Instancja przenośna to
samodzielny folder, gdziekolwiek chcesz (drugi dysk, partycja z grami), który
możesz przenosić, tak jak instancje przenośne MO2:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

`eidos-instance.ini` w folderze nazywa grę, więc folder niczego więcej nie
potrzebuje. `EIDOS_INSTANCE=<folder>` w środowisku kieruje identyfikator gry na
ten folder, na przykład w opcji uruchamiania Steama. Instancje przenośne, które
utworzyłeś lub otworzyłeś, są pamiętane, od ostatnio używanej, w
`~/.config/Colony/Eidos/instances.ini`: ekran powitalny okna je wymienia,
uruchomienie ze Steama trafia na tę, w którą ostatnio grałeś, a obsługa `nxm://`
pobiera do niej.

Ręczne przeniesienie folderu przenośnego zachowuje wszystko poza wpisami narzędzi
zarejestrowanymi ze ścieżkami bezwzględnymi do starego miejsca. `eidos pack` i
`eidos unpack` ([niżej](#przenoszenie-instancji-na-inny-komputer)) je naprawiają;
zwykłe `mv` nie. Pamięć podręczna runtime'ów w
`~/.local/share/Colony/Eidos/runtimes/` jest współdzielona przez każdą instancję na
komputerze, więc 78 MB hosta .NET nie jest kopiowane do każdej z nich.

Jedno miejsce jest odrzucane: wnętrze folderu instalacyjnego gry, gdzie
użytkownicy MO2 często umieszczają swoją instancję. Steam jest właścicielem tego
drzewa, a aktualizacja, „verify integrity” albo odinstalowanie mogą je przepisać
lub usunąć razem z twoją konfiguracją. Eidos montuje też nad korzeniem gry, więc
instancja w środku leżałaby wewnątrz własnego celu montowania. Kreator,
`eidos init`, `eidos play` i `eidos shell` to odrzucają. Umieść folder obok gry;
folder-rodzeństwo na tym samym dysku jest tak samo wygodny.

### Gdzie Eidos trzyma swoje pliki

Eidos trzyma własne pliki pod `Colony/Eidos`, w układzie, którego używa każdy
program z rodziny Colony: `~/.config/Colony/Eidos/` na to, co wybrałeś
(preferencje, twoja sesja Nexusa, twoja lista instancji, napisane przez ciebie
definicje gier i dodatków), `~/.local/share/Colony/Eidos/` na instancje globalne
(`instances/`), pobrane runtime'y i logi sesji (`logs/`), a `~/.cache/Colony/Eidos/`
na masterlisty LOOT, które są pobierane ponownie, gdy ich brakuje.

Gdy znajdzie pliki w układzie starszego Eidosa (`~/.config/eidos/`,
`~/.local/share/eidos/` i `~/.local/state/`), pierwsze uruchomienie kopiuje je i
mówi o tym w logu. Instancja globalna jest przenoszona przez dowiązania twarde, a
nie kopię bajt po bajcie, więc jej mody nie zajmują dodatkowego miejsca, a jej
wpisy narzędzi są przekierowywane na nowy folder. Nic w starych katalogach nie
jest zmieniane ani usuwane; usuń je sam, gdy nowy układ zadziała. Jeśli kopii nie
da się zrobić (pełny lub tylko do odczytu dysk, stary folder instancji na innym
dysku, gra nadal z niego uruchomiona), Eidos dalej używa starego miejsca i próbuje
ponownie przy następnym uruchomieniu. Opcja uruchamiania Steama, która nadal nazywa
stary folder instancji, otwiera przeniesiony.

Dowiązania twarde oszczędzają miejsce tylko na początku. Mod lub pobranie, które
później usuniesz lub zainstalujesz ponownie, zachowuje swoją starą kopię na dysku
przez stary folder instancji, a usunięcie `~/.local/share/eidos/<game>/` oddaje to
miejsce.

Instancja przenośna nigdy nie jest przenoszona: zostaje tam, gdzie ją umieścisz.

## Wiersz poleceń

```sh
eidos games                                # supported games installed here, like MO2's list
eidos init skyrimse [folder]               # create an instance; with a folder it is portable
eidos init skyrimse --game-path <folder>   # use the copy installed in <folder> when several are installed
eidos install skyrimse mod.7z [name]       # install an archive (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstall over the mod (--merge installs over its files)
eidos import skyrimse <mo2-profile>        # take over an MO2 profile's mod order and plugin state
eidos sort skyrimse [--dry-run]            # LOOT-sort the load order (--update-masterlist refreshes it first)
eidos play skyrimse                        # show what would be mounted
eidos play skyrimse -- <command>           # run <command> with the mods mounted over the game
eidos shell skyrimse                       # a shell in the merged view; writes land in Overwrite
eidos check skyrimse [--json]              # Health checks without the window
eidos sessions skyrimse                    # the recorded game and tool runs, newest first
eidos sessions skyrimse diff [<id>]        # what changed since the last run that exited with code 0
eidos stock-game skyrimse                  # which game files the instance runs; freeze a copy or switch
eidos tool skyrimse                        # list the tools (add, rm and run: see Tools below)
eidos tool skyrimse run --exe <path>       # run an exe once through the merged view
eidos tool skyrimse import-mo2 [--write]   # the executables of a ModOrganizer.ini; --write adds them
eidos prereqs skyrimse [--install]         # what the tools need; --install fetches what is missing
eidos collection skyrimse <link>           # install a Nexus collection (--dry-run, --no-optional)
eidos nexus status                         # check the stored Nexus sign-in
eidos nexus update skyrimse [--apply]      # check for mod updates; --apply installs them
eidos nxm <link>                           # download a Mod Manager Download link
eidos nxm --register                       # make the site's Mod Manager Download button open Eidos
eidos export skyrimse -o mods.csv --active # MO2's CSV format; --active lists enabled mods only
eidos pack skyrimse backup.eidos           # the whole instance in one file
eidos unpack backup.eidos [folder]         # without a folder a global instance goes back to its central place
eidos --version                            # print the version
```

`eidos` bez argumentów wypisuje listę poleceń. Większość poleceń wypisuje własne
opcje, gdy brakuje argumentu, a `eidos tool --help` wymienia podpolecenia
`tool`. `eidos pack` przyjmuje `--dry-run`, `--level`, `--no-downloads`,
`--no-recovery` i `--force`, a `eidos unpack` przyjmuje `--info` i `--force`; oba
opisano [niżej](#przenoszenie-instancji-na-inny-komputer). `eidos stock-game`
opisano w [Pliki gry](#pliki-gry-instalacja-ze-sklepu-albo-kopia).

### Uruchamianie gry przez scalony widok

`eidos play` montuje mody instancji nad własnym katalogiem `Data` gry wewnątrz
prywatnej przestrzeni nazw, a potem uruchamia polecenie przez ten widok. Własne
pliki gry są najpierw odkładane przez bind, więc demon nadal czyta je bez zmian.
Zapisy (zapisy gry, przegenerowane konfiguracje) lądują w warstwie `overwrite/`
instancji; instalacja gry i każdy folder moda zostają bajt w bajt takie, jakie
były.

Eidos nie potrzebuje roota. Montuje w prywatnej przestrzeni nazw użytkownika i
montowań, bez pomocnika setuid, usługi systemowej i niczego do nadania.
Opcjonalne uprawnienie na pliku `eidos` pozwala na passthrough FUSE, który zostaje
wyłączony, dopóki nie ustawisz `EIDOS_FUSE_PASSTHROUGH=1`; zob.
[install.md](install.md#opcjonalnie-passthrough-fuse). Z tym uprawnieniem Eidos używa
zwykłej przestrzeni nazw montowań zamiast przestrzeni użytkownika, a mody są
wdrażane tak samo w obu.

### Powłoka w scalonym widoku

`eidos shell <instance>` uruchamia twój `$SHELL` (albo `/bin/sh`) w folderze gry
takim, jakim widziałaby go gra: każdy włączony mod zamontowany nad `Data`, mody
Root nad korzeniem gry, a wszystko, co tam zapisane, ląduje w Overwrite.
`eidos shell <instance> -- <command>` uruchamia zamiast tego jedno polecenie, na
przykład `eidos shell skyrimse -- ls Data/meshes` albo natywne narzędzie takie jak
NifSkope. Uruchomienie trzyma blokadę instancji jak sesja gry, więc okno nie może
zapisać zmian, dopóki nie wyjdziesz, i jest zapisane w `eidos sessions` pod nazwą
`shell`, nigdy jako uruchomienie gry. Ctrl+C na poleceniu zatrzymuje uruchomienie
tak jak Stop: Eidos kończy polecenie, przechwytuje sesję i odmontowuje. W
interaktywnej powłoce Ctrl+C należy jak zwykle do powłoki; wpisz `exit`, żeby
zakończyć uruchomienie.

### Kontrole Health z terminala

`eidos check <instance>` uruchamia kontrole Health, które nie potrzebują okna, więc
mogą działać przez SSH albo w Game Mode i trafić do zgłoszenia błędu:

- prefiks Protona i jego rejestr;
- diagnostyka wtyczek i brakujące mastery;
- script extender i jego wtyczki;
- aktualizacja gry od ostatniego czystego uruchomienia;
- opcja uruchamiania Steama;
- ustawienia grafiki (NVAPI, pełny ekran, ENB);
- log awarii z ostatniego uruchomienia;
- wiersze `sTestFile` w INI profilu;
- wtyczki `.esp` i blueprint w Starfield;
- `vm.max_map_count` i limit otwartych plików;
- kopia gry, z której działa instancja, oraz `gamePath` w `ModOrganizer.ini` listy,
  który nazywa inny folder gry;
- reguły kolejności kolekcji, które już nie obowiązują;
- pliki gry na NTFS, exFAT lub FAT;
- ścieżki danych BodySlide;
- foldery `Data` pozostawione w `Overwrite/Root`;
- biblioteki czcionek, które nazywa `Interface/fontconfig.txt`, a których nie
  dostarcza żaden mod;
- skompresowany dźwięk w archiwach BA2 modów Fallouta 4 i Fallouta 4 VR.

Niektóre karty liczy tylko okno, w zakładce Diagnostics: kolejność łatek, skrypty
extendera, które zastępuje mod, archiwa, których nic nie wczytuje, wymagania z
Nexusa, wtyczki oznaczone przez LOOT jako brudne, build Protona uruchamiający grę,
karta Display i wynik narzędzia starszy od swoich danych wejściowych.

Polecenie wypisuje te same wiersze systemowe co Copy report, a potem jeden wiersz
na ustalenie, z twoim folderem domowym pokazanym jako `~` i loginem jako `<user>`.
Kod wyjścia to 0, gdy nie znaleziono problemu (same rady też dają 0), 1, gdy
znaleziono co najmniej jeden problem, i 2, gdy Eidos nie mógł sprawdzić: złe
argumenty, instancja, której nie da się otworzyć, albo gra, której nie wykrywa.

`eidos check <instance> --json` wypisuje zamiast tego jeden obiekt JSON. Jego pola
najwyższego poziomu to `eidos` (wersja), `game`, `instance`, `system` (`proton`,
`flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`,
`fusermount3`, `max_map_count`, `open_files_hard`) i `findings`. `max_map_count` i
`open_files_hard` to liczby albo `null`, gdy Eidos nie mógł ich odczytać. Każde
ustalenie to `code`, `level` (`problem` lub `advice`) i jego `args`; zakładka
Diagnostics pokazuje zdanie dla każdego kodu.

### Co zmieniło się od ostatniego dobrego uruchomienia

Każde uruchomienie przez `eidos play`, `eidos shell` lub `eidos tool run`,
rozpoczęte z okna, terminala albo ze Steama, zapisuje rekord w
`<instance>/logs/sessions/`: profil, każdy włączony mod z jego wersją i
identyfikatorami plików Nexusa, kolejność wtyczek, sumę kontrolną każdego INI
profilu, wersję pliku wykonywalnego gry, wersję Eidosa, kod wyjścia i zapisy,
które uruchomienie utworzyło. Zachowuje się 50 najnowszych, a `logs/` nie trafia
do spakowanej instancji. `eidos sessions <instance> diff` porównuje najnowsze
uruchomienie gry, które zakończyło się kodem 0, z bieżącym stanem; `diff <id>`
wybiera inny rekord z listy. Lista oznacza [uruchomienie testowe](#uruchomienia-testowe) jako
`test`, a uruchomienie testowe nigdy nie jest wybierane jako ostatnie dobre.
Uruchomienie, które zatrzymałeś, zapisuje kod wyjścia 143, więc też nigdy nie jest
wybierane. Eidos nie wykrywa tu jeszcze awarii, a gra, która wywaliła się pod
Protonem, nadal może zakończyć się kodem 0: jeśli wybrane uruchomienie było
takim, wskaż wcześniejsze przez `diff <id>`.

## Lista modów

Tak jak w MO2, każdy profil ma własny `modlist.txt` (które mody są włączone, w
jakim priorytecie), a foldery modów leżą we wspólnym `mods/` instancji. Folder
wrzucony ręcznie do `mods/` nie jest jeszcze wymieniony: pojawia się jako
wyłączony z najwyższym priorytetem, na dole listy okna jak w MO2, więc wygrywa
swoje konflikty po włączeniu.

Ponieważ foldery są współdzielone, usunięcie lub zmiana nazwy moda zmienia każdy
profil: jego wiersz jest usuwany z `modlist.txt` każdego profilu albo w nim
przemianowywany, a każdy profil zachowuje własny stan i kolejność dla reszty.
Profil bez żadnego moda nie ma w ogóle `modlist.txt`.

Okno pracuje na własnej kopii listy modów i kolejności wczytywania. Jeśli inny
proces Eidosa zapisze jedno z nich, gdy okno jest otwarte (`eidos install` lub
`eidos sort` w terminalu, kolekcja, sesja uruchomiona przez Steama, drugie okno),
następna edycja w oknie jest odrzucana zamiast nadpisać tę pracę. Okno wczytuje
dane z dysku ponownie i mówi *Not saved: another Eidos process changed the mod
list. It has been reloaded - redo the change.* (albo to samo dla kolejności
wczytywania); przepada tylko to jedno kliknięcie, więc zrób je jeszcze raz.
Rename, New mod, Add separator, Remove i zamiana Overwrite w moda sprawdzają to
przed dotknięciem `mods/`, więc odrzucona edycja niczego nie zmienia na dysku.
Edytor INI podobnie odmawia zapisu nad plikiem, który zmienił się od jego
otwarcia.

## Zapisy należą do profilu

Każdy profil trzyma własne zapisy w `profiles/<name>/saves/`, a na czas
uruchomienia ten folder jest zamontowany nad folderem zapisów gry w prefiksie
Protona. Pierwsze uruchomienie profilu kopiuje do niego zapisy już obecne w
prefiksie, jeden raz, więc zapis, który później usuniesz z profilu, zostaje
usunięty. Po każdym uruchomieniu najnowsze zapisy profilu są kopiowane z powrotem
do prefiksu dla Steam Cloud; zapis w prefiksie, którego Eidos tam nie położył,
jest najpierw zachowywany w profilu jako `orphan-<time>-<name>`, nigdy nie
nadpisywany.

Zapis, który trafia do prefiksu po tej pierwszej kopii z pominięciem Eidosa
(pobranie ze Steam Cloud z Steam Decka, uruchomienie bez Eidosa), pozostaje
ukryty, dopóki zapisy profilu są zamontowane. Nic go nie usuwa: uruchomienie
wymienia go w logu (w `logs/run-*.log` instancji przy uruchomieniu z okna, w
konsoli Steama przy uruchomieniu przez Steam), raz za każdym razem, gdy ten zestaw
plików się zmienia, a karta Saves zlicza te zapisy w wierszu pod swoim nagłówkiem.
Żeby w jeden zagrać, naciśnij Import na karcie Saves. Import kopiuje każdy z jego
cosave do profilu i nigdy nie zastępuje zapisu profilu: zapis, którego nazwy
profil już używa, jest importowany jako `orphan-<time>-<name>`.

## Uruchomienia testowe

Przycisk Test run obok Run uruchamia grę tak jak Run, ale wszystko, co gra
zapisuje, idzie do `<instance>/test-run/` zamiast do twojego profilu, Overwrite i
prefiksu: nowe i zmienione zapisy, zmiany INI, `plugins.txt`, pliki zapisane w
Data lub folderze gry oraz logi w `Documents/My Games/<game>`. Możesz testować z
modami profilu, bez żadnego moda albo tylko z zaznaczonymi modami, które są
włączone, plus modami, których wtyczek potrzebują jako masterów.

Gdy gra się kończy, Eidos wymienia te pliki w pięciu grupach (zapisy, zmiany INI,
stan wtyczek, wynik w Overwrite, logi) i prosi, żebyś każdą zachował albo
wyrzucił. Apply czeka, aż każda grupa, która zawiera pliki, dostanie odpowiedź.
Stan wtyczek nie jest oferowany, gdy uruchomienie użyło innego zestawu modów niż
profil. Zachowanie zapisów nigdy nie usuwa zapisu z profilu, nawet takiego, który
gra usunęła podczas uruchomienia. Zachowanie wyniku w Overwrite usuwa też z
Overwrite pliki, które gra skasowała, a lista najpierw je zlicza. Wyrzucenie
czegokolwiek wymaga drugiego naciśnięcia. Decide later zamyka listę i zostawia
pliki w `test-run/`, a lista otwiera się ponownie przy następnym otwarciu
instancji albo naciśnięciu F5. Jeśli Eidos albo komputer zatrzymają się podczas
uruchomienia testowego, pliki też tam zostają, a lista otwiera się tak samo.

Gdy okna nie uruchomił sklep gry, Test run przekazuje grę temu sklepowi (Steam,
Heroic lub Legendary), tak jak Run, a uruchomienie, które startuje sklep, jest
uruchomieniem testowym. Jeśli sklep nie uruchomi gry, żądanie czeka: następne
uruchomienie gry tego profilu, z Run albo ze sklepu, jest uruchomieniem testowym,
a Cancel the waiting test run je porzuca. Uruchomienie testowe obejmuje tylko
foldery wymienione wyżej: inne zapisy gry, na przykład rejestr Wine, idą tam, gdzie
zawsze. Zapisy z uruchomienia testowego trafiają do Steam Cloud dopiero po tym,
jak je zachowasz i zagrasz normalne uruchomienie.

### Szukanie moda stojącego za problemem

Find the mod behind a problem, w zakładce Diagnostics, szuka moda, który powoduje
problem, który umiesz odtworzyć, na przykład awarię w danym miejscu. Każdy krok to
uruchomienie testowe z połową pozostałych podejrzanych, potrzebnymi im masterami i
modami, które zostawiasz zawsze włączone: włączonymi modami zaznaczonymi na liście
w chwili startu oraz modem dostarczającym script extender. Po każdym uruchomieniu
odpowiadasz, czy problem wystąpił. Not sure dzieli podejrzanych inaczej, a Run it
again powtarza krok. Odpowiedź albo Run it again wyrzuca pliki zapisane przez krok,
po drugim naciśnięciu. Żeby zachować część z nich, naciśnij See the files przed
odpowiedzią: otwiera tę samą listę co po uruchomieniu testowym. Gdy gra się wywaliła, a
program rejestrujący awarie zapisał log, albo gra zakończyła się kodem błędu, Eidos
oznacza Yes jako prawdopodobną odpowiedź. 200 podejrzanych to około 8 uruchomień.

Wyszukiwanie jest zapisane w `<instance>/bisect.json`, więc możesz zamknąć Eidosa
między krokami, i należy do profilu, w którym się zaczęło. Stop the search porzuca
je po drugim naciśnięciu. Wynik wskazuje moda albo zestaw modów, które potrzebują
nawzajem swoich masterów, i jest zachowywany w `<instance>/logs/bisect-<time>.txt`.
Wygenerowane mody (wynik Nemesis lub DynDOLOD, Bashed Patch) są budowane dla pełnej
listy i mogą zawieść z połową: trzymaj je zawsze włączone albo wyłączone przez całe
wyszukiwanie.

## Listy Wabbajack

Eidos nie ma instalatora Wabbajack. Zainstaluj listę przez CLF3 albo Jackify,
poza Eidosem, a potem wskaż folder instalacji jako instancję przenośną w kreatorze.
Eidos czyta folder tak, jak układa go MO2: `mods/`, `profiles/<name>/modlist.txt` i
listy wtyczek. Zachowuje `ModOrganizer.ini` zapisany przez instalator i nigdy go
nie nadpisuje, więc własne pliki wykonywalne listy zostają zarejestrowane.

Wiele list dodaje folder `Stock Game` lub `Game Root`, który zawiera załataną
kopię gry, a MO2 uruchamia grę stamtąd. Gdy kreator lub `eidos init` po raz
pierwszy przejmuje folder listy, a ten folder zawiera plik wykonywalny gry,
instancja też uruchamia grę z niego. Gra nadal startuje przez Steama albo sklep,
do którego należy, więc nic nie trzeba przenosić. Eidos znajduje folder po
`gamePath` w `ModOrganizer.ini` listy albo po tych dwóch nazwach. Instancja przejęta
przez wcześniejszego Eidosa zachowuje instalację ze sklepu, dopóki jej nie
przełączysz (zob. następna sekcja). Gdy ten `gamePath` nazywa folder gry, z którego
instancja nie działa, Diagnostics to mówi, bo narzędzia w trybie Mod Organizer 2,
takie jak PGPatcher, czytają grę stamtąd. CLF3 przebudowuje pliki BSA i DDS własnymi
narzędziami, więc wynik może się różnić od instalacji Wabbajack, a autorzy list go
nie wspierają.

## Pliki gry: instalacja ze sklepu albo kopia

Domyślnie gra działa z instalacji ze sklepu, folderu, w którym trzymają ją Steam,
Heroic lub Legendary. Instancja może zamiast tego działać z kopii tego folderu
wewnątrz instancji. Mody leżą na kopii, a instalacja ze sklepu jest tylko miejscem,
w którym Eidos montuje wynik. Sklep nadal uruchamia grę, więc Steam zachowuje swoją
nakładkę, pamięć podręczną shaderów i zapisy w chmurze.

Karta Game files na górze zakładki Diagnostics pokazuje, z jakich plików działa
instancja, z wersją gry kopii i instalacji ze sklepu, i przełącza między nimi. W
terminalu `eidos stock-game <instance>` wypisuje te same fakty, a `use <folder>`,
`store`, `freeze` i `delete --yes` robią to, co przyciski karty. `freeze` wypisuje
rozmiar i rodzaj kopii, a `freeze --yes` kopiuje.

Freeze this version kopiuje instalację ze sklepu do `<instance>/Stock Game` i
uruchamia z niej grę, więc aktualizacja sklepu nie psuje już wtyczek script
extendera: aktualizacja zmienia instalację ze sklepu, a nie kopię. Pierwsze
naciśnięcie pokazuje rozmiar i to, co system plików z nim zrobi, a drugie kopiuje.
Na systemie plików, który potrafi współdzielić bloki (reflink), takim jak btrfs lub
XFS, kopia prawie nie zajmuje miejsca, dopóki sklep nie zmieni pliku. Na ext4 albo
gdy gra i instancja leżą na różnych systemach plików, to pełna kopia wielkości
folderu gry. Eidos odmawia, gdy Steam zgłasza grę jako zajętą lub niedokończoną,
gdy działa gra lub narzędzie instancji, gdy instancja ma już kopię gry z listy,
zamrożoną kopię albo niedokończoną, oraz gdy system plików ma mniej wolnego
miejsca, niż potrzebuje pełna kopia. Kopie Heroic i Legendary nie są sprawdzane pod
kątem trwającej aktualizacji. Stop, błąd albo aktualizacja gry przez Steama w
trakcie kopiowania zostawia `Stock Game.eidos-tmp`, które karta proponuje usunąć;
nigdy nie jest używane.

Kopia ukrywa każdy plik, który sklep doda, zmieni lub usunie w folderze gry po
zrobieniu kopii: nowe pliki Creation Club, nowy plik wykonywalny, nowe DLL. Gra,
lista wtyczek, LOOT i kontrole Health czytają kopię. Nie ukrywa tego, co dzieje
się poza folderem gry: aktualizacja nadal może zainstalować runtime'y w prefiksie
Protona przy następnym uruchomieniu, a Steam może przełączyć grę na inny Proton.

Return to the store version wyjmuje kopię z następnego uruchomienia po drugim
naciśnięciu. Kopia zostaje na dysku, a Delete the copy ją usuwa, znowu po drugim
naciśnięciu. Delete jest odrzucane, dopóki instancja nadal działa z kopii. Eidos
nigdy nie usuwa folderu `Stock Game` ani `Game Root` należącego do listy. Jeśli
folder kopii zniknie albo nie zawiera już pliku wykonywalnego gry, uruchomienia są
odrzucane, a Diagnostics wskazuje folder, więc wersja ze sklepu nigdy nie
uruchomi się przez niespodzianki.

Morrowind oraz Oblivion z `bUseMyGamesDirectory=0` trzymają listę wtyczek w
folderze gry i Eidos nie uruchamia ich jeszcze z kopii.

## Instalowanie kolekcji Nexusa

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # read it and say what would happen
eidos collection skyrimse rqhcxy --no-optional  # skip the members it marks optional
```

W oknie wklej link do File -> Open a Nexus collection... albo kliknij jeden na
stronie. Eidos wymienia członków rewizji, każdego dopasowanego do tej instancji:
zainstalowany, pobrany albo brakujący. *Install this collection* uruchamia cały
przepis w wątku roboczym, z paskiem postępu używanym przez zadania pack i unpack, a
pasek po zakończeniu staje się raportem. Okno i `eidos collection` uruchamiają tę
samą instalację.

Bez linku File -> Open a Nexus collection... otwiera się na karcie Browse.
Wymienia kolekcje opublikowane dla gry otwartej instancji, po 20, posortowane
według pobrań, ostatniej aktualizacji lub najnowsze, a pole wyszukiwania zawęża
listę do kolekcji, których nazwa lub tekst pasuje. Każdy wiersz podaje autora,
najnowszą rewizję, liczbę modów, pobrania i datę. View otwiera najnowszą rewizję tej
kolekcji na karcie Link, tak samo jak wklejenie jej linku. Kolekcje oznaczone jako
dla dorosłych są pomijane, chyba że twoje konto Nexus pokazuje treści dla
dorosłych, kolekcje bez oceny są pomijane zawsze, a wiersz pod listą zlicza oba
przypadki. Przeglądanie wymaga zalogowanego konta Nexus, jak reszta panelu.

Kolekcja otwiera się tylko na instancji własnej gry. Kolekcja Skyrim otwarta, gdy
wczytana jest instancja Fallouta 4, jest odrzucana z nazwy, zamiast dopasowywać
jej członków do złej listy modów.

### Co Eidos czyta z kolekcji

Prawie nic z kolekcji nie jest widoczne przez API Nexusa. Kolejność instalacji,
odpowiedzi autora dla skryptowanego instalatora każdego moda, to, który mod wygrywa
konflikt pliku, gdzie ładują się które wtyczki, i łatki binarne są w
`collection.json` wewnątrz własnego archiwum kolekcji. Pobranie tych samych modów i
zainstalowanie ich z opcjami domyślnymi daje inną grę.

Eidos pobiera archiwum, czyta `collection.json` i je stosuje:

| Kolekcja mówi | Eidos robi |
| --- | --- |
| `phase` | instaluje w tej kolejności, opcjonalnych członków na końcu |
| `choices` | odtwarza odpowiedzi autora dla każdego FOMOD i mówi o tym, gdy nie potrafi |
| `modRules` | przestawia listę modów tak, żeby właściwy mod wygrywał każdy plik |
| `plugins` / `pluginRules` | scala reguły LOOT do twojego userlist i sortuje |
| `INI Tweaks/` | instaluje je jako osobny mod |
| `tools` | mówi, jakich narzędzi oczekuje; niczego nie tworzy |

Członkowie są włączani w trakcie instalacji, na końcu listy modów w kolejności
instalacji, zanim `modRules` ich przestawi. Członek pominięty na liście modów nie
byłby wczytany, a raport by tego nie pokazał.

Stan jest zapisywany do `<instance>/collections/<slug>-<revision>.state.json` po
każdym członku. Leży obok folderu tej rewizji, nigdy wewnątrz, więc kolekcja nie
może dostarczyć własnego pliku stanu, a instalacja przerwana na członku 147 z 200
kontynuuje od 147. Jest zapisywany pod tymczasową nazwą i przemianowywany na
miejsce, więc przerwanie zostawia stary zapis albo nowy, nigdy połowę któregoś. Jeśli
kiedykolwiek nie da się go odczytać, Eidos zatrzymuje się i mówi o tym, zamiast
zaczynać całą kolekcję od nowa. Uruchom to samo polecenie jeszcze raz.

Zainstalowanie nowszej rewizji kolekcji, którą już masz, przejmuje foldery
poprzedniej rewizji: niezmieniony członek jest weryfikowany na miejscu, członek,
którego plik autor zaktualizował, jest zastępowany wewnątrz swojego starego
folderu, a członek, którego nowa rewizja nie zawiera, zostaje zainstalowany i jest
wymieniony w raporcie, żebyś go wyłączył lub usunął. Folder, który włącza inny
profil, nigdy nie jest przejmowany, bo folder modów jest współdzielony, a ten
profil zmieniłby się pod tobą: nowa rewizja instaluje zamiast tego własną kopię
obok, a raport prosi, żebyś wyłączył starą w tym profilu, nigdy żeby ją usuwać.
Powrót do starszej rewizji przejmuje jej foldery z powrotem w ten sam sposób albo
instaluje nowe kopie tych, których nie może przejąć. Ilekroć członek jest w ten
sposób zastępowany, a w jego folderze zmieniłeś lub dodałeś pliki, stary folder jest
najpierw zachowywany obok jako `<name>_backup`, który nigdy nie jest wczytywany do
gry, a raport go wymienia.

Pozostałość po wcześniejszej rewizji, która zostaje włączona, sprawia, że
instalacja nie liczy się jako ta, którą zbudował autor; notatka znika, gdy ją
wyłączysz. Członek pominięty przez `--no-optional` nie jest przejmowany i jest
wymieniony tak samo. Jeśli samo przejęcie się nie powiedzie, uruchomienie zatrzymuje
się z *previous revision: its folders could not be taken over* i powodem; napraw to
i uruchom polecenie ponownie.

### Darmowe konta i członkowie, których pobierasz sam

Darmowe konto Nexus nie może pobrać modów członków. Nexus nie daje darmowym kontom
linków pobierania przez API: każdy plik wymaga przycisku „Mod Manager Download” ze
strony. Archiwum kolekcji pobiera się na darmowym koncie, więc możesz przeczytać
cały przepis, ale każdy członek jest zgłaszany z dokładnym adresem swojej strony,
żebyś pobrał go sam, a ponawianie tego nie zmienia. Pobierz je przyciskiem ze
strony i uruchom polecenie ponownie: ci członkowie są szukani w `downloads/` przy
każdym uruchomieniu, więc kolekcja kończy się w miarę, jak je dostarczasz.

W oknie darmowe konto dostaje w panelu kolekcji *Fetch the N missing required
files, one by one* zamiast pobrania zbiorczego. Otwiera stronę pierwszego
brakującego wymaganego pliku na Nexusie, czeka, aż ten plik wyląduje w
`downloads/`, potem otwiera następny. Nadal naciskasz przycisk pobierania na
stronie dla każdego pliku. Panel pokazuje, jak daleko zaszedł (*3 of 12*), z
*Skip* i *Stop*. Gdy zamiast przypiętego pliku ląduje inny plik tego samego moda,
panel wymienia oba identyfikatory plików i dalej czeka na przypięty. Opcjonalni
członkowie nie są jego częścią i zachowują własny przycisk *Open*.

Eidos stosuje też łatki binarne (każdą sprawdzoną względem zapisanego CRC),
nadpisania plików (wskazane pliki są ukrywane przez `.mohidden`, jak w MO2) oraz
członków, których pliki podróżują wewnątrz archiwum kolekcji (`bundle`). Członkowie,
których kolekcja oczekuje, że pobierzesz ręcznie (`browse`, `manual`), przenoszą do
raportu własne instrukcje autora. Członek, którego nazwa jest już modem u ciebie,
jest instalowany obok pod wolną nazwą, nigdy na nim, a raport mówi który.

### Raport

Członek, którego odpowiedzi instalatora nie wszystkie dało się odtworzyć, dostaje
własny wiersz, *installed, but not the way the collection asks*, oddzielony od
członków zainstalowanych tak, jak zbudował je autor, bo jego pliki na dysku różnią
się od plików autora. Raport wymienia też reguły kolejności, które nic nie nazywały
w kolekcji, mody złapane w pętlę sprzecznych reguł, grupy LOOT, które nie istnieją,
i każdą część manifestu, której ta wersja Eidosa nie rozumie.

### Zachowanie kolejności kolekcji

Eidos zachowuje reguły każdej zainstalowanej kolekcji po instalacji. Gdy później
przeniesiesz dwa jej mody wbrew regule przed lub po, zakładka Diagnostics pokazuje
jedną kartę dla tej kolekcji, nazywającą pierwszą taką parę i zliczającą reguły,
które już nie obowiązują. Liczą się tylko mody włączone w bieżącym profilu, a reguły
złapane w pętlę są pomijane, bo żadna kolejność ich nie zachowa. Restore collection
order przenosi mody kolekcji z powrotem do kolejności, która zachowuje każdą
regułę, zachowuje twoją kolejność wszędzie, gdzie reguły na to pozwalają, i
zostawia mody spoza kolekcji tam, gdzie są. Ctrl+Z to cofa. `eidos check` wymienia
to samo ustalenie jako `collection_order_broken`. Kontrola czyta przepis rewizji w
`collections/`, więc kolekcja, której zapisany przepis usunięto, nie jest
sprawdzana.

## Przenoszenie instancji na inny komputer

`eidos pack` zapisuje całą instancję (każdy mod, kolejność wczytywania, wszystkie
profile, Overwrite z twoimi zapisami i archiwa, z których wszystko zainstalowano) w
jednym pliku. `eidos unpack` przywraca ją:

```sh
eidos pack skyrimse ~/backup.eidos                  # or give a folder: named for the game and the date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # on the other machine
```

`eidos pack --dry-run` wymienia dokładnie, co wejdzie, czego nie i dlaczego, oraz
ile miejsca potrzebuje, niczego nie zapisując. `eidos unpack --info` robi to samo
dla pliku, który już masz.

Jeśli 7-Zip nie potrafił czegoś odczytać albo przejście musiało pominąć coś, czego
oczekiwałbyś w kopii zapasowej (dowiązanie symboliczne, folder, którego nie dało
się odczytać, nazwę, której nie można bezpiecznie nadać 7-Zipowi), archiwum jest
nadal zapisane pod wybraną nazwą, ale `eidos pack` kończy się kodem 1 i mówi, czego
brakuje, więc skrypt uruchamiający `eidos pack ... && ...` zatrzymuje się tam.

Plik `.eidos` to archiwum 7-Zip z własnym rozszerzeniem. 7-Zip jest już wymagany do
instalacji moda, więc nie dodaje to zależności, a kopia otwiera się bez Eidosa w
każdym archiwizatorze, który czyta 7-Zip. Jest kompresowany LZMA2 z `-mx1`, do około
43% oryginału na liście pełnej tekstur; `-mx9` trwa znacznie dłużej i oszczędza o
kilka punktów więcej. Tekstury i siatki są już formatami skompresowanymi, więc
większy słownik znajduje bardzo mało do skompresowania. `--level` (0, 1, 3, 5, 7
lub 9) wybiera inny poziom, a `--no-downloads` pomija `downloads/`.

Przy domyślnym poziomie 1 pliki trafiają do bloków litych o rozmiarze najwyżej
16 MB. Wyciągnięcie jednego pliku z kopii 70 GB dekoduje najwyżej blok, w którym
leży, nigdy wszystko przed nim, a pakowanie zawartości gry działa 2 do 5 razy
szybciej niż przy każdym pliku kompresowanym osobno (rozpakowywanie 2 do 4 razy).
Koszt jest przy uszkodzeniu: jeden uszkodzony bajt może zabrać ze sobą każdy plik w
swoim bloku, do 16 MB sąsiednich plików. Dane odzyskiwania opisane niżej to
pokrywają. Każdy inny poziom pozostaje nielity: każdy plik to osobny strumień, a
uszkodzony bajt kosztuje ten jeden plik. Poziom 0 przechowuje pliki. Od poziomu 3 w
górę LZMA2 już pracuje w porcjach 16 MB lub większych, a bloki lite zmierzono tam
jako wolniejsze: na poziomie 5 pakowanie siatek trwało 1,4 raza dłużej, a
rozpakowywanie tekstur 6,8 raza dłużej.

Pakowanie bierze blokadę instancji, więc nie może działać, gdy gra lub inny Eidos
zapisuje do tej instancji, a nic nie zmienia instancji w trakcie jej czytania.
Kopia jest zapisywana pod nazwą `.part`, zrzucana na dysk i czytana z powrotem w
całości przez `7z t`, zanim dostanie wybraną nazwę. To sprawdzenie dekoduje każdy
blok i porównuje każdą sumę kontrolną, tę samą pracę, którą wykonuje rozpakowanie
na drugim komputerze. Pokazuje, co dysk oddał zaraz po zapisie, nie to, że dysk to
utrzyma; do tego służą dane odzyskiwania niżej. Jeśli się nie powiedzie, `.part`
jest usuwany, nic nie jest zapisane pod twoją nazwą, a `eidos pack` cytuje wiersz,
w którym 7-Zip znalazł uszkodzenie. Przerwane lub nieudane pakowanie zostawia plik
`.part` albo nic, nigdy plik `.eidos`, który wygląda na kompletny, a nie jest.

Gdy zainstalowany jest par2cmdline (`sudo pacman -S par2cmdline` na Archu,
`sudo apt install par2` na Debianie i Ubuntu, `sudo dnf install par2cmdline` na
Fedorze), `eidos pack` zapisuje też dane odzyskiwania PAR2 obok archiwum:
`backup.eidos.par2` i kilka plików `backup.eidos.volNN+MM.par2`, 5% archiwum plus
mały indeks. Trzymaj je w tym samym folderze co plik `.eidos`, pod nazwami, z
którymi zapisano, i kopiuj razem z nim: kopia przeniesiona bez nich nie ma z czego
być naprawiona, a dane odzyskiwania zapisują nazwę archiwum, więc zmieniona nazwa
kopii już do nich nie pasuje. `eidos unpack` znajduje je po nazwie i sprawdza
archiwum względem nich przed rozpakowaniem czegokolwiek, także kopię, której koniec
obcięto tak, że 7-Zip nie może jej otworzyć. Jeśli kilka bajtów się zepsuło, par2
najpierw naprawia archiwum i zachowuje uszkodzoną kopię jako `backup.eidos.1`, co
wymaga wolnego miejsca wielkości archiwum w jego folderze; uszkodzenie wykraczające
poza to, co obejmuje 5%, zatrzymuje rozpakowanie, zanim zapisany zostanie choć jeden
plik. Pliki odzyskiwania, które nie pasują do archiwum, takie jak te starszej kopii
o tej samej nazwie, nie są używane, gdy 7-Zip czyta archiwum i uznaje je za
nienaruszone, a rozpakowanie o tym mówi. Na komputerze bez par2 rozpakowanie nadal
działa, z ostrzeżeniem, że danych odzyskiwania nie użyto. `--no-recovery` pakuje bez
nich, a `eidos pack --dry-run` mówi, czy zostaną zapisane. `--force` zastępuje
istniejący plik o tej samej nazwie.

### Czego w niej nie ma i dlaczego

| Pominięte | Dlaczego |
| --- | --- |
| Prefiks Protona | Jest ukształtowany pod komputer (mapowania dysków, widok `Z:` *tego* systemu plików), a Eidos odbudowuje go w minutę przez `eidos prereqs <instance> --install`. Przenoszenie go mniej więcej podwoiłoby archiwum, żeby wysłać coś, co drugi komputer i tak musi wygenerować. |
| Gra | Instancja moduje grę, której nie zawiera. Zainstaluj ją ze Steama tam. |
| Narzędzia poza instancją | xEdit, DynDOLOD i inne narzędzia to pliki binarne stron trzecich z własnymi instalatorami. Są nazwane w archiwum, więc rozpakowanie mówi dokładnie, których brakuje na nowym komputerze. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Zapisy komputera, który zrobił kopię. Zwłaszcza `prereqs.done` twierdziłby, że biblioteki runtime są już zainstalowane w prefiksie, którego tam nie ma. |
| `loot/`, poza `userlist.yaml` | Pamięć podręczna masterlisty, którą Eidos pobiera ponownie na żądanie. Twoich własnych reguł LOOT w `userlist.yaml` nie da się pobrać ponownie, więc ten jeden plik wchodzi. |
| `.base/`, `.base-root/` | Puste punkty montowania, gdzie podczas sesji odkładane są własne pliki gry. |
| `test-run/`, `bisect.json` | [Uruchomienie testowe](#uruchomienia-testowe) czekające na przegląd i wyszukiwanie moda stojącego za problemem między dwoma krokami. Oba należą do uruchomień na tym komputerze. |
| Niedokończone pliki | Wstrzymane pobranie (`*.unfinished`), zapis atomowy w toku (`*.eidos-tmp*`). |
| Dowiązania symboliczne | 7-Zip podążyłby za jednym i skopiował to, na co wskazuje, co dla dowiązania bezwzględnego oznacza wciągnięcie obcego drzewa do twojej kopii. Są zgłaszane zamiast tego, a kopia liczy się jako niekompletna: `mods/` lub `downloads/` dowiązane do innego dysku nie jest pakowane. Żeby zrobić kopię celowo bez dowiązanego `downloads/`, podaj `--no-downloads` (albo odznacz *Include downloads/* w oknie); celowe wykluczenie nie liczy się jako brak. |

Kopia gry w instancji (`Stock Game`) jest pakowana jak każdy inny folder, więc kopia
zamrożonej instancji jest o rozmiar gry większa. Niedokończona kopia
(`Stock Game.eidos-tmp`) jest pomijana.

Każda z tych pozycji jest wymieniona z powodem w `eidos-backup.ini` w korzeniu
archiwum, więc przeczytanie tego pliku pokazuje, co pominięto. Ten sam plik zapisuje,
gdzie instancja mieszkała, który profil był aktywny, ile zajmie po rozpakowaniu i
jakich narzędzi będzie potrzebował nowy komputer.

Rekordy pobrań (`downloads/*.meta`) wchodzą z opróżnionym wierszem `url=`. Ten URL to
podpisany link Nexusa, który przestaje działać w ciągu godzin i niesie `user_id`
konta, które pobrało plik, a kopia może zostać przekazana komuś innemu.
Identyfikator moda, identyfikator pliku i wersja zostają, więc pobranie można znaleźć
ponownie.

### W oknie

Oba są w menu File: *Pack this instance...* i *Unpack a backup...*. Rozpakowanie jest
też na ekranie powitalnym, pod listą instancji, więc komputer bez żadnej instancji
może rozpakować jedną.

Zanim cokolwiek zostanie zapisane, dialog Pack pokazuje, ile plików, jak duże, ile z
tego to `downloads/`, mniej więcej ile wyniesie archiwum i które narzędzia są
nazwane, ale nieprzenoszone. Pole *Write recovery data* jest zaznaczone, gdy
zainstalowany jest par2, a wyszarzone z pakietem do zainstalowania, gdy nie jest.
Dialog Unpack czyta manifest kopii, gdy tylko wybierzesz plik, więc może powiedzieć,
jaką grę zawiera, kiedy ją zrobiono, gdzie mieszkała i których jej narzędzi tu
brakuje, zanim wybierzesz folder.

Oba działają w wątku roboczym z paskiem postępu, a pasek po zakończeniu staje się
raportem.

### Co naprawia rozpakowanie

Zwykła kopia folderu zostawiłaby każdy przycisk narzędzia wskazujący ścieżkę na
starym komputerze. `eidos unpack` przepisuje wartości, które nazywają stary korzeń
instancji, i tylko je: `exe`, `workdir` i numerowane klucze `arg` w `tools.ini` oraz
`installationFile` w `meta.ini` każdego moda. Narzędzie, które nigdy nie było
wewnątrz instancji (`/opt/xedit/SSEEdit.exe`), zostaje dokładnie takie, jakie było, i
jest wymienione jako coś do ponownej instalacji; Eidos nie zgaduje, gdzie to
narzędzie jest na nowym komputerze. Poprawia też to, czy instancja nazywa siebie
centralną czy przenośną, więc późniejsze polecenia szukają jej tam, gdzie teraz jest.

Rozpakowanie odrzuca folder, który nie jest pusty (podaj `--force`, jeśli chcesz
nadpisać), dysk za mały względem własnej liczby z manifestu, archiwum zrobione przez
nowszego Eidosa niż twój i każde archiwum zawierające wpis, który byłby zapisany
*poza* wybranym folderem. Bez folderu kopia instancji globalnej wraca do
centralnego miejsca jej gry; przenośna wymaga folderu.

Potem:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # rebuild the Proton prefix
eidos play /mnt/games/EidosSkyrim
```

## Okno

Uruchom `eidos-gui` albo plik `eidos-linux`, który instaluje Colony.

Pierwsze uruchomienie otwiera kreator jak w MO2: powitanie, rodzaj instancji
(przenośna lub globalna), gra, nazwa i położenie, podsumowanie, potem okno główne.
Ekran powitalny wymienia też każdą znaną instancję, globalną i przenośną, od
ostatnio używanej, do otwarcia jednym kliknięciem, więc służy też jako przełącznik
instancji. Wskazanie w kreatorze folderu, który już zawiera instancję, otwiera tę
instancję taką, jaka jest, zamiast tworzyć nową nad nią, a folder należący do innej
gry jest odrzucany.

Okno główne ma dwa panele: listę modów oraz karty Data, Plugins, Conflicts,
Overwrite, Archives, Saves, Downloads i Diagnostics, z przyciskiem Run, przyciskiem
Test run ([Uruchomienia testowe](#uruchomienia-testowe)) i wyborem celu uruchomienia. Listę
modów filtrujesz, zaznaczasz, przestawiasz, grupujesz separatorami, zawężasz
kategorią i klikasz prawym przyciskiem po akcje. Poza wysłaniem na górę i na dół
menu moda ma przesunięcia celowane z MO2: wysłanie nad pierwszy konfliktujący mod,
pod ostatni, na dany priorytet albo do grupy separatora.

Wybór profilu przełącza profile. Kliknij profil prawym przyciskiem po Rename, Copy
to new..., Desktop shortcut, Import from MO2... i usunięcie, które wymaga dwóch
kliknięć i odrzuca profil aktywny lub ostatni. Import from MO2... przejmuje kolejność
modów i kolejność wczytywania profilu MO2. Desktop shortcut zapisuje wpis pulpitu,
który uruchamia `eidos tool <instance> run "<game>" --profile <name>`: czyni ten
profil aktywnym i prosi sklep o uruchomienie gry (szczegóły w
[tools.md](tools.md#dotarcie-do-narzędzia-ukrywanie-przypinanie-i-skrót-na-pulpicie)).

Menu File uruchamia też winecfg, regedit, eksplorator plików Wine i deinstalator w
prefiksie gry, a Prefix drive_c otwiera dysk `C:` prefiksu; zob.
[tools.md](tools.md#własne-programy-wine-w-prefiksie-gry). *Instances...*
wymienia instancje, które zna Eidos; Forget zdejmuje jedną z tej listy i niczego nie
dotyka na dysku. *Export the mod list...* zapisuje listę w formacie CSV z MO2, tak
jak `eidos export`.

### Preferencje

Kliknij Eidos w lewym górnym rogu, żeby otworzyć Preferences zamiast dwóch paneli, i
kliknij go ponownie (albo Close), żeby wrócić. Najpierw General, Appearance i
Accessibility, jak w każdym programie Colony, potem Mod list, Nexus i About. Każda
zmiana stosuje się i zapisuje od razu; nie ma przycisku Save.

- General: gra, którą otwiera Eidos, rozmiar okna, język interfejsu, folder
  narzędzi i sposób uruchamiania gry. Lock the window while a game runs jest
  domyślnie włączone. Start the game at once when a store opens Eidos oraz
  powiadomienie pulpitu, które mówi, gdy długie zadanie kończy się, gdy okno jest w
  tle, są domyślnie wyłączone. Okno mówi po angielsku i francusku; System podąża za
  `LC_ALL`, `LC_MESSAGES` lub `LANG`, a zmiana stosuje się od razu. Wiersz poleceń
  `eidos` zawsze odpowiada po angielsku.
- Appearance: motyw (26 rodzin, 59 palet, domyślnie pergamin Eidosa), kolor
  akcentu, rozmiar tekstu, efekty i podgląd na żywo.
- Accessibility: wysoki kontrast, czcionka przyjazna dysleksji, ograniczony ruch i
  rozmiar tekstu do czytania, który mnoży się z tym z Appearance.
- Mod list: szybkość automatycznego przewijania, gdy mod jest przeciągany do górnej
  lub dolnej krawędzi listy.
- Nexus: logowanie, folder pobrań, obsługa `nxm://` i jej przycisk Register,
  preferowane serwery, Install downloads when they finish i tryb Offline.
- About: gdzie leżą pliki, skróty klawiszowe, wersja i sprawdzanie wydań.

Wygląd i język interfejsu żyją w `~/.config/Colony/Eidos/preferences.json`. Pozostałe
ustawienia, które czyta też wiersz poleceń (domyślna gra, folder narzędzi, serwery
Nexusa, tryb Offline i reszta), żyją obok w `settings.ini`, a sesja Nexusa w
`nexus.ini`. Motyw, który starszy Eidos trzymał w `settings.ini`, jest przenoszony
przy pierwszym razie.

About -> Version zawiera sprawdzanie wydań, domyślnie włączone. Przy starcie,
najwyżej raz dziennie, Eidos pyta GitHub o najnowsze wydanie `Project-Colony/Eidos`.
Gdy jest nowsze niż działająca kopia, pasek menu pokazuje po Help
`Eidos <version> is available: release notes`, a link otwiera stronę tego wydania z
jego listą zmian. Eidos nigdy sam niczego nie pobiera ani się nie podmienia.
Sprawdzanie jest pomijane w trybie offline. Jest wyłączone dla kopii zainstalowanej
w `/usr` przez menedżer pakietów, który ją aktualizuje; `/usr/local`, dokąd
`install.sh --system` kładzie Eidosa, liczy się jako twoja własna instalacja.

### Kolumny, sortowanie i grupowanie

Lista domyślnie pokazuje cztery kolumny (Category, Content, Version i Flags, te,
które pokazuje MO2) i oferuje osiem: Category, Content, Version, Author, Installed,
Nexus id, Game, Flags. Zaznacz je w menu View. Przy wszystkich ośmiu kolumna nazwy
ma mało miejsca.

Kliknij nagłówek, żeby posortować po nim, ponownie, żeby odwrócić, a trzeci raz, żeby
wrócić do kolejności wczytywania. Listę można przeciągać tylko w kolejności
wczytywania: pozycja upuszczenia adresuje prawdziwą listę, a przy sortowaniu miejsce
wiersza na ekranie nie jest jego miejscem na tej liście. Gdy sortowanie jest
włączone, paski wstawiania nie są rysowane, a przeciąganie jest odrzucane, jak w
MO2. Menu View to mówi i oferuje drogę powrotną.

Menu View może też grupować całą listę, według kategorii albo źródła (z Nexusa lub
zainstalowane ręcznie). Nagłówki grup nie są separatorami: nie można ich
przemianowywać, kolorować ani przenosić. Zwijają się, a zwinięty nagłówek zachowuje
swoją liczbę. Separatory są ukryte, gdy włączone jest sortowanie lub grupowanie, bo
separator otwiera wiersze, które po nim następują w kolejności wczytywania, a oba
zmieniają tę kolejność na ekranie.

### Mysz i klawiatura

Podwójne kliknięcie moda otwiera Information, Ctrl+podwójne kliknięcie jego folder,
Shift+podwójne kliknięcie jego stronę na Nexusie. Ctrl+F stawia kursor w polu
filtra. Wpisanie litery skacze do następnego moda zaczynającego się od niej, a
ponowne jej naciśnięcie przechodzi przez resztę zamiast zostawać na pierwszym. Żaden
z nich nie ląduje na wierszu, który ukrywa filtr, zwinięty separator lub zwinięta
grupa, więc następny Spacja nigdy nie przełączy moda, którego nie widzisz.

„Collapse others” w menu separatora zwija każdą grupę poza tą. Podczas przeciągania
przytrzymanie wskaźnika nieruchomo nad zwiniętą grupą otwiera ją, więc mod można
upuścić do środka; przejechanie nad nią tego nie robi.

Ctrl+Z cofa ostatnią zmianę listy modów lub kolejności wczytywania: zaznaczenie,
przeniesienie, zbiorcze wyłączenie, sortowanie LOOT-em. Ctrl+Shift+Z ją ponawia.
Okno trzyma 50 kroków i zapomina je przy F5, na końcu uruchomienia gry lub
narzędzia, przy zmianie profilu i gdy inny proces Eidosa zmienia listę.
Instalowanie, zmiana nazwy lub usunięcie moda zmienia foldery na dysku, więc cofnięcie
poza któreś z nich jest odrzucane.

### Filtrowanie listy

Przycisk Filters otwiera po jednym wierszu na kryterium: active, conflicted, update
available, no category, endorsed, tracked, has a Nexus id, has hidden files, no
valid game data, for another game, has notes oraz po jednym wierszu na rodzaj
zawartości (plugins, BSA or BA2 archives, textures, meshes, scripts, script extender
plugins, interface files, sound, FaceGen). Kliknięcie przełącza wiersz między off,
only i except. Wiersz na górze przełącza między dopasowaniem wszystkich włączonych
kryteriów a dopasowaniem któregokolwiek z nich. Pole nazwy i lista kategorii zawsze
dodatkowo zawężają wynik. Wiersz o ukrytych plikach patrzy tylko na mody włączone.

### Co lista mówi ci o modzie

Dwie flagi doradcze, obie jako glif z wyjaśnieniem po najechaniu. No valid game data
znaczy, że nic na szczycie moda nie wygląda jak coś, co ta gra wczytuje; może
potrzebować przeniesienia folderów o poziom wyżej albo nie być modem do tej gry.
Another game znaczy, że własny `meta.ini` moda nazywa inną grę. Żadna nie blokuje
niczego: mod nadal się wdraża. „Mark as valid” w menu wiersza uciszają obie przez
własny klucz `validated=` MO2, więc mod oznaczony jako poprawny w jednym menedżerze
nie jest oflagowany w drugim.

Kontrola układu oflagowuje moda tylko wtedy, gdy jest pewna: drzewo `Root/`,
nieczytelny folder i pusty folder liczą się jako poprawne dane gry.

### Wyłączanie moda, którego potrzebują twoje zapisy lub inne mody

Zanim mod zostanie wyłączony lub usunięty, Eidos czyta listy wtyczek trzech
najnowszych zapisów profilu. Gdy jeden z nich używa wtyczki z tego moda, pasek nad
listą wymienia wtyczki i czeka na Disable anyway albo Cancel (Escape też anuluje).
Remove dodaje te same nazwy do swojego komunikatu „kliknij ponownie”. Czytane są
tylko zapisy Skyrim i Fallouta 4; inne gry wyłączają bez pytania.

Information moda, karta General, wymienia mody, których wymaga, i mody, które
wymagają jego, odczytane z masterów włączonych wtyczek. Wyłączenie moda, którego
wymagają inne, pokazuje ten sam pasek z Disable them too, które wyłącza też każdy
mod, który go potrzebuje, i mody, które potrzebują tamtych, albo Keep them enabled.
Karta brakujących masterów w zakładce Diagnostics ma Enable what provides them:
włącza mod, który zawiera każdy brakujący master, i zaznacza wtyczkę master, która
jest obecna, ale niezaznaczona.

### Kopia moda, zanim go dotkniesz

„Back up this mod” kopiuje jego folder obok jako `<name>_backup` (potem
`_backup2` i tak dalej; kopia nigdy nie zastępuje poprzedniej). Kopia nie jest
modem: jej pole wyboru niczego nie robi i niczego nie dodaje do scalonego widoku, bo
zaznaczenie jej wdrożyłoby dwie kopie jednego moda jedna nad drugą. „Restore this
backup over the mod” przywraca ją, w dwóch kliknięciach; bieżąca zawartość jest
najpierw odkładana na bok i odrzucana dopiero po udanym skopiowaniu.

Backups na pasku narzędzi trzyma punkty przywracania listy modów profilu, jego
kolejności wczytywania (`plugins.txt` i `loadorder.txt` razem) i jego INI gry, po 10
każdego rodzaju, w `<profile>/backups/`. Back up now dodaje jeden, a Restore najpierw
robi kopię bieżącego stanu, więc błędny wybór można cofnąć. Eidos zachowuje też punkt
przywracania INI przed każdym uruchomieniem BethINI (zob.
[tools.md](tools.md#bethini-pie)).

### Karty

Data to drzewo scalonego widoku, rozwijane po jednym poziomie: otwarcie folderu
czyta ten folder w każdej warstwie, która go ma, nigdy każdy włączony mod naraz.
Jest budowane z tego samego stosu warstw, który obsługuje montowanie, więc
whiteouty i ukryte pliki są respektowane, a karta pokazuje to, co zobaczy gra.
Filtruj ją po nazwie, zawęź do plików spornych, rozeznaj, co gdzie jest, kolumnami
Size i Modified, i pokaż dowolny wiersz w menedżerze plików (Reveal). Najechanie na
plik wymienia inne mody, które go dostarczają. Najwyższego poziomu `meta.ini`,
`readme.txt`, `fomod/` i `Root/` moda są pomijane w Data, w tej karcie i w
montowaniu: pierwsze trzy są dla menedżera modów, a `Root/` trafia zamiast tego do
folderu gry. Plik, który gra lub narzędzie zapisze pod jedną z tych nazw, ląduje w
Overwrite i pozostaje widoczny.

Wiersz `.exe` w Data ma Add tool, które otwiera edytor Executables z wypełnionym
tym plikiem (nic nie jest zapisywane, dopóki nie naciśniesz Save), oraz Run, które
uruchamia go jednorazowo przez scalony widok jak zarejestrowane narzędzie
([Narzędzia](#narzędzia)). Save tree zapisuje całe scalone drzewo Data do pliku
tekstowego, po jednym wierszu na plik z modem, który go dostarcza, z usuniętym
folderem domowym i nazwą użytkownika, do zgłoszenia błędu.

Plugins to kolejność wczytywania ESP/ESM/ESL: przełączaj, przestawiaj ręcznie albo
sortuj LOOT-em i czytaj raport po sortowaniu, którego linki z radami otwierają się w
przeglądarce. Jej nagłówek zlicza włączone wtyczki w każdej przestrzeni indeksów
względem limitu gry, na przykład `full 231/254, light 512/4096`, i robi się
pomarańczowy, gdy któraś jest pełna. Pole filtra zawęża listę po nazwie. Spacja,
Ctrl+A, zakresy z Shift oraz Activate all shown i Deactivate all shown w menu
wiersza działają wtedy tylko na pokazanych wierszach, a przeciąganie jest
odrzucane, dopóki filtr nie zostanie wyczyszczony. Zaznaczenie wtyczki podświetla jej
mastery na liście wtyczek i mod, który ją dostarcza, na liście modów. Wtyczki, które
LOOT oznacza jako brudne, dostają kartę Diagnostics, która może je czyścić xEdit-em
([tools.md](tools.md#czyszczenie-pluginu-który-loot-oznacza-jako-brudny)).

Po sortowaniu LOOT-em pełna wtyczka, której wszystkie nowe rekordy mieszczą się w
zakresie light, mówi o tym po najechaniu, a menu jej wiersza ma Flag as light. Eidos
zapisuje kopię z flagą light do nowego moda o nazwie `<plugin> (light)` na dole
listy modów (najwyższy priorytet) i zostawia oryginał w spokoju, więc wyłączenie tego
moda to cofa. Odmawia, gdy jeden z trzech najnowszych zapisów profilu wczytuje tę
wtyczkę jako pełną. Czytane są tylko zapisy Skyrim i Fallouta 4, więc w Starfield
żaden zapis nie jest sprawdzany. LOOT sprawdza tylko zakresy rekordów: wtyczkę,
która dodaje komórki lub światy, obejrzyj w xEdit, zanim ją oflagujesz.

Najechanie na wtyczkę pokazuje, skąd pochodzi, jej autora, wersję nagłówka i form,
pierwsze 300 znaków opisu, jej mastery i, po sortowaniu LOOT-em, komunikaty LOOT. W
Skyrim, Skyrim SE, Skyrim VR i Enderal SE gra czyta też INI nazwane od wczytanej
wtyczki; kolumna INI oznacza te wtyczki, a kliknięcie otwiera ten plik tylko do
odczytu.

Conflicts wyjaśnia zwycięzców i przegranych dla każdego pliku. Overwrite zamienia to,
co zapisała gra, w prawdziwego moda jednym krokiem.

Archives wymienia każdy BSA lub BA2, który dostarczają włączone mody, mod, z którego
pochodzi, i dlaczego się wczytuje: aktywna wtyczka o tej samej nazwie albo rejestruje
go INI. Ten, którego nic nie wczytuje, jest oznaczony jako zbędny balast. Gdy inny
mod dostarcza archiwum o tej samej nazwie, które wygrywa konflikt, wiersz wskazuje
ten mod. Extract rozpakowuje archiwum do nowego moda nazwanego od niego bez
rozszerzenia (`Foo.ba2` daje `Foo (extracted)`) i zostawia archiwum w jego modzie;
luźne pliki wygrywają z plikami w każdym archiwum, bez względu na kolejność modów.

Saves czyta nagłówek każdego zapisu (postać, poziom, lokacja, czas gry), porównuje
zapisaną w nim listę wtyczek z bieżącą i ma przycisk, który włącza potrzebne mody.
Zapisy, które trafiły do prefiksu bez Eidosa, opisano w
[Zapisy należą do profilu](#zapisy-należą-do-profilu).

Downloads i Diagnostics mają własne sekcje niżej.

### Informacje o modzie

„Information...” otwiera dialog moda z kartami General, Conflicts, Filetree, INI
Tweaks, Notes, Nexus files i Nexus page. Z filetree (i z drzewa Data) każdy plik
można ukryć: jest przemianowywany na `<name>.mohidden`, co zdejmuje go ze scalonego
widoku bez usuwania, więc kilka zabłąkanych plików jednego moda można wyciszyć bez
zmiany priorytetów. Filetree wykonuje też zwykłe operacje na plikach: nowy folder,
zmiana nazwy, usunięcie, otwarcie. Każda odrzuca ścieżkę, która nie jest zwykłą
ścieżką wewnątrz tego moda: bez `..`, bez ścieżki bezwzględnej i bez dowiązania
symbolicznego po drodze, bo podążenie za nim mogłoby usunąć plik poza folderem moda.
Zmiana nazwy zmienia tylko ostatnią część ścieżki, więc nigdy nie przenosi pliku, i
odrzuca nazwę, która jest już zajęta. Usunięcie wymaga dwóch kliknięć i nie da się go
cofnąć.

View na dowolnym wierszu w filetree lub drzewie Data podgląda plik: obrazy, tekst,
tekstury DDS (wybierz mip, warstwę, ścianę i kanał) i statyczne modele NIF z ich
teksturami diffuse. Podgląd NIF potrzebuje `eidos-nif-preview` obok Eidosa albo w
`PATH`; pakiet wydania, pakiet Arch i kompilacje ze źródeł go mają, `eidos-linux`
nie. Tekst jest czytany do 64 KB i mówi, gdzie się zatrzymał.

INI Tweaks wymienia fragmenty, które mod dostarcza w folderze `INI Tweaks/`; włączone
są scalane do INI gry profilu przy uruchomieniu, w kolejności priorytetów, i
wyjmowane z powrotem, gdy INI uruchomienia są przechwytywane, więc wyłączenie
tweaka usuwa jego ustawienia.

Karta Nexus files wymienia pliki moda na Nexusie, od najnowszego, i oznacza ten
zainstalowany tutaj oraz ten, który go zastępuje. Ten drugi wiersz ma przycisk
Update; zob. [Aktualizacje modów](#aktualizacje-modów).

Karta Nexus page pokazuje opis moda i, od najnowszej, listę zmian każdej wersji
pliku z jego strony. Eidos czyta oba przy pierwszym otwarciu karty, z API GraphQL
Nexusa, które nie liczy się do godzinnego limitu żądań. Zachowuje je, dopóki Eidos
się nie zamknie albo nie naciśniesz Refresh. Są pokazane jako zwykły tekst:
formatowanie BBCode i HTML jest usuwane, adres linku jest zapisany po jego tekście, a
obrazy i filmy są pomijane. Endorse i Track nad tekstem robią to, co menu prawego
przycisku moda. Strona moda dla dorosłych pozostaje ukryta, chyba że twoje konto
Nexus pokazuje treści dla dorosłych, a strona bez oceny pozostaje ukryta w każdym
przypadku.

### Downloads

Pobranie można przeciągnąć z listy Downloads na pozycję na liście modów, żeby
zainstalować je z tym priorytetem, a archiwa lub foldery upuszczone na okno z
menedżera plików też się instalują. Upuszczanie plików wymaga sesji X11 lub
XWayland: winit implementuje je tylko dla X11. Pobrania można wstrzymywać i
wznawiać: wstrzymanie zatrzymuje transfer i zachowuje częściowy plik, a Resume prosi
Nexus o świeży link i kontynuuje od miejsca zatrzymania.

Karta Downloads trzyma pobrane archiwa. Filtruj ją po nazwie (także przyjaznej
nazwie moda, więc „skyui” znajduje `SkyUI_5_2_SE-12604-5-2SE.7z`), sortuj po
najnowszych, nazwie, rozmiarze lub stanie i ukryj archiwum, z którym skończyłeś:
plik zostaje, znika tylko wiersz. „Show hidden” je przywraca, a ten sam przycisk
odkrywa. „Remove N installed” usuwa archiwa modów, które już zainstalowałeś, w
dwóch kliknięciach, i tylko te, które pokazuje filtr.

Preferences -> Nexus -> Install downloads when they finish jest domyślnie
wyłączone. Włączone, pobranie z Nexusa, które kończy się, gdy okno jest otwarte,
instaluje się bez odwiedzin w karcie Downloads. To zwykła instalacja: FOMOD otwiera
swój instalator, a nazwa już obecna na liście modów pyta, co zrobić. Archiwa, które
były w `downloads/` przed otwarciem okna albo przed włączeniem opcji, są
pozostawione w spokoju, tak samo pobranie, którego instalację anulowałeś. Gdy w
panelu kolekcji jest otwarta kolekcja, żadne pobranie nie instaluje się samo, a to,
które kończy się w międzyczasie, też nie zainstaluje się samo później; kolekcja
instaluje swoich członków. Plik, na który czeka oczekujący Update, instaluje Update.
Tą drogą instalowane są tylko pobrania z Nexusa, a to, które ukrywa filtr Downloads,
czeka, aż filtr zostanie wyczyszczony. Gdy trwa przeciąganie albo gdy gra lub inny
Eidos trzyma instancję, instalacja czeka.

### Aktualizacje modów

Przycisk aktualizacji blisko prawego końca paska narzędzi sprawdza każdy mod, który
ma identyfikator Nexusa. Gdy strona moda pokazuje zainstalowaną wersję, nic więcej
nie jest pytane. W przeciwnym razie Eidos czyta listę plików moda (jedno dodatkowe
żądanie, zachowane na resztę sprawdzenia) i podąża za łańcuchem aktualizacji od
identyfikatora pliku, który zapisuje `meta.ini` moda: autorzy oznaczają nowe
wgranie jako następcę starszego, a MO2 czyta ten sam łańcuch. Nowszy plik w tym
łańcuchu jest aktualizacją. Gdy łańcuch nie wskazuje żadnego, a zainstalowany plik
nadal jest wymieniony na stronie, mod liczy się jako aktualny, nawet jeśli strona
pokazuje inną wersję. Wersja ze strony decyduje tylko wtedy, gdy nie zapisano
identyfikatora pliku, gdy zainstalowany plik usunięto lub zarchiwizowano albo gdy
Nexus nie przysłał listy plików.

Każdy mod używa domeny Nexusa gry, którą nazywa jego własny `meta.ini`
(`gameName`), a nie domeny instancji: sprawdzanie aktualizacji, Endorse, Track i
Visit on Nexus idą tam. Enderal SE przyjmuje też mody `skyrimspecialedition`, więc
mod Skyrim SE zainstalowany w instancji Enderal SE jest sprawdzany na swojej stronie
Skyrim SE. Link Mod Manager Download trafia do instancji, której użyłeś ostatnio,
gdy jej gra przyjmuje domenę linku. W przeciwnym razie trafia do instancji gry,
której własną domeną jest ta domena, przed instancją gry, która tylko ją przyjmuje.
`EIDOS_INSTANCE=<folder>` nadpisuje oba.

Track i Untrack w menu prawego przycisku moda dodają moda do twoich śledzonych na
Nexusie lub go usuwają, więc powiadomienia strony podążają za tym, co śledzisz tutaj.
Endorse i Abstain w tym samym menu polecają moda na Nexusie albo wycofują polecenie;
przycisk endorse obok przycisku aktualizacji poleca zaznaczonego moda. Każde
sprawdzenie aktualizacji czyta też twoje polecenia i śledzone mody z Nexusa (dwa
żądania) i ustawia flagi Endorsed i Tracked każdego moda tak, żeby się zgadzały, w
domenie Nexusa danego moda. Gdy oba źródła się różnią, wygrywa Nexus. Jeśli któraś
lista nie da się odczytać, sprawdzenie mówi o tym i zostawia te flagi bez zmian.

Przycisk Update na karcie Nexus files w „Information...” instaluje nowszy plik. Na
koncie premium Eidos sam pobiera plik. Na darmowym otwiera stronę pliku, gdzie Mod
Manager Download odsyła plik z powrotem, a instalacja startuje, gdy archiwum trafi
do `downloads/`. Stamtąd to zwykła instalacja: FOMOD otwiera się z odpowiedziami, z
którymi mod zainstalowano, potem pytanie oferuje Replace z zaznaczonym „Keep a
backup of the existing mod”, cokolwiek wybrałeś przy ostatnim pytaniu. Replace
zachowuje priorytet i stan włączenia moda.

Zanim Replace podmieni folder moda, Eidos porównuje stary folder z nowym po nazwie
pliku i rozmiarze i czyta nagłówki wtyczek w obu. Dotyczy to Update, aktualizacji
kolekcji i Replace wybranego w pytaniu instalacji. Wiersz stanu zlicza wtedy pliki
dodane, usunięte i zmienione rozmiarem oraz wymienia każdy master, którego nowa
wersja potrzebuje, a stara nie. Karta General w „Information...” kończy się sekcją
Last update, która wymienia wersje, wtyczki dodane lub usunięte, mastery dodane lub
usunięte, wtyczki, którym zmieniła się flaga ESM, ESL lub ESH, mody, z którymi ten
mod teraz dzieli pliki lub już nie dzieli, i pliki. Plik przepisany w tym samym
rozmiarze nie jest wymieniony. Raport jest zachowywany w `logs/updates/<mod>.json`
w instancji, który `eidos pack` pomija; świeża instalacja albo Merge do folderu go
usuwa. `eidos install --replace` wypisuje te same liczby w wierszu `changes:`.

Folder, który zawiera kilka plików Nexusa, taki jak plik główny z opcjonalnym
plikiem scalonym do niego, jest odrzucany: zastąpienie go nowym plikiem głównym
porzuciłoby pozostałe. Aktualizuj takie ręcznie.

`eidos nexus update skyrimse --apply` uruchamia to samo sprawdzenie, a potem na
koncie premium pobiera każdy nowy plik i instaluje go nad jego folderem z
`--replace --backup`. Na darmowym koncie wypisuje zamiast tego `OPEN <url>` dla
każdej strony pliku. Foldery scalające kilka plików są pomijane, z powodem. Mod
FOMOD bez zapisanych odpowiedzi, taki jak zainstalowany, zanim Eidos zaczął je
zachowywać, nie jest instalowany ponownie z domyślnymi opcjami instalatora: ta
aktualizacja kończy się komunikatem, żeby zaktualizować moda raz z okna, co zapisuje
odpowiedzi. FOMOD, którego instalator nie ma nic do wyboru, jest instalowany jak
zwykle. Polecenie kończy się kodem 1, jeśli którakolwiek aktualizacja się nie
udała.

### Aktualizacje kolekcji

Gdy uruchamiasz Check for updates, Eidos pyta też Nexus o najnowszą rewizję każdej
kolekcji zainstalowanej w tej instancji. Wiersz nad listą modów wymienia każdą
kolekcję z nowszą rewizją i zlicza mody dodane (+), zaktualizowane (~) i usunięte
(-); najedź na niego po ich nazwy. Liczb brakuje, gdy przepis zainstalowanej rewizji
nie jest już zapisany w pamięci. Update otwiera nowszą rewizję w panelu kolekcji,
gdzie *Install this collection* ją uruchamia. Lista jest budowana przy każdym
sprawdzeniu i nie jest zapisywana.

### Ustawienia Nexusa

Preferences -> Nexus -> Offline zatrzymuje całkowicie kontakt Eidosa z Nexusem i
pomija codzienne sprawdzanie wydań na GitHubie. Sprawdzanie aktualizacji,
logowanie, pobrania i kolekcje mówią o tym zamiast zawodzić błędem połączenia.
Domyślnie jest wyłączone.

Preferred servers ustawia, które węzły CDN preferuje pobranie, od najlepszego.
Tylko konto premium dostaje kiedykolwiek więcej niż jedno lustro do wyboru, więc dla
wszystkich innych wybiera Nexus, a to nic nie zmienia. Gdy żaden z nazwanych
serwerów nie jest oferowany, pobranie i tak się odbywa, z pierwszego węzła, który
Nexus oferuje.

### Kategorie

Kategorie można przypisać do jednego moda albo całego zaznaczenia, katalog można
edytować z tego samego dialogu, a oficjalną listę kategorii gry można pobrać z
Nexusa. Oba pliki katalogu to własne pliki MO2 (`categories.dat` i
`nexuscatmap.dat`), więc współdzielona instancja zachowuje jeden katalog. Mod
pobrany z Nexusa jest kategoryzowany przy instalacji, gdy Fetch from Nexus zmapuje
katalog.

### Edytor INI, log i rozszerzenia

View -> INI editor edytuje INI gry profilu, kopię, którą trzyma Eidos; kopia w
prefiksie Protona jest nadpisywana przy każdym uruchomieniu. Jego ostatnia karta to
`launch-env.ini` profilu
([Zmienne i wrappery per profil](#zmienne-i-wrappery-per-profil)). View -> Log
czyta logi sesji. View -> Extensions wymienia twoje własne dodatki; zob.
[extensions.md](extensions.md).

Gdy uruchomienie nie znajduje głównego INI gry (`Skyrim.ini`, `Fallout4.ini`,
`Oblivion.ini`...) ani w profilu, ani w prefiksie, Eidos najpierw kopiuje domyślne
INI z folderu gry (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) do profilu, jak
robi to MO2. Gdy instancja uruchamia kopię gry, domyślne pochodzi z kopii. Prefiks i
folder gry nie są zmieniane.

### Instalowanie modów

Instalator obsługuje proste archiwa, FOMOD, paczki BAIN Wrye Basha (zaznacz
podpaczki, które scalają się po kolei) i paczki OMOD z Oblivion. Skrypt OMOD
działa, gdy używa poleceń OBMM obsługiwanych przez Eidosa albo jest jednym z trzech
znanych Eidosowi instalatorów w C#; Eidos nie uruchamia żadnego innego kodu C# ani
Visual Basic i mówi o tym. Gdy żadna heurystyka nie rozpozna układu, ręczny wybór
pokazuje drzewo archiwum i pozwala wskazać korzeń danych.

Ponowna instalacja lub aktualizacja moda FOMOD otwiera kreator z zaznaczonymi
odpowiedziami z jego ostatniej instalacji, na pierwszym kroku, którego opcje zmieniły
się od tamtej pory, z wymienionymi zmianami. Reinstall with new choices zaczyna
zamiast tego od domyślnych opcji instalatora. `eidos install --replace` lub `--merge`
używa zapisanych odpowiedzi i odmawia, wymieniając zmiany, gdy już nie pasują.

### Diagnostics

Zakładka Diagnostics uruchamia kontrole stanu na żywo. Zgłasza brakujące mastery,
wtyczki oznaczone przez LOOT jako brudne, archiwa, których żadna aktywna wtyczka nie
wczyta, czy lista modów nadal zgadza się z folderem modów, czy passthrough FUSE jest
włączony i, gdy jest zażądany, czy jest uprawnienie uruchomienia, którego potrzebuje.
Po uruchomieniu pokazuje, co własny log script extendera mówi o każdej z jego
bibliotek DLL wtyczek, więc widać, czy twoje wtyczki SKSE się wczytały. Zgłasza też
aktualizację gry od ostatniego czystego uruchomienia, brakujący script extender albo
taki, który nie pasuje do wersji gry (ze stroną pobierania extendera i przyciskiem
instalacji pobranego archiwum), opcję uruchamiania Steama, która nie uruchamia
Eidosa z `%command%`, NVAPI pozostawione wyłączone dla DLSS, pełny ekran wyłączny,
który zatrzymuje generowanie klatek, `enblocal.ini` ENB bez `LinuxVersion=true`,
deklaracje wtyczek F4SE i SFSE oraz log awarii z ostatniej sesji z modami, które
wymienia.

Kontrole INI profilu i wtyczek szukają wierszy `sTestFile` w `[General]` własnych INI
Fallouta 4, Fallouta 4 VR i Starfield, które sprawiają, że gra ignoruje `plugins.txt`
(Fix usuwa je z kopii profilu i zachowuje poprzedni plik jako `<ini>.pre-fix`),
włączonych wtyczek `.esp` w Starfield oraz wtyczek blueprint, którym brakuje flagi
blueprint, prefiksu `BlueprintShips-` albo rozszerzenia `.esm`, albo których głównej
wtyczki nie włączono. Kontrole systemu oflagowują `vm.max_map_count` lub twardy limit
otwartych plików poniżej tego, czego potrzebuje Proton, oraz grę, prefiks lub
instancję na NTFS, exFAT lub FAT. Kontrole modów wskazują ścieżkę danych BodySlide,
gdy nie prowadzi do `Data` gry, foldery `Data` zapisane przez narzędzie w
`Overwrite/Root`, łatkę, której luźne pliki przegrywają z modem, który łata, oraz
moda, który zastępuje skrypty Papyrus script extendera.

Check for updates czyta też wymagania Nexusa każdego moda. Diagnostics wymienia
wymagany mod, którego brakuje albo który jest wyłączony, z Ignore dla wymagania
spełnionego inaczej.

Zakładka pokazuje też, który build Protona uruchamia grę, i ostrzega, zanim starszy
Proton uruchomi prefiks, który przygotował nowszy; Rebuild odkłada zepsuty prefiks na
bok, żeby Steam zrobił nowy
([troubleshooting.md](troubleshooting.md#który-proton-uruchamia-grę-i-zepsuty-prefiks)).
Inne karty oflagowują prefiks, którego nigdy nie przygotowano lub którego rejestr jest
nieaktualny, program inny niż Eidos odbierający linki Mod Manager Download (Register
to naprawia), wynik narzędzia starszy od swoich danych wejściowych (z przyciskiem,
który uruchamia narzędzie ponownie) oraz oczyszczoną lub lekką kopię wtyczki, której
oryginał zmienił się od tamtej pory. Każda karta jest wyjaśniona w
[troubleshooting.md](troubleshooting.md#karty-health). Copy report umieszcza w
schowku raport systemu i każdego ustalenia, z ukrytym folderem domowym i loginem, do
zgłoszenia błędu.

Dla Skyrim SE, Enderal SE i Fallouta 4 Diagnostics ma też kartę Display. Pokazuje
rozmiar i tryb okna ustawione w `SkyrimPrefs.ini` profilu (`EnderalPrefs.ini`,
`Fallout4Prefs.ini`), albo w kopii z prefiksu, gdy profil jeszcze jej nie ma. Gdy żadne
nie ustawia rozmiaru, jej pola zaczynają od największego podłączonego ekranu
wymienionego w `/sys/class/drm`, więc Steam Deck czyta 1280x800. Szerokość i wysokość
to liczby całkowite od 320 do 16384. Apply zapisuje `iSize W`, `iSize H`,
`bFull Screen` i `bBorderless` w kopii profilu, po przejęciu kopii z prefiksu, jeśli
profil jej nie ma. Gdy okno ustawia SSE Display Tweaks, Apply zapisuje ten sam rozmiar
i tryb w jego kopii w Overwrite; własny plik moda nie jest zmieniany.

Kartę Game files na górze zakładki opisano w
[Pliki gry](#pliki-gry-instalacja-ze-sklepu-albo-kopia), kartę kolejności kolekcji w
[Zachowanie kolejności kolekcji](#zachowanie-kolejności-kolekcji), a Find the mod behind
a problem w [Uruchomienia testowe](#szukanie-moda-stojącego-za-problemem).

### Uruchamianie gry

Żeby uruchomić grę przez okno, ustaw opcję uruchamiania gry w Steamie na bezwzględną
ścieżkę programu, bo Steam nie czyta `PATH` twojej powłoki
([install.md](install.md#następnie-wskazać-go-steamowi)):

```
~/.local/bin/eidos-gui %command%
```

Z Colony ścieżka to `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos otwiera
się na instancji gry, tej, której ostatnio używałeś, więc instancja przenośna jest
znajdowana ponownie tak samo jak globalna; kliknij Run, żeby uruchomić ją przez
scalony widok.

Przy włączonym Start the game at once when a store opens Eidos (Preferences,
General, Running a game; domyślnie wyłączone) Eidos pomija to kliknięcie: sprawdza
Health i uruchamia grę bez pokazywania okna. Okno otwiera się jak zwykle, gdy Health
wymienia problem, gdy lista modów wymaga uwagi, gdy narzędzie ustawione do
uruchomienia przed grą ma nieaktualny wynik albo, dla Skyrim SE, Enderal SE i
Fallouta 4, gdy profil nie ma jeszcze rozmiaru okna (okno otwiera się wtedy na
Diagnostics). Żeby dostać okno na jedno uruchomienie, wstaw `EIDOS_WINDOW=1` przed
poleceniem: `EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

W oknie, którego nie otworzył żaden sklep, Run nie uruchamia gry sam, a Eidos nigdy
nie uruchamia gry bezpośrednio przez Proton ani Wine. Run prosi sklep, do którego
należy kopia, o jej uruchomienie, a sklep uruchamia Eidosa:

- Steam uruchamia `steam -applaunch <appid>`, gdy ustawiona jest opcja uruchamiania
  wyżej, a gra ma prefiks Protona, który Steam tworzy przy pierwszym uruchomieniu
  gry. Steam startuje najpierw, jeśli jest zamknięty.
- Heroic (kopia GOG lub Epic, natywna lub Flatpak) dostaje link `heroic://launch`,
  gdy Eidos jest wrapperem w ustawieniach gry w Heroic, z bezwzględną ścieżką
  `eidos-gui` (lub `eidos-linux`) jako plikiem wykonywalnym i bez argumentów. Heroic
  startuje najpierw, jeśli jest zamknięty.
- Samo Legendary uruchamia `legendary launch <app>`, gdy
  `~/.config/legendary/config.ini` ma `wrapper =` i bezwzględną ścieżkę `eidos-gui`
  (lub `eidos-linux`) w sekcji `[<app>]` gry albo w `[default]`.

Loader script extendera, launcher i plik wykonywalny gry na liście Run są w ten
sposób przekazywane sklepowi, tak samo jak `eidos tool <instance> run <entry>` dla
jednego z nich w terminalu. Eidos nie potrafi przekazać niektórych kopii ich
sklepowi, takich jak kopia GOG poza Heroic: Run mówi wtedy, żeby uruchomić grę z tego
sklepu.

Przekazanie zapisuje tę instancję jako tę, której użyłeś ostatnio, a sklep otwiera
potem drugie okno Eidosa na niej. Przy wyłączonym Start the game at once klikasz tam
Run; przy włączonym to okno uruchamia grę bez pokazywania się, chyba że otworzy je
któryś z przypadków wyżej. Pierwsze okno nie podąża za tym uruchomieniem, więc po
grze naciśnij w nim F5, żeby przeładować listy. Gdy brakuje ustawienia sklepu albo
loadera script extendera nie ma obok pliku wykonywalnego gry, Run niczego nie
uruchamia i pokazuje, co ustawić i gdzie, z prawdziwą ścieżką działającego pliku.

Gdy gra lub narzędzie działa, Run zmienia się w Stop. Dwa naciśnięcia kończą każdy
program uruchomiony przez to uruchomienie, a Eidos potem przechwytuje sesję i
odmontowuje jak po każdym wyjściu; zatrzymane uruchomienie nigdy nie liczy się jako
czyste
([troubleshooting.md](troubleshooting.md#gra-lub-narzędzie-które-się-nie-zamyka)).

`%command%` Steama dla tytułów Bethesdy zwykle wskazuje `<Game>Launcher.exe`. Eidos
nigdy go nie uruchamia: launcher to osobna aplikacja ustawień, która skanuje `Data`
ponownie i przepisuje `plugins.txt`, cofając dopiero co wdrożoną kolejność
wczytywania. Eidos uruchamia zamiast niego loader script extendera, jeśli jest
zainstalowany, w przeciwnym razie plik wykonywalny gry, i mówi o tym, gdy musi się
cofnąć do pliku wykonywalnego gry.

Eidos skanuje biblioteki DLL włączonych modów w poszukiwaniu importu
`d3dcompiler_47.dll`, kopiuje DLL Microsoftu do prefiksu i dopiero potem ustawia
`WINEDLLOVERRIDES=d3dcompiler_47=n,b`; nie musisz ustawiać tego sam.

### Zmienne i wrappery per profil

Profil może nieść własne zmienne środowiskowe i polecenia wrapperów w
`profiles/<name>/launch-env.ini`, po jednym wpisie w wierszu:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Stosują się do każdego `eidos play` tego profilu (gra uruchomiona przez `%command%`
Steama albo przez Run w oknie otwartym przez sklep) i do każdego uruchomienia
narzędzia. `eidos shell` dostaje zmienne, a nie wrapper. Pierwszy wiersz `wrapper=`
jest najbardziej zewnętrzny. Wiersz `wrapper=`, który zawiera `%command%`, dostaje
polecenie w tym miejscu, jak w opcjach uruchamiania Steama; wiersz bez niego dostaje
polecenie na końcu. Wrappery działają wewnątrz prywatnej przestrzeni nazw Eidosa,
więc widzą mody.

Zmienna tutaj zastępuje wartość, którą Eidos ustawia dla tej samej nazwy, a własny
wiersz `env=` narzędzia w `tools.ini` zastępuje oba. `WINEDLLOVERRIDES` jest
dodawane po nadpisaniach, które Eidos ustawia dla ENB, ReShade i dołączonych DLL, więc
DLL nazwana w obu bierze ustawienie profilu, a pozostałe zachowują swoje.

View -> INI editor otwiera plik jako ostatnią kartę, dla każdej gry. Puste wiersze i
wiersze zaczynające się od `#` lub `;` są pomijane. Zapis wymienia numery wierszy,
które Eidos ignoruje: wiersze, które nie są ani `env=NAME=value`, ani
`wrapper=command`, oraz wiersz `wrapper=`, którego pierwsze słowo to `NAME=value`, jak
dopuszczają opcje uruchamiania Steama; umieść tę zmienną w wierszu `env=`.
`eidos play <instance>` bez polecenia wymienia zmienne, które dostaje uruchomienie, i
wrappery. Copy report w Diagnostics wymienia zmienne i programy wrapperów, bez ich
wartości.

## Narzędzia

xEdit, BodySlide, DynDOLOD i inne narzędzia działają przez scalony widok wewnątrz
prefiksu Wine gry (prefiksu Protona ze Steama albo tego, który Heroic założył dla
kopii GOG lub Epic):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # run an exe once without registering it
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen i Synthesis są
znajdowane bez rejestrowania. Pandora, Nemesis, FNIS, BodySlide i Synthesis
przychodzą z już ustawionym modem przechwytującym, więc ich wynik trafia do
własnego moda (szczegóły w
[tools.md](tools.md#kierowanie-wyjścia-narzędzia-do-własnego-moda)).

Tytuł, który nadasz narzędziu, decyduje, jakie runtime'y i biblioteki DLL Eidos mu
zapewnia: `BodySlide` dostaje swoje biblioteki DirectX, `BS` nie dostaje nic. W oknie
dialog Executables pokazuje prawdziwy stan każdego wymagania wstępnego pod polem, a
brakujące są przyciskami.

Pakiet wydania zawiera instalatory vcrun2022 (x86 i x64), .NET 8 Desktop Runtime
8.0.12 i runtime .NET 10 10.0.10 w `runtimes/`, a `install.sh` kopiuje je do
`~/.local/share/Colony/Eidos/runtimes/`, więc `eidos prereqs <instance> --install`
instaluje je bez pobierania. Eidos uruchamia instalatory vcrun2022 i .NET 8 od
Microsoftu po cichu w prefiksie gry, bez winetricks. `eidos-linux` i kompilacje ze
źródeł nie mają folderu `runtimes/`, więc pobierają te same przypięte pliki od
Microsoftu. SDK .NET 10, którego potrzebuje Synthesis (około 300 MB), jest zawsze
pobierane.

Tabela, trzy poziomy wymagań wstępnych, dlaczego DynDOLOD potrzebuje runtime'u .NET,
którego winetricks nie umie zainstalować, i dlaczego narzędzie zainstalowane jako mod
jest uruchamiane ze scalonej ścieżki, a nie z własnego folderu, są w
[tools.md](tools.md).

## Rozszerzenia

Eidosa można rozszerzać bez przebudowy: manifest TOML w
`~/.config/Colony/Eidos/addons/` dodaje narzędzie do listy Extensions albo kontrolę do
zakładki Diagnostics. Nic nie jest ładowane do Eidosa: rozszerzenie to program, który
on uruchamia. Zob. [extensions.md](extensions.md).

## Wypróbuj proof of concept

Skrypt nie potrzebuje gry. Pokazuje scalony widok, copy-on-write, folder gry
pozostawiony nietknięty i scalenie, które istnieje tylko wewnątrz swojej przestrzeni
nazw, używając wyłącznie nieuprzywilejowanego OverlayFS w przestrzeni nazw
użytkownika (Linux 5.11 lub nowszy):

```sh
./scripts/poc-overlay.sh
```
