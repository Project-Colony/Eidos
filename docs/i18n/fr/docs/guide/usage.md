<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# Utiliser Eidos

Comment utiliser Eidos : les instances et les copies du jeu, la ligne de commande,
les parties test, la fenêtre, les collections et les mises à jour Nexus, déplacer une
instance vers une autre machine, les outils et les extensions. Pour les problèmes,
voir [troubleshooting.md](troubleshooting.md) ; pour compiler depuis les sources et
la disposition du dépôt, voir
[../internals/contributing.md](../../../../internals/contributing.md).

## Instances : globale et portable

Les commandes qui prennent une instance acceptent un identifiant de jeu ou le dossier
d'une instance portable. `skyrimse` désigne l'instance globale, qu'Eidos garde dans
`~/.local/share/Colony/Eidos/instances/skyrimse`. Une instance portable est un dossier
autonome où vous voulez (un second disque, une partition de jeux), et vous pouvez le
déplacer, comme les instances portables de MO2 :

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # créer une instance portable à cet endroit
eidos install /mnt/games/EidosSkyrim mod.7z  # toute commande accepte le dossier
eidos play /mnt/games/EidosSkyrim -- %command%
```

Le `eidos-instance.ini` du dossier nomme le jeu, donc le dossier n'a besoin de rien
d'autre. `EIDOS_INSTANCE=<folder>` dans l'environnement redirige un identifiant de jeu
vers ce dossier, par exemple dans une option de lancement Steam. Les instances
portables que vous avez créées ou ouvertes sont mémorisées, la plus récente d'abord,
dans `~/.config/Colony/Eidos/instances.ini` : l'écran d'accueil de la fenêtre les
liste, le lancement Steam arrive sur celle que vous avez jouée en dernier, et le
gestionnaire `nxm://` télécharge dedans.

Déplacer un dossier portable à la main garde tout sauf les entrées d'outils que vous
avez enregistrées avec des chemins absolus vers l'ancien emplacement. `eidos pack` et
`eidos unpack` ([plus bas](#déplacer-une-instance-vers-une-autre-machine)) les corrigent ; un
simple `mv` non. Le cache de runtimes dans `~/.local/share/Colony/Eidos/runtimes/` est
partagé par toutes les instances de la machine, si bien qu'un hôte .NET de 78 Mo n'est
pas copié dans chacune.

Un endroit est refusé : l'intérieur du dossier d'installation d'un jeu, où les
utilisateurs de MO2 mettent souvent leur instance. Steam possède cette arborescence, et
une mise à jour, une « vérification d'intégrité » ou une désinstallation peut la
réécrire ou la supprimer avec votre configuration. Eidos monte aussi sur la racine du
jeu, si bien qu'une instance placée là serait à l'intérieur de sa propre cible de
montage. L'assistant, `eidos init`, `eidos play` et `eidos shell` le refusent. Mettez
plutôt le dossier à côté du jeu ; un dossier voisin sur le même disque est tout aussi
commode.

### Où Eidos range ses fichiers

Eidos range ses propres fichiers sous `Colony/Eidos`, la disposition que suit chaque
programme de la famille Colony : `~/.config/Colony/Eidos/` pour ce que vous avez choisi
(préférences, votre session Nexus, votre liste d'instances, les définitions de jeux et
d'extensions que vous avez écrites), `~/.local/share/Colony/Eidos/` pour les instances
globales (`instances/`), les runtimes téléchargés et les journaux de session
(`logs/`), et `~/.cache/Colony/Eidos/` pour les masterlists LOOT, qui sont
retéléchargées chaque fois qu'elles manquent.

Quand il trouve des fichiers dans la disposition d'un Eidos plus ancien
(`~/.config/eidos/`, `~/.local/share/eidos/` et `~/.local/state/`), le premier
lancement les copie ailleurs et le dit dans le journal. Une instance globale est
déplacée avec des liens physiques au lieu d'une copie octet par octet, si bien que ses
mods ne prennent pas de place en plus, et ses entrées d'outils sont repointées vers le
nouveau dossier. Rien dans les anciens dossiers n'est modifié ni supprimé ; retirez-les
vous-même une fois la nouvelle disposition en service. Si une copie ne peut pas être
faite (un disque plein ou en lecture seule, un ancien dossier d'instance sur un autre
disque, un jeu encore en cours depuis lui), Eidos continue d'utiliser l'ancien
emplacement et réessaie au lancement suivant. Une option de lancement Steam qui nomme
encore l'ancien dossier d'instance ouvre celui qui a été déplacé.

Les liens physiques n'économisent de la place qu'au début. Un mod ou un téléchargement
que vous retirez ou réinstallez ensuite garde son ancienne copie sur le disque par
l'ancien dossier d'instance, et supprimer `~/.local/share/eidos/<game>/` rend cette
place.

Une instance portable n'est jamais déplacée : elle reste où vous l'avez mise.

## La ligne de commande

```sh
eidos games                                # jeux pris en charge installés ici, comme la liste de MO2
eidos init skyrimse [folder]               # créer une instance ; avec un dossier, elle est portable
eidos init skyrimse --game-path <folder>   # utiliser la copie installée dans <folder> quand plusieurs sont installées
eidos install skyrimse mod.7z [name]       # installer une archive (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # réinstaller par-dessus le mod (--merge installe par-dessus ses fichiers)
eidos import skyrimse <mo2-profile>        # reprendre l'ordre des mods et l'état des plugins d'un profil MO2
eidos sort skyrimse [--dry-run]            # trier l'ordre de chargement avec LOOT (--update-masterlist la met d'abord à jour)
eidos play skyrimse                        # montrer ce qui serait monté
eidos play skyrimse -- <command>           # exécuter <command> avec les mods montés sur le jeu
eidos shell skyrimse                       # un shell dans la vue fusionnée ; les écritures vont dans Overwrite
eidos check skyrimse [--json]              # contrôles du diagnostic sans la fenêtre
eidos sessions skyrimse                    # les exécutions de jeu et d'outil enregistrées, la plus récente d'abord
eidos sessions skyrimse diff [<id>]        # ce qui a changé depuis la dernière exécution terminée avec le code 0
eidos stock-game skyrimse                  # quels fichiers du jeu l'instance exécute ; figer une copie ou basculer
eidos tool skyrimse                        # lister les outils (add, rm et run : voir Outils plus bas)
eidos tool skyrimse run --exe <path>       # lancer un exe une fois à travers la vue fusionnée
eidos tool skyrimse import-mo2 [--write]   # les exécutables d'un ModOrganizer.ini ; --write les ajoute
eidos prereqs skyrimse [--install]         # ce dont les outils ont besoin ; --install récupère ce qui manque
eidos collection skyrimse <link>           # installer une collection Nexus (--dry-run, --no-optional)
eidos nexus status                         # vérifier la connexion Nexus enregistrée
eidos nexus update skyrimse [--apply]      # chercher les mises à jour de mods ; --apply les installe
eidos nxm <link>                           # télécharger un lien Mod Manager Download
eidos nxm --register                       # faire ouvrir Eidos par le bouton Mod Manager Download du site
eidos export skyrimse -o mods.csv --active # format CSV de MO2 ; --active ne liste que les mods activés
eidos pack skyrimse backup.eidos           # toute l'instance dans un seul fichier
eidos unpack backup.eidos [folder]         # sans dossier, une instance globale retourne à sa place centrale
eidos --version                            # afficher la version
```

`eidos` sans argument affiche la liste des commandes. La plupart des commandes
affichent leurs propres options quand un argument manque, et `eidos tool --help` liste
les sous-commandes d'outil. `eidos pack` accepte `--dry-run`, `--level`,
`--no-downloads`, `--no-recovery` et `--force`, et `eidos unpack` accepte `--info` et
`--force` ; les deux sont décrits [plus bas](#déplacer-une-instance-vers-une-autre-machine).
`eidos stock-game` est décrit sous
[Fichiers du jeu](#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie).

### Lancer un jeu à travers la vue fusionnée

`eidos play` monte les mods de l'instance sur le répertoire `Data` du jeu dans un
espace de noms privé, puis exécute la commande à travers cette vue. Les propres
fichiers du jeu sont d'abord mis de côté par liaison, si bien que le démon les lit
toujours sans changement. Les écritures (sauvegardes, configurations régénérées)
atterrissent dans la couche `overwrite/` de l'instance ; l'installation du jeu et
chaque dossier de mod restent octet pour octet tels qu'ils étaient.

Eidos n'a pas besoin de root. Il monte dans un espace de noms privé d'utilisateur et de
montage, sans assistant setuid, sans service système et sans rien à accorder. Une
capacité facultative sur le binaire `eidos` autorise le passthrough FUSE, qui reste
désactivé tant que `EIDOS_FUSE_PASSTHROUGH=1` n'est pas défini ; voir
[install.md](install.md#facultatif--le-passthrough-fuse). Avec la capacité, Eidos utilise un
espace de noms de montage simple au lieu d'un espace de noms d'utilisateur, et les mods
se déploient de la même façon dans les deux.

### Un shell dans la vue fusionnée

`eidos shell <instance>` démarre votre `$SHELL` (ou `/bin/sh`) dans le dossier du jeu
tel que le jeu le verrait : chaque mod activé monté sur `Data`, les mods root sur la
racine du jeu, et tout ce qui y est écrit atterrissant dans l'Overwrite.
`eidos shell <instance> -- <command>` exécute une seule commande à la place, par
exemple `eidos shell skyrimse -- ls Data/meshes` ou un outil natif comme NifSkope.
L'exécution tient le verrou de l'instance comme une session de jeu, si bien que la
fenêtre ne peut pas enregistrer de modifications avant que vous quittiez, et elle est
enregistrée dans `eidos sessions` sous le nom `shell`, jamais comme un lancement du
jeu. Ctrl+C sur une commande arrête l'exécution comme le fait Arrêter : Eidos termine la
commande, capture la session et démonte. Dans le shell interactif, Ctrl+C appartient au
shell, comme d'habitude ; tapez `exit` pour terminer l'exécution.

### Contrôles de santé depuis le terminal

`eidos check <instance>` lance les contrôles du diagnostic qui n'ont pas besoin de fenêtre,
si bien qu'ils peuvent tourner par SSH ou en Game Mode et aller dans un rapport de
bug :

- le préfixe Proton et son registre ;
- les diagnostics de plugins et les masters manquants ;
- le script extender et ses plugins ;
- une mise à jour du jeu depuis le dernier lancement propre ;
- l'option de lancement Steam ;
- les réglages graphiques (NVAPI, plein écran, ENB) ;
- le journal de plantage de la dernière exécution ;
- les lignes `sTestFile` dans les INI du profil ;
- les plugins `.esp` et blueprint de Starfield ;
- `vm.max_map_count` et la limite de fichiers ouverts ;
- la copie du jeu que l'instance exécute, et un `gamePath` dans le `ModOrganizer.ini`
  d'une liste qui nomme un autre dossier de jeu ;
- les règles d'ordre de collection qui ne tiennent plus ;
- les fichiers du jeu sur NTFS, exFAT ou FAT ;
- les chemins de données de BodySlide ;
- les dossiers `Data` laissés dans `Overwrite/Root` ;
- les bibliothèques de polices que nomme `Interface/fontconfig.txt` et qu'aucun mod ne
  fournit ;
- l'audio compressé dans les archives BA2 des mods de Fallout 4 et Fallout 4 VR.

Certaines cartes ne sont calculées que par la fenêtre, dans l'onglet Diagnostics :
l'ordre des patchs, les scripts de l'extender qu'un mod remplace, les archives que rien
ne charge, les prérequis Nexus, les plugins que LOOT signale comme sales, le build de
Proton qui exécute le jeu, la carte Display et la sortie d'outil plus ancienne que ses
entrées.

La commande affiche les mêmes lignes système que **Copier le rapport**, puis une ligne par
constat, avec votre dossier personnel écrit `~` et votre nom de connexion `<user>`. Le
code de sortie est 0 quand aucun problème n'a été trouvé (un simple conseil donne
quand même 0), 1 quand au moins un problème a été trouvé, et 2 quand Eidos n'a pas pu
vérifier : mauvais arguments, une instance qu'il ne peut pas ouvrir, ou un jeu qu'il ne
détecte pas.

`eidos check <instance> --json` affiche un seul objet JSON à la place. Ses champs de
premier niveau sont `eidos` (la version), `game`, `instance`, `system` (`proton`,
`flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`,
`fusermount3`, `max_map_count`, `open_files_hard`) et `findings`. `max_map_count` et
`open_files_hard` sont des nombres, ou `null` quand Eidos n'a pas pu les lire. Chaque
constat est un `code`, un `level` (`problem` ou `advice`) et ses `args` ; l'onglet
Diagnostics montre la phrase de chaque code.

### Ce qui a changé depuis le dernier bon lancement

Chaque exécution par `eidos play`, `eidos shell` ou `eidos tool run`, lancée depuis la
fenêtre, un terminal ou Steam, écrit une fiche dans `<instance>/logs/sessions/` : le
profil, chaque mod activé avec sa version et ses identifiants de fichier Nexus, l'ordre
des plugins, une somme de contrôle de chaque INI du profil, la version de l'exécutable
du jeu, la version d'Eidos, le code de sortie et les sauvegardes créées par
l'exécution. Les 50 plus récentes sont gardées, et `logs/` reste en dehors d'une
instance empaquetée. `eidos sessions <instance> diff` compare l'exécution de jeu la
plus récente terminée avec le code 0 à l'état actuel ; `diff <id>` choisit une autre
fiche dans la liste. La liste marque une [partie test](#parties-test) comme `test`, et une partie
test n'est jamais choisi comme dernier bon lancement. Une exécution que vous avez
arrêtée enregistre le code de sortie 143, donc elle n'est jamais choisie non plus.
Eidos ne détecte pas encore les plantages ici, et un jeu qui a planté sous Proton peut
quand même se terminer avec le code 0 : si l'exécution choisie était de celles-là,
nommez-en une plus ancienne avec `diff <id>`.

## La liste de mods

Comme dans MO2, chaque profil a son propre `modlist.txt` (quels mods sont activés, avec
quelle priorité), tandis que les dossiers de mods vivent dans le `mods/` partagé de
l'instance. Un dossier que vous déposez à la main dans `mods/` n'est pas encore listé :
il apparaît désactivé à la priorité la plus haute, en bas de la liste de la fenêtre
comme dans MO2, si bien qu'il gagne ses conflits une fois activé.

Parce que les dossiers sont partagés, retirer ou renommer un mod change tous les
profils : sa ligne est retirée du `modlist.txt` de chaque profil, ou renommée dedans, et
chaque profil garde son propre état et son propre ordre pour le reste. Un profil sans
mods n'a pas de `modlist.txt` du tout.

La fenêtre travaille sur sa propre copie de la liste de mods et de l'ordre de
chargement. Si un autre processus Eidos écrit l'une ou l'autre pendant qu'elle est
ouverte (`eidos install` ou `eidos sort` dans un terminal, une collection, une session
que Steam a lancée, une seconde fenêtre), la modification suivante de la fenêtre est
refusée au lieu d'écraser ce travail. Elle recharge depuis le disque et dit *Non enregistré : un autre
processus Eidos a modifié la liste des mods. Elle a été rechargée, refaites la modification.*
(ou la même chose pour l'ordre de chargement) ; seul ce clic est perdu, refaites-le
donc. Renommer, Nouveau mod vide, Ajouter un séparateur au-dessus, Supprimer et transformer l'Overwrite en mod
vérifient avant de toucher à `mods/`, si bien qu'une modification refusée ne change rien
sur le disque. L'éditeur d'INI refuse de même d'enregistrer par-dessus un fichier qui a
changé depuis qu'il l'a ouvert.

## Les sauvegardes appartiennent au profil

Chaque profil garde ses propres sauvegardes dans `profiles/<name>/saves/`, et pendant
la durée d'une exécution ce dossier est monté sur le dossier de sauvegardes du jeu dans
le préfixe Proton. Le premier lancement d'un profil copie dedans les sauvegardes déjà
présentes dans le préfixe, une seule fois, si bien qu'une sauvegarde que vous supprimez
ensuite du profil reste supprimée. Après chaque exécution, les sauvegardes les plus
récentes du profil sont recopiées dans le préfixe pour Steam Cloud ; une sauvegarde du
préfixe qu'Eidos n'y a pas mise est d'abord gardée dans le profil sous le nom
`orphan-<time>-<name>`, jamais écrasée.

Une sauvegarde qui arrive dans le préfixe après cette première copie sans passer par
Eidos (un téléchargement Steam Cloud depuis un Steam Deck, un lancement sans Eidos)
reste cachée tant que les sauvegardes du profil sont montées. Rien ne la supprime : le
lancement la nomme dans le journal de l'exécution (le `logs/run-*.log` de l'instance
pour un lancement depuis la fenêtre, la console de Steam pour un lancement Steam), une
fois chaque fois que cet ensemble de fichiers change, et l'onglet Sauvegardes compte ces
sauvegardes sur la ligne sous son titre. Pour en jouer une, appuyez sur Importer dans
l'onglet Sauvegardes. Importer copie chacune avec son cosave dans le profil et ne remplace
jamais une sauvegarde du profil : une sauvegarde dont le nom est déjà pris par le
profil est importée sous le nom `orphan-<time>-<name>`.

## Parties test

Le bouton Partie test, à côté de Lancer, démarre le jeu comme le fait Lancer, mais tout ce que le
jeu écrit va dans `<instance>/test-run/` au lieu de votre profil, de l'Overwrite et du
préfixe : sauvegardes nouvelles et modifiées, modifications d'INI, `plugins.txt`,
fichiers écrits dans Data ou dans le dossier du jeu, et journaux dans
`Documents/My Games/<game>`. Vous pouvez tester avec les mods du profil, sans aucun mod,
ou avec seulement les mods sélectionnés qui sont activés, plus les mods dont leurs
plugins ont besoin comme masters.

Quand le jeu se termine, Eidos liste ces fichiers en cinq groupes (sauvegardes,
changements d'INI, état des plugins, sortie de l'Overwrite, journaux) et vous demande
de garder ou de jeter chacun. Appliquer attend que chaque groupe qui contient des fichiers
ait une réponse. L'état des plugins n'est pas proposé quand l'exécution a utilisé un
autre ensemble de mods que celui du profil. Garder les sauvegardes ne supprime jamais
une sauvegarde du profil, même une que le jeu a supprimée pendant l'exécution. Garder
la sortie de l'Overwrite retire aussi de l'Overwrite les fichiers que le jeu a
supprimés, et la liste les compte d'abord. Jeter quoi que ce soit demande un second
appui. Décider plus tard ferme la liste et laisse les fichiers dans `test-run/`, et la liste
se rouvre la prochaine fois que vous ouvrez l'instance ou appuyez sur F5. Si Eidos ou
l'ordinateur s'arrête pendant une partie test, les fichiers restent là aussi et la liste
s'ouvre de la même façon.

Quand la fenêtre n'a pas été lancée par la boutique du jeu, Partie test confie le jeu à
cette boutique (Steam, Heroic ou Legendary), comme le fait Lancer, et l'exécution que la
boutique démarre est la partie test. Si la boutique ne démarre pas le jeu, la demande
attend : la prochaine exécution du jeu de ce profil, depuis Lancer ou depuis la boutique,
est la partie test, et Annuler la partie test en attente l'abandonne. Une partie test ne couvre que
les dossiers listés ci-dessus : les autres écritures du jeu, comme le registre de Wine,
vont là où elles vont toujours. Les sauvegardes d'une partie test n'atteignent Steam Cloud
qu'après que vous les avez gardées et joué une exécution normale.

### Trouver le mod à l'origine d'un problème

Trouver le mod à l'origine d'un problème, dans l'onglet Diagnostics, cherche le mod qui cause un
problème que vous pouvez reproduire, comme un plantage à un endroit donné. Chaque étape
est une partie test avec la moitié des suspects restants, les masters dont ils ont besoin
et les mods que vous gardez toujours activés : les mods activés sélectionnés dans la
liste au moment du départ, et le mod qui fournit le script extender. Après chaque
exécution, vous répondez si le problème s'est produit. « Je ne sais pas » répartit les suspects
autrement, et Relancer répète l'étape. Répondre, ou appuyer sur Relancer, jette
les fichiers que l'étape a écrits, après un second appui. Pour en garder, appuyez
sur Voir les fichiers avant de répondre : cela ouvre la même liste qu'après une partie test. Quand le jeu
a planté et qu'un enregistreur de plantages a écrit un journal, ou que le jeu s'est
terminé avec un code d'erreur, Eidos marque « Oui, il s'est produit » comme la réponse probable. 200 suspects
demandent environ 8 exécutions.

La recherche est enregistrée dans `<instance>/bisect.json`, si bien que vous pouvez
fermer Eidos entre deux étapes, et elle appartient au profil dans lequel elle a
commencé. Arrêter la recherche l'abandonne après un second appui. Le résultat nomme le mod,
ou l'ensemble de mods qui ont besoin des masters les uns des autres, et il est gardé
dans `<instance>/logs/bisect-<time>.txt`. Les mods générés (une sortie de Nemesis ou de
DynDOLOD, un Bashed Patch) sont construits pour la liste complète et peuvent échouer
avec la moitié de celle-ci : gardez-les toujours activés, ou laissez-les désactivés
pendant toute la recherche.

## Listes Wabbajack

Eidos n'a pas d'installateur Wabbajack. Installez la liste avec CLF3 ou Jackify, hors
d'Eidos, puis choisissez le dossier d'installation comme instance portable dans
l'assistant. Eidos lit le dossier tel que MO2 le dispose : `mods/`,
`profiles/<name>/modlist.txt` et les listes de plugins. Il garde le `ModOrganizer.ini`
qu'a écrit l'installateur et ne l'écrase jamais, si bien que les exécutables propres à
la liste restent enregistrés.

Beaucoup de listes ajoutent un dossier `Stock Game` ou `Game Root` qui contient une
copie patchée du jeu, et MO2 exécute le jeu depuis là. Quand l'assistant ou
`eidos init` adopte pour la première fois un dossier de liste et que ce dossier
contient l'exécutable du jeu, l'instance exécute elle aussi le jeu depuis lui. Le jeu
démarre toujours par Steam ou la boutique qui le possède, donc rien n'a besoin d'être
déplacé. Eidos trouve le dossier grâce au `gamePath` du `ModOrganizer.ini` de la liste,
ou grâce à ces deux noms. Une instance adoptée avec un Eidos plus ancien garde
l'installation de la boutique jusqu'à ce que vous la changiez (voir la section
suivante). Quand ce `gamePath` nomme un dossier de jeu que l'instance n'exécute pas,
Diagnostics le dit, parce que les outils en mode Mod Organizer 2, comme PGPatcher, y
lisent le jeu. CLF3 reconstruit les fichiers BSA et DDS avec ses propres outils, si bien
que le résultat peut différer d'une installation Wabbajack, et les auteurs de listes ne
le prennent pas en charge.

## Fichiers du jeu : l'installation de la boutique ou une copie

Par défaut, le jeu s'exécute depuis son installation de la boutique, le dossier où
Steam, Heroic ou Legendary le garde. Une instance peut à la place s'exécuter depuis une
copie de ce dossier située dans l'instance. Les mods se posent par-dessus la copie, et
l'installation de la boutique n'est que l'endroit où Eidos monte le résultat. La
boutique démarre toujours le jeu, si bien que Steam garde son overlay, son cache de
shaders et ses sauvegardes cloud.

La carte Fichiers du jeu, en haut de l'onglet Diagnostics, montre quels fichiers l'instance
exécute, avec la version du jeu de la copie et de l'installation de la boutique, et
bascule entre les deux. Dans un terminal, `eidos stock-game <instance>` affiche les
mêmes faits, et `use <folder>`, `store`, `freeze` et `delete --yes` font ce que font les
boutons de la carte. `freeze` affiche la taille et le genre de copie, et `freeze --yes`
copie.

**Figer cette version** copie l'installation de la boutique dans `<instance>/Stock Game`
et exécute le jeu depuis elle, si bien qu'une mise à jour de la boutique ne casse plus
les plugins du script extender : la mise à jour change l'installation de la boutique,
pas la copie. Le premier appui montre la taille et ce que le système de fichiers en
fait, et le second appui copie. Sur un système de fichiers capable de partager des blocs
(reflink), comme btrfs ou XFS, la copie ne prend presque pas de place tant que la
boutique ne modifie pas un fichier. Sur ext4, ou quand le jeu et l'instance sont sur des
systèmes de fichiers différents, c'est une copie complète aussi grande que le dossier du
jeu. Eidos refuse tant que Steam signale le jeu occupé ou pas entièrement installé,
tant qu'un jeu ou un outil de l'instance tourne, quand l'instance contient déjà la
copie du jeu d'une liste, une copie figée ou une copie inachevée, et quand le système de
fichiers a moins d'espace libre qu'une copie complète n'en demande. Les copies Heroic et
Legendary ne sont pas vérifiées pour une mise à jour en cours. Un appui sur Arrêter, un échec ou une
mise à jour Steam du jeu pendant la copie laisse `Stock Game.eidos-tmp`, que la carte
propose de supprimer ; il n'est jamais utilisé.

Une copie masque chaque fichier que la boutique ajoute, change ou retire dans le dossier
du jeu après la copie : nouveaux fichiers Creation Club, nouvel exécutable, nouvelles
DLL. Le jeu, la liste des plugins, LOOT et les contrôles du diagnostic lisent tous la copie.
Elle ne masque pas ce qui se passe hors du dossier du jeu : une mise à jour peut encore
installer des runtimes dans le préfixe Proton au lancement suivant, et Steam peut faire
passer le jeu à un autre Proton.

**Revenir à la version de la boutique** retire la copie du lancement suivant après un second
appui. La copie reste sur le disque, et **Supprimer la copie** la supprime, là encore après
un second appui. La suppression est refusée tant que l'instance exécute encore la copie. Eidos ne
supprime jamais le dossier `Stock Game` ou `Game Root` propre à une liste. Si le dossier
de la copie disparaît, ou ne contient plus l'exécutable du jeu, les lancements sont
refusés et Diagnostics nomme le dossier, si bien que la version de la boutique ne
s'exécute jamais par surprise.

Morrowind, et Oblivion avec `bUseMyGamesDirectory=0`, gardent leur liste de plugins dans
le dossier du jeu, et Eidos ne les exécute pas encore depuis une copie.

## Installer une collection Nexus

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # la lire et dire ce qui se passerait
eidos collection skyrimse rqhcxy --no-optional  # sauter les membres qu'elle marque facultatifs
```

Dans la fenêtre, collez le lien dans **Fichier -> Ouvrir une collection Nexus...**, ou cliquez
sur l'un d'eux sur le site. Eidos liste les membres de la révision, chacun comparé à
cette instance : installé, téléchargé ou manquant. *Installer cette collection* exécute
toute la recette sur un thread de travail, avec la barre de progression qu'utilisent
les tâches d'empaquetage et de dépaquetage, et la barre devient le rapport à la fin. La
fenêtre et `eidos collection` lancent la même installation.

Sans lien, **Fichier -> Ouvrir une collection Nexus...** s'ouvre sur l'onglet **Parcourir**. Il
liste les collections publiées pour le jeu de l'instance ouverte, 20 à la fois, triées
par téléchargements, mise à jour récente ou nouveauté, et la zone de recherche réduit la
liste aux collections dont le nom ou le texte correspond. Chaque ligne donne l'auteur,
la dernière révision, le nombre de mods, les téléchargements et une date. **Voir** ouvre
la dernière révision de cette collection dans l'onglet **Lien**, comme en collant son
lien. Les collections marquées pour adultes sont écartées sauf si votre compte Nexus
affiche le contenu adulte, les collections sans note sont toujours écartées, et la ligne
sous la liste compte les deux. Parcourir demande un compte Nexus connecté, comme le
reste du volet.

Une collection ne s'ouvre que sur une instance de son propre jeu. Une collection Skyrim
ouverte alors qu'une instance Fallout 4 est chargée est refusée nommément, au lieu de
comparer ses membres à la mauvaise liste de mods.

### Ce qu'Eidos lit dans une collection

Presque rien d'une collection n'est visible par l'API de Nexus. L'ordre d'installation,
les réponses que l'auteur a données à l'installateur scripté de chaque mod, quel mod
gagne un conflit de fichier, quels plugins se chargent où et les patchs binaires sont
tous dans un `collection.json` à l'intérieur de l'archive de la collection. Télécharger
les mêmes mods et les installer avec leurs options par défaut donne un autre jeu.

Eidos télécharge l'archive, lit `collection.json` et l'applique :

| La collection dit | Eidos fait |
| --- | --- |
| `phase` | installe dans cet ordre, les membres facultatifs en dernier |
| `choices` | rejoue les réponses de l'auteur à chaque FOMOD, et le dit quand il ne peut pas |
| `modRules` | réordonne la liste de mods pour que le bon mod gagne chaque fichier |
| `plugins` / `pluginRules` | fusionne les règles LOOT dans votre userlist et trie |
| `INI Tweaks/` | les installe comme un mod à part |
| `tools` | vous dit quels outils elle attend ; ne crée rien |

Les membres sont activés à mesure qu'ils s'installent, à la fin de la liste de mods dans
l'ordre d'installation, avant que `modRules` les déplace. Un membre laissé hors de la
liste de mods ne se chargerait pas, et le rapport ne le montrerait pas.

L'état est écrit dans `<instance>/collections/<slug>-<revision>.state.json` après chaque
membre. Il se trouve à côté du dossier de cette révision, jamais dedans, si bien qu'une
collection ne peut pas livrer son propre fichier d'état, et une installation interrompue
au membre 147 sur 200 reprend à 147. Il est écrit sous un nom temporaire puis renommé en
place, si bien qu'une interruption laisse l'ancien enregistrement ou le nouveau, jamais
la moitié de l'un ou de l'autre. S'il ne peut jamais être lu, Eidos s'arrête et le dit
au lieu de recommencer toute la collection. Relancez la même commande.

Installer une révision plus récente d'une collection que vous avez déjà reprend les
dossiers de la révision précédente : un membre inchangé est vérifié sur place, un membre
dont l'auteur a mis à jour le fichier est remplacé dans son ancien dossier, et un membre
que la nouvelle révision a abandonné reste installé et est nommé dans le rapport pour
que vous le désactiviez ou le retiriez. Un dossier qu'un autre profil active n'est
jamais repris, parce que le dossier des mods est partagé et que ce profil changerait
sous vos pieds : la nouvelle révision installe plutôt sa propre copie à côté, et le
rapport vous demande de désactiver l'ancien dans ce profil, jamais de le retirer.
Revenir à une révision plus ancienne reprend ses dossiers de la même façon, ou installe
des copies neuves de ceux qu'elle ne peut pas reprendre. Chaque fois qu'un membre est
remplacé de cette façon et que vous aviez modifié ou ajouté des fichiers dans son
dossier, l'ancien dossier est d'abord gardé à côté sous le nom `<name>_backup`, qui
n'est jamais chargé dans le jeu, et le rapport le nomme.

Un reste d'une révision antérieure qui reste activé empêche l'installation de compter
comme celle que l'auteur a construite ; la note disparaît une fois que vous l'avez
désactivé. Un membre que vous avez sauté avec `--no-optional` n'est pas repris et est
nommé de la même façon. Si la reprise elle-même échoue, l'exécution s'arrête avec
*previous revision: its folders could not be taken over* et la raison ; corrigez cela
et relancez la commande.

### Comptes gratuits et membres que vous récupérez vous-même

Un compte Nexus gratuit ne peut pas récupérer les mods membres. Nexus ne donne aux
comptes gratuits aucun lien de téléchargement par l'API : chaque fichier demande le
bouton « Mod Manager Download » du site. L'archive de la collection se télécharge avec
un compte gratuit, si bien que vous pouvez lire toute la recette, mais chaque membre est
signalé avec l'URL exacte de sa page pour que vous le récupériez vous-même, et réessayer
n'y change rien. Récupérez-les par le bouton du site et relancez la commande : ces
membres sont cherchés dans `downloads/` à chaque exécution, si bien que la collection se
termine à mesure que vous les fournissez.

Dans la fenêtre, un compte gratuit a
*Récupérer un par un les N fichiers requis manquants* dans le volet de la collection à la
place de la récupération par lot. Il ouvre la page du premier fichier requis manquant
sur Nexus, attend que ce fichier arrive dans `downloads/`, puis ouvre le suivant. Vous
appuyez toujours vous-même sur le bouton de téléchargement de la page pour chaque
fichier. Le volet montre où il en est (*3 sur 12*), avec *Passer* et *Arrêter*. Quand un autre
fichier du même mod arrive à la place, le volet nomme les deux identifiants de fichier
et continue d'attendre celui qui est visé. Les membres facultatifs n'en font pas partie
et gardent leur propre bouton *Ouvrir*.

Eidos applique aussi les patchs binaires (chacun vérifié contre son CRC enregistré), les
remplacements de fichiers (les fichiers nommés sont masqués avec `.mohidden`, comme le
fait MO2) et les membres dont les fichiers voyagent dans l'archive de la collection
(`bundle`). Les membres que la collection attend que vous récupériez à la main
(`browse`, `manual`) transmettent au rapport les instructions propres à l'auteur. Un
membre dont le nom est déjà un de vos mods est installé à côté sous un nom libre, jamais
par-dessus, et le rapport dit lequel.

### Le rapport

Un membre dont les réponses d'installateur n'ont pas toutes pu être rejouées a sa propre
ligne, *installed, but not the way the collection asks*, à part des membres qui se sont
installés comme l'auteur les a construits, parce que ses fichiers sur le disque diffèrent
de ceux de l'auteur. Le rapport liste aussi les règles d'ordre qui ne nommaient rien dans
la collection, les mods pris dans une boucle de règles contradictoires, les groupes LOOT
qui n'existent pas, et toute partie du manifeste que cette version d'Eidos ne comprend
pas.

### Garder l'ordre de la collection

Eidos garde après l'installation les règles de chaque collection installée. Quand vous
déplacez plus tard deux de ses mods contre une règle d'avant ou d'après, l'onglet
Diagnostics montre une carte pour cette collection, nommant la première paire de ce
genre et comptant les règles qui ne tiennent plus. Seuls les mods activés dans le
profil actuel comptent, et les règles prises dans une boucle sont sautées, parce
qu'aucun ordre ne peut les respecter. **Restaurer l'ordre de la collection** remet les mods de la
collection dans un ordre qui respecte chaque règle, garde votre propre ordre partout où
les règles le permettent, et laisse où ils sont les mods qui ne sont pas dans la
collection. Ctrl+Z l'annule. `eidos check` liste le même constat sous le nom
`collection_order_broken`. La vérification lit la recette de la révision dans
`collections/`, si bien qu'une collection dont la recette en cache a été retirée n'est
pas vérifiée.

## Déplacer une instance vers une autre machine

`eidos pack` écrit une instance entière (chaque mod, l'ordre de chargement, tous les
profils, l'Overwrite avec vos sauvegardes dedans, et les archives d'où tout a été
installé) dans un seul fichier. `eidos unpack` la remet en place :

```sh
eidos pack skyrimse ~/backup.eidos                  # ou donnez un dossier : nommé d'après le jeu et la date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # sur l'autre machine
```

`eidos pack --dry-run` liste exactement ce qui entrerait, ce qui n'entrerait pas et
pourquoi, et la place qu'il faut, sans rien écrire. `eidos unpack --info` fait la même
chose pour un fichier que vous avez déjà.

Si 7-Zip n'a pas pu lire quelque chose, ou si le parcours a dû sauter quelque chose
qu'on attendrait dans une sauvegarde (un lien symbolique, un dossier illisible, un nom
qu'on ne peut pas donner à 7-Zip en toute sécurité), l'archive est quand même écrite
sous le nom choisi, mais `eidos pack` se termine avec le code 1 et dit ce qui manque, si
bien qu'un script qui lance `eidos pack ... && ...` s'arrête là.

Un fichier `.eidos` est une archive 7-Zip avec sa propre extension. 7-Zip est déjà exigé
pour installer un mod, donc cela n'ajoute aucune dépendance, et la sauvegarde s'ouvre
sans Eidos dans tout gestionnaire d'archives qui lit le 7-Zip. Elle est compressée en
LZMA2 à `-mx1`, à environ 43 % de l'original sur une liste riche en textures ; `-mx9`
prend beaucoup plus de temps et gagne quelques points de plus. Les textures et les
meshes sont déjà dans des formats compressés, si bien qu'un dictionnaire plus grand
trouve très peu de choses encore à compresser. `--level` (0, 1, 3, 5, 7 ou 9) choisit un
autre niveau, et `--no-downloads` laisse de côté `downloads/`.

Au niveau 1 par défaut, les fichiers vont dans des blocs solides d'au plus 16 Mo.
Extraire un fichier d'une sauvegarde de 70 Go ne décode au plus que le bloc où il se
trouve, jamais tout ce qui le précède, et empaqueter du contenu de jeu va de 2 à 5 fois
plus vite qu'avec chaque fichier compressé séparément (le dépaquetage de 2 à 4 fois).
Le coût est en cas de dommage : un octet corrompu peut emporter chaque fichier de son
bloc, jusqu'à 16 Mo de fichiers voisins. Les données de récupération décrites plus bas
couvrent cela. Tous les autres niveaux restent non solides : chaque fichier est son
propre flux, et un octet corrompu coûte ce seul fichier. Le niveau 0 stocke les
fichiers. À partir du niveau 3, LZMA2 travaille déjà par morceaux de 16 Mo ou plus, et
les blocs solides se sont mesurés plus lents là : au niveau 5, empaqueter des meshes a
pris 1,4 fois plus de temps et dépaqueter des textures 6,8 fois plus.

L'empaquetage prend le verrou de l'instance, donc il ne peut pas tourner pendant que le
jeu ou un autre Eidos écrit dans cette instance, et rien ne change l'instance pendant
qu'elle est lue. La sauvegarde est écrite sous un nom `.part`, vidée sur le disque, et
relue en entier avec `7z t` avant de recevoir le nom que vous avez choisi. Cette
vérification décode chaque bloc et compare chaque somme de contrôle, le même travail que
fait un dépaquetage sur l'autre machine. Elle montre ce que le disque a rendu juste après
l'écriture, pas que le disque le gardera ; les données de récupération ci-dessous sont
là pour cela. Si elle échoue, le `.part` est supprimé, rien n'est écrit sous votre nom,
et `eidos pack` cite la ligne où 7-Zip a trouvé le dommage. Un empaquetage interrompu ou
échoué laisse un fichier `.part` ou rien, jamais un fichier `.eidos` qui a l'air complet
sans l'être.

Quand par2cmdline est installé (`sudo pacman -S par2cmdline` sur Arch,
`sudo apt install par2` sur Debian et Ubuntu, `sudo dnf install par2cmdline` sur
Fedora), `eidos pack` écrit aussi des données de récupération PAR2 à côté de l'archive :
`backup.eidos.par2` et quelques fichiers `backup.eidos.volNN+MM.par2`, soit 5 % de
l'archive plus un petit index. Gardez-les dans le même dossier que le fichier `.eidos`,
sous les noms avec lesquels ils ont été écrits, et copiez-les avec lui : une sauvegarde
déplacée sans eux n'a rien dont elle puisse être réparée, et les données de récupération
enregistrent le nom de l'archive, si bien qu'une sauvegarde renommée ne leur correspond
plus. `eidos unpack` les trouve par leur nom et vérifie l'archive contre eux avant
d'extraire quoi que ce soit, y compris une sauvegarde dont la fin a été coupée au point
que 7-Zip ne peut pas l'ouvrir. Si quelques octets se sont abîmés, par2 répare d'abord
l'archive et garde la copie abîmée sous le nom `backup.eidos.1`, ce qui demande dans son
dossier un espace libre de la taille de l'archive ; un dommage au-delà de ce que couvrent
les 5 % arrête le dépaquetage avant qu'un seul fichier soit écrit. Les fichiers de
récupération qui ne correspondent pas à l'archive, comme ceux d'une ancienne sauvegarde
de même nom, ne sont pas utilisés quand 7-Zip lit l'archive et la trouve intacte, et le
dépaquetage le dit. Sur une machine sans par2, le dépaquetage s'exécute quand même, avec
un avertissement disant que les données de récupération n'ont pas été utilisées.
`--no-recovery` empaquette sans elles, et `eidos pack --dry-run` dit si elles seront
écrites. `--force` remplace un fichier existant du même nom.

### Ce qui n'y est pas, et pourquoi

| Laissé de côté | Pourquoi |
| --- | --- |
| Le préfixe Proton | Il est façonné par la machine (correspondances de lecteurs, une vue `Z:` de *ce* système de fichiers) et Eidos le reconstruit en une minute avec `eidos prereqs <instance> --install`. Le porter ferait à peu près doubler l'archive pour livrer quelque chose que l'autre machine doit de toute façon régénérer. |
| Le jeu | Une instance modifie un jeu qu'elle ne contient pas. Installez-le depuis Steam là-bas. |
| Les outils hors de l'instance | xEdit, DynDOLOD et d'autres outils sont des binaires tiers avec leurs propres installateurs. Ils sont nommés dans l'archive, si bien que le dépaquetage vous dit exactement lesquels manquent sur la nouvelle machine. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Des enregistrements de la machine qui a fait la sauvegarde. `prereqs.done` en particulier affirmerait que les bibliothèques runtime sont déjà installées dans un préfixe qui n'est pas là. |
| `loot/`, sauf `userlist.yaml` | Un cache de masterlist qu'Eidos récupère de nouveau à la demande. Vos propres règles LOOT dans `userlist.yaml` ne peuvent pas être récupérées de nouveau, donc ce seul fichier est inclus. |
| `.base/`, `.base-root/` | Des points de montage vides où les propres fichiers du jeu sont mis de côté pendant une session. |
| `test-run/`, `bisect.json` | Une [partie test](#parties-test) en attente d'examen, et une recherche du mod à l'origine d'un problème entre deux étapes. Les deux appartiennent à des exécutions de cette machine. |
| Les fichiers à moitié écrits | Un téléchargement en pause (`*.unfinished`), une écriture atomique en cours (`*.eidos-tmp*`). |
| Les liens symboliques | 7-Zip en suivrait un et copierait ce qu'il désigne, ce qui, pour un lien absolu, revient à tirer une arborescence étrangère dans votre sauvegarde. Ils sont signalés à la place, et la sauvegarde compte comme incomplète : un `mods/` ou un `downloads/` lié vers un autre disque n'est pas empaqueté. Pour sauvegarder volontairement sans un `downloads/` lié, passez `--no-downloads` (ou décochez *Inclure downloads/* dans la fenêtre) ; une exclusion délibérée ne compte pas comme un manque. |

Une copie du jeu dans l'instance (`Stock Game`) est empaquetée comme tout autre dossier,
si bien que la sauvegarde d'une instance figée est plus grosse de la taille du jeu. Une
copie inachevée (`Stock Game.eidos-tmp`) est laissée de côté.

Chacun de ces éléments est listé avec sa raison dans `eidos-backup.ini` à la racine de
l'archive, si bien que lire ce fichier montre ce qui a été laissé de côté. Le même
fichier enregistre où l'instance vivait, quel profil était actif, la taille que prend
le dépaquetage, et de quels outils la nouvelle machine aura besoin.

Les fiches de téléchargement (`downloads/*.meta`) entrent avec leur ligne `url=` vidée.
Cette URL est un lien Nexus signé qui cesse de fonctionner en quelques heures et porte
le `user_id` du compte qui a téléchargé le fichier, et une sauvegarde peut être remise à
quelqu'un d'autre. L'identifiant du mod, l'identifiant du fichier et la version restent,
si bien que le téléchargement peut être retrouvé.

### Dans la fenêtre

Les deux sont dans le menu **Fichier** : *Archiver cette instance...* et *Restaurer une archive...*.
Le dépaquetage est aussi sur l'écran d'accueil, sous la liste des instances, si bien
qu'une machine sans instance peut en dépaqueter une.

Avant que quoi que ce soit soit écrit, la boîte de dialogue Archiver cette instance montre combien de
fichiers, quelle taille, quelle part en est `downloads/`, à peu près ce que fera
l'archive, et quels outils sont nommés mais non portés. Sa case *Écrire des données de récupération à côté*
est cochée quand par2 est installé, et grisée avec le paquet à installer quand il ne
l'est pas. La boîte de dialogue Restaurer une archive lit le manifeste de la sauvegarde dès que vous
choisissez le fichier, si bien qu'elle peut vous dire quel jeu elle contient, quand elle
a été faite, où elle vivait et lesquels de ses outils manquent ici, avant que vous
choisissiez un dossier.

Les deux s'exécutent sur un thread de travail avec une barre de progression, et la barre
devient le rapport à la fin.

### Ce que le dépaquetage répare

Une simple copie du dossier laisserait chaque bouton d'outil pointer vers un chemin de
l'ancienne machine. `eidos unpack` réécrit les valeurs qui nomment l'ancienne racine de
l'instance, et seulement celles-là : `exe`, `workdir` et les clés numérotées `arg` dans
`tools.ini`, et `installationFile` dans le `meta.ini` de chaque mod. Un outil qui n'a
jamais été dans l'instance (`/opt/xedit/SSEEdit.exe`) est laissé exactement tel quel et
listé comme quelque chose à réinstaller ; Eidos ne devine pas où cet outil se trouve sur
la nouvelle machine. Il corrige aussi si l'instance se dit centrale ou portable, si bien
que les commandes suivantes la cherchent là où elle est maintenant.

Le dépaquetage refuse un dossier qui n'est pas vide (passez `--force` si vous voulez
écraser), un disque trop petit pour le chiffre du manifeste lui-même, une archive faite
par un Eidos plus récent que le vôtre, et toute archive contenant une entrée qui serait
écrite *en dehors* du dossier que vous avez choisi. Sans dossier, la sauvegarde d'une
instance globale retourne à la place centrale de son jeu ; une instance portable demande
un dossier.

Ensuite :

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # reconstruire le préfixe Proton
eidos play /mnt/games/EidosSkyrim
```

## La fenêtre

Lancez `eidos-gui`, ou le fichier `eidos-linux` qu'installe Colony.

Le premier lancement ouvre un assistant comme celui de MO2 : accueil, type d'instance
(portable ou globale), jeu, nom et emplacement, résumé, puis la fenêtre principale.
L'écran d'accueil liste aussi chaque instance connue, globale et portable, la plus
récente d'abord, à ouvrir d'un clic, si bien qu'il sert aussi de sélecteur d'instance.
Pointer l'assistant vers un dossier qui contient déjà une instance ouvre cette instance
telle quelle au lieu d'en créer une nouvelle par-dessus, et un dossier qui appartient à
un autre jeu est refusé.

La fenêtre principale a deux volets : la liste de mods, et les onglets Data, Plugins,
Conflits, Overwrite, Archives, Sauvegardes, Téléchargements et Diagnostics, avec un bouton Lancer, un
bouton Partie test ([Parties test](#parties-test)) et un sélecteur de cible d'exécution. Vous
filtrez la liste de mods, sélectionnez, réordonnez, regroupez avec des séparateurs,
restreignez par catégorie et faites un clic droit pour les actions. Outre l'envoi tout
en haut et tout en bas, le menu d'un mod a les déplacements ciblés de MO2 : envoyer
au-dessus du premier mod en conflit, en dessous du dernier, à une priorité donnée, ou
dans le groupe d'un séparateur.

Le sélecteur de profil change de profil. Faites un clic droit sur un profil pour
Renommer, Copier vers un nouveau..., Raccourci de bureau, Importer depuis MO2... et Supprimer, qui demande deux
clics et refuse le profil actif ou le dernier. Importer depuis MO2... reprend l'ordre des
mods et l'ordre de chargement d'un profil MO2. Raccourci de bureau écrit une entrée de
bureau qui exécute `eidos tool <instance> run "<game>" --profile <name>` : elle fait de
ce profil le profil actif et demande à la boutique de démarrer le jeu (détails dans
[tools.md](tools.md#atteindre-un-outil--masquer-épingler-et-un-raccourci-de-bureau)).

Le menu **Fichier** lance aussi winecfg, regedit, l'explorateur de fichiers de Wine et le
désinstalleur dans le préfixe du jeu, et drive_c du préfixe ouvre le lecteur `C:` du
préfixe ; voir [tools.md](tools.md#les-programmes-propres-à-wine-dans-le-préfixe-du-jeu).
*Instances...* liste les instances qu'Eidos connaît ; Oublier en retire une de cette
liste et ne touche à rien sur le disque. *Exporter la liste des mods...* écrit la liste au
format CSV de MO2, comme le fait `eidos export`.

### Préférences

Cliquez sur **Eidos**, en haut à gauche, pour ouvrir les Préférences à la place des deux
volets, et cliquez de nouveau dessus (ou sur **Fermer**) pour revenir. Général,
Apparence et Accessibilité viennent en premier, comme dans chaque programme Colony,
puis Liste des mods, Nexus et À propos. Chaque changement s'applique et s'enregistre aussitôt ;
il n'y a pas de bouton Enregistrer.

- Général : le jeu qu'Eidos ouvre, la taille de la fenêtre, la langue de l'interface, le
  dossier d'outils, et la façon dont un jeu est lancé. Verrouiller la fenêtre pendant le jeu
  est activé par défaut. Lancer le jeu directement quand une boutique ouvre Eidos, et la
  notification de bureau qui dit quand une longue tâche se termine pendant que la
  fenêtre est en arrière-plan, sont désactivés par défaut. La fenêtre parle anglais et
  français ; Système suit `LC_ALL`, `LC_MESSAGES` ou `LANG`, et le changement s'applique
  aussitôt. La ligne de commande `eidos` répond toujours en anglais.
- Apparence : le thème (26 familles, 59 palettes, le parchemin d'Eidos par défaut), la
  couleur d'accent, la taille du texte, les effets et un aperçu en direct.
- Accessibilité : le contraste élevé, une police adaptée à la dyslexie, la réduction des
  animations et une taille de texte de lecture qui se multiplie avec celle d'Apparence.
- Liste des mods : la vitesse de défilement automatique pendant qu'un mod est glissé vers le
  bord haut ou bas de la liste.
- Nexus : la connexion, le dossier de téléchargements, le gestionnaire `nxm://` et son
  bouton Enregistrer Eidos pour les liens nxm, les serveurs préférés, Installer les téléchargements dès qu'ils sont terminés, et le mode
  hors ligne.
- À propos : où vivent les fichiers, les raccourcis clavier, la version et la vérification
  de version.

L'apparence et la langue de l'interface vivent dans
`~/.config/Colony/Eidos/preferences.json`. Les autres réglages, que la ligne de commande
lit aussi (le jeu par défaut, le dossier d'outils, les serveurs Nexus, le mode hors ligne
et le reste), vivent dans `settings.ini` à côté, et la session Nexus dans `nexus.ini`. Le
thème qu'un Eidos plus ancien gardait dans `settings.ini` est repris la première fois.

**À propos -> Version** contient la vérification de version, activée par défaut. Au
démarrage, au plus une fois par jour, Eidos demande à GitHub la dernière version de
`Project-Colony/Eidos`. Quand elle est plus récente que la copie en cours, la barre de
menus montre `Eidos <version> est disponible : notes de version` après Aide, et le lien ouvre
la page de cette version avec son journal des modifications. Eidos ne se télécharge ni ne
se remplace jamais lui-même. La vérification est sautée en mode hors ligne. Elle est
désactivée pour une copie installée sous `/usr` par un gestionnaire de paquets, qui la
met à jour ; `/usr/local`, où `install.sh --system` place Eidos, compte comme votre
propre installation.

### Colonnes, tri et regroupement

La liste montre quatre colonnes par défaut (Catégorie, Contenu, Version et Indicateurs, celles
que montre MO2) et en propose huit : Catégorie, Contenu, Version, Auteur, Installé,
Id Nexus, Jeu, Indicateurs. Cochez-les dans le menu Affichage. Avec les huit activées, la colonne
du nom a peu de place.

Cliquez sur un en-tête pour trier dessus, de nouveau pour inverser, et une troisième fois
pour revenir à l'ordre de chargement. La liste ne peut être glissée qu'en ordre de
chargement : une position de dépôt désigne la vraie liste, et sous un tri la place d'une
ligne à l'écran n'est pas sa place dans cette liste. Tant qu'un tri est actif, les
bandes d'insertion ne sont pas dessinées et un glisser est refusé, comme dans MO2. Le
menu Affichage le dit et propose le chemin du retour.

Le menu Affichage peut aussi regrouper toute la liste, par catégorie ou par source (depuis
Nexus, ou installé à la main). Les en-têtes de groupe ne sont pas des séparateurs : on ne
peut ni les renommer, ni les colorer, ni les déplacer. Ils se replient, et un en-tête
replié garde son compte. Les séparateurs sont masqués tant qu'un tri ou un regroupement
est actif, parce qu'un séparateur coiffe les lignes qui le suivent dans l'ordre de
chargement, et que les deux changent cet ordre à l'écran.

### Souris et clavier

Double-cliquez sur un mod pour Information, Ctrl+double-clic pour son dossier,
Maj+double-clic pour sa page Nexus. Ctrl+F place le curseur dans la zone de filtre.
Taper une lettre saute au mod suivant qui commence par elle, et la retaper parcourt le
reste au lieu de rester sur le premier. Aucun ne s'arrête sur une ligne que le filtre, un
séparateur replié ou un groupe replié masque, si bien que la prochaine pression sur
Espace ne bascule jamais un mod que vous ne voyez pas.

« Replier les autres » dans le menu d'un séparateur replie tous les groupes sauf celui-là.
Pendant un glisser, maintenir le pointeur immobile sur un groupe replié l'ouvre, si bien
que le mod peut être déposé dedans ; passer dessus ne l'ouvre pas.

Ctrl+Z annule la dernière modification de la liste de mods ou de l'ordre de chargement :
une case cochée, un déplacement, une désactivation par lot, un tri LOOT. Ctrl+Maj+Z la
rétablit. La fenêtre garde 50 étapes et les oublie sur F5, à la fin d'une exécution de
jeu ou d'outil, lors d'un changement de profil et quand un autre processus Eidos change
la liste. Installer, renommer ou retirer un mod change les dossiers sur le disque, donc
une annulation au-delà de l'un d'eux est refusée.

### Filtrer la liste

Le bouton Filtres ouvre une ligne par critère : actifs, en conflit, mise à jour disponible,
sans catégorie, recommandés, suivis, avec un id Nexus, avec des fichiers masqués, sans données de jeu valides,
pour un autre jeu, avec des notes, et une ligne par type de contenu (plugins, archives BSA ou
BA2, textures, meshes, scripts, plugins de script extender, fichiers d'interface, son,
FaceGen). Un clic fait passer une ligne d'inactive à « seulement » puis à « sauf ». La ligne du haut
bascule entre correspondre à tous les critères activés et correspondre à l'un d'eux. La
zone de nom et la liste des catégories restreignent toujours davantage le résultat. La
ligne des fichiers masqués ne regarde que les mods activés.

### Ce que la liste vous dit d'un mod

Deux drapeaux consultatifs, chacun un glyphe avec l'explication au survol. « Sans données de jeu
valides » veut dire que rien en haut du mod ne ressemble à quelque chose que ce jeu charge ;
il lui faut peut-être que ses dossiers remontent d'un niveau, ou ce n'est peut-être pas
un mod pour ce jeu. « Pour un autre jeu » veut dire que le `meta.ini` du mod nomme un autre jeu.
Aucun des deux ne bloque quoi que ce soit : le mod se déploie quand même. « Marquer comme
valide » dans le menu de la ligne fait taire l'un comme l'autre par la clé `validated=`
propre à MO2, si bien qu'un mod marqué valide dans un gestionnaire n'est pas signalé
dans l'autre.

La vérification de disposition ne signale un mod que quand elle en est sûre : une
arborescence `Root/`, un dossier illisible et un dossier vide comptent tous comme
données de jeu valides.

### Désactiver un mod que vos sauvegardes ou d'autres mods utilisent

Avant qu'un mod soit désactivé ou retiré, Eidos lit les listes de plugins des trois
sauvegardes les plus récentes du profil. Quand l'une d'elles utilise un plugin de ce
mod, une barre au-dessus de la liste nomme les plugins et attend « Désactiver quand même » ou
Annuler (Échap annule aussi). Supprimer ajoute les mêmes noms à son message de reclic. Seules
les sauvegardes de Skyrim et de Fallout 4 sont lues ; les autres jeux désactivent sans
demander.

L'onglet Général des informations d'un mod liste les mods dont il a besoin et ceux qui
ont besoin de lui, lus dans les masters des plugins activés. Désactiver un mod dont
d'autres ont besoin montre la même barre avec « Les désactiver aussi », qui désactive aussi
chaque mod qui en a besoin et les mods qui ont besoin de ceux-là, ou « Les laisser activés ».
La carte des masters manquants de l'onglet Diagnostics a « Activer ce qui les fournit » : elle
active le mod qui contient chaque master manquant et coche un plugin master présent mais
non coché.

### Sauvegarder un mod avant d'y toucher

« Sauvegarder ce mod » copie son dossier à côté sous le nom `<name>_backup` (puis
`_backup2`, et ainsi de suite ; une sauvegarde ne remplace jamais la précédente). La
copie n'est pas un mod : sa case ne fait rien et elle n'ajoute rien à la vue fusionnée,
parce que la cocher déploierait deux copies d'un même mod l'une sur l'autre. « Restaurer
cette sauvegarde sur le mod » la remet en place, en deux clics ; le contenu actuel est
d'abord mis de côté et n'est jeté qu'une fois la copie réussie.

**Sauvegardes** dans la barre d'outils garde des points de restauration de la liste de mods
du profil, de son ordre de chargement (`plugins.txt` et `loadorder.txt` ensemble) et de
ses INI du jeu, 10 de chaque, dans `<profile>/backups/`. Sauvegarder maintenant en ajoute un, et
Restaurer sauvegarde d'abord l'état actuel, si bien qu'un mauvais choix peut être annulé.
Eidos garde aussi un point de restauration des INI avant chaque exécution de BethINI
(voir [tools.md](tools.md#bethini-pie)).

### Les onglets

**Data** est un arbre de la vue fusionnée, déplié d'un niveau à la fois : ouvrir un
dossier lit ce dossier dans chaque couche qui l'a, jamais chaque mod activé d'un coup. Il
est construit à partir de la même pile de couches que celle que sert le montage, si bien
que les whiteouts et les fichiers masqués sont respectés et que l'onglet montre ce que le
jeu verra. Filtrez-le par nom, restreignez-le aux fichiers contestés, démêlez ce qui est
où avec les colonnes Taille et Modifié, et faites Localiser sur n'importe quelle ligne dans
un gestionnaire de fichiers. Survoler un fichier liste les autres mods qui le livrent. Les
`meta.ini`, `readme.txt`, `fomod/` et `Root/` de premier niveau d'un mod sont laissés hors
de Data, dans cet onglet et dans le montage : les trois premiers sont pour le gestionnaire
de mods, et `Root/` va plutôt dans le dossier du jeu. Un fichier que le jeu ou un outil
écrit sous l'un de ces noms atterrit dans l'Overwrite et reste visible.

Une ligne `.exe` dans Data a un bouton Ajouter, qui ouvre l'éditeur Exécutables avec ce fichier
renseigné (rien n'est enregistré avant d'appuyer sur Enregistrer), et un bouton Lancer, qui le lance une fois
à travers la vue fusionnée comme un outil enregistré ([Outils](#outils)). Enregistrer l'arbre écrit
tout l'arbre Data fusionné dans un fichier texte, une ligne par fichier avec le mod qui le
fournit, votre dossier personnel et votre nom d'utilisateur retirés, pour un rapport de
bug.

**Plugins** est l'ordre de chargement ESP/ESM/ESL : basculez, réordonnez à la main, ou
triez avec LOOT et lisez le rapport après tri, dont les liens de conseils s'ouvrent dans
votre navigateur. Son en-tête compte les plugins activés dans chaque espace d'index
contre la limite du jeu, par exemple `pleins 231/254, légers 512/4096`, et devient orange
quand l'un est plein. La zone de filtre restreint la liste par nom. Espace, Ctrl+A, les
plages avec Maj et les entrées Activer les plugins affichés et Désactiver les plugins affichés du menu de la
ligne n'agissent alors que sur les lignes affichées, et un glisser est refusé tant que le
filtre n'est pas effacé. Sélectionner un plugin teinte ses masters dans la liste de
plugins et le mod qui le livre dans la liste de mods. Les plugins que LOOT signale comme
sales ont une carte Diagnostics qui peut les nettoyer avec xEdit
([tools.md](tools.md#nettoyer-un-plugin-que-loot-signale-comme-sale)).

Après un tri LOOT, un plugin complet dont tous les nouveaux enregistrements tiennent dans
la plage légère le dit au survol, et le menu de sa ligne a « Marquer léger, dans un nouveau mod ». Eidos écrit une
copie avec l'indicateur léger dans un nouveau mod nommé `<plugin> (light)` en bas de la
liste de mods (la priorité la plus haute) et laisse l'original tranquille, si bien que
désactiver ce mod l'annule. Il refuse quand l'une des trois sauvegardes les plus récentes
du profil charge le plugin comme plugin complet. Seules les sauvegardes de Skyrim et de
Fallout 4 sont lues, donc sur Starfield aucune sauvegarde n'est vérifiée. LOOT ne vérifie
que les plages d'enregistrements : regardez dans xEdit un plugin qui ajoute des cellules
ou des espaces du monde avant de le signaler.

Survoler un plugin montre d'où il vient, son auteur, la version de son en-tête et de ses
formulaires, les 300 premiers caractères de sa description, ses masters et, après un tri
LOOT, les messages de LOOT. Sur Skyrim, Skyrim SE, Skyrim VR et Enderal SE, le jeu lit
aussi un INI nommé d'après un plugin chargé ; la colonne INI marque ces plugins, et un
clic ouvre ce fichier en lecture seule.

**Conflits** explique les gagnants et les perdants fichier par fichier. **Overwrite**
transforme en une étape ce que le jeu a écrit en un vrai mod.

**Archives** liste chaque BSA ou BA2 que livrent les mods activés, le mod d'où il vient et
pourquoi il se charge : un plugin actif du même nom, ou l'INI l'enregistre. Une archive que
rien ne charge est marquée comme poids mort. Quand un autre mod livre une archive du même
nom qui gagne le conflit, la ligne nomme ce mod. Extraire décompresse une archive dans un
nouveau mod nommé d'après elle sans son extension (`Foo.ba2` donne `Foo (extracted)`) et
laisse l'archive dans son mod ; les fichiers libres l'emportent sur les fichiers de
n'importe quelle archive, quel que soit l'ordre des mods.

**Sauvegardes** lit l'en-tête de chaque sauvegarde (personnage, niveau, lieu, temps de jeu),
compare la liste de plugins qui y est stockée à l'actuelle, et a un bouton qui active les
mods dont elle a besoin. Les sauvegardes arrivées dans le préfixe sans Eidos sont décrites
sous [Les sauvegardes appartiennent au profil](#les-sauvegardes-appartiennent-au-profil).

**Téléchargements** et **Diagnostics** ont leurs propres sections plus bas.

### Informations d'un mod

« Informations... » ouvre une boîte de dialogue par mod avec les onglets Général,
Conflits, Fichiers, Réglages INI, Notes, Fichiers Nexus et Page Nexus. Depuis l'onglet Fichiers (et
depuis l'arbre Data), n'importe quel fichier peut être masqué : il est renommé en
`<name>.mohidden`, ce qui le retire de la vue fusionnée sans le supprimer, si bien que
quelques fichiers égarés d'un mod peuvent être supprimés sans changer les priorités. Le
l'onglet Fichiers fait aussi les opérations ordinaires sur les fichiers : nouveau dossier,
renommer, supprimer, ouvrir. Chacune refuse un chemin qui n'est pas un simple chemin à
l'intérieur de ce mod : pas de `..`, pas de chemin absolu et pas de lien symbolique en
route, puisqu'en suivre un pourrait supprimer un fichier hors du dossier du mod. Renommer ne
change que la dernière partie du chemin, si bien qu'il ne déplace jamais un fichier, et il
refuse un nom déjà pris. Supprimer demande deux clics et ne peut pas être annulé.

**Voir** sur n'importe quelle ligne de l'onglet Fichiers ou de l'arbre Data prévisualise le
fichier : images, texte, textures DDS (choisissez le mip, la couche, la face et le canal)
et modèles NIF statiques avec leurs textures diffuses. L'aperçu NIF a besoin de
`eidos-nif-preview` à côté d'Eidos ou dans le `PATH` ; le paquet de version, le paquet Arch
et les compilations depuis les sources l'ont, `eidos-linux` non. Le texte est lu jusqu'à
64 Ko et dit où il s'est arrêté.

**Réglages INI** liste les fragments qu'un mod livre dans son dossier `INI Tweaks/` ; ceux
qui sont activés sont fusionnés dans l'INI du jeu du profil au lancement, dans l'ordre de
priorité, et retirés de nouveau quand les INI de l'exécution sont capturés, si bien que
désactiver un ajustement retire ses réglages.

L'onglet **Fichiers Nexus** liste les fichiers du mod sur Nexus, le plus récent d'abord, et
marque celui qui est installé ici et celui qui le remplace. Cette seconde ligne a un bouton
**Mettre à jour** ; voir [Mises à jour des mods](#mises-à-jour-des-mods).

L'onglet **Page Nexus** montre la description du mod et, le plus récent d'abord, le
journal des modifications de chaque version de fichier sur sa page. Eidos lit les deux la
première fois que vous ouvrez l'onglet, par l'API GraphQL de Nexus, qui ne compte pas dans
le budget horaire de requêtes. Il les garde jusqu'à la fermeture d'Eidos ou jusqu'à ce que
vous appuyiez sur **Actualiser**. Ils sont affichés en texte brut : le formatage BBCode et
HTML est retiré, l'adresse d'un lien est écrite après son texte, et les images et les
vidéos sont laissées de côté. **Recommander** et **Suivre** au-dessus du texte font ce que fait
le menu contextuel du mod. La page d'un mod pour adultes reste masquée sauf si votre compte
Nexus affiche le contenu adulte, et une page sans note reste masquée dans tous les cas.

### Téléchargements

Un téléchargement peut être glissé de la liste Téléchargements vers une position de la liste de
mods pour s'installer à cette priorité, et les archives ou dossiers déposés sur la fenêtre
depuis un gestionnaire de fichiers s'installent aussi. Les dépôts de fichiers demandent une
session X11 ou XWayland : winit ne les implémente que pour X11. Les téléchargements peuvent
être mis en pause et repris : mettre en pause arrête le transfert et garde le fichier
partiel, et Reprendre demande à Nexus un lien neuf et continue là où il s'était arrêté.

L'onglet Téléchargements garde les archives que vous avez téléchargées. Filtrez-le par nom (le
nom convivial du mod aussi, si bien que « skyui » trouve `SkyUI_5_2_SE-12604-5-2SE.7z`),
triez par plus récent, nom, taille ou état, et masquez une archive dont vous en avez fini :
le fichier reste et seule la ligne disparaît. « Afficher les masqués » les ramène, et le même bouton
les démasque. « Retirer les N installés » supprime les archives des mods que vous avez déjà
installés, en deux clics, et seulement celles que le filtre montre.

**Préférences -> Nexus -> Installer les téléchargements dès qu'ils sont terminés** est désactivé par défaut.
Activé, un téléchargement Nexus qui se termine pendant que la fenêtre est ouverte
s'installe sans passer par l'onglet Téléchargements. C'est l'installation habituelle : un FOMOD
ouvre son installateur, et un nom déjà présent dans la liste de mods demande quoi faire.
Les archives qui étaient dans `downloads/` avant l'ouverture de la fenêtre ou avant que
l'option soit activée sont laissées tranquilles, de même qu'un téléchargement dont vous avez
annulé l'installation. Tant qu'une collection est ouverte dans le volet des collections,
aucun téléchargement ne s'installe de lui-même, et un téléchargement qui se termine entre
temps ne s'installera pas de lui-même plus tard non plus ; la collection installe ses
membres. Le fichier qu'attend une mise à jour en attente (bouton Mettre à jour) est installé par cette mise à jour. Seuls les
téléchargements Nexus sont installés ainsi, et un téléchargement que le filtre de l'onglet Téléchargements
masque attend que le filtre soit effacé. Pendant un glisser en cours, ou pendant que le jeu
ou un autre Eidos tient l'instance, l'installation attend.

### Mises à jour des mods

Le bouton de mise à jour vers l'extrémité droite de la barre d'outils vérifie chaque mod qui
a un identifiant Nexus. Quand la page du mod montre la version que vous avez installée, rien
de plus n'est demandé. Sinon, Eidos lit la liste de fichiers du mod (une requête de plus,
gardée pour le reste de la vérification) et suit la chaîne de mise à jour à partir de
l'identifiant de fichier qu'enregistre le `meta.ini` du mod : les auteurs marquent un nouvel
envoi comme le successeur d'un plus ancien, et MO2 lit la même chaîne. Un fichier plus récent
dans cette chaîne est la mise à jour. Quand la chaîne n'en nomme aucun et que le fichier
installé est toujours listé sur la page, le mod compte comme à jour, même si la page montre
une autre version. La version de la page ne tranche que quand aucun identifiant de fichier
n'est enregistré, quand le fichier installé a été retiré ou archivé, ou quand Nexus n'a pas
envoyé la liste de fichiers.

Chaque mod utilise le domaine Nexus du jeu que nomme son propre `meta.ini` (`gameName`), pas
celui de l'instance : la vérification de mise à jour, Recommander, Suivre et Voir sur Nexus y
vont tous. Enderal SE accepte aussi les mods `skyrimspecialedition`, si bien qu'un mod
Skyrim SE installé dans une instance Enderal SE est vérifié sur sa page Skyrim SE. Un lien
Mod Manager Download va à l'instance que vous avez utilisée en dernier quand son jeu accepte
le domaine du lien. Sinon, il va à une instance du jeu dont c'est le propre domaine, avant
celle d'un jeu qui ne fait que l'accepter. `EIDOS_INSTANCE=<folder>` remplace les deux.

Suivre et Ne plus suivre dans le menu contextuel d'un mod ajoutent le mod à vos mods suivis sur
Nexus ou l'en retirent, si bien que les notifications du site suivent ce que vous suivez
ici. Recommander et S'abstenir dans le même menu approuvent le mod sur Nexus ou retirent
l'approbation ; le bouton d'approbation à côté du bouton de mise à jour approuve le mod
sélectionné. Chaque vérification de mise à jour lit aussi vos approbations et vos mods suivis
sur Nexus (deux requêtes) et règle les drapeaux Recommandé et Suivi de chaque mod pour qu'ils
correspondent, sur le domaine Nexus propre à ce mod. Quand les deux divergent, Nexus gagne. Si
l'une des listes ne peut pas être lue, la vérification le dit et laisse ces drapeaux comme ils
étaient.

Le bouton **Mettre à jour** de l'onglet Fichiers Nexus d'« Informations... » installe le fichier plus
récent. Avec un compte premium, Eidos télécharge le fichier lui-même. Avec un compte gratuit,
il ouvre la page du fichier, où Mod Manager Download renvoie le fichier, et l'installation
démarre quand l'archive arrive dans `downloads/`. À partir de là, c'est l'installation
habituelle : un FOMOD s'ouvre sur les réponses avec lesquelles le mod a été installé, puis
l'invite propose Remplacer avec « Garder une sauvegarde du mod existant » coché, quoi que vous ayez
choisi à la dernière invite. Remplacer garde la priorité et l'état activé du mod.

Avant qu'un remplacement échange le dossier d'un mod, Eidos compare l'ancien dossier au nouveau par
nom de fichier et par taille et lit les en-têtes des plugins des deux. Cela couvre Mettre à jour, une
mise à jour de collection et Remplacer choisi à l'invite d'installation. La ligne d'état
compte alors les fichiers ajoutés, retirés et changés de taille, et nomme chaque master dont la
nouvelle version a besoin et que l'ancienne n'avait pas. L'onglet **Général** de la boîte
« Informations... » se termine par une section **Dernière mise à jour** qui liste les versions, les
plugins ajoutés ou retirés, les masters ajoutés ou retirés, les plugins dont l'indicateur ESM,
ESL ou ESH a changé, les mods avec lesquels celui-ci partage maintenant des fichiers ou ne
partage plus, et les fichiers. Un fichier réécrit à la même taille n'est pas listé. Le rapport
est gardé dans `logs/updates/<mod>.json` dans l'instance, que `eidos pack` laisse de côté ; une
installation neuve ou une fusion (Fusionner) dans le dossier le supprime. `eidos install --replace` affiche
les mêmes comptes sur une ligne `changes:`.

Un dossier qui contient plusieurs fichiers Nexus, comme un fichier principal avec un fichier
facultatif fusionné dedans, est refusé : le remplacer par le nouveau fichier principal
laisserait tomber les autres. Mettez ceux-là à jour à la main.

`eidos nexus update skyrimse --apply` lance la même vérification, puis, avec un compte premium,
télécharge chaque nouveau fichier et l'installe par-dessus son dossier avec
`--replace --backup`. Avec un compte gratuit, il affiche à la place `OPEN <url>` pour chaque
page de fichier. Les dossiers qui fusionnent plusieurs fichiers sont sautés avec la raison. Un
mod FOMOD sans réponses enregistrées, comme un installé avant qu'Eidos les garde, n'est pas
réinstallé avec les valeurs par défaut de l'installateur : cette mise à jour échoue avec un
message disant de mettre le mod à jour une fois depuis la fenêtre, ce qui enregistre les
réponses. Un FOMOD dont l'installateur n'a rien à choisir est installé comme d'habitude. La
commande se termine avec le code 1 si une mise à jour a échoué.

### Mises à jour de collections

Quand vous lancez Vérifier les mises à jour, Eidos demande aussi à Nexus la dernière révision de chaque
collection installée dans cette instance. Une ligne au-dessus de la liste de mods nomme chaque
collection qui a une révision plus récente et compte les mods ajoutés (+), mis à jour (~) et
abandonnés (-) ; survolez-la pour leurs noms. Les comptes manquent quand la recette de la
révision installée n'est plus en cache. **Mettre à jour** ouvre la révision plus récente dans le volet
des collections, où *Installer cette collection* l'exécute. La liste est reconstruite à chaque
vérification et n'est pas enregistrée.

### Réglages Nexus

**Préférences -> Nexus -> Hors ligne** empêche Eidos de contacter Nexus du tout, et saute la
vérification quotidienne de version sur GitHub. Les vérifications de mises à jour, la connexion,
les téléchargements et les collections le disent au lieu d'échouer avec une erreur de
connexion. C'est désactivé par défaut.

**Serveurs préférés** classe les nœuds CDN qu'un téléchargement préfère, le meilleur d'abord.
Seul un compte premium reçoit jamais plus d'un miroir parmi lesquels choisir, si bien que pour
tous les autres Nexus choisit et cela ne change rien. Quand aucun des serveurs que vous avez
nommés n'est proposé, le téléchargement a quand même lieu, depuis le premier nœud que Nexus
propose.

### Catégories

Les catégories peuvent être attribuées à un mod ou à toute une sélection, le catalogue peut
être modifié depuis la même boîte de dialogue, et la liste officielle des catégories du jeu
peut être tirée de Nexus. Les deux fichiers de catalogue sont ceux de MO2 (`categories.dat` et
`nexuscatmap.dat`), si bien qu'une instance partagée garde un seul catalogue. Un mod téléchargé
depuis Nexus est catégorisé à l'installation, une fois que Récupérer depuis Nexus a fait
correspondre le catalogue.

### Éditeur d'INI, journal et extensions

**Affichage -> Éditeur d'INI...** modifie les INI du jeu du profil, la copie qu'Eidos garde ; la copie
dans le préfixe Proton est écrasée à chaque lancement. Son dernier onglet est le
`launch-env.ini` du profil
([Variables et wrappers par profil](#variables-et-wrappers-par-profil)).
**Affichage -> Journal...** lit les journaux de session. **Affichage -> Extensions...** liste vos propres
extensions ; voir [extensions.md](extensions.md).

Quand un lancement ne trouve l'INI principal du jeu (`Skyrim.ini`, `Fallout4.ini`,
`Oblivion.ini`...) ni dans le profil ni dans le préfixe, Eidos copie d'abord celui par défaut
du dossier du jeu (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) dans le profil, comme le
fait MO2. Quand l'instance exécute une copie du jeu, le défaut vient de la copie. Le préfixe
et le dossier du jeu ne sont pas modifiés.

### Installer des mods

L'installateur gère les archives Simple, FOMOD, les paquets BAIN de Wrye Bash (cochez les
sous-paquets, qui fusionnent dans l'ordre) et les paquets OMOD d'Oblivion. Un script OMOD
s'exécute quand il utilise les commandes OBMM qu'Eidos prend en charge ou qu'il est l'un des
trois installateurs C# qu'Eidos connaît ; Eidos n'exécute aucun autre code C# ou Visual Basic
et le dit. Quand aucune heuristique ne reconnaît la disposition, un sélecteur manuel montre
l'arbre de l'archive et vous laisse désigner la racine des données.

Réinstaller ou mettre à jour un mod FOMOD ouvre l'assistant avec les réponses de sa dernière
installation cochées, sur la première étape dont les options ont changé depuis, avec les
changements listés. **Réinstaller avec de nouveaux choix** part plutôt des valeurs par défaut de
l'installateur. `eidos install --replace` ou `--merge` réutilise les réponses enregistrées et
refuse, en listant les changements, quand elles ne conviennent plus.

### Diagnostics

L'onglet Diagnostics lance des contrôles de santé en direct. Il signale les masters manquants,
les plugins que LOOT signale comme sales, les archives qu'aucun plugin actif ne chargera, si la
liste de mods correspond toujours au dossier des mods, si le passthrough FUSE est activé et,
quand il est demandé, si la capacité de lancement dont il a besoin est là. Après une
exécution, il montre ce que dit le journal propre du script extender sur chacune de ses DLL de
plugin, si bien que vous voyez si vos plugins SKSE se sont chargés. Il signale aussi une mise à
jour du jeu depuis le dernier lancement propre, un script extender absent ou qui ne correspond
pas à la version du jeu (avec la page de téléchargement de l'extender et un bouton
d'installation de l'archive que vous téléchargez), une option de lancement Steam qui ne démarre
pas Eidos avec `%command%`, NVAPI laissé désactivé pour DLSS, le plein écran exclusif qui
bloque la génération d'images, un `enblocal.ini` d'ENB sans `LinuxVersion=true`, les
déclarations de plugins des plugins F4SE et SFSE, et le journal de plantage de la dernière
session avec les mods qu'il nomme.

Les contrôles des INI du profil et des plugins cherchent des lignes `sTestFile` dans `[General]`
des INI personnalisés de Fallout 4, Fallout 4 VR et Starfield, qui font ignorer `plugins.txt` au
jeu (Corriger les retire de la copie du profil et garde le fichier précédent sous le nom
`<ini>.pre-fix`), les plugins `.esp` activés dans Starfield, et les plugins blueprint qui n'ont
pas l'indicateur blueprint, le préfixe `BlueprintShips-` ou l'extension `.esm`, ou dont le
plugin principal n'est pas activé. Les contrôles du système signalent un `vm.max_map_count` ou
une limite stricte de fichiers ouverts sous ce dont Proton a besoin, et un jeu, un préfixe ou
une instance sur NTFS, exFAT ou FAT. Les contrôles de mods nomment le chemin de données de
BodySlide quand il ne pointe pas vers le `Data` du jeu, les dossiers `Data` qu'un outil a
écrits dans `Overwrite/Root`, un patch dont les fichiers libres perdent face au mod qu'il
corrige, et un mod qui remplace les scripts Papyrus du script extender. Le contrôle `sTestFile`
ne s'applique qu'à Fallout 4, Fallout 4 VR et Starfield.

Vérifier les mises à jour lit aussi les prérequis Nexus de chaque mod. Diagnostics liste un mod requis
qui est absent ou désactivé, avec Ignorer pour un prérequis satisfait autrement.

L'onglet montre aussi quel build de Proton exécute le jeu et prévient avant qu'un Proton plus
ancien exécute un préfixe qu'un plus récent a préparé ; Reconstruire le préfixe met de côté un préfixe cassé
pour que Steam en crée un nouveau
([troubleshooting.md](troubleshooting.md#quel-proton-lance-le-jeu-et-un-préfixe-cassé)). Les
autres cartes signalent un préfixe jamais préparé ou dont le registre est périmé, un programme
autre qu'Eidos qui reçoit les liens Mod Manager Download (Enregistrer Eidos pour les liens nxm le corrige), une sortie
d'outil plus ancienne que ses entrées (avec un bouton qui relance l'outil), et une copie
nettoyée ou légère d'un plugin dont l'original a changé depuis. Chaque carte est expliquée dans
[troubleshooting.md](troubleshooting.md#cartes-de-diagnostic). **Copier le rapport** met sur le presse-papiers
un rapport du système et de chaque constat, votre dossier personnel et votre nom de connexion
masqués, pour un rapport de bug.

Pour Skyrim SE, Enderal SE et Fallout 4, Diagnostics a aussi une carte d'affichage. Elle montre la
taille et le mode de fenêtre réglés dans le `SkyrimPrefs.ini` du profil (`EnderalPrefs.ini`,
`Fallout4Prefs.ini`), ou dans la copie du préfixe quand le profil n'en a pas encore. Quand
aucun ne règle de taille, ses champs partent de l'écran connecté le plus grand listé dans
`/sys/class/drm`, si bien qu'un Steam Deck lit 1280x800. La largeur et la hauteur sont des
nombres entiers de 320 à 16384. Appliquer écrit `iSize W`, `iSize H`, `bFull Screen` et
`bBorderless` dans la copie du profil, après avoir adopté la copie du préfixe si le profil n'en
a pas. Quand SSE Display Tweaks règle la fenêtre, Appliquer écrit la même taille et le même mode
dans sa copie dans l'Overwrite ; le fichier du mod n'est pas changé.

La carte Fichiers du jeu en haut de l'onglet est décrite sous
[Fichiers du jeu](#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie), la carte d'ordre de collection sous
[Garder l'ordre de la collection](#garder-lordre-de-la-collection), et Trouver le mod à l'origine d'un
problème sous [Parties test](#trouver-le-mod-à-lorigine-dun-problème).

### Lancer le jeu

Pour lancer le jeu par la fenêtre, mettez comme option de lancement Steam du jeu le chemin
absolu du programme, parce que Steam ne lit pas le `PATH` de votre shell
([install.md](install.md#ensuite--pointer-steam-dessus)) :

```
~/.local/bin/eidos-gui %command%
```

Depuis Colony, le chemin est `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos s'ouvre sur
l'instance du jeu, celle que vous avez utilisée en dernier, si bien qu'une instance portable est
retrouvée comme l'est la globale ; cliquez sur Lancer pour le lancer à travers la vue fusionnée.

Avec **Lancer le jeu directement quand une boutique ouvre Eidos** activé (Préférences, Général, Pendant le
jeu ; désactivé par défaut), Eidos saute ce clic : il vérifie le diagnostic et démarre le jeu sans
montrer sa fenêtre. La fenêtre s'ouvre comme d'habitude quand le diagnostic signale un problème, quand la
liste de mods demande de l'attention, quand un outil réglé pour s'exécuter avant de jouer a une
sortie périmée, ou, pour Skyrim SE, Enderal SE et Fallout 4, quand le profil n'a pas encore de
taille de fenêtre (la fenêtre s'ouvre alors sur Diagnostics). Pour avoir la fenêtre pour un
lancement, mettez `EIDOS_WINDOW=1` devant la commande :
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

Dans une fenêtre qu'aucune boutique n'a ouverte, Lancer ne démarre pas le jeu lui-même, et Eidos ne
démarre jamais un jeu directement par Proton ou Wine. Lancer demande à la boutique qui possède la
copie de le démarrer, et la boutique démarre Eidos :

- Steam exécute `steam -applaunch <appid>` quand l'option de lancement ci-dessus est définie et
  que le jeu a un préfixe Proton, que Steam crée la première fois qu'il démarre le jeu. Steam
  démarre d'abord s'il est fermé.
- Heroic (une copie GOG ou Epic, native ou Flatpak) reçoit un lien `heroic://launch` quand Eidos
  est un wrapper dans les réglages du jeu dans Heroic, avec le chemin absolu d'`eidos-gui` (ou
  d'`eidos-linux`) comme exécutable et sans arguments. Heroic démarre d'abord s'il est fermé.
- Legendary seul exécute `legendary launch <app>` quand `~/.config/legendary/config.ini` a
  `wrapper =` et le chemin absolu d'`eidos-gui` (ou d'`eidos-linux`) sous la section `[<app>]` du
  jeu, ou sous `[default]`.

Le chargeur du script extender, le launcher et l'exécutable du jeu dans la liste Lancer sont tous
confiés à la boutique de cette façon, et il en va de même pour
`eidos tool <instance> run <entry>` pour l'un d'eux dans un terminal. Eidos ne peut pas confier
certaines copies à leur boutique, comme une copie GOG hors d'Heroic : Lancer dit alors de démarrer
le jeu depuis cette boutique.

Le transfert enregistre cette instance comme celle que vous avez utilisée en dernier, et la
boutique ouvre alors une seconde fenêtre d'Eidos dessus. Avec Lancer le jeu directement désactivé,
vous cliquez sur Lancer là ; avec lui activé, cette fenêtre démarre le jeu sans se montrer, sauf si
l'un des cas ci-dessus l'ouvre. La première fenêtre ne suit pas cette exécution, donc appuyez sur
F5 dedans après avoir joué pour recharger ses listes. Quand le réglage de la boutique manque, ou
que le chargeur du script extender n'est pas à côté de l'exécutable du jeu, Lancer ne démarre rien et
montre quoi régler et où, avec le vrai chemin du binaire en cours.

Pendant qu'un jeu ou un outil tourne, Lancer devient Arrêter. Deux appuis terminent tous les programmes
que l'exécution a démarrés, et Eidos capture alors la session et démonte comme après toute
fermeture ; une exécution arrêtée ne compte jamais comme un lancement propre
([troubleshooting.md](troubleshooting.md#un-jeu-ou-un-outil-qui-ne-se-ferme-pas)).

Le `%command%` de Steam pour les titres Bethesda pointe d'ordinaire vers `<Game>Launcher.exe`.
Eidos ne l'exécute jamais : le lanceur est une application de réglages distincte qui rebalaye
`Data` et réécrit `plugins.txt`, annulant l'ordre de chargement qui vient d'être déployé. Eidos
exécute plutôt le chargeur du script extender s'il y en a un d'installé, le binaire du jeu sinon,
et le dit quand il doit se rabattre sur le binaire du jeu.

Eidos analyse les DLL des mods activés à la recherche d'un import de `d3dcompiler_47.dll`, copie
la DLL de Microsoft dans le préfixe et ne définit qu'ensuite `WINEDLLOVERRIDES=d3dcompiler_47=n,b` ;
vous n'avez pas besoin de le définir vous-même.

### Variables et wrappers par profil

Un profil peut porter ses propres variables d'environnement et commandes wrapper dans
`profiles/<name>/launch-env.ini`, une entrée par ligne :

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Elles s'appliquent à chaque `eidos play` de ce profil (le jeu démarré par le `%command%` de Steam,
ou par Lancer dans une fenêtre qu'une boutique a ouverte) et à chaque exécution d'outil.
`eidos shell` reçoit les variables et aucun wrapper. La première ligne `wrapper=` est la plus
externe. Une ligne `wrapper=` qui contient `%command%` reçoit la commande à cet endroit, comme dans
les options de lancement de Steam ; une ligne sans lui reçoit la commande à la fin. Les wrappers
s'exécutent dans l'espace de noms privé d'Eidos, si bien qu'ils voient les mods.

Une variable ici remplace la valeur qu'Eidos définit pour le même nom, et la ligne `env=` propre
à un outil dans `tools.ini` remplace les deux. `WINEDLLOVERRIDES` est plutôt ajouté après les
surcharges qu'Eidos définit pour ENB, ReShade et les DLL fournies, si bien qu'une DLL nommée dans
les deux prend le réglage du profil et que les autres gardent le leur.

Affichage -> Éditeur d'INI... ouvre le fichier comme dernier onglet, pour chaque jeu. Les lignes vides et les
lignes qui commencent par `#` ou `;` sont sautées. L'enregistrement liste les numéros de ligne
qu'Eidos ignore : les lignes qui ne sont ni `env=NAME=value` ni `wrapper=command`, et une ligne
`wrapper=` dont le premier mot est `NAME=value`, comme le permettent les options de lancement de
Steam ; mettez plutôt cette variable sur une ligne `env=`. `eidos play <instance>` sans commande
liste les variables qu'une exécution reçoit et les wrappers. Copier le rapport dans Diagnostics nomme
les variables et les programmes wrapper, sans leurs valeurs.

## Outils

xEdit, BodySlide, DynDOLOD et d'autres outils s'exécutent à travers la vue fusionnée à l'intérieur
du préfixe Wine du jeu (le préfixe Proton de Steam, ou celui qu'Heroic a créé pour une copie GOG
ou Epic) :

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # lancer un exe une fois sans l'enregistrer
eidos prereqs skyrimse            # ce dont les outils enregistrés ont besoin, et leur état
eidos prereqs skyrimse --install  # récupérer ce qui manque
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen et Synthesis sont trouvés sans qu'on
les enregistre. Pandora, Nemesis, FNIS, BodySlide et Synthesis arrivent avec un mod de capture
déjà renseigné, si bien que leur sortie va dans un mod à elle (détails dans
[tools.md](tools.md#envoyer-la-sortie-dun-outil-dans-son-propre-mod)).

Le titre que vous donnez à un outil décide des runtimes et des DLL qu'Eidos lui fournit :
`BodySlide` reçoit ses bibliothèques DirectX, `BS` ne reçoit rien. Dans la fenêtre, la boîte de
dialogue Exécutables montre l'état réel de chaque prérequis sous le champ, et ceux qui manquent
sont des boutons.

Le paquet de version porte les installateurs vcrun2022 (x86 et x64), le .NET 8 Desktop Runtime
8.0.12 et le runtime .NET 10 10.0.10 dans `runtimes/`, et `install.sh` les copie dans
`~/.local/share/Colony/Eidos/runtimes/`, si bien que `eidos prereqs <instance> --install` les
installe sans téléchargement. Eidos exécute en silence dans le préfixe du jeu les installateurs
vcrun2022 et .NET 8 de Microsoft, sans winetricks. `eidos-linux` et les compilations depuis les
sources n'ont pas de dossier `runtimes/`, donc ils téléchargent chez Microsoft les mêmes fichiers
épinglés. Le SDK .NET 10 dont Synthesis a besoin (environ 300 Mo) est toujours un téléchargement.

Le tableau, les trois paliers de prérequis, pourquoi DynDOLOD a besoin d'un runtime .NET que
winetricks ne sait pas installer, et pourquoi un outil installé comme mod est lancé depuis le
chemin fusionné plutôt que depuis son propre dossier sont dans [tools.md](tools.md).

## Extensions

Eidos peut être étendu sans être recompilé : un manifeste TOML dans
`~/.config/Colony/Eidos/addons/` ajoute un outil à la liste Extensions ou un contrôle à l'onglet
Diagnostics. Rien n'est chargé dans Eidos : une extension est un programme qu'il exécute. Voir
[extensions.md](extensions.md).

## Essayer la preuve de concept

Le script n'a besoin d'aucun jeu. Il montre la vue fusionnée, la copie sur écriture, un dossier de
jeu laissé intact, et une fusion qui n'existe que dans son espace de noms, en n'utilisant que
OverlayFS sans privilèges dans un espace de noms d'utilisateur (Linux 5.11 ou plus récent) :

```sh
./scripts/poc-overlay.sh
```
