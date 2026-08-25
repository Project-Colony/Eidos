<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Uitbreidingen

Een uitbreiding voegt een item aan Eidos toe zonder deel van Eidos te zijn. Ze is
een TOML-manifest dat een programma noemt, plus hooguit dat programma.

Manifesten staan in `~/.config/Colony/Eidos/addons/`, één `.toml` per uitbreiding.
Open de map via **View -> Extensions -> Open folder** en druk op **Reload** -
geen herstart.

## Waarom er niets in Eidos geladen wordt

Mod Organizer 2 laadt plug-ins als gedeelde bibliotheken en draait die in Python
via Qt. Geen van beide laat zich overzetten. Rust heeft geen stabiele ABI, dus een
gedeelde bibliotheek gebouwd met een andere compiler - of een andere
optimalisatievlag, of een andere featureset van een gedeelde afhankelijkheid - is
ongedefinieerd gedrag en geen versieverschil. En de widgets van Eidos zijn
generiek op compileertijd, dus een bibliotheek zou er niet eens een kunnen bouwen
om terug te geven, zelfs met een stabiele ABI.

Een uitbreiding is dus een programma dat Eidos *uitvoert*. Ze kan het venster niet
laten crashen, geen modlijst beschadigen, en blijft werken over updates van Eidos
heen.

## Een gereedschap

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # weglaten voor elk spel
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Ze verschijnt in **View -> Extensions** met een Run-knop en start losgekoppeld -
Eidos wacht er niet op.

## Een controle

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Ze draait bij elke verversing en drukt één bevinding per regel af:

```
level<TAB>title<TAB>detail
```

waarbij `level` `problem`, `advice` of `ok` is. Het detail is optioneel. Alles wat
niet met een bekend niveau begint wordt genegeerd, zodat voortgangsuitvoer en
verdwaalde waarschuwingen geen regel kunnen opwerpen die op een eigen controle van
Eidos lijkt. Bevindingen verschijnen op het tabblad **Health**, voorafgegaan door
de naam van de uitbreiding.

Een controle krijgt drie seconden. Eén die daaroverheen gaat wordt gestopt en als
probleem tegen zichzelf gemeld - ze draait bij dezelfde verversing die op elke
klik volgt, dus een hangende zou het venster bevriezen.

## Plaatshouders

Zowel `args` als `workdir` vullen deze in:

| Plaatshouder    | Wat het is                                   |
| --------------- | -------------------------------------------- |
| `{instance}`    | de wortel van de instantie                   |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | de naam van het actieve profiel              |
| `{profile_dir}` | de map van het actieve profiel               |
| `{game}`        | de spel-id, bijv. `skyrimse`                 |
| `{game_name}`   | de weergavenaam van het spel                 |
| `{install}`     | de installatiemap van het spel               |
| `{data}`        | de `Data`-map van het spel                   |

Een onbekende plaatshouder blijft precies staan zoals hij geschreven is in plaats
van leeggemaakt te worden, zodat een vergissing zichtbaar faalt en `--out {typo}`
niet in `--out --next-flag` verandert. Een gereedschap draaien waarvan niet alle
plaatshouders op te lossen zijn wordt geweigerd, en Eidos zegt welke ontbreken.

## Wat een uitbreiding niet kan

Ze krijgt waarden en draait; ze kan niet terugbellen naar Eidos, de modlijst niet
wijzigen en niets in het venster tekenen. Dat is opzet. Waarvoor MO2 plug-ins
gebruikt en wat wél naar binnen MOET reiken - spelondersteuning, installers, de
conflictmotor - is hier ingebouwd in plaats van erop geschroefd: een speldefinitie
is haar eigen TOML in `~/.config/Colony/Eidos/games/`, en de FOMOD- en
BAIN-installers zijn ingebouwd.
