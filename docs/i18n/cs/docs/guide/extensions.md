<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Rozšíření

Rozšíření přidá Eidosu položku, aniž by bylo součástí Eidosu. Je to TOML
manifest pojmenovávající program plus - nanejvýš - ten program.

Manifesty leží v `~/.config/Colony/Eidos/addons/`, jeden `.toml` na rozšíření.
Otevřete složku přes View -> Extensions -> Open folder a stiskněte Reload
- žádný restart.

`id`, `kind` a `exec` jsou povinné, `name` se bez zadání rovná `id`. `exec`
pojmenovává program. Jméno bez `/` se hledá v `PATH`. Když program chybí, Eidos
rozšíření přeskočí a jeho řádek ve View -> Extensions řekne proč (`<exec> is not
on PATH` nebo `<exec> is not there`). `games` omezuje rozšíření na uvedené
identifikátory her. Bez něj platí pro každou hru a u jiné hry jeho řádek
ukazuje "not for this game".

Manifest, který Eidos odmítne, se vypíše na začátku i s důvodem: soubor, který
nejde přečíst, neplatný TOML nebo chybějící povinné pole, prázdné `id` nebo `id`
s mezerou, prázdné `exec`, `id`, které už převzal dřívější manifest (soubory se
čtou podle názvu), `kind`, který Eidos nezná, nebo instalátor, náhled či
informace o uložených hrách bez `protocol = 1` a názvu souboru `extensions`.

## Proč se do Eidosu nic nenačítá

Mod Organizer 2 načítá zásuvné moduly jako sdílené knihovny a ty pythonovské
hostuje přes Qt. Ani jedno se sem nepřenáší. Rust nemá stabilní ABI, takže sdílená
knihovna sestavená jiným překladačem - nebo s jiným optimalizačním přepínačem či
jinou sadou vlastností sdílené závislosti - je nedefinované chování a nic ho
nenahlásí jako neshodu verzí. A widgety Eidosu jsou generické v době překladu, takže knihovna by žádný
nedokázala postavit a vrátit, ani kdyby ABI stabilní bylo.

Rozšíření je program, který Eidos spouští s přístupem vašeho účtu k souborovému systému. Instalujte pouze pomocné programy, kterým důvěřujete. Oddělení procesů a kontrola odpovědí neposkytují sandbox operačního systému.

## Nástroj

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # vynechte pro všechny hry
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Objeví se ve View -> Extensions s tlačítkem Run a startuje odpojeně - Eidos na
něj nečeká a stavový řádek uvede jeho pid. Výstup se zahazuje. Bez `workdir`
startuje ve složce, ze které byl spuštěn sám Eidos.

## Kontrola

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Běží při každém obnovení, v kořeni instance, pokud `workdir` neříká jinak, a
vypisuje jedno zjištění na řádek:

```
level<TAB>title<TAB>detail
```

kde `level` je `problem`, `advice` nebo `ok`. Detail je volitelný, řádek bez titulku se zahodí.
Cokoli, co nezačíná známou úrovní, se ignoruje, takže výpis průběhu ani
zbloudilá varování nemohou vyrobit řádek, který vypadá jako vlastní kontrola
Eidosu. Zjištění se objeví v záložce Diagnostics jako `<name> - <title>`, za
kartami prefixu a Nexusu a před ostatními kartami Health.

Kontrola dostane tři sekundy. Ta, která je přetáhne, se zastaví a nahlásí jako
problém sama proti sobě - běží při témže obnovení, které následuje po každém
kliknutí, takže zaseknutá by okno zmrazila. Čas běží, dokud kontrola neskončí a nezavřou
se její výstupní roury, takže potomek na pozadí, který je stále drží, se do tří
sekund počítá. Stejně se nahlásí kontrola, která skončí chybou a nic nevypíše,
s prvním řádkem jejího chybového výstupu, i ta, která vypíše na kterémkoli
proudu víc než 1 MiB. V obou případech ji Eidos znovu nespustí, dokud
nestisknete Reload, a její řádek říká "It will not be retried until you press
Reload." Kontrola nečte nic ze standardního vstupu.

## Zástupné symboly

`args` i `workdir` rozvíjejí tyto:

| Zástupný symbol | Co to je                                     |
| --------------- | -------------------------------------------- |
| `{instance}`    | kořen instance                               |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | název aktivního profilu                      |
| `{profile_dir}` | adresář aktivního profilu                    |
| `{game}`        | identifikátor hry, např. `skyrimse`          |
| `{game_name}`   | zobrazovaný název hry                        |
| `{install}`     | složka hry, kterou instance spouští          |
| `{data}`        | adresář `Data` v této složce                 |

`{install}` je instalace z obchodu, nebo kopie hry, pokud instance běží z ní
(zmrazený `Stock Game` nebo vlastní složka hry v seznamu; viz
[usage.md](usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie)). `{data}` ji
následuje.

Kontrola dostane navíc `{addon_dir}`, složku s jejím manifestem, a stejně tak
rozšíření protokolu 1 níže. Nástroj ji nedostane.

Neznámý zástupný symbol se nikdy nevyprázdní, to by proměnilo `--out {typo}` v
`--out --next-flag`. Eidos odmítne spustit nástroj nebo kontrolu, jejichž
zástupné symboly nelze všechny vyřešit, a řekne, které chybí: u nástroje na
stavovém řádku, u kontroly v jejím řádku v záložce Diagnostics.

## Co rozšíření nemůže

Rozšíření nemůže zpětně volat Eidos, zaregistrovat se z archivu ani kreslit vlastní widgety. Strukturované instalátory protokolu 1 mohou vracet kontrolované plány souborů a výzvy k výběru; kontrolu, přípravu v dočasném adresáři a zveřejnění řídí Eidos. FOMOD a BAIN zůstávají nativními instalátory. Rozšíření pro náhled a informace o uložených hrách používají stejného hostitele s omezenými odpověďmi. Prioritu přiřazování, zaznamenané odpovědi CLI, limity a kontroly identity při opakování popisuje [specifikace protokolu se spustitelnými příklady](../../../../guide/examples/extensions/README.md).
