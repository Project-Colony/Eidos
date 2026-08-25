<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Rozszerzenia

Rozszerzenie dodaje Eidosowi wpis, nie będąc częścią Eidosa. To manifest TOML
wskazujący program, plus - co najwyżej - ten program.

Manifesty leżą w `~/.config/Colony/Eidos/addons/`, po jednym `.toml` na
rozszerzenie. Otwórz folder przez **View -> Extensions -> Open folder** i naciśnij
**Reload** - bez restartu.

## Dlaczego nic nie jest ładowane do Eidosa

Mod Organizer 2 ładuje wtyczki jako biblioteki współdzielone, a te w Pythonie
uruchamia przez Qt. Żadne z tego się nie przenosi. Rust nie ma stabilnego ABI,
więc biblioteka współdzielona zbudowana innym kompilatorem - albo z inną flagą
optymalizacji, albo z innym zestawem cech wspólnej zależności - to zachowanie
niezdefiniowane, a nie niezgodność wersji. Do tego widżety Eidosa są generyczne na
etapie kompilacji, więc biblioteka nie zbudowałaby żadnego do zwrócenia, nawet
gdyby ABI było stabilne.

Rozszerzenie jest więc programem, który Eidos *uruchamia*. Nie może wywrócić okna,
nie może uszkodzić listy modów i działa dalej mimo aktualizacji Eidosa.

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

Pojawia się w **View -> Extensions** z przyciskiem Run i startuje odłączone -
Eidos na nie nie czeka.

## Kontrola

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Wykonuje się przy każdym odświeżeniu i wypisuje jedno ustalenie na wiersz:

```
level<TAB>title<TAB>detail
```

gdzie `level` to `problem`, `advice` albo `ok`. Szczegół jest opcjonalny.
Wszystko, co nie zaczyna się od znanego poziomu, jest pomijane, więc wypisywanie
postępu i zabłąkane ostrzeżenia nie mogą wytworzyć wiersza wyglądającego jak
własna kontrola Eidosa. Ustalenia trafiają do zakładki **Health**, poprzedzone
nazwą rozszerzenia.

Kontrola dostaje trzy sekundy. Ta, która je przekroczy, zostaje zatrzymana i
zgłoszona jako problem przeciwko sobie samej - działa przy tym samym odświeżeniu,
które następuje po każdym kliknięciu, więc zawieszona zamroziłaby okno.

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
| `{install}`     | katalog instalacji gry                       |
| `{data}`        | katalog `Data` gry                           |

Nieznany symbol zostaje dokładnie tak, jak go zapisano, zamiast zostać
wyczyszczony - żeby pomyłka zawiodła widocznie, a nie zamieniła `--out {typo}` w
`--out --next-flag`. Uruchomienie narzędzia, którego symboli nie da się w całości
rozwinąć, jest odrzucane, a Eidos mówi, których brakuje.

## Czego rozszerzenie nie może

Dostaje wartości i działa; nie może oddzwonić do Eidosa, zmienić listy modów ani
narysować czegokolwiek w oknie. To celowe. To, do czego MO2 używa wtyczek i co
naprawdę MUSI sięgnąć do środka - obsługa gier, instalatory, silnik konfliktów -
jest tu wbudowane, a nie doczepione: definicja gry to własny TOML w
`~/.config/Colony/Eidos/games/`, a instalatory FOMOD i BAIN są natywne.
