<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# Dépannage et diagnostics

Les interrupteurs d'environnement, la lecture des compteurs d'opérations, les
cartes Health de l'onglet Diagnostics, comment trouver le mod derrière un
plantage, quoi joindre à un rapport de bug, et les problèmes connus avec leur
histoire.

## Diagnostiquer le VFS

Deux variables d'environnement couvrent la plupart des cas où le jeu voit quelque
chose que le système de fichiers ne confirme pas :

```sh
EIDOS_FUSE_STATS=1                  # compteurs d'opérations, vidés au démontage
EIDOS_FUSE_NO_CACHE=1               # tous les caches côté noyau désactivés
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # ou nommez-les un par un
```

La forme granulaire est ce qui a trouvé le plantage `FOPEN_KEEP_CACHE` décrit dans
les problèmes connus : tout couper répond à « est-ce le cache ? », et seul le fait
de les nommer répond à « lequel ». Les compteurs répondent à l'autre moitié. Avec le
passthrough activé (il est désactivé par défaut), un chargement qui affiche
`read 0` est un chargement où `FUSE_PASSTHROUGH` a servi chaque octet dans le noyau,
donc tout ce que vous vous apprêtiez à régler sur le chemin de lecture est déjà
gratuit.

Quelques interrupteurs de plus réduisent encore le champ :

```sh
EIDOS_FUSE_TRACE=open,opendir       # journalise chaque open / opendir (1 = tous les canaux)
EIDOS_FUSE_THREADS=1                # un seul thread du démon (défaut : nombre de CPU, 4 au plus)
EIDOS_FUSE_OPENDIR=1                # garde les handles opendir même quand le noyau peut s'en passer
EIDOS_NO_INDEX=1                    # parcourt les couches au lieu d'utiliser l'index des couches
EIDOS_NO_OW_INDEX=1                 # idem pour l'index d'appartenance de l'Overwrite
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE de nouveau activé, voir les problèmes connus
```

Pour un lancement Steam, mettez-les devant la commande dans l'option de lancement,
par exemple `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. La sortie va dans
le journal de l'exécution : le `logs/run-*.log` de l'instance pour un lancement
depuis la fenêtre, la console de Steam pour un lancement Steam.

Pour regarder la vue fusionnée elle-même, `eidos shell <instance>` ouvre un shell
dans le dossier du jeu tel que le jeu le voit, et
`eidos shell <instance> -- ls Data/meshes` y exécute une seule commande ; tout ce qui
est écrit atterrit dans l'Overwrite
([usage.md](usage.md#un-shell-dans-la-vue-fusionnée)). Le bouton Save tree de l'onglet
Data écrit tout l'arbre `Data` fusionné dans un fichier texte, un fichier par ligne
avec le mod d'où il vient, votre dossier personnel étant masqué.

## Monter une union à la main

`eidos-fuse` vient avec le paquet Arch et avec les compilations depuis les sources :
`just build` le compile, et `just install` ou `install.sh` le copie quand il est
là. Le paquet de version et `eidos-linux` ne le contiennent pas. Sa ligne d'usage :

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

La première `--layer` gagne en cas de conflit ; la dernière est vos données de jeu
intactes. Le montage n'a besoin que de `/dev/fuse` et de `fusermount3` (pas
d'overlayfs, pas de Wine) :

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... lisez et écrivez à travers /mnt/point ...
fusermount3 -u /mnt/point
```

Les écritures atterrissent dans `--overwrite <dir>` (un dossier temporaire si
omis), donc les couches elles-mêmes restent intactes, même ici.

## Pourquoi le passthrough est désactivé par défaut

Le passthrough donne au noyau le vrai fichier sous-jacent, si bien que les
lectures contournent entièrement ce démon. L'A/B ci-dessous a été mesuré avec
une version antérieure du démon, avant le changement décrit plus bas dans le
problème connu « Le passthrough FUSE empêche le jeu de charger le moindre
contenu de mod ». Ce changement évite la cause la plus probable, mais n'a pas
encore été remesuré en jeu, donc le passthrough reste désactivé par défaut
jusqu'à ce qu'il le soit. Mesuré en A/B sur Skyrim SE 1.6.1170, proton-cachyos
11.0, noyau 7.1.4, le même ordre de chargement de 82 plugins, la seule variable
étant la présence de la capacité sur le binaire :

| passthrough | échecs `NtCreateFile` en `STATUS_ACCESS_VIOLATION` |
|-------------|-----------------------------------------------------|
| activé      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`     |
| désactivé   | 0                                                    |

Activé, le jeu n'ouvrait aucune de ses propres archives ni aucun de ses plugins,
ce qui se manifestait en jeu par des mods absents - aucune erreur, aucune ligne
de log. Désactivé, le même ordre de chargement arrivait en jeu avec ses plugins,
ses archives et ses scripts Papyrus vivants.

L'échec était invisible depuis l'intérieur du démon, et c'est ce qui l'a rendu
coûteux à trouver : son propre `open` réussissait à chaque fois et le noyau ne
refusait jamais un fichier sous-jacent (vérifié sur une session complète en
échec avec `EIDOS_FUSE_TRACE=open` : zéro `open FAILED`, zéro
`passthrough refused`). L'erreur était produite après que le démon avait répondu
`opened_passthrough`, donc aucune journalisation côté démon ne pouvait la voir.
Ce n'était pas non plus lié à l'extension - cela touchait archives et plugins
pareillement, c'est-à-dire les fichiers que le jeu garde ouverts pendant toute
son exécution.

`EIDOS_FUSE_PASSTHROUGH=1` le réactive, pour mesurer ce qu'il apporte ou pour
retester le mécanisme. Les avertissements de capacité dans le lanceur et dans
l'onglet Diagnostics n'apparaissent que si vous l'avez demandé.

## L'index des couches est-il réellement utilisé ?

`LayerStack::new` construit soit une carte complète des couches en lecture
seule, soit aucune. Sans carte, chaque requête parcourt les couches comme avant :
les réponses sont les mêmes, un chargement de sauvegarde prend environ trois fois
plus longtemps, et rien d'autre ne change en apparence.

Quand l'index est désactivé, le montage en donne la raison, une fois, au
démarrage :

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

La raison nomme un dossier illisible, une couche à plus de 64 niveaux de
profondeur, plus de 4 000 000 d'entrées, ou `EIDOS_NO_INDEX`. La ligne va dans le
journal de l'exécution : le `logs/run-*.log` de l'instance pour un lancement
depuis la fenêtre, la console de Steam pour un lancement Steam. Avec
`EIDOS_FUSE_STATS=1`, le relevé au démontage contient aussi une ligne
`layer index:`, et un compteur `unindexed` non nul sur un montage normal signifie
que l'index était désactivé.

Les exemples ci-dessous vérifient la même chose hors ligne, sur les dossiers
d'une instance, sans montage :

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` résout de vrais chemins avec et sans l'index et compare les
parcours de dossiers. `index_agrees` vérifie que les deux répondent LA MÊME chose,
sur chaque chemin et chaque listage d'une instance réelle. `listing_cost` mesure
ce que la carte des enfants fusionnés économise sur `readdir`.

`EIDOS_NO_INDEX=1` force le parcours, pour quand la différence entre les deux
réponses est précisément ce qu'on débogue.

## Lancer le jeu avec la CLI

Pour lancer le jeu par la CLI `eidos` plutôt que par la fenêtre, mettez comme option
de lancement Steam le chemin absolu d'`eidos`, par exemple :

```
~/.local/bin/eidos play skyrimse -- %command%
```

Aucun préfixe `WINEDLLOVERRIDES` n'est nécessaire. Quand un mod activé importe
`d3dcompiler_47`, Eidos déploie sa copie fournie dans le préfixe, la force en
natif, et fusionne cela avec les surcharges de DLL qu'un mod embarque (chargeurs
ENB/ReShade/`.asi`).

Eidos ne démarre jamais le jeu de lui-même par Proton.
`eidos tool <instance> run <title>` avec une entrée qui démarre le jeu (le chargeur
du script extender, le lanceur ou le binaire du jeu) demande à la boutique qui
possède la copie de le démarrer, comme le fait Run dans une fenêtre qu'aucune
boutique n'a ouverte, et affiche quoi régler quand le réglage de la boutique ne
démarre pas Eidos. `--print` affiche la commande de la boutique au lieu de
l'exécuter.

## Un jeu ou un outil qui ne se ferme pas

Pendant qu'un jeu ou un outil tourne, le bouton Run devient Stop, et la carte de
verrouillage a elle aussi un bouton Stop. Appuyez dessus deux fois. Le second appui
ferme tous les programmes que l'exécution a démarrés : SIGTERM d'abord, puis SIGKILL
au bout de 10 secondes pour ce qui reste. Ce que le jeu n'avait pas sauvegardé est
perdu. Eidos fait ensuite ce qu'il fait après toute fermeture : les changements
d'INI et de plugins retournent dans le profil, la sortie des outils est enregistrée
et les mods sont démontés. L'exécution est enregistrée avec le code de sortie 143
quoi qu'ait renvoyé le programme, si bien que `eidos sessions <instance> diff` ne la
prend jamais pour le dernier bon lancement. Unlock est différent : il rend la
fenêtre et laisse le programme tourner. Depuis un terminal, `kill <pid>` sur le
processus `eidos` (le pid est sur la carte de verrouillage) fait la même chose que
Stop.

## Trouver ce qui a cassé

Pour trouver quel changement a cassé le jeu :

- `eidos sessions <instance> diff` liste ce qui a changé depuis la dernière
  exécution du jeu terminée avec le code 0 : mods, versions, ordre des plugins, INI
  et exécutable du jeu
  ([usage.md](usage.md#ce-qui-a-changé-depuis-le-dernier-bon-lancement)).
- Test run, à côté de Run, joue avec les mods du profil, sans mods, ou avec les mods
  sélectionnés et les masters dont ils ont besoin, et envoie tout ce que le jeu écrit
  dans `<instance>/test-run/`. Après l'exécution, vous gardez ou jetez chaque groupe
  de fichiers : sauvegardes, modifications d'INI, état des plugins, production
  d'Overwrite et journaux ([usage.md](usage.md#parties-test)).
- Find the mod behind a problem, dans l'onglet Diagnostics, lance des test runs avec
  la moitié des suspects restants à chaque fois et demande si le problème s'est
  produit. 200 suspects demandent environ 8 exécutions, et la recherche survit à la
  fermeture d'Eidos
  ([usage.md](usage.md#trouver-le-mod-à-lorigine-dun-problème)).

Une exécution arrêtée ne pré-remplit jamais un Yes dans cette recherche, parce
qu'elle est enregistrée avec le code de sortie 143 (voir ci-dessus).

## Cartes Health

L'onglet Diagnostics montre une carte Health pour chaque problème qu'il trouve, avec
sa cause et, pour certaines, un bouton qui le corrige. Les cartes de l'option de
lancement sont décrites dans [install.md](install.md), les cartes NVAPI, plein
écran et ENB dans [graphics.md](graphics.md), et les cartes Proton et préfixe dans la
section suivante. `eidos check <instance>` affiche les constats qui n'ont pas besoin
de fenêtre ([usage.md](usage.md#contrôles-de-santé-depuis-le-terminal) les liste), dont les
cartes du préfixe, du registre et du lecteur `S:`. Seule la fenêtre montre la carte
Proton avec ses variantes Flatpak, build inconnu et build plus ancien, l'ordre des
patchs, les scripts de l'extender, les archives qu'aucun plugin ne charge, les
prérequis Nexus, les copies périmées de plugins et la carte Display.

### Le système

- « vm.max_map_count is <value> » apparaît en dessous de 1048576, la valeur par
  défaut d'Arch Linux, de Fedora 39 et suivantes, et d'Ubuntu 24.04 et suivantes.
  Certains jeux sous Proton mappent plus de régions mémoire et plantent alors sans
  erreur. Mettez `vm.max_map_count = 1048576` dans un fichier sous `/etc/sysctl.d`,
  par exemple `80-gamecompatibility.conf`, puis lancez `sudo sysctl --system`.
- « The open files limit is <value> » apparaît quand la limite stricte est sous
  524288, ce dont l'esync de Proton a besoin ; les jeux peuvent ne pas démarrer ou
  planter en dessous. systemd 240 et suivants la fixent. Sur une installation plus
  ancienne, mettez `DefaultLimitNOFILE=524288` dans `/etc/systemd/system.conf` et
  `/etc/systemd/user.conf`, puis reconnectez-vous.
- « Game files on a Windows file system » nomme le dossier du jeu, le préfixe ou la
  racine de l'instance quand il est sur `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g,
  exfat-fuse), `exfat` ou `vfat`. Proton a besoin de liens symboliques, de
  permissions Unix et de noms sensibles à la casse. Déplacez le jeu, son préfixe et
  l'instance vers un système de fichiers Linux comme ext4, btrfs ou xfs.

### Les fichiers du jeu

La carte Game files, en haut de l'onglet, montre si l'instance exécute
l'installation de la boutique ou une copie du jeu située dans l'instance
([usage.md](usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie)). Les lancements sont
refusés, avec une carte qui dit pourquoi, dans trois cas :

- « The game copy '<folder>' is missing » : l'instance exécute une copie qui a
  disparu. Remettez-la, ou revenez à la version de la boutique.
- « '<folder>' has no <binary> » : le dossier que l'instance exécute ne contient
  aucun exécutable du jeu, ce n'est donc pas une copie du jeu.
- « <game> cannot run from a copy yet » : Morrowind, et Oblivion avec
  `bUseMyGamesDirectory=0`, gardent leur liste de plugins dans le dossier du jeu.

« ModOrganizer.ini points tools at another game folder » apparaît quand le
`ModOrganizer.ini` propre à une liste, non écrit par Eidos, a un `gamePath` autre
que les fichiers du jeu que l'instance exécute. Les outils en mode Mod Organizer 2,
comme PGPatcher, y lisent le jeu. Changez `gamePath` dans ce fichier, ou supprimez
le fichier pour qu'Eidos écrive le sien.

### L'ordre de chargement

- « sTestFile lines make the game ignore plugins.txt » apparaît pour Fallout 4,
  Fallout 4 VR et Starfield quand le `Fallout4Custom.ini` ou le `StarfieldCustom.ini`
  du profil a des lignes `sTestFile1` à `sTestFile10` dans `[General]`. Tant qu'une
  est définie, le jeu charge ces plugins et l'ordre de chargement qu'Eidos écrit n'a
  aucun effet. Fix retire les lignes de la copie du profil et garde le fichier
  précédent sous le nom `<ini>.pre-fix`.
- Starfield : les plugins `.esp` activés sans l'indicateur master reçoivent une
  carte, parce que Starfield ne peut pas les trier parmi les `.esm` et les plugins à
  indicateur master et garde certains de leurs enregistrements chargés pendant toute
  la session. Une version `.esm` de l'auteur évite les deux.
- Starfield : « Invalid blueprint plugins » liste les plugins qui ont une partie des
  trois critères du plugin blueprint (indicateur, nom commençant par
  `BlueprintShips-`, extension `.esm`) mais pas tous. « Blueprint plugins without
  their main plugin » liste ceux dont le plugin principal n'est pas activé :
  `BlueprintShips-Example.esm` a besoin de `Example.esm`, `.esp` ou `.esl`.
- « <patch> loses <n> file(s) to <master> » apparaît quand un plugin d'un mod a
  besoin d'un master d'un autre mod, et que cet autre mod a la priorité la plus
  haute, si bien que ses copies des fichiers que les deux livrent gagnent. Un patch
  se place d'ordinaire sous le mod qu'il corrige, et le bouton Move de la carte l'y
  met. Gardez l'ordre si le patch est censé perdre ces fichiers.
- « Out-of-date copy of <plugin> » apparaît quand un mod contient une copie nettoyée
  ou à indicateur léger d'un plugin dont l'original a changé depuis (mise à jour d'un
  mod ou du jeu). Le jeu charge l'ancienne copie. Masquez-la, puis nettoyez ou
  signalez de nouveau le nouveau plugin.

### La liste de mods

- « Order rules of the collection <name> that no longer hold » apparaît quand les
  mods d'une collection installée ne sont plus dans l'ordre que demandent ses
  règles. Restore collection order ne remet en place que les mods de cette
  collection, et Ctrl+Z l'annule.
- « <mod> requires <mod>, which is not installed » (ou « which is disabled ») vient
  des prérequis Nexus que lit Check for updates. Open on Nexus ouvre la page du mod
  requis, Download main file le télécharge avec un compte premium, et Ignore masque
  un prérequis qu'un autre mod satisfait déjà. Un prérequis de DLC n'est qu'un
  conseil : Eidos cherche un plugin portant le nom du DLC.
- « fontconfig.txt names a font library that is missing » (moteurs Skyrim) nomme les
  polices qu'aucune couche ni aucune archive ne contient, et le mod dont
  l'`Interface/fontconfig.txt` gagne. Le texte des menus peut s'afficher en
  rectangles tant que la police n'est pas installée ou la ligne retirée.
- « BA2 archive stores audio compressed » (Fallout 4 et Fallout 4 VR) compte les
  fichiers `.wav` et `.xwm` que le BA2 d'un mod stocke compressés. Le jeu peut
  planter quand il les joue. Réempaquetez l'archive avec l'audio non compressé.

### Le script extender et les mises à jour du jeu

- « <loader> is not installed » apparaît quand des plugins de script extender sont
  installés (dans `SKSE/Plugins`, `F4SE/Plugins` et ainsi de suite) mais que le
  chargeur n'est dans aucun mod activé ni dans le dossier du jeu. Le jeu démarre
  sans eux.
- « The script extender does not match game version <version> » apparaît quand le
  chargeur est installé mais pas la DLL d'exécution de la version du jeu en cours,
  par exemple `skse64_1_6_1170.dll`. Le chargeur s'arrête avec une erreur. Cela suit
  en général une mise à jour du jeu. Cette carte et la précédente ont un bouton Open
  the download page pour le site de l'extender et un bouton Install the downloaded
  archive, qui place le chargeur dans le dossier `Root/` du mod.
- « <game> changed since the last clean launch » apparaît quand l'exécutable du jeu,
  ou pour une copie Steam son identifiant de build, a changé depuis le dernier
  lancement par Eidos qui s'est terminé sans erreur. Une mise à jour Steam qui laisse
  l'exécutable tel quel ne la déclenche pas. Les plugins de script extender compilés
  pour l'ancienne version peuvent ne plus se charger, et les cartes en dessous
  nomment ceux qui le déclarent. Pour garder la version du jeu sur laquelle une liste
  a été bâtie, figez-la dans l'instance avec Freeze this version sur la carte Game
  files, ou avec `eidos stock-game <instance> freeze --yes` après que `freeze` seul a
  montré la taille : les mises à jour ultérieures de la boutique changent
  l'installation de la boutique, pas la copie.
- « <mod> replaces <n> <extender> script(s) » apparaît quand un mod de priorité plus
  haute que le script extender a des copies libres `.pex` des scripts propres à
  l'extender. Une copie ancienne casse les fonctions qu'elle déclare, et le jeu
  n'affiche aucune erreur. Placez l'extender sous ce mod (le bouton Move de la carte
  le fait quand les deux sont des mods), ou masquez ces fichiers dans le mod. Quand
  les scripts de l'extender sont libres dans le `Data` du jeu, installez l'extender
  comme mod pour qu'il puisse être ordonné. Les archives ne sont pas vérifiées,
  seulement les fichiers libres.

### La sortie des outils

- « Overwrite/Root holds meshes » (ou textures) veut dire qu'un outil dont le chemin
  de données pointait vers le dossier du jeu a écrit ces dossiers là, où le jeu ne
  les lit jamais. Corrigez le chemin de l'outil, supprimez les dossiers et relancez
  l'outil. Le bouton Open folder ouvre `Overwrite/Root`.
- « BodySlide's GameDataPath is not the game's Data folder » (ou `OutputDataPath`) :
  voir [Outils](tools.md#les-réglages-propres-à-un-outil-restent-les-siens).

### Un plantage dans la dernière session

« The game crashed in the last session » apparaît quand un enregistreur de plantages
a écrit un journal pendant le dernier lancement. La carte nomme l'exception et les DLL
et plugins du journal que vos mods ou l'Overwrite installent. Prenez-les comme des
pistes : un mod nommé dans un journal de plantage est souvent l'endroit où le
plantage s'est vu, pas sa cause. Quand le journal ne nomme aucun fichier de vos
mods, la carte le dit. Open the log l'ouvre, et Copy the log (home path hidden) le
copie avec votre dossier personnel masqué.

### La taille de la fenêtre

Pour Skyrim SE, Enderal SE et Fallout 4, une carte Display montre la taille et le
mode de fenêtre avec lesquels le profil démarre le jeu, et dit « No window size set
for this profile » quand ni l'INI Prefs du profil ni celui du préfixe n'en définit
une. Apply écrit la taille et le mode dans la copie du profil, et dans une copie de
l'INI de SSE Display Tweaks dans l'Overwrite quand ce mod règle la fenêtre
([usage.md](usage.md#diagnostics)).

## Quel Proton lance le jeu, et un préfixe cassé

L'onglet Diagnostics a une carte Proton : la version qu'Eidos et Steam utilisent pour
le jeu, le réglage propre à Steam, et la version qui a lancé le préfixe en dernier.
Quand un Proton plus récent que celui choisi a préparé le préfixe en dernier, la
carte devient un conseil, car un Proton plus ancien sur un préfixe plus récent peut
réinitialiser ses fichiers Wine ou ne pas démarrer. Choisissez une version au moins
aussi récente sous Propriétés du jeu, Compatibilité dans Steam, ou reconstruisez le
préfixe.

Les autres cartes Proton et préfixe :

- « Proton comes from the Flatpak Steam install » : ce Proton garde son runtime et
  ses bibliothèques steamclient dans le bac à sable du Flatpak, donc l'exécuter
  depuis l'hôte peut échouer. Eidos ne le démarre pas par `flatpak run`, ce qui
  mettrait le jeu dans le bac à sable, hors de la vue fusionnée. Installez un Proton
  dans `~/.steam/root/compatibilitytools.d/` et choisissez-le pour le jeu.
- « Eidos cannot tell which Proton runs this game » : le `config.vdf` de Steam ne
  nomme aucun outil de compatibilité pour lui, et le préfixe n'enregistre aucun
  build installé. Les outils ne démarrent pas tant qu'un n'est pas choisi dans Steam
  (Propriétés du jeu, Compatibilité).
- « <game> has no Proton prefix yet » : lancez le jeu une fois par Steam, puis
  appuyez sur F5. Si Steam ne crée pas le préfixe, vérifiez que le dossier du jeu
  sous `steamapps/compatdata` est accessible en écriture ; un dossier vide
  appartenant à root le bloque sans message.
- « Windows tools cannot find <game> » : l'enregistrement du chemin d'installation
  dans le préfixe est faux ou absent, et xEdit, DynDOLOD, Wrye Bash et le Creation
  Kit le lisent. Eidos écrit le bon chemin la prochaine fois que vous lancez un
  outil.
- « The prefix's S: drive points outside the Steam library » : `S:` ne correspond ni
  à la racine de la bibliothèque, où Eidos la pointe pour un outil, ni à
  `<library>/steamapps`, où Steam la pointe pour le jeu. La bibliothèque a
  probablement été déplacée, ou le préfixe vient d'une autre machine. Tout chemin
  relatif à `S:` enregistré dans le préfixe ne mène nulle part jusqu'au prochain
  lancement par Eidos ou Steam qui la pointe de nouveau.
- « <store> installation: started through its store » est la carte d'une copie GOG
  ou Epic. Run demande à Heroic ou à Legendary de démarrer le jeu quand Eidos y est le
  wrapper du jeu, et les outils s'exécutent avec le build de Wine ou de Proton
  qu'Heroic enregistre pour la copie.

Le menu File lance winecfg, regedit, l'explorateur de fichiers de Wine et le
désinstalleur dans le préfixe du jeu, avec le Proton qui exécute les outils du jeu
([tools.md](tools.md#les-programmes-propres-à-wine-dans-le-préfixe-du-jeu)). Leurs lignes restent
grisées tant que le préfixe n'a pas ces programmes.

Le bouton Rebuild prefix de la carte Proton sert pour un préfixe qui ne fonctionne
plus. Appuyez dessus deux fois. Eidos ne supprime rien : il renomme
`steamapps/compatdata/<appid>` en `<appid>.eidos-backup-<date>-<time>` dans le même
dossier, et Steam crée un nouveau préfixe au prochain démarrage du jeu depuis Steam.
Il refuse tant qu'un jeu, un outil ou n'importe quel programme Wine utilise le
préfixe. Tant que le nouveau préfixe n'existe pas, Run dans la fenêtre ne peut pas
confier le jeu à Steam et montre l'option de lancement à la place : appuyez alors sur
Jouer dans Steam. Les sauvegardes et réglages gardés seulement dans l'ancien préfixe
restent dans le dossier renommé : recopiez ce dont vous avez besoin, et supprimez ce
dossier vous-même quand vous n'en voulez plus.

## Quoi joindre à un rapport de bug

- Copy report, dans l'onglet Diagnostics, copie la version d'Eidos, le jeu, le build
  de Proton, si Steam est le Flatpak, l'état du passthrough, le type de session, la
  distribution, le noyau, `vm.max_map_count`, la limite de fichiers ouverts, si
  `/dev/fuse` et `fusermount3` sont là, les noms des variables de lancement et des
  programmes wrapper du profil, et chaque carte Health. Votre dossier personnel
  apparaît comme `~` et votre nom de connexion comme `<user>`.
  `eidos check <instance>` affiche le même en-tête dans un terminal, et `--json` le
  donne en JSON.
- Le journal de l'exécution, `<instance>/logs/run-*.log`, commence par la commande
  exacte qu'Eidos a exécutée. File, Eidos logs ouvre le dossier des journaux
  propres d'Eidos, `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), sur la carte de plantage, copie le journal de
  plantage.
- Save tree, dans l'onglet Data, enregistre l'arbre `Data` fusionné avec la source
  de chaque fichier.

## Problèmes connus

### DLSS ou la génération d'images ne fait rien, en silence

Trois causes distinctes, chacune sans le moindre message d'erreur : NVAPI non activé
dans les options de lancement, plein écran exclusif, ou un plafond d'IPS Reflex
périmé. La liste complète est dans [graphics.md](graphics.md).

### Run ne fait rien avec le Steam Flatpak

Dans une fenêtre qu'aucune boutique n'a ouverte, Run demande à Steam de démarrer le
jeu en exécutant `steam -applaunch <appid>`. Le Steam Flatpak ne met aucun `steam`
dans le `PATH` de l'hôte, donc la ligne d'état dit « Could not start Steam » avec
l'erreur. Appuyez plutôt sur Jouer dans Steam.

### Une sauvegarde venue d'un autre appareil n'apparaît pas

Une sauvegarde qui arrive dans le préfixe sans passer par Eidos, comme un
téléchargement Steam Cloud depuis un Steam Deck, reste cachée tant que les
sauvegardes du profil sont montées. Rien ne la supprime. L'onglet Saves compte ces
sauvegardes sous son titre, et Import les copie dans le profil sans remplacer une
sauvegarde du profil ([usage.md](usage.md#les-sauvegardes-appartiennent-au-profil)).

### Un mod qui écrit un dossier de deux façons a perdu tout ce qui était sous la seconde

Corrigé. ext4 distingue `meshes/` de `Meshes/` ; la vue fusionnée ne
doit pas, et de vrais mods livrent les deux - XP32 Maximum Skeleton a ses
animations et son fichier de comportement FNIS sous la version capitalisée, ses
`character assets` sous l'autre.

Le résolveur prenait la correspondance de casse exacte pour chaque composant de
chemin et s'y tenait : il entrait dans `meshes/`, n'y trouvait pas la suite du
chemin, et abandonnait LA COUCHE ENTIÈRE. Tout fichier sous l'autre orthographe
était invisible pour le jeu - aucune erreur, aucun log, rien dans aucun
diagnostic. Sur une vraie instance de 50 couches, cela faisait 74 fichiers.

Un composant qui correspond est désormais un candidat, pas une décision ; la casse
exacte est toujours essayée en premier, et ce n'est que lorsque le reste échoue
en dessous que le parcours cherche des voisins équivalents à la casse près. Les
listages avaient le même défaut un dossier plus haut et lisent maintenant chaque
dossier équivalent à la casse près, par couche.

L'index de chemins n'a jamais eu ce bug, parce qu'il parcourt tous les dossiers
qu'il trouve. Pendant un temps, l'index rendait donc des fichiers que le repli ne
pouvait pas rendre, alors que le repli est la réponse censée ne jamais se tromper.

### LODGen de DynDOLOD meurt en laissant un journal vide

Corrigé par `dotnet10` ; voir [tools.md](tools.md). Le
`LODGen_SSE_<world>_log.txt` de chaque monde contient une bannière de version, une
ligne `.NET Version:` et rien d'autre, et une boîte de dialogue dit seulement
« failed to generate object LOD for one or more worlds ». La cause est le Mono de
Wine qui répond à la place de .NET Framework, et aucune installation de .NET
Framework n'y change rien : Proton remplace `mscoree.dll` par un lien symbolique
dans son propre arbre à chaque mise à jour du préfixe. Le paquet de version
fournit le runtime .NET 10 ; les autres installations le téléchargent une fois.

### Wine ne pouvait pas savoir que le montage replie la casse

Corrigé.

Le `get_dir_case_sensitivity` de Wine interroge d'abord le dossier par
`FS_IOC_GETFLAGS`, puis renifle le marqueur que CIOPFS laisse dans les dossiers
qu'il sert. Sans l'un ni l'autre, Wine suppose SENSIBLE à la casse, et chaque
recherche dont l'orthographe ne correspond pas octet pour octet se rabat sur la
lecture du dossier ENTIER pour trouver une correspondance insensible à la casse.
Les jeux Bethesda demandent `data/ccbgssse001-fish.bsa` alors que le fichier
s'appelle `ccBGSSSE001-Fish.bsa`, donc cela se déclenchait sur presque chaque
asset : 4471 sondages de marqueur et 2236 relectures complètes de dossier en
huit secondes, et 195796 énumérations de `Data` en quatre-vingt-dix. Skyrim SE
n'atteignait jamais son menu principal - il restait à 240 Mo résidents pendant
que le démon brûlait 92 % d'un cœur.

Eidos repliait la casse dans `resolve_read` depuis le début. Tout le coût venait
de ne jamais le dire. `lookup` répond désormais `.ciopfs` ; `readdir` continue
de ne pas le lister. `ioctl` répond aussi à `FS_IOC_GETFLAGS` sur les dossiers
avec l'indicateur de repli de casse, que le Wine actuel demande avant de
chercher le marqueur.

Deux choses l'ont rendu fatal plutôt que simplement lent. Le coût croît avec la
taille du dossier, donc installer le contenu Anniversary (`Data` passant de 37 à
177 fichiers) a fait basculer la chose. Et `opendir` construisait avidement le
listage fusionné, ce qui est du pur gaspillage quand Wine ouvre un dossier
uniquement pour faire un `stat` sur ce marqueur - l'instantané est désormais pris
au premier `readdir`.

Après : le menu principal, 2,1 Go résidents, démon à 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` est ce qui l'a trouvé, et il est livré. Les compteurs
d'opérations disent combien ; 195796 énumérations d'un seul dossier sont
invisibles dans un total.

### Le jeu a réécrit `plugins.txt` à vide

Très probablement la même chose que le repli de casse ci-dessus : un `Data` qu'il ne pouvait pas énumérer en un temps raisonnable, d'où il concluait
qu'il n'y avait rien et sauvegardait cela. Non prouvé, et digne d'être revérifié.
Quoi qu'il en soit, la garde de capture (une capture qui vide entièrement
l'ensemble actif est refusée, quelle que soit sa taille) fait qu'elle ne peut plus
abîmer le profil.

### `FOPEN_KEEP_CACHE` est désactivé

Corrigé. Il faisait planter Skyrim SE sur un déréférencement nul quelques
secondes après le menu principal, de façon déterministe, sans aucun mod
installé ; les trois autres caches côté noyau ont été éliminés un par un par
bissection et seul celui-ci comptait. Cette bissection tournait avec
`FUSE_PASSTHROUGH` actif, et le noyau fait échouer avec EIO une ouverture en
passthrough qui porte `FOPEN_KEEP_CACHE`, si bien qu'avec le drapeau chaque
ouverture en passthrough échouait. Ce sont peut-être ces ouvertures échouées qui
faisaient planter le jeu, et le drapeau seul, passthrough désactivé, n'a jamais
été testé. Les réponses en passthrough ne le portent plus. Sa perte avait été
mesurée comme gratuite à l'époque, mais avec le passthrough actif le démon sert
*zéro* lecture (`EIDOS_FUSE_STATS` rapportait `read 0` pour un chargement
complet), donc avec le passthrough désactivé par défaut le vrai
coût n'est pas mesuré. Réactivez-le avec `EIDOS_FUSE_KEEP_CACHE=1` pour le
tester seul.

### Le passthrough FUSE empêche le jeu de charger le moindre contenu de mod

Le démon évite désormais la cause la plus probable (ci-dessous), mais cela n'a pas
encore été remesuré en jeu, donc le passthrough reste désactivé par défaut ;
`EIDOS_FUSE_PASSTHROUGH=1` l'active. Avant ce changement, passthrough activé,
Skyrim SE échouait à ouvrir 152 de ses propres fichiers en
`STATUS_ACCESS_VIOLATION`, si bien qu'aucun contenu de mod ne se chargeait, en
silence. La mesure, et pourquoi les journaux du démon montraient une exécution
propre, sont sous « Pourquoi le passthrough est désactivé par défaut » plus haut.

Le noyau lie un seul fichier sous-jacent par inode (`fs/fuse/iomode.c`). Une
ouverture qui apporte un second identifiant de fichier sous-jacent pour un inode
qui en a déjà un échoue avec EIO, tout comme une ouverture normale (sans
`FOPEN_PASSTHROUGH`) d'un inode qui a un fichier sous-jacent. Une ouverture en
passthrough qui porte `FOPEN_KEEP_CACHE` échoue aussi avec EIO. Le démon
enregistrait autrefois un nouvel identifiant à chaque ouverture, donc la deuxième
ouverture d'un fichier que le jeu tenait déjà ouvert échouait. Wine rapporte EIO
comme `STATUS_ACCESS_VIOLATION`, l'erreur derrière les 152 ouvertures échouées.
Cette règle vient du code source du noyau et correspond à ces 152 échecs, ce qui
en fait la cause la plus probable.

Le démon enregistre désormais un seul fichier sous-jacent par inode, partagé par
toutes ses ouvertures en passthrough, et ne met jamais `FOPEN_KEEP_CACHE` sur
une ouverture en passthrough. Le passthrough ne concerne que les ouvertures en
lecture seule de fichiers des couches du jeu et des mods qui font 1 Mio ou plus
ou se terminent par `.dll`, `.exe` ou `.asi`. Une fois ce fichier sous-jacent
enregistré, toute ouverture en lecture seule du fichier l'utilise, et tant que
le fichier a des ouvertures normales, les nouveaux lecteurs ont eux aussi des
ouvertures normales. Un fichier de l'Overwrite ne passe jamais par le
passthrough, et un montage qui projette les horodatages des plugins ne l'utilise
pas du tout. Une ouverture en écriture d'un fichier que des lecteurs en
passthrough tiennent reçoit ETXTBSY (dans Wine : `STATUS_SHARING_VIOLATION`)
avant tout copy-up, si bien qu'une ouverture `O_TRUNC` ne peut pas laisser dans
l'Overwrite une copie vide qui cacherait le fichier du mod.

### Le bouton Mod Manager Download ne fait rien

Preferences, Nexus, Downloads montre quel programme reçoit les liens `nxm://`. Une
carte Health apparaît quand ce programme n'est pas Eidos, qu'il est absent, ou
qu'aucun n'est défini. Le bouton Register écrit `eidos-nxm.desktop` et en fait le
gestionnaire par défaut de `x-scheme-handler/nxm`. Depuis un terminal,
`eidos nxm --register` fait la même chose.

Si l'état nomme déjà Eidos et que le bouton ne fait toujours rien, le navigateur
utilise peut-être un choix fait plus tôt. Effacez le choix enregistré pour les liens
nxm dans les réglages du navigateur pour qu'il redemande. Un navigateur Flatpak ne
peut pas lancer lui-même les programmes de l'hôte et transmet le lien par le portail
de bureau, qui lit le gestionnaire enregistré sur l'hôte.
