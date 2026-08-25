<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Nástroje: xEdit, BodySlide, DynDOLOD, FNIS

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
- **tools folder**, který nastavíte v Settings (Tools -> Tools folder), pro
  adresář sdílený mezi instancemi - `/mnt/Games/Tools` a podobně.

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
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| cokoli jiného | nic |

Takže nástroj zaregistrovaný jako **`BodySlide`** dostane své DirectX DLL; tentýž
spustitelný soubor zaregistrovaný jako **`BS`** nedostane nic a může selhat při
startu s chybou, která o DLL neříká nic. Pojmenovávejte nástroje podle programu.

Seznam je v `default_prereqs` (`crates/eidos-instance/src/tools.rs`) a pole
`Prereqs` v dialogu Executables je editovatelné - detekce je výchozí nastavení,
ne pravidlo.

### Tři druhy prerekvizit

**Úroveň 1 - přibalené DLL** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
je dodává a při spuštění je kopíruje do prefixu. Nic není třeba dělat, žádná síť.

**Úroveň 2 - winetricks verbs** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Ty zapisují klíče registru, GAC a CLR hosty, takže je
nelze zkopírovat jako soubory. **Stahují se od Microsoftu.**

**Úroveň 3 - runtimes** (`dotnet10`). Moderní běhové prostředí .NET je 193
souborů, které žijí ve vlastním adresáři a hledají se přes `DOTNET_ROOT`: nikdy
se neregistrují, do prefixu se vůbec neinstalují, takže je neunese ani jedna z
ostatních úrovní. Eidos si ho stáhne sám, ověří proti kontrolnímu součtu
zabudovanému do binárky a uloží do mezipaměti v
`~/.local/share/Colony/Eidos/runtimes/` - **mimo jakoukoli instanci**, protože
78 MB není zvlášť pro každou hru ani pro každý profil.

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
