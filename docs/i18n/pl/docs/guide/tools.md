<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Narzędzia: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Narzędzie uruchomione przez Eidos widzi scalony widok, wewnątrz prefiksu Wine
gry (prefiksu Protona ze Steama albo tego, który założył Heroic). Czyta to, co
przeczyta gra (każdy włączony mod, w kolejności priorytetów), a cokolwiek zapisze,
ląduje w Overwrite, gdzie jedno kliknięcie zmienia to w prawdziwego moda. Narzędzie
z `runner=native` działa bez Protona; zob.
[niżej](#zmienne-środowiskowe-i-narzędzia-uruchamiane-bez-protona).

## Te, które Eidos znajduje sam

Niektóre narzędzia mają nazwy na tyle jednoznaczne, że da się je znaleźć zamiast
deklarować, a xEdit jest oczywistym przypadkiem: `FO4Edit.exe` dla Fallouta 4,
`SSEEdit.exe` dla Skyrim SE, `TES5Edit.exe` dla oryginału i tak dalej. Każdy ma
wpis QuickAutoClean, który czyści brudne edycje, przed którymi ostrzega LOOT.
`PGPatcher.exe` jest znajdowany tak samo w każdej grze Bethesdy poza Morrowindem
i przychodzi z argumentem, bez którego nie zadziała (zob.
[niżej](#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck)). Eidos szuka ich, po nazwie
pliku, w:

- `Overwrite/Root/`, folderze gry, z którego działa instancja (instalacja ze
  sklepu albo jej kopia gry) oraz w drzewach `Root/` włączonych modów;
- `mods/` tej instancji, czyli tam, gdzie użytkownicy MO2 instalują narzędzia;
- folderze narzędzi ustawionym w Preferences (General -> Tools), dla katalogu
  współdzielonego między instancjami, takiego jak `/mnt/Games/Tools`.

Popularne generatory są znajdowane w ten sam sposób, każdy dla gier, które obsługuje: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen i Synthesis. Tak samo BethINI Pie (`Bethini.exe`), dla tych samych gier co PGPatcher; zob. [BethINI Pie](#bethini-pie). W Stardew Valley SMAPI (`StardewModdingAPI.exe`) jest znajdowany tak samo. Jest też loaderem script extendera tej gry, więc uruchamia grę przez sklep, jak opisano w [Dotarcie do narzędzia](#dotarcie-do-narzędzia-ukrywanie-przypinanie-i-skrót-na-pulpicie).

Lista jest osobna dla każdej gry, więc instancja Skyrima nigdy nie dostanie
edytora Fallouta. Szukanie zatrzymuje się cztery poziomy w głąb, bo pula modów to
setki tysięcy plików, a dzieje się to za każdym razem, gdy budowana jest lista
narzędzi, i nie podąża za dowiązaniami symbolicznymi. Narzędzie znalezione w ten
sposób jest skonfigurowane dokładnie tak jak wpisane ręcznie: jego wymagania
wstępne, argumenty (`--ignore-mo2vfscheck` PGPatchera) i mod przechwytujący
wynikają z jego tytułu, według reguł niżej.

Jeśli narzędzie jest gdzie indziej albo chcesz innych argumentów, dodaj je ręcznie.
Wpis użytkownika o tym samym tytule zastępuje wszystko, co zostało znalezione
automatycznie.

## Czyszczenie pluginu, który LOOT oznacza jako brudny

Po sortowaniu LOOT-em zakładka Diagnostics wymienia wtyczki, które LOOT oznacza
jako brudne, dla gier, które mają zarówno xEdit, jak i dane czyszczenia LOOT
(Skyrim LE, SE i VR, Enderal SE, Fallout 3, New Vegas, Fallout 4 i 4 VR,
Oblivion, Starfield). Gdy Eidos znajdzie xEdit, zarejestrowany w Executables albo
gdziekolwiek, gdzie w instancji szuka narzędzi, karta ma Clean all, a każda brudna
wtyczka ma w menu wiersza Clean with QuickAutoClean; w przeciwnym razie karta
mówi, żeby dodać xEdit w Executables. Eidos nie pobiera xEdit. Uruchamia tryb
QuickAutoClean xEdit na jednej wtyczce naraz, przez scalony widok, z
`-qac -autoexit -autoload`. xEdit zapisuje oczyszczoną wtyczkę nad tą, którą
wczytał, co w scalonym widoku jest nowym plikiem w Overwrite; Eidos przenosi go
do moda o nazwie `Cleaned plugins`, tworzonego jako włączony na dole listy, gdzie
wygrywa każdy konflikt. Wtyczka w swoim własnym modzie ani w folderze Data gry
nigdy nie jest zapisywana. Wyłącz `Cleaned plugins` albo ukryj w nim jeden plik,
żeby cofnąć czyszczenie. Wtyczka leżąca w Overwrite nie jest czyszczona, bo xEdit
przepisałby ten plik sam; najpierw zamień Overwrite w moda.

Kopie zapasowe, pamięć podręczna i log xEdit idą do `<instance>/logs/qac-<time>/`,
poza scalonym widokiem, a wiersz stanu podaje liczby z tego logu. Eidos usuwa ten
folder po czyszczeniu, które się udało, i zachowuje po takim, które się nie
udało. Eidos zapisuje rozmiar i datę wtyczki, z której zrobiono każdą kopię, w
`Cleaned plugins/meta.ini`. Gdy ta wtyczka się zmieni, zakładka Diagnostics
pokazuje „Out-of-date copy of <plugin>”, z przyciskiem, który ukrywa starą kopię.

## Dodawanie narzędzia

W GUI: Tools na pasku menu albo Executables na pasku narzędzi, potem Add. Z
wiersza poleceń:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` przechwytuje wynik tego uruchomienia do `mods/<mod>`,
cokolwiek mówi własne ustawienie narzędzia. `rm` usuwa tylko wpisy, które
dodałeś; domyślny wpis danej gry można ukryć, nie usunąć.

Plik exe uruchomiony z `--exe`, którego nazwa pliku jest znana Eidosowi dla gry
(bez względu na wielkość liter), dostaje tytuł tego narzędzia, a z nim wymagania
wstępne, argumenty i mod przechwytujący opisane niżej. `GenerateFNISforUsers.exe`
to FNIS, więc powyższy wiersz z `--exe` prosi o `dotnet48` i przechwytuje do
`FNIS Output`. Każdy inny exe dostaje tytuł od nazwy pliku. W zakładce Data wiersz
`.exe` ma Add tool, które otwiera okno Executables z wypełnionym wpisem, oraz Run,
które uruchamia plik jednorazowo, tak jak `--exe`. Oba nadają plikowi ten sam
tytuł.

Dla polecenia, które nie jest narzędziem Windows, `eidos shell skyrimse` otwiera
twój `$SHELL` (`/bin/sh`, gdy nie jest ustawiony) w folderze gry z zamontowanym
scalonym widokiem. To, co powłoka zapisze, ląduje w Overwrite, a wyjście z
powłoki odmontowuje widok. `eidos shell skyrimse -- <command>` uruchamia zamiast
tego jedno polecenie. Ctrl+C w interaktywnej powłoce zatrzymuje wpisane
polecenie i zostawia powłokę otwartą. Powłoka działa bez Protona; program Windows
idzie przez `eidos tool`.

W oknie Stop kończy uruchomienie narzędzia, które się zawiesiło: naciśnij go dwa
razy, a Eidos zakończy programy uruchomienia, po czym obsłuży jego pliki tak jak
po każdym wyjściu. Przy włączonym Notify the desktop when a long job ends
(Preferences, General, Running a game) uruchomienie narzędzia, które kończy się,
gdy okno jest w tle, wysyła powiadomienie pulpitu.

Loader script extendera, plik wykonywalny gry i launcher są wykrywane
automatycznie; rejestrować trzeba tylko dodatkowe narzędzia. W instancji
działającej z kopii gry są wykrywane w kopii; zob.
[Narzędzia na kopii gry](#narzędzia-na-kopii-gry).

### Wskaż prawdziwy plik, gdziekolwiek jest

Zarejestruj plik wykonywalny tam, gdzie naprawdę leży. Jeśli narzędzie
zainstalowano jako mod, to wewnątrz folderu moda:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

To ścieżka instancji globalnej. Dla instancji przenośnej ta sama reguła obowiązuje
pod jej własnym folderem, `<instance>/mods/...`. Taka bezwzględna ścieżka jest
jedyną rzeczą, która nie przetrwa późniejszego przeniesienia folderu przenośnego.

Eidos przepisuje tę ścieżkę na scaloną przed uruchomieniem, więc narzędzie działa z
`<game>/Data/CalienteTools/BodySlide/` i widzi tam także pliki wszystkich innych
modów. BodySlide dostarcza pusty katalog `SliderSets`, a każde ciało, które potrafi
zbudować, pochodzi z CBBE i modów strojów. Uruchomiony z własnego folderu moda nie
znajduje niczego i wygląda na zepsuty.

MO2 robi to samo przepisywanie, z tego samego powodu: jego własny komentarz
wymienia FNIS.

Narzędzia w wyłączonym modzie nie da się przepisać, bo jego plików też nie ma w
widoku. Eidos mówi o tym i uruchamia je z jego własnego folderu.

## Narzędzia z listy Mod Organizer 2

Lista Wabbajack albo inna instancja MO2 otwarta jako instancja Eidosa zachowuje
swój `ModOrganizer.ini`, a skonfigurowane tam pliki wykonywalne można skopiować do
`tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

W oknie dialog Executables ma Import from ModOrganizer.ini, z polem do odznaczenia
przy każdym wpisie. Add selected wstawia zaznaczone wpisy na listę, a nic nie jest
zapisywane, dopóki nie naciśniesz Save. Liczy się tylko `ModOrganizer.ini`, którego
Eidos nie napisał; ten, który pisze Eidos, opisuje
[PGPatcher i PGLite](#pgpatcher-i-pglite-w-trybie-mod-organizer-2).

Ścieżki na `Z:` są ścieżkami linuksowymi i zostają bez zmian. Ścieżka na innej
literze dysku przenosi się pod folder listy, gdy go nazywa
(`D:\Lists\MyList\tools\SSEEdit.exe` staje się `<list>/tools/SSEEdit.exe`), w pliku
wykonywalnym, katalogu roboczym i argumentach. Loader script extendera, launcher i
plik wykonywalny gry stają się zwykłymi nazwami, więc działają z folderu gry,
nad którym Eidos montuje, tak jak wpisy wbudowane. `hide` z MO2 staje się Hide from
picker, a `steamAppID` staje się Steam AppID. Wpisy wskazujące gdzie indziej
(program systemowy Windows, narzędzie spoza listy) są wymienione jako pominięte,
z powodem, a tytuł już obecny w `tools.ini` nigdy nie jest zastępowany.

Zaimportowany wpis zachowuje tytuł i argumenty z MO2, a Eidos niczego nie
wypełnia z jego tytułu: żadnych wymagań wstępnych, żadnego dodatkowego argumentu i
żadnego moda przechwytującego. Ustaw je w oknie Executables. Wpis PGPatchera z
listy zrobionej w Windows zwykle nie ma `--ignore-mo2vfscheck`; dodaj go (zob.
[niżej](#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck)).

Zaimportowana ścieżka wskazująca do folderu `Stock Game` listy jest przenoszona na
instalację ze sklepu przy uruchomieniu tylko wtedy, gdy instancja działa z tej
kopii; zob. następna sekcja.

## Narzędzia na kopii gry

Gdy instancja działa z kopii gry (folder `Stock Game` lub `Game Root` listy albo
zamrożona kopia; zob.
[Pliki gry](usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia)), narzędzie widzi
pliki kopii, a nie instalacji ze sklepu. Scalony widok jest nadal zamontowany na
instalacji ze sklepu: widok root ma kopię na dole, a widok Data czyta `Data`
kopii. Eidos szuka script extendera, launchera, pliku wykonywalnego gry i znanych
narzędzi w kopii zamiast w instalacji ze sklepu.

Ścieżka narzędzia, argument albo katalog roboczy wskazujący do kopii jest
przenoszony na instalację ze sklepu przed uruchomieniem. Może być zapisany jako
ścieżka linuksowa albo ścieżka Wine `Z:`: `-D:"Z:\...\Stock Game\Data"` staje się
tą samą ścieżką pod instalacją ze sklepu. Wpisy zaimportowane z `ModOrganizer.ini`
listy tego potrzebują. Gdy instancja działa z instalacji ze sklepu, nic nie jest
przenoszone, więc taki wpis czyta gołą kopię bez twoich modów.

`installed path` gry w rejestrze prefiksu i `gamePath` w `ModOrganizer.ini`
pisanym przez Eidosa nadal wskazują instalację ze sklepu, gdzie jest scalony
widok. Jeśli folderu kopii brakuje albo nie zawiera pliku wykonywalnego gry,
uruchomienie narzędzia zatrzymuje się, tak jak uruchomienie gry, a komunikat
wymienia `eidos stock-game <instance> store`, żeby wrócić do instalacji ze
sklepu.

`eidos stock-game <instance>` pokazuje, z jakich plików gry działa instancja i na
które kopie mogłaby przełączyć. `use <folder>` i `store` przełączają je, a
`freeze --yes` kopiuje instalację ze sklepu do folderu `Stock Game` instancji i
działa z niego. Karta Game files w zakładce Diagnostics robi to samo; zob.
[Pliki gry](usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia).

## Kierowanie wyjścia narzędzia do własnego moda

Generator (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) zapisuje setki plików.
Domyślnie lądują w Overwrite razem ze wszystkim innym. Ustaw Capture output into
w edytorze Executables, a wynik tego uruchomienia trafi zamiast tego do tego moda:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide i Synthesis przychodzą z już ustawionym modem
przechwytującym: `Pandora Output`, `Nemesis Output`, `FNIS Output`,
`BodySlide Output` i `Synthesis Output`. DynDOLOD, TexGen i xLODGen przychodzą bez
niego, bo zapisują do podanego im folderu wyjściowego.

Mod jest tworzony, jeśli nie istnieje. Przenoszone są tylko pliki, które
wytworzyło to uruchomienie; wszystko, co już było w Overwrite, tam zostaje, więc
dwa narzędzia z celami przechwytywania nie zabierają sobie nawzajem wyniku.
Uruchomienie, które niczego nie zapisało, nie zostawia pustego moda.

Eidos przenosi pliki po uruchomieniu. MO2 zamiast tego kieruje warstwę zapisu na
moda, co podnosi tego moda do najwyższego priorytetu na cały czas uruchomienia:
każdy konflikt, w którym jest, się odwraca, a potem odwraca z powrotem. Zapisy
idą też prosto przez własne pliki moda, bez copy-up. Przechwytywanie osiąga ten
sam stan końcowy bez jednego i drugiego.

Jeśli docelowy mod jest wyłączony, wynik nadal jest zapisywany, ale gra go nie
zobaczy, więc narzędzie wygenerowałoby te same pliki przy następnym uruchomieniu.
Eidos ostrzega, gdy tak jest.

### Gdy jego wynik jest nieaktualny

Eidos zapisuje, z jakich modów, kolejności wtyczek i profilu powstał wynik
narzędzia, rozmiar i datę pliku wykonywalnego narzędzia oraz ścieżkę i argumenty
wpisu w `tools.ini`. Gdy któreś z nich się zmieni, zakładka Diagnostics pokazuje
„Generated output inputs changed”, a karta ma przycisk Run <tool> again, który
uruchamia narzędzie tak, jak jest teraz skonfigurowane.

Dla generatora, którego nieaktualny wynik psuje grę, takiego jak Pandora czy
Nemesis, dodaj `check_before_play=1` do jego wpisu w `tools.ini` albo włącz dla
niego Ask before playing when its output is out of date w oknie Executables. Run
pyta wtedy najpierw: uruchomić narzędzie czy grać mimo to. Play anyway jest
pamiętane, dopóki mody, wtyczki lub profil znów się nie zmienią albo dopóki okno
się nie zamknie. Eidos nigdy sam nie uruchamia narzędzia. Przy włączonym Start the
game at once when a store opens Eidos takie pytanie zostawia okno otwarte zamiast
uruchamiać grę.

## Wymagania wstępne i argumenty wynikają z tytułu

Tytuł, który nadasz narzędziu, decyduje, jakie wymagania wstępne środowiska
uruchomieniowego Eidos mu zapewnia. Dopasowanie to podciąg tytułu bez względu na
wielkość liter.

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

Narzędzie zarejestrowane jako `BodySlide` dostaje więc swoje biblioteki DirectX;
ten sam plik wykonywalny zarejestrowany jako `BS` nie dostaje nic i może się nie
uruchomić z błędem, który nic nie mówi o DLL. Nazywaj narzędzia od programu.

Lista jest w `default_prereqs` (`crates/eidos-instance/src/tools.rs`), a pole
`Prereqs` w oknie Executables można edytować: tytuł wypełnia tylko wartość
domyślną.

Tytuł decyduje o argumentach tak samo, dla jedynego narzędzia, które potrzebuje
takiego, jakiego nikt by nie zgadł: tytuł zawierający `pgpatcher` (albo
`parallaxgen`, jego dawną nazwę) dostaje `--ignore-mo2vfscheck`. Dlaczego i
dlaczego nie jest to opcjonalne, opisano
[niżej](#dlaczego-pgpatcher-potrzebuje---ignore-mo2vfscheck). Mod przechwytujący też
wynika z tytułu (zob. [wyżej](#kierowanie-wyjścia-narzędzia-do-własnego-moda)).

### Trzy rodzaje wymagań wstępnych

Poziom 1 to dołączone biblioteki DLL: `d3dx9_43`, `d3dcompiler_47`,
`d3dcompiler_43` i `d3dx11_43`. Eidos nosi je wewnątrz swoich plików wykonywalnych
i kopiuje do prefiksu przy uruchomieniu. W 64-bitowym prefiksie, domyślnym dla
Protona, wersja 64-bitowa trafia do `system32`, a 32-bitowa do `syswow64`. Nic do
zrobienia, żadnej sieci. Każda wersja jest przechowywana skompresowana gzipem, więc
osiem plików zajmuje 7,9 MB każdego pliku wykonywalnego zamiast 17,9 MB, a
rozpakowywany jest tylko ten kopiowany (najwyżej 4,7 MB).

Poziom 2 to instalatory: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` i `xact_x64`. Zapisują one
klucze rejestru, GAC i hosty CLR, więc nie da się ich skopiować jako plików. Eidos
instaluje trzy z nich bez winetricks: `vcrun2022` uruchamia
`VC_redist.x86.exe` i `VC_redist.x64.exe` Microsoftu, a `dotnet8` i
`dotnetdesktop8` uruchamiają instalator .NET 8.0.12 Desktop Runtime, który niesie
też runtime .NET. Pakiet wydania (`eidos-linux-bundle.tar.gz`) zawiera te
instalatory w folderze `runtimes/`, a `install.sh` kopiuje je do
`~/.local/share/Colony/Eidos/runtimes/` dla użytkownika, który go uruchamia, także
z `--system`. Eidos szuka każdego pliku najpierw tam, potem w folderze `runtimes/`
obok pliku wykonywalnego `eidos` (rozpakowany pakiet uruchamiany na miejscu). Bez
żadnej z tych kopii (pojedynczy plik `eidos-linux`, kompilacja ze źródeł albo inny
użytkownik instalacji systemowej) Eidos pobiera je od Microsoftu
(`download.visualstudio.microsoft.com`) do `~/.local/share/Colony/Eidos/runtimes/`
i tam zachowuje. Tak czy inaczej plik jest odrzucany, jeśli jego SHA-256 nie
zgadza się z wbudowanym w Eidosa, i działa bez okna
(`/install /quiet /norestart`) w prefiksie gry, z buildem Protona lub Wine, który
uruchamia narzędzia gry. Tak jak winetricks, `vcrun2022` najpierw ustawia
biblioteki DLL Visual C++ na `native,builtin` w nadpisaniach DLL prefiksu. Potem,
z buildem Protona, Eidos sprawdza, czy runtime C++ prefiksu nie jest w połowie
Microsoftu, a w połowie Protona. Gdy jest, instalacja kończy się błędem:
komunikat wymienia pary DLL pochodzące z różnych buildów i mówi, żeby zamiast tego
zainstalować `vcrun2022` przez winetricks, który sam zastępuje `msvcp140.dll`.
Instalowany jest tylko 64-bitowy .NET 8; narzędzie z 32-bitowym .NET 8 nadal
potrzebuje `winetricks dotnetdesktop8`.

Pozostałe sześć czasowników idzie przez systemowy `winetricks`, który musi być w
`PATH`. Eidos uruchamia go z `wine` buildu Protona lub Wine, który uruchamia
narzędzia gry, na prefiksie gry, co omija kontener pressure-vessel Steama i
niezgodność protontricks + Proton-GE. Niektóre czasowniki winetricks potrzebują
`cabextract`, a Eidos ostrzega, gdy go brak. Narzędzie, które deklaruje
niezainstalowany czasownik poziomu 2, nadal się uruchamia, z ostrzeżeniem
wymieniającym czasownik i polecenie naprawy, bo mogłeś zainstalować go inaczej.

Czasownik poziomu 2 liczy się jako gotowy, gdy `winetricks.log` samego prefiksu go
wymienia, bez względu na to, kto uruchomił winetricks lub protontricks, albo gdy
Eidos zainstalował go do tego samego prefiksu. Eidos zapisuje własne instalacje w
`prereqs.done` w instancji, związane ze ścieżką i i-węzłem prefiksu, więc
odbudowany prefiks pokazuje swoje czasowniki znów jako oczekujące. Rebuild prefix,
akcja Diagnostics dla kopii ze Steama, zmienia nazwę `compatdata/<appid>` na
`<appid>.eidos-backup-<time>` i niczego nie usuwa; Steam tworzy nowy prefiks przy
następnym uruchomieniu gry.

Poziom 3 to runtime'y, `dotnet10` i `dotnetsdk10`. Nowoczesny runtime .NET to 193
pliki, które żyją we własnym katalogu i są znajdowane przez `DOTNET_ROOT`. Nigdy
nie jest rejestrowany ani instalowany do prefiksu, więc żaden z pozostałych
poziomów nie może go nieść. Eidos trzyma go poza jakąkolwiek instancją, bo 78 MB
nie jest ani per gra, ani per profil: każdy runtime jest rozpakowywany do
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos szuka zipa runtime'u pod jego nazwą pobierania w
tych samych dwóch folderach co instalatory poziomu 2 i rozpakowuje kopię, którą
dostarcza pakiet wydania (`dotnet-runtime-10.0.10-win-x64.zip`, tylko dla
`dotnet10`), albo ją pobiera. Pakiet nie zawiera `dotnet-sdk-10.0.403-win-x64.zip`
dla SDK, ale kopia włożona tam ręcznie jest używana i oszczędza pobierania. Eidos
odrzuca każdy z tych plików, jeśli jego SHA-256 nie zgadza się z wbudowanym w
Eidosa. Rozpakowuje zip przez `7z`, ten sam 7-Zip, którego używa do archiwów modów.

`dotnetsdk10` to SDK .NET 10, pobranie 301 MB, które po rozpakowaniu zajmuje
808 MB. Synthesis potrzebuje go do kompilowania swoich patcherów i wywołuje gołe
`dotnet`, żeby je budować i uruchamiać, więc narzędzie deklarujące `dotnetsdk10`
dostaje też folder SDK w `WINEPATH`, plus `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` i `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Budowanie patchera zostawiłoby uruchomione węzły MSBuild i serwer VBCSCompiler, a
pod Wine utrzymują one przy życiu wineserver prefiksu, więc narzędzie dostaje też
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` i
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` pozwala patcherowi
zbudowanemu pod .NET 8 (Mutagen przed 0.54) działać na runtime .NET 10 z SDK, gdy
nie zostanie znaleziony żaden runtime .NET 8. Wpis Synthesis w `tools.ini`, który
wciąż ma starą wartość domyślną `dotnet8`, `vcrun2022`, dostaje `dotnetsdk10`,
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

Eidos nie instaluje niczego z poziomów 2 ani 3, dopóki o to nie poprosisz:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

W GUI te same stany siedzą pod polem Prereqs, a brakujące są przyciskami.
Czasownik, który nie jest ani dołączony, ani runtime'em, ani czasownikiem poziomu
2, jest zgłaszany jako prawdopodobna literówka, a nie oferowany do pobrania.

`eidos prereqs <instance> --install` odmawia, dopóki gra, Steam albo narzędzie
nadal trzyma prefiks. Runtime'y poziomu 3 instalują się nawet zanim prefiks
istnieje. Instalatory uruchamiane przez samego Eidosa potrzebują istniejącego
prefiksu: zanim sklep uruchomi grę choć raz, zatrzymują się z komunikatem „start
the game once through its store, then install again". Czasowniki poziomu 2 są
instalowane po jednym, a każdy sukces jest zapisywany, więc po błędzie to samo
polecenie wznawia pracę. Dla kopii GOG lub Epic bez ustawień Heroic `--install`
kopiuje dołączone DLL, a potem zatrzymuje się przed czasownikami poziomu 2: mówi,
żeby użyć winetricks tego runnera, i czyta `winetricks.log` prefiksu przy
następnym sprawdzeniu.

### Dlaczego DynDOLOD potrzebuje `dotnet10`

DynDOLOD sam nie buduje LOD obiektów: wywołuje LODGen i dostarcza trzy jego
wersje. `LODGenx64.exe` celuje w .NET Framework 4.8, który pod Protonem jest
kierowany do Mono z Wine, a inicjalizator `System.Uri` w Mono wywołuje metodę,
której Mono nie implementuje. LODGen umiera przed pierwszą linią pracy, zostawiając
log z samym banerem wersji i nic więcej, oraz okno DynDOLOD, które mówi tylko
„failed for one or more worlds".

Zainstalowanie prawdziwego .NET Framework tego nie naprawia: Proton zastępuje
`mscoree.dll`, loader, który by go znalazł, dowiązaniem do własnego drzewa i robi
to znowu przy każdej aktualizacji prefiksu.

Wersja, która działa, to `LODGenx64Win10.exe`, celująca w nowoczesny .NET i nigdy
nie dotykająca `mscoree`. Skieruj `DOTNET_ROOT` na runtime .NET 10, a ruszy. To
właśnie zapewnia `dotnet10`, a Eidos ustawia tę zmienną przy uruchamianiu każdego
narzędzia, które go deklaruje.

### Dlaczego Pandora potrzebuje `dotnet10`

Pandora to aplikacja Avalonia, nie WPF, i celuje w `net10.0` od Pandory 4.1.0,
więc sam runtime .NET 10 ją uruchamia. Gdy nie znajdzie żadnego runtime'u, mówi,
żeby zainstalować .NET Desktop Runtime. To ogólny tekst, który launcher .NET
pokazuje dla każdego programu z GUI, który nie znajdzie runtime'u, a
`dotnetdesktop8` tego nie naprawia.

Eidos 1.21.0 i starsze dawały Pandorze `dotnetdesktop8`, a wpis zachowuje swój
wiersz `prereqs=`. Gdy Eidos czyta wpis Pandory, który nadal wymienia tylko
`dotnetdesktop8`, a `.runtimeconfig.json` obok jej pliku wykonywalnego wymienia
`net10.0`, wpis dostaje zamiast tego `dotnet10`. Pandora 2.x celuje w .NET 8 i
zachowuje to, co ma.

### Dlaczego PGPatcher potrzebuje `--ignore-mo2vfscheck`

PGPatcher przepisuje siatki i wtyczki tak, żeby wszystkie twoje mody tekstur
(vanilla, parallax, complex material, PBR) używały właściwego shadera na każdej
powierzchni. Odmawia uruchomienia poza menedżerem modów, bo łatanie gołego folderu
gry byłoby destrukcyjne.

PGPatcher sprawdza usvfs, warstwę przechwytującą DLL w Windows, którą MO2 wstrzykuje
do gry. Eidos jej nie ma: scalony widok to montowanie FUSE, w które Wine wchodzi,
nie wiedząc, że istnieje. Sprawdzenie nic więc nie znajduje, a narzędzie natychmiast
kończy pracę z komunikatem

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

wymieniającym program, którego użytkownik nie uruchamia, zanim zrobi jakąkolwiek
pracę. Odpowiedzią jego autora na dwa linuksowe zgłoszenia ([#716][pg716] i
[#725][pg725], oba z Fluorine, linuksowego forka MO2, opartego na FUSE z tego samego
powodu) jest flaga pomijająca sprawdzenie.

Eidos więc ją wstawia. PGPatcher znaleziony w puli modów przychodzi z już
ustawionym argumentem, a wpisanie `PGPatcher` jako tytułu w oknie Executables
wypełnia pole Arguments tak samo. To, co wpiszesz, zawsze wygrywa: wstawianie
wypełnia tylko pole, które zostawiłeś puste, dokładnie jak wymagania wstępne
wyżej.

Lista jest w `default_args` (`crates/eidos-instance/src/tools.rs`), obok
`default_prereqs`.

Jego runtime nie potrzebuje niczego, co jest niezwykłe dla narzędzia .NET: niesie
własny (`dotnetlib/`, z `runtimeconfig.json` deklarującym `includedFrameworks`) i
dostarcza biblioteki DLL CRT Microsoftu obok swojego pliku wykonywalnego. Zostaw
Prereqs puste.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher i PGLite w trybie Mod Organizer 2

Przy Mod Manager ustawionym na `None` PGPatcher widzi jeden scalony folder `Data` i
nie potrafi powiedzieć, z którego moda pochodzi siatka lub tekstura, więc nie ma
konfliktów do pokazania ani kolejności modów do zachowania. Jego tryb
`Mod Organizer 2` czyta samą instancję: `modlist.txt` profilu, włączone mody w
kolejności priorytetów i pliki w folderze każdego moda. PGLite, wersja w grze, czyta
instancję tak samo. Instancja Eidosa ma już układ MO2 (`mods/`,
`profiles/<name>/modlist.txt`, `overwrite/`), więc ten tryb na niej działa.

W launcherze PGPatchera:

1. Ustaw Mod Manager na `Mod Organizer 2`.
2. Ustaw MO2 Options -> Instance Location na folder instancji Eidosa, zapisany
   jako ścieżka Windows na dysku `Z:` Wine, którym jest `/`. Instancja przenośna w
   `/mnt/Games/Eidos-Skyrim` to `Z:\mnt\Games\Eidos-Skyrim`; globalna to
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Zostaw `--ignore-mo2vfscheck` w argumentach narzędzia. W tym trybie PGPatcher
   szuka usvfs i zatrzymuje się, gdy go brak.

PGPatcher przyjmuje folder tylko wtedy, gdy zawiera `modorganizer.ini`, więc Eidos
zapisuje `ModOrganizer.ini` w katalogu głównym instancji za każdym razem, gdy
uruchamia grę albo narzędzie, dla uruchamianego profilu. Plik jest przepisywany
tylko wtedy, gdy zmienia się jego zawartość. Zawiera:

- w `[General]`: `gameName` (nazwa gry u MO2, więc `New Vegas` dla Fallout: New
  Vegas), `gamePath` (folder instalacji gry ze sklepu jako ścieżka `Z:`, także w
  instancji działającej z kopii gry) i `selected_profile`;
- w `[Settings]`: `mod_directory`, `profiles_directory` i `overwrite_directory`
  jako bezwzględne ścieżki `Z:`, plus `download_directory`, gdy instancja ma folder
  `downloads/`.

Jego pierwszy wiersz to komentarz mówiący, że napisał go Eidos. `ModOrganizer.ini`
bez tego wiersza, pod jakąkolwiek pisownią nazwy, nigdy nie jest zmieniany ani
usuwany: instancja MO2 otwarta w Eidosie zachowuje swój własny, a PGPatcher czyta
ten plik takim, jaki jest. MO2 gubi komentarze, gdy zapisuje swoje ustawienia, więc
plik zapisany przez MO2 od tej chwili należy do MO2. Pusty plik liczy się jako
plik Eidosa.

Taki plik wskazuje profil, który MO2 wybrał ostatnio, a PGPatcher używa tego
profilu, nie tego, który uruchamia Eidos. Gdy oba się różnią, Eidos zapisuje
ostrzeżenie w logu przy uruchomieniu. Wybierz ten sam profil w MO2 albo zmień
nazwę lub usuń plik, żeby Eidos zapisał własny.

Narzędzia w trybie Mod Organizer 2 czytają też grę z `gamePath` tego pliku. Gdy
nie wskazuje ani folderu gry, z którego działa instancja, ani instalacji ze sklepu,
zakładka Diagnostics pokazuje „ModOrganizer.ini points tools at another game
folder”, ze ścieżką i folderem gry, z którego działa ta instancja. Zmień `gamePath`
w pliku albo usuń plik, żeby Eidos zapisał własny.

Eidos zapisuje swój plik tylko wtedy, gdy coś uruchamia, więc między
uruchomieniami wskazuje profil ostatniego z nich. Narzędzie uruchamiane przez
Eidosa znajduje tam własny profil.

PGPatcher zatrzymuje się z błędem, gdy profil nie ma włączonego moda i gdy jego
folder wyjściowy jest folderem włączonego moda, tak jak pod MO2.

## Własne programy Wine w prefiksie gry

Menu File uruchamia cztery programy Wine w prefiksie gry: winecfg, regedit,
eksplorator plików Wine i deinstalator. Startują jak każde narzędzie, z buildem
Protona lub Wine, który uruchamia narzędzia gry, z zamontowanymi modami, a okno
na nie czeka. Prefix drive_c otwiera dysk `C:` prefiksu w twoim menedżerze plików.
Wiersz jest wyszarzony, dopóki prefiks nie ma tego programu, co dzieje się, gdy
sklep po raz pierwszy uruchomi grę. W terminalu `eidos tool <instance> run winecfg`
robi to samo, tak samo `regedit`, `explorer` i `uninstaller`, gdy żadne narzędzie
na liście nie ma takiego tytułu.

Zmiana `installed path` gry dokonana w regedit jest przywracana przy następnym
uruchomieniu narzędzia; zob. następna sekcja.

## Ścieżka gry w prefiksie

Narzędzia Windows znajdują swoją grę, czytając
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, klucz zapisywany przez
własny instalator gry. Steam pod Protonem nigdy nie uruchamia tego instalatora.
Bez tego klucza xEdit, Wrye Bash i DynDOLOD otwierają się z pustą ścieżką.

Przed uruchomieniem narzędzia Eidos zapisuje ten klucz importem `regedit` w
prefiksie gry, chyba że wcześniejszy import zapisał już tę samą ścieżkę, a prefiks
nadal ją ma. Klucz wskazuje instalację ze sklepu, także w instancji działającej z
kopii gry, bo tam zamontowany jest scalony widok. Dla narzędzia deklarującego
`dotnetsdk10` ten sam import dodaje certyfikaty podpisujące NuGet (zob.
[wyżej](#trzy-rodzaje-wymagań-wstępnych)). Narzędzie z `runner=native` pomija ten
krok. Gdy prefiks jeszcze nie istnieje albo trzyma go inny program, narzędzie
nadal działa, a Eidos wypisuje ostrzeżenie wskazujące sklep, z którego raz
uruchomić grę, albo program do zamknięcia. Gdy klucz wskazuje folder inny niż
instalacja ze sklepu, zakładka Diagnostics pokazuje „Windows tools cannot find
<game>”, dopóki następne uruchomienie narzędzia nie zapisze go ponownie.

## Dotarcie do narzędzia: ukrywanie, przypinanie i skrót na pulpicie

Domyślne wpisy gry zawierają narzędzia, których możesz nigdy nie używać. W oknie
Executables:

- Pin to top umieszcza wpis na początku listy Run.
- Hide from picker zdejmuje go z listy bez usuwania.
- Desktop shortcut zapisuje `.desktop` w `~/.local/share/applications`, więc
  pojawia się w menu aplikacji i w wyszukiwaniu, a nie na pulpicie. Uruchamia
  `eidos tool <instance> run <title>` bezpośrednio, więc narzędzie startuje przez
  scalony widok z profilem tej instancji, bez otwierania okna Eidosa.

Ukrywanie i przypinanie zmieniają tylko sposób dotarcia do narzędzia, więc działają
na domyślnych wpisach gry tak samo jak na twoich własnych.

Loader script extendera, launcher i plik wykonywalny gry nie mają przycisku
Desktop shortcut. Uruchamiają grę, która zawsze startuje przez swój sklep, a
własny skrót sklepu już to robi. Skrót zrobiony dla jednego z nich w Eidosie 1.22
lub starszym nadal działa: jak `eidos tool <instance> run <title>` wpisane w
terminalu, prosi sklep o uruchomienie gry, tak jak Run w oknie, którego nie
otworzył żaden sklep. Gdy brakuje ustawienia sklepu, polecenie niczego nie
uruchamia i wypisuje, co ustawić. Skrót nie pokazuje żadnego komunikatu, więc
uruchom to samo polecenie w terminalu, żeby go przeczytać.

Eidos przekazuje każdy wpis sklepowi w ten sposób, gdy jego plik wykonywalny ma
nazwę pliku loadera script extendera, launchera albo pliku wykonywalnego gry, bez
względu na wielkość liter: wpis domyślny, dodany przez ciebie, zaimportowany z MO2
albo uruchomienie `--exe`. Najpierw oznacza tę instancję jako ostatnio używaną,
żeby okno otwarte przez sklep pokazało tę instancję, potem uruchamia
`steam -applaunch <id>`, `xdg-open heroic://launch?appName=<app>&runner=<runner>`
dla Heroic albo `legendary launch <app>`. Z `--print` wypisuje to polecenie
sklepu i niczego nie uruchamia. Polecenie `steam` musi być w `PATH`: Flatpak Steam
go tam nie umieszcza, więc start się nie udaje, a komunikat mówi, żeby uruchomić
grę ze Steama.

Wpis działa bezpośrednio przez scalony widok, jak każde narzędzie, gdy proces
uruchomił sklep: ustawione jest jedno z `SteamGameId`, `SteamAppId`,
`HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` lub `EIDOS_STORE_STARTED`. Bez żadnego z
nich Eidos nie uruchamia gry sam przez Proton. Gdy instancja nie ma loadera script
extendera (w folderze gry, z którego działa, w `Root/` włączonego moda albo w
`Overwrite/Root/`), nic się nie uruchamia, a wiersz mówi, żeby najpierw
zainstalować script extender. Dla kopii GOG poza Heroic albo identyfikatora
sklepu, którego Eidos nie umie przekazać, mówi, żeby uruchomić grę z tego sklepu.

Profil może mieć własny skrót do gry. Kliknij profil prawym przyciskiem i wybierz
Desktop shortcut: wpis uruchamia
`eidos tool <instance> run "<game>" --profile <name>`, co czyni ten profil
aktywnym, tak jak wybranie go w oknie, a potem prosi sklep o uruchomienie gry.
Okno Eidosa otwarte przez sklep pokazuje ten profil. `--profile` działa z każdym
`eidos tool ... run` i wybiera profil przed startem narzędzia; z `--print` nie
wybiera niczego.

## Narzędzie, które jest własną aplikacją Steam

Creation Kit to osobna aplikacja Steama i chce własnego AppID; kilka innych
narzędzi do modowania dostarczanych przez Steam jest takich samych. Ustaw Steam
AppID we wpisie, a Eidos uruchomi je pod tym identyfikatorem zamiast pod
identyfikatorem gry.

W Windows oznacza to inny launcher. Tutaj to dwie zmienne środowiskowe w tym samym
uruchomieniu, `SteamAppId` i `SteamGameId`. Eidos ustawia obie, bo Proton czyta
jedną, a własne biblioteki Steama drugą, a narzędzie, które widzi, że się różnią,
zawodzi bez czytelnego błędu. `eidos tool ... --print` pokazuje dokładnie to, co
dostałoby prawdziwe uruchomienie.

## Zmienne środowiskowe i narzędzia uruchamiane bez Protona

Trzy klucze `<instance>/tools.ini` nie mają jeszcze pola w oknie Executables, więc
dopisz je ręcznie do sekcji narzędzia. Każdy wiersz `env=NAME=value` ustawia jedną
zmienną tylko dla uruchomień tego narzędzia:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos stosuje te wiersze po wszystkim, co ustawia sam, i po zmiennych z
`launch-env.ini` profilu, więc wartość tutaj zastępuje oba. Dotyczy to też
`WINEDLLOVERRIDES`, które Eidos buduje z bibliotek DLL dostarczanych przez twoje
mody: narzędzie, które je ustawia, musi wymienić w nim każdą potrzebną DLL.
Wrappery profilu opakowują polecenie narzędzia tak jak polecenie gry; zob.
[Zmienne i wrappery per profil](usage.md#zmienne-i-wrappery-per-profil).
Nazwa to litery, cyfry i `_` i nie zaczyna się od cyfry. Wiersz z inną nazwą jest
ignorowany.

`runner=native` uruchamia samo `exe`, bez Protona. Scalony widok jest zamontowany
jak dla każdego narzędzia, a wiersze `arg0=`, `arg1=` są jego argumentami. Użyj go
dla programu linuksowego albo dla narzędzia na kopii, którą Steam nie zarządza, a
dla której Heroic nie zapisuje runnera: `exe` to Wine tej kopii, program Windows
jest pierwszym argumentem, a prefiks pochodzi z `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` musi być ścieżką bezwzględną. Katalog roboczy domyślnie jest folderem, który
zawiera `exe`, tu folderem runnera, więc ustaw `workdir=`. Tylko `exe` jest
przepisywane na swoją scaloną ścieżkę: dla programu Windows wewnątrz moda podaj
jego ścieżkę pod folderem `Data` gry, gdzie pokazuje go scalony widok. Narzędzie
natywne pomija kroki Protona: klucz rejestru wyżej, pamięć podręczną shaderów
Steama, puste korzenie .NET i wyjmowanie nakładki Steama z `LD_PRELOAD`.

Na kopii GOG lub Epic zainstalowanej przez Heroic narzędzia nie potrzebują takiego
wpisu, gdy ustawienia Heroic gry (`GamesConfig/<appName>.json`) wskazują build Wine
lub Protona: Eidos uruchamia je z tym buildem w prefiksie Heroic gry, a
`eidos prereqs <instance> --install` używa go do instalatorów i winetricks. Kopia
zainstalowana samym Legendary nie ma ustawień Heroic.

`slr=1` uruchamia narzędzie, i import rejestru przed nim, wewnątrz kontenera Steam
Linux Runtime, który wskazuje build Protona narzędzia (`require_tool_appid` w jego
`toolmanifest.vdf`), tak jak Steam uruchamia grę. Ten runtime musi być
zainstalowany w bibliotece Steama; jeśli nie jest, uruchomienie się zatrzymuje i
wskazuje aplikację Steama do zainstalowania. `eidos prereqs` nadal uruchamia swoje
instalatory i winetricks na hoście, a narzędzie z `runner=native` ignoruje ten
klucz. Kontener widzi twój folder domowy i bibliotekę Steama gry. Narzędzie, mod
lub runtime gdziekolwiek indziej, na przykład na innym dysku, potrzebuje
`env=STEAM_COMPAT_MOUNTS=/that/path` (kilka ścieżek oddzielonych `:`). Build
Protona zainstalowany pod `/usr/share/steam/compatibilitytools.d` prawdopodobnie
nie nadaje się do użycia z `slr=1`, bo `/usr` wewnątrz kontenera należy do samego
runtime'u. Żadne uruchomienie narzędzia wewnątrz kontenera nie zostało jeszcze
przetestowane.

`eidos tool <instance> run <title> --print` pokazuje końcowe polecenie i każdą
zmienną, którą uruchomienie ustawia ponad odziedziczone środowisko, razem z tymi
wierszami. Dla wpisu, który uruchamia grę, wypisuje zamiast tego polecenie sklepu
(zob. [Dotarcie do narzędzia](#dotarcie-do-narzędzia-ukrywanie-przypinanie-i-skrót-na-pulpicie)).

## BethINI Pie

Eidos nie dostarcza BethINI Pie. Włóż wydanie pobrane z jego strony na Nexusie do
`mods/` tej instancji albo do folderu narzędzi, a `Bethini.exe` pojawi się na
liście narzędzi, dla tych samych gier co PGPatcher. Uruchomiony przez Eidosa edytuje
kopię INI gry należącą do profilu. Przed każdym uruchomieniem narzędzia, którego
tytuł zawiera `bethini`, Eidos zachowuje punkt przywracania tych INI; Backups na
pasku narzędzi, potem INIs, przywraca jeden z nich. INI, którego nie było, gdy
robiono kopię, zostaje bez zmian. Okno zachowuje 10 najnowszych punktów
przywracania każdego rodzaju, a przywrócenie jednego najpierw robi kopię INI takich,
jakie są, więc błędny wybór można cofnąć.

## Własne ustawienia narzędzia pozostają jego własne

Eidos nie sprawdza większości ustawień narzędzi; błędne zwykle zawodzi bez błędu.

Game Data Path w BodySlide (Settings) musi wskazywać katalog `Data` gry, a nie
folder gry nad nim. Ustawiony o jeden poziom za wysoko, build wsadowy zgłasza „All
sets processed successfully” i zapisuje 1439 siatek tam, gdzie gra nigdy ich nie
szuka. Eidos łapie je w `Overwrite/Root/`, więc twoja instalacja gry jest
nietknięta, a jedynym śladem w grze jest to, że twoje ciała nie są zbudowane.

Przed uruchomieniem BodySlide lub Outfit Studio wiersz stanu ostrzega, gdy
`GameDataPath` lub `OutputDataPath` w zwycięskim
`CalienteTools/BodySlide/Config.xml` nie prowadzi w czasie uruchomienia do folderu
`Data` gry, a zakładka Diagnostics pokazuje to samo ostrzeżenie jako kartę z
wartością do ustawienia. `eidos check <instance>` też to zgłasza. Ścieżka `S:` jest
czytana tak, jak Eidos mapuje `S:` dla narzędzia, na folder biblioteki Steama.
`OutputDataPath` wewnątrz folderu `mods/` instancji, dla moda wyjściowego, jest
akceptowany. Diagnostics pokazuje też kartę, gdy `Overwrite/Root/` zawiera folder
`meshes` lub `textures`.

Wynik narzędzia należy do Overwrite. Gdy uruchomienie wytwarza coś wartego
zachowania, Overwrite -> Create mod... zamienia to w zwykłego moda, który można
porządkować, wyłączać i usuwać jak każdy inny.
