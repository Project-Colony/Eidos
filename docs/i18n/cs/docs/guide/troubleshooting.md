<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Řešení potíží a diagnostika

Všechno pro den, kdy hra uvidí něco, s čím souborový systém nesouhlasí:
přepínače prostředí, jak číst čítače operací, známé problémy a jejich historie
a příběh passthroughu.

### Diagnostika VFS

Pro chvíle, kdy hra vidí něco, s čím souborový systém nesouhlasí, existují dvě
proměnné prostředí:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

Právě podrobná podoba našla pád popsaný v troubleshooting.md: vypnutí všech čtyř
odpoví na „je to cachování?", a teprve jejich pojmenování odpoví „které".
Čítače odpovídají na druhou půlku - načtení, které ukazuje `read 0`, je takové,
kde `FUSE_PASSTHROUGH` obsloužil každý bajt v jádře, takže cokoli, co jste se
chystali ladit na čtecí cestě, je už zadarmo.

## Připojit sjednocení ručně

První `--layer` vyhrává při konfliktu; poslední jsou vaše nedotčená data hry.
Připojení potřebuje jen `/dev/fuse` a `fusermount3` (žádný overlayfs, žádný
Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Zápisy přistávají v `--overwrite <dir>` (dočasný adresář, když jej vynecháte),
takže samotné vrstvy zůstávají nedotčené i tady.


#### Proč je passthrough ve výchozím stavu vypnutý

Passthrough předá jádru skutečný podkladový soubor, takže čtení tohoto démona
úplně obcházejí. Je to zisk v propustnosti, který tady stojí správnost. Měřeno
A/B na Skyrim SE 1.6.1170, proton-cachyos 11.0, jádro 7.1.4, stejné pořadí
načítání s 82 pluginy, jediná proměnná byla, zda binárka nesla tu capability:

| passthrough | selhání `NtCreateFile` se `STATUS_ACCESS_VIOLATION` |
|-------------|--------------------------------------------------------|
| zapnutý     | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| vypnutý     | 0                                                      |

Se zapnutým hra neotevře žádný ze svých archivů ani pluginů, což se ve hře
projeví jako módy, které prostě nejsou - žádná chyba, žádný řádek v logu.
S vypnutým totéž pořadí načítání dojde ke hraní se živými pluginy, archivy
a Papyrus skripty.

Selhání je zevnitř démona neviditelné, což ho udělalo drahým na nalezení: naše
vlastní `open` uspěje pokaždé a jádro nikdy neodmítne podkladový soubor
(ověřeno napříč celou selhávající relací s `EIDOS_FUSE_TRACE=open`: nula
`open FAILED`, nula `passthrough refused`). Chyba vzniká až poté, co démon
odpoví `opened_passthrough`, takže ji žádné logování na straně démona nevidí.
Není ani vázaná na příponu - trefuje archivy stejně jako pluginy, tedy soubory,
které hra drží otevřené po celý svůj běh.

`EIDOS_FUSE_PASSTHROUGH=1` jej zase zapne, pro měření toho, co přináší, nebo
pro opětovné testování mechanismu. Varování o capability ve spouštěči a na kartě
Diagnostics se objeví, jen když jste si o něj řekli.

Chcete-li spustit samotnou hru skrz Eidos, nastavte její parametry spuštění ve
Steamu na:

```
eidos play skyrimse -- %command%
```

Předřaďte `WINEDLLOVERRIDES="d3dcompiler_47=n"`, pokud Proton potřebuje nativní
d3dcompiler pro kompilaci shaderů; Eidos to sloučí s jakýmikoli DLL overrides,
které přináší mód (ENB/ReShade/`.asi` loadery).


### Používá se vlastně index vrstev?

Index je všechno nebo nic a staví se mlčky: `LayerStack::new` buď dostane
úplnou mapu vrstev jen pro čtení, nebo `None`, načež je každý dotaz prochází
přesně jako dřív. Nic v logu relace ty dva stavy nerozliší, takže zásobník,
který tiše spadl zpátky, vypadá stejně jako ten, který funguje - a přitom platí
starou cenu.

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` rozřeší skutečné cesty s indexem i bez něj a porovná průchody
adresáři. `index_agrees` kontroluje, že oba odpovídají TOTÉŽ, na každé cestě
a každém výpisu skutečné instance. `listing_cost` měří, co mapa sloučených
potomků ušetří na `readdir`.

`EIDOS_NO_INDEX=1` vynutí průchod, pro chvíle, kdy je laděným předmětem právě
rozdíl mezi oběma odpověďmi.

## Známé problémy

### DLSS nebo generování snímků tiše nedělá nic

Tři samostatné příčiny, každá bez jakékoli chybové hlášky: nezapnuté NVAPI
v parametrech spuštění, exkluzivní režim celé obrazovky, nebo zastaralý Reflex
limit FPS. Celý kontrolní seznam žije v [graphics.cs.md](graphics.md).

**Mód, který píše jeden adresář dvěma způsoby, přišel o všechno pod tím
druhým.** Opraveno. ext4 drží `meshes/` a `Meshes/` odděleně; sloučený pohled
nesmí, a skutečné módy dodávají obojí - XP32 Maximum Skeleton má své animace
a svůj FNIS behaviour soubor pod tím s velkým písmenem, své `character assets`
pod tím druhým.

Resolver vzal pro každou složku cesty shodu s přesnou velikostí písmen a držel
se jí: vstoupil do `meshes/`, nenašel tam zbytek cesty a zahodil CELOU VRSTVU.
Každý soubor pod druhým zápisem byl pro hru neviditelný - žádná chyba, žádný
log, nic v žádné diagnostice. Na skutečné instanci s 50 vrstvami to bylo 74
souborů.

Složka, která se shoduje, je teď kandidát, ne rozhodnutí; přesná velikost
písmen se stále zkouší první a teprve když pod ní selže zbytek, hledá průchod
sourozence shodné po sjednocení velikosti písmen. Výpisy měly tutéž chybu
o adresář výš a teď čtou každý takto shodný adresář na vrstvu.

Za povšimnutí kvůli tvaru toho celého: index cest tuhle chybu nikdy neměl,
protože prochází každý adresář, který najde. Tiše vracel soubory, které záložní
cesta vrátit nedokázala, což je obráceně - záložní cesta je ta odpověď, která
nemá být nikdy špatně.

**LODGen z DynDOLODu umírá a nechává prázdný log.** Opraveno pomocí `dotnet10`;
viz [tools.cs.md](tools.md). Příznak je nezaměnitelný:
`LODGen_SSE_<world>_log.txt` obsahuje hlavičku s verzí, řádek `.NET Version:`
a nic víc, a to pro každý svět, a dialog říkající pouze „failed to generate
object LOD for one or more worlds". Příčinou je Wine Mono odpovídající za .NET
Framework a žádné množství instalací .NET Frameworku to nespraví - Proton
nahrazuje `mscoree.dll` symlinkem do svého vlastního stromu při každé
aktualizaci prefixu.

**Wine nedokázal poznat, že připojení sjednocuje velikost písmen.** Opraveno,
a byl to ten problém, na kterém záleželo.

Neexistuje API pro „je tenhle souborový systém case-insensitive", takže Wine ve
své `get_dir_case_sensitivity` čichá po značce, kterou CIOPFS nechává
v adresářích, jež obsluhuje. Když chybí, Wine předpokládá case-SENSITIVE
a každé vyhledání, jehož zápis se neshoduje bajt po bajtu, spadne zpátky ke
čtení CELÉHO adresáře, aby našlo shodu bez ohledu na velikost písmen. Hry od
Bethesdy se ptají na `data/ccbgssse001-fish.bsa`, zatímco soubor je
`ccBGSSSE001-Fish.bsa`, takže to vystřelilo skoro u každého assetu: 4471 dotazů
na značku a 2236 úplných opětovných čtení adresáře za osm sekund a 195796 výčtů
`Data` za devadesát. Skyrim SE se nikdy nedostal do hlavní nabídky - seděl na
240 MB rezidentní paměti, zatímco démon spaloval 92 % jádra.

Eidos sjednocoval velikost písmen v `resolve_read` od začátku. Celá ta cena byla
za to, že to nikdy neřekl. `lookup` teď odpovídá `.ciopfs`; `readdir` jej stále
nevypisuje.

Fatálním, ne jen pomalým, to udělaly dvě věci. Cena roste s velikostí adresáře,
takže instalace obsahu Anniversary (`Data` ze 37 souborů na 177) to překlopila.
A `opendir` dychtivě stavěl sloučený výpis, což je čirá ztráta, když Wine
otevírá adresář jen proto, aby uvnitř udělal `stat` na tu značku - snímek se
teď bere až při prvním `readdir`.

Potom: hlavní nabídka, 2,1 GB rezidentní paměti, démon na 0 % CPU.

`EIDOS_FUSE_TRACE=opendir` je to, co to našlo, a dodává se. Čítače operací
říkají kolik; 195796 výčtů jednoho adresáře je v součtu neviditelných.

**Hra přepisující `plugins.txt` naprázdno** byla velmi pravděpodobně totéž -
`Data`, které nedokázala v rozumném čase vyjmenovat, takže usoudila, že tam nic
není, a uložila to. Neprokázáno a stojí za opětovné ověření. Tak či tak pojistka
zachytávání (zachycení, které úplně vyprázdní aktivní sadu, je odmítnuto při
jakékoli velikosti) znamená, že už profil poškodit nemůže.

**`FOPEN_KEEP_CACHE` je vypnutý.** Opraveno a stojí za to vědět proč. Shazoval
Skyrim SE na null dereferenci pár sekund po hlavní nabídce, deterministicky,
s nula nainstalovanými módy; ostatní tři cache na straně jádra byly
vybisektovány jednotlivě a záležel jen tenhle. Jeho ztráta byla tehdy naměřena
jako zadarmo, jenže to měření vzniklo s aktivním `FUSE_PASSTHROUGH`, kde démon
obsluhuje *nula* čtení (`EIDOS_FUSE_STATS` hlásil `read 0` za celé načtení)
a jádro už ty stránky cachovalo proti podkladovému souboru. Passthrough je teď
ve výchozím stavu vypnutý (níže), takže ten argument už neplatí a skutečná cena
je nezměřená - pád je stejně dost dobrý důvod nechat jej vypnutý. Znovu zapnete
přes `EIDOS_FUSE_KEEP_CACHE=1`, chcete-li to zkoumat; ty dva přepínače už nejsou
provázané, takže se teď dá testovat samostatně.

### FUSE passthrough brání hře načíst jakýkoli obsah módů

Opraveno tím, že se vypnul; `EIDOS_FUSE_PASSTHROUGH=1` jej vrátí zpět. Se
zapnutým passthroughem Skyrim SE neotevře 152 vlastních souborů (75 `.bsa`, 65
`.esl`, 10 `.esm`, 2 `.esp`) se `STATUS_ACCESS_VIOLATION`, proti 0 s vypnutým,
na jádře 7.1.4 - takže se tiše nenačte žádný obsah módů. Jádro vyvolá chybu
poté, co démon odpověděl `opened_passthrough`, takže vlastní logy démona
ukazují čistý běh (nula neúspěšných otevření, nula odmítnutých podkladových
souborů). Kořenová příčina v cestě jádrem není zjištěna; přepínač se zachovává,
aby šlo mechanismus znovu otestovat a aby se passthrough dal zúžit jen na DLL,
kdyby se ukázalo, že to mapování obrazů potřebuje.
