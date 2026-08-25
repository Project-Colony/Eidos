<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# Utiliser Eidos

Le manuel pratique : la ligne de commande, la GUI, l'option de lancement Steam,
la compilation depuis les sources, et le script de preuve de concept. Pour savoir
quoi faire quand quelque chose semble anormal, voir
[troubleshooting.fr.md](troubleshooting.md).

## L'utiliser (CLI)

```sh
eidos games                       # les jeux supportés installés ici (comme la liste de MO2)
eidos init skyrimse               # créer une instance de modding
# ...déposez chaque mod comme un dossier dans <instance>/mods/ (l'instance globale
#    vit dans ~/.local/share/eidos/skyrimse ; `eidos init` affiche la vôtre)...
eidos install skyrimse mod.7z     # ou installer une archive téléchargée (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopter l'ordre et l'état des plugins d'un profil MO2 existant
eidos sort skyrimse               # trier l'ordre de chargement des plugins avec LOOT
eidos play skyrimse               # afficher ce qui serait monté
eidos play skyrimse -- <command>  # exécuter <command> avec les mods montés sur le jeu
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` et `eidos export`
complètent l'ensemble ; lancez `eidos` sans argument pour la liste complète.

### Instances : globale et portable

Chaque commande ci-dessus s'adresse à une instance. `skyrimse` désigne
l'instance **globale** - stockée centralement dans
`~/.local/share/eidos/skyrimse`, gérée par Eidos. L'autre sorte est
**portable** : un dossier autonome là où vous le voulez (un second disque, une
partition de jeux), déplaçable et isolé, exactement comme les instances portables
de MO2. Partout où une commande accepte un identifiant de jeu, elle accepte aussi
le dossier d'une instance portable :

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # créer une instance portable ici
eidos install /mnt/games/EidosSkyrim mod.7z  # chaque commande accepte le dossier
eidos play /mnt/games/EidosSkyrim -- %command%
```

Le dossier se décrit lui-même (son `eidos-instance.ini` nomme le jeu), rien
d'autre n'est donc nécessaire - et `EIDOS_INSTANCE=<folder>` dans
l'environnement redirige un identifiant de jeu vers ce dossier, ce qui est
pratique dans les options de lancement Steam. Les instances portables que vous
avez créées ou ouvertes sont mémorisées (la plus récemment utilisée en premier)
dans `~/.config/Colony/Eidos/instances.ini` ; l'écran d'accueil de la GUI les
liste pour les ouvrir en un clic, le lancement Steam retombe sur celle à laquelle
vous avez joué en dernier, et le gestionnaire `nxm://` télécharge dedans. Deux
réserves à connaître : déplacer un dossier portable conserve tout, sauf les
entrées d'outils que vous avez enregistrées avec des chemins absolus vers
l'ancien emplacement (à ré-ajouter), et le cache de runtimes partagé
(`~/.local/share/Colony/Eidos/runtimes/`) reste délibérément global à la
machine - un hôte .NET de 78 Mo n'est pas par instance.

Eidos range ses propres fichiers sous `Colony/Eidos`, la disposition qu'utilise
chaque programme de la famille Colony : `~/.config/Colony/Eidos/` pour ce que
vous avez choisi (préférences, votre session Nexus, votre liste d'instances, les
définitions de jeux et d'add-ons que vous avez écrites),
`~/.local/state/Colony/Eidos/logs/` pour les logs de session, et
`~/.local/share/Colony/Eidos/` pour ce qu'Eidos a téléchargé. Un Eidos plus
ancien gardait tout cela dans `~/.config/eidos/` et `~/.local/state/eidos/` ; le
premier lancement après la mise à jour les **copie** et le dit dans le log. Les
anciens répertoires sont laissés exactement tels quels - rien n'est supprimé, une
mauvaise mise à jour ne peut donc pas vous coûter une session ouverte - et vous
pouvez les supprimer vous-même une fois rassuré.

Vos mods ne font pas partie de tout cela. Une instance globale vit toujours dans
`~/.local/share/eidos/<game>/`, et une portable là où vous l'avez mise, parce que
ces chemins sont inscrits dans votre liste d'instances et peut-être dans une
option de lancement Steam : les déplacer casserait un lien dont Eidos ne possède
pas les deux bouts.

Un endroit est refusé net : **à l'intérieur du dossier d'installation d'un jeu**
(le réflexe du vétéran de MO2). Steam est propriétaire de cette arborescence -
une mise à jour, une « vérification de l'intégrité » ou une désinstallation
peuvent la réécrire ou la supprimer, emportant toute votre installation avec
elle - et Eidos monte par-dessus la racine du jeu, si bien qu'une instance placée
là se retrouverait à l'intérieur de sa propre cible de montage. L'assistant,
`eidos init` et `eidos play` disent tous non ; mettez plutôt le dossier À CÔTÉ du
jeu (un voisin sur le même disque vous donne la même commodité).

`play` monte les mods de l'instance par-dessus le répertoire `Data` du jeu
lui-même (via un bind-stash, si bien que le démon lit toujours les fichiers
d'origine) dans un espace de noms privé, puis exécute la commande à travers cette
vue. Les écritures (sauvegardes, configurations régénérées) atterrissent dans la
couche `overwrite/` de l'instance ; l'installation du jeu et chaque source de mod
restent intactes, octet pour octet.

### Aucune étape privilégiée n'est nécessaire

Eidos s'exécute entièrement sans root. Il monte dans un espace de noms
utilisateur + montage privé, donc pas d'assistant setuid, pas de démon, et rien à
autoriser.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` est **facultatif** et ne
conditionne qu'une seule chose : le passthrough FUSE du noyau, désactivé par
défaut parce qu'il casse le jeu (voir plus bas). Avec cette capability, Eidos
prend un simple espace de noms de montage au lieu d'un espace de noms
utilisateur ; le déploiement des mods est identique dans les deux cas.


Pourquoi l'ancien conseil `setcap` a disparu - et pourquoi le passthrough FUSE
est livré désactivé - est expliqué dans
[troubleshooting.fr.md](troubleshooting.md#pourquoi-le-passthrough-est-désactivé-par-défaut).

## GUI

```sh
cargo run -p eidos-gui
```

Un assistant de premier lancement à la MO2, dans le look parchemin / bordeaux de
Colony : accueil -> type d'instance (portable / globale) -> jeu -> nom et
emplacement -> résumé -> création -> écran principal. L'écran d'accueil liste
aussi toutes les instances existantes connues (globales et portables, la dernière
utilisée en premier) pour les ouvrir en un clic - il fait aussi office de
sélecteur d'instance - et pointer l'assistant sur un dossier qui contient déjà
une instance l'ADOPTE telle quelle au lieu de créer par-dessus (avec un refus net
si le dossier appartient à un autre jeu).

La fenêtre principale à deux volets est faite elle aussi : un sélecteur de profil
(changer, ou en créer un nouveau en copiant l'actuel), une liste de mods que vous
filtrez, sélectionnez, réordonnez, groupez avec des séparateurs, restreignez par
catégorie et dont le clic droit ouvre les actions, plus les onglets Data /
Plugins / Conflicts / Overwrite / Saves / Downloads / Diagnostics et un bouton
Run avec un sélecteur de cible.

Réordonner ne se limite pas à envoyer tout en haut ou tout en bas : les
déplacements ciblés de MO2 sont là aussi - envoyer au-dessus du premier mod en
conflit, au-dessous du dernier, à une priorité explicite, ou dans le groupe d'un
séparateur. Ils passent tous par un même helper de déplacement, si bien que le
décalage d'un cran qui vient du retrait des lignes avant leur réinsertion
n'existe qu'à un seul endroit au lieu de cinq.

### Colonnes, tri et regroupement

La liste dessine quatre colonnes d'origine et en propose huit : Category,
Content, Version, Author, Installed, Nexus id, Game, Flags. Cochez-les dans le
menu View. Le défaut n'est pas les huit, et c'est délibéré - une liste dont
toutes les colonnes sont affichées n'a plus de place pour le NOM, qui est la
colonne que vous lisez réellement.

Cliquez sur n'importe quel en-tête pour trier dessus. Un deuxième clic inverse,
et un troisième revient à l'**ordre de chargement**, ce qui compte plus qu'il n'y
paraît : l'ordre de chargement est le seul ordre dans lequel la liste peut être
déplacée à la souris, parce qu'un intervalle d'insertion s'adresse à la vraie
liste alors qu'une ligne triée est ailleurs, tout simplement. Tant qu'un tri est
actif, les bandes d'insertion ne sont pas dessinées et un glisser-déposer est
refusé plutôt que d'atterrir là où personne ne visait - la même chose que fait
MO2, et pour la même raison. Le menu View le dit et propose le chemin du retour.

Le menu View peut aussi **grouper** toute la liste, par catégorie ou par source
(venant de Nexus, ou installé à la main). Les en-têtes de groupe ne sont pas des
séparateurs : il n'y a rien derrière eux à renommer, colorer ou déplacer, ils se
replient, et le compte reste sur l'en-tête une fois replié. Les séparateurs
quittent la liste sous un tri ou un regroupement - un séparateur coiffe les
lignes qui le suivent dans l'ordre de chargement, et l'un comme l'autre les ont
déplacées.

### Souris et clavier

Double-cliquez sur un mod pour Information, Ctrl+double-clic pour son dossier,
Shift+double-clic pour sa page Nexus. Ctrl+F place le curseur dans le champ de
filtre. Taper une lettre saute au mod suivant qui commence par elle, et la
retaper parcourt les autres au lieu de rester sur le premier. Aucun d'eux ne peut
atterrir sur une ligne que le filtre, un séparateur replié ou un groupe replié
cache - déplacer une surbrillance que vous ne voyez pas, c'est ainsi que le Space
suivant active un mod que vous ne regardiez pas.

« Collapse others » dans le menu d'un séparateur replie tous les groupes sauf
celui-là. Pendant un glisser-déposer, s'arrêter sur un groupe replié l'ouvre, si
bien qu'un mod peut être déposé dedans sans abandonner le glissement d'abord -
s'arrêter, pas passer dessus.

### Ce que la liste vous dit d'un mod

Deux drapeaux consultatifs, chacun un glyphe avec l'explication au survol. **No
valid game data** veut dire que rien à la racine du mod ne ressemble à quelque
chose que ce jeu charge ; il faut peut-être remonter ses dossiers d'un niveau, ou
ce n'est peut-être pas un mod pour ce jeu. **Another game** veut dire que le
`meta.ini` du mod lui-même en nomme un autre. Aucun des deux ne bloque quoi que
ce soit - le mod se déploie quand même - et « Mark as valid » dans le menu de la
ligne fait taire l'un comme l'autre, via la clé `validated=` de MO2, si bien
qu'un mod dont vous vous êtes porté garant dans un gestionnaire arrive silencieux
dans l'autre.

La vérification de la disposition est délibérément généreuse : une arborescence
`Root/` compte, un dossier illisible compte, un dossier vide compte. Un
avertissement erroné sur une liste de cinq cents lignes est pire qu'un
avertissement manquant.

### Sauvegarder un mod avant d'y toucher

« Back up this mod » copie son dossier de côté sous le nom `<name>_backup` (puis
`_backup2`, et ainsi de suite - une sauvegarde ne remplace jamais la précédente).
La copie est **inerte** : ce n'est pas un mod, sa case à cocher ne fait rien, et
elle ne contribue en rien à la vue fusionnée, parce que la cocher déploierait
deux copies d'un même mod l'une par-dessus l'autre. « Restore this backup over
the mod » la remet en place, en deux clics ; le contenu actuel est d'abord mis de
côté et n'est écarté qu'une fois la copie réussie.

**Data** est une vraie arborescence de la vue fusionnée, dépliée un niveau à la
fois, si bien qu'ouvrir un nœud coûte une lecture de répertoire par couche qui le
possède plutôt qu'un parcours récursif de tous les mods activés. Il est servi par
la MÊME pile de couches que celle depuis laquelle le montage sert, donc les
whiteouts et les fichiers masqués sont respectés et l'onglet ne peut pas
contredire ce que le jeu verra. Filtrez par nom, restreignez aux seuls fichiers
disputés, démêlez ce qui est où avec les colonnes Size et Modified, et faites
Reveal sur n'importe quelle ligne dans un gestionnaire de fichiers. **Plugins**,
c'est l'ordre de chargement des ESP/ESM/ESL (activer, réordonner à la main, ou
trier avec LOOT et lire le rapport d'après-tri, dont les liens de conseils
s'ouvrent dans votre navigateur). **Conflicts** explique les gagnants et les
perdants fichier par fichier. **Overwrite** transforme ce que le jeu a écrit en
un vrai mod en une seule étape. **Saves** analyse l'en-tête de chaque sauvegarde
- personnage, niveau, lieu, temps de jeu - et compare la liste de plugins qui y
est gravée à la vôtre, avec un bouton qui active les mods dont elle a besoin,
parce que les nommer et vous laisser faire le reste, c'est la moitié ennuyeuse.

« Information... » ouvre une boîte de dialogue par mod : général, conflits,
arborescence des fichiers, INI tweaks, notes. Depuis l'arborescence (et depuis
l'arbre Data), n'importe quel fichier peut être **masqué** - renommé en
`<name>.mohidden`, ce qui le sort de la vue virtuelle sans le supprimer, si bien
que les trois meshes égarés d'un mod peuvent être supprimés sans toucher aux
priorités. L'arborescence fait aussi les opérations de fichiers ordinaires :
nouveau dossier, renommer, supprimer, ouvrir. Elles passent toutes par un même
résolveur qui refuse tout ce qui n'est pas un chemin simple à l'intérieur de ce
mod - pas de `..`, pas de chemin absolu, et aucun composant qui soit un lien
symbolique, puisqu'en suivre un placerait une suppression entièrement hors du
dossier du mod. Renommer ne remplace que le dernier composant, si bien que cela
ne peut jamais devenir un déplacement, et refuse un nom déjà pris plutôt que de
remplacer ce fichier en silence. Supprimer demande deux clics ; c'est la seule
action ici qu'un nouveau clic ne peut pas annuler.

**View** sur n'importe quelle ligne de l'arborescence ou de l'arbre Data
prévisualise le fichier : images et texte. Ni DDS ni NIF - il leur faut un
décodeur de blocs et un moteur de rendu que cet arbre n'a pas - mais ils le
disent au lieu d'afficher une case vide, et renvoient vers Reveal. Le texte est
lu jusqu'à 64 Ko et dit où il s'est arrêté, parce qu'une prévisualisation est un
coup d'œil et qu'un log Papyrus peut faire cent mégaoctets. **INI Tweaks** liste
les fragments qu'un mod livre dans son dossier `INI Tweaks/` ; ceux qui sont
activés sont fusionnés dans l'INI de jeu du profil au lancement, dans l'ordre de
priorité, et retirés quand les INI de l'exécution sont capturés - sinon un tweak
deviendrait silencieusement un réglage et le désactiver ne ferait rien.

Un téléchargement peut être **glissé depuis la liste Downloads jusqu'à une
position dans la liste de mods** pour l'installer à cette priorité, et les
archives ou dossiers déposés sur la fenêtre depuis un gestionnaire de fichiers
s'installent aussi (cette moitié-là exige une session X11 ou XWayland - winit
n'implémente le dépôt de fichiers que pour X11). Les téléchargements eux-mêmes
peuvent être mis en pause et repris : la pause arrête le transfert et conserve le
fichier partiel, et Resume résout un nouveau lien et continue là où il s'était
arrêté.

L'onglet Downloads est une **bibliothèque** d'archives, pas une file de
transferts. Filtrez par nom (le nom lisible du mod aussi, si bien que « skyui »
trouve `SkyUI_5_2_SE-12604-5-2SE.7z`), triez par date, nom, taille ou état, et
**masquez** une archive dont vous avez fini - ce qui conserve le fichier et ne
retire que la ligne, si bien que ranger un livre n'est pas le brûler. « Show
hidden » les fait revenir, et le même bouton les démasque. « Remove N installed »
supprime les archives des mods que vous avez déjà installés, en deux clics, et
seulement celles **à l'écran** : le filtre, c'est votre façon d'avoir dit
lesquelles vous vouliez dire.

### Collections Nexus

Collez un lien de collection - ou cliquez-en un sur le site - et Eidos liste les
membres de la révision, chacun rapproché de cette instance : installé,
téléchargé, ou manquant. Il **lit** une collection ; il n'en installe pas, et le
panneau le dit. Quatre choses rendent ici un installeur malhonnête plutôt que
seulement difficile : les membres sont des fichiers Nexus ordinaires qui exigent
une clé par fichier que seul un compte premium peut émettre en dehors du bouton
du site lui-même ; une installation complète, c'est trois appels d'API par membre
contre un budget que ce client refuse de dépasser ; les phases, les règles et les
réponses FOMOD rejouées du manifeste n'ont pas pu être vérifiées contre une vraie
collection Bethesda publiée, et deviner produit un ordre de chargement qui a
l'air juste et ne l'est pas. Lire coûte une requête et est exact.

Une collection ne peut être lue que contre **son propre jeu**. Ouvrez une
collection Skyrim avec une instance Fallout 4 chargée et il refuse en nommant le
jeu plutôt que de rapprocher les membres de la mauvaise liste de mods, où chaque
« installé » et chaque « manquant » serait du bruit déguisé en réponse.

### Mode hors ligne

**Settings -> Nexus -> Offline** empêche complètement Eidos de contacter Nexus.
Les vérifications de mise à jour, la connexion, les téléchargements et les
collections le disent au lieu d'échouer sur une erreur de connexion. C'est
désactivé sauf si vous l'activez - un fichier de réglages écrit par un Eidos plus
ancien n'a pas cette clé, et lire une clé absente comme « activée » couperait le
réseau à tous ceux qui mettent à jour.

**Preferred servers** classe les nœuds CDN qu'un téléchargement préfère, le
meilleur en premier. Seul un compte premium se voit jamais proposer plus d'un
miroir au choix, donc pour tous les autres c'est Nexus qui choisit et ceci ne
change rien. C'est un ordre, pas un filtre : si rien de ce que vous avez nommé
n'est proposé aujourd'hui, le téléchargement a quand même lieu, depuis le nœud
que Nexus a proposé en premier.

**Categories** sont modifiables, pas seulement affichées : assignez-les à un mod
ou à toute une sélection, modifiez le catalogue lui-même depuis la même boîte de
dialogue, et récupérez la liste officielle des catégories du jeu depuis Nexus.
Les deux fichiers de catalogue sont ceux de MO2 (`categories.dat` et
`nexuscatmap.dat`), si bien qu'une instance partagée garde un seul catalogue.

**View -> INI editor** modifie les INI de jeu du profil - la copie qui persiste,
plutôt que celle enfouie dans le préfixe Proton qui est écrasée à chaque
lancement. **View -> Log** lit les logs de session. **View -> Extensions** liste
vos propres add-ons ; voir [extensions.fr.md](extensions.md).

L'installation accepte tout : les chemins Simple et FOMOD, plus les paquets
**BAIN** de Wrye Bash (cochez les sous-paquets, qui fusionnent dans l'ordre) et
un sélecteur **manuel** qui montre l'arborescence de l'archive et vous laisse
désigner la racine des données quand aucune heuristique ne reconnaît la
disposition. Aucune archive n'est refusée.

**Diagnostics** exécute des contrôles de santé en direct : la capacité à lancer
avant tout, les masters manquants (le prédicteur de crash le plus fiable qui
soit), les archives qu'aucun plugin actif ne chargera, si la liste de mods
correspond toujours au dossier mods, et - après une exécution - ce que dit le log
du script extender lui-même sur chacune de ses DLL de plugins, ce qui fait passer
« est-ce que mes plugins SKSE se sont chargés ? » de la déduction à la preuve.

Pour lancer le jeu depuis la GUI, réglez l'option de lancement Steam du jeu sur
le chemin absolu du binaire (Steam ne voit pas `~/.cargo/bin` dans le PATH) :

```
~/.cargo/bin/eidos-gui %command%
```

Eidos s'ouvre sur l'instance de ce jeu - celle que vous avez utilisée en dernier,
si bien qu'une instance portable est retrouvée tout comme la globale ; cliquez
sur Run pour le lancer à travers la vue fusionnée. (Le bouton Run affiche
exactement cette ligne, avec le vrai chemin du binaire en cours d'exécution, si
vous l'actionnez hors de Steam.)

Le `%command%` de Steam pour les titres Bethesda pointe le plus souvent sur
`<Game>Launcher.exe`. Eidos ne l'exécute jamais : le launcher est une application
de réglages séparée qui rescanne `Data` et réécrit `plugins.txt`, défaisant
l'ordre de chargement qui vient d'être déployé. Il met à la place le loader du
script extender si l'un est installé, le binaire du jeu sinon, et le dit quand il
doit se rabattre - un jeu qui démarre avec tous les mods SKSE inertes est pire
qu'un jeu qui ne démarre pas.

D'anciennes instructions ici imposaient
`WINEDLLOVERRIDES="d3dcompiler_47=n"`. Ce n'est plus nécessaire et ce ne fut
jamais tout à fait juste : un override vers *native* n'aide que si un vrai
`d3dcompiler_47.dll` est déjà dans le préfixe. Eidos analyse maintenant les
imports de DLL des mods activés, déploie lui-même la vraie DLL Microsoft, et ne
définit l'override qu'ensuite.

## Essayer la preuve de concept

Aucun jeu requis. Elle prouve l'union + le copy-on-write + le zéro-touche + la
portée par espace de noms en n'utilisant qu'OverlayFS non privilégié dans un
espace de noms utilisateur (Linux >= 5.11) :

```sh
./scripts/poc-overlay.sh
```

## Outils

xEdit, BodySlide, DynDOLOD et consorts s'exécutent à travers la vue fusionnée, à
l'intérieur du préfixe Proton du jeu :

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # ce dont les outils enregistrés ont besoin, et son état
eidos prereqs skyrimse --install  # récupérer ce qui manque
```

Une chose à savoir avant de nommer un outil : **le titre décide des DLL runtime
qu'Eidos lui provisionne** - `BodySlide` reçoit ses bibliothèques DirectX, `BS`
ne reçoit rien. Dans la GUI, la boîte de dialogue Executables montre l'état réel
de chaque prérequis sous le champ, et ceux qui manquent sont des boutons.

Le tableau, les trois paliers de prérequis, pourquoi DynDOLOD a besoin d'un
runtime .NET que winetricks ne peut pas installer, et pourquoi un outil installé
comme un mod est lancé depuis le chemin fusionné plutôt que depuis son propre
dossier sont dans [tools.fr.md](tools.md).

La compilation depuis les sources et la disposition du dépôt sont dans
[../internals/contributing.md](../../../../internals/contributing.md).

## Extensions

Eidos peut être étendu sans être recompilé : un manifeste TOML dans
`~/.config/Colony/Eidos/addons/` ajoute un outil à la liste Extensions ou un
contrôle à l'onglet Health. Rien n'est chargé dans Eidos - une extension est un
programme qu'il exécute. Voir [extensions.fr.md](extensions.md).
