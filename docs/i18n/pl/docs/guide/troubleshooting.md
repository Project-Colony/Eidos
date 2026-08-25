<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Rozwiązywanie problemów i diagnostyka

Wszystko na dzień, w którym gra widzi coś, z czym system plików się nie zgadza:
przełączniki środowiskowe, odczyt liczników operacji, znane problemy wraz z ich
historią oraz sprawa passthrough.

### Diagnozowanie VFS

Na wypadek, gdy gra widzi coś, z czym system plików się nie zgadza, istnieją dwie
zmienne środowiskowe:

```sh
EIDOS_FUSE_STATS=1                  # liczniki operacji, zrzucane przy odmontowaniu
EIDOS_FUSE_NO_CACHE=1               # każdy cache po stronie jądra wyłączony
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # albo nazwij je pojedynczo
```

To właśnie postać szczegółowa znalazła opisaną niżej awarię: wyłączenie wszystkich
czterech odpowiada na pytanie „czy to cache?", a dopiero nazwanie ich odpowiada
„który". Liczniki odpowiadają na drugą połowę - wczytywanie pokazujące `read 0` to
takie, w którym `FUSE_PASSTHROUGH` podał każdy bajt w jądrze, więc wszystko, co
zamierzałeś stroić na ścieżce odczytu, jest już darmowe.

## Zamontować unię ręcznie

Pierwsza `--layer` wygrywa przy konflikcie; ostatnia to twoje nietknięte dane gry.
Montowanie potrzebuje tylko `/dev/fuse` i `fusermount3` (bez overlayfs, bez Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... czytaj i pisz przez /mnt/point ...
fusermount3 -u /mnt/point
```

Zapisy lądują w `--overwrite <dir>` (katalog tymczasowy, gdy pominięty), więc same
warstwy pozostają nietknięte nawet tutaj.

#### Dlaczego passthrough jest domyślnie wyłączony

Passthrough przekazuje jądru prawdziwy plik bazowy, dzięki czemu odczyty całkowicie
omijają tego demona. To zysk przepustowości, który tutaj kosztuje poprawność.
Zmierzone A/B na Skyrim SE 1.6.1170, proton-cachyos 11.0, jądro 7.1.4, ta sama
kolejność wczytywania 82 wtyczek, przy jedynej zmiennej: czy plik binarny nosił tę
zdolność:

| passthrough | niepowodzenia `NtCreateFile` z `STATUS_ACCESS_VIOLATION` |
|-------------|----------------------------------------------------------|
| włączony    | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`         |
| wyłączony   | 0                                                        |

Przy włączonym gra nie otwiera żadnego z własnych archiwów ani wtyczek, co w grze
objawia się jako mody, których po prostu nie ma - bez błędu, bez wiersza w
dzienniku. Przy wyłączonym ta sama kolejność wczytywania dociera do rozgrywki z
żywymi wtyczkami, archiwami i skryptami Papyrus.

Awaria jest niewidoczna z wnętrza demona i to właśnie uczyniło jej znalezienie
kosztownym: nasze własne `open` udaje się za każdym razem, a jądro nigdy nie odmawia
pliku bazowego (sprawdzone na całej nieudanej sesji z `EIDOS_FUSE_TRACE=open`: zero
`open FAILED`, zero `passthrough refused`). Błąd powstaje po tym, jak demon
odpowiedział `opened_passthrough`, więc żadne logowanie po stronie demona go nie
zobaczy. Nie zależy też od rozszerzenia - uderza tak samo w archiwa i wtyczki, czyli
w pliki, które gra trzyma otwarte przez cały czas działania.

`EIDOS_FUSE_PASSTHROUGH=1` włącza go z powrotem, by zmierzyć, co daje, albo by
ponownie przetestować mechanizm. Ostrzeżenia o zdolności w launcherze i w zakładce
Diagnostics pojawiają się wyłącznie wtedy, gdy o nią poprosiłeś.

Aby uruchomić samą grę przez Eidos, ustaw jej opcję uruchamiania w Steamie na:

```
eidos play skyrimse -- %command%
```

Poprzedź to `WINEDLLOVERRIDES="d3dcompiler_47=n"`, jeśli Proton potrzebuje natywnego
d3dcompilera do kompilacji shaderów; Eidos scala to z dowolnymi nadpisaniami DLL,
które dostarcza mod (ładowarki ENB/ReShade/`.asi`).

### Czy indeks warstw jest naprawdę używany?

Indeks działa na zasadzie wszystko albo nic i budowany jest w ciszy:
`LayerStack::new` dostaje albo kompletną mapę warstw tylko do odczytu, albo `None`,
po czym każde zapytanie przechodzi je dokładnie tak jak wcześniej. Nic w dzienniku
sesji nie rozróżnia tych dwóch przypadków, więc stos, który po cichu wrócił do
przechodzenia, wygląda identycznie jak działający - płacąc przy tym starą cenę.

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

## Znane problemy

### DLSS albo generowanie klatek po cichu nic nie robi

Trzy odrębne przyczyny, każda bez jakiegokolwiek komunikatu błędu: niewłączone NVAPI
w opcjach uruchamiania, pełny ekran wyłączny albo przeterminowany limit FPS Reflexa.
Cała lista kontrolna jest w [graphics.pl.md](graphics.md).

**Mod, który zapisuje jeden katalog na dwa sposoby, tracił wszystko pod drugim.**
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

**LODGen z DynDOLOD-a umiera, zostawiając pusty dziennik.** Naprawione przez
`dotnet10`; zob. [tools.md](tools.md). Objaw jest nie do pomylenia:
`LODGen_SSE_<world>_log.txt` zawierający baner wersji, wiersz `.NET Version:` i nic
więcej, dla każdego świata, oraz okno mówiące tylko „failed to generate object LOD
for one or more worlds". Przyczyną jest Mono z Wine odpowiadające za .NET Framework i
żadna ilość zainstalowanego .NET Framework tego nie naprawia - Proton przy każdej
aktualizacji prefiksu zastępuje `mscoree.dll` dowiązaniem do własnego drzewa.

**Wine nie potrafiło stwierdzić, że montowanie składa wielkość liter.** Naprawione i
to był ten, który miał znaczenie.

Nie istnieje API „czy ten system plików jest niewrażliwy na wielkość liter", więc
`get_dir_case_sensitivity` w Wine węszy za znacznikiem, który CIOPFS zostawia w
obsługiwanych przez siebie katalogach. Gdy go brak, Wine zakłada WRAŻLIWOŚĆ na
wielkość liter, a każde wyszukanie, którego pisownia nie zgadza się bajt w bajt,
cofa się do przeczytania CAŁEGO katalogu w poszukiwaniu dopasowania bez względu na
wielkość liter. Gry Bethesdy proszą o `data/ccbgssse001-fish.bsa`, podczas gdy plik
nazywa się `ccBGSSSE001-Fish.bsa`, więc uruchamiało się to przy niemal każdym
zasobie: 4471 sondowań znacznika i 2236 pełnych ponownych odczytów katalogu w osiem
sekund oraz 195796 wyliczeń `Data` w dziewięćdziesiąt. Skyrim SE nigdy nie dochodził
do menu głównego - siedział na 240 MB rezydentnych, podczas gdy demon spalał 92 %
rdzenia.

Eidos składał wielkość liter w `resolve_read` od samego początku. Cały koszt brał się
z tego, że nigdy tego nie powiedział. `lookup` odpowiada teraz `.ciopfs`; `readdir`
nadal go nie wypisuje.

Dwie rzeczy uczyniły to śmiertelnym, a nie tylko powolnym. Koszt rośnie z rozmiarem
katalogu, więc zainstalowanie zawartości Anniversary (`Data` z 37 plików do 177)
przeważyło szalę. A `opendir` zachłannie budował scaloną listę, co jest czystym
marnotrawstwem, gdy Wine otwiera katalog tylko po to, by zrobić `stat` na tym
znaczniku w środku - migawka jest teraz robiona przy pierwszym `readdir`.

Po: menu główne, 2,1 GB rezydentnych, demon przy 0 % CPU.

`EIDOS_FUSE_TRACE=opendir` to właśnie to, co go znalazło, i jest dostarczane.
Liczniki operacji mówią ile; 195796 wyliczeń jednego katalogu jest w sumie
niewidoczne.

**To, że gra przepisywała `plugins.txt` na pusty**, było bardzo prawdopodobnie tym
samym - `Data`, którego nie mogła wyliczyć w rozsądnym czasie, więc wywnioskowała, że
nic tam nie ma, i to zapisała. Nieudowodnione i warte ponownego sprawdzenia. Tak czy
inaczej, zabezpieczenie przechwytywania (przechwycenie, które całkowicie czyści zbiór
aktywnych, jest odrzucane przy każdym rozmiarze) sprawia, że nie może to już
uszkodzić profilu.

**`FOPEN_KEEP_CACHE` jest wyłączony.** Naprawione i warto wiedzieć dlaczego. Wywalał
Skyrim SE na wyłuskaniu wskaźnika zerowego kilka sekund po menu głównym,
deterministycznie, przy zerze zainstalowanych modów; pozostałe trzy cache po stronie
jądra zostały wyeliminowane pojedynczo przez bisekcję i tylko ten miał znaczenie.
Jego utratę zmierzono wtedy jako darmową, ale ten pomiar wykonano przy aktywnym
`FUSE_PASSTHROUGH`, gdzie demon obsługuje *zero* odczytów (`EIDOS_FUSE_STATS`
raportował `read 0` przy pełnym wczytaniu), a jądro i tak już buforowało te strony
względem pliku bazowego. Passthrough jest teraz domyślnie wyłączony (poniżej), więc
tamten argument już nie obowiązuje, a rzeczywisty koszt pozostaje niezmierzony - sama
awaria i tak wystarcza, by zostawić go wyłączonym. Włącz z powrotem
`EIDOS_FUSE_KEEP_CACHE=1`, by badać; obie flagi nie są już splątane, więc teraz można
go testować osobno.

### Passthrough FUSE nie pozwala grze wczytać żadnej zawartości modów

Naprawione przez wyłączenie; `EIDOS_FUSE_PASSTHROUGH=1` przywraca go. Przy włączonym
passthrough Skyrim SE nie potrafi otworzyć 152 własnych plików (75 `.bsa`, 65 `.esl`,
10 `.esm`, 2 `.esp`) z `STATUS_ACCESS_VIOLATION`, wobec 0 przy wyłączonym, na jądrze
7.1.4 - a więc żadna zawartość modów się nie wczytuje, po cichu. Jądro zgłasza błąd
po tym, jak demon odpowiedział `opened_passthrough`, więc własne dzienniki demona
pokazują czysty przebieg (zero nieudanych otwarć, zero odrzuconych plików bazowych).
Przyczyna źródłowa w ścieżce jądra nie została ustalona; przełącznik zostaje, żeby
dało się to przetestować ponownie i żeby passthrough można było zawęzić wyłącznie do
bibliotek DLL, gdyby okazało się, że mapowanie obrazów tego potrzebuje.
