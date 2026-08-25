<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# Outils : xEdit, BodySlide, DynDOLOD, FNIS

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
- le **tools folder** que vous avez défini dans Settings (Tools -> Tools
  folder), pour le répertoire partagé entre instances - `/mnt/Games/Tools` et
  consorts.

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
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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

## Les DLL dont un outil a besoin sont choisies par son NOM

C'est la partie surprenante, alors autant le dire clairement : **le titre que
vous donnez à un outil décide des prérequis runtime qu'Eidos lui provisionne.**
La correspondance est une sous-chaîne du titre, insensible à la casse.

| Si le titre contient | Eidos demande |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| n'importe quoi d'autre | rien |

Ainsi un outil enregistré sous **`BodySlide`** reçoit ses DLL DirectX ; le même
exécutable enregistré sous **`BS`** ne reçoit rien et peut échouer au démarrage
avec une erreur qui ne dit rien des DLL. Nommez les outils d'après le programme.

La liste est dans `default_prereqs` (`crates/eidos-instance/src/tools.rs`), et le
champ `Prereqs` de la boîte de dialogue Executables est modifiable - la détection
est un défaut, pas une règle.

### Trois sortes de prérequis

**Palier 1 - DLL fournies** (`d3dx9_43`, `d3dcompiler_47`, `d3dx11_43`). Eidos
les embarque et les copie dans le préfixe au lancement. Rien à faire, pas de
réseau.

**Palier 2 - verbes winetricks** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`,
`dotnet48`, `xact`...). Ceux-là écrivent des clés de registre, le GAC et les
hôtes CLR, ils ne peuvent donc pas être copiés comme des fichiers. Ils
**téléchargent depuis Microsoft**.

**Palier 3 - runtimes** (`dotnet10`). Un runtime .NET moderne, c'est 193 fichiers
qui vivent dans leur propre répertoire et sont trouvés via `DOTNET_ROOT` : jamais
enregistré, jamais installé dans le préfixe du tout, si bien qu'aucun des deux
autres paliers ne peut le porter. Eidos le télécharge lui-même, le vérifie contre
une somme de contrôle intégrée au binaire, et le met en cache dans
`~/.local/share/Colony/Eidos/runtimes/` - **hors de toute instance**, parce que
78 Mo, ce n'est ni par jeu ni par profil.

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
