<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Instalacja Eidos

Trzy drogi wejścia. Każda daje te same dwa pliki wykonywalne - `eidos` (wiersz
poleceń) i `eidos-gui` - oraz obsługę `nxm://`, dzięki której przycisk
„Mod Manager Download" na Nexusie ląduje w twojej instancji.

## Co jest potrzebne najpierw

| | |
|---|---|
| **Linux z FUSE** | `fusermount3` w PATH. Dostarcza go każda obecna dystrybucja. |
| **Gra pod Protonem, raz uruchomiona** | Steam tworzy prefiks Wine gry dopiero przy pierwszym uruchomieniu, a Eidos działa w jego wnętrzu. |
| **`7z`** | Do instalowania archiwów modów. W większości dystrybucji to `p7zip`. |

Bez roota, bez demona, bez edycji `/etc/fuse.conf` i bez dopisywania czegokolwiek
do twoich grup. Eidos montuje w prywatnej przestrzeni nazw należącej do procesu
gry.

## Arch

```bash
cd packaging && makepkg -si
```

## Archiwum wydania

```bash
./install.sh
```

Domyślnie instaluje do `~/.local/bin`. `--system` umieszcza w `/usr/local/bin`,
`--bindir DIR` gdziekolwiek indziej. Ponowne uruchomienie to przewidziany sposób
aktualizacji.

## Ze źródeł

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Następnie: wskazać go Steamowi

Eidos uruchamia się *jako* polecenie startowe twojej gry - i właśnie dlatego
zdąży zamontować widok, zanim gra wystartuje. W Steamie prawy przycisk na grze ->
Właściwości -> Opcje uruchamiania:

```
~/.local/bin/eidos-gui %command%
```

Naciśnij Graj. Eidos otworzy się na instancji tej gry; instaluj mody, sortuj
LOOT-em, klikaj Run. Po wyjściu montowanie znika razem z grą, a twoja instalacja
jest dokładnie taka, jaka była.

Użyj ścieżki bezwzględnej - Steam nie czyta `PATH` twojej powłoki.

### Jeśli wolisz terminal

```sh
eidos init skyrimse               # utworzyć instancję (podaj folder, by była przenośna)
eidos install skyrimse mod.7z     # mody Simple / FOMOD / BAIN / root
eidos sort skyrimse               # posortować kolejność wczytywania LOOT-em
eidos play skyrimse -- %command%  # uruchomić cokolwiek przez scalony widok
```

Każde polecenie przyjmujące identyfikator gry przyjmuje też folder instancji
przenośnej - zob. [usage.pl.md](usage.md). Pełna wycieczka jest tamże.

## Opcjonalnie: passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` włącza jądrowy passthrough
FUSE. Jest **domyślnie wyłączony i niemal na pewno chcesz, żeby tak zostało**:
zmierzone na Skyrim SE - powstrzymuje grę przed otwarciem własnych archiwów i
wtyczek, przez co mody po cichu się nie wczytują. Przełącznik istnieje po to, by
mechanizm dało się przetestować ponownie, a nie dlatego, że jest zalecany.

Szczegóły i pomiary stojące za tą decyzją w
[troubleshooting.pl.md](troubleshooting.md).

## Coś już nie działa?

[troubleshooting.pl.md](troubleshooting.md) opisuje przełączniki
środowiskowe, odczyt liczników operacji i każdy problem, który dotąd kogoś ugryzł.
