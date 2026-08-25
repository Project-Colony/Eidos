<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 przez Eidos

Fallout 4 nie potrzebuje żadnej specjalnej opcji uruchamiania, żadnego
przemianowanego pliku wykonywalnego ani skryptu opakowującego. Warto powiedzieć to
wprost, bo każdy inny linuksowy poradnik do F4SE twierdzi inaczej - a ich rady psują
się przy następnej aktualizacji Steama.

## Opcja uruchamiania

```
~/.local/bin/eidos-gui %command%
```

Celem uruchamiania Steama dla Fallouta 4 jest `Fallout4Launcher.exe`, nigdy
`Fallout4.exe`, więc doprowadzenie do tego, by script extender w ogóle wystartował,
sprowadza się do pytania „jak zmusić Steama do uruchomienia innego programu".
Zwykłe odpowiedzi przepisują `%command%` w bashu:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

albo kopiują `f4se_loader.exe` na `Fallout4Launcher.exe`, co Steam po cichu
przywraca przy każdej aktualizacji gry - po czym grasz bez F4SE i nic o tym nie
mówi.

Eidos wykonuje podmianę sam, na podstawie deskryptora gry: zastępuje launcher przez
`f4se_loader.exe`, gdy jest zainstalowany, cofa się do `Fallout4.exe`, gdy go nie
ma, i **mówi ci**, kiedy musiał się cofnąć. Gra, która startuje z wszystkimi modami
F4SE martwymi, jest gorsza od gry, która nie startuje.

Jest i drugi powód, by nigdy nie uruchamiać launchera: skanuje on ponownie `Data` i
przepisuje `plugins.txt`, cofając dopiero co wdrożoną kolejność wczytywania. Eidos
nigdy go nie wykonuje.

## Czym Eidos zajmuje się za ciebie

| | |
|---|---|
| Unieważnianie archiwów | `Fallout4Custom.ini` dostaje `[Archive]` `bInvalidateOlderFiles=1` oraz pusty `sResourceDataDirsFinal=` - dwa klucze, dzięki którym luźne pliki spoza `Data` są w ogóle widoczne. Zapisywane w profilu, nie w folderze gry. |
| Kolejność wczytywania | `plugins.txt` w formacie z gwiazdką, którego używa Fallout 4 (`*` oznacza aktywny), z uwzględnieniem `Fallout4.ccc` dla domyślnych wtyczek Creation Club |
| LOOT | Sortowanie działa tak samo jak w Skyrimie - `eidos sort <instance>` pobiera masterlistę `fallout4` |
| Zapisy | Zapisy `.fos` i ich cosave'y `.f4se` są wypisywane, kopiowane i trzymane per profil; panel szczegółów czyta własną tablicę wtyczek zapisu, więc zapis wymagający wyłączonej przez ciebie wtyczki powie o tym, zanim go wczytasz |
| Mody root | Wszystko, co mod dostarcza obok pliku wykonywalnego (sam F4SE, ENB, `dxvk.conf`), trafia tam tym samym mechanizmem `Root/`, którego używa Skyrim |

## Kwestia wersji

Fallout 4 nie jest już tą zamrożoną grą, jaką był w latach 2019-2024. Na sierpień
2026 żyją trzy gałęzie, a biblioteka DLL moda zbudowana pod jedną nie wczyta się w
drugiej:

| Gałąź | Wersja | F4SE |
|---|---|---|
| Klasyczna („old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Dwie konsekwencje, które warto znać przed budowaniem listy modów:

- **Sprawdź, co naprawdę masz.** Foldery `Creations/` i `Mods/` w katalogu gry
  oznaczają linię 1.11.x. Panel szczegółów zapisu w Eidosie pokazuje też build,
  który go zapisał - Fallout wpisuje to do zapisu, a Eidos wyświetla jako
  „Game build".
- **Świeża łatka to zły dzień na start.** F4SE zwykle pojawia się dzień lub dwa po
  aktualizacji Bethesdy, ale *Address Library for F4SE Plugins* - przez którą
  większość modów DLL rozwiązuje swoje przesunięcia - idzie własnym harmonogramem.
  Pomiędzy nimi połowa ekosystemu oparta na DLL leży. Mody bez DLL (tekstury,
  siatki, wtyczki) są nietknięte.

Gdy twój zestaw już działa, wyłącz automatyczne aktualizacje Steama dla Fallouta 4
(Właściwości → Aktualizacje → „Aktualizuj tę grę tylko przy uruchomieniu"), bo
następna łatka rozbije każdą zainstalowaną bibliotekę DLL.

## Uwaga sprzętowa: odłamki broni wywalają grę na NVIDII

Efekt odłamków broni w Falloucie 4 działa na NVIDIA FleX, pochodnej PhysX, której
NVIDIA przestała wspierać po generacji Pascal. Na każdej karcie Turing i nowszej -
GTX 16, RTX 20 aż po RTX 50 - wywala grę. To błąd gry, niezwiązany z Linuksem,
Protonem ani Eidosem.

Dwa rozwiązania, każde wystarczy: wyłącz „Weapon Debris" w ustawieniach gry albo
zainstaluj *Weapon Debris Crash Fix* (Nexus 48078), który wyłącza kolizję odłamków
zamiast samego efektu.

## Jeśli coś wygląda nie tak

Ogólna lista kontrolna jest w [troubleshooting.pl.md](troubleshooting.pl.md);
pierwsze pytanie właściwe dla Fallouta brzmi zawsze *który plik wykonywalny naprawdę
wystartował*. Eidos wpisuje pełne polecenie uruchomienia do dziennika uruchomień
instancji, więc:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Jeśli widnieje tam `f4se_loader.exe`, podmiana się odbyła. Jeśli
`Fallout4Launcher.exe`, to F4SE nie jest zainstalowany tam, gdzie Eidos może go
znaleźć - jego miejsce jest obok pliku wykonywalnego gry, co przy zestawie
zarządzanym modami oznacza katalog `Root/` jakiegoś moda (albo sam folder gry,
zainstalowany ręcznie).
