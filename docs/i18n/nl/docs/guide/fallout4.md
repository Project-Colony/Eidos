<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 via Eidos

Met Eidos heeft Fallout 4 geen bijzondere opstartoptie nodig, geen hernoemd
uitvoerbaar bestand en geen omhullend script. Het gebruikt dezelfde opstartoptie
als elk ander spel.

## De opstartoptie

```
~/.local/bin/eidos-gui %command%
```

Steams startdoel voor Fallout 4 is `Fallout4Launcher.exe`, nooit `Fallout4.exe`, dus
de script extender aan de praat krijgen is eigenlijk de vraag "hoe laat ik Steam een
ander programma starten". De gebruikelijke antwoorden herschrijven `%command%` in
bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

of kopiëren `f4se_loader.exe` over `Fallout4Launcher.exe` heen, wat Steam bij elke
spelupdate herstelt - waarna je zonder F4SE speelt en niets dat zegt.

Eidos doet de wissel zelf, vanuit de speldescriptor. Wanneer het venster het spel
start, vervangt het `Fallout4Launcher.exe` door `f4se_loader.exe` als de loader in
de spelmap staat, in de `Root/` van een ingeschakelde mod of in `Overwrite/Root/`.
Zonder de loader draait het `Fallout4.exe`, en de statusregel zegt
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
Vindt het geen van beide bestanden, dan draait het de launcher ongewijzigd en zegt
dat ook. Met `eidos play fallout4 -- %command%` als opstartoptie wisselt de CLI
alleen wanneer de loader bestaat, en draait anders `Fallout4Launcher.exe`.

Er is een tweede reden om de launcher te vermijden: hij doorzoekt `Data` opnieuw en
herschrijft `plugins.txt`, waarmee de zojuist uitgerolde laadvolgorde ongedaan wordt
gemaakt.

Open je het venster zelf en druk je op Run, dan vraagt Eidos Steam het spel te
starten (`steam -applaunch 377160`), en Steam start Eidos via de opstartoptie
hierboven. Staat `f4se_loader.exe` niet naast het uitvoerbare bestand van het spel
in de instantie, dan doet het dat niet, en de statusregel vraagt je eerst de script
extender te installeren. Fallout 4 VR werkt hetzelfde, met `Fallout4VRLauncher.exe`
en `f4sevr_loader.exe`.

## Wat Eidos voor je afhandelt

| Onderdeel | Wat Eidos doet |
|---|---|
| Archiefinvalidatie | `Fallout4Custom.ini` krijgt `[Archive]` `bInvalidateOlderFiles=1` en een lege `sResourceDataDirsFinal=`, de twee sleutels die losse bestanden buiten `Data` überhaupt zichtbaar maken. Geschreven in het profiel, niet in de spelmap. |
| INI's | `Fallout4.ini`, `Fallout4Prefs.ini` en `Fallout4Custom.ini` horen bij het profiel. BethINI Pie, via Eidos gedraaid, bewerkt de kopieën van het profiel, en Backups -> INIs zet de kopie terug die Eidos voor elke run bewaart ([tools.md](tools.md#bethini-pie)). |
| Laadvolgorde | `plugins.txt` in het sterretjesformaat dat Fallout 4 gebruikt (`*` markeert actief), met `Fallout4.ccc` gerespecteerd voor de impliciete Creation Club-plugins |
| LOOT | Sorteren werkt hetzelfde als bij Skyrim - `eidos sort <instance>` haalt de `fallout4`-masterlist op |
| Saves | `.fos`-saves en hun `.f4se`-cosaves worden opgesomd, gekopieerd en per profiel bewaard; het detailpaneel leest de eigen plugintabel van de save, dus een save die een door jou uitgeschakelde plugin nodig heeft zegt dat vóór je hem laadt. Een mod uitschakelen of verwijderen waarvan de plugin door een van de drie nieuwste saves van het profiel gebruikt wordt, vraagt eerst om bevestiging. |
| Root-mods | Alles wat een mod naast het uitvoerbare bestand levert (F4SE zelf, ENB, een `dxvk.conf`) belandt daar via hetzelfde `Root/`-mechanisme dat Skyrim gebruikt |

## De versievraag

Fallout 4 is niet langer het bevroren spel dat het tussen 2019 en 2024 was. Per
augustus 2026 zijn er drie levende takken, en een mod-DLL gebouwd voor de ene laadt
niet op de andere:

| Tak | Versie | F4SE |
|---|---|---|
| Klassiek ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 tot 1.11.240 | 0.7.4 tot 0.7.9 |

Twee gevolgen die je moet kennen voordat je een modlijst bouwt:

- Mappen `Creations/` en `Mods/` in de spelmap betekenen dat je op de 1.11.x-lijn
  zit. Het detailpaneel van een save in Eidos toont ook de build die hem schreef:
  Fallout schrijft dat in de save, en Eidos toont het als "Game build".
- F4SE verschijnt meestal binnen een dag of twee na een Bethesda-update, maar
  *Address Library for F4SE Plugins* - waarlangs de meeste DLL-mods hun offsets
  oplossen - volgt zijn eigen schema. Daartussenin laden DLL-mods niet. Mods zonder
  DLL (textures, meshes, plugins) hebben er geen last van, dus een verse patch is een
  slechte dag om een modlijst te beginnen.

Zodra je opstelling werkt, bevries je de spelversie in de instantie: Freeze this
version op de Game files-kaart van het tabblad Diagnostics, of
`eidos stock-game fallout4 freeze` om de grootte te zien en
`eidos stock-game fallout4 freeze --yes` om te kopiëren. Eidos kopieert de
winkelinstallatie naar `<instance>/Stock Game` en draait het spel vanuit die kopie,
nog steeds door Steam gestart. Een latere patch wijzigt de winkelinstallatie en niet
de kopie, dus de DLL's die je installeerde houden hun spelversie. Op btrfs of XFS
deelt de kopie de blokken van de winkel en neemt ze bijna geen ruimte in tot Steam
een bestand wijzigt; op ext4 is het een volledige kopie zo groot als de spelmap.
Eidos weigert zolang Steam het spel als bezig of niet volledig geïnstalleerd meldt.
De details staan in [usage.md](usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie).

Zonder kopie zet je Steams automatische updates voor Fallout 4 uit (Eigenschappen,
Updates, "Dit spel alleen bijwerken wanneer ik het start"), anders breekt de
volgende patch elke DLL die je installeerde.

## Hardwarenotitie: wapenpuin crasht op NVIDIA

Het wapenpuineffect van Fallout 4 draait op NVIDIA FleX, een PhysX-afgeleide die
NVIDIA na de Pascal-generatie niet meer ondersteunt. Op elke Turing-kaart of nieuwer
- GTX 16, RTX 20 tot en met RTX 50 - laat het het spel crashen. Dit is een spelfout
en heeft niets met Linux, Proton of Eidos te maken.

Twee oplossingen, beide werken: zet "Weapon Debris" uit in de spelinstellingen, of
installeer *Weapon Debris Crash Fix* (Nexus 48078), dat de botsing van de fragmenten
uitschakelt in plaats van het effect.

## Health-kaarten voor Fallout 4

Het tabblad Diagnostics toont een Health-kaart voor elk van deze gevallen:

- F4SE-plugins staan in `F4SE/Plugins`, maar `f4se_loader.exe` zit in geen enkele
  ingeschakelde mod en niet in de spelmap. Het spel start zonder die plugins.
- `f4se_loader.exe` is geïnstalleerd, maar de `f4se_<version>.dll` voor de draaiende
  spelversie (`f4se_1_10_984.dll` voor 1.10.984) niet. De loader stopt met een fout;
  dit volgt meestal op een spelupdate.
- Het uitvoerbare bestand van het spel of zijn Steam-build is gewijzigd sinds de
  laatste start via Eidos die zonder fout eindigde. F4SE-plugins gebouwd voor de oude
  versie laden mogelijk niet meer.
- Een mod met hogere prioriteit dan F4SE heeft losse `.pex`-kopieën van de eigen
  scripts van F4SE. Een oude kopie breekt de F4SE-functies die ze declareert, en het
  spel toont geen fout.
- De `Fallout4Custom.ini` van het profiel heeft regels `sTestFile1` tot
  `sTestFile10` in `[General]`. Zolang er een is ingesteld, laadt het spel die
  plugins en negeert het `plugins.txt`. Fix verwijdert ze en bewaart het vorige
  bestand als `Fallout4Custom.ini.pre-fix`.
- Een BA2-archief van een mod bewaart `.wav`- of `.xwm`-audio gecomprimeerd. Het spel
  kan crashen wanneer het dat geluid afspeelt. Verpak het archief met ongecomprimeerde
  audio. Fallout 4 VR krijgt dezelfde kaart.
- Een crashlogger schreef een log tijdens de laatste start: `crash-*.log` van
  Buffout 4 in `Documents/My Games/Fallout4/F4SE`, of een `.log` in
  `F4SE/Crashlogs`. De kaart noemt de uitzondering en de DLL's en plugins in de log
  die door je mods geïnstalleerd zijn.
- De Display-kaart toont de venstergrootte en -modus uit de `Fallout4Prefs.ini` van
  het profiel, en Apply schrijft daar `iSize W`, `iSize H`, `bFull Screen` en
  `bBorderless`.
- Een Nexus-vereiste noemt een DLC. Eidos zoekt een plugin met de naam van de DLC,
  en de DLC-plugins van Fallout 4 hebben andere namen (`DLCCoast.esm` voor Far
  Harbor), dus deze kaart is advies: controleer met de hand en druk dan op Ignore.

De twee loader-kaarten hebben een knop Open the download page voor
<https://f4se.silverlock.org/> en een knop Install the downloaded archive, die de
loader in de `Root/` van de mod zet. `eidos check fallout4` drukt dezelfde
bevindingen af, behalve de `.pex`-kaart, de Display-kaart en de kaarten voor
Nexus-vereisten, die alleen het venster berekent.

## Als er iets fout lijkt

De algemene checklist staat in [troubleshooting.md](troubleshooting.md); de
Fallout-specifieke eerste vraag is altijd *welk uitvoerbaar bestand er werkelijk
startte*. Wanneer het venster het spel start, schrijft het de volledige startopdracht
in het uitvoerlogboek van de instantie, dus:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Noemt het `f4se_loader.exe`, dan is de wissel gebeurd. Noemt het `Fallout4.exe`, dan
staat F4SE niet waar Eidos het kan vinden - het hoort naast het uitvoerbare bestand
van het spel, wat bij een door mods beheerde opstelling de `Root/`-map van een mod
betekent (of de spelmap zelf, met de hand geïnstalleerd). Noemt het
`Fallout4Launcher.exe`, dan vond Eidos in de rootweergave noch de loader noch
`Fallout4.exe`.

Start het juiste uitvoerbare bestand en crasht het spel toch, dan beperken Test run
en Find the mod behind a problem het tot één mod zonder de saves van het profiel aan
te raken ([usage.md](usage.md#testruns)).
