<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Uitbreidingen

Een uitbreiding voegt een item aan Eidos toe zonder deel van Eidos te zijn. Ze is
een TOML-manifest dat een programma noemt, plus hooguit dat programma.

Manifesten staan in `~/.config/Colony/Eidos/addons/`, één `.toml` per uitbreiding.
Open de map via View -> Extensions -> Open folder en druk op Reload - geen
herstart.

`id`, `kind` en `exec` zijn verplicht; `name` valt terug op de id. `exec` noemt
het programma. Een naam zonder `/` wordt op `PATH` opgezocht. Ontbreekt het
programma, dan slaat Eidos de uitbreiding over en zegt de rij in View ->
Extensions waarom (`<exec> is not on PATH` of `<exec> is not there`). `games`
beperkt de uitbreiding tot die spel-id's; zonder dat veld geldt ze voor elk spel,
en bij een ander spel staat in haar rij "not for this game".

Een manifest dat Eidos weigert staat bovenaan, met de reden: een bestand dat niet
te lezen is, geen geldige TOML of een ontbrekend verplicht veld, een lege `id` of
een `id` met een spatie, een lege `exec`, een `id` die een eerder manifest al
heeft ingenomen (bestanden worden op naam gelezen), een `kind` die Eidos niet
kent, of een installatie-, voorvertonings- of opgeslagen-spel-soort zonder
`protocol = 1` en bestandsnaam `extensions`.

## Waarom er niets in Eidos geladen wordt

Mod Organizer 2 laadt plug-ins als gedeelde bibliotheken en draait die in Python
via Qt. Geen van beide laat zich overzetten. Rust heeft geen stabiele ABI, dus een
gedeelde bibliotheek gebouwd met een andere compiler - of een andere
optimalisatievlag, of een andere featureset van een gedeelde afhankelijkheid - is
ongedefinieerd gedrag, en niets meldt het als versieverschil. En de widgets van
Eidos zijn generiek op compileertijd, dus een bibliotheek zou er niet eens een
kunnen bouwen om terug te geven, zelfs met een stabiele ABI.

Een uitbreiding is een programma dat Eidos uitvoert met de bestandssysteemrechten van jouw account. Installeer alleen hulpprogramma's die je vertrouwt. Procesisolatie en gecontroleerde antwoorden bieden geen sandbox van het besturingssysteem.

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

Ze verschijnt in View -> Extensions met een Run-knop en start losgekoppeld -
Eidos wacht er niet op, en de statusregel geeft de pid. De uitvoer wordt
weggegooid. Zonder `workdir` start ze in de map waarin Eidos zelf gestart is.

## Een controle

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Ze draait bij elke verversing, in de wortel van de instantie tenzij `workdir`
anders zegt, en drukt één bevinding per regel af:

```
level<TAB>title<TAB>detail
```

waarbij `level` `problem`, `advice` of `ok` is. Het detail is optioneel; een regel
zonder titel wordt weggelaten. Alles wat niet met een bekend niveau begint wordt
genegeerd, zodat voortgangsuitvoer en verdwaalde waarschuwingen geen regel kunnen
opwerpen die op een eigen controle van Eidos lijkt. Bevindingen verschijnen op het
tabblad Diagnostics als `<name> - <title>`, na de prefix- en Nexus-kaarten en voor
de andere Health-kaarten.

Een controle krijgt drie seconden. Eén die daaroverheen gaat wordt gestopt en als
probleem tegen zichzelf gemeld - ze draait bij dezelfde verversing die op elke
klik volgt, dus een hangende zou het venster bevriezen. De klok loopt tot de
controle is afgesloten en haar uitvoerpijpen dicht zijn, dus een achtergrondproces
dat ze nog openhoudt telt mee voor de drie seconden. Een controle die met een fout
stopt en niets afdrukt wordt op dezelfde manier gemeld, met de eerste regel van
haar foutuitvoer, en ook één die op een van beide stromen meer dan 1 MiB afdrukt.
In beide gevallen draait Eidos haar niet opnieuw tot je op Reload drukt, en haar
rij zegt "It will not be retried until you press Reload." Een controle leest niets
van haar standaardinvoer.

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
| `{install}`     | de spelmap die de instantie gebruikt         |
| `{data}`        | de `Data`-map van die map                    |

`{install}` is de winkelinstallatie, of de spelkopie wanneer de instantie er een
gebruikt (een bevroren `Stock Game` of de eigen spelmap van een lijst; zie
[usage.md](usage.md#spelbestanden-de-winkelinstallatie-of-een-kopie)). `{data}` volgt die
keuze.

Een controle krijgt ook `{addon_dir}`, de map met haar manifest, en de
protocol-1-uitbreidingen hieronder ook. Een gereedschap niet.

Een onbekende plaatshouder wordt nooit leeggemaakt, want dat zou `--out {typo}` in
`--out --next-flag` veranderen. Eidos weigert een gereedschap of een controle uit
te voeren waarvan niet alle plaatshouders op te lossen zijn, en zegt welke
ontbreken: in de statusregel bij een gereedschap, in de rij van de controle op het
tabblad Diagnostics bij een controle.

## Wat een uitbreiding niet kan

Een uitbreiding kan Eidos niet terug aanroepen, zichzelf niet vanuit een archief registreren en geen eigen widgets tekenen. Gestructureerde installatieprogramma's met protocol 1 kunnen gecontroleerde bestandsplannen en keuzevragen teruggeven; Eidos beheert de beoordeling, tijdelijke voorbereiding en publicatie. FOMOD en BAIN blijven native installatieprogramma's. Uitbreidingen voor voorvertoningen en informatie over opgeslagen spellen gebruiken dezelfde host met begrensde antwoorden. Zie de [protocolspecificatie en uitvoerbare voorbeelden](../../../../guide/examples/extensions/README.md) voor de prioriteit van overeenkomsten, vastgelegde CLI-antwoorden, limieten en identiteitscontroles bij herhaling.
