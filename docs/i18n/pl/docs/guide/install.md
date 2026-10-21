<!-- eidos-i18n: source=docs/guide/install.md sha=bc05db1de00bb7c80224f065c3f730f3adc542dc -->

# Instalacja Eidos

Cztery drogi wejścia. Colony instaluje jeden plik, `eidos-linux`, który jest
całym programem; pozostałe dają obok siebie `eidos` (wiersz poleceń),
`eidos-gui` i program pomocniczy do statycznego podglądu NIF, a także obsługę
`nxm://`, dzięki której przycisk „Mod Manager Download” na Nexusie trafia do
twojej instancji.

## Co jest potrzebne najpierw

| | |
|---|---|
| **Linux z FUSE** | `fusermount3` w PATH. Dostarcza go każda obecna dystrybucja. |
| **Gra pod Protonem, raz uruchomiona** | Steam tworzy prefiks Wine gry dopiero przy pierwszym uruchomieniu, a Eidos działa w jego wnętrzu. |
| **`7z`** | Do instalowania archiwów modów. W większości dystrybucji to `p7zip`. |
| **`par2`** (opcjonalnie) | Dane naprawcze PAR2 dla kopii robionych przez `eidos pack`. `par2cmdline` na Archu i Fedorze, `par2` na Debianie i Ubuntu. |

Bez roota, bez demona, bez edycji `/etc/fuse.conf` i bez dopisywania czegokolwiek
do twoich grup. Eidos montuje w prywatnej przestrzeni nazw należącej do procesu
gry.

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
potrzebuje `eidos-nif-preview`, który mają tylko pakiet wydania i kompilacje ze
źródeł; bez niego podgląd to zgłasza, a nic innego nie jest dotknięte. Wskaż
Steamowi sam plik, jak opisano niżej.

## Pakiet wydania

Pobierz `eidos-linux-bundle.tar.gz` oraz jego `.sig`, `.meta` i `.meta.sig`
z [najnowszego wydania](https://github.com/Project-Colony/Eidos/releases/latest)
i sprawdź je tak, jak pokazuje
[Polityka podpisywania kodu](../../README.md#polityka-podpisywania-kodu) w
README. Następnie:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Domyślnie instaluje do `~/.local/bin`. `--system` umieszcza w `/usr/local/bin`,
`--bindir DIR` gdziekolwiek indziej. Ponowne uruchomienie to przewidziany sposób
aktualizacji. Pakiet i `eidos-linux` wymagają Linuksa x86_64 z glibc 2.39 lub
nowszą.

## Arch

```bash
cd packaging && makepkg -si
```

PKGBUILD buduje bieżący `main` ze źródeł.

## Ze źródeł

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Budowanie ze źródeł wymaga Rusta 1.89 lub nowszego, `just`, CMake 3.20+, kompilatora C++17 oraz Python 3 do testów natywnego programu pomocniczego. Polecenia bez `just` znajdziesz w [instrukcji natywnego programu pomocniczego](../../../../../crates/eidos-nif-preview/README.md). Zbuduj Rust poleceniem `cargo build --release --locked`, skopiuj program pomocniczy do `target/release`, a następnie uruchom `packaging/install.sh --from target/release`.

## Następnie: wskazać go Steamowi

Eidos uruchamia się *jako* polecenie startowe twojej gry - i właśnie dlatego
zdąży zamontować widok, zanim gra wystartuje. W Steamie prawy przycisk na grze ->
Właściwości -> Opcje uruchamiania:

```
~/.local/bin/eidos-gui %command%
```

Przy instalacji z Colony ścieżka to
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Naciśnij Graj. Eidos otworzy się
na instancji tej gry; instaluj mody, sortuj LOOT-em, klikaj Run. Po wyjściu
montowanie znika razem z grą, a twoja instalacja jest dokładnie taka, jaka była.

Użyj ścieżki bezwzględnej - Steam nie czyta `PATH` twojej powłoki.

### Jeśli wolisz terminal

```sh
eidos init skyrimse               # create an instance (add a folder to make it portable)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root mods
eidos sort skyrimse               # LOOT-sort the load order
eidos play skyrimse -- %command%  # run anything through the merged view
```

Każde polecenie przyjmujące identyfikator gry przyjmuje też folder instancji
przenośnej - zob. [usage.md](usage.md#instancje-globalne-i-przenośne). Pełna
wycieczka jest w [usage.md](usage.md).

## Opcjonalnie: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` nadaje opcjonalne uprawnienie, ale nie włącza passthrough.
Nadaje się je osobnemu plikowi wykonywalnemu `eidos` z pakietu albo z kompilacji
ze źródeł, nigdy `eidos-linux`, który jest też oknem
([SECURITY.md](../../../../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` to osobny przełącznik używany podczas uruchamiania. Jest **domyślnie wyłączony i niemal na pewno chcesz, żeby tak zostało**:
zmierzone na Skyrim SE - powstrzymuje grę przed otwarciem własnych archiwów i
wtyczek, przez co mody po cichu się nie wczytują. Przełącznik istnieje po to, by
mechanizm dało się przetestować ponownie, a nie dlatego, że jest zalecany.

Szczegóły i pomiary stojące za tą decyzją w
[troubleshooting.md](troubleshooting.md).

## Coś już nie działa?

[troubleshooting.md](troubleshooting.md) opisuje przełączniki
środowiskowe, odczyt liczników operacji i każdy problem, który dotąd kogoś ugryzł.
