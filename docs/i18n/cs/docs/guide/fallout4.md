<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 skrz Eidos

S Eidosem Fallout 4 nepotřebuje žádný zvláštní parametr spuštění, žádný
přejmenovaný spustitelný soubor a žádný obalovací skript. Používá stejný
parametr spuštění jako každá jiná hra.

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

nebo kopírují `f4se_loader.exe` přes `Fallout4Launcher.exe`, což Steam obnoví při
každé aktualizaci hry - načež hrajete bez F4SE a nic to neřekne.

Eidos výměnu provede sám, podle deskriptoru hry. Když okno hru spouští, nahradí
`Fallout4Launcher.exe` souborem `f4se_loader.exe`, pokud je loader ve složce hry,
v `Root/` zapnutého módu nebo v `Overwrite/Root/`. Bez loaderu spustí
`Fallout4.exe` a stavový řádek hlásí
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Když tam není ani jeden soubor, spustí launcher beze změny a také to oznámí. Při
parametru spuštění `eidos play fallout4 -- %command%` CLI vymění soubor jen tehdy,
když loader existuje, jinak spustí `Fallout4Launcher.exe`.

Je i druhý důvod launcher nespouštět: znovu prohledá `Data` a přepíše
`plugins.txt`, čímž zruší právě nasazené pořadí načítání.

Když okno otevřete sami a stisknete Run, Eidos požádá Steam o spuštění hry
(`steam -applaunch 377160`) a Steam spustí Eidos přes parametr spuštění výše.
Pokud `f4se_loader.exe` v instanci neleží vedle spustitelného souboru hry, Eidos
hru nespustí a stavový řádek vás požádá, abyste nejdřív nainstalovali script
extender. Fallout 4 VR funguje stejně, s `Fallout4VRLauncher.exe` a
`f4sevr_loader.exe`.

## Co Eidos zařídí za vás

| Oblast | Co Eidos dělá |
|---|---|
| Zneplatnění archivů | `Fallout4Custom.ini` dostane `[Archive]` `bInvalidateOlderFiles=1` a prázdné `sResourceDataDirsFinal=` - dva klíče, díky nimž jsou volné soubory mimo `Data` vůbec vidět. Zapisuje se do profilu, ne do složky hry. |
| INI soubory | `Fallout4.ini`, `Fallout4Prefs.ini` a `Fallout4Custom.ini` patří profilu. BethINI Pie spuštěné přes Eidos upravuje kopie profilu a Backups -> INIs vrátí kopii, kterou Eidos uchovává před každým svým během ([tools.md](tools.md#bethini-pie)). |
| Pořadí načítání | `plugins.txt` ve formátu s hvězdičkou, který Fallout 4 používá (`*` značí aktivní), s respektovaným `Fallout4.ccc` pro implicitní pluginy Creation Clubu |
| LOOT | Řazení funguje stejně jako u Skyrimu - `eidos sort <instance>` stáhne masterlist `fallout4` |
| Uložené pozice | Pozice `.fos` a jejich cosavy `.f4se` se vypisují, kopírují a drží po profilech; panel detailů čte vlastní tabulku pluginů uložené pozice, takže pozice vyžadující plugin, který jste vypnuli, to řekne dřív, než ji načtete. Vypnutí nebo odebrání módu, jehož plugin používá jedna ze tří nejnovějších pozic profilu, se nejdřív zeptá. |
| Root módy | Vše, co mód přináší vedle spustitelného souboru (samotné F4SE, ENB, `dxvk.conf`), tam přistane týmž mechanismem `Root/`, jaký používá Skyrim |

## Otázka verzí

Fallout 4 už není ta zamrzlá hra z let 2019 až 2024. K srpnu 2026 existují tři živé
větve a DLL módu postavená pro jednu se v jiné nenačte:

| Větev | Verze | F4SE |
|---|---|---|
| Klasická („old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 to 1.11.240 | 0.7.4 to 0.7.9 |

Dva důsledky, které je dobré znát před stavbou seznamu módů:

- Složky `Creations/` a `Mods/` v kořeni hry znamenají, že jste na linii 1.11.x.
  Panel detailů uložené pozice v Eidosu navíc ukazuje build, který ji zapsal:
  Fallout to do pozice píše a Eidos to ukazuje jako „Game build".
- F4SE obvykle vyjde do dne či dvou po aktualizaci Bethesdy, ale *Address
  Library for F4SE Plugins* - přes kterou většina DLL módů řeší své offsety -
  jde vlastním tempem. Mezi tím se DLL módy nenačtou. Módy bez DLL (textury,
  meshe, pluginy) zůstávají nedotčené, takže čerstvá záplata je špatný den na
  začátek seznamu módů.

Jakmile vám sestava funguje, zmrazte verzi hry do instance: Freeze this version
na kartě Game files v záložce Diagnostics, nebo `eidos stock-game fallout4 freeze`
pro zobrazení velikosti a `eidos stock-game fallout4 freeze --yes` pro samotné
kopírování. Eidos zkopíruje instalaci z obchodu do `<instance>/Stock Game` a hru
spouští z této kopie, stále přes Steam. Pozdější záplata změní instalaci z
obchodu a ne kopii, takže nainstalované DLL si drží svou verzi hry. Na btrfs nebo
XFS kopie sdílí bloky s obchodem a zabere skoro žádné místo, dokud Steam nezmění
soubor; na ext4 je to plná kopie velká jako složka hry. Eidos odmítne, dokud
Steam hlásí hru jako zaneprázdněnou nebo ne zcela nainstalovanou. Podrobnosti
jsou v [usage.md](usage.md#soubory-hry-instalace-z-obchodu-nebo-kopie).

Bez kopie vypněte Steamu automatické aktualizace pro Fallout 4 (Vlastnosti,
Aktualizace, „Aktualizovat tuto hru jen při spuštění"), jinak další záplata
rozbije každou nainstalovanou DLL.

## Poznámka k hardwaru: úlomky zbraní padají na NVIDII

Efekt úlomků zbraní ve Falloutu 4 běží na NVIDIA FleX, odvozenině PhysX, kterou
NVIDIA přestala podporovat po generaci Pascal. Na jakékoli kartě Turing a novější -
GTX 16, RTX 20 až RTX 50 - hru shodí. Je to chyba hry, nemá nic společného s
Linuxem, Protonem ani Eidosem.

Dvě nápravy, stačí kterákoli: vypněte „Weapon Debris" v nastavení hry, nebo
nainstalujte *Weapon Debris Crash Fix* (Nexus 48078), který vypíná kolizi úlomků
místo samotného efektu.

## Karty Health pro Fallout 4

Záložka Diagnostics ukazuje kartu Health pro každý z těchto případů:

- Pluginy F4SE jsou nainstalované v `F4SE/Plugins`, ale `f4se_loader.exe` není v
  žádném zapnutém módu ani ve složce hry. Hra se spustí bez nich.
- `f4se_loader.exe` je nainstalovaný, ale `f4se_<version>.dll` pro verzi běžící
  hry (`f4se_1_10_984.dll` pro 1.10.984) chybí. Loader skončí s chybou; obvykle to
  následuje po aktualizaci hry.
- Spustitelný soubor hry nebo její build ve Steamu se změnil od posledního
  spuštění přes Eidos, které skončilo bez chyby. Pluginy F4SE postavené pro
  starou verzi se mohou přestat načítat.
- Mód s vyšší prioritou než F4SE má volné kopie `.pex` vlastních skriptů F4SE.
  Stará kopie rozbije funkce F4SE, které deklaruje, a hra nezobrazí žádnou chybu.
- `Fallout4Custom.ini` profilu má v `[General]` řádky `sTestFile1` až
  `sTestFile10`. Dokud je některý nastaven, hra načítá tyto pluginy a ignoruje
  `plugins.txt`. Fix je odstraní a předchozí soubor zachová jako
  `Fallout4Custom.ini.pre-fix`.
- Archiv BA2 nějakého módu ukládá zvuk `.wav` nebo `.xwm` komprimovaný. Hra může
  při jeho přehrání spadnout. Přebalte archiv se zvukem bez komprese. Stejnou
  kartu dostane i Fallout 4 VR.
- Záznamník pádů zapsal log během posledního spuštění: `crash-*.log` od Buffout 4
  v `Documents/My Games/Fallout4/F4SE`, nebo `.log` v `F4SE/Crashlogs`. Karta
  jmenuje výjimku a ty DLL a pluginy z logu, které instalují vaše módy.
- Karta Display ukazuje velikost a režim okna nastavené v `Fallout4Prefs.ini`
  profilu a Apply tam zapíše `iSize W`, `iSize H`, `bFull Screen` a
  `bBorderless`.
- Požadavek z Nexusu jmenuje DLC. Eidos hledá plugin s názvem DLC a pluginy DLC
  Falloutu 4 mají jiná jména (`DLCCoast.esm` pro Far Harbor), takže tato karta
  je rada: zkontrolujte ručně a pak stiskněte Ignore.

Obě karty loaderu mají tlačítko Open the download page pro
<https://f4se.silverlock.org/> a tlačítko Install the downloaded archive, které
vloží loader do `Root/` módu. `eidos check fallout4` vypíše stejná zjištění,
kromě karty `.pex`, karty Display a karet s požadavky z Nexusu, které počítá jen
okno.

## Když něco vypadá špatně

Obecný kontrolní seznam je v [troubleshooting.md](troubleshooting.md); první
otázka specifická pro Fallout zní vždy *který spustitelný soubor se doopravdy
spustil*. Když okno hru spouští, zapíše celý spouštěcí příkaz do běhového logu
instance, takže:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Pokud jmenuje `f4se_loader.exe`, výměna proběhla. Pokud jmenuje `Fallout4.exe`,
F4SE není nainstalované tam, kde jej Eidos najde - patří vedle spustitelného
souboru hry, což u sestavy spravované módy znamená adresář `Root/` nějakého módu
(nebo samotnou složku hry, instalováno ručně). Pokud jmenuje
`Fallout4Launcher.exe`, Eidos v kořenovém pohledu nenašel ani loader, ani
`Fallout4.exe`.

Když se spustí správný spustitelný soubor a hra přesto padá, Test run a Find the
mod behind a problem zúží příčinu na jeden mód, aniž by sáhly na uložené pozice
profilu ([usage.md](usage.md#testovací-běhy)).
