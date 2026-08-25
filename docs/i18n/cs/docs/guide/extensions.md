<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Rozšíření

Rozšíření přidá Eidosu položku, aniž by bylo součástí Eidosu. Je to TOML
manifest pojmenovávající program plus - nanejvýš - ten program.

Manifesty leží v `~/.config/Colony/Eidos/addons/`, jeden `.toml` na rozšíření.
Otevřete složku přes **View -> Extensions -> Open folder** a stiskněte **Reload**
- žádný restart.

## Proč se do Eidosu nic nenačítá

Mod Organizer 2 načítá zásuvné moduly jako sdílené knihovny a ty pythonovské
hostuje přes Qt. Ani jedno se sem nepřenáší. Rust nemá stabilní ABI, takže sdílená
knihovna sestavená jiným překladačem - nebo s jiným optimalizačním přepínačem či
jinou sadou vlastností sdílené závislosti - je nedefinované chování, ne neshoda
verzí. A widgety Eidosu jsou generické v době překladu, takže knihovna by žádný
nedokázala postavit a vrátit, ani kdyby ABI stabilní bylo.

Rozšíření je tedy program, který Eidos *spouští*. Nemůže shodit okno, nemůže
poškodit seznam módů a funguje dál napříč aktualizacemi Eidosu.

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

Objeví se ve **View -> Extensions** s tlačítkem Run a startuje odpojeně - Eidos na
něj nečeká.

## Kontrola

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Běží při každém obnovení a vypisuje jedno zjištění na řádek:

```
level<TAB>title<TAB>detail
```

kde `level` je `problem`, `advice` nebo `ok`. Detail je volitelný. Cokoli, co
nezačíná známou úrovní, se ignoruje, takže výpis průběhu ani zbloudilá varování
nemohou vyrobit řádek, který vypadá jako vlastní kontrola Eidosu. Zjištění se
objeví v záložce **Health**, s názvem rozšíření jako předponou.

Kontrola dostane tři sekundy. Ta, která je přetáhne, se zastaví a nahlásí jako
problém sama proti sobě - běží při témže obnovení, které následuje po každém
kliknutí, takže zaseknutá by okno zmrazila.

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
| `{install}`     | instalační adresář hry                       |
| `{data}`        | adresář `Data` hry                           |

Neznámý zástupný symbol zůstane přesně tak, jak byl napsán, místo aby se
vyprázdnil - aby chyba selhala viditelně a neproměnila `--out {typo}` v
`--out --next-flag`. Spuštění nástroje, jehož zástupné symboly nelze všechny
vyřešit, je odmítnuto a Eidos řekne, které chybí.

## Co rozšíření nemůže

Dostane hodnoty a běží; nemůže volat zpět do Eidosu, měnit seznam módů ani cokoli
kreslit v okně. To je záměr. To, k čemu MO2 používá zásuvné moduly a co skutečně
MUSÍ sáhnout dovnitř - podpora her, instalátory, engine konfliktů - je tu vestavěné,
ne přišroubované: definice hry je vlastní TOML v `~/.config/Colony/Eidos/games/` a
instalátory FOMOD a BAIN jsou nativní.
