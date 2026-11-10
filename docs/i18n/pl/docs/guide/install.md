<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Instalacja Eidos

Cztery drogi wejścia. Colony instaluje jeden plik, `eidos-linux`, który jest
całym programem. Pakiet wydania, pakiet Arch i kompilacja ze źródeł dają obok
siebie `eidos` (wiersz poleceń), `eidos-gui` i statyczny program pomocniczy do
podglądu NIF, `eidos-nif-preview`. Pakiet Arch i kompilacja ze źródeł instalują
też `eidos-fuse` i `eidos-launch`, dwa niższopoziomowe programy wiersza poleceń.
Każda droga może zarejestrować obsługę `nxm://`, dzięki której przycisk „Mod
Manager Download” na Nexusie trafia do twojej instancji.

## Co jest potrzebne najpierw

| Wymaganie | Po co |
|---|---|
| Linux z FUSE | `fusermount3` w PATH. Dostarcza go każda obecna dystrybucja. |
| Gra pod Protonem, raz uruchomiona | Steam tworzy prefiks Wine gry dopiero przy pierwszym uruchomieniu, a Eidos działa w jego wnętrzu. Eidos wykrywa też kopie GOG i Epic Skyrim Special Edition, kopie Epic Fallouta 3 i Fallouta: New Vegas oraz kopię GOG Stardew Valley. Działają one przez Heroic lub Legendary, z Eidosem jako wrapperem gry: zobacz [usage.md](usage.md#uruchamianie-gry). |
| Program 7-Zip: `7z`, `7zz` lub `7za` | Do instalowania archiwów modów. `7zip` na Archu, `p7zip` w większości innych dystrybucji. |
| `par2` (opcjonalnie) | Dane naprawcze PAR2 dla kopii robionych przez `eidos pack`. `par2cmdline` na Archu i Fedorze, `par2` na Debianie i Ubuntu. |

Bez roota, bez demona, bez edycji `/etc/fuse.conf` i bez dopisywania czegokolwiek
do twoich grup. Eidos montuje w prywatnej przestrzeni nazw należącej do procesu
gry.

Niektóre ustawienia systemu objawiają się dopiero awariami. Zakładka Diagnostics
sprawdza `vm.max_map_count` (1048576 lub więcej), twardy limit otwartych plików
(524288 lub więcej) oraz to, czy gra, jej prefiks albo instancja leży na NTFS,
exFAT lub FAT; poprawka dla każdego z nich jest w
[troubleshooting.md](troubleshooting.md#system).

## Przez Colony

Wyszukaj **Eidos** w [Colony](https://github.com/Project-Colony/Colony) i
zainstaluj go; aktualizacje przychodzą przez launcher. Colony trzyma jeden plik,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Uruchom go, a otworzy się okno.
Wywołany przez dowiązanie o nazwie `eidos`, ten sam plik jest wierszem poleceń.
Umieść takie dowiązanie w swoim `PATH` i zarejestruj obsługę pobierania z
Nexusa:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

Okno nie potrzebuje dowiązania: za każdym razem, gdy uruchamia grę albo
narzędzie, wywołuje własny plik jako wiersz poleceń. Podgląd modeli NIF
potrzebuje `eidos-nif-preview`, który mają tylko pakiet wydania, pakiet Arch i
kompilacje ze źródeł; bez niego podgląd to zgłasza, a nic innego nie jest
dotknięte. `eidos-linux` nie zawiera też żadnego z runtime'ów Microsoftu, które
dostarcza pakiet ([niżej](#runtimey-microsoftu-w-pakiecie)): gdy narzędzie
któregoś potrzebuje, Eidos pobiera go od Microsoftu, gdy zaczniesz instalację
wymagań. Wskaż Steamowi sam plik, jak opisano niżej.

## Pakiet wydania

Pobierz `eidos-linux-bundle.tar.gz` oraz jego `.sig`, `.meta` i `.meta.sig`
z [najnowszego wydania](https://github.com/Project-Colony/Eidos/releases/latest)
i sprawdź je tak, jak pokazuje
[Polityka podpisywania kodu](../../README.md#polityka-podpisywania-kodu) w
README. Pakiet zawiera na najwyższym poziomie `eidos`, `eidos-gui` i
`eidos-nif-preview`, a do tego `install.sh`, `README.md`, `LICENSE`,
`SECURITY.md`, ikony w `assets/brand/png`, noty stron trzecich w `licenses/`
(nifly, omod, microsoft-directx, microsoft-runtimes i crate'y Rusta) oraz folder
`runtimes/` opisany niżej. Nie zawiera `eidos-fuse` ani `eidos-launch`.
Następnie:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` kopiuje pliki wykonywalne do `~/.local/bin` albo do `$XDG_BIN_HOME`,
gdy jest ustawione. `--system` umieszcza je w `/usr/local/bin` przez sudo,
`--bindir DIR` gdziekolwiek indziej, a `--from DIR` bierze pliki wykonywalne z
innego katalogu. Skrypt także:

- rejestruje obsługę `nxm://`, uruchamiając `eidos nxm --register`, więc krok
  rejestracji z sekcji o Colony nie jest potrzebny;
- zapisuje `~/.local/share/applications/eidos.desktop` oraz ikony Eidosa w
  `~/.local/share/icons/hicolor`;
- kopiuje noty licencyjne do `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` z `--system`);
- kopiuje `runtimes/` do `~/.local/share/Colony/Eidos/runtimes`, także z
  `--system`, żeby runtime'y należały do użytkownika uruchamiającego skrypt;
- na końcu wypisuje opcję uruchamiania Steama z bezwzględną ścieżką
  zainstalowanego `eidos-gui`.

`--cap` prosi sudo o nadanie `CAP_SYS_ADMIN` zainstalowanemu `eidos` (zob.
passthrough FUSE niżej). `--no-cap` jest domyślne i zostawia instalację bez
roota. Ponowne uruchomienie skryptu to przewidziany sposób aktualizacji. Pakiet i
`eidos-linux` wymagają Linuksa x86_64 z glibc 2.39 lub nowszą.

### Runtime'y Microsoftu w pakiecie

Folder `runtimes/` pakietu zawiera pliki Microsoftu, o które proszą narzędzia:
`VC_redist.x86.exe` i `VC_redist.x64.exe` (Visual C++ 2015-2022, wymaganie
`vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (.NET 8 Desktop
Runtime, dla `dotnet8` i `dotnetdesktop8`) oraz
`dotnet-runtime-10.0.10-win-x64.zip` (runtime .NET 10, dla `dotnet10`). Eidos
sam uruchamia instalatory Visual C++ i .NET 8 w prefiksie gry, bez winetricks,
a zip .NET 10 rozpakowuje raz do `~/.local/share/Colony/Eidos/runtimes`, gdzie
współdzielą go wszystkie instancje (78 MB po rozpakowaniu). Każdy plik jest
używany tylko wtedy, gdy jego SHA-256 zgadza się z przypiętym w Eidosie. SDK .NET
10, którego potrzebuje Synthesis (`dotnetsdk10`, około 300 MB), nie jest w
pakiecie i zawsze jest pobierane.

Rozpakowany pakiet działa też na miejscu: Eidos szuka `runtimes/` obok własnego
pliku wykonywalnego, po folderze danych opisanym wyżej.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD buduje bieżący `main` ze źródeł. Instaluje `eidos`, `eidos-gui`,
`eidos-nif-preview`, `eidos-fuse` i `eidos-launch` do `/usr/bin`, razem z
systemową obsługą `nxm://`, wpisem pulpitu i ikonami. Zależy od `fuse3`, `7zip` i
`xdg-utils`. Nie zawiera folderu `runtimes/`, więc runtime'y Microsoftu są
pobierane, gdy narzędzie ich potrzebuje. `winetricks` jest opcjonalny: Eidos nadal
używa go do wymagań, których sam nie instaluje, takich jak `dotnet48` i `xact`.

## Ze źródeł

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Budowanie ze źródeł wymaga Rusta 1.92 lub nowszego, `just`, CMake 3.20+, kompilatora C++17 oraz Python 3 do testów natywnego programu pomocniczego. `just build` buduje cały workspace, więc `eidos-fuse` i `eidos-launch` też są budowane, a `just install` uruchamia `packaging/install.sh` na wyniku budowania, który kopiuje je razem z resztą. Polecenia bez `just` znajdziesz w [instrukcji natywnego programu pomocniczego](../../../../../crates/eidos-nif-preview/README.md). Zbuduj Rust poleceniem `cargo build --release --locked`, skopiuj program pomocniczy do `target/release`, a następnie uruchom `packaging/install.sh --from target/release`. Kompilacja ze źródeł też nie ma folderu `runtimes/`, więc runtime'y Microsoftu są pobierane, gdy narzędzie ich potrzebuje.

Profil release używa jednej jednostki generowania kodu, pełnego LTO i usuwa symbole. Z rustc 1.99.1 i 3 zadaniami budowania zimna kompilacja `eidos` i `eidos-gui` trwała 334 s, wobec 243 s z cienkim LTO, i dała `eidos` o rozmiarze 20,9 MB oraz `eidos-gui` o rozmiarze 49,7 MB. Każdy plik wykonywalny osadza osiem bibliotek DLL DirectX przechowywanych skompresowane gzipem: 7,9 MB zamiast 17,9 MB. `just profile=dev build` buduje bez optymalizacji.

## Następnie: wskazać go Steamowi

Eidos uruchamia się *jako* polecenie startowe twojej gry - i właśnie dlatego
zdąży zamontować widok, zanim gra wystartuje. W Steamie prawy przycisk na grze ->
Właściwości -> Opcje uruchamiania, i wpisz bezwzględną ścieżkę okna Eidosa, a po
niej `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Użyj ścieżki bezwzględnej - Steam nie czyta `PATH` twojej powłoki. Ścieżka zależy
od sposobu instalacji Eidosa, a `install.sh` wypisuje wiersz dla katalogu, którego
użył:

| Instalacja | Opcja uruchamiania |
|---|---|
| Pakiet wydania lub `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Pakiet Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Naciśnij Graj. Eidos otworzy się na instancji tej gry; instaluj mody, sortuj
LOOT-em, klikaj Run. Po wyjściu montowanie znika razem z grą, a twoja
instalacja jest dokładnie taka, jaka była.

Gra zawsze startuje przez swój sklep. W oknie otwartym przez ciebie Run
uruchamia `steam -applaunch <appid>`, Steam uruchamia Eidosa przez opcję
uruchamiania, a otwiera się drugie okno Eidosa, w którym naciskasz Run. Dopóki
opcja uruchamiania nie uruchomi Eidosa, Run niczego nie startuje, a wiersz stanu
pokazuje wiersz do ustawienia. Przy włączonym Start the game at once when a store
opens Eidos (Preferences, General, Running a game; domyślnie wyłączone) Graj w
Steamie uruchamia grę bez okna, gdy Health nie wymienia żadnego problemu. Heroic
i Legendary działają tak samo przez swoje ustawienie wrappera;
[usage.md](usage.md#uruchamianie-gry) ma szczegóły.

Zakładka Diagnostics sprawdza ten wiersz. Karta Health pokazuje wiersz do
ustawienia, gdy opcja uruchamiania jest pusta, nie wskazuje żadnego pliku
Eidosa, wskazuje plik, który nie istnieje, nie ma `%command%` po Eidosie albo
uruchamia inną kopię Eidosa niż ta, która działa.

### Jeśli wolisz terminal

```sh
eidos init skyrimse               # create an instance (add a folder to make it portable)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root mods
eidos sort skyrimse               # LOOT-sort the load order
eidos check skyrimse              # the Health checks that need no window
eidos play skyrimse -- %command%  # run anything through the merged view
eidos shell skyrimse              # a shell in the merged view, writes land in Overwrite
```

Każde polecenie przyjmujące identyfikator gry przyjmuje też folder instancji
przenośnej - zob. [usage.md](usage.md#instancje-globalne-i-przenośne). Pełna
wycieczka jest w [usage.md](usage.md).

## Aktualizacje

Colony aktualizuje `eidos-linux` przez swój launcher. Dla pakietu wydania
uruchom `install.sh` nowego pakietu; dla pakietu Arch uruchom ponownie
`makepkg -si`; dla kompilacji ze źródeł zrób pull i uruchom `just install`. Okno
przy starcie pyta GitHub o najnowsze wydanie, najwyżej raz dziennie, a gdy jest
nowsze, pokazuje link do jego notatek wydania po Help na pasku menu. Nigdy samo
niczego nie pobiera ani się nie podmienia. Preferences, About, Version wyłącza
sprawdzanie, tryb offline je pomija, a kopia zainstalowana w `/usr` (pakiet Arch)
nigdy nie pyta, bo aktualizuje ją menedżer pakietów.

## Opcjonalnie: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` nadaje opcjonalne
uprawnienie, ale nie włącza passthrough. `./install.sh --cap` i `just setcap`
nadają je instalowanemu lub budowanemu `eidos`. Na montowaniu `nosuid` jądro
ignoruje uprawnienia plików, więc `install.sh` pomija nadanie z ostrzeżeniem, a
`just setcap` kończy się błędem. Uprawnienie nadaje się osobnemu plikowi
wykonywalnemu `eidos` z pakietu wydania, pakietu Arch albo z kompilacji ze
źródeł, nigdy `eidos-linux`, który jest też oknem
([SECURITY.md](../../../../../SECURITY.md)). Przebudowany lub zainstalowany
ponownie plik to nowy plik bez uprawnienia, więc jeśli go używasz, nadaj je
ponownie po każdej aktualizacji.

`EIDOS_FUSE_PASSTHROUGH=1` to osobny przełącznik używany podczas uruchamiania i
jest domyślnie wyłączony. Przy włączonym passthrough wcześniejsza wersja demona
sprawiała, że Skyrim SE nie wczytywał żadnej zawartości modów. Najbardziej
prawdopodobna przyczyna to rejestrowanie nowego pliku bazowego przy każdym
otwarciu, podczas gdy jądro przyjmuje jeden na i-węzeł. Demon dzieli teraz jeden
plik bazowy między wszystkie otwarcia pliku przez passthrough, ale nie zmierzono
tego jeszcze ponownie w grze. Do tego czasu włączaj passthrough tylko po to, by
go testować. Pomiary i przyczyna są w [troubleshooting.md](troubleshooting.md).

## Coś już nie działa?

[troubleshooting.md](troubleshooting.md) opisuje przełączniki
środowiskowe, odczyt liczników operacji, karty Health i każdy problem, który
dotąd kogoś ugryzł.
