<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 skrz Eidos

Fallout 4 nepotřebuje žádný zvláštní parametr spuštění, žádný přejmenovaný
spustitelný soubor a žádný obalovací skript. Stojí za to to říct rovnou, protože
každý jiný linuxový návod k F4SE tvrdí opak - a jejich rada se rozpadne při další
aktualizaci Steamu.

## Parametr spuštění

```
~/.local/bin/eidos-gui %command%
```

Cílem spuštění Steamu pro Fallout 4 je `Fallout4Launcher.exe`, nikdy
`Fallout4.exe`, takže rozběhat script extender je ve skutečnosti otázka „jak
přimět Steam spustit jiný program". Obvyklé odpovědi přepisují `%command%` v bashi:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

nebo kopírují `f4se_loader.exe` přes `Fallout4Launcher.exe`, což Steam potichu
obnoví při každé aktualizaci hry - načež hrajete bez F4SE a nic to neřekne.

Eidos výměnu provede sám, podle deskriptoru hry: nahradí launcher souborem
`f4se_loader.exe`, když je nainstalovaný, spadne zpět na `Fallout4.exe`, když není,
a **řekne vám**, kdy musel spadnout zpět. Hra, která se spustí se všemi F4SE módy
mrtvými, je horší než hra, která se nespustí.

Je i druhý důvod launcher nikdy nespouštět: znovu prohledá `Data` a přepíše
`plugins.txt`, čímž zruší právě nasazené pořadí načítání. Eidos jej nikdy nespouští.

## Co Eidos zařídí za vás

| | |
|---|---|
| Zneplatnění archivů | `Fallout4Custom.ini` dostane `[Archive]` `bInvalidateOlderFiles=1` a prázdné `sResourceDataDirsFinal=` - dva klíče, díky nimž jsou volné soubory mimo `Data` vůbec vidět. Zapisuje se do profilu, ne do složky hry. |
| Pořadí načítání | `plugins.txt` ve formátu s hvězdičkou, který Fallout 4 používá (`*` značí aktivní), s respektovaným `Fallout4.ccc` pro implicitní pluginy Creation Clubu |
| LOOT | Řazení funguje stejně jako u Skyrimu - `eidos sort <instance>` stáhne masterlist `fallout4` |
| Uložené pozice | Pozice `.fos` a jejich cosavy `.f4se` se vypisují, kopírují a drží po profilech; panel detailů čte vlastní tabulku pluginů uložené pozice, takže pozice vyžadující plugin, který jste vypnuli, to řekne dřív, než ji načtete |
| Root módy | Vše, co mód přináší vedle spustitelného souboru (samotné F4SE, ENB, `dxvk.conf`), tam přistane týmž mechanismem `Root/`, jaký používá Skyrim |

## Otázka verzí

Fallout 4 už není ta zamrzlá hra z let 2019 až 2024. K srpnu 2026 existují tři živé
větve a DLL módu postavená pro jednu se v jiné nenačte:

| Větev | Verze | F4SE |
|---|---|---|
| Klasická („old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Dva důsledky, které je dobré znát před stavbou seznamu módů:

- **Ověřte, co skutečně máte.** Složky `Creations/` a `Mods/` v kořeni hry znamenají,
  že jste na linii 1.11.x. Panel detailů uložené pozice v Eidosu navíc ukazuje build,
  který ji zapsal - Fallout to do pozice píše a Eidos to vynáší jako „Game build".
- **Čerstvá záplata není dobrý den na začátek.** F4SE obvykle vyjde do dne či dvou
  po aktualizaci Bethesdy, ale *Address Library for F4SE Plugins* - přes kterou
  většina DLL módů řeší své offsety - jde vlastním tempem. Mezi tím leží DLL polovina
  ekosystému. Módy bez DLL (textury, meshe, pluginy) zůstávají nedotčené.

Jakmile vám sestava funguje, vypněte Steamu automatické aktualizace pro Fallout 4
(Vlastnosti → Aktualizace → „Aktualizovat tuto hru jen při spuštění"), jinak další
záplata rozbije každou nainstalovanou DLL.

## Poznámka k hardwaru: úlomky zbraní padají na NVIDII

Efekt úlomků zbraní ve Falloutu 4 běží na NVIDIA FleX, odvozenině PhysX, kterou
NVIDIA přestala podporovat po generaci Pascal. Na jakékoli kartě Turing a novější -
GTX 16, RTX 20 až RTX 50 - hru shodí. Je to chyba hry, nemá nic společného s
Linuxem, Protonem ani Eidosem.

Dvě nápravy, stačí kterákoli: vypněte „Weapon Debris" v nastavení hry, nebo
nainstalujte *Weapon Debris Crash Fix* (Nexus 48078), který vypíná kolizi úlomků
místo samotného efektu.

## Když něco vypadá špatně

Obecný kontrolní seznam je v [troubleshooting.cs.md](troubleshooting.cs.md); první
otázka specifická pro Fallout zní vždy *který spustitelný soubor se doopravdy
spustil*. Eidos zapisuje celý spouštěcí příkaz do běhového logu instance, takže:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Pokud jmenuje `f4se_loader.exe`, výměna proběhla. Pokud jmenuje
`Fallout4Launcher.exe`, F4SE není nainstalované tam, kde jej Eidos najde - patří
vedle spustitelného souboru hry, což u sestavy spravované módy znamená adresář
`Root/` nějakého módu (nebo samotnou složku hry, instalováno ručně).
