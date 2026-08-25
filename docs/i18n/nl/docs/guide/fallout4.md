<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 via Eidos

Fallout 4 heeft geen bijzondere opstartoptie nodig, geen hernoemd uitvoerbaar
bestand en geen omhullend script. Dat is het waard om ronduit te zeggen, want elke
andere Linux-handleiding voor F4SE beweert het tegendeel - en hun advies breekt bij
de volgende Steam-update.

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
spelupdate stilletjes herstelt - waarna je zonder F4SE speelt en niets dat zegt.

Eidos doet de wissel zelf, vanuit de speldescriptor: het vervangt de launcher door
`f4se_loader.exe` als er een geïnstalleerd is, valt terug op `Fallout4.exe` als dat
niet zo is, en **zegt het je** wanneer het moest terugvallen. Een spel dat start met
alle F4SE-mods dood is erger dan een spel dat niet start.

Er is een tweede reden om de launcher nooit te draaien: hij doorzoekt `Data` opnieuw
en herschrijft `plugins.txt`, waarmee de zojuist uitgerolde laadvolgorde ongedaan
wordt gemaakt. Eidos voert hem nooit uit.

## Wat Eidos voor je afhandelt

| | |
|---|---|
| Archiefinvalidatie | `Fallout4Custom.ini` krijgt `[Archive]` `bInvalidateOlderFiles=1` en een lege `sResourceDataDirsFinal=`, de twee sleutels die losse bestanden buiten `Data` überhaupt zichtbaar maken. Geschreven in het profiel, niet in de spelmap. |
| Laadvolgorde | `plugins.txt` in het sterretjesformaat dat Fallout 4 gebruikt (`*` markeert actief), met `Fallout4.ccc` gerespecteerd voor de impliciete Creation Club-plugins |
| LOOT | Sorteren werkt hetzelfde als bij Skyrim - `eidos sort <instance>` haalt de `fallout4`-masterlist op |
| Saves | `.fos`-saves en hun `.f4se`-cosaves worden opgesomd, gekopieerd en per profiel bewaard; het detailpaneel leest de eigen plugintabel van de save, dus een save die een door jou uitgeschakelde plugin nodig heeft zegt dat vóór je hem laadt |
| Root-mods | Alles wat een mod naast het uitvoerbare bestand levert (F4SE zelf, ENB, een `dxvk.conf`) belandt daar via hetzelfde `Root/`-mechanisme dat Skyrim gebruikt |

## De versievraag

Fallout 4 is niet langer het bevroren spel dat het tussen 2019 en 2024 was. Per
augustus 2026 zijn er drie levende takken, en een mod-DLL gebouwd voor de ene laadt
niet op de andere:

| Tak | Versie | F4SE |
|---|---|---|
| Klassiek ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Twee gevolgen die je moet kennen voordat je een modlijst bouwt:

- **Controleer wat je werkelijk hebt.** Mappen `Creations/` en `Mods/` in de
  spelmap betekenen dat je op de 1.11.x-lijn zit. Het detailpaneel van een save in
  Eidos toont ook de build die hem schreef - Fallout schrijft dat in de save, en
  Eidos brengt het naar boven als "Game build".
- **Een verse patch is geen goede dag om te beginnen.** F4SE verschijnt meestal
  binnen een dag of twee na een Bethesda-update, maar *Address Library for F4SE
  Plugins* - waarlangs de meeste DLL-mods hun offsets oplossen - volgt zijn eigen
  schema. Daartussenin ligt de DLL-helft van het ecosysteem plat. Mods zonder DLL
  (textures, meshes, plugins) hebben er geen last van.

Zodra je opstelling werkt, zet je Steams automatische updates voor Fallout 4 uit
(Eigenschappen → Updates → "Dit spel alleen bijwerken wanneer ik het start"), anders
breekt de volgende patch elke DLL die je installeerde.

## Hardwarenotitie: wapenpuin crasht op NVIDIA

Het wapenpuineffect van Fallout 4 draait op NVIDIA FleX, een PhysX-afgeleide die
NVIDIA na de Pascal-generatie niet meer ondersteunt. Op elke Turing-kaart of nieuwer
- GTX 16, RTX 20 tot en met RTX 50 - laat het het spel crashen. Dit is een spelfout
en heeft niets met Linux, Proton of Eidos te maken.

Twee oplossingen, beide werken: zet "Weapon Debris" uit in de spelinstellingen, of
installeer *Weapon Debris Crash Fix* (Nexus 48078), dat de botsing van de fragmenten
uitschakelt in plaats van het effect.

## Als er iets fout lijkt

De algemene checklist staat in [troubleshooting.nl.md](troubleshooting.md); de
Fallout-specifieke eerste vraag is altijd *welk uitvoerbaar bestand er werkelijk
startte*. Eidos schrijft de volledige startopdracht in het uitvoerlogboek van de
instantie, dus:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Noemt het `f4se_loader.exe`, dan is de wissel gebeurd. Noemt het
`Fallout4Launcher.exe`, dan staat F4SE niet waar Eidos het kan vinden - het hoort
naast het uitvoerbare bestand van het spel, wat bij een door mods beheerde opstelling
de `Root/`-map van een mod betekent (of de spelmap zelf, met de hand geïnstalleerd).
