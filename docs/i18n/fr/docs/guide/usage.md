<!-- eidos-i18n: source=docs/guide/usage.md sha=aaff8588275e2b036c2a1c344c01509a4c9d0f00 -->

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
#    vit dans ~/.local/share/Colony/Eidos/instances/skyrimse ; `eidos init` affiche la vôtre)...
eidos install skyrimse mod.7z     # ou installer une archive téléchargée (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopter l'ordre et l'état des plugins d'un profil MO2 existant
eidos sort skyrimse               # trier l'ordre de chargement des plugins avec LOOT
eidos play skyrimse               # afficher ce qui serait monté
eidos play skyrimse -- <command>  # exécuter <command> avec les mods montés sur le jeu
eidos shell skyrimse              # un shell dans la vue fusionnée, les écritures vont dans Overwrite
eidos check skyrimse              # contrôles de santé sans la fenêtre (--json pour les scripts)
eidos sessions skyrimse           # les lancements du jeu et des outils enregistrés, du plus récent au plus ancien
eidos sessions skyrimse diff      # ce qui a changé depuis le dernier lancement terminé avec le code 0
eidos stock-game skyrimse         # quels fichiers du jeu l'instance utilise ; les figer ou en changer
eidos collection skyrimse <link>  # installer une collection Nexus comme son auteur l'a construite
eidos nexus update skyrimse --apply  # installer les fichiers Nexus qui ont remplacé les vôtres
eidos pack skyrimse backup.eidos  # toute l'instance dans un seul fichier, pour la déplacer ailleurs
eidos unpack backup.eidos <folder>   # la remettre en place sur l'autre machine
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` et `eidos export`
complètent l'ensemble ; lancez `eidos` sans argument pour la liste complète.

### Contrôles de santé depuis le terminal

`eidos check <instance>` exécute les contrôles du préfixe, des plugins, des masters manquants et du script extender de l'onglet **Diagnostics** sans ouvrir la fenêtre, pour qu'ils puissent tourner par SSH ou en Game Mode et aller dans un rapport de bug. Il affiche les mêmes lignes système que **Copier le rapport**, puis une ligne par constat, avec votre dossier personnel affiché comme `~` et votre nom d'utilisateur comme `<user>`. Le code de sortie vaut 0 quand aucun problème n'a été trouvé (des conseils seuls donnent toujours 0), 1 quand au moins un problème a été trouvé, et 2 quand Eidos n'a pas pu vérifier : arguments incorrects, instance qu'il ne peut pas ouvrir, ou jeu qu'il ne détecte pas.

`eidos check <instance> --json` affiche à la place un seul objet JSON. Ses champs de premier niveau sont `eidos` (la version), `game`, `instance`, `system` (`proton`, `flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`, `fusermount3`, `max_map_count`, `open_files_hard`) et `findings`. `max_map_count` et `open_files_hard` sont des nombres, ou `null` quand Eidos n'a pas pu les lire. Chaque constat est un `code`, un `level` (`problem` ou `advice`) et ses `args` ; l'onglet Diagnostics affiche la phrase de chaque code.

### Un shell dans la vue fusionnée

`eidos shell <instance>` lance votre `$SHELL` (ou `/bin/sh`) dans le dossier du jeu tel que le jeu le verrait : chaque mod activé monté sur `Data`, les mods Root sur la racine du jeu, et tout ce qui y est écrit atterrit dans l'Overwrite. `eidos shell <instance> -- <command>` exécute une seule commande à la place, par exemple `eidos shell skyrimse -- ls Data/meshes` ou un outil natif comme NifSkope. L'exécution tient le verrou de l'instance comme une session de jeu : la fenêtre ne peut pas enregistrer de modifications avant votre sortie, et elle apparaît dans `eidos sessions` sous le nom `shell`, jamais comme un lancement du jeu. Ctrl+C sur une commande arrête l'exécution comme Stop : Eidos termine la commande, capture la session et démonte. Dans le shell interactif, Ctrl+C reste au shell, comme d'habitude ; tapez `exit` pour terminer l'exécution.

### Instances : globale et portable

Chaque commande ci-dessus s'adresse à une instance. `skyrimse` désigne
l'instance **globale** - stockée centralement dans
`~/.local/share/Colony/Eidos/instances/skyrimse`, gérée par Eidos. L'autre sorte est
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
réserves à connaître : déplacer un dossier portable à la main conserve tout, sauf
les entrées d'outils que vous avez enregistrées avec des chemins absolus vers
l'ancien emplacement (`eidos pack` / `eidos unpack`, plus bas, les corrigent pour
vous ; un simple `mv` ne le fait pas), et le cache de runtimes partagé
(`~/.local/share/Colony/Eidos/runtimes/`) reste délibérément global à la
machine - un hôte .NET de 78 Mo n'est pas par instance.

Eidos range ses propres fichiers sous `Colony/Eidos`, la disposition qu'utilise
chaque programme de la famille Colony : `~/.config/Colony/Eidos/` pour ce que
vous avez choisi (préférences, votre session Nexus, votre liste d'instances, les
définitions de jeux et d'add-ons que vous avez écrites),
`~/.local/share/Colony/Eidos/` pour les instances globales (`instances/`), les
runtimes téléchargés et les logs de session (`logs/`), et
`~/.cache/Colony/Eidos/` pour les masterlists LOOT, retéléchargées chaque fois
qu'elles manquent.

Un Eidos plus ancien gardait tout cela dans `~/.config/eidos/`,
`~/.local/share/eidos/` et `~/.local/state/`. Le premier lancement après la mise
à jour les **copie** et le dit dans le log. Une instance globale est déplacée
par liens physiques plutôt que par une copie octet par octet : des dizaines de
gigaoctets de mods changent de place en un instant sans occuper d'espace
supplémentaire, et ses entrées d'outils sont redirigées vers le nouveau dossier.
Les anciens répertoires sont laissés exactement tels quels - rien n'est
supprimé, une mauvaise mise à jour ne peut donc vous coûter ni une session
ouverte ni une sauvegarde - et vous pouvez les supprimer vous-même une fois
rassuré. Si une copie ne peut pas être faite (un disque plein ou en lecture
seule, un ancien dossier d'instance sur un autre disque, un jeu qui tourne
encore depuis celui-ci), Eidos continue d'utiliser l'ancien emplacement et
réessaie au lancement suivant. Une option de lancement Steam qui nomme encore
l'ancien dossier d'instance ouvre l'instance déplacée.
Le déplacement n'occupe pas d'espace supplémentaire qu'au début : un mod ou un
téléchargement que vous retirez ou réinstallez ensuite garde son ancienne copie
sur le disque à travers l'ancien dossier d'instance, et c'est la suppression de
`~/.local/share/eidos/<game>/` qui rend cet espace.

Une instance portable n'est jamais déplacée : elle reste là où vous l'avez mise.

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

### Listes Wabbajack

Eidos n'a pas d'installeur Wabbajack. Installez la liste avec CLF3 ou Jackify, en dehors d'Eidos, puis choisissez le dossier d'installation comme instance portable dans l'assistant. Eidos lit le dossier selon la disposition de MO2 : `mods/`, `profiles/<nom>/modlist.txt` et les listes de plugins. Il garde le `ModOrganizer.ini` écrit par l'installeur et ne l'écrase jamais, donc les exécutables propres à la liste restent enregistrés.

Beaucoup de listes ajoutent un dossier `Stock Game` ou `Game Root` qui contient une copie modifiée du jeu, et MO2 lance le jeu depuis là. Quand l'assistant ou `eidos init` adopte un dossier de liste pour la première fois, l'instance lance elle aussi le jeu depuis ce dossier, toujours démarré par Steam ou par la boutique qui possède le jeu : rien n'est à déplacer. Eidos trouve le dossier grâce au `gamePath` du `ModOrganizer.ini` de la liste, ou par ces deux noms. Une instance adoptée avec une version antérieure d'Eidos garde l'installation de la boutique jusqu'à ce que vous changiez (voir la section suivante). CLF3 reconstruit les fichiers BSA et DDS avec ses propres outils, donc le résultat peut différer d'une installation Wabbajack, et les auteurs de listes ne le prennent pas en charge.

### Fichiers du jeu : l'installation de la boutique ou une copie

Par défaut, le jeu tourne depuis son installation de la boutique, le dossier où Steam, Heroic ou Legendary le range. Une instance peut à la place tourner depuis une copie de ce dossier placée dans l'instance. Les mods se posent sur la copie, et l'installation de la boutique n'est plus que l'endroit où Eidos monte le résultat. La boutique démarre toujours le jeu, donc Steam garde son overlay, son cache de shaders et ses sauvegardes dans le cloud.

La carte « Fichiers du jeu », en haut de l'onglet Diagnostics, indique quels fichiers l'instance utilise et permet de passer de l'un à l'autre. `eidos stock-game <instance>` fait de même depuis un terminal : `use <folder>`, `store`, `freeze` et `delete --yes`.

**Figer cette version** copie l'installation de la boutique dans `<instance>/Stock Game` et lance le jeu depuis cette copie : une mise à jour de la boutique ne casse plus les plugins du script extender, puisqu'elle modifie l'installation de la boutique et pas la copie. Le premier appui affiche la taille et ce que le système de fichiers en fait, le second copie. Sur btrfs et XFS, la copie partage les blocs de la boutique (reflink) et ne prend presque pas de place tant que la boutique ne modifie aucun fichier. Sur ext4, ou quand le jeu et l'instance sont sur des systèmes de fichiers différents, c'est une copie complète, aussi grosse que le dossier du jeu. Eidos refuse tant que Steam signale le jeu occupé ou en cours de mise à jour, et tant qu'un jeu ou un outil de l'instance tourne. **Arrêter**, ou un échec, laisse `Stock Game.eidos-tmp`, que la carte propose de supprimer ; ce dossier n'est jamais utilisé.

Une copie masque chaque fichier que la boutique ajoute, modifie ou supprime dans le dossier du jeu après la copie : nouveaux fichiers Creation Club, nouvel exécutable, nouvelles DLL. Le jeu, la liste des plugins, LOOT et les contrôles de santé lisent tous la copie. Elle ne masque pas ce qui se passe hors du dossier du jeu : une mise à jour peut encore installer des runtimes dans le préfixe Proton au lancement suivant, et Steam peut faire passer le jeu à un autre Proton.

**Revenir à la version de la boutique** retire la copie du prochain lancement après un second appui. La copie reste sur le disque, et **Supprimer la copie** l'efface, là encore après un second appui. Eidos ne supprime jamais le dossier `Stock Game` ou `Game Root` d'une liste. Si le dossier de la copie disparaît, les lancements sont refusés et Diagnostics nomme le dossier : la version de la boutique ne se lance jamais par surprise.

Morrowind, et Oblivion avec `bUseMyGamesDirectory=0`, gardent leur liste de plugins dans le dossier du jeu, et Eidos ne les lance pas encore depuis une copie.

### La liste de mods

Comme dans MO2, chaque profil a son propre `modlist.txt` (quels mods sont
activés, et à quelle priorité), tandis que les dossiers des mods vivent dans le
`mods/` partagé de l'instance. Un dossier que vous déposez à la main dans `mods/`
n'est pas encore listé : il apparaît **désactivé à la priorité la plus haute** -
en bas de la liste de la fenêtre, là où MO2 le met aussi - si bien qu'une fois
activé, il gagne ses conflits. Les versions plus anciennes le mettaient en haut,
où tous les autres mods l'écrasaient.

Comme les dossiers sont partagés, retirer ou renommer un mod change chaque
profil : sa ligne est retirée du `modlist.txt` de chaque profil, ou renommée
dedans, et chaque profil garde pour le reste son propre état et son propre
ordre. Un profil qui se retrouve sans aucun mod n'a plus de `modlist.txt` du tout
plutôt qu'un fichier vide, qui se lisait comme un fichier endommagé et bloquait
l'installation suivante.

La fenêtre travaille sur sa propre copie de la liste de mods et de l'ordre de
chargement. Si un autre processus Eidos écrit l'une ou l'autre pendant qu'elle
est ouverte - `eidos install` ou `eidos sort` dans un terminal, une collection,
une session lancée par Steam, une seconde fenêtre - la modification suivante de
la fenêtre est refusée au lieu d'écraser ce travail. Elle recharge depuis le
disque et dit *Not saved: another Eidos process changed the mod list. It has
been reloaded - redo the change.* (ou la même chose pour l'ordre de chargement) ;
seul ce clic-là est perdu, refaites-le donc. Rename, New mod, Add separator,
Remove et la transformation de l'Overwrite en mod vérifient avant de toucher à
`mods/`, si bien qu'une modification refusée ne change rien sur le disque.
L'éditeur d'INI refuse de même d'enregistrer par-dessus un fichier qui a changé
depuis qu'il l'a ouvert.

### Les sauvegardes appartiennent au profil

Chaque profil garde ses propres sauvegardes dans `profiles/<name>/saves/`, et
pendant toute la durée d'une exécution ce dossier est monté par-dessus le dossier
de sauvegardes du jeu dans le préfixe Proton. Le premier lancement d'un profil y
copie les sauvegardes déjà présentes dans le préfixe, une seule fois, si bien
qu'une sauvegarde que vous supprimez ensuite du profil reste supprimée. Après
chaque exécution, les sauvegardes les plus récentes du profil sont recopiées dans
le préfixe pour Steam Cloud ; une sauvegarde du préfixe qu'Eidos n'y a pas mise
est d'abord conservée dans le profil sous le nom `orphan-<time>-<name>`, jamais
écrasée.

Une sauvegarde qui arrive dans le préfixe après cette première copie sans passer
par Eidos - un téléchargement Steam Cloud depuis un Steam Deck, un lancement sans
Eidos - reste cachée tant que les sauvegardes du profil sont montées. Rien ne la
supprime : le lancement la nomme dans le log de l'exécution (le
`logs/run-*.log` de l'instance pour un lancement depuis la fenêtre, la console
de Steam pour un lancement par Steam), une fois à chaque changement de cet
ensemble de fichiers. Pour y jouer, appuyez sur Importer dans l'onglet
Sauvegardes. Le bouton Importer la copie avec son co-save dans le profil et ne
remplace jamais une sauvegarde du profil.

### Ce qui a changé depuis le dernier bon lancement

Chaque lancement par `eidos play` ou `eidos tool run`, depuis la fenêtre, un terminal ou Steam, écrit un enregistrement dans `<instance>/logs/sessions/` : le profil, chaque mod activé avec sa version et ses identifiants de fichier Nexus, l'ordre des plugins, une somme de contrôle de chaque INI du profil, la version de l'exécutable du jeu, la version d'Eidos, le code de sortie et les sauvegardes créées pendant le lancement. Les 50 plus récents sont conservés, et `logs/` n'entre pas dans une instance empaquetée. `eidos sessions <instance> diff` compare le dernier lancement du jeu terminé avec le code 0 à l'état actuel ; `diff <id>` choisit un autre enregistrement de la liste. Eidos ne détecte pas encore les plantages ici, et un jeu planté sous Proton peut quand même sortir avec le code 0 : si le lancement choisi en fait partie, indiquez-en un plus ancien avec `diff <id>`.

### Parties test

Le bouton Partie test, à côté de Lancer, démarre le jeu comme Lancer, mais tout ce que le jeu écrit va dans `<instance>/test-run/` au lieu de votre profil, de l'Overwrite et du préfixe : les sauvegardes nouvelles ou modifiées, les changements d'INI, `plugins.txt`, les fichiers écrits dans Data ou dans le dossier du jeu, et les journaux de `Documents/My Games/<game>`. Vous pouvez tester avec les mods du profil, sans aucun mod, ou seulement avec les mods sélectionnés dans la liste, plus les mods dont les plugins leur servent de maîtres.

Quand le jeu se ferme, Eidos range ces fichiers en cinq groupes (sauvegardes, changements d'INI, état des plugins, sortie de l'Overwrite, journaux) et vous demande de garder ou de jeter chacun d'eux. Garder les sauvegardes ne supprime jamais une sauvegarde du profil, même si le jeu l'a supprimée pendant la partie. Jeter quoi que ce soit demande un second appui. Si Eidos ou l'ordinateur s'arrête pendant une partie test, les fichiers restent dans `test-run/` et la liste s'ouvre la prochaine fois que vous ouvrez l'instance ou appuyez sur F5.

Quand la fenêtre n'a pas été démarrée par la boutique du jeu, Partie test confie le jeu à cette boutique (Steam, Heroic ou Legendary), comme Lancer, et la partie que lance la boutique est la partie test. Une partie test ne couvre que les dossiers cités plus haut : les autres écritures du jeu, comme le registre Wine, vont là où elles vont toujours. Les sauvegardes d'une partie test n'arrivent dans Steam Cloud qu'une fois que vous les avez gardées puis avez joué une partie normale.

### Trouver le mod à l'origine d'un problème

Le bouton Trouver le mod à l'origine d'un problème, dans Diagnostics, cherche le mod qui cause un problème que vous savez reproduire, par exemple un plantage à un endroit précis. Chaque étape est une partie test avec la moitié des suspects restants, les maîtres dont ils ont besoin et les mods que vous gardez toujours actifs (les mods sélectionnés dans la liste au moment de commencer, et le mod qui fournit le script extender). Après chaque partie, vous indiquez si le problème s'est produit ; « Je ne sais pas » répartit les suspects autrement, et Relancer refait l'étape. Répondre, ou appuyer sur Relancer, jette les fichiers que l'étape a écrits, après un second appui. Pour en garder certains, appuyez d'abord sur Voir les fichiers : la même liste qu'après une partie test s'ouvre. Quand le jeu a planté et qu'un crash logger a écrit un journal, ou que le jeu est sorti avec un code d'erreur, Eidos met en avant le bouton « Oui, il s'est produit » comme réponse probable. 200 suspects demandent environ 8 parties.

La recherche est enregistrée dans `<instance>/bisect.json`, vous pouvez donc fermer Eidos entre deux étapes. Le bouton Arrêter la recherche l'abandonne après un second appui. Le résultat nomme le mod, ou l'ensemble de mods qui ont besoin des maîtres les uns des autres, et il est conservé dans `<instance>/logs/bisect-<time>.txt`. Les mods générés (une sortie de Nemesis ou de DynDOLOD, un Bashed Patch) sont construits pour la liste complète et peuvent échouer avec la moitié : gardez-les toujours actifs, ou laissez-les désactivés pendant toute la recherche.

### Aucune étape privilégiée n'est nécessaire

Eidos s'exécute entièrement sans root. Il monte dans un espace de noms
utilisateur + montage privé, donc pas d'assistant setuid, pas de démon, et rien à
autoriser.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` est **facultatif** et ne
conditionne qu'une seule chose : le passthrough FUSE du noyau, désactivé par
défaut parce qu'une version antérieure du démon cassait le jeu avec, et que le
changement qui évite la cause la plus probable n'a pas encore été remesuré en
jeu (voir plus bas). Avec cette capability, Eidos prend un simple espace de noms
de montage au lieu d'un espace de noms utilisateur ; le déploiement des mods est
identique dans les deux cas.


Pourquoi l'ancien conseil `setcap` a disparu - et pourquoi le passthrough FUSE
est livré désactivé - est expliqué dans
[troubleshooting.fr.md](troubleshooting.md#pourquoi-le-passthrough-est-désactivé-par-défaut).

## Installer une collection Nexus

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run     # la lire et dire ce qui se passerait
```

Dans la fenêtre : collez le lien dans **File -> Open a Nexus collection...** et
appuyez sur *Install this collection*.

### Pourquoi ce n'est pas « télécharger ces mods »

Une collection est une RECETTE, pas une liste de courses, et presque rien de
cette recette n'est visible depuis l'API Nexus. L'ordre d'installation, les
réponses que l'auteur a données à l'installeur scripté de chaque mod, quel mod
gagne un conflit de fichiers, quels plugins se chargent où, les patchs
binaires - tout cela vit dans un `collection.json` à l'intérieur de l'archive de
la collection elle-même. Télécharger les mêmes mods et les installer avec leurs
options par défaut produit un jeu différent.

Eidos télécharge donc l'archive, lit la recette, et l'applique :

| La collection dit | Eidos fait |
| --- | --- |
| `phase` | installe dans cet ordre, les membres facultatifs en dernier |
| `choices` | rejoue les réponses de l'auteur à chaque FOMOD, et le dit quand il ne peut pas |
| `modRules` | réordonne la liste de mods pour que le bon mod gagne chaque fichier |
| `plugins` / `pluginRules` | fusionne les règles LOOT dans votre userlist et trie |
| `INI Tweaks/` | les installe comme un mod à part entière |
| `tools` | vous dit quels outils il attend ; n'en crée aucun |

Les membres sont **activés** au fur et à mesure de leur installation, à la fin de
la liste de mods dans l'ordre d'installation, avant que `modRules` ne les
déplace - un mod que rien ne liste est un mod que rien ne charge, et une
collection posée inerte sur le disque est le seul échec que le rapport ne
pouvait pas voir.

L'état est écrit dans `<instance>/collections/<slug>-<revision>.state.json` après
**chaque** membre - à côté du dossier de cette révision, jamais dedans, si bien
qu'une collection ne peut pas embarquer sa propre comptabilité - et une
installation interrompue au membre 147 sur 200 reprend à 147. Il est écrit sous
un nom temporaire puis renommé en place, si bien qu'une interruption laisse
l'ancien enregistrement ou le nouveau, jamais la moitié de l'un ou de l'autre ;
et s'il devenait un jour illisible, Eidos s'arrête et le dit plutôt que de
recommencer silencieusement toute la collection. Relancez la même commande.

Installer une révision plus récente d'une collection que vous avez déjà reprend
les dossiers de la révision précédente : un membre inchangé est vérifié sur
place, un membre dont l'auteur a mis à jour le fichier est remplacé à l'intérieur
de son ancien dossier, et un membre que la nouvelle révision a abandonné reste
installé et est nommé dans le rapport pour que vous le désactiviez ou le
retiriez. Un dossier qu'un autre profil active n'est jamais repris, parce que le
dossier des mods est partagé et que ce profil changerait dans votre dos : la
nouvelle révision installe à la place sa propre copie à côté, et le rapport vous
demande de désactiver l'ancienne dans ce profil, jamais de la retirer. Revenir à
une révision plus ancienne reprend ses dossiers de la même façon, ou installe des
copies neuves de ceux qu'elle ne peut pas reprendre.
Chaque fois qu'un membre est remplacé ainsi alors que vous aviez modifié ou
ajouté des fichiers dans son dossier, l'ancien dossier est d'abord conservé à
côté sous le nom `<name>_backup`, qui n'est jamais chargé dans le jeu, et le
rapport le nomme.

Un vestige d'une révision antérieure qui demeure activé empêche l'installation de
compter comme celle que l'auteur a construite ; la remarque disparaît dès que
vous le désactivez. Un membre que vous avez sauté avec `--no-optional` n'est pas
repris et est nommé de la même façon. Si la reprise elle-même échoue, l'exécution
s'arrête sur *previous revision: its folders could not be taken over* avec la
raison ; corrigez cela et relancez la commande.

### Ce qu'il ne fera pas semblant de faire

**Un compte Nexus gratuit ne peut pas récupérer les mods membres.** Nexus n'émet
pas de liens de téléchargement pour les comptes gratuits via l'API ; chaque
fichier exige le bouton « Mod Manager Download » du site lui-même. L'ARCHIVE de
la collection se télécharge très bien avec un compte gratuit - vous pouvez donc
lire toute la recette - mais chaque membre est signalé avec l'URL exacte de sa
page pour que vous le récupériez vous-même. C'est une règle commerciale de Nexus,
et aucune quantité de réessais n'en vient à bout. Récupérez-les avec le bouton du
site et relancez la commande : ces membres sont cherchés dans `downloads/` à
chaque exécution, si bien que la collection s'achève à mesure que vous la
fournissez.

Dans la fenêtre, un compte gratuit a dans le panneau de la collection le bouton
*Fetch the N missing required files, one by one* à la place de la récupération
par lots. Il ouvre sur Nexus la page du premier fichier requis manquant, attend
que ce fichier arrive dans `downloads/`, puis ouvre le suivant. C'est toujours
vous qui appuyez sur le bouton de téléchargement de la page, pour chaque
fichier. Le panneau indique où il en est (*3 of 12*), avec *Skip* et *Stop*. Si
un autre fichier du même mod arrive à sa place, le panneau donne les deux
identifiants de fichier et continue d'attendre celui que la collection désigne.
Les membres facultatifs n'en font pas partie et gardent leur propre bouton
*Open*.

Appliqués comme la collection les décrit : les **patchs binaires** (chacun est vérifié par rapport au CRC enregistré), les **surcharges de fichiers** (les fichiers nommés sont masqués avec `.mohidden`, comme le fait MO2) et les membres dont les fichiers voyagent à l'intérieur de l'archive de la collection (`bundle`). Les
membres que la collection attend que vous récupériez à la main (`browse`,
`manual`) transmettent jusque dans le rapport les instructions de l'auteur
lui-même. Un membre dont le nom est déjà celui d'un de vos mods est installé à
côté de lui sous un nom libre, jamais par-dessus, et le rapport dit lequel.

### Le rapport

Tout l'intérêt du rapport, c'est que « installé » vaille la peine d'être cru. Un
membre dont les réponses d'installeur n'ont pas toutes pu être rejouées est
**installé, mais pas comme la collection le demande** - sa propre ligne, séparée
des succès auxquels il ressemble superficiellement, parce que les fichiers sur le
disque diffèrent de ceux de l'auteur. Les règles d'ordre qui ne nommaient rien
dans la collection, les mods pris dans une boucle de règles contradictoires, les
groupes LOOT qui n'existent pas, et toute partie du manifeste que cette version
d'Eidos ne comprend pas sont listés eux aussi.

L'alternative - journaliser ce genre de choses et déclarer l'installation
terminée - c'est ainsi qu'une collection en vient à mal tourner en jeu pour des
raisons que personne n'arrive à trouver.

## Déplacer une instance vers une autre machine

`eidos pack` écrit une instance entière - chaque mod, l'ordre de chargement, tous
les profils, l'Overwrite avec vos sauvegardes dedans, et les archives depuis
lesquelles tout a été installé - dans un seul fichier. `eidos unpack` la remet en
place :

```sh
eidos pack skyrimse ~/backup.eidos                  # ou donnez un dossier : nommé d'après le jeu et la date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # sur l'autre machine
```

`eidos pack --dry-run` liste exactement ce qui y entrerait, ce qui n'y entrerait
pas et pourquoi, et la place qu'il lui faut, sans rien écrire. `eidos unpack
--info` fait la même chose pour un fichier que vous avez déjà.

Si 7-Zip n'a pas pu lire quelque chose, ou si le parcours a dû sauter quelque
chose que vous vous attendriez à trouver dans une sauvegarde (un lien
symbolique, un dossier qu'il n'a pas pu lire, un nom qu'on ne peut pas donner
sans risque à 7-Zip), l'archive est quand même écrite et nommée - elle vaut la
peine d'être gardée - mais `eidos pack` sort avec le code **1** et dit ce qui
manque. Une sauvegarde incomplète n'est pas un succès, et
c'est une commande que les gens mettent devant un `&&`.

Un fichier `.eidos` est une archive 7-Zip sous un nom à elle, ce qui est un
choix plutôt qu'un déguisement : 7-Zip est déjà nécessaire pour installer un mod
tout court, cela n'ajoute donc aucune dépendance, et si vous perdez un jour
Eidos votre sauvegarde s'ouvre toujours dans n'importe quel archiveur que vous
avez. Elle est compressée en LZMA2 avec `-mx1` - environ 43 % de l'original sur
une liste chargée en textures, en une fraction du temps que `-mx9` passe à
gagner quelques points de plus. Les textures et les meshes sont déjà des formats
compressés ; il ne reste presque rien à trouver pour un dictionnaire plus grand.
`--level` (0, 1, 3, 5, 7, 9) passe outre si vous n'êtes pas d'accord sur votre
propre contenu, et `--no-downloads` laisse `downloads/` de côté.

Au niveau 1, celui par défaut, les fichiers sont rangés dans des
**blocs solides de 16 Mo au plus**. Extraire un fichier d'une sauvegarde de
70 Go décode au plus le bloc où il se trouve, jamais tout ce qui le précède, et
l'empaquetage de contenu de jeu va 2 à 5 fois plus vite qu'avec chaque fichier
compressé à part (la restauration 2 à 4 fois). Le prix se paie en cas d'avarie :
un seul octet corrompu peut emporter tous les fichiers de son bloc, jusqu'à
16 Mo de fichiers voisins. Les données de récupération décrites plus bas
couvrent ce cas. Tous les autres niveaux restent non solides : chaque fichier
forme son propre flux, et un octet corrompu ne coûte que ce fichier. Le niveau 0
stocke les fichiers sans les compresser. À partir du niveau 3, LZMA2 travaille
déjà par morceaux de 16 Mo ou plus, et les blocs solides s'y sont montrés plus
lents à la mesure : au niveau 5, l'empaquetage des meshes était 1,4 fois plus
lent et la restauration des textures 6,8 fois plus lente.

L'empaquetage prend le verrou de l'instance, il ne peut donc pas tourner pendant
que le jeu ou un autre Eidos écrit dans cette instance - la sauvegarde est un
instantané, pas un flou. Elle est écrite sous un nom en `.part`, vidée sur le
disque, puis relue en entier avec `7z t` avant de recevoir le nom que vous avez
choisi. Cette vérification décode chaque bloc et compare chaque somme de
contrôle, soit le même travail qu'une restauration sur l'autre machine. Elle
montre ce que le disque a rendu juste après l'écriture, pas que le support le
gardera : c'est le rôle des données de récupération décrites plus bas. Si elle
échoue, le `.part` est supprimé, rien n'est écrit sous votre nom, et
`eidos pack` cite la ligne où 7-Zip a trouvé l'avarie. Un empaquetage interrompu
ou raté laisse quelque chose de manifestement inachevé, ou rien du tout, plutôt
qu'un fichier `.eidos` qui a l'air complet et ne l'est pas.

Quand par2cmdline est installé (`sudo pacman -S par2cmdline` sur Arch,
`sudo apt install par2` sur Debian et Ubuntu, `sudo dnf install par2cmdline` sur
Fedora), `eidos pack` écrit aussi des données de récupération PAR2 à côté de
l'archive : `backup.eidos.par2` et quelques fichiers
`backup.eidos.volNN+MM.par2`, soit 5 % de l'archive plus un petit index.
Gardez-les dans le même dossier que le fichier `.eidos`, sous les noms qu'ils
ont reçus, et copiez-les avec lui : une sauvegarde déplacée sans eux n'a rien
pour être réparée, et les données de récupération enregistrent le nom de
l'archive, si bien qu'une sauvegarde renommée ne leur correspond plus.
`eidos unpack` les trouve par leur nom et vérifie l'archive avec eux avant
d'extraire quoi que ce soit, y compris une sauvegarde dont la fin a été coupée
au point que 7-Zip ne peut plus l'ouvrir. Si quelques octets se sont abîmés,
par2 répare d'abord l'archive et garde la copie endommagée sous le nom
`backup.eidos.1`, ce qui demande dans son dossier autant d'espace libre que la
taille de l'archive ; une avarie plus étendue que ce que couvrent les 5 % arrête
la restauration avant qu'un seul fichier soit écrit. Des fichiers de
récupération qui ne correspondent pas à l'archive, comme ceux d'une sauvegarde
plus ancienne du même nom, ne servent pas quand 7-Zip lit l'archive et la trouve
intacte, et la restauration le signale. Sur une machine sans par2, la
restauration se fait quand même, avec un avertissement indiquant que les données
de récupération n'ont pas servi. `--no-recovery` empaquette sans elles, et
`eidos pack --dry-run` indique si elles seront écrites.

### Ce qui n'y est pas, et pourquoi

| Laissé de côté | Pourquoi |
| --- | --- |
| Le préfixe Proton | Il a la forme de la machine (mappages de lecteurs, une vue `Z:` de *ce* système de fichiers) et Eidos le reconstruit en une minute avec `eidos prereqs <instance> --install`. L'emporter doublerait à peu près l'archive pour livrer quelque chose que l'autre machine doit de toute façon régénérer. |
| Le jeu | Une instance modifie un jeu qu'elle ne contient pas. Installez-le depuis Steam là-bas. |
| Les outils hors de l'instance | xEdit, DynDOLOD et consorts sont des binaires tiers avec leurs propres installeurs. Ils sont **nommés** dans l'archive, si bien que le dépaquetage vous dit exactement lesquels manquent sur la nouvelle machine. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Des traces de la machine qui a fait la sauvegarde. `prereqs.done` en particulier prétendrait que les bibliothèques runtime sont déjà installées dans un préfixe qui n'est pas là. |
| `loot/`, sauf `userlist.yaml` | Un cache de masterlist qu'Eidos re-télécharge à la demande. Vos propres règles LOOT ne sont pas un cache - personne ne les re-télécharge - ce seul fichier fait donc le voyage. |
| `.base/`, `.base-root/` | Des points de montage vides où les fichiers du jeu lui-même sont mis de côté pendant une session. |
| Les fichiers à moitié écrits | Un téléchargement en pause (`*.unfinished`), une écriture atomique en cours (`*.eidos-tmp*`). |
| Les liens symboliques | 7-Zip en suivrait un et copierait ce qu'il désigne, ce qui, pour un lien absolu, veut dire tirer une arborescence étrangère dans votre sauvegarde. Ils sont signalés à la place, et la sauvegarde compte comme incomplète : un `mods/` ou un `downloads/` lié vers un autre disque N'EST PAS empaqueté. Pour sauvegarder volontairement sans un `downloads/` lié, passez `--no-downloads` (ou décochez *Include downloads/* dans la fenêtre) ; une exclusion délibérée ne compte pas comme un manque. |

Une copie du jeu dans l'instance (`Stock Game`) est empaquetée comme n'importe quel autre dossier : la sauvegarde d'une instance figée est donc plus grosse d'à peu près la taille du jeu. Une copie inachevée (`Stock Game.eidos-tmp`) est laissée de côté.

Chacun d'eux figure dans `eidos-backup.ini` à la racine de l'archive, avec sa
raison - si bien que la réponse à « qu'est-ce qui n'est pas là-dedans ? » est
`cat`, pas une supposition. Le même fichier note où l'instance vivait, quel
profil était actif, quelle taille elle fait une fois dépaquetée, et de quels
outils la nouvelle machine aura besoin.

Les enregistrements de téléchargement (`downloads/*.meta`) y entrent avec leur
ligne `url=` **vidée**. Cette URL est un lien Nexus signé : il cesse de
fonctionner en quelques heures, et il porte le `user_id` du compte qui a
téléchargé le fichier. Une sauvegarde est faite pour être remise à quelqu'un
d'autre, elle ne transporte donc pas cela. Tout ce qui rend le téléchargement
retrouvable - l'identifiant du mod, celui du fichier, la version - reste.

### Dans la fenêtre

Les deux sont dans le menu **File** : *Pack this instance...* et *Unpack a
backup...*. Le dépaquetage est aussi sur l'écran d'accueil, sous la liste des
instances, parce que la machine qui en a le plus besoin est celle qui n'a encore
aucune instance.

La boîte de dialogue Pack prévisualise avant de s'engager : combien de fichiers,
quelle taille, quelle part de cela est `downloads/`, à peu près ce que fera
l'archive, et quels outils sont nommés mais pas emportés. Sa case *Write
recovery data* est cochée quand par2 est installé, et grisée avec le nom du
paquet à installer quand il ne l'est pas. La boîte de dialogue Unpack lit le
manifeste de la sauvegarde dès l'instant où vous choisissez le fichier, si bien
qu'elle peut vous dire quel jeu elle contient, quand elle a été faite, où elle
vivait et lesquels de ses outils manquent ici - avant que vous ne lui donniez un
dossier.

Les deux tournent sur un fil de travail avec une barre de progression, si bien
que la fenêtre continue de répondre pendant qu'ils travaillent, et la barre
devient le rapport quand ils ont fini. L'empaquetage garde le verrou de
l'instance pendant toute la durée : rien d'autre ne peut toucher à l'instance
pendant qu'elle est lue, et c'est ce qui fait de la sauvegarde un instantané
plutôt qu'un flou.

### Ce que le dépaquetage répare

Une simple copie du dossier laisserait chaque bouton d'outil pointer sur un
chemin de l'ancienne machine. `eidos unpack` réécrit les valeurs qui nomment
l'ancienne racine de l'instance et seulement celles-là : `exe`, `workdir` et les
clés `arg` numérotées dans `tools.ini`, et `installationFile` dans le `meta.ini`
de chaque mod. Un outil qui n'a jamais été à l'intérieur de l'instance
(`/opt/xedit/SSEEdit.exe`) est laissé exactement tel quel et listé comme quelque
chose à réinstaller - deviner où est passé le xEdit de quelqu'un d'autre, c'est
de l'invention, pas de la réparation. Il corrige aussi le fait que l'instance se
dise centrale ou portable, si bien que les commandes suivantes la cherchent là où
elle est désormais.

Le dépaquetage refuse un dossier qui n'est pas vide (passez `--force` si vous
voulez vraiment écraser), un disque trop petit pour le chiffre annoncé par le
manifeste lui-même, une archive faite par un Eidos plus récent que le vôtre, et
toute archive contenant une entrée qui serait écrite *hors* du dossier que vous
avez choisi.

Ensuite :

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # reconstruire le préfixe Proton
eidos play /mnt/games/EidosSkyrim
```

## GUI

```sh
cargo run -p eidos-gui
```

Un assistant de premier lancement à la MO2, dessiné dans les thèmes partagés de
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

### Préférences

Cliquez sur **Eidos**, en haut à gauche, pour ouvrir Preferences à la place des
deux volets, et cliquez à nouveau (ou sur **Close**) pour revenir. General,
Appearance et Accessibility viennent en premier, comme dans chaque programme
Colony, puis Mod list, Nexus et About. Chaque changement s'applique et
s'enregistre aussitôt ; il n'y a pas de bouton Save.

- **General** : le jeu qu'Eidos ouvre, la taille de la fenêtre, la langue de
  l'interface, le dossier d'outils, la façon de lancer un jeu et si une notification
  du bureau signale la fin d'une tâche longue quand la fenêtre est en
  arrière-plan. La fenêtre
  parle anglais et français ; **System** suit `LC_ALL`, `LC_MESSAGES` ou
  `LANG`, et le changement s'applique aussitôt. La ligne de commande `eidos`
  répond toujours en anglais.
- **Appearance** : le thème (25 familles, 57 palettes, le parchemin d'Eidos par
  défaut), la couleur d'accent, la taille du texte, les effets et un aperçu en
  direct.
- **Accessibility** : contraste élevé, une police adaptée à la dyslexie,
  animations réduites et une taille de texte de lecture qui se multiplie avec
  celle d'Appearance.

Ces réglages vivent dans `~/.config/Colony/Eidos/preferences.json`. Le thème
qu'un Eidos plus ancien gardait dans `settings.ini` est repris la première fois.

**About -> Version** contient la vérification des versions, activée par défaut. Au démarrage, une fois par jour au plus, Eidos demande à GitHub la dernière version de `Project-Colony/Eidos`. Si elle est plus récente que la copie en cours d'exécution, la barre de menus affiche `Eidos 1.24.0 is available: release notes` après Help, et le lien ouvre la page de cette version avec son journal des modifications. Eidos ne se télécharge et ne se remplace jamais lui-même. La vérification est sautée en mode hors ligne. Elle est désactivée pour une copie installée sous `/usr` par un gestionnaire de paquets, qui la met à jour ; `/usr/local`, où `install.sh --system` place Eidos, compte comme votre propre installation.

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

Ctrl+Z annule la dernière modification de la liste des mods ou de l'ordre de
chargement : une case cochée, un déplacement, une désactivation groupée, un tri
LOOT. Ctrl+Shift+Z la rétablit. La fenêtre
garde 50 étapes et les oublie sur F5, à la fin d'un lancement de jeu ou d'outil,
au changement de profil et quand un autre processus Eidos modifie la liste.
Installer, renommer ou supprimer un mod change les dossiers sur le disque : une
annulation qui remonte avant l'une de ces opérations est refusée.

### Filtrer la liste

Le bouton Filters ouvre une ligne par critère : actif, en conflit, mise à jour
disponible, sans catégorie, recommandé sur Nexus (endorsed), suivi (tracked),
avec un id Nexus, avec des fichiers masqués, sans données de jeu valides, pour
un autre jeu, avec des notes, et une ligne par type de contenu (plugins,
archives BSA ou BA2, textures, maillages, scripts, plugins de script extender,
fichiers d'interface, sons, FaceGen). Chaque clic fait passer une ligne à
l'état suivant : désactivée, « only », « except », puis de nouveau désactivée.
La ligne du haut bascule entre les mods qui remplissent tous les critères
choisis et ceux qui en remplissent au moins un. Le champ de filtre par nom et
la liste des catégories restreignent toujours le résultat en plus. La ligne des
fichiers masqués ne regarde que les mods activés.

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

### Désactiver un mod que vos sauvegardes ou d'autres mods utilisent

Avant de désactiver ou de supprimer un mod, Eidos lit les listes de plugins des
trois sauvegardes les plus récentes du profil. Quand l'une d'elles utilise un
plugin de ce mod, une barre au-dessus de la liste nomme les plugins et attend
« Disable anyway » ou « Cancel » (Échap annule aussi). Remove ajoute les mêmes
noms à son message demandant de cliquer encore. Seules les sauvegardes de Skyrim
et de Fallout 4 sont lues ; pour les autres jeux, le mod est désactivé sans
question.

L'onglet General de « Information » d'un mod liste les mods dont il a besoin et les mods qui ont besoin de lui, lus dans les masters des plugins activés. Désactiver un mod dont d'autres ont besoin affiche la même barre avec « Disable them too », qui désactive aussi chaque mod qui en a besoin et les mods qui ont besoin de ceux-là, ou « Keep them enabled ». La carte des masters manquants de l'onglet Health propose « Enable what provides them » : elle active le mod qui contient chaque master manquant et coche un plugin master présent mais décoché.

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
Reveal sur n'importe quelle ligne dans un gestionnaire de fichiers. Les
`meta.ini`, `readme.txt`, `fomod/` et `Root/` placés à la racine d'un mod
restent hors de Data, dans cet onglet comme dans le montage : les trois premiers
servent au gestionnaire de mods, et `Root/` va dans le dossier du jeu. Un
fichier que le jeu ou un outil écrit sous l'un de ces noms arrive dans
l'Overwrite et reste visible. **Plugins**,
c'est l'ordre de chargement des ESP/ESM/ESL (activer, réordonner à la main, ou
trier avec LOOT et lire le rapport d'après-tri, dont les liens de conseils
s'ouvrent dans votre navigateur). Son en-tête compte les plugins actifs de chaque espace d'index face à la limite du jeu, par exemple `full 231/254, light 512/4096`, et devient orange quand l'un est plein. Après un tri LOOT, un plugin complet dont tous les nouveaux enregistrements tiennent dans la plage légère l'indique au survol, et le menu de sa ligne contient Flag as light. Eidos écrit une copie portant le drapeau léger dans un nouveau mod nommé `<plugin> (light)`, en bas de la liste des mods (la priorité la plus haute), et laisse l'original intact : désactiver ce mod annule l'opération. Eidos refuse quand l'une des trois sauvegardes les plus récentes du profil charge le plugin comme plugin complet. Seules les sauvegardes de Skyrim et de Fallout 4 sont lues : sous Starfield, aucune sauvegarde n'est vérifiée. LOOT ne vérifie que les plages d'enregistrements : avant de marquer un plugin qui ajoute des cellules ou des worldspaces, examinez-le dans xEdit. **Conflicts** explique les gagnants et les
perdants fichier par fichier. **Overwrite** transforme ce que le jeu a écrit en
un vrai mod en une seule étape. **Saves** analyse l'en-tête de chaque sauvegarde
- personnage, niveau, lieu, temps de jeu - et compare la liste de plugins qui y
est gravée à la vôtre, avec un bouton qui active les mods dont elle a besoin,
parce que les nommer et vous laisser faire le reste, c'est la moitié ennuyeuse.

Au survol d'un plugin s'affichent son origine, son auteur, sa version d'en-tête et de formulaire, les 300 premiers caractères de sa description, ses masters et, après un tri LOOT, les messages de LOOT. Sur Skyrim, Skyrim SE, Skyrim VR et Enderal SE, le jeu lit aussi un INI portant le nom d'un plugin chargé ; la colonne INI marque ces plugins, et un clic ouvre le fichier en lecture seule.

Une ligne `.exe` de l'onglet Data propose Ajouter, qui ouvre l'éditeur d'exécutables avec ce fichier déjà rempli (rien n'est enregistré avant d'appuyer sur Save), et Lancer, qui le lance une fois à travers la vue fusionnée, comme un outil enregistré. Enregistrer l'arbre écrit tout l'arbre Data fusionné dans un fichier texte, une ligne par fichier avec le mod qui le fournit, sans votre dossier personnel ni votre nom d'utilisateur, pour un rapport de bogue.

Les sauvegardes que Steam Cloud télécharge depuis un autre appareil, ou qu'écrit un lancement sans Eidos, arrivent dans le dossier de sauvegardes du jeu, que les sauvegardes du profil recouvrent pendant que le jeu tourne. L'onglet Sauvegardes les compte sur la ligne sous son titre, et Importer copie chacune avec son co-save dans le profil. Une sauvegarde dont le profil utilise déjà le nom est importée sous `orphan-<time>-<name>` ; rien n'est remplacé dans le profil.

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

**Preferences -> Nexus -> Install downloads when they finish** est désactivé par défaut. Une fois activé, un téléchargement Nexus qui se termine pendant que la fenêtre est ouverte s'installe sans passer par l'onglet Downloads. C'est l'installation habituelle : un FOMOD ouvre son installateur, et un nom déjà présent dans la liste des mods demande quoi faire. Les archives qui étaient dans `downloads/` avant l'ouverture de la fenêtre ou avant l'activation de l'option ne sont pas touchées, pas plus qu'un téléchargement dont vous avez annulé l'installation. Les membres d'une collection ouverte dans le volet des collections et le fichier qu'attend une mise à jour en cours ne sont installés que par ces parcours. Tant que le jeu ou un autre Eidos tient l'instance, l'installation attend qu'elle soit libre.

### Mises à jour des mods

Le bouton de mise à jour, à l'extrémité droite de la barre d'outils, compare
chaque fichier installé au fichier qui l'a remplacé sur Nexus. Les auteurs
marquent un nouvel envoi comme successeur d'un plus ancien, et Eidos suit cette
chaîne depuis l'identifiant de fichier que note le `meta.ini` du mod. Quand la
chaîne ne nomme aucun successeur, c'est la version de la page du mod qui est
comparée.

Track et Untrack, dans le menu du clic droit d'un mod, ajoutent le mod à vos
mods suivis sur Nexus ou l'en retirent, pour que les notifications du site
correspondent à ce que vous suivez ici. Chaque vérification des mises à jour lit
aussi vos recommandations et vos mods suivis sur Nexus (deux requêtes) et règle
les indicateurs Endorsed et Tracked de chaque mod en conséquence, sur le domaine
Nexus propre au mod. En cas de désaccord, Nexus l'emporte. Si l'une des deux
listes ne peut pas être lue, la vérification le signale et laisse ces
indicateurs tels qu'ils étaient.

L'onglet **Nexus files** de « Information... » liste les fichiers du mod sur
Nexus, du plus récent au plus ancien, et marque celui installé ici et celui qui
le remplace. Cette seconde ligne porte un bouton **Update**. Avec un compte
Premium, Eidos télécharge le fichier lui-même. Avec un compte gratuit, il ouvre
la page du fichier, où Mod Manager Download renvoie le fichier, et
l'installation démarre quand l'archive arrive dans `downloads/`. La suite est
l'installation habituelle : un FOMOD s'ouvre sur les réponses avec lesquelles le
mod a été installé, puis la fenêtre propose Replace avec « Keep a backup of the
existing mod » coché, quel que soit votre choix à la fenêtre précédente. Replace
garde la priorité du mod et son état activé ou non.

L'onglet **Page Nexus**, à côté, affiche la description du mod et, du plus
récent au plus ancien, le journal des modifications de chaque version de fichier
de sa page. Eidos les lit dans l'API GraphQL de Nexus, qui ne compte pas dans le
quota horaire de requêtes, la première fois que vous ouvrez l'onglet, et les
garde jusqu'à la fermeture d'Eidos ou jusqu'à ce que vous appuyiez sur
**Actualiser**. Ils s'affichent en texte brut : la mise en forme BBCode et HTML
est retirée, l'adresse d'un lien est écrite après son texte, et les images et
les vidéos sont laissées de côté. **Recommander** et **Suivre**, au-dessus du
texte, font ce que fait le menu du clic droit du mod. La page d'un mod pour
adultes reste masquée tant que votre compte Nexus n'affiche pas le contenu pour
adultes, et une page sans classification reste masquée dans tous les cas.

Avant qu'un Replace ne remplace le dossier d'un mod, Eidos compare l'ancien
dossier au nouveau par nom et taille de fichier, et lit les en-têtes des plugins
des deux. Cela couvre Update, une mise à jour de collection et Replace choisi à
la fenêtre d'installation. La ligne d'état compte ensuite les fichiers ajoutés,
supprimés et de taille modifiée, et nomme chaque master dont la nouvelle version
a besoin et pas l'ancienne. L'onglet **Général** de « Information... » se
termine par une section **Dernière mise à jour** qui liste les versions, les
plugins ajoutés ou supprimés, les masters ajoutés ou abandonnés, les plugins
dont l'indicateur ESM, ESL ou ESH a changé, les mods avec lesquels celui-ci
partage désormais des fichiers ou n'en partage plus, et les fichiers. Un fichier
réécrit à la même taille n'est pas listé. Le rapport est gardé dans
`logs/updates/<mod>.json` de l'instance, que `eidos pack` laisse de côté ; une
nouvelle installation ou un Merge dans le dossier le supprime. `eidos install
--replace` affiche les mêmes totaux sur une ligne `changes:`.

Un dossier qui contient plusieurs fichiers Nexus, par exemple un fichier
principal avec un fichier optionnel fusionné dedans, est refusé : le remplacer
par le nouveau fichier principal ferait perdre les autres. Mettez-les à jour à
la main.

`eidos nexus update skyrimse --apply` fait la même vérification, puis, avec un
compte Premium, télécharge chaque nouveau fichier et l'installe sur son dossier
avec `--replace --backup`. Avec un compte gratuit, il affiche plutôt
`OPEN <url>` pour chaque page de fichier. Les dossiers qui fusionnent plusieurs
fichiers sont sautés avec la raison. Un mod FOMOD sans réponses enregistrées,
par exemple installé avant qu'Eidos ne les garde, n'est pas réinstallé avec les
valeurs par défaut de l'installateur : cette mise à jour échoue avec un message
qui demande de mettre à jour le mod une fois depuis la fenêtre, qui enregistre
alors les réponses. Un FOMOD dont l'installateur ne propose aucun choix
s'installe normalement. La commande sort avec le code 1 si une mise à jour a
échoué.

### Collections Nexus

Collez un lien de collection - ou cliquez-en un sur le site - et Eidos liste les
membres de la révision, chacun rapproché de cette instance : installé,
téléchargé, ou manquant. *Install this collection* déroule ensuite toute la
recette sur un fil de travail, avec la même barre de progression que les travaux
d'empaquetage et de dépaquetage, et la barre devient le rapport quand il a fini.
Tout ce qui est dit plus haut sur ce qui est appliqué et ce qui est seulement
nommé vaut ici mot pour mot : la fenêtre et `eidos collection` sont un seul
moteur avec deux façades.

Sans lien, **Fichier -> Ouvrir une collection Nexus...** s'ouvre sur l'onglet **Parcourir**. Il liste les collections publiées pour le jeu de l'instance ouverte, 20 à la fois, par nombre de téléchargements, dernière mise à jour ou date de création, et le champ de recherche réduit la liste aux collections dont le nom ou le texte correspond. Chaque ligne donne l'auteur, la dernière révision, le nombre de mods, les téléchargements et une date. **Voir** ouvre la dernière révision de cette collection dans l'onglet **Lien**, comme si vous colliez son lien. Les collections marquées adultes sont écartées sauf si votre compte Nexus affiche le contenu adulte, les collections sans note le sont toujours, et la ligne sous la liste compte les deux. L'onglet **Parcourir** demande un compte Nexus connecté, comme le reste du volet.

Une collection ne peut être lue que contre **son propre jeu**. Ouvrez une
collection Skyrim avec une instance Fallout 4 chargée et il refuse en nommant le
jeu plutôt que de rapprocher les membres de la mauvaise liste de mods, où chaque
« installé » et chaque « manquant » serait du bruit déguisé en réponse.

Quand vous lancez la recherche de mises à jour, Eidos demande aussi à Nexus la dernière révision de chaque collection installée dans cette instance. Une ligne au-dessus de la liste des mods nomme chaque collection dont une révision plus récente existe et compte les mods ajoutés (+), mis à jour (~) et retirés (-) ; survolez-la pour voir leurs noms. Les comptes manquent quand la recette de la révision installée n'est plus en cache. **Mettre à jour** ouvre la révision récente dans le volet des collections, où *Install this collection* l'exécute. La liste est refaite à chaque vérification et n'est pas enregistrée.

Eidos garde les règles de chaque collection installée après l'installation. Quand un glisser-déposer ultérieur place deux de ses mods à l'encontre d'une règle d'ordre (avant ou après), l'onglet **Diagnostics** affiche une carte pour cette collection, qui nomme la première paire concernée et compte les règles qui ne sont plus respectées. Seuls les mods activés dans le profil actuel comptent, et les règles prises dans une boucle sont ignorées, car aucun ordre ne peut les respecter. **Restaurer l'ordre de la collection** remet les mods de la collection dans un ordre qui respecte chaque règle, garde votre propre ordre partout où les règles le permettent et laisse à leur place les mods qui ne font pas partie de la collection. Ctrl+Z annule l'opération. `eidos check` signale le même constat sous le code `collection_order_broken`. Le contrôle a besoin de la recette de la révision dans `collections/` : une collection dont la recette en cache a été supprimée n'est pas contrôlée.

### Mode hors ligne

**Preferences -> Nexus -> Offline** empêche complètement Eidos de contacter Nexus et saute la vérification quotidienne des versions sur GitHub.
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
`nexuscatmap.dat`), si bien qu'une instance partagée garde un seul catalogue. Un mod téléchargé depuis Nexus est classé à l'installation, dès que Récupérer depuis Nexus a associé le catalogue.

**View -> INI editor** modifie les INI de jeu du profil - la copie qui persiste,
plutôt que celle enfouie dans le préfixe Proton qui est écrasée à chaque
lancement. **View -> Log** lit les logs de session. **View -> Extensions** liste
vos propres add-ons ; voir [extensions.fr.md](extensions.md).

Quand un lancement ne trouve l'INI principal du jeu (`Skyrim.ini`, `Fallout4.ini`, `Oblivion.ini`...) ni dans le profil ni dans le préfixe, Eidos copie d'abord dans le profil celui par défaut du dossier du jeu (`Skyrim_Default.ini`, `Fallout4_Default.ini`...), comme MO2. Le préfixe et le dossier du jeu ne sont pas modifiés.

L'installation accepte tout : les chemins Simple et FOMOD, plus les paquets
**BAIN** de Wrye Bash (cochez les sous-paquets, qui fusionnent dans l'ordre) et
un sélecteur **manuel** qui montre l'arborescence de l'archive et vous laisse
désigner la racine des données quand aucune heuristique ne reconnaît la
disposition. Aucune archive n'est refusée.

Réinstaller ou mettre à jour un mod FOMOD ouvre l'assistant avec les réponses de
sa dernière installation cochées, sur la première étape dont les options ont
changé depuis, avec la liste des changements. **Reinstall with new choices**
part plutôt des valeurs par défaut de l'installateur. `eidos install --replace`
ou `--merge` reprend les réponses enregistrées et refuse d'installer, en listant
les changements, quand elles ne correspondent plus.

**Diagnostics** exécute des contrôles de santé en direct : la capacité à lancer
avant tout, les masters manquants (le prédicteur de crash le plus fiable qui
soit), les archives qu'aucun plugin actif ne chargera, si la liste de mods
correspond toujours au dossier mods, et - après une exécution - ce que dit le log
du script extender lui-même sur chacune de ses DLL de plugins, ce qui fait passer
« est-ce que mes plugins SKSE se sont chargés ? » de la déduction à la preuve.
Il signale aussi une mise à jour du jeu depuis le dernier lancement propre, un script extender absent ou qui ne correspond pas à la version du jeu (avec la page de téléchargement de l'extender et un bouton qui installe l'archive que vous téléchargez), une option de lancement Steam qui ne démarre pas Eidos avec `%command%`, NVAPI laissé désactivé pour DLSS, un plein écran exclusif qui empêche la génération d'images, un `enblocal.ini` d'ENB sans `LinuxVersion=true`, les déclarations des plugins F4SE et SFSE, et le log de crash de la dernière session avec les mods qu'il nomme.
La recherche de mises à jour lit aussi les prérequis Nexus de chaque mod. Diagnostics liste un mod requis absent ou désactivé, avec Ignore pour un prérequis rempli autrement.

Pour Skyrim SE, Enderal SE et Fallout 4, Diagnostics a aussi une carte Affichage. Elle montre la taille et le mode de la fenêtre réglés dans le `SkyrimPrefs.ini` du profil (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), ou dans la copie du préfixe quand le profil n'en a pas encore. Quand aucune des deux ne fixe de taille, ses champs partent du plus grand écran connecté listé dans `/sys/class/drm` : un Steam Deck affiche 1280x800. Apply écrit `iSize W`, `iSize H`, `bFull Screen` et `bBorderless` dans la copie du profil, après avoir adopté celle du préfixe si le profil n'en a pas. Quand SSE Display Tweaks règle la fenêtre, Apply écrit la même taille et le même mode dans sa copie dans Overwrite ; le fichier du mod lui-même n'est pas modifié.

Pour lancer le jeu depuis la GUI, réglez l'option de lancement Steam du jeu sur
le chemin absolu du binaire (Steam ne voit pas `~/.cargo/bin` dans le PATH) :

```
~/.cargo/bin/eidos-gui %command%
```

Eidos s'ouvre sur l'instance de ce jeu - celle que vous avez utilisée en dernier,
si bien qu'une instance portable est retrouvée tout comme la globale ; cliquez
sur Run pour le lancer à travers la vue fusionnée.

Avec **Start the game at once when a store opens Eidos** activé (Preferences, General, Running a game), Eidos se passe de ce clic : il vérifie Health et lance le jeu sans afficher sa fenêtre. La fenêtre s'ouvre comme d'habitude quand Health signale un problème, quand la liste des mods demande votre attention, quand un outil réglé pour passer avant le jeu a une sortie périmée ou, pour Skyrim SE, Enderal SE et Fallout 4, quand le profil n'a pas encore de taille de fenêtre (la fenêtre s'ouvre alors sur Diagnostics). Pour avoir la fenêtre le temps d'un lancement, placez `EIDOS_WINDOW=1` devant la commande : `EIDOS_WINDOW=1 ~/.cargo/bin/eidos-gui %command%`.

Hors de Steam, Run ne démarre pas le jeu lui-même avec Proton ou Wine. Il
demande à la boutique qui possède la copie de le démarrer, et la boutique
démarre Eidos :

- Steam exécute `steam -applaunch <appid>` quand l'option de lancement ci-dessus
  est réglée. Steam démarre d'abord s'il est fermé.
- Heroic (une copie GOG ou Epic, native ou Flatpak) reçoit un lien
  `heroic://launch` quand Eidos est un wrapper dans les réglages du jeu dans
  Heroic, avec le chemin absolu de `eidos-gui` comme exécutable et aucun
  argument. Heroic démarre d'abord s'il est fermé.
- Legendary seul exécute `legendary launch <app>` quand
  `~/.config/legendary/config.ini` contient `wrapper =` et le chemin absolu de
  `eidos-gui` dans la section `[<app>]` du jeu, ou dans `[default]`.

Eidos ouvre alors une deuxième fenêtre sur l'instance utilisée en dernier, et
vous cliquez sur Run dans celle-ci. La première fenêtre ne suit pas cette
partie : appuyez sur F5 dans celle-ci après avoir joué pour recharger ses
listes. Quand le réglage de la boutique manque, ou que le loader de l'extenseur
de scripts n'est pas à côté de l'exécutable du jeu, Run ne démarre rien et
indique quoi régler et où, avec le vrai chemin du binaire en cours d'exécution.

### Variables et wrappers par profil

Un profil peut porter ses propres variables d'environnement et commandes wrapper dans `profiles/<nom>/launch-env.ini`, une entrée par ligne :

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

Elles s'appliquent à chaque `eidos play` de ce profil (le jeu lancé par le `%command%` de Steam, ou par Run dans une fenêtre qu'une boutique a ouverte) et à chaque lancement d'outil. `eidos shell` reçoit les variables, sans wrapper. La première ligne `wrapper=` est la plus extérieure. Une ligne `wrapper=` qui contient `%command%` reçoit la commande à cet endroit, comme dans les options de lancement de Steam ; une ligne sans `%command%` reçoit la commande à la fin. Les wrappers tournent dans l'espace de noms privé d'Eidos, ils voient donc les mods.

Une variable placée ici remplace la valeur qu'Eidos définit pour le même nom, et la ligne `env=` propre à un outil dans `tools.ini` remplace les deux. `WINEDLLOVERRIDES` fait exception : elle est ajoutée après les overrides qu'Eidos définit pour ENB, ReShade et les DLL embarquées, si bien qu'une DLL citée dans les deux prend le réglage du profil et que les autres gardent le leur.

View -> INI editor ouvre le fichier comme dernier onglet, pour tous les jeux. À l'enregistrement, la liste des numéros de ligne qu'Eidos ignore apparaît : les lignes qui ne sont ni `env=NOM=valeur` ni `wrapper=commande`. `eidos play <instance>` sans commande liste les variables que reçoit un lancement et les wrappers. Copy report dans Diagnostics nomme les variables et les programmes wrapper, sans leurs valeurs.

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
