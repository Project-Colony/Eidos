<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Rozszerzenia

Rozszerzenie dodaje Eidosowi wpis, nie będąc częścią Eidosa. To manifest TOML
wskazujący program, plus - co najwyżej - ten program.

Manifesty leżą w `~/.config/Colony/Eidos/addons/`, po jednym `.toml` na
rozszerzenie. Otwórz folder przez View -> Extensions -> Open folder i naciśnij
Reload - bez restartu.

`id`, `kind` i `exec` są wymagane; `name` domyślnie przyjmuje wartość `id`.
`exec` wskazuje program. Nazwa bez `/` jest szukana w `PATH`. Gdy programu
brakuje, Eidos pomija rozszerzenie, a jego wiersz w View -> Extensions podaje
powód (`<exec> is not on PATH` albo `<exec> is not there`). `games` ogranicza
rozszerzenie do podanych identyfikatorów gier; bez niego rozszerzenie dotyczy
każdej gry, a przy innej grze jego wiersz mówi „not for this game".

Manifest odrzucony przez Eidosa jest wymieniony na początku, razem z powodem:
plik, którego nie da się odczytać, niepoprawny TOML albo brak wymaganego pola,
pusty `id` lub `id` ze spacją, pusty `exec`, `id` zajęty już przez wcześniejszy
manifest (pliki są czytane w kolejności nazw), `kind`, którego Eidos nie zna,
albo rodzaj instalatora, podglądu lub informacji o zapisie bez `protocol = 1`
i bez nazw plików w `extensions`.

## Dlaczego nic nie jest ładowane do Eidosa

Mod Organizer 2 ładuje wtyczki jako biblioteki współdzielone, a te w Pythonie
uruchamia przez Qt. Żadne z tego się nie przenosi. Rust nie ma stabilnego ABI,
więc biblioteka współdzielona zbudowana innym kompilatorem - albo z inną flagą
optymalizacji, albo z innym zestawem cech wspólnej zależności - to zachowanie
niezdefiniowane i nic nie zgłasza tego jako niezgodności wersji. Do tego widżety Eidosa są generyczne na
etapie kompilacji, więc biblioteka nie zbudowałaby żadnego do zwrócenia, nawet
gdyby ABI było stabilne.

Rozszerzenie to program, który Eidos uruchamia z uprawnieniami twojego konta do systemu plików. Instaluj tylko programy pomocnicze, którym ufasz. Izolacja procesów i sprawdzanie odpowiedzi nie zapewniają piaskownicy systemu operacyjnego.

## Narzędzie

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # pomiń, by objąć każdą grę
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Pojawia się w View -> Extensions z przyciskiem Run i startuje odłączone -
Eidos na nie nie czeka, a wiersz stanu podaje jego pid. Jego wyjście jest
odrzucane. Bez `workdir` startuje w folderze, z którego uruchomiono samego
Eidosa.

## Kontrola

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Wykonuje się przy każdym odświeżeniu, w katalogu głównym instancji, o ile
`workdir` nie mówi inaczej, i wypisuje jedno ustalenie na wiersz:

```
level<TAB>title<TAB>detail
```

gdzie `level` to `problem`, `advice` albo `ok`. Szczegół jest opcjonalny; wiersz bez tytułu
jest odrzucany. Wszystko, co nie zaczyna się od znanego poziomu, jest pomijane,
więc wypisywanie postępu i zabłąkane ostrzeżenia nie mogą wytworzyć wiersza
wyglądającego jak własna kontrola Eidosa. Ustalenia trafiają do zakładki
Diagnostics jako `<name> - <title>`, po kartach prefiksu i Nexusa, a przed
pozostałymi kartami Health.

Kontrola dostaje trzy sekundy. Ta, która je przekroczy, zostaje zatrzymana i
zgłoszona jako problem przeciwko sobie samej - działa przy tym samym odświeżeniu,
które następuje po każdym kliknięciu, więc zawieszona zamroziłaby okno.
Zegar biegnie, aż kontrola się zakończy i zamkną się jej potoki wyjścia, więc
proces potomny działający w tle, który nadal je trzyma, wlicza się w te trzy
sekundy. Kontrola, która kończy się błędem i nic nie wypisuje, jest zgłaszana
tak samo, z pierwszym wierszem jej wyjścia błędów, podobnie jak ta, która
wypisze więcej niż 1 MiB na którymkolwiek strumieniu. W obu przypadkach Eidos
nie uruchamia jej ponownie, dopóki nie naciśniesz Reload, a jej wiersz mówi
„It will not be retried until you press Reload." Kontrola nic nie czyta ze
swojego standardowego wejścia.

## Symbole zastępcze

Zarówno `args`, jak i `workdir` rozwijają te:

| Symbol          | Co to jest                                   |
| --------------- | -------------------------------------------- |
| `{instance}`    | katalog główny instancji                     |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | nazwa aktywnego profilu                      |
| `{profile_dir}` | katalog aktywnego profilu                    |
| `{game}`        | identyfikator gry, np. `skyrimse`            |
| `{game_name}`   | wyświetlana nazwa gry                        |
| `{install}`     | folder gry, z którego działa instancja       |
| `{data}`        | katalog `Data` tego folderu                  |

`{install}` to instalacja ze sklepu albo kopia gry, gdy instancja działa z
kopii (zamrożony `Stock Game` albo własny folder gry z listy; zobacz
[usage.md](usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia)). `{data}` za nią
podąża.

Kontrola dostaje też `{addon_dir}`, czyli folder z jej manifestem, tak samo jak
rozszerzenia protokołu 1 opisane niżej. Narzędzie go nie dostaje.

Nieznany symbol nigdy nie jest czyszczony, bo zamieniłoby to `--out {typo}` w
`--out --next-flag`. Eidos odmawia uruchomienia narzędzia lub kontroli, której
symboli nie da się w całości rozwinąć, i mówi, których brakuje: w wierszu stanu
dla narzędzia, w wierszu kontroli w zakładce Diagnostics dla kontroli.

## Czego rozszerzenie nie może

Rozszerzenie nie może wywoływać Eidosa zwrotnie, rejestrować się z archiwum ani rysować własnych widżetów. Instalatory korzystające ze strukturalnego protokołu 1 mogą zwracać sprawdzone plany plików i pytania z wyborem; Eidos odpowiada za przegląd, przygotowanie w katalogu tymczasowym i publikację. FOMOD i BAIN pozostają instalatorami natywnymi. Rozszerzenia podglądu i informacji o zapisach gry używają tego samego hosta z ograniczonymi odpowiedziami. [Specyfikacja protokołu i uruchamialne przykłady](../../../../guide/examples/extensions/README.md) opisują priorytet dopasowania, zapisane odpowiedzi CLI, limity oraz kontrolę tożsamości przy ponownym wykonaniu.
