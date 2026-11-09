<!-- eidos-i18n: source=docs/guide/tools.md sha=80ee63cfac4035db5e5010e5c389ef6bc6f5a370 -->

# Nástroje: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Nástroj spuštěný skrz Eidos vidí **sloučený pohled**, uvnitř vlastního Proton
prefixu hry. Čte to, co bude číst hra - každý zapnutý mód, v pořadí priority - a
cokoli zapíše, přistane v Overwrite, kde se to jedním kliknutím promění ve
skutečný mód.

## Ty, které si Eidos najde sám

Některé nástroje mají dost jedinečný název na to, aby se daly najít místo
deklarovat, a xEdit je ten zjevný případ: `FO4Edit.exe` pro Fallout 4,
`SSEEdit.exe` pro Skyrim SE, `TES5Edit.exe` pro původní díl a tak dále - spolu s
dvojčetem **QuickAutoClean** ke každému z nich, což je to tlačítko na dirty
edits, na které LOOT pořád upozorňuje. Eidos je hledá podle názvu souboru v:

- instalační složce hry a ve stromech `Root/` zapnutých módů;
- **`mods/` této instance**, kam si nástroje instalují uživatelé MO2;
- **tools folder**, který nastavíte v Preferences (General -> Tools), pro
  adresář sdílený mezi instancemi - `/mnt/Games/Tools` a podobně.

`PGPatcher.exe` se stejnym zpusobem najde u her na Creation Engine a prichazi s argumentem, bez ktereho nemuze pracovat - viz [nize](#proc-pgpatcher-potrebuje---ignore-mo2vfscheck).

Seznam je zvlášť pro každou hru, takže instanci Skyrimu se nikdy nenabídne
editor Falloutu. Hledání se zastaví čtyři úrovně hluboko, protože pool módů jsou
statisíce souborů a tohle běží pokaždé, když se sestavuje seznam nástrojů, a
nenásleduje symlinky. Nástroj nalezený tímto způsobem je nastavený přesně jako
ten, který jste zadali sami: jeho runtimes plynou z jeho názvu, podle stejného
pravidla jako všechno níže.

Pokud je nástroj někde jinde nebo chcete jiné argumenty, přidejte ho ručně -
uživatelská položka se stejným názvem přebije cokoli nalezeného automaticky.

## Přidání nástroje

V GUI: **Tools -> Executables**, pak Add. Z příkazové řádky:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # vypsat, co je zaregistrováno
eidos tool skyrimse run BodySlide         # spustit ho skrz sloučený pohled
eidos tool skyrimse run BodySlide --print # ukázat příkaz bez spuštění
```

Script extender, binárka hry a launcher se detekují automaticky; registrovat je
potřeba jen nástroje navíc.

### Nasměrujte ho na skutečný soubor, ať je kdekoli

Zaregistrujte spustitelný soubor tam, kde doopravdy leží. Pokud byl nástroj
nainstalovaný jako mód, je to uvnitř složky módu:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(to je cesta globální instance - u přenosné instance platí totéž pravidlo pod
její vlastní složkou, `<instance>/mods/...`; pozor, absolutní cesta jako tahle je
jediná věc, která nepřežije pozdější PŘESUN přenosné složky).

Eidos tuhle cestu před spuštěním přepíše na tu sloučenou, takže nástroj běží z
`<game>/Data/CalienteTools/BodySlide/` a vidí tam i soubory všech ostatních módů.
Záleží na tom víc, než to zní: BodySlide přináší **prázdný** adresář
`SliderSets` a každé tělo, které umí postavit, pochází z CBBE a z módů s
oblečením. Spuštěn ze své vlastní složky módu nenajde nic a vypadá rozbitě.

MO2 dělá totéž přepisování, ze stejného důvodu - jeho vlastní komentář jmenuje
FNIS.

Nástroj ve **vypnutém** módu přepsat nelze, protože jeho soubory nejsou ani v
pohledu. Eidos to řekne a spustí ho z jeho vlastní složky, místo aby předstíral.

## Posílání výstupu nástroje do jeho vlastního módu

Generátor - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - zapíše stovky
souborů. Ve výchozím stavu přistanou v Overwrite se vším ostatním. Nastavte
**Capture output into** v editoru Executables a výstup tohoto běhu půjde místo
toho do zvoleného módu:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Mód se vytvoří, pokud neexistuje. Přesunou se jen soubory, které vyrobil TENTO
běh; cokoli už v Overwrite bylo, tam zůstane, takže dva nástroje s cílem pro
zachytávání si výstup navzájem nekradou. Běh, který nic nezapsal, po sobě
nenechá prázdný mód.

Děje se to až po běhu, ne nasměrováním zapisovací vrstvy na mód, jak to dělá MO2.
Nasměrování zapisovací vrstvy na mód by ho na celý běh povýšilo na nejvyšší
prioritu - převrátilo by každý konflikt, ve kterém je, a potom je převrátilo
zpět - a zapisovalo by rovnou skrz vlastní soubory módu bez copy-up. Zachytávání
dosáhne stejného koncového stavu bez jednoho i druhého.

Pokud je cílový mód vypnutý, výstup se zapíše, ale hra ho neuvidí, takže by
nástroj při dalším běhu vygeneroval tytéž soubory znovu. Eidos na to upozorní.

## DLL, které nástroj potřebuje, se vybírají podle jeho NÁZVU

Tohle je ta překvapivá část, takže stojí za to říct ji na rovinu: **název, který
nástroji dáte, rozhoduje o tom, jaké běhové prerekvizity mu Eidos zajistí.**
Porovnává se podřetězec názvu bez ohledu na velikost písmen.

| Pokud název obsahuje | Eidos vyžádá |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| cokoli jiného | nic |

Takže nástroj zaregistrovaný jako **`BodySlide`** dostane své DirectX DLL; tentýž
spustitelný soubor zaregistrovaný jako **`BS`** nedostane nic a může selhat při
startu s chybou, která o DLL neříká nic. Pojmenovávejte nástroje podle programu.

Seznam je v `default_prereqs` (`crates/eidos-instance/src/tools.rs`) a pole
`Prereqs` v dialogu Executables je editovatelné - detekce je výchozí nastavení,
ne pravidlo.

Nazev rozhoduje o **argumentech** stejne, u jedineho nastroje, ktery nejaky
potrebuje a nikdo by jej neuhodl: nazev obsahujici `pgpatcher` (nebo
`parallaxgen`, jeho drivejsi jmeno) dostane `--ignore-mo2vfscheck`. Proc a proc
neni volitelny, [nize](#proc-pgpatcher-potrebuje---ignore-mo2vfscheck).

### Tři druhy prerekvizit

**Úroveň 1 - přibalené DLL** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos je dodává a při spuštění je kopíruje do prefixu.
Nic není třeba dělat, žádná síť.

**Úroveň 2 - winetricks verbs** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Ty zapisují klíče registru, GAC a CLR hosty, takže je
nelze zkopírovat jako soubory. **Stahují se od Microsoftu.**

**Úroveň 3 - runtimes** (`dotnet10`, `dotnetsdk10`). Moderní běhové prostředí .NET je 193
souborů, které žijí ve vlastním adresáři a hledají se přes `DOTNET_ROOT`: nikdy
se neregistrují, do prefixu se vůbec neinstalují, takže je neunese ani jedna z
ostatních úrovní. Eidos si ho stáhne sám, ověří proti kontrolnímu součtu
zabudovanému do binárky a uloží do mezipaměti v
`~/.local/share/Colony/Eidos/runtimes/` - **mimo jakoukoli instanci**, protože
78 MB není zvlášť pro každou hru ani pro každý profil.

`dotnetsdk10` je .NET 10 SDK: 301 MB ke stažení, po rozbalení 808 MB. Synthesis
ho potřebuje ke kompilaci svých patcherů a k jejich sestavení a spuštění volá
holé `dotnet`, takže nástroj, který deklaruje `dotnetsdk10`, dostane navíc
složku SDK v `WINEPATH` a k tomu `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` a `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
Záznam Synthesis v `tools.ini`, který má pořád starou výchozí hodnotu `dotnet8`,
`vcrun2022`, dostane při načtení Eidosem `dotnetsdk10`, `vcrun2022`.

NuGet ověřuje podpis každého balíčku, který obnovuje, proti úložišti kořenových
certifikátů Windows, a v prefixu Wine chybí kořenové certifikáty pro
podepisování kódu a časová razítka, které SDK dodává jako
`sdk/10.0.403/trustedroots/codesignctl.pem` a `timestampctl.pem`. Bez nich
obnova `Mutagen.Bethesda.Synthesis` 0.36.6 selže s chybami `NU3018` a `NU3028`.
Proto když nástroj deklaruje `dotnetsdk10` a SDK je stažené, Eidos před jeho
spuštěním přidá 385 certifikátů z obou souborů do úložiště kořenových
certifikátů aktuálního uživatele v prefixu hry, a to ve stejném importu přes
`regedit`, který zapisuje cestu ke hře (viz níže). Pokud některý z nich zmizí,
importuje je znovu. Těmto kořenovým certifikátům pak důvěřuje každý program pro
Windows v tomto prefixu. S nimi v novém prefixu a s prázdnou mezipamětí NuGet
stejná obnova i sestavení projdou se systémovým Wine 11.21 i s GE-Proton11-9.

Nic z úrovně 2 ani 3 neběží potichu:

```sh
eidos prereqs skyrimse            # ukázat, co registrované nástroje potřebují, a jejich stav
eidos prereqs skyrimse --install  # stáhnout, co chybí (stahování)
```

V GUI sedí tytéž stavy pod polem Prereqs a chybějící jsou tlačítka. Verb, který
není ani přibalený, ani runtime, ani známý winetricks verb, je nahlášen jako
pravděpodobný překlep, místo aby byl nabídnut ke stažení.

### Proč DynDOLOD potřebuje `dotnet10`

DynDOLOD nestaví object LOD sám: volá LODGen a přináší tři jeho verze.
`LODGenx64.exe` cílí na .NET Framework 4.8, který je pod Protonem směrován na
Wine Mono - jehož inicializátor `System.Uri` volá metodu, kterou Mono
neimplementuje. Umře dřív, než odvede první kus práce, a nechá po sobě log s
hlavičkou verze a ničím dalším a dialog DynDOLODu, který říká jen „failed for
one or more worlds".

Instalace skutečného .NET Frameworku to nespraví: Proton nahradí `mscoree.dll` -
zavaděč, který by ho našel - symlinkem do vlastního stromu, a dělá to znovu při
každé aktualizaci prefixu.

Verze, která funguje, je `LODGenx64Win10.exe`, která cílí na moderní .NET a
`mscoree` se vůbec nedotkne. Nasměrujte `DOTNET_ROOT` na runtime .NET 10 a
poběží. To je to, co `dotnet10` zajistí, a Eidos tu proměnnou nastavuje při
spouštění každého nástroje, který ji deklaruje.

Eidos spouští systémový `winetricks` proti vlastnímu `wine` Protonu a prefixu
hry, čímž obchází kontejner pressure-vessel Steamu a nesoulad protontricks +
Proton-GE. Nástroj, který deklaruje nenainstalovaný verb úrovně 2, se přesto
spustí, s varováním, které jmenuje verb i příkaz na nápravu - uživatel ho může
mít odjinud.

### Proc PGPatcher potrebuje `--ignore-mo2vfscheck`

PGPatcher prepisuje site a pluginy tak, aby vase modifikace textur - vanilla,
parallax, complex material, PBR - pouzivaly na kazdem povrchu spravny shader.
Pravem odmita bezet mimo spravce modu: zaplatovat holou slozku hry by bylo
zhoubne.

Potiz je v tom, JAK to overuje: hleda **usvfs**, vrstvu zachytavani DLL, kterou
MO2 ve Windows vstrikuje do hry. Eidos zadnou nema a nikdy mit nebude - slouceny
pohled je pripojeni FUSE, do ktereho Wine vstupuje, aniz by o nem vedel - takze
kontrola nic nenajde a nastroj okamzite skonci s

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

a jmenuje program, ktery uzivatel vubec nespousti, jeste pred jakoukoli praci.
Odpovedi samotneho autora na dve linuxova hlaseni ([#716][pg716] a [#725][pg725],
obe z Fluorine, linuxoveho forku MO2, ktery ze stejneho duvodu stoji na FUSE) je
prepinac, ktery kontrolu preskoci.

Eidos jej proto predvyplni. PGPatcher nalezeny ve fondu modu prichazi s uz
nastavenym argumentem a napsani `PGPatcher` do nazvu v dialogu Executables
vyplni pole Arguments stejne. To, co napisete, vzdy vyhrava: predvyplni se jen
pole, ktere jste nechali prazdne, presne jako u pozadavku vyse.

Seznam je v `default_args` (`crates/eidos-instance/src/tools.rs`), vedle
`default_prereqs`. Ma jednu polozku a jedna staci na vlastni funkci: prepinac
nikdo neuhodne a chyba, ktere predchazi, jmenuje spatny program.

Za behu nevyzaduje nic, coz je u nastroje .NET neobvykle: nese si vlastni
prostredi (`dotnetlib/`, s `runtimeconfig.json` deklarujicim
`includedFrameworks`) a dodava knihovny CRT od Microsoftu vedle spustitelneho
souboru. Nechte Prereqs prazdne.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher a PGLite v režimu Mod Organizer 2

S Mod Manager nastaveným na `None` vidí PGPatcher jedinou sloučenou složku
`Data` a nepozná, ze kterého modu pochází mesh nebo textura, takže nemá žádné
konflikty k zobrazení ani pořadí modů, kterým by se řídil. Jeho režim
`Mod Organizer 2` čte samotnou instanci: soubor `modlist.txt` profilu, zapnuté
mody v pořadí priority a soubory ve složce každého modu. PGLite, verze běžící ve
hře, čte instanci stejně. Instance Eidos už má rozložení MO2 (`mods/`,
`profiles/<name>/modlist.txt`, `overwrite/`), takže tento režim s ní funguje.

V launcheru PGPatcheru:

1. Nastavte **Mod Manager** na `Mod Organizer 2`.
2. Nastavte **MO2 Options -> Instance Location** na složku instance Eidos,
   zapsanou jako cesta Windows na disku `Z:` ve Wine, kterým je `/`. Přenosná
   instance v `/mnt/Games/Eidos-Skyrim` je `Z:\mnt\Games\Eidos-Skyrim`; globální
   je `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Ponechte `--ignore-mo2vfscheck` v argumentech nástroje. V tomto režimu
   PGPatcher hledá usvfs a skončí, když chybí.

PGPatcher přijme složku jen tehdy, když obsahuje `modorganizer.ini`, a proto
Eidos zapisuje `ModOrganizer.ini` do kořene instance při každém spuštění hry
nebo nástroje, pro spouštěný profil. Soubor se přepisuje jen tehdy, když se
změní jeho obsah. Obsahuje:

- v `[General]`: `gameName` (název hry podle MO2, tedy `New Vegas` pro Fallout: New Vegas), `gamePath` (instalační složku hry jako cestu `Z:`)
  a `selected_profile`;
- v `[Settings]`: `mod_directory`, `profiles_directory` a
  `overwrite_directory` jako absolutní cesty `Z:`, a navíc `download_directory`,
  když má instance složku `downloads/`.

Jeho první řádek je komentář, který říká, že soubor zapsal Eidos. Soubor
`ModOrganizer.ini` bez tohoto řádku, v jakékoli velikosti písmen v názvu, se
nikdy nemění ani nemaže: instance MO2, kterou jste otevřeli v Eidos, si
ponechá svůj a PGPatcher čte tento soubor tak, jak je. MO2 při ukládání
nastavení komentáře zahazuje, takže soubor, přes který MO2 uložil, od té chvíle
patří MO2. Prázdný soubor se počítá jako soubor Eidos.

Takový soubor uvádí profil, který MO2 vybral naposledy, a PGPatcher použije
tento profil, ne ten, který spouští Eidos. Když se liší, Eidos při spuštění
zapíše do protokolu varování. Vyberte v MO2 stejný profil, nebo soubor
přejmenujte či odstraňte, aby Eidos zapsal vlastní.

Eidos zapisuje svůj soubor jen při spuštění něčeho, takže mezi dvěma spuštěními
uvádí profil toho posledního. Nástroj, který spustí Eidos, v něm najde svůj
vlastní profil.

PGPatcher skončí s chybou, když profil nemá žádný zapnutý mod, a když je jeho
výstupní složka složkou zapnutého modu, stejně jako pod MO2.

## Cesta ke hře v prefixu

Windowsové nástroje najdou svou hru přečtením
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, klíče, který zapisuje
vlastní instalátor hry - a který Steam pod Protonem nikdy nespustí. Bez něj se
xEdit, Wrye Bash a DynDOLOD otevřou na prázdné cestě. Eidos ho zapíše před
spuštěním nástroje: idempotentně, přírůstkově a přeskočí to, pokud prefix není
inicializovaný nebo se právě používá.

## Jak se k nástroji dostat: skrýt, připnout a zástupce na ploše

Ve výchozím vybavení hry jsou nástroje, které možná nikdy nepoužijete, a výběr,
kde je osm položek, aby se došlo k druhé, je výběr, který nikdo nečte. V dialogu
Executables:

- **Pin to top** postaví položku na začátek seznamu Run.
- **Hide from picker** ji z něj vyjme, aniž by ji smazal.
- **Desktop shortcut** zapíše `.desktop` do
  `~/.local/share/applications` - kam spouštěč na freedesktopovém systému patří,
  takže se objeví v nabídce aplikací a ve vyhledávání, ne na ploše. Spouští
  přímo `eidos tool <instance> run <title>`, což znamená, že nástroj naběhne
  **skrz sloučený pohled s profilem této instance**, aniž by okno Eidosu bylo
  vůbec otevřené.

Skrývání a připínání se týkají toho, jak se k nástroji *dostanete*, ne toho, co
spouští, takže platí i pro výchozí nástroje dané hry, nejen pro vaše vlastní
položky.

## Nástroj, který je vlastní aplikací na Steamu

Creation Kit je samostatná aplikace na Steamu a chce vlastní AppID; pár dalších
moddingových nástrojů dodávaných přes Steam je na tom stejně. Nastavte na
položce **Steam AppID** a Eidos ji spustí pod tímto id místo id hry.

Na Windows to znamená jiný launcher. Tady jsou to dvě proměnné prostředí u běhu,
který se stejně už stavěl - `SteamAppId` a `SteamGameId`, obě, protože Proton
čte jednu a vlastní knihovny Steamu druhou, a nástroj, který je vidí
nesouhlasit, selže podivně místo jasně. `eidos tool ... --print` ukáže přesně to,
co by skutečný běh dostal.

## Proměnné prostředí a nástroje spuštěné bez Protonu

Tři klíče v `<instance>/tools.ini` zatím nemají v dialogu Executables žádné
pole, takže je do sekce nástroje dopište ručně. Každý řádek `env=NAME=value`
nastaví jednu proměnnou jen pro běhy tohoto nástroje:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos tyto řádky použije až po všem, co nastavuje sám, takže hodnota zadaná tady
nahradí tu jeho. To platí i pro `WINEDLLOVERRIDES`, který Eidos skládá z DLL
dodaných vašimi módy: nástroj, který ho nastaví, v něm musí uvést každou DLL,
kterou potřebuje. Jméno tvoří písmena, číslice a `_` a nezačíná číslicí. Řádek
s jiným jménem se ignoruje.

`runner=native` spustí přímo `exe`, bez Protonu. Sloučený pohled se připojí jako
u každého nástroje a řádky `arg0=`, `arg1=` jsou jeho argumenty. Použijte to pro
linuxový program nebo pro nástroj ke kopii, kterou nespravuje Steam a pro kterou
Heroic nemá zapsaný žádný runner: `exe` je Wine této kopie, program pro Windows
je první argument a prefix přijde z `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` musí být absolutní cesta. Pracovní adresář je ve výchozím stavu složka, ve
které leží `exe`, tady tedy složka runneru, a proto nastavte `workdir=`. Na
sloučenou cestu se přepisuje jen `exe`: u programu pro Windows uvnitř módu
zadejte jeho cestu ve složce `Data` hry, kde ho ukazuje sloučený pohled. Nativní
nástroj vynechá kroky pro Proton: klíč registru popsaný výše, shader cache
Steamu, prázdné kořeny .NET a odstranění overlaye Steamu z `LD_PRELOAD`.

U kopie z GOG nebo Epicu nainstalované přes Heroic takový záznam nástroje
nepotřebují, pokud nastavení hry v Heroicu (`GamesConfig/<appName>.json`) uvádí
build Wine nebo Protonu: Eidos je spustí s tímto buildem v prefixu hry
z Heroicu a `eidos prereqs <instance> --install` ho předá winetricks. Kopie
nainstalovaná jen přes Legendary žádná nastavení Heroicu nemá.

`slr=1` spustí nástroj i import do registru před ním uvnitř kontejneru Steam
Linux Runtime, který jmenuje Proton nástroje (`require_tool_appid` v jeho
`toolmanifest.vdf`), stejně jako Steam spouští hru. Tento runtime musí být
nainstalovaný v některé knihovně Steamu; pokud není, běh se zastaví a uvede
aplikaci Steamu, kterou je třeba nainstalovat. `eidos prereqs` dál spouští
winetricks na hostiteli a nástroj s `runner=native` tento klíč ignoruje.
Kontejner vidí vaši domovskou složku a knihovnu Steamu se hrou. Nástroj, mod
nebo runtime kdekoli jinde, například na jiném disku, potřebuje
`env=STEAM_COMPAT_MOUNTS=/ta/cesta` (více cest oddělte `:`). Proton
nainstalovaný v `/usr/share/steam/compatibilitytools.d` s `slr=1` nejspíš použít
nepůjde, protože `/usr` uvnitř kontejneru patří runtimu. Běh nástroje uvnitř
kontejneru zatím nebyl testován.

`eidos tool <instance> run <title> --print` ukáže výsledný příkaz a každou
proměnnou, kterou běh nastaví nad zděděné prostředí, včetně těchto řádků.

## Vlastní nastavení nástroje zůstává jeho

Eidos postaví nástroj na správné místo se správnými DLL. Co pak nástroj udělá se
svou konfigurací, je věc mezi vámi a nástrojem, a selhání bývá tiché.

Ukázkový případ, protože jinak stojí hodinu: **Game Data Path** BodySlidu
(Settings) musí ukazovat na adresář `Data` hry, ne na složku hry nad ním.
Nastavený o úroveň výš hlásí dávkový build „All sets processed successfully" a
zapíše 1439 meshů tam, kde je hra nikdy hledat nebude. Eidos je zachytí -
přistanou v `Overwrite/Root/` místo ve vaší instalaci - ale z pohledu hry není
nic špatně, jen vaše těla nejsou postavená.

Výstup nástroje patří do Overwrite. Když běh vyprodukuje něco, co stojí za
uchování, **Overwrite -> Create mod...** z toho udělá běžný mód, který se dá
řadit, vypnout a odebrat jako každý jiný.
