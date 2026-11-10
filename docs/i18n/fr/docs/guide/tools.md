<!-- eidos-i18n: source=docs/guide/tools.md sha=bc1c18f7029d1b12bcae46d861626848edfafb5a -->

# Outils : xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Un outil lancé à travers Eidos voit **la vue fusionnée**, à l'intérieur du
préfixe Proton du jeu lui-même. Il lit ce que le jeu lira - tous les mods
activés, dans l'ordre de priorité - et tout ce qu'il écrit atterrit dans
l'Overwrite, où un clic en fait un vrai mod.

## Ceux qu'Eidos trouve tout seul

Certains outils portent un nom assez unique pour être trouvés plutôt que
déclarés, et xEdit est le cas évident : `FO4Edit.exe` pour Fallout 4,
`SSEEdit.exe` pour Skyrim SE, `TES5Edit.exe` pour l'original, et ainsi de suite -
avec pour chacun son jumeau **QuickAutoClean**, qui est le bouton pour les dirty
edits dont LOOT n'arrête pas de prévenir. Eidos les cherche, par nom de fichier,
dans :

- le dossier d'installation du jeu, et les arborescences `Root/` des mods
  activés ;
- **le `mods/` de cette instance**, là où les utilisateurs de MO2 installent
  leurs outils ;
- le **tools folder** que vous avez défini dans Preferences (General -> Tools),
  pour le répertoire partagé entre instances - `/mnt/Games/Tools` et
  consorts.

`PGPatcher.exe` est trouvé de la même manière sur les jeux Creation Engine, et il arrive avec un argument sans lequel il ne peut pas fonctionner - voir [plus bas](#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck).

La liste est par jeu, donc une instance Skyrim ne se verra jamais proposer
l'éditeur de Fallout. La recherche s'arrête à quatre niveaux de profondeur, parce
qu'un pool de mods représente des centaines de milliers de fichiers et que cela
s'exécute à chaque construction de la liste d'outils, et elle ne suit pas les
liens symboliques. Un outil trouvé ainsi est configuré exactement comme un outil
que vous auriez saisi : ses runtimes viennent de son nom, selon la même règle que
tout ce qui suit.

Si un outil est ailleurs, ou si vous voulez d'autres arguments, ajoutez-le à la
main - une entrée utilisateur portant le même titre l'emporte sur tout ce qui a
été trouvé automatiquement.

## En ajouter un

Dans l'interface : **Tools -> Executables**, puis Add. Depuis la ligne de
commande :

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # lister ce qui est enregistré
eidos tool skyrimse run BodySlide         # le lancer à travers la vue fusionnée
eidos tool skyrimse run BodySlide --print # afficher la commande sans l'exécuter
```

Le script extender, le binaire du jeu et le launcher sont détectés
automatiquement ; seuls les outils supplémentaires ont besoin d'être enregistrés.

### Pointez-le sur le vrai fichier, où qu'il soit

Enregistrez l'exécutable là où il se trouve réellement. Si l'outil a été installé
comme un mod, c'est à l'intérieur du dossier du mod :

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(c'est le chemin de l'instance globale - pour une instance portable la même règle
s'applique sous son propre dossier, `<instance>/mods/...` ; notez qu'un chemin
absolu comme celui-ci est la seule chose qui ne survit pas au DÉPLACEMENT
ultérieur d'un dossier portable).

Eidos réécrit ce chemin vers le chemin fusionné avant le lancement, si bien que
l'outil s'exécute depuis `<game>/Data/CalienteTools/BodySlide/` et y voit aussi
les fichiers de tous les autres mods. Cela compte plus qu'il n'y paraît :
BodySlide livre un répertoire `SliderSets` **vide**, et tous les corps qu'il sait
construire viennent de CBBE et des mods de tenues. Lancé depuis son propre
dossier de mod, il ne trouve rien et paraît cassé.

MO2 fait la même réécriture, pour la même raison - son propre commentaire cite
FNIS.

Un outil dans un mod **désactivé** ne peut pas être réécrit, parce que ses
fichiers ne sont pas dans la vue non plus. Eidos le dit et le lance depuis son
propre dossier plutôt que de faire semblant.

## Envoyer la sortie d'un outil dans son propre mod

Un générateur - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - écrit des
centaines de fichiers. Par défaut ils atterrissent dans l'Overwrite avec tout le
reste. Définissez **Capture output into** dans l'éditeur Executables et la sortie
de cette exécution ira dans ce mod-là :

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Le mod est créé s'il n'existe pas. Seuls les fichiers produits par CETTE
exécution sont déplacés ; tout ce qui était déjà dans l'Overwrite y reste, si
bien que deux outils avec des cibles de capture ne se volent pas mutuellement
leur sortie. Une exécution qui n'a rien écrit ne laisse pas de mod vide derrière
elle.

C'est fait après l'exécution plutôt qu'en pointant la couche d'écriture sur le
mod, ce que fait MO2. Pointer la couche d'écriture sur un mod le promouvrait en
priorité maximale pour toute l'exécution - inversant tous les conflits dans
lesquels il est impliqué, puis les inversant à nouveau ensuite - et écrirait
directement à travers les fichiers du mod lui-même, sans copy-up. La capture
atteint le même état final sans ni l'un ni l'autre.

Si le mod cible est désactivé, la sortie est quand même écrite mais le jeu ne la
verra pas, si bien que l'outil régénérerait les mêmes fichiers à l'exécution
suivante. Eidos prévient quand c'est le cas.

### Quand sa sortie n'est plus à jour

Eidos note à partir de quels mods, de quel ordre des plugins, de quel profil et de quel exécutable la sortie d'un outil a été produite. Quand l'un d'eux change, l'onglet Diagnostics le signale, et la carte propose un bouton **Run <tool> again** qui relance l'outil avec sa configuration actuelle.

Pour un générateur dont la sortie périmée casse le jeu, comme Pandora ou Nemesis, ajoutez `check_before_play=1` à son entrée dans `tools.ini`, ou activez **Demander avant de jouer si sa sortie n'est plus à jour** dans la fenêtre Exécutables. Lancer le jeu demande alors d'abord : lancer l'outil, ou jouer quand même. Le choix de jouer quand même est retenu jusqu'à ce que les mods, les plugins ou le profil changent de nouveau, ou que la fenêtre se ferme. Eidos ne lance jamais l'outil de lui-même.

## Les DLL dont un outil a besoin sont choisies par son NOM

C'est la partie surprenante, alors autant le dire clairement : **le titre que
vous donnez à un outil décide des prérequis runtime qu'Eidos lui provisionne.**
La correspondance est une sous-chaîne du titre, insensible à la casse.

| Si le titre contient | Eidos demande |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| n'importe quoi d'autre | rien |

Ainsi un outil enregistré sous **`BodySlide`** reçoit ses DLL DirectX ; le même
exécutable enregistré sous **`BS`** ne reçoit rien et peut échouer au démarrage
avec une erreur qui ne dit rien des DLL. Nommez les outils d'après le programme.

La liste est dans `default_prereqs` (`crates/eidos-instance/src/tools.rs`), et le
champ `Prereqs` de la boîte de dialogue Executables est modifiable - la détection
est un défaut, pas une règle.

Le titre décide des **arguments** de la même manière, pour le seul outil qui en
exige un que personne ne pourrait deviner : un titre contenant `pgpatcher` (ou
`parallaxgen`, son ancien nom) reçoit `--ignore-mo2vfscheck`. Pourquoi, et
pourquoi ce n'est pas optionnel, [plus bas](#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck).

### Trois sortes de prérequis

**Palier 1 - DLL fournies** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos les embarque et les copie dans le préfixe au lancement. Rien à faire, pas de
réseau.

**Palier 2 - verbes winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Ceux-là écrivent des clés de registre, le GAC et les
hôtes CLR, ils ne peuvent donc pas être copiés comme des fichiers. Ils
**téléchargent depuis Microsoft**.

**Palier 3 - runtimes** (`dotnet10`, `dotnetsdk10`). Un runtime .NET moderne, c'est 193 fichiers
qui vivent dans leur propre répertoire et sont trouvés via `DOTNET_ROOT` : jamais
enregistré, jamais installé dans le préfixe du tout, si bien qu'aucun des deux
autres paliers ne peut le porter. Eidos le télécharge lui-même, le vérifie contre
une somme de contrôle intégrée au binaire, et le met en cache dans
`~/.local/share/Colony/Eidos/runtimes/` - **hors de toute instance**, parce que
78 Mo, ce n'est ni par jeu ni par profil.

`dotnetsdk10` est le SDK .NET 10 : 301 Mo à télécharger, 808 Mo une fois
décompressé. Synthesis en a besoin pour compiler ses patchers, et il appelle un
simple `dotnet` pour les construire et les lancer. Un outil qui déclare
`dotnetsdk10` reçoit donc aussi le dossier du SDK dans `WINEPATH`, ainsi que
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`
et `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
La compilation d'un patcher laisserait tourner des nœuds MSBuild et le serveur
VBCSCompiler, qui sous Wine maintiennent en vie le wineserver du préfixe, donc
l'outil reçoit aussi `MSBUILDDISABLENODEREUSE=1`,
`DOTNET_CLI_USE_MSBUILD_SERVER=0` et `UseSharedCompilation=false`.
`DOTNET_ROLL_FORWARD=Major` permet à un patcher compilé pour .NET 8 (Mutagen
avant 0.54) de tourner sur le runtime .NET 10 du SDK quand aucun runtime .NET 8
n'est trouvé.
Une entrée Synthesis de `tools.ini` qui
porte encore l'ancienne valeur par défaut, `dotnet8`, `vcrun2022`, passe à
`dotnetsdk10`, `vcrun2022` quand Eidos la lit.

NuGet vérifie la signature de chaque paquet qu'il restaure auprès du magasin de
racines de Windows, et il manque à un préfixe Wine les racines de signature de
code et d'horodatage que le SDK fournit sous
`sdk/10.0.403/trustedroots/codesignctl.pem` et `timestampctl.pem`. Sans elles,
la restauration de `Mutagen.Bethesda.Synthesis` 0.36.6 échoue avec `NU3018` et
`NU3028`. Avant de lancer un outil qui déclare `dotnetsdk10`, une fois le SDK
téléchargé, Eidos ajoute donc les 385 certificats des deux fichiers au magasin
de racines de l'utilisateur courant dans le préfixe du jeu, dans le même import
`regedit` que celui qui écrit le chemin du jeu (voir plus bas). Si l'un d'eux
disparaît, il les importe de nouveau. Tous les programmes Windows de ce préfixe
font alors confiance à ces racines. Avec elles, dans un préfixe neuf et avec un
cache NuGet vide, la même restauration et la compilation réussissent avec le
Wine 11.21 du système et avec GE-Proton11-9.

Rien dans les paliers 2 ou 3 ne s'exécute en silence :

```sh
eidos prereqs skyrimse            # afficher ce dont les outils enregistrés ont besoin, et leur état
eidos prereqs skyrimse --install  # récupérer ce qui manque (téléchargements)
```

Dans l'interface, les mêmes états se trouvent sous le champ Prereqs, et ceux qui
manquent sont des boutons. Un verbe qui n'est ni fourni, ni un runtime, ni un
verbe winetricks connu est signalé comme une faute de frappe probable plutôt que
proposé au téléchargement.

### Pourquoi DynDOLOD a besoin de `dotnet10`

DynDOLOD ne construit pas le LOD des objets lui-même : il délègue à LODGen, et il
en livre trois. `LODGenx64.exe` cible .NET Framework 4.8, qui sous Proton est
routé vers le Mono de Wine - dont l'initialiseur de `System.Uri` appelle une
méthode que Mono n'implémente pas. Il meurt avant sa première ligne de travail,
laissant un log qui contient une bannière de version et rien d'autre, et une
boîte de dialogue DynDOLOD qui dit seulement « failed for one or more worlds ».

Installer le vrai .NET Framework n'y change rien : Proton remplace `mscoree.dll`
- le loader qui l'aurait trouvé - par un lien symbolique vers sa propre
arborescence, et refait cela à chaque mise à jour du préfixe.

La build qui marche, c'est `LODGenx64Win10.exe`, qui cible le .NET moderne et ne
touche jamais à `mscoree`. Pointez `DOTNET_ROOT` sur un runtime .NET 10 et il
s'exécute. C'est ce que provisionne `dotnet10`, et Eidos définit la variable au
lancement de tout outil qui le déclare.

Eidos exécute le `winetricks` du système contre le `wine` de Proton lui-même et
le préfixe du jeu, ce qui contourne le conteneur pressure-vessel de Steam et le
décalage protontricks + Proton-GE. Un outil qui déclare un verbe de palier 2 non
installé se lance quand même, avec un avertissement nommant le verbe et la
commande pour y remédier - l'utilisateur l'a peut-être obtenu par ailleurs.

### Pourquoi PGPatcher a besoin de `--ignore-mo2vfscheck`

PGPatcher réécrit les meshes et les plugins pour que les mods de textures que
vous avez - vanilla, parallax, complex material, PBR - utilisent le bon shader
sur chaque surface. Il refuse de tourner hors d'un gestionnaire de mods, à
raison : patcher un dossier de jeu nu serait destructeur.

Le problème est la MANIÈRE dont il le vérifie : il cherche **usvfs**, la couche
d'accrochage de DLL que MO2 injecte dans le jeu sous Windows. Eidos n'en a pas
et n'en aura jamais - la vue fusionnée est un montage FUSE que Wine traverse
sans savoir qu'il existe - donc la vérification ne trouve rien et l'outil quitte
immédiatement avec

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

en nommant un programme que l'utilisateur ne fait pas tourner, avant le moindre
travail. La réponse de son propre auteur aux deux rapports Linux ([#716][pg716]
et [#725][pg725], tous deux venus de Fluorine, le portage Linux de MO2, en FUSE
pour la même raison) est un drapeau qui saute la vérification.

Eidos l'ensemence donc. Un PGPatcher trouvé dans un dossier de mods arrive avec
l'argument déjà mis, et taper `PGPatcher` comme titre dans le dialogue
Executables remplit le champ Arguments de la même façon. Ce que vous tapez gagne
toujours : l'ensemencement ne remplit qu'un champ laissé vide, exactement comme
les prérequis ci-dessus.

La liste est dans `default_args` (`crates/eidos-instance/src/tools.rs`), à côté
de `default_prereqs`. Elle a une entrée, et une seule suffit à justifier une
fonction : personne ne peut deviner un drapeau, et l'échec qu'il évite nomme le
mauvais programme.

Son exécution n'exige rien, fait rare pour un outil .NET : il embarque le sien
(`dotnetlib/`, avec un `runtimeconfig.json` déclarant `includedFrameworks`) et
livre les DLL du CRT Microsoft à côté de son exécutable. Laissez Prereqs vide.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher et PGLite en mode Mod Organizer 2

Avec Mod Manager réglé sur `None`, PGPatcher voit un seul dossier `Data`
fusionné et ne peut pas savoir de quel mod vient un mesh ou une texture : il n'a
donc aucun conflit à montrer ni aucun ordre de mods à suivre. Son mode
`Mod Organizer 2` lit l'instance elle-même : le `modlist.txt` du profil, les mods
activés dans leur ordre de priorité, et les fichiers du dossier de chaque mod.
PGLite, la version en jeu, lit l'instance de la même façon. Une instance Eidos a
déjà la disposition de MO2 (`mods/`, `profiles/<name>/modlist.txt`,
`overwrite/`), ce mode fonctionne donc avec elle.

Dans le lanceur de PGPatcher :

1. Réglez **Mod Manager** sur `Mod Organizer 2`.
2. Réglez **MO2 Options -> Instance Location** sur le dossier de l'instance
   Eidos, écrit comme un chemin Windows sur le lecteur `Z:` de Wine, qui est `/`.
   Une instance portable dans `/mnt/Games/Eidos-Skyrim` devient
   `Z:\mnt\Games\Eidos-Skyrim` ; une instance globale devient
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Gardez `--ignore-mo2vfscheck` dans les arguments de l'outil. Dans ce mode,
   PGPatcher cherche usvfs et s'arrête s'il ne le trouve pas.

PGPatcher n'accepte le dossier que s'il contient un `modorganizer.ini` : Eidos
écrit donc `ModOrganizer.ini` à la racine de l'instance chaque fois qu'il lance
le jeu ou un outil, pour le profil lancé. Le fichier n'est réécrit que si son
contenu change. Il contient :

- dans `[General]` : `gameName` (le nom du jeu dans MO2, donc `New Vegas` pour Fallout: New Vegas), `gamePath` (le dossier d'installation du jeu
  sous forme de chemin `Z:`) et `selected_profile` ;
- dans `[Settings]` : `mod_directory`, `profiles_directory` et
  `overwrite_directory` en chemins `Z:` absolus, plus `download_directory` si
  l'instance a un dossier `downloads/`.

Sa première ligne est un commentaire qui dit qu'Eidos l'a écrit. Un
`ModOrganizer.ini` sans cette ligne, quelle que soit la casse de son nom, n'est
jamais modifié ni supprimé : une instance MO2 que vous avez ouverte dans Eidos
garde le sien, et PGPatcher lit ce fichier tel quel. MO2 supprime les
commentaires quand il enregistre ses réglages, donc un fichier sur lequel MO2 a
enregistré lui appartient dès lors. Un fichier vide compte comme celui d'Eidos.

Un tel fichier nomme le profil que MO2 a sélectionné en dernier, et PGPatcher
utilise ce profil, pas celui qu'Eidos lance. Quand les deux diffèrent, Eidos
consigne un avertissement au lancement. Sélectionnez le même profil dans MO2, ou
renommez ou supprimez le fichier pour qu'Eidos écrive le sien.

Eidos n'écrit son fichier qu'au lancement de quelque chose : entre deux
lancements, il nomme le profil du dernier. Un outil lancé par Eidos y trouve
son propre profil.

PGPatcher s'arrête sur une erreur quand le profil n'a aucun mod activé, et quand
son dossier de sortie est celui d'un mod activé, comme il le fait sous MO2.

## Le chemin du jeu dans le préfixe

Les outils Windows trouvent leur jeu en lisant
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, une clé qu'écrit
l'installeur du jeu lui-même - et que Steam sous Proton n'exécute jamais. Sans
elle, xEdit, Wrye Bash et DynDOLOD s'ouvrent sur un chemin vide. Eidos l'écrit
avant de lancer un outil : idempotent, additif, et sauté si le préfixe n'est pas
initialisé ou s'il est en cours d'utilisation.

## Atteindre un outil : masquer, épingler, et un raccourci de bureau

Les défauts d'un jeu comprennent des outils dont vous ne vous servirez peut-être
jamais, et un sélecteur qui liste huit entrées pour en atteindre la deuxième est
un sélecteur que personne ne lit. Dans la boîte de dialogue Executables :

- **Pin to top** place une entrée en tête de la liste Run.
- **Hide from picker** en retire une sans la supprimer.
- **Desktop shortcut** écrit un `.desktop` dans
  `~/.local/share/applications` - là où un lanceur a sa place sur un système
  freedesktop, si bien qu'il apparaît dans votre menu d'applications et dans une
  recherche plutôt que sur le bureau. Il exécute directement
  `eidos tool <instance> run <title>`, ce qui veut dire que l'outil démarre **à
  travers la vue fusionnée avec le profil de cette instance** sans que la fenêtre
  d'Eidos soit ouverte du tout.

Masquer et épingler concernent la façon dont un outil est *atteint* plutôt que ce
qu'il exécute, ils s'appliquent donc aux défauts par jeu comme à vos propres
entrées.

## Un outil qui est sa propre application Steam

Le Creation Kit est une application Steam distincte et veut son propre AppID ;
quelques autres outils de modding distribués via Steam sont dans le même cas.
Définissez **Steam AppID** sur l'entrée et Eidos le lance sous cet identifiant
plutôt que sous celui du jeu.

Sous Windows cela veut dire un launcher différent. Ici, ce sont deux variables
d'environnement sur l'exécution qui était déjà en train d'être construite -
`SteamAppId` et `SteamGameId`, les deux, parce que Proton lit l'une et les
bibliothèques de Steam lisent l'autre, et qu'un outil qui les voit en désaccord
échoue bizarrement plutôt que clairement. `eidos tool ... --print` montre
exactement ce que recevrait la vraie exécution.

## Variables d'environnement et outils lancés sans Proton

Trois clés de `<instance>/tools.ini` n'ont pas encore de champ dans la boîte de
dialogue Executables ; ajoutez-les à la main dans la section de l'outil. Chaque
ligne `env=NAME=value` définit une variable pour les exécutions de cet outil
seulement :

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos applique ces lignes après tout ce qu'il définit lui-même, donc une valeur
mise ici remplace la sienne. Cela vaut aussi pour le `WINEDLLOVERRIDES` qu'il
construit à partir des DLL que livrent vos mods : un outil qui en définit un
doit y lister toutes les DLL dont il a besoin. Un nom se compose de lettres, de
chiffres et de `_`, et ne commence pas par un chiffre. Une ligne avec un autre
nom est ignorée.

`runner=native` lance `exe` lui-même, sans Proton. La vue fusionnée est montée
comme pour tout outil, et les lignes `arg0=`, `arg1=` sont ses arguments.
Servez-vous-en pour un programme Linux, ou pour un outil sur une copie que Steam
ne gère pas et pour laquelle Heroic n'enregistre aucun runner : `exe` est le
Wine de cette copie, le programme Windows est le premier argument, et le
préfixe vient de `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` doit être un chemin absolu. Le répertoire de travail est par défaut le
dossier qui contient `exe`, ici celui du lanceur, donc définissez `workdir=`.
Seul `exe` est réécrit vers son chemin fusionné : pour un programme Windows
placé dans un mod, donnez son chemin sous le dossier `Data` du jeu, là où la vue
fusionnée le montre. Un outil natif saute les étapes propres à Proton : la clé
de registre ci-dessus, le cache de shaders de Steam, les racines .NET vides et
le retrait de l'overlay Steam de `LD_PRELOAD`.

Sur une copie GOG ou Epic installée avec Heroic, les outils n'ont pas besoin
d'une telle entrée quand les réglages Heroic du jeu
(`GamesConfig/<appName>.json`) désignent une version de Wine ou de Proton :
Eidos les lance avec cette version dans le préfixe Heroic du jeu, et
`eidos prereqs <instance> --install` la donne à winetricks. Une copie installée
avec Legendary seul n'a pas de réglages Heroic.

`slr=1` lance l'outil, et l'import dans le registre qui le précède, dans le
conteneur Steam Linux Runtime que nomme le Proton de l'outil
(`require_tool_appid` dans son `toolmanifest.vdf`), comme Steam lance le jeu. Ce
runtime doit être installé dans une bibliothèque Steam ; sinon, l'exécution
s'arrête et nomme l'application Steam à installer. `eidos prereqs` exécute
toujours winetricks sur l'hôte, et un outil en `runner=native` ignore la clé. Le
conteneur voit votre dossier personnel et la bibliothèque Steam du jeu. Un
outil, un mod ou un runtime situé ailleurs, sur un autre disque par exemple,
demande `env=STEAM_COMPAT_MOUNTS=/ce/chemin` (plusieurs chemins séparés par
`:`). Un Proton installé sous `/usr/share/steam/compatibilitytools.d` n'est
probablement pas utilisable avec `slr=1`, car `/usr` dans le conteneur est celui
du runtime. Aucune exécution d'outil dans le conteneur n'a encore été testée.

`eidos tool <instance> run <title> --print` montre la commande finale et chaque
variable que l'exécution définit par-dessus l'environnement hérité, ces lignes
comprises.

## Les réglages propres à un outil restent les siens

Eidos met un outil au bon endroit avec les bonnes DLL. Ce que l'outil fait
ensuite de sa configuration, c'est entre vous et lui, et l'échec est généralement
silencieux.

L'exemple travaillé, parce qu'il coûte une heure sinon : le **Game Data Path** de
BodySlide (Settings) doit pointer sur le répertoire `Data` du jeu, pas sur le
dossier du jeu au-dessus. Réglé un niveau trop haut, un batch build annonce
« All sets processed successfully » et écrit 1439 meshes là où le jeu n'ira
jamais les chercher. Eidos les rattrape - ils atterrissent dans `Overwrite/Root/`
plutôt que dans votre installation - mais rien ne cloche du point de vue du jeu,
sinon que vos corps ne sont pas construits.

La sortie des outils a sa place dans l'Overwrite. Quand une exécution produit
quelque chose qui vaut la peine d'être gardé, **Overwrite -> Create mod...** en
fait un mod ordinaire, qu'on peut ordonner, désactiver et supprimer comme
n'importe quel autre.
