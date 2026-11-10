<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 przez Eidos

Z Eidosem Fallout 4 nie potrzebuje żadnej specjalnej opcji uruchamiania,
przemianowanego pliku wykonywalnego ani skryptu opakowującego. Dostaje tę samą
opcję uruchamiania co każda inna gra.

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

albo kopiują `f4se_loader.exe` na `Fallout4Launcher.exe`, co Steam przywraca
przy każdej aktualizacji gry - po czym grasz bez F4SE i nic o tym nie mówi.

Eidos wykonuje podmianę sam, na podstawie deskryptora gry. Gdy okno uruchamia
grę, zastępuje `Fallout4Launcher.exe` przez `f4se_loader.exe`, jeśli loader leży
w folderze gry, w `Root/` włączonego moda albo w `Overwrite/Root/`. Bez loadera
uruchamia `Fallout4.exe`, a wiersz stanu pisze
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Gdy nie ma tam żadnego z tych plików, uruchamia launcher bez zmian i też o tym
mówi. Jeśli opcją uruchamiania jest `eidos play fallout4 -- %command%`, CLI
podmienia tylko wtedy, gdy loader istnieje, a w przeciwnym razie uruchamia
`Fallout4Launcher.exe`.

Jest i drugi powód, by unikać launchera: skanuje on ponownie `Data` i
przepisuje `plugins.txt`, cofając dopiero co wdrożoną kolejność wczytywania.

Gdy sam otwierasz okno i naciskasz Run, Eidos prosi Steama o uruchomienie gry
(`steam -applaunch 377160`), a Steam uruchamia Eidosa przez opcję uruchamiania
opisaną wyżej. Jeśli `f4se_loader.exe` nie leży obok pliku wykonywalnego gry w
instancji, Eidos tego nie robi, a wiersz stanu prosi o najpierw zainstalowanie
script extendera. Fallout 4 VR działa tak samo, z `Fallout4VRLauncher.exe` i
`f4sevr_loader.exe`.

## Czym Eidos zajmuje się za ciebie

| Obszar | Co robi Eidos |
|---|---|
| Unieważnianie archiwów | `Fallout4Custom.ini` dostaje `[Archive]` `bInvalidateOlderFiles=1` oraz pusty `sResourceDataDirsFinal=` - dwa klucze, dzięki którym luźne pliki spoza `Data` są w ogóle widoczne. Zapisywane w profilu, nie w folderze gry. |
| INI | `Fallout4.ini`, `Fallout4Prefs.ini` i `Fallout4Custom.ini` należą do profilu. BethINI Pie uruchomiony przez Eidosa edytuje kopie profilu, a Backups -> INIs przywraca kopię, którą Eidos robi przed każdym jego uruchomieniem ([tools.md](tools.md#bethini-pie)). |
| Kolejność wczytywania | `plugins.txt` w formacie z gwiazdką, którego używa Fallout 4 (`*` oznacza aktywny), z uwzględnieniem `Fallout4.ccc` dla domyślnych wtyczek Creation Club |
| LOOT | Sortowanie działa tak samo jak w Skyrimie - `eidos sort <instance>` pobiera masterlistę `fallout4` |
| Zapisy | Zapisy `.fos` i ich cosave'y `.f4se` są wypisywane, kopiowane i trzymane per profil; panel szczegółów czyta własną tablicę wtyczek zapisu, więc zapis wymagający wyłączonej przez ciebie wtyczki powie o tym, zanim go wczytasz. Wyłączenie lub usunięcie moda, którego wtyczki używa jeden z trzech najnowszych zapisów profilu, najpierw pyta. |
| Mody root | Wszystko, co mod dostarcza obok pliku wykonywalnego (sam F4SE, ENB, `dxvk.conf`), trafia tam tym samym mechanizmem `Root/`, którego używa Skyrim |

## Kwestia wersji

Fallout 4 nie jest już tą zamrożoną grą, jaką był w latach 2019-2024. Na sierpień
2026 żyją trzy gałęzie, a biblioteka DLL moda zbudowana pod jedną nie wczyta się w
drugiej:

| Gałąź | Wersja | F4SE |
|---|---|---|
| Klasyczna („old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 do 1.11.240 | 0.7.4 do 0.7.9 |

Dwie konsekwencje, które warto znać przed budowaniem listy modów:

- Foldery `Creations/` i `Mods/` w katalogu gry oznaczają linię 1.11.x. Panel
  szczegółów zapisu w Eidosie pokazuje też build, który go zapisał: Fallout
  wpisuje to do zapisu, a Eidos wyświetla jako „Game build".
- F4SE zwykle pojawia się dzień lub dwa po aktualizacji Bethesdy, ale *Address
  Library for F4SE Plugins* - przez którą większość modów DLL rozwiązuje swoje
  przesunięcia - idzie własnym harmonogramem. Pomiędzy nimi mody DLL się nie
  wczytują. Mody bez DLL (tekstury, siatki, wtyczki) są nietknięte, więc świeża
  łatka to zły dzień na start listy modów.

Gdy twój zestaw już działa, zamroź wersję gry w instancji: Freeze this version
na karcie Game files w zakładce Diagnostics albo `eidos stock-game fallout4 freeze`,
żeby zobaczyć rozmiar, i `eidos stock-game fallout4 freeze --yes`, żeby skopiować.
Eidos kopiuje instalację ze sklepu do `<instance>/Stock Game` i uruchamia grę z
tej kopii, nadal przez Steama. Późniejsza łatka zmienia instalację ze sklepu, a
nie kopię, więc zainstalowane DLL zachowują swoją wersję gry. Na btrfs lub XFS
kopia dzieli bloki ze sklepem i prawie nie zajmuje miejsca, dopóki Steam nie
zmieni pliku; na ext4 to pełna kopia wielkości folderu gry. Eidos odmawia, gdy
Steam zgłasza grę jako zajętą lub niedokończoną. Szczegóły są w
[usage.md](usage.md#pliki-gry-instalacja-ze-sklepu-albo-kopia).

Bez kopii wyłącz automatyczne aktualizacje Steama dla Fallouta 4 (Właściwości,
Aktualizacje, „Aktualizuj tę grę tylko przy uruchomieniu"), bo następna łatka
rozbije każdą zainstalowaną bibliotekę DLL.

## Uwaga sprzętowa: odłamki broni wywalają grę na NVIDII

Efekt odłamków broni w Falloucie 4 działa na NVIDIA FleX, pochodnej PhysX, której
NVIDIA przestała wspierać po generacji Pascal. Na każdej karcie Turing i nowszej -
GTX 16, RTX 20 aż po RTX 50 - wywala grę. To błąd gry, niezwiązany z Linuksem,
Protonem ani Eidosem.

Dwa rozwiązania, każde wystarczy: wyłącz „Weapon Debris" w ustawieniach gry albo
zainstaluj *Weapon Debris Crash Fix* (Nexus 48078), który wyłącza kolizję odłamków
zamiast samego efektu.

## Karty Health dla Fallouta 4

Zakładka Diagnostics pokazuje kartę Health dla każdego z tych przypadków:

- Wtyczki F4SE są zainstalowane w `F4SE/Plugins`, ale `f4se_loader.exe` nie ma w
  żadnym włączonym modzie ani w folderze gry. Gra startuje bez nich.
- `f4se_loader.exe` jest zainstalowany, ale brakuje `f4se_<version>.dll` dla
  wersji uruchamianej gry (`f4se_1_10_984.dll` dla 1.10.984). Loader zatrzymuje
  się z błędem; zwykle następuje to po aktualizacji gry.
- Plik wykonywalny gry albo jej build Steama zmienił się od ostatniego
  uruchomienia przez Eidosa, które zakończyło się bez błędu. Wtyczki F4SE
  zbudowane pod starą wersję mogą przestać się wczytywać.
- Mod o wyższym priorytecie niż F4SE ma luźne kopie `.pex` własnych skryptów
  F4SE. Stara kopia psuje funkcje F4SE, które deklaruje, a gra nie pokazuje
  żadnego błędu.
- `Fallout4Custom.ini` profilu ma w `[General]` wiersze od `sTestFile1` do
  `sTestFile10`. Gdy któryś jest ustawiony, gra wczytuje te wtyczki i ignoruje
  `plugins.txt`. Fix usuwa je i zachowuje poprzedni plik jako
  `Fallout4Custom.ini.pre-fix`.
- Archiwum BA2 moda przechowuje dźwięk `.wav` lub `.xwm` w postaci
  skompresowanej. Gra może się wywalić, odtwarzając taki dźwięk. Spakuj archiwum
  ponownie z nieskompresowanym dźwiękiem. Fallout 4 VR dostaje tę samą kartę.
- Program rejestrujący awarie zapisał log podczas ostatniego uruchomienia:
  `crash-*.log` Buffouta 4 w `Documents/My Games/Fallout4/F4SE` albo `.log` w
  `F4SE/Crashlogs`. Karta wskazuje wyjątek oraz te DLL i wtyczki z logu, które
  instalują twoje mody.
- Karta Display pokazuje rozmiar i tryb okna ustawione w `Fallout4Prefs.ini`
  profilu, a Apply zapisuje tam `iSize W`, `iSize H`, `bFull Screen` i
  `bBorderless`.
- Wymaganie z Nexusa wskazuje DLC. Eidos szuka wtyczki o nazwie DLC, a wtyczki
  DLC Fallouta 4 mają inne nazwy (`DLCCoast.esm` dla Far Harbor), więc ta karta
  jest tylko radą: sprawdź ręcznie, potem naciśnij Ignore.

Dwie karty loadera mają przycisk Open the download page dla
<https://f4se.silverlock.org/> i przycisk Install the downloaded archive, który
umieszcza loader w `Root/` moda. `eidos check fallout4` wypisuje te same ustalenia
poza kartą `.pex`, kartą Display i kartami wymagań z Nexusa, które liczy tylko
okno.

## Jeśli coś wygląda nie tak

Ogólna lista kontrolna jest w [troubleshooting.md](troubleshooting.md);
pierwsze pytanie właściwe dla Fallouta brzmi zawsze *który plik wykonywalny naprawdę
wystartował*. Gdy okno uruchamia grę, Eidos wpisuje pełne polecenie uruchomienia
do dziennika uruchomień instancji, więc:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Jeśli widnieje tam `f4se_loader.exe`, podmiana się odbyła. Jeśli `Fallout4.exe`,
to F4SE nie jest zainstalowany tam, gdzie Eidos może go znaleźć - jego miejsce
jest obok pliku wykonywalnego gry, co przy zestawie zarządzanym modami oznacza
katalog `Root/` jakiegoś moda (albo sam folder gry, zainstalowany ręcznie). Jeśli
`Fallout4Launcher.exe`, Eidos nie znalazł w widoku root ani loadera, ani
`Fallout4.exe`.

Gdy startuje właściwy plik wykonywalny, a gra nadal się wywala, Test run i Find
the mod behind a problem zawężają to do jednego moda bez dotykania zapisów
profilu ([usage.md](usage.md#uruchomienia-testowe)).
