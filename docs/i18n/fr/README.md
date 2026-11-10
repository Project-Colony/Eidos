<!-- eidos-i18n: source=README.md sha=37e97bafbd2cf43f2101a28b23c1464420a5f9ec -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Un gestionnaire de mods natif pour Linux qui ne touche jamais au dossier de votre jeu.**

</div>

> 🇬🇧 L'anglais fait foi. En cas de désaccord entre cette page et
> [README.md](../../../README.md), c'est l'anglais qui a raison.

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

Eidos apporte aux jeux Bethesda sous Linux ce que Mod Organizer 2 leur apporte
sous Windows : une vue fusionnée de vos mods, construite à chaque lancement. Il
s'appuie sur les espaces de noms de montage de Linux et sur FUSE plutôt que sur
le détournement d'API Windows, et le gestionnaire tourne sans Wine. Rien n'est
copié dans le dossier du jeu, donc il n'y a rien à nettoyer ensuite. Eidos a
aussi des définitions pour Stellar Blade, Stardew Valley, 7 Days to Die et
Crimson Desert.

Les copies Steam lancent Eidos depuis l'option de lancement du jeu. Les copies
GOG et Epic dans Heroic ou Legendary le lancent comme wrapper. Le jeu démarre
toujours par la boutique qui possède la copie ; Eidos ne le démarre jamais
lui-même avec Proton.

```
Steam ──> eidos-gui %command% ──> [ espace de noms privé ]
                                  │  mods ⊕ jeu  ──> ce que le jeu voit
                                  └─ meurt avec le jeu ; l'installation reste intacte
```

> **État :** Skyrim SE est joué tous les jours avec Eidos, avec SKSE, des
> préchargeurs de script extender, Creation Club, des ordres de chargement triés
> par LOOT et des sauvegardes par profil. Une seule famille de jeux est
> éprouvée en jeu réel à ce jour. Dix autres jeux Bethesda et quatre autres jeux
> sont câblés et attendent des testeurs.

## Pourquoi Eidos

La vue fusionnée est montée dans un espace de noms de montage privé que seul
l'arbre de processus du jeu peut voir. Les autres programmes, un second jeu
compris, ne la voient jamais et n'ont besoin d'aucune autorisation pour elle.
Quand le jeu se termine ou est tué, ou que la machine perd le courant, l'espace
de noms disparaît avec ses processus et l'installation est comme avant.

Chaque profil possède sa liste de mods, l'ordre de ses plugins, ses INI et ses
sauvegardes. Au lancement, les fichiers de plugins et le dossier de sauvegardes
sont montés par liaison sur les chemins du jeu, si bien que ce que le jeu y
écrit atterrit dans le profil. Changer de profil change tout cela.

Eidos n'a besoin ni d'un assistant setuid, ni d'un démon système, ni de
`sudo setcap`, ni d'une modification de `/etc/fuse.conf`. Le démon FUSE tourne
sous votre utilisateur pour un seul lancement et se termine avec le jeu. Vous
installez un programme et réglez une option de lancement Steam.

Avant chaque exécution, Eidos garde un instantané de la liste des plugins. Si un
plantage abîme la liste, Diagnostics la compare à cet instantané, et Restore the
pre-session order remet l'ancien ordre. Une capture qui effacerait votre ordre de
chargement est refusée, avec la raison.

Test run démarre le jeu avec les mods du profil, sans mods, ou avec seulement les
mods sélectionnés et leurs masters. Les sauvegardes, les modifications d'INI,
l'état des plugins, la production d'Overwrite et les autres fichiers de My Games
de cette exécution, journaux compris, vont dans `<instance>/test-run/`. Quand le
jeu se termine, Eidos liste ces cinq groupes et vous gardez ou jetez chacun.
Les autres écritures, comme le registre de Wine, vont là où elles vont
toujours. Find the mod behind a problem, dans Diagnostics, lance des test runs
avec la moitié des mods suspects à la fois, plus les masters dont ils ont
besoin. Après chaque exécution, vous répondez si le problème s'est produit, et
Eidos réduit les suspects jusqu'à nommer le mod, ou les mods qui ont besoin des
masters les uns des autres ; 200 suspects demandent environ 8 exécutions. Les
mods sélectionnés dans la liste avant de commencer restent activés à chaque
étape. Détails dans [usage.md](docs/guide/usage.md#parties-test) et
[usage.md](docs/guide/usage.md#trouver-le-mod-à-lorigine-dun-problème).

### Comment il se compare

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / déployeurs par liens |
|---|---|---|---|---|
| Gestionnaire natif | oui | non, une appli Windows dans Wine | oui (portage Qt) | oui |
| Dossier du jeu intact | oui, toujours | oui | oui | non, des liens y sont écrits |
| Montage visible par | le jeu seul | le jeu seul | tout le système | sans objet |
| Nettoyage après plantage | aucun | aucun | récupération de montage mort | dé-déploiement manuel |
| Mods root (ENB, préchargeurs) | oui, nativement | greffon requis | greffon requis | partiel |
| Privilèges requis | aucun | aucun | modifier `/etc/fuse.conf` | aucun |

### Sa rapidité

| | avant | maintenant |
|---|---|---|
| charger une sauvegarde | ~20 secondes | 6-7 secondes |
| lectures de dossiers dans une session | 5,6 millions | 335 mille |

Les changements de cellule sont immédiats. Trouver un fichier interrogeait
auparavant chacun des cinquante mods l'un après l'autre, et lister un dossier le
faisait cinquante fois. Les couches en lecture seule sont maintenant indexées une
fois au montage, donc trouver un fichier est une seule recherche par hachage, et
la liste fusionnée d'un dossier est elle aussi calculée une fois au montage. Les
chiffres viennent d'une instance réelle jouée normalement, pas d'un banc d'essai :
[internals/performance.md](../../internals/performance.md).

### Travaux antérieurs et remerciements

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) et
  [usvfs](https://github.com/ModOrganizer2/usvfs) : la sémantique qu'Eidos
  reproduit, et la base de code face à laquelle sa parité a été étudiée
- [LOOT](https://loot.github.io/) : le moteur de tri, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) et les autres gestionnaires Linux, qui
  montrent qu'une communauté veut voir ce problème résolu

## Ce qu'il fait

### Mods

Eidos installe les archives simples, les assistants FOMOD, les paquets BAIN de
Wrye Bash, les conteneurs OMOD dont les scripts OBMM sont pris en charge, et tout
le reste par un sélecteur manuel. Les mods root (préchargeurs de script extender,
ENB, Engine Fixes) fonctionnent nativement, sans greffon Root Builder et sans
rien copier dans votre installation. Vous pouvez masquer des fichiers isolés,
regrouper les mods avec des séparateurs, déplacer une sélection vers une cible,
garder des notes et des catégories par mod, et importer un profil MO2.

La liste fonctionne comme celle de MO2 : huit colonnes optionnelles et un tri sur
chacune, regroupement par catégorie ou par source, gestes au double-clic, saisie
pour sauter à un nom, sauvegardes par mod qui ne font rien tant que vous ne les
restaurez pas, et des drapeaux pour un mod dont la disposition ne sera pas
chargée par ce jeu ou qui a été téléchargé pour un autre. Les filtres choisissent
les mods selon leur état Nexus (approuvé, suivi, mise à jour disponible), les
notes, les fichiers masqués et le type de contenu (plugins, archives, textures,
meshes, scripts, sons et plus). L'arbre de fichiers crée des dossiers, renomme,
supprime et ouvre des fichiers, et prévisualise images et textes sans rien
lancer.

Ctrl+Z et Ctrl+Maj+Z annulent et rétablissent les changements de la liste de mods
et de l'ordre de chargement. Désactiver ou supprimer un mod qu'utilise l'une des
trois sauvegardes les plus récentes du profil demande d'abord confirmation et
nomme les plugins (sauvegardes Skyrim et Fallout 4). Réinstaller ou mettre à jour
un mod FOMOD ouvre l'assistant avec les réponses de sa dernière installation.
Les informations d'un mod listent les mods dont il a besoin et ceux qui ont
besoin de lui, lus dans les masters des plugins activés.

### Plugins

L'ordre de chargement a le tri LOOT intégré, les index de mods tels que le jeu
les calcule, les avertissements de masters manquants, et vos DLC et contenus
Creation Club affichés comme des lignes non gérées. L'en-tête compte les places
utilisées dans chaque espace d'index (full, medium, light). Survoler un plugin
montre son auteur, sa description et la version de son en-tête. Sur les moteurs
Skyrim, un plugin dont l'INI du même nom est chargé par le jeu est signalé.

Les plugins que LOOT signale comme sales sont nettoyés par QuickAutoClean de
xEdit dans un mod `Cleaned plugins`, jamais sur place. Flag as light, into a new
mod écrit une copie légère d'un plugin, et refuse si l'une des sauvegardes les
plus récentes charge ce plugin comme plugin complet (sauvegardes Skyrim et
Fallout 4).

### Instances

Une instance est globale, rangée sous `~/.local/share/Colony/Eidos/instances/`,
ou portable : un dossier autonome où vous voulez (un second disque, une partition
de jeux), déplaçable et isolé comme celles de MO2. Les instances portables sont
mémorisées d'une session à l'autre. La GUI, le lancement Steam et chaque commande
suivent celle que vous avez utilisée en dernier, et toute commande accepte le
dossier partout où elle accepte un identifiant de jeu. Détails dans
[usage.md](docs/guide/usage.md#instances--globale-et-portable).

Une instance peut aussi lancer le jeu depuis une copie du dossier du jeu située
dans l'instance. À la première ouverture d'une liste Wabbajack, Eidos utilise la
copie propre à la liste : le dossier que nomme son `ModOrganizer.ini`, sinon un
dossier `Stock Game` ou `Game Root`. Freeze this version, sur la carte Game files
de Diagnostics, copie l'installation de la boutique dans `<instance>/Stock Game`,
si bien qu'une mise à jour de la boutique ne change plus les fichiers que le jeu
exécute. Quand les deux dossiers sont sur un même système de fichiers capable de
partager des blocs, comme btrfs ou XFS, la copie partage ces blocs avec
l'installation de la boutique (reflink). Sur ext4, ou entre deux systèmes de
fichiers, c'est une copie complète, et la taille et l'espace libre sont affichés
d'abord. C'est toujours la boutique qui démarre le jeu. Return to the store
version et Delete the copy annulent l'opération, et
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
fait la même chose dans un terminal. Morrowind, et Oblivion avec
`bUseMyGamesDirectory=0`, ne peuvent pas tourner depuis une copie. Détails dans
[usage.md](docs/guide/usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie).

### Profils

Chaque profil a son propre ordre de mods, son état de plugins, ses INI et ses
sauvegardes. Les sauvegardes sont analysées et comparées à vos plugins actuels,
avec un bouton qui active ce dont une sauvegarde a besoin, et recopiées dans le
préfixe pour Steam Cloud après chaque session. Les sauvegardes que Steam Cloud a
apportées d'un autre appareil, ou qu'un lancement sans Eidos a laissées dans le
préfixe, sont listées dans l'onglet Saves. Import les copie dans le profil, et
une sauvegarde dont le nom est déjà pris par le profil arrive sous un nouveau nom
en `orphan-` au lieu de le remplacer.

Un profil peut porter des variables de lancement et des wrappers
(`env=NOM=valeur` et `wrapper=commande`, un par ligne, dans l'éditeur d'INI) pour
les exécutions de son jeu et de ses outils. Pour Skyrim SE, Enderal SE et
Fallout 4, une carte Display dans Diagnostics règle la taille et le mode de la
fenêtre dans les copies d'INI du profil. Backups garde des points de restauration
des INI du profil, et Eidos en prend un avant chaque exécution de BethINI Pie. Le
raccourci de bureau d'un profil démarre le jeu par sa boutique. `eidos sessions`
liste les exécutions enregistrées et compare l'une d'elles à l'état actuel.

### Nexus

Connectez un compte pour recevoir les liens Mod Manager Download, vérifier les
mises à jour et consulter les auteurs et leurs profils. Chaque mod est vérifié,
approuvé et lié sur le domaine Nexus d'où vient son fichier, si bien qu'un mod
Skyrim SE dans une instance Enderal SE reste sur le site de Skyrim SE. Une mise à
jour suit la chaîne de successeurs que l'auteur a définie sur Nexus, remplace le
dossier du mod par le nouveau fichier (une sauvegarde de l'ancien est proposée),
garde sa priorité et son état activé, et rejoue les réponses FOMOD enregistrées à
l'installation. `eidos nexus update <instance> --apply` fait la même chose dans un
terminal avec un compte premium, et affiche les pages de fichiers avec un compte
gratuit. Track et Endorse sont dans le menu du mod, et chaque vérification de mise
à jour les synchronise avec votre compte Nexus. Les informations d'un mod ont un
onglet Nexus page avec sa description et son journal des modifications en texte
brut, et leur onglet General se termine par ce que la dernière mise à jour a
changé : les fichiers ajoutés, supprimés et redimensionnés, et les masters ajoutés
ou retirés. Install downloads when they finish (Preferences, Nexus, désactivé par
défaut) installe un téléchargement dès qu'il se termine, tant que la fenêtre est
ouverte.

Les collections s'installent à partir de leurs recettes, avec une progression
reprenable et des rapports explicites sur les étapes non prises en charge. La
correspondance vérifie les fichiers source exacts ; les anciennes métadonnées
incomplètes restent non vérifiées. Le volet des collections parcourt et cherche
les collections publiées pour un jeu, montre quand une collection installée a une
révision plus récente, et guide un compte gratuit à travers les fichiers
manquants, une page à la fois. Quand un glisser-déposer casse les règles d'ordre
d'une collection, Diagnostics le dit, et Restore collection order remet ses mods
en place. L'onglet Downloads filtre, trie, masque et supprime les archives. Le
mode hors ligne (Preferences, Nexus) arrête l'accès à Nexus et la vérification
quotidienne de version.

### Outils

xEdit, BodySlide, DynDOLOD et d'autres outils de modding s'exécutent à travers la
vue fusionnée dans le préfixe Wine du jeu (le préfixe Proton de Steam, ou celui
qu'Heroic a créé). Ils voient vos mods, leur production atterrit dans Overwrite,
et un clic transforme cette production en vrai mod. Pour Skyrim SE, Eidos trouve
SSEEdit, SSEEdit QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide,
DynDOLOD, TexGen, xLODGen, Synthesis et BethINI Pie dans le dossier du jeu, dans
un mod, ou dans le dossier d'outils que vous gardez à côté de vos jeux, chacun
avec les runtimes dont il a besoin. Les autres jeux ont leur propre liste.
Pandora, Nemesis, FNIS, BodySlide et Synthesis écrivent leur production dans un
mod nommé (`Nemesis Output` et ainsi de suite).

Le paquet de version fournit les installateurs de Visual C++ 2015-2022 et du .NET
8 Desktop Runtime de Microsoft, ainsi que le runtime .NET 10. Eidos installe
`vcrun2022` et .NET 8 avec ces installateurs, sans winetricks, et sans le paquet
il télécharge les mêmes fichiers chez Microsoft, chacun vérifié contre le SHA-256
intégré à Eidos. Synthesis reçoit le SDK .NET 10, un téléchargement de 300 Mo que
le paquet ne fournit pas. Les autres prérequis (.NET Framework 4.8, XACT et les
anciennes versions de .NET) passent toujours par winetricks. Rien n'est
téléchargé tant que vous ne demandez pas d'installer les prérequis d'un outil.

Épinglez les outils que vous utilisez, masquez les autres, donnez à un outil son
propre AppID Steam quand il est sa propre application Steam, et écrivez un
raccourci `.desktop` qui le lance à travers la vue fusionnée sans ouvrir Eidos.
`eidos tool <game> import-mo2` liste les exécutables du `ModOrganizer.ini` du
dossier d'une liste, et `--write` les ajoute. Un exe de l'onglet Data peut être
lancé une fois ou ajouté comme outil, et `eidos tool <instance> run --exe <path>`
en lance un depuis un terminal. Avec Ask before playing, Run demande d'abord
quand la production d'un générateur est périmée.

Stop termine un jeu ou un outil en cours. Le menu File lance winecfg, regedit,
l'explorateur de fichiers de Wine et le désinstalleur Windows dans le préfixe du
jeu. Rebuild prefix, dans Diagnostics, renomme le préfixe pour que Steam en crée
un nouveau, et ne supprime rien. `eidos shell <instance>` ouvre un shell dans la
vue fusionnée, ou exécute la commande donnée après `--`, et ce qu'il écrit
atterrit dans Overwrite.

Quand la fenêtre est en arrière-plan, Eidos peut envoyer une notification de
bureau à la fin d'une exécution d'outil, d'une installation de collection, d'un
empaquetage ou dépaquetage, ou du dernier téléchargement en cours, et quand un
tri LOOT échoue. Notify the desktop when a long job ends, dans Preferences,
l'active ; c'est désactivé par défaut.

### Diagnostics

L'onglet Diagnostics liste les masters manquants, les archives orphelines, la
dérive de la liste de mods et les jeux de plugins abîmés, et après une exécution,
ce que le journal du script extender dit avoir chargé. Il signale aussi :

- un script extender absent ou qui ne correspond pas à la version du jeu
- un jeu qui a changé depuis le dernier lancement propre
- une option de lancement Steam qui ne démarre pas Eidos ou n'a pas de `%command%`
  après lui
- un Proton plus ancien que celui qui a configuré le préfixe en dernier, ou un
  Proton du Steam Flatpak
- `PROTON_ENABLE_NVAPI` absent pour DLSS, et le plein écran exclusif, qui bloque
  la génération d'images de Community Shaders
- un `enblocal.ini` sans `LinuxVersion=true`
- le journal de plantage de la dernière session et les mods qu'il nomme
- des prérequis Nexus manquants ou désactivés
- un gestionnaire `nxm://` qui est un autre programme ou un fichier absent
- des lignes `sTestFile`, qui font ignorer `plugins.txt` au jeu
- des plugins `.esp` activés dans Starfield, et des plugins blueprint invalides
  ou non appariés
- un patch dont les fichiers libres perdent face au mod qu'il corrige
- un mod dont les copies des scripts Papyrus du script extender l'emportent sur
  celles de l'extender
- un `vm.max_map_count` ou une limite de fichiers ouverts trop bas
- des fichiers de jeu sur un système de fichiers Windows
- des chemins de données BodySlide qui ne sont pas le dossier Data du jeu, et des
  dossiers de `Overwrite/Root` qui vont dans Data
- un `ModOrganizer.ini` de liste qui pointe les outils vers un autre dossier de
  jeu
- une copie de jeu absente ou sans exécutable du jeu

`eidos check <instance> [--json]` lance les vérifications qui n'ont pas besoin de
fenêtre et se termine avec le code 1 quand il trouve un problème. Copy report
copie un rapport où votre dossier personnel est écrit `~` et votre nom
d'utilisateur `<user>`, et Save tree, dans l'onglet Data, écrit l'arbre Data
fusionné avec la source de chaque fichier, expurgé de la même façon.

Consultez [Contrôles d'intégrité et fichiers générés](../../internals/integrity.md) (en anglais) pour la protection des remplacements, les conflits entre fichiers BSA/BA2, les diagnostics statiques SKSE et les traces de génération, ainsi que leurs limites.

### Où il range ses propres fichiers

Eidos utilise la disposition que suit chaque programme de la famille Colony :
`~/.config/Colony/Eidos/` pour ce que vous avez choisi (préférences, votre session
Nexus, votre liste d'instances, les définitions de jeux et d'extensions que vous
avez écrites), `~/.local/share/Colony/Eidos/` pour les instances globales, les
runtimes d'outils (téléchargés, ou copiés depuis le paquet par `install.sh`) et
les journaux de session, et `~/.cache/Colony/Eidos/` pour les masterlists LOOT.
Dans une instance, `Stock Game` contient une copie figée du jeu et `test-run/`
contient un test run en attente de revue.

Un Eidos plus ancien rangeait ses fichiers dans `~/.config/eidos/`,
`~/.local/share/eidos/` et `~/.local/state/`. Le premier lancement après mise à
jour les transfère, le dit dans le journal, et laisse les anciens dossiers tels
qu'ils étaient. Tant qu'un transfert n'est pas terminé, Eidos continue d'utiliser
les anciens fichiers. Détails dans [internals/paths.md](../../internals/paths.md)
(en anglais).

### Jeux pris en charge

Skyrim SE/AE est éprouvé en jeu réel. Fallout 4 est lui aussi câblé de bout en
bout (F4SE substitué automatiquement, invalidation d'archives, ordre de
chargement à astérisques, LOOT, sauvegardes `.fos`) ; voir
[guide/fallout4.md](docs/guide/fallout4.md). Câblés et en attente de testeurs :
Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV, Starfield,
Oblivion et Morrowind. Fallout 3, Fallout NV, Oblivion et Morrowind ordonnent
leurs plugins par la date des fichiers : Eidos garde un ordre par profil et
montre ces dates dans le montage, sans modifier les fichiers sur le disque.

Quatre autres jeux sont câblés et attendent aussi des testeurs : Stellar Blade
(mods Unreal `.pak` dans `~mods`), Stardew Valley (mods SMAPI, SMAPI étant
installé séparément), 7 Days to Die (dossiers `Mods/` à côté du jeu) et Crimson
Desert. Crimson Desert ne prend que des mods de code : plugins ASI, ReShade et
DLL de chargeur dans `bin64`. Ses mods de données, qui vivent dans des groupes
PAZ, ne sont pas pris en charge.

Un jeu est un fichier TOML, et les vôtres peuvent aller dans
`~/.config/Colony/Eidos/games/` sans recompiler :
[internals/adding-games.md](../../internals/adding-games.md).

## Installation

### Via Colony (recommandé)

Cherchez **Eidos** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le. Les mises à jour arrivent par le lanceur. Colony installe un seul
fichier, `eidos-linux` : la fenêtre, et aussi la ligne de commande `eidos` quand
il est appelé à travers un lien nommé `eidos`. Ce fichier ne contient ni
l'utilitaire d'aperçu des modèles NIF, ni les runtimes Microsoft du paquet
ci-dessous. Sans l'utilitaire, l'aperçu le signale et tout le reste fonctionne, et
Eidos télécharge un runtime chez Microsoft quand vous installez les prérequis d'un
outil. Que faire après l'installation, et comment y faire pointer Steam :
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Téléchargement direct du binaire

Prenez un fichier dans la [dernière version](https://github.com/Project-Colony/Eidos/releases/latest) :

| Fichier | Ce que c'est |
|---|---|
| `eidos-linux` | tout le programme en un seul fichier, tel que Colony l'installe |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, l'utilitaire d'aperçu NIF, les installateurs de Visual C++ 2015-2022 et du .NET 8 Desktop Runtime et le runtime .NET 10, les icônes, les mentions de licence et un installateur qui enregistre l'entrée de bureau et le gestionnaire de téléchargements Nexus |

Vérifiez d'abord le téléchargement, comme le montre la
[politique de signature du code](#politique-de-signature-du-code), puis :

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

L'installateur met les binaires dans `~/.local/bin` sans sudo et copie le dossier
`runtimes/` du paquet vers `~/.local/share/Colony/Eidos/runtimes`. `--system` et
`--bindir DIR` placent les binaires ailleurs, et `--from DIR` les prend dans un
autre dossier. `--cap` demande à sudo `CAP_SYS_ADMIN`, dont seul le passthrough
FUSE expérimental a besoin, et le passthrough reste désactivé ; `--no-cap` est la
valeur par défaut. Les deux fichiers exigent Linux x86_64 avec glibc 2.39 ou plus
récent, FUSE (`fusermount3`) et 7-Zip pour les archives de mods.

Eidos ne se met pas à jour tout seul. Une fois par jour au démarrage, la fenêtre
demande à GitHub la dernière version, et quand une plus récente existe elle
affiche un lien vers ses notes de version après Help dans la barre de menus.
Preferences le désactive, et une copie sous `/usr` hors de `/usr/local`, qu'un
gestionnaire de paquets met à jour, ne le demande jamais.

### Compiler depuis les sources

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Exige Rust 1.92 ou plus récent (la `rust-version` que vérifie la CI ; la CI et
les versions publiées compilent avec 1.99.1), ainsi que `just`, CMake 3.20+ et un
compilateur C++17 pour l'utilitaire d'aperçu NIF. `just build` utilise le profil
release : une seule unité de génération de code et une LTO complète. Une
compilation à froid de `eidos` et `eidos-gui` avec 3 tâches a pris 334 secondes
ainsi et 243 secondes avec une LTO légère, et la LTO complète rend `eidos` plus
petit de 1,2 Mo et `eidos-gui` de 3,8 Mo (mesures dans `Cargo.toml`).
`just profile=dev build` compile plus vite. Un paquet Arch se compile depuis
`packaging/PKGBUILD`. Prérequis et commandes sans `just` :
[docs/guide/install.md](docs/guide/install.md).

### Options de lancement Steam

Eidos s'exécute comme commande de lancement du jeu, et c'est ainsi qu'il monte
avant que le jeu démarre. La ligne de base suffit à la plupart des configurations,
avec le chemin absolu de votre installation (Steam ne lit pas votre `PATH`) :

```
~/.local/bin/eidos-gui %command%
```

Depuis Colony, ce chemin est `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Appuyez ensuite sur Jouer : Eidos s'ouvre sur l'instance de ce jeu, et Run
démarre le jeu à travers la vue fusionnée.

Dans une fenêtre que vous avez ouverte vous-même, Run demande à Steam de démarrer
le jeu (`steam -applaunch <appid>`), donc l'option de lancement s'applique
toujours, et vous appuyez sur Run dans la fenêtre que Steam ouvre. Heroic reçoit
un lien `heroic://launch`, et Legendary exécute `legendary launch <app>`, une fois
Eidos réglé comme wrapper du jeu dans ces outils. Quand le réglage de la boutique
manque, Run ne démarre rien et dit quoi régler :
[guide/usage.md](docs/guide/usage.md#lancer-le-jeu). Avec Start the game at
once when a store opens Eidos (Preferences, désactivé par défaut), Eidos lance les
vérifications de Diagnostics et démarre le jeu sans montrer sa fenêtre quand rien
ne demande d'attention.

Tout le reste est constitué de variables d'environnement empilées devant, et
elles se combinent librement :

| Vous voulez... | Mettez devant |
|---|---|
| DLSS avec Community Shaders | `PROTON_ENABLE_NVAPI=1`. Sans elle, DLSS ne s'initialise jamais, sans rien dire ; la liste complète est dans [guide/graphics.md](docs/guide/graphics.md) |
| un compteur d'IPS à l'écran | `DXVK_HUD=fps` |
| l'interpolation d'images au niveau du pilote, sans mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, jamais en même temps que la génération d'images de Community Shaders |
| des journaux détaillés pour un rapport de bug | `EIDOS_LOG=debug` (les journaux de session vont dans `~/.local/share/Colony/Eidos/logs/`) |
| un rapport d'E/S par session depuis le montage | `EIDOS_FUSE_STATS=1` |
| un autre nombre de travailleurs FUSE | `EIDOS_FUSE_THREADS=8` (4 par défaut ; `1` est la première chose à essayer face à un bug de concurrence) |
| épingler ce lancement à une instance portable | `EIDOS_INSTANCE=/chemin/vers/dossier`. Sans elle, Eidos ouvre l'instance utilisée en dernier |
| la fenêtre pour un lancement alors que Start the game at once est activé | `EIDOS_WINDOW=1` |

La ligne à garder pour une installation moddée moderne (Community Shaders, DLSS,
génération d'images) est la commande finale, pas un exemple :

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Ajoutez `DXVK_HUD=fps` devant le temps de vérifier que tout marche, puis retirez-le.

Les mêmes variables peuvent aller dans les variables de lancement du profil au
lieu de la ligne Steam, ce qui les garde par profil et les applique aussi aux
exécutions d'outils :
[guide/usage.md](docs/guide/usage.md#variables-et-wrappers-par-profil).

Les interrupteurs de diagnostic plus profonds (`EIDOS_FUSE_TRACE`, les bascules de
bissection du cache et de l'index, pourquoi `EIDOS_FUSE_PASSTHROUGH` est
désactivé par défaut) sont dans
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentation

Tout est dans [docs/](../../), indexé selon le lecteur dans
[docs/README.md](docs/README.md) : le guide de l'utilisateur (installation, la
ligne de commande et la GUI, des outils comme xEdit et DynDOLOD, le graphisme, le
dépannage), le fonctionnement interne d'Eidos, et pourquoi il existe. Politique
de sécurité et comment signaler une vulnérabilité :
[SECURITY.md](../../../SECURITY.md) (en anglais).

## Langue

Les pages dont un joueur a besoin sont traduites. L'anglais fait foi : quand une
traduction le contredit, c'est le fichier anglais qui a raison.

- **Français** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

Tout le reste est en anglais. `docs/internals/` et `docs/project/` sont lus par
des gens qui lisent aussi le Rust, `CHANGELOG.md` est généré, et `SECURITY.md`
n'est pas traduit non plus. Traduire les deux premiers ferait plus de 19 000 mots
de plus à tenir à jour pour des lecteurs qui n'en ont pas besoin.

Chaque traduction porte l'empreinte du fichier anglais dont elle est issue, et la
CI échoue tant que cette empreinte ne correspond plus à l'anglais ; voir
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh). La traduction est alors
mise à jour et réestampillée avec `scripts/i18n-check.sh --fix`. Une traduction
périmée est mise à jour, pas supprimée.

Une langue tient dans un seul dossier. `docs/i18n/<lang>/` reproduit
l'arborescence de la racine du dépôt, si bien que
`docs/i18n/de/docs/guide/install.md` est le `docs/guide/install.md` allemand.
C'est pourquoi un lien entre deux pages traduites est la même chaîne que le lien
entre leurs originaux anglais, et pourquoi retirer une langue se fait avec un seul
`rm -r`.

## Politique de signature du code

Chaque fichier publié est signé par l'organisation Project Colony dans la CI,
jamais sur la machine d'un développeur. À côté de chaque fichier, sur la page de
la version :

| Fichier | Ce que c'est |
|---|---|
| `<asset>.sig` | une signature ed25519 du fichier, faite avec la clé de publication de l'organisation |
| `<asset>.meta` | trois lignes qui lient le fichier à son nom, à son sha256 et au numéro de version |
| `<asset>.meta.sig` | une signature ed25519 du `.meta` |

La clé privée est un secret de l'organisation, utilisé uniquement par le
[workflow sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
partagé, dans une tâche qui ne compile rien ; les tâches qui compilent Eidos ne
la voient jamais. Colony vérifie les trois fichiers avant d'installer ou de
mettre à jour Eidos, et refuse une version plus ancienne que celle installée.
Pour vérifier vous-même un téléchargement avec OpenSSL 3 (les mêmes commandes
valent pour chaque fichier) :

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

Comment les versions sont compilées, signées et publiées :
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing) (en anglais).

## Confidentialité

Eidos n'envoie ni télémétrie, ni statistiques d'utilisation, ni rapports de
plantage. Il ne se connecte qu'aux serveurs de ce tableau, et seulement pour
l'action qui y est indiquée. Le mode hors ligne (Preferences, Nexus) arrête toute
connexion à Nexus et la vérification quotidienne de version.

| Donnée | Stockée ou envoyée | Où, et pourquoi |
|---|---|---|
| Préférences, liste d'instances, définitions de jeux et d'extensions | stockées | `~/.config/Colony/Eidos/` |
| Instances, runtimes d'outils, journaux de session | stockés | `~/.local/share/Colony/Eidos/`, ou le dossier choisi pour une instance portable |
| Masterlists LOOT | stockées | `~/.cache/Colony/Eidos/loot/` |
| Vérification de version | envoyée | Au démarrage, au plus une fois par jour, une requête vers `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos lit seulement l'étiquette de la version et ne télécharge rien. Elle est activée par défaut et se coupe dans Preferences, et elle est ignorée en mode hors ligne et pour une copie installée sous `/usr` hors de `/usr/local`, qu'un gestionnaire de paquets met à jour. [Déclaration de confidentialité de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Connexion à Nexus | envoyée | Se connecter ouvre `users.nexusmods.com` dans votre navigateur, qui renvoie le résultat à un écouteur qu'Eidos ouvre sur `127.0.0.1` pour cette seule connexion. Eidos l'échange ensuite contre des jetons sur `users.nexusmods.com/oauth/token` et récupère les clés publiques avec lesquelles Nexus les signe. [Politique de confidentialité de Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Identifiants Nexus | stockés | Les jetons d'accès et de renouvellement, dans `~/.config/Colony/Eidos/nexus.ini` avec le mode 0600. Une ligne `api_key=` écrite là par un Eidos plus ancien est conservée mais jamais lue ; supprimez-la si vous le souhaitez. |
| Requêtes Nexus | envoyées | `api.nexusmods.com`, quand vous utilisez une fonction Nexus : le nom de votre compte et votre statut premium, les détails des mods et des fichiers, les vérifications de mises à jour, les collections et les liens de téléchargement. Vos approbations et vos mods suivis sont lus à chaque vérification de mises à jour et écrits quand vous appuyez sur Endorse ou Track. Les prérequis, descriptions et journaux des modifications des mods, ainsi que la recherche de collections, vont à `api.nexusmods.com/v2/graphql`. Chaque requête porte votre jeton d'accès et indique Eidos et sa version. |
| Téléchargements Nexus | envoyés | Les fichiers que vous demandez, depuis l'hôte CDN de Nexus que nomme le lien de téléchargement, enregistrés dans le dossier de téléchargements de l'instance avec un `.meta` au format MO2 à côté de chacun. |
| Liens `nxm://` | stockés | `eidos nxm --register`, que `install.sh` lance pour vous, écrit `~/.local/share/applications/eidos-nxm.desktop` et en fait le gestionnaire `nxm://` via `xdg-mime`, si bien que le bouton Mod Manager Download du site transmet son lien à Eidos. Le paquet Arch installe la même entrée pour tout le système. |
| Masterlists LOOT | envoyées | `raw.githubusercontent.com/loot/...`, au premier tri LOOT d'un jeu et chaque fois que vous demandez une mise à jour. Rien de ce qui vous concerne n'est envoyé. [Déclaration de confidentialité de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes des outils | envoyés | Seulement quand vous installez les prérequis d'un outil, et seulement pour ce que le paquet de version ne fournissait pas (le fichier unique `eidos-linux` et une compilation depuis les sources ne fournissent rien) : le runtime .NET 10 depuis `builds.dotnet.microsoft.com` et les installateurs Visual C++ et .NET 8 depuis `download.visualstudio.microsoft.com`, chacun refusé s'il ne correspond pas au SHA-256 intégré à Eidos ; le SDK .NET 10, toujours depuis `builds.dotnet.microsoft.com` et vérifié de la même façon ; et `winetricks` pour les autres prérequis (.NET Framework 4.8, XACT, .NET 6 et 7), qui télécharge ce qu'il installe depuis Microsoft et les autres hôtes qu'utilise winetricks. [Déclaration de confidentialité de Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Journaux de session | stockés | `~/.local/share/Colony/Eidos/logs/`, mode 0600 : dix sessions par instance, effacées au bout de 30 jours. Votre dossier personnel y est écrit `~` et votre nom d'utilisateur `<user>`, si bien qu'un journal peut être collé tel quel dans un ticket. Ils ne quittent jamais votre machine à moins que vous ne les envoyiez. |
| Fiches d'exécution | stockées | `<instance>/logs/sessions/` : pour chaque exécution de jeu ou d'outil, le profil, les mods activés avec leurs versions et leurs identifiants de fichier Nexus, l'ordre des plugins, une somme de contrôle de chaque INI du profil, les versions du jeu et d'Eidos, le code de sortie et les sauvegardes créées par l'exécution. Les 50 plus récentes sont gardées, `eidos sessions` les lit, et `eidos pack` les laisse de côté. Elles restent sur votre machine. |
| Rapports de mise à jour | stockés | `<instance>/logs/updates/<mod>.json` : ce que la dernière mise à jour ou le dernier Replace a changé dans le dossier de ce mod. `eidos pack` les laisse de côté. |
| Raccourcis | stockés | `install.sh` écrit `eidos.desktop` et ses icônes sous `~/.local/share` ; un raccourci d'outil ou de profil que vous créez va dans `~/.local/share/applications`. |
| Notifications de bureau | ni l'un ni l'autre | Désactivées par défaut. Activées, elles vont au service de notifications de votre bureau par D-Bus (`org.freedesktop.Notifications`) et nomment la tâche qui s'est terminée. |
| Liens que vous ouvrez | ni l'un ni l'autre | Les pages de mods et les autres liens s'ouvrent dans votre propre navigateur via `xdg-open`. |
| Extensions | ni l'un ni l'autre | Les extensions que vous installez s'exécutent comme des programmes distincts sur votre machine ; ce qu'elles font ne relève que d'elles. |

## Licence

Le code original d'Eidos est sous licence GPL-3.0-or-later ; voir
[LICENSE](../../../LICENSE). Les parties suivantes sont sous GPL-3.0 sans
l'option « or later » :

- les traductions de scripts OMOD dans `eidos-install` ; voir leurs
  [crédits d'origine](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md) ;
- l'utilitaire `eidos-nif-preview`, construit sur la bibliothèque nifly
  intégrée au dépôt ; voir les [licences et crédits de ses composants](../../../crates/eidos-nif-preview/UPSTREAM.md) ;
- les crates `esplugin` et `libloadorder` auxquelles le code de l'ordre de
  chargement est lié, qui déclarent `GPL-3.0`.

Les autres crates Rust présentes dans les binaires gardent leurs propres
licences (MIT, Apache-2.0, BSD et similaires). Le paquet de la version porte
leurs mentions dans `licenses/crates/`, avec un index de chaque crate et de sa
licence.

Les binaires embarquent aussi huit DLL redistribuables Microsoft DirectX
authentiques (`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` et `d3dx11_43`, en
32 et 64 bits), stockées compressées en gzip (7,9 Mo au lieu de 17,9 Mo), qu'Eidos
décompresse et déploie dans le préfixe du jeu quand un mod ou un outil en a
besoin. Elles restent sous les conditions de redistribution de Microsoft ; voir
leur [provenance](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

Le paquet de la version porte aussi, sous `runtimes/`, les installateurs de Visual
C++ 2015-2022, l'installateur du .NET 8 Desktop Runtime et l'archive zip du
runtime .NET 10, tels que Microsoft les publie. Leurs conditions sont dans
`licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
