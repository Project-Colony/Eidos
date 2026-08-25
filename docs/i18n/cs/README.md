<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Nativní správce módů pro Linux, který se nikdy nedotkne vaší hry.**

</div>

Eidos dává hrám od Bethesdy na Linuxu to, co jim na Windows dává
Mod Organizer 2 - virtuální sloučený pohled na vaše módy, vytvořený znovu při
každém spuštění - postavený na linuxových primitivech místo hookování
Windows API. Žádný Wine pro správce. Žádné soubory kopírované do složky hry.
Žádný postup úklidu, protože není co uklízet.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Stav:** Skyrim SE se přes Eidos hraje denně - SKSE, preloadery script
> extenderu, Creation Club, pořadí načítání seřazená LOOTem, uložené pozice po
> profilech, všechno. Zatím jedna rodina her ověřená skutečným hraním; dalších
> deset je zapojeno a čeká na testery.

## Proč Eidos

- 🔒 **Připojení, které vidí jen vaše hra.** Sloučený pohled žije v soukromém
  jmenném prostoru připojení: váš správce souborů, vaše zálohovací úloha, druhá
  hra - žádný z nich ho nevidí, žádný pro něj nepotřebuje oprávnění. Zabijte
  hru, vytáhněte napájení: jmenný prostor umírá se stromem procesů a vaše
  instalace je přesně taková, jaká byla. Nezůstávají žádné zbytky *už
  z konstrukce*.
- 🧾 **Jediná kopie pravdy.** Váš profil vlastní svůj seznam módů, pořadí
  pluginů, INI soubory a uložené pozice. Soubory pluginů a složka uložených
  pozic se při spuštění připojí vazbou přes vlastní cesty hry, takže i zápisy
  samotné hry přistanou ve vašem profilu. Přepnutí profilu přepne všechno.
- 🐧 **Zcela bez rootu.** Žádný setuid pomocník, žádný démon, žádné
  `sudo setcap`, žádné úpravy `/etc/fuse.conf`. Jeden binární soubor, jeden
  parametr spuštění ve Steamu.
- 🛡️ **Pojistky, které doloží proč.** Pád, který zničí váš seznam pluginů, se
  označí proti snímku pořízenému před sezením a obnova je na jedno kliknutí.
  Zachycení, které by smazalo vaše pořadí načítání, je odmítnuto a řekne proč.

## Co umí

**Módy.** Jednoduché archivy, průvodci FOMOD, balíčky BAIN z Wrye Bash, ruční
výběr pro zbytek - a **root módy nativně** (preloadery script extenderu, ENB,
Engine Fixes), bez pluginu Root Builder a bez čehokoli kopírovaného do vaší
instalace. Skrývání jednotlivých souborů, seskupování oddělovači, cílené
přesuny, poznámky a kategorie u jednotlivých módů a import profilů z MO2.

Seznam je z MO2, i s jeho zvyky: osm volitelných sloupců a řazení podle
kteréhokoli z nich, seskupení podle kategorie nebo podle zdroje, gesta
dvojklikem, skok psaním, zálohy jednotlivých módů, které nic nedělají, dokud je
neobnovíte, a upozorňující příznaky u módu, jehož rozvržení tato hra nenačte
nebo který byl stažen pro jinou. Jeho strom souborů zvládá běžné operace - nová
složka, přejmenovat, smazat, otevřít - a zobrazí náhled obrázků a textu, aniž by
cokoli spouštěl.

**Pluginy.** Pořadí načítání s vestavěným řazením LOOTem, indexy módů tak, jak je
počítá hra, varování na chybějící mastery a váš obsah z DLC a Creation Clubu
zobrazený jako nespravované řádky, kterými je.

**Instance.** Globální - spravované centrálně pod `~/.local/share/eidos` - nebo
přenosné: samostatná složka kdekoli chcete (druhý disk, herní oddíl),
přesunutelná a izolovaná, jako v MO2. Přenosné instance si Eidos pamatuje mezi
sezeními; GUI, spuštění ze Steamu i každý příkaz z příkazové řádky se drží té,
kterou jste použili naposledy, a každý příkaz bere složku všude tam, kde bere
identifikátor hry. Podrobnosti v
[usage.cs.md](docs/guide/usage.md#instance-globální-a-přenosné).

**Profily.** Pořadí módů, stav pluginů, INI soubory a uložené pozice zvlášť pro
každý profil. Uložené pozice se rozeberou, porovnají s vašimi současnými
pluginy - s tlačítkem, které zapne to, co daná pozice potřebuje - a po každém
sezení se synchronizují zpět pro Steam Cloud.

**Nexus.** Připojte účet a tlačítko „Mod Manager Download" na webu přistane
rovnou ve vaší instanci, spolu s kontrolou aktualizací proti tomu, co máte
nainstalováno, s tím, kdo který mód vytvořil, a s odkazem na jeho profil. Odkaz
na **kolekci** vypíše její členy spárované s vaší instancí - nainstalováno,
staženo, chybí - což je čtení kolekce, ne její instalace, a panel řekne proč.
Karta Downloads je knihovna archivů: filtrovat, řadit, skrýt bez mazání a
vyčistit ty, které už jsou nainstalované. Přepínač **offline** to všechno
zastaví.

**Nástroje.** xEdit, BodySlide, DynDOLOD a spol. běží *skrz sloučený pohled*
uvnitř Proton prefixu dané hry - vidí vaše módy, jejich výstup přistane
v Overwrite a jedno kliknutí z něj udělá skutečný mód. Jakýkoli runtime, který
kterýkoli z nich potřebuje, se stáhne na vyžádání, takže chybějící DLL je
tlačítko, ne odpoledne. xEdit a jeho dvojče QuickAutoClean si Eidos najde sám -
ve složce hry, uvnitř módu nebo v adresáři s nástroji, který si držíte vedle
svých her - a rovnou zvolí správné runtimy. Připněte si ty, které používáte,
skryjte ty, které ne, dejte nástroji vlastní
Steam AppID, když je sám o sobě aplikací na Steamu, a zapište zástupce
`.desktop`, který jej spustí skrz sloučený pohled, aniž by se Eidos vůbec
otevřel.

**Diagnostika.** Chybějící mastery, osiřelé archivy, rozejití seznamu módů,
poškozené sady pluginů - a po skončení běhu to, co podle vlastního logu script
extenderu skutečně načetlo.

**Kde si drží vlastní soubory.** `~/.config/Colony/Eidos/` pro to, co jste
zvolili - předvolby, vaše sezení na Nexusu, váš seznam instancí, definice her a
doplňků, které jste napsali - a logy pod `~/.local/state/Colony/Eidos/`.
Rozvržení, které používá každý program z rodiny Colony. Starší Eidos je držel
v `~/.config/eidos/`; první spuštění po aktualizaci je zkopíruje, napíše to do
logu a starý adresář nechá přesně tak, jak byl.

## Jak si stojí ve srovnání

| | Eidos | MO2 přes Wine | Fluorine-Manager | Limo / nasazení přes odkazy |
|---|---|---|---|---|
| Správce běží nativně | ✅ | ❌ aplikace Windows ve Wine | ✅ (port do Qt) | ✅ |
| Složka hry nedotčená | ✅ vždy | ✅ | ✅ | ❌ zapisují se do ní odkazy |
| Připojení viditelné pro | jen hru | jen hru | **celý systém** | neuplatňuje se |
| Nutný úklid po pádu | žádný, už z návrhu | žádný | obnova po mrtvém připojení | ruční zrušení nasazení |
| Root módy (ENB, preloadery) | ✅ nativně | vyžaduje plugin | vyžaduje plugin | částečně |
| Vyžadovaná oprávnění | žádná | žádná | úprava `/etc/fuse.conf` | žádná |

## Jak je rychlý

| | dříve | nyní |
|---|---|---|
| načtení uložené pozice | ~20 sekund | **6-7 sekund** |
| čtení adresářů v jednom sezení | 5,6 milionu | 465 tisíc |

Přechody mezi buňkami jsou okamžité. Zisk přišel z toho, že se vašich módů ptáme
méně: hledání jednoho souboru dřív vyslýchalo všech padesát po řadě a výpis
jedné složky to dělal padesátkrát. Ani jedno už to nedělá. Měřeno na skutečné
instanci hrané normálně, ne na benchmarku.

## Začínáme

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Potom nastavte parametr spuštění vaší hry ve Steamu na
`~/.local/bin/eidos-gui %command%` a stiskněte Hrát.

Balíčky pro Arch a archivy vydání, co je potřeba mít nainstalováno nejdřív, a
cesta přes příkazovou řádku:
**[docs/guide/install.cs.md](docs/guide/install.md)**.

## Parametry spuštění ve Steamu

Základní řádek je všechno, co většina sestav potřebuje:

```
~/.local/bin/eidos-gui %command%
```

Všechno ostatní jsou proměnné prostředí naskládané před něj a volně se
kombinují:

| Chcete... | Dejte dopředu |
|---|---|
| DLSS s Community Shaders | `PROTON_ENABLE_NVAPI=1` - bez ní se DLSS tiše nikdy neinicializuje; kompletní seznam je [guide/graphics.cs.md](docs/guide/graphics.md) |
| počítadlo FPS na obrazovce | `DXVK_HUD=fps` |
| interpolaci snímků na úrovni ovladače, nula módů (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nikdy zároveň s vlastní generací snímků z Community Shaders |
| podrobné logy pro hlášení chyby | `EIDOS_LOG=debug` (logy sezení přistanou v `~/.local/state/Colony/Eidos/logs/`) |
| zprávu o I/O z připojení za jedno sezení | `EIDOS_FUSE_STATS=1` |
| jiný počet FUSE workerů | `EIDOS_FUSE_THREADS=8` (výchozí 4; `1` je první věc, kterou zkusit při honu na souběhovou chybu) |
| toto spuštění připnuté k jedné přenosné instanci | `EIDOS_INSTANCE=/path/to/folder` - bez ní Eidos otevře instanci, kterou jste použili naposledy, což je obvykle to, co chcete |

Řádek, který si nechat pro moderní moddovanou sestavu (Community Shaders, DLSS,
generace snímků) - tohle je finální příkaz, ne příklad:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Přidejte dopředu `DXVK_HUD=fps`, dokud ověřujete, že sestava funguje, a jakmile
funguje, zase ho odeberte.

Hlubší diagnostické přepínače (`EIDOS_FUSE_TRACE`, přepínače pro bisekci cache a
indexu, proč je `EIDOS_FUSE_PASSTHROUGH` ve výchozím stavu vypnutý) žijí
v [guide/troubleshooting.cs.md](docs/guide/troubleshooting.md).

## Kam dál

| Pokud chcete... | |
|---|---|
| nainstalovat ho | [guide/install.cs.md](docs/guide/install.md) |
| naučit se příkazovou řádku a GUI | [guide/usage.cs.md](docs/guide/usage.md) |
| nastavit xEdit, BodySlide nebo DynDOLOD | [guide/tools.cs.md](docs/guide/tools.md) |
| hrát Fallout 4 (F4SE, verze, pád na NVIDIA debris) | [guide/fallout4.cs.md](docs/guide/fallout4.md) |
| rozchodit DLSS / generaci snímků (Community Shaders) | [guide/graphics.cs.md](docs/guide/graphics.md) |
| opravit něco, co vypadá špatně | [guide/troubleshooting.cs.md](docs/guide/troubleshooting.md) |
| vědět, proč je rychlý, a ověřit si to sami | [internals/performance.md](../../internals/performance.md) |
| porozumět tomu, jak funguje uvnitř | [internals/architecture.md](../../internals/architecture.md) |
| sestavit ho, otestovat, přispět | [internals/contributing.md](../../internals/contributing.md) |
| vědět, proč vůbec existuje | [project/landscape.md](../../project/landscape.md) |

Jazyk je jeden adresář: `docs/i18n/cs/` zrcadlí kořen repozitáře, takže odkaz mezi
dvěma přeloženými stránkami je tentýž řetězec jako odkaz mezi jejich anglickými
originály.

## Jazyk

Stránky, které hráč potřebuje, jsou přeložené. **Kanonická je angličtina**: když
s ní překlad nesouhlasí, pravdu má anglický soubor.

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

**Všechno ostatní je anglicky záměrně, ne opomenutím.** `docs/internals/` a
`docs/project/` čtou lidé, kteří zároveň čtou Rust, a `CHANGELOG.md` se generuje.
Jejich překlad by znamenal dalších 17 678 slov, které je třeba udržovat poctivé,
pro publikum, které je nepotřebuje.

Každý překlad nese hash anglického souboru, ze kterého vznikl, a CI selže, když
se angličtina pohne dopředu - viz
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). Překlad, který nelze vrátit do
aktuálního stavu, se **smaže**, nenechá se ležet: zastaralá stránka pořád vypadá
autoritativně a rozdává příkazy z minulého měsíce, což je pro čtenáře horší než
být poslán na angličtinu.

Přidání jazyka jsou čtyři soubory a řádek v této tabulce;
[`docs/internals/contributing.md`](../../internals/contributing.md) má postup.

## Podporované hry

**Skyrim SE/AE** - ověřený skutečným hraním. **Fallout 4** je zapojený od
začátku do konce také (F4SE se podsune automaticky, invalidace archivů, pořadí
načítání s hvězdičkami, LOOT, uložené pozice `.fos`) - viz
[guide/fallout4.cs.md](docs/guide/fallout4.md). Zapojené podle sdíleného
deskriptoru her a hledající testery: Skyrim LE, Skyrim VR, Enderal SE,
Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion a Morrowind
(poslední dvě se připojí a spravují módy; jejich seznamy pluginů řazené podle
časových značek zatím spravované nejsou).

Přidání rodiny je jeden řádek deskriptoru:
[internals/adding-games.md](../../internals/adding-games.md).

## Předchozí práce a poděkování

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) a
  [usvfs](https://github.com/ModOrganizer2/usvfs) - sémantika, kterou Eidos
  reprodukuje, a kódová základna, proti které se studovala jeho parita
- [LOOT](https://loot.github.io/) - řadicí engine, přes libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) a ostatní linuxoví správci - důkaz,
  že existuje komunita, která chce tohle vyřešit

## Licence

GPL-3.0-or-later. Správa módů patří všem.
