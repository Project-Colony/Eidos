<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# Outils : xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

Un outil lancé à travers Eidos voit la vue fusionnée, à l'intérieur du préfixe Wine
du jeu (le préfixe Proton de Steam, ou celui qu'Heroic a créé). Il lit ce que le
jeu lira (tous les mods activés, dans l'ordre de priorité), et tout ce qu'il écrit
atterrit dans l'Overwrite, où un clic en fait un vrai mod. Un outil `runner=native`
tourne sans Proton ; voir
[plus bas](#variables-denvironnement-et-outils-lancés-sans-proton).

## Ceux qu'Eidos trouve tout seul

Certains outils portent un nom assez unique pour être trouvés plutôt que déclarés, et
xEdit est le cas évident : `FO4Edit.exe` pour Fallout 4, `SSEEdit.exe` pour Skyrim
SE, `TES5Edit.exe` pour l'original, et ainsi de suite. Chacun vient avec une entrée
QuickAutoClean, qui nettoie les dirty edits dont LOOT prévient. `PGPatcher.exe` est
trouvé de la même manière sur tous les jeux Bethesda sauf Morrowind, et il arrive
avec un argument sans lequel il ne peut pas fonctionner (voir
[plus bas](#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck)). Eidos les cherche, par nom
de fichier, dans :

- `Overwrite/Root/`, le dossier du jeu que l'instance exécute (l'installation de la
  boutique, ou sa copie du jeu), et les arborescences `Root/` des mods activés ;
- le `mods/` de cette instance, là où les utilisateurs de MO2 installent leurs
  outils ;
- le dossier d'outils que vous avez défini dans les Préférences (Général -> Outils), pour
  un répertoire partagé entre instances, comme `/mnt/Games/Tools`.

Les générateurs courants se trouvent de la même façon, chacun pour les jeux qu'il
prend en charge : Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen et
Synthesis. BethINI Pie (`Bethini.exe`) aussi, sur les mêmes jeux que PGPatcher ;
voir [BethINI Pie](#bethini-pie). Sur Stardew Valley, SMAPI
(`StardewModdingAPI.exe`) est trouvé de la même façon. C'est aussi le chargeur de
script extender de ce jeu, donc il démarre le jeu par la boutique, comme décrit dans
[Atteindre un outil](#atteindre-un-outil--masquer-épingler-et-un-raccourci-de-bureau).

La liste est par jeu, donc une instance Skyrim ne se verra jamais proposer l'éditeur
de Fallout. La recherche s'arrête à quatre niveaux de profondeur, parce qu'un pool
de mods représente des centaines de milliers de fichiers et que cela s'exécute à
chaque construction de la liste d'outils, et elle ne suit pas les liens
symboliques. Un outil trouvé ainsi est configuré exactement comme un outil que vous
auriez saisi : ses prérequis, ses arguments (le `--ignore-mo2vfscheck` de PGPatcher)
et son mod de capture viennent de son titre, selon les règles ci-dessous.

Si un outil est ailleurs, ou si vous voulez d'autres arguments, ajoutez-le à la
main. Une entrée utilisateur portant le même titre l'emporte sur tout ce qui a été
trouvé automatiquement.

## Nettoyer un plugin que LOOT signale comme sale

Après un tri LOOT, l'onglet Diagnostics liste les plugins que LOOT signale comme
sales, pour les jeux qui ont à la fois un xEdit et des données de nettoyage LOOT
(Skyrim LE, SE et VR, Enderal SE, Fallout 3, New Vegas, Fallout 4 et 4 VR,
Oblivion, Starfield). Quand Eidos trouve xEdit, enregistré dans Exécutables ou à
n'importe quel endroit de l'instance où il cherche des outils, la carte propose
Tout nettoyer et chaque plugin sale propose Nettoyer avec QuickAutoClean dans le menu de sa
ligne ; sinon, la carte indique d'ajouter xEdit dans Exécutables. Eidos ne
télécharge pas xEdit. Il lance le mode QuickAutoClean de xEdit sur un plugin à la
fois, à travers la vue fusionnée, avec `-qac -autoexit -autoload`. xEdit enregistre
le plugin nettoyé par-dessus celui qu'il a chargé, ce qui, dans la vue fusionnée,
donne un nouveau fichier dans l'Overwrite ; Eidos le déplace dans un mod nommé
`Cleaned plugins`, créé activé en bas de la liste, où il l'emporte dans tous les
conflits. Le plugin dans son propre mod, ou dans le dossier Data du jeu, n'est
jamais modifié. Désactivez `Cleaned plugins`, ou masquez-y un fichier, pour annuler
un nettoyage. Un plugin qui se trouve dans l'Overwrite n'est pas nettoyé, parce que
xEdit réécrirait ce fichier lui-même ; transformez d'abord l'Overwrite en mod.

Les sauvegardes, le cache et le journal de xEdit vont dans
`<instance>/logs/qac-<time>/`, hors de la vue fusionnée, et la ligne d'état donne
les nombres lus dans ce journal. Eidos supprime ce dossier après un nettoyage réussi
et le garde après un échec. Eidos enregistre dans `Cleaned plugins/meta.ini` la
taille et la date du plugin dont chaque copie a été tirée. Quand ce plugin change,
l'onglet Diagnostics affiche **Copie périmée de <plugin>**, avec un bouton qui
masque l'ancienne copie.

## En ajouter un

Dans l'interface : **Outils** dans la barre de menus, ou **Exécutables** dans la
barre d'outils, puis Ajouter. Depuis la ligne de commande :

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # lister ce qui est enregistré
eidos tool skyrimse run BodySlide         # le lancer à travers la vue fusionnée
eidos tool skyrimse run BodySlide --print # afficher la commande sans l'exécuter
eidos tool skyrimse rm BodySlide          # retirer une entrée que vous avez ajoutée
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # lancer un exe une fois, sans l'enregistrer
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` capture la sortie de cette exécution dans `mods/<mod>`, quel que
soit le réglage propre à l'outil. `rm` ne retire que les entrées que vous avez
ajoutées ; une entrée par défaut d'un jeu peut être masquée, pas retirée.

Un exe lancé avec `--exe` dont le nom de fichier est un nom qu'Eidos connaît pour le
jeu, sans tenir compte de la casse, reçoit le titre de cet outil, et avec lui les
prérequis, les arguments et le mod de capture ci-dessous. `GenerateFNISforUsers.exe`
est FNIS, donc la ligne `--exe` ci-dessus demande `dotnet48` et capture dans
`FNIS Output`. Tout autre exe est titré d'après son nom de fichier. Dans l'onglet
Data, une ligne `.exe` a **Ajouter**, qui ouvre la boîte de dialogue Exécutables
avec l'entrée préremplie, et **Lancer**, qui lance le fichier une fois comme le fait
`--exe`. Les deux titrent le fichier de la même façon.

Pour une commande qui n'est pas un outil Windows, `eidos shell skyrimse` ouvre votre
`$SHELL` (`/bin/sh` s'il n'est pas défini) dans le dossier du jeu avec la vue
fusionnée montée. Ce que le shell écrit atterrit dans l'Overwrite, et quitter le
shell démonte la vue. `eidos shell skyrimse -- <command>` exécute une seule commande
à la place. Ctrl+C dans le shell interactif arrête la commande que vous avez tapée
et laisse le shell ouvert. Le shell tourne sans Proton ; un programme Windows passe
par `eidos tool`.

Dans la fenêtre, **Arrêter** termine une exécution d'outil qui se bloque : appuyez deux
fois, et Eidos termine les programmes de l'exécution, puis traite ses fichiers comme
après n'importe quelle fermeture. Avec **Prévenir le bureau à la fin d'une tâche longue**
activé (Préférences, Général, Pendant le jeu), une exécution d'outil qui se termine
alors que la fenêtre est en arrière-plan envoie une notification de bureau.

Le script extender, le binaire du jeu et le launcher sont détectés automatiquement ;
seuls les outils supplémentaires ont besoin d'être enregistrés. Sur une instance qui
tourne depuis une copie du jeu, ils sont détectés dans la copie ; voir
[Outils sur une copie du jeu](#outils-sur-une-copie-du-jeu).

### Pointez-le sur le vrai fichier, où qu'il soit

Enregistrez l'exécutable là où il se trouve réellement. Si l'outil a été installé
comme un mod, c'est à l'intérieur du dossier du mod :

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

C'est le chemin de l'instance globale. Pour une instance portable, la même règle
s'applique sous son propre dossier, `<instance>/mods/...`. Un chemin absolu comme
celui-ci est la seule chose qui ne survit pas au déplacement ultérieur d'un dossier
portable.

Eidos réécrit ce chemin vers le chemin fusionné avant le lancement, si bien que
l'outil s'exécute depuis `<game>/Data/CalienteTools/BodySlide/` et y voit aussi les
fichiers de tous les autres mods. BodySlide livre un répertoire `SliderSets` vide, et
tous les corps qu'il sait construire viennent de CBBE et des mods de tenues. Lancé
depuis son propre dossier de mod, il ne trouve rien et paraît cassé.

MO2 fait la même réécriture, pour la même raison : son propre commentaire cite FNIS.

Un outil dans un mod désactivé ne peut pas être réécrit, parce que ses fichiers ne
sont pas dans la vue non plus. Eidos le dit et le lance depuis son propre dossier.

## Outils d'une liste Mod Organizer 2

Une liste Wabbajack ou une autre instance MO2 ouverte comme instance Eidos garde son
`ModOrganizer.ini`, et les exécutables qui y sont configurés peuvent être copiés dans
`tools.ini` :

```sh
eidos tool /path/to/list import-mo2           # affiche ce qui serait ajouté
eidos tool /path/to/list import-mo2 --write   # les ajoute
```

Dans la fenêtre, la boîte de dialogue Exécutables a **Importer depuis ModOrganizer.ini**,
avec une case à décocher pour chaque entrée. **Ajouter la sélection** place les entrées
cochées dans la liste, et rien n'est enregistré avant d'appuyer sur Enregistrer. Seul compte
un `ModOrganizer.ini` qu'Eidos n'a pas écrit ; voir
[PGPatcher et PGLite](#pgpatcher-et-pglite-en-mode-mod-organizer-2) pour celui
qu'Eidos écrit.

Les chemins sur `Z:` sont des chemins Linux et restent tels quels. Un chemin sur une
autre lettre de lecteur est ramené sous le dossier de la liste quand il nomme ce
dossier (`D:\Lists\MyList\tools\SSEEdit.exe` devient `<list>/tools/SSEEdit.exe`),
dans l'exécutable, le dossier de travail et les arguments. Le chargeur du script
extender, le launcher et le binaire du jeu deviennent de simples noms, et s'exécutent
donc depuis le dossier du jeu sur lequel Eidos monte la vue, comme les entrées
intégrées. Le `hide` de MO2 devient **Masquer du sélecteur**, et `steamAppID` devient
**AppID Steam**. Les entrées qui pointent ailleurs (un programme système de Windows,
un outil hors de la liste) sont signalées comme ignorées avec la raison, et un titre
déjà présent dans `tools.ini` n'est jamais remplacé.

Une entrée importée garde le titre et les arguments de MO2, et Eidos ne remplit rien à
partir de son titre : ni prérequis, ni argument supplémentaire, ni mod de capture.
Réglez-les dans la boîte de dialogue Exécutables. Une entrée PGPatcher venue d'une
liste faite sous Windows n'a en général pas `--ignore-mo2vfscheck` ; ajoutez-le
(voir [plus bas](#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck)).

Un chemin importé qui pointe dans le dossier `Stock Game` de la liste n'est ramené
sur l'installation de la boutique, au moment de l'exécution seulement, que si
l'instance tourne depuis cette copie ; voir la section suivante.

## Outils sur une copie du jeu

Quand l'instance tourne depuis une copie du jeu (le dossier `Stock Game` ou
`Game Root` d'une liste, ou une copie figée ; voir
[Fichiers du jeu](usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie)), un outil voit
les fichiers de la copie, pas ceux de l'installation de la boutique. La vue fusionnée
est toujours montée sur l'installation de la boutique : la vue root a la copie tout
en bas, et la vue Data lit le `Data` de la copie. Eidos cherche le script extender,
le launcher, le binaire du jeu et les outils connus dans la copie plutôt que dans
l'installation de la boutique.

Un chemin d'outil, un argument ou un dossier de travail qui pointe dans la copie est
ramené sur l'installation de la boutique avant l'exécution. Il peut être écrit comme
un chemin Linux ou comme un chemin Wine `Z:` : `-D:"Z:\...\Stock Game\Data"` devient
le même chemin sous l'installation de la boutique. Les entrées importées du
`ModOrganizer.ini` d'une liste en ont besoin. Quand l'instance tourne depuis
l'installation de la boutique, rien n'est déplacé, si bien qu'une telle entrée lit la
copie nue, sans vos mods.

Le `installed path` du jeu dans le registre du préfixe et le `gamePath` du
`ModOrganizer.ini` qu'écrit Eidos nomment toujours l'installation de la boutique, là
où est la vue fusionnée. Si le dossier de la copie est absent, ou ne contient aucun
binaire du jeu, une exécution d'outil s'arrête, comme un lancement du jeu, et le
message nomme `eidos stock-game <instance> store` pour revenir à l'installation de la
boutique.

`eidos stock-game <instance>` montre quels fichiers du jeu l'instance exécute et
vers quelles copies elle pourrait basculer. `use <folder>` et `store` font basculer,
et `freeze --yes` copie l'installation de la boutique dans le dossier `Stock Game` de
l'instance et exécute depuis elle. La carte Fichiers du jeu de l'onglet Diagnostics fait
la même chose ; voir
[Fichiers du jeu](usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie).

## Envoyer la sortie d'un outil dans son propre mod

Un générateur (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) écrit des centaines de
fichiers. Par défaut ils atterrissent dans l'Overwrite avec tout le reste. Dans
l'éditeur Exécutables, réglez **Capturer la sortie dans** et la sortie de cette exécution
ira dans ce mod-là :

```
Exécutables -> (votre outil) -> Capturer la sortie dans : FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide et Synthesis arrivent avec un mod de capture déjà
renseigné : `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` et
`Synthesis Output`. DynDOLOD, TexGen et xLODGen arrivent sans, car ils écrivent dans
le dossier de sortie que vous leur donnez.

Le mod est créé s'il n'existe pas. Seuls les fichiers produits par cette exécution
sont déplacés ; tout ce qui était déjà dans l'Overwrite y reste, si bien que deux
outils avec des cibles de capture ne se volent pas mutuellement leur sortie. Une
exécution qui n'a rien écrit ne laisse pas de mod vide derrière elle.

Eidos déplace les fichiers après l'exécution. MO2, lui, pointe la couche d'écriture
sur le mod, ce qui le promeut en priorité maximale pour toute l'exécution : tous les
conflits dans lesquels il est impliqué s'inversent, puis s'inversent à nouveau
ensuite. Les écritures passent aussi directement à travers les fichiers du mod
lui-même, sans copy-up. La capture atteint le même état final sans ni l'un ni
l'autre.

Si le mod cible est désactivé, la sortie est quand même écrite mais le jeu ne la
verra pas, si bien que l'outil régénérerait les mêmes fichiers à l'exécution
suivante. Eidos prévient quand c'est le cas.

### Quand sa sortie n'est plus à jour

Eidos note à partir de quels mods, de quel ordre des plugins et de quel profil la
sortie d'un outil a été produite, la taille et la date de l'exécutable de l'outil, et
le chemin et les arguments de l'entrée dans `tools.ini`. Quand l'un d'eux change,
l'onglet Diagnostics affiche **Les entrées des fichiers générés ont changé**, et la carte propose
un bouton **Relancer <tool>** qui relance l'outil avec sa configuration actuelle.

Pour un générateur dont la sortie périmée casse le jeu, comme Pandora ou Nemesis,
ajoutez `check_before_play=1` à son entrée dans `tools.ini`, ou activez **Demander avant
de jouer si sa sortie n'est plus à jour** pour lui dans la boîte de dialogue
Exécutables. Lancer demande alors d'abord : lancer l'outil, ou jouer quand même. Jouer
quand même est retenu jusqu'à ce que les mods, les plugins ou le profil changent de
nouveau, ou que la fenêtre se ferme. Eidos ne lance jamais l'outil de lui-même. Avec
**Lancer le jeu directement quand une boutique ouvre Eidos** activé, une telle question garde
la fenêtre ouverte au lieu de démarrer le jeu.

## Les prérequis et les arguments viennent du titre

Le titre que vous donnez à un outil décide des prérequis runtime qu'Eidos lui
provisionne. La correspondance est une sous-chaîne du titre, insensible à la casse.

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

Ainsi un outil enregistré sous `BodySlide` reçoit ses DLL DirectX ; le même
exécutable enregistré sous `BS` ne reçoit rien et peut échouer au démarrage avec une
erreur qui ne dit rien des DLL. Nommez les outils d'après le programme.

La liste est dans `default_prereqs` (`crates/eidos-instance/src/tools.rs`), et le champ
Prérequis de la boîte de dialogue Exécutables est modifiable : le titre ne remplit que
la valeur par défaut.

Le titre décide des arguments de la même manière, pour le seul outil qui en exige un
que personne ne pourrait deviner : un titre contenant `pgpatcher` (ou `parallaxgen`,
son ancien nom) reçoit `--ignore-mo2vfscheck`. Pourquoi, et pourquoi ce n'est pas
optionnel, [plus bas](#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck). Le mod de capture
vient aussi du titre (voir [plus haut](#envoyer-la-sortie-dun-outil-dans-son-propre-mod)).

### Trois sortes de prérequis

Le palier 1 est celui des DLL fournies : `d3dx9_43`, `d3dcompiler_47`,
`d3dcompiler_43` et `d3dx11_43`. Eidos les embarque dans ses binaires et les copie
dans le préfixe au lancement. Dans un préfixe 64 bits, le choix par défaut de Proton,
la version 64 bits va dans `system32` et la 32 bits dans `syswow64`. Rien à faire,
pas de réseau. Chaque version est stockée compressée en gzip, si bien que les huit
fichiers prennent 7,9 Mo de chaque binaire au lieu de 17,9 Mo, et seul celui qui est
copié est décompressé (4,7 Mo au plus).

Le palier 2 est celui des installateurs : `vcrun2022`, `dotnet6`, `dotnet7`,
`dotnet8`, `dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` et `xact_x64`. Ils
écrivent des clés de registre, le GAC et les hôtes CLR, ils ne peuvent donc pas être
copiés comme des fichiers. Eidos en installe trois sans winetricks : `vcrun2022`
lance les `VC_redist.x86.exe` et `VC_redist.x64.exe` de Microsoft, et `dotnet8` et
`dotnetdesktop8` lancent l'installateur du .NET 8.0.12 Desktop Runtime, qui contient
aussi le runtime .NET. Le paquet de version (`eidos-linux-bundle.tar.gz`) fournit ces
installateurs dans son dossier `runtimes/`, et `install.sh` les copie dans
`~/.local/share/Colony/Eidos/runtimes/` pour l'utilisateur qui le lance, y compris
avec `--system`. Eidos y cherche chaque fichier d'abord, puis dans un dossier
`runtimes/` à côté du binaire `eidos` (un paquet décompressé exécuté sur place). Sans
l'une ou l'autre copie (le fichier unique `eidos-linux`, une compilation depuis les
sources, ou un autre utilisateur d'une installation système), Eidos les télécharge
chez Microsoft (`download.visualstudio.microsoft.com`) dans
`~/.local/share/Colony/Eidos/runtimes/` et les y garde. Dans les deux cas, un fichier
est refusé si son SHA-256 ne correspond pas à celui intégré à Eidos, et il s'exécute
sans fenêtre (`/install /quiet /norestart`) dans le préfixe du jeu, avec la version de
Proton ou de Wine qui lance les outils du jeu. Comme le fait winetricks, `vcrun2022`
passe d'abord les DLL Visual C++ en `native,builtin` dans les remplacements de DLL du
préfixe. Ensuite, avec une version de Proton, Eidos vérifie que le runtime C++ du
préfixe n'est pas à moitié celui de Microsoft et à moitié celui de Proton. Dans ce
cas l'installation échoue : le message nomme les paires de DLL qui viennent de
versions différentes, et dit d'installer plutôt `vcrun2022` avec winetricks, qui
remplace `msvcp140.dll` lui-même. Seul le .NET 8 64 bits est installé ; un outil
.NET 8 32 bits demande toujours `winetricks dotnetdesktop8`.

Les six autres verbes passent par le `winetricks` du système, qui doit être dans le
`PATH`. Eidos l'exécute avec le `wine` de la version de Proton ou de Wine qui lance
les outils du jeu, contre le préfixe du jeu, ce qui contourne le conteneur
pressure-vessel de Steam et le décalage protontricks + Proton-GE. Certains verbes
winetricks ont besoin de `cabextract`, et Eidos avertit quand il manque. Un outil qui
déclare un verbe de palier 2 non installé se lance quand même, avec un avertissement
nommant le verbe et la commande pour y remédier, parce que vous l'avez peut-être
installé autrement.

Un verbe de palier 2 est compté comme fait quand le `winetricks.log` du préfixe le
liste, quel que soit celui qui a lancé winetricks ou protontricks, ou quand Eidos l'a
installé dans ce même préfixe. Eidos enregistre ses propres installations dans
`prereqs.done` dans l'instance, liées au chemin et à l'inode du préfixe, si bien
qu'un préfixe reconstruit voit ses verbes de nouveau en attente. **Reconstruire le préfixe**,
une action de Diagnostics pour une copie Steam, renomme `compatdata/<appid>` en
`<appid>.eidos-backup-<time>` et ne supprime rien ; Steam crée un nouveau préfixe au
prochain démarrage du jeu.

Le palier 3 est celui des runtimes, `dotnet10` et `dotnetsdk10`. Un runtime .NET
moderne, c'est 193 fichiers qui vivent dans leur propre répertoire et sont trouvés via
`DOTNET_ROOT`. Il n'est jamais enregistré et jamais installé dans le préfixe, si bien
qu'aucun des deux autres paliers ne peut le porter. Eidos le garde hors de toute
instance, parce que 78 Mo, ce n'est ni par jeu ni par profil : chaque runtime est
décompressé dans `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`
(`dotnet10-10.0.10`, `dotnetsdk10-10.0.403`). Eidos cherche le zip du runtime sous son
nom de téléchargement dans les deux mêmes dossiers que les installateurs du palier 2,
et décompresse la copie que livre le paquet de version
(`dotnet-runtime-10.0.10-win-x64.zip`, pour `dotnet10` seulement) ou la télécharge. Le
paquet ne livre pas le `dotnet-sdk-10.0.403-win-x64.zip` du SDK, mais une copie
placée là à la main est utilisée et évite le téléchargement. Eidos refuse l'un ou
l'autre fichier si son SHA-256 ne correspond pas à celui intégré à Eidos. Il
décompresse le zip avec `7z`, le même 7-Zip que pour les archives de mods.

`dotnetsdk10` est le SDK .NET 10, un téléchargement de 301 Mo qui se décompresse en
808 Mo. Synthesis en a besoin pour compiler ses patchers, et il appelle un simple
`dotnet` pour les construire et les lancer. Un outil qui déclare `dotnetsdk10` reçoit
donc aussi le dossier du SDK dans `WINEPATH`, ainsi que
`NUGET_CERT_REVOCATION_MODE=offline`, `DOTNET_GENERATE_ASPNET_CERTIFICATE=false` et
`DOTNET_CLI_TELEMETRY_OPTOUT=1`. La compilation d'un patcher laisserait tourner des
nœuds MSBuild et le serveur VBCSCompiler, qui sous Wine maintiennent en vie le
wineserver du préfixe, donc l'outil reçoit aussi `MSBUILDDISABLENODEREUSE=1`,
`DOTNET_CLI_USE_MSBUILD_SERVER=0` et `UseSharedCompilation=false`.
`DOTNET_ROLL_FORWARD=Major` permet à un patcher compilé pour .NET 8 (Mutagen avant
0.54) de tourner sur le runtime .NET 10 du SDK quand aucun runtime .NET 8 n'est
trouvé. Une entrée Synthesis de `tools.ini` qui porte encore l'ancienne valeur par
défaut, `dotnet8`, `vcrun2022`, passe à `dotnetsdk10`, `vcrun2022` quand Eidos la lit.

NuGet vérifie la signature de chaque paquet qu'il restaure auprès du magasin de
racines de Windows, et il manque à un préfixe Wine les racines de signature de code et
d'horodatage que le SDK fournit sous `sdk/10.0.403/trustedroots/codesignctl.pem` et
`timestampctl.pem`. Sans elles, la restauration de `Mutagen.Bethesda.Synthesis` 0.36.6
échoue avec `NU3018` et `NU3028`. Avant de lancer un outil qui déclare `dotnetsdk10`,
une fois le SDK téléchargé, Eidos ajoute donc les 385 certificats des deux fichiers au
magasin de racines de l'utilisateur courant dans le préfixe du jeu, dans le même
import `regedit` que celui qui écrit le chemin du jeu (voir plus bas). Il les importe
de nouveau si l'un d'eux disparaît. Tous les programmes Windows de ce préfixe font
alors confiance à ces racines. Avec elles, dans un préfixe neuf et avec un cache NuGet
vide, la même restauration et la compilation réussissent avec le Wine 11.21 du système
et avec GE-Proton11-9.

Eidos n'installe rien des paliers 2 ou 3 tant que vous ne le demandez pas :

```sh
eidos prereqs skyrimse            # afficher ce dont les outils enregistrés ont besoin, et leur état
eidos prereqs skyrimse --install  # installer ce qui manque (les copies du paquet d'abord, puis les téléchargements)
```

Dans l'interface, les mêmes états se trouvent sous le champ Prereqs, et ceux qui
manquent sont des boutons. Un verbe qui n'est ni fourni, ni un runtime, ni un verbe de
palier 2 est signalé comme une faute de frappe probable plutôt que proposé au
téléchargement.

`eidos prereqs <instance> --install` refuse tant que le jeu, Steam ou un outil tient
encore le préfixe. Les runtimes du palier 3 s'installent même avant que le préfixe
existe. Les installateurs qu'Eidos lance lui-même ont besoin d'un préfixe existant :
tant que la boutique n'a pas démarré le jeu une fois, ils s'arrêtent avec « start the
game once through its store, then install again ». Les verbes du palier 2 sont
installés un par un et chaque succès est enregistré, si bien qu'après un échec la même
commande reprend. Pour une copie GOG ou Epic sans réglages Heroic, `--install` copie
les DLL fournies puis s'arrête avant les verbes du palier 2 : il dit d'utiliser le
winetricks propre à ce runner, et lit le `winetricks.log` du préfixe à la vérification
suivante.

### Pourquoi DynDOLOD a besoin de `dotnet10`

DynDOLOD ne construit pas le LOD des objets lui-même : il délègue à LODGen, et il en
livre trois. `LODGenx64.exe` cible .NET Framework 4.8, qui sous Proton est routé vers
le Mono de Wine, dont l'initialiseur de `System.Uri` appelle une méthode que Mono
n'implémente pas. LODGen meurt avant sa première ligne de travail, laissant un journal
qui contient une bannière de version et rien d'autre, et une boîte de dialogue
DynDOLOD qui dit seulement « failed for one or more worlds ».

Installer le vrai .NET Framework n'y change rien : Proton remplace `mscoree.dll`, le
chargeur qui l'aurait trouvé, par un lien symbolique vers sa propre arborescence, et
refait cela à chaque mise à jour du préfixe.

La build qui marche est `LODGenx64Win10.exe`, qui cible le .NET moderne et ne touche
jamais à `mscoree`. Pointez `DOTNET_ROOT` sur un runtime .NET 10 et il s'exécute.
C'est ce que provisionne `dotnet10`, et Eidos définit la variable au lancement de tout
outil qui le déclare.

### Pourquoi Pandora a besoin de `dotnet10`

Pandora est une application Avalonia, pas WPF, et elle cible `net10.0` depuis Pandora
4.1.0, si bien que le runtime .NET 10 seul suffit à la lancer. Quand elle ne trouve
aucun runtime, elle dit d'installer le .NET Desktop Runtime. C'est le texte générique
que le lanceur .NET affiche pour tout programme graphique qui ne trouve aucun runtime,
et `dotnetdesktop8` n'y change rien.

Eidos 1.21.0 et antérieurs donnaient `dotnetdesktop8` à Pandora, et une entrée garde sa
ligne `prereqs=`. Quand Eidos lit une entrée Pandora qui ne liste encore que
`dotnetdesktop8`, et que le `.runtimeconfig.json` à côté de son exécutable nomme
`net10.0`, l'entrée reçoit `dotnet10` à la place. Pandora 2.x cible .NET 8 et garde ce
qu'elle a.

### Pourquoi PGPatcher a besoin de `--ignore-mo2vfscheck`

PGPatcher réécrit les meshes et les plugins pour que les mods de textures que vous avez
(vanilla, parallax, complex material, PBR) utilisent le bon shader sur chaque surface.
Il refuse de tourner hors d'un gestionnaire de mods, parce que patcher un dossier de
jeu nu serait destructeur.

PGPatcher cherche usvfs, la couche d'accrochage de DLL que MO2 injecte dans le jeu
sous Windows. Eidos n'en a pas : la vue fusionnée est un montage FUSE que Wine
traverse sans savoir qu'il existe. La vérification ne trouve donc rien et l'outil
quitte immédiatement avec

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

en nommant un programme que l'utilisateur ne fait pas tourner, avant le moindre
travail. La réponse de son propre auteur aux deux rapports Linux ([#716][pg716] et
[#725][pg725], tous deux venus de Fluorine, le portage Linux de MO2, en FUSE pour la
même raison) est un drapeau qui saute la vérification.

Eidos l'ensemence donc. Un PGPatcher trouvé dans un dossier de mods arrive avec
l'argument déjà mis, et taper `PGPatcher` comme titre dans la boîte de dialogue
Exécutables remplit le champ Arguments de la même façon. Ce que vous tapez gagne
toujours : l'ensemencement ne remplit qu'un champ laissé vide, exactement comme les
prérequis ci-dessus.

La liste est dans `default_args` (`crates/eidos-instance/src/tools.rs`), à côté de
`default_prereqs`.

Son exécution n'exige rien, fait rare pour un outil .NET : il embarque le sien
(`dotnetlib/`, avec un `runtimeconfig.json` déclarant `includedFrameworks`) et livre
les DLL du CRT Microsoft à côté de son exécutable. Laissez Prereqs vide.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher et PGLite en mode Mod Organizer 2

Avec Mod Manager réglé sur `None`, PGPatcher voit un seul dossier `Data` fusionné et ne
peut pas savoir de quel mod vient un mesh ou une texture : il n'a donc aucun conflit à
montrer ni aucun ordre de mods à suivre. Son mode `Mod Organizer 2` lit l'instance
elle-même : le `modlist.txt` du profil, les mods activés dans leur ordre de priorité,
et les fichiers du dossier de chaque mod. PGLite, la version en jeu, lit l'instance de
la même façon. Une instance Eidos a déjà la disposition de MO2 (`mods/`,
`profiles/<name>/modlist.txt`, `overwrite/`), ce mode fonctionne donc avec elle.

Dans le lanceur de PGPatcher :

1. Réglez **Mod Manager** sur `Mod Organizer 2`.
2. Réglez **MO2 Options -> Instance Location** sur le dossier de l'instance Eidos,
   écrit comme un chemin Windows sur le lecteur `Z:` de Wine, qui est `/`. Une
   instance portable dans `/mnt/Games/Eidos-Skyrim` devient
   `Z:\mnt\Games\Eidos-Skyrim` ; une instance globale devient
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Gardez `--ignore-mo2vfscheck` dans les arguments de l'outil. Dans ce mode,
   PGPatcher cherche usvfs et s'arrête s'il ne le trouve pas.

PGPatcher n'accepte le dossier que s'il contient un `modorganizer.ini` : Eidos écrit
donc `ModOrganizer.ini` à la racine de l'instance chaque fois qu'il lance le jeu ou un
outil, pour le profil lancé. Le fichier n'est réécrit que si son contenu change. Il
contient :

- dans `[General]` : `gameName` (le nom du jeu dans MO2, donc `New Vegas` pour
  Fallout: New Vegas), `gamePath` (le dossier d'installation du jeu dans la boutique,
  sous forme de chemin `Z:`, aussi sur une instance qui tourne depuis une copie du
  jeu) et `selected_profile` ;
- dans `[Settings]` : `mod_directory`, `profiles_directory` et `overwrite_directory`
  en chemins `Z:` absolus, plus `download_directory` si l'instance a un dossier
  `downloads/`.

Sa première ligne est un commentaire qui dit qu'Eidos l'a écrit. Un `ModOrganizer.ini`
sans cette ligne, quelle que soit la casse de son nom, n'est jamais modifié ni
supprimé : une instance MO2 que vous avez ouverte dans Eidos garde le sien, et
PGPatcher lit ce fichier tel quel. MO2 supprime les commentaires quand il enregistre
ses réglages, donc un fichier sur lequel MO2 a enregistré lui appartient dès lors. Un
fichier vide compte comme celui d'Eidos.

Un tel fichier nomme le profil que MO2 a sélectionné en dernier, et PGPatcher utilise
ce profil, pas celui qu'Eidos lance. Quand les deux diffèrent, Eidos consigne un
avertissement au lancement. Sélectionnez le même profil dans MO2, ou renommez ou
supprimez le fichier pour qu'Eidos écrive le sien.

Les outils en mode Mod Organizer 2 lisent aussi le jeu dans le `gamePath` de ce
fichier. Quand il ne nomme ni le dossier du jeu que l'instance exécute ni
l'installation de la boutique, l'onglet Diagnostics affiche **ModOrganizer.ini indique un
autre dossier de jeu aux outils**, avec ce chemin et le dossier du jeu que cette
instance exécute. Changez `gamePath` dans le fichier, ou supprimez le fichier pour
qu'Eidos écrive le sien.

Eidos n'écrit son fichier qu'au lancement de quelque chose : entre deux lancements, il
nomme le profil du dernier. Un outil lancé par Eidos y trouve son propre profil.

PGPatcher s'arrête sur une erreur quand le profil n'a aucun mod activé, et quand son
dossier de sortie est celui d'un mod activé, comme il le fait sous MO2.

## Les programmes propres à Wine dans le préfixe du jeu

Le menu Fichier lance quatre programmes de Wine dans le préfixe du jeu : winecfg, regedit,
l'explorateur de fichiers de Wine et le désinstalleur. Ils démarrent comme n'importe
quel outil, avec la version de Proton ou de Wine qui lance les outils du jeu, mods
montés, et la fenêtre les attend. **drive_c du préfixe** ouvre le lecteur `C:` du préfixe
dans votre gestionnaire de fichiers. Une ligne reste grisée tant que le préfixe n'a pas
ce programme, ce qui arrive la première fois que la boutique lance le jeu. Depuis un
terminal, `eidos tool <instance> run winecfg` fait la même chose, de même que
`regedit`, `explorer` et `uninstaller`, quand aucun outil de la liste n'a ce titre.

Une modification faite dans regedit au `installed path` du jeu est remise en place au
lancement d'outil suivant ; voir la section suivante.

## Le chemin du jeu dans le préfixe

Les outils Windows trouvent leur jeu en lisant
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, une clé qu'écrit
l'installateur du jeu lui-même. Steam sous Proton n'exécute jamais cet installateur.
Sans la clé, xEdit, Wrye Bash et DynDOLOD s'ouvrent sur un chemin vide.

Avant un lancement d'outil, Eidos écrit la clé par un import `regedit` dans le
préfixe du jeu, sauf si un import précédent a déjà écrit le même chemin et que le
préfixe le contient toujours. La clé nomme l'installation de la boutique, aussi sur
une instance qui tourne depuis une copie du jeu, parce que la vue fusionnée est
montée là. Pour un outil qui déclare `dotnetsdk10`, le même import ajoute les racines
de signature NuGet (voir [plus haut](#trois-sortes-de-prérequis)). Un outil
`runner=native` saute cette étape. Quand le préfixe n'existe pas encore, ou qu'un
autre programme le tient, l'outil se lance quand même, et Eidos affiche un
avertissement qui nomme la boutique par laquelle démarrer le jeu une fois, ou le
programme à fermer. Quand la clé nomme un dossier autre que l'installation de la
boutique, l'onglet Diagnostics affiche **Les outils Windows ne trouvent pas <game>** jusqu'à ce
que le prochain lancement d'outil l'écrive de nouveau.

## Atteindre un outil : masquer, épingler, et un raccourci de bureau

Les défauts d'un jeu comprennent des outils dont vous ne vous servirez peut-être
jamais. Dans la boîte de dialogue Exécutables :

- **Épingler en haut** place une entrée en tête de la liste Lancer.
- **Masquer du sélecteur** en retire une sans la supprimer.
- **Raccourci de bureau** écrit un `.desktop` dans `~/.local/share/applications`, si bien
  qu'il apparaît dans votre menu d'applications et dans une recherche, pas sur le
  bureau. Il exécute directement `eidos tool <instance> run <title>`, ce qui veut dire
  que l'outil démarre à travers la vue fusionnée avec le profil de cette instance sans
  que la fenêtre d'Eidos soit ouverte.

Masquer et épingler ne changent que la façon dont un outil est atteint, ils
s'appliquent donc aux défauts par jeu comme à vos propres entrées.

Le chargeur du script extender, le launcher et le binaire du jeu n'ont pas de bouton
Raccourci de bureau. Ils démarrent le jeu, qui démarre toujours par sa boutique, et le
raccourci de la boutique fait déjà cela. Un raccourci créé pour l'un d'eux avec Eidos
1.22 ou antérieur fonctionne toujours : comme `eidos tool <instance> run <title>` tapé
dans un terminal, il demande à la boutique de démarrer le jeu, comme Lancer dans une
fenêtre qu'aucune boutique n'a ouverte. Quand le réglage de la boutique manque, la
commande ne démarre rien et affiche quoi régler. Un raccourci n'affiche aucun message :
lancez la même commande dans un terminal pour le lire.

Eidos confie ainsi une entrée à la boutique quand son exécutable a le nom de fichier
du chargeur du script extender, du launcher ou du binaire du jeu, sans tenir compte de
la casse : une entrée par défaut, une que vous avez ajoutée, une importée de MO2, ou
une exécution `--exe`. Il marque d'abord cette instance comme la dernière utilisée, si
bien que la fenêtre que la boutique ouvre montre cette instance, puis exécute
`steam -applaunch <id>`, `xdg-open heroic://launch?appName=<app>&runner=<runner>` pour
Heroic, ou `legendary launch <app>`. Avec `--print`, il affiche cette commande de la
boutique et ne démarre rien. La commande `steam` doit être dans le `PATH` : le Steam
Flatpak ne l'y met pas, donc le démarrage échoue et le message dit de démarrer le jeu
depuis Steam.

L'entrée s'exécute directement à travers la vue fusionnée, comme n'importe quel outil,
quand une boutique a démarré le processus : l'une de `SteamGameId`, `SteamAppId`,
`HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` ou `EIDOS_STORE_STARTED` est définie. Sans l'une
d'elles, Eidos ne démarre pas lui-même le jeu avec Proton. Quand l'instance n'a aucun
chargeur de script extender (dans le dossier du jeu qu'elle exécute, dans le `Root/`
d'un mod activé ou dans `Overwrite/Root/`), rien ne démarre et la ligne dit
d'installer d'abord le script extender. Pour une copie GOG hors d'Heroic, ou un
identifiant de boutique qu'Eidos ne sait pas transmettre, elle dit de démarrer le jeu
depuis cette boutique.

Un profil peut avoir son propre raccourci pour le jeu. Faites un clic droit sur le
profil et choisissez **Raccourci de bureau** : l'entrée exécute
`eidos tool <instance> run "<game>" --profile <name>`, qui fait de ce profil le profil
actif, comme le choisir dans la fenêtre, puis demande à la boutique de démarrer le jeu.
La fenêtre d'Eidos que la boutique ouvre affiche ce profil. `--profile` fonctionne avec
tout `eidos tool ... run` et sélectionne le profil avant que l'outil démarre ; avec
`--print`, il ne sélectionne rien.

## Un outil qui est sa propre application Steam

Le Creation Kit est une application Steam distincte et veut son propre AppID ; quelques
autres outils de modding distribués via Steam sont dans le même cas. Définissez
**Steam AppID** sur l'entrée et Eidos le lance sous cet identifiant plutôt que sous
celui du jeu.

Sous Windows cela veut dire un launcher différent. Ici, ce sont deux variables
d'environnement sur la même exécution, `SteamAppId` et `SteamGameId`. Eidos définit les
deux, parce que Proton lit l'une et les bibliothèques de Steam lisent l'autre, et
qu'un outil qui les voit en désaccord échoue sans erreur claire.
`eidos tool ... --print` montre exactement ce que recevrait la vraie exécution.

## Variables d'environnement et outils lancés sans Proton

Trois clés de `<instance>/tools.ini` n'ont pas encore de champ dans la boîte de
dialogue Exécutables ; ajoutez-les à la main dans la section de l'outil. Chaque ligne
`env=NAME=value` définit une variable pour les exécutions de cet outil seulement :

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos applique ces lignes après tout ce qu'il définit lui-même et après les variables
du `launch-env.ini` du profil, donc une valeur mise ici remplace les deux. Cela vaut
aussi pour le `WINEDLLOVERRIDES` qu'Eidos construit à partir des DLL que livrent vos
mods : un outil qui en définit un doit y lister toutes les DLL dont il a besoin. Les
wrappers du profil entourent la commande d'un outil comme ceux du jeu ; voir
[Variables et wrappers par profil](usage.md#variables-et-wrappers-par-profil). Un
nom se compose de lettres, de chiffres et de `_`, et ne commence pas par un chiffre.
Une ligne avec un autre nom est ignorée.

`runner=native` lance `exe` lui-même, sans Proton. La vue fusionnée est montée comme
pour tout outil, et les lignes `arg0=`, `arg1=` sont ses arguments. Servez-vous-en pour
un programme Linux, ou pour un outil sur une copie que Steam ne gère pas et pour
laquelle Heroic n'enregistre aucun runner : `exe` est le Wine de cette copie, le
programme Windows est le premier argument, et le préfixe vient de `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` doit être un chemin absolu. Le répertoire de travail est par défaut le dossier
qui contient `exe`, ici celui du runner, donc définissez `workdir=`. Seul `exe` est
réécrit vers son chemin fusionné : pour un programme Windows placé dans un mod, donnez
son chemin sous le dossier `Data` du jeu, là où la vue fusionnée le montre. Un outil
natif saute les étapes propres à Proton : la clé de registre ci-dessus, le cache de
shaders de Steam, les racines .NET vides et le retrait de l'overlay Steam de
`LD_PRELOAD`.

Sur une copie GOG ou Epic installée avec Heroic, les outils n'ont pas besoin d'une
telle entrée quand les réglages Heroic du jeu (`GamesConfig/<appName>.json`) désignent
une version de Wine ou de Proton : Eidos les lance avec cette version dans le préfixe
Heroic du jeu, et `eidos prereqs <instance> --install` l'utilise pour les installateurs
et pour winetricks. Une copie installée avec Legendary seul n'a pas de réglages Heroic.

`slr=1` lance l'outil, et l'import dans le registre qui le précède, dans le conteneur
Steam Linux Runtime que nomme le Proton de l'outil (`require_tool_appid` dans son
`toolmanifest.vdf`), comme Steam lance le jeu. Ce runtime doit être installé dans une
bibliothèque Steam ; sinon, l'exécution s'arrête et nomme l'application Steam à
installer. `eidos prereqs` exécute toujours ses installateurs et winetricks sur l'hôte,
et un outil en `runner=native` ignore la clé. Le conteneur voit votre dossier personnel
et la bibliothèque Steam du jeu. Un outil, un mod ou un runtime situé ailleurs, sur un
autre disque par exemple, demande `env=STEAM_COMPAT_MOUNTS=/ce/chemin` (plusieurs
chemins séparés par `:`). Un Proton installé sous `/usr/share/steam/compatibilitytools.d`
n'est probablement pas utilisable avec `slr=1`, car `/usr` dans le conteneur est celui
du runtime. Aucune exécution d'outil dans le conteneur n'a encore été testée.

`eidos tool <instance> run <title> --print` montre la commande finale et chaque
variable que l'exécution définit par-dessus l'environnement hérité, ces lignes
comprises. Pour une entrée qui démarre le jeu, il affiche à la place la commande de la
boutique (voir
[Atteindre un outil](#atteindre-un-outil--masquer-épingler-et-un-raccourci-de-bureau)).

## BethINI Pie

Eidos ne fournit pas BethINI Pie. Placez la version téléchargée depuis sa page Nexus
dans le dossier `mods/` de cette instance ou dans votre dossier d'outils, et
`Bethini.exe` apparaît dans la liste des outils, sur les mêmes jeux que PGPatcher.
Lancé par Eidos, il modifie la copie des INI du jeu propre au profil. Avant chaque
lancement d'un outil dont le titre contient `bethini`, Eidos garde un point de
restauration de ces INI ; **Sauvegardes** dans la barre d'outils, puis **INI**, en remet
un. Un INI qui n'existait pas au moment de la copie reste tel quel. La boîte de
dialogue garde les 10 points de restauration les plus récents de chaque sorte, et en
restaurer un sauvegarde d'abord les INI tels qu'ils sont, si bien qu'un mauvais choix
peut être annulé.

## Les réglages propres à un outil restent les siens

Eidos ne vérifie pas la plupart des réglages d'un outil ; un mauvais réglage échoue
en général sans erreur.

Le **Game Data Path** de BodySlide (Settings) doit pointer sur le répertoire `Data` du
jeu, pas sur le dossier du jeu au-dessus. Réglé un niveau trop haut, un batch build
annonce « All sets processed successfully » et écrit 1439 meshes là où le jeu n'ira
jamais les chercher. Eidos les rattrape dans `Overwrite/Root/`, si bien que votre
installation du jeu n'est pas touchée, et le seul signe dans le jeu est que vos corps
ne sont pas construits.

Avant une exécution de BodySlide ou d'Outfit Studio, la ligne d'état prévient quand
`GameDataPath` ou `OutputDataPath` dans le `CalienteTools/BodySlide/Config.xml` qui
l'emporte ne mène pas au dossier `Data` du jeu pendant l'exécution, et l'onglet
Diagnostics affiche le même avertissement sous forme de carte avec la valeur à
indiquer. `eidos check <instance>` le signale aussi. Un chemin en `S:` est lu comme
Eidos fait correspondre `S:` pour un outil, au dossier de la bibliothèque Steam. Un
`OutputDataPath` dans le dossier `mods/` de l'instance, pour un mod de sortie, est
accepté. Diagnostics affiche aussi une carte quand `Overwrite/Root/` contient un
dossier `meshes` ou `textures`.

La sortie des outils a sa place dans l'Overwrite. Quand une exécution produit quelque
chose qui vaut la peine d'être gardé, **Overwrite -> Créer un mod...** en fait un mod
ordinaire, qu'on peut ordonner, désactiver et supprimer comme n'importe quel autre.
