<!-- eidos-i18n: source=README.md sha=6d693599c6ceb4b2af2e9a86c3994d86d7057dc5 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Le gestionnaire de mods natif Linux qui ne touche jamais à votre jeu.**

</div>

> 🇬🇧 L'anglais fait foi. En cas de désaccord entre cette page et
> [README.md](README.md), c'est l'anglais qui a raison.

Eidos apporte aux jeux Bethesda sous Linux ce que Mod Organizer 2 leur apporte
sous Windows - une vue fusionnée de vos mods, virtuelle et recréée à chaque
lancement - construite sur des primitives Linux plutôt que sur du détournement
d'API Windows. Pas de Wine pour le gestionnaire. Aucun fichier copié dans le
dossier du jeu. Aucune procédure de nettoyage, parce qu'il n'y a rien à nettoyer.

```
Steam ──> eidos-gui %command% ──> [ espace de noms privé ]
                                  │  mods ⊕ jeu  ──> ce que le jeu voit
                                  └─ meurt avec le jeu ; l'installation reste intacte
```

> **État :** Skyrim SE est joué quotidiennement à travers Eidos - SKSE,
> préchargeurs de script extender, Creation Club, ordres de chargement triés par
> LOOT, sauvegardes par profil, tout. Une famille de jeux éprouvée en jeu réel à
> ce jour ; dix autres sont câblées et attendent des testeurs.

## Pourquoi Eidos

- 🔒 **Un montage que seul votre jeu voit.** La vue fusionnée vit dans un espace
  de noms de montage privé : votre gestionnaire de fichiers, votre sauvegarde
  automatique, un second jeu - aucun ne le voit, aucun n'a besoin d'autorisation
  pour lui. Tuez le jeu, coupez le courant : l'espace de noms meurt avec l'arbre
  de processus et votre installation est exactement telle qu'elle était. Il n'y a
  aucun résidu *par construction*.
- 🧾 **Une seule copie de la vérité.** Votre profil possède sa liste de mods,
  l'ordre de ses plugins, ses INI et ses sauvegardes. Les fichiers de plugins et
  le dossier de sauvegardes sont montés par liaison sur les chemins du jeu au
  lancement, si bien que même les écritures du jeu atterrissent dans votre profil.
  Changer de profil change tout.
- 🐧 **Entièrement sans privilèges.** Pas d'assistant setuid, pas de démon, pas de
  `sudo setcap`, aucune modification de `/etc/fuse.conf`. Un binaire, une option
  de lancement Steam.
- 🛡️ **Des garde-fous qui montrent leurs preuves.** Un plantage qui abîme votre
  liste de plugins est signalé face à un instantané pris avant la session, avec
  une restauration en un clic. Une capture qui effacerait votre ordre de
  chargement est refusée, en disant pourquoi.

## Ce qu'il fait

**Mods.** Archives simples, assistants FOMOD, paquets BAIN de Wrye Bash, un
sélecteur manuel pour le reste - et **les mods root nativement** (préchargeurs de
script extender, ENB, Engine Fixes), sans greffon Root Builder et sans rien copier
dans votre installation. Masquez des fichiers isolés, regroupez avec des
séparateurs, déplacements ciblés, notes et catégories par mod, et un importateur
de profils MO2.

La liste est celle de MO2, avec ses habitudes : huit colonnes optionnelles et un
tri sur chacune, regroupement par catégorie ou par source, gestes au double-clic,
saisie pour sauter à un nom, sauvegardes par mod inertes tant que vous ne les
restaurez pas, et des drapeaux consultatifs pour un mod dont la disposition ne
sera pas chargée par ce jeu, ou qui a été téléchargé pour un autre. Son arbre de
fichiers fait les opérations ordinaires - nouveau dossier, renommer, supprimer,
ouvrir - et prévisualise images et textes sans rien lancer.

**Plugins.** L'ordre de chargement avec le tri LOOT intégré, les index de mods
tels que le jeu les calcule, les avertissements de masters manquants, et vos DLC
et contenus Creation Club affichés comme les lignes non gérées qu'ils sont.

**Instances.** Globales - gérées centralement sous `~/.local/share/eidos` - ou
portables : un dossier autonome où vous voulez (un second disque, une partition de
jeux), déplaçable et isolé, comme celles de MO2. Les instances portables sont
mémorisées d'une session à l'autre ; la GUI, le lancement Steam et chaque commande
suivent celle que vous avez utilisée en dernier, et toute commande accepte le
dossier partout où elle accepte un identifiant de jeu. Détails dans
[usage.md](docs/guide/usage.md) 🇬🇧.

**Profils.** Ordre des mods, état des plugins, INI et sauvegardes par profil. Les
sauvegardes sont analysées, comparées à vos plugins actuels - avec un bouton qui
active ce dont une sauvegarde a besoin - et resynchronisées pour Steam Cloud après
chaque session.

**Nexus.** Connectez un compte et le bouton « Mod Manager Download » du site
atterrit directement dans votre instance, avec vérification des mises à jour face
à ce que vous avez installé, l'auteur de chaque mod et un lien vers son profil. Un
lien de **collection** en liste les membres croisés avec votre instance -
installés, téléchargés, manquants - ce qui revient à lire une collection plutôt
qu'à l'installer, et le panneau dit pourquoi. L'onglet Téléchargements est une
bibliothèque d'archives : filtrer, trier, masquer sans supprimer, et purger celles
déjà installées. Un interrupteur **hors ligne** arrête tout cela.

**Outils.** xEdit, BodySlide, DynDOLOD et compagnie s'exécutent *à travers la vue
fusionnée* dans le préfixe Proton du jeu - ils voient vos mods, leur production
atterrit dans Overwrite, et un clic en fait un vrai mod. Le runtime dont chacun a
besoin est récupéré à la demande, si bien qu'une DLL manquante est un bouton
plutôt qu'un après-midi. xEdit et son jumeau QuickAutoClean sont trouvés pour vous
- dans le dossier du jeu, dans un mod, ou dans le dossier d'outils que vous gardez
à côté de vos jeux - avec les bons runtimes déjà choisis. Épinglez ceux dont vous
vous servez, masquez les autres, donnez à un outil son propre AppID Steam quand il
est sa propre application Steam, et écrivez un raccourci `.desktop` qui le lance à
travers la vue fusionnée sans ouvrir Eidos du tout.

**Diagnostics.** Masters manquants, archives orphelines, dérive de la liste de
mods, jeux de plugins abîmés - et, après une exécution, ce que le journal du
script extender dit avoir réellement chargé.

**Où il range ses propres fichiers.** `~/.config/Colony/Eidos/` pour ce que vous
avez choisi - préférences, votre session Nexus, votre liste d'instances, les
définitions de jeux et d'extensions que vous avez écrites - avec les journaux sous
`~/.local/state/Colony/Eidos/`. La disposition qu'utilise chaque programme de la
famille Colony. Un Eidos plus ancien rangeait cela dans `~/.config/eidos/` ; le
premier lancement après mise à jour les recopie, le dit dans le journal, et laisse
l'ancien dossier exactement tel qu'il était.

## Comment il se compare

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / déployeurs par liens |
|---|---|---|---|---|
| Gestionnaire natif | ✅ | ❌ appli Windows dans Wine | ✅ (portage Qt) | ✅ |
| Dossier du jeu intact | ✅ toujours | ✅ | ✅ | ❌ des liens y sont écrits |
| Montage visible par | le jeu seul | le jeu seul | **tout le système** | sans objet |
| Nettoyage après plantage | aucun, par conception | aucun | récupération de montage mort | dé-déploiement manuel |
| Mods root (ENB, préchargeurs) | ✅ natif | greffon requis | greffon requis | partiel |
| Privilèges requis | aucun | aucun | modifier `/etc/fuse.conf` | aucun |

## Sa rapidité

| | avant | maintenant |
|---|---|---|
| charger une sauvegarde | ~20 secondes | **6-7 secondes** |
| lectures de dossiers dans une session | 5,6 millions | 465 mille |

Les changements de cellule sont immédiats. Le gain vient d'avoir posé moins de
questions à vos mods : trouver un fichier interrogeait auparavant les cinquante
couches l'une après l'autre, et lister un dossier le faisait cinquante fois. Ni
l'un ni l'autre ne le fait plus. Mesuré sur une instance réelle jouée normalement,
pas sur un banc d'essai.

## Démarrer

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Mettez ensuite l'option de lancement Steam de votre jeu à
`~/.local/bin/eidos-gui %command%` et appuyez sur Jouer.

Paquets Arch, archives de version, ce qu'il faut installer d'abord et la voie en
ligne de commande : **[docs/guide/install.fr.md](docs/guide/install.fr.md)**.

## Options de lancement Steam

La ligne de base suffit à la plupart des configurations :

```
~/.local/bin/eidos-gui %command%
```

Tout le reste est constitué de variables d'environnement empilées devant, et elles
se combinent librement :

| Vous voulez... | Mettez devant |
|---|---|
| DLSS avec Community Shaders | `PROTON_ENABLE_NVAPI=1` - sans elle, DLSS ne s'initialise jamais, en silence ; la liste complète est dans [guide/graphics.md](docs/guide/graphics.md) |
| un compteur d'IPS à l'écran | `DXVK_HUD=fps` |
| l'interpolation d'images au niveau du pilote, sans mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - jamais en même temps que la génération d'images de Community Shaders |
| des journaux détaillés pour un rapport de bug | `EIDOS_LOG=debug` (les journaux de session vont dans `~/.local/state/Colony/Eidos/logs/`) |
| un rapport d'E/S par session depuis le montage | `EIDOS_FUSE_STATS=1` |
| un autre nombre de travailleurs FUSE | `EIDOS_FUSE_THREADS=8` (4 par défaut ; `1` est la première chose à essayer face à un bug de concurrence) |
| épingler ce lancement à une instance portable | `EIDOS_INSTANCE=/chemin/vers/dossier` - sans elle Eidos ouvre l'instance utilisée en dernier, ce qui est généralement ce que vous voulez |

La ligne à garder pour une installation moddée moderne (Community Shaders, DLSS,
génération d'images) - c'est la commande finale, pas un exemple :

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Ajoutez `DXVK_HUD=fps` devant le temps de vérifier que tout marche, puis retirez-le.

Les interrupteurs de diagnostic plus profonds (`EIDOS_FUSE_TRACE`, les bascules de
bissection du cache et de l'index, pourquoi `EIDOS_FUSE_PASSTHROUGH` est
désactivé par défaut) vivent dans
[guide/troubleshooting.fr.md](docs/guide/troubleshooting.fr.md).

## Où aller ensuite

| Si vous voulez... | |
|---|---|
| l'installer | [guide/install.fr.md](docs/guide/install.fr.md) |
| apprendre la ligne de commande et la GUI | [guide/usage.md](docs/guide/usage.md) 🇬🇧 |
| réparer quelque chose qui semble anormal | [guide/troubleshooting.fr.md](docs/guide/troubleshooting.fr.md) |
| configurer xEdit, BodySlide ou DynDOLOD | [guide/tools.md](docs/guide/tools.md) 🇬🇧 |
| jouer à Fallout 4 | [guide/fallout4.md](docs/guide/fallout4.md) 🇬🇧 |
| faire marcher DLSS et la génération d'images | [guide/graphics.md](docs/guide/graphics.md) 🇬🇧 |
| comprendre son fonctionnement interne | [internals/architecture.md](docs/internals/architecture.md) 🇬🇧 |
| le compiler, le tester, y contribuer | [internals/contributing.md](docs/internals/contributing.md) 🇬🇧 |

Les pages marquées 🇬🇧 ne sont pas traduites, volontairement : elles s'adressent à
des gens qui lisent aussi le Rust. L'index complet est dans
[docs/README.md](docs/README.md).

## Jeux pris en charge

**Skyrim SE/AE** - éprouvé en jeu réel. **Fallout 4** est câblé de bout en bout
également (F4SE substitué automatiquement, invalidation d'archives, ordre de
chargement à astérisques, LOOT, sauvegardes `.fos`) - voir
[guide/fallout4.md](docs/guide/fallout4.md). Câblés selon le descripteur de jeu
partagé et en attente de testeurs : Skyrim LE, Skyrim VR, Enderal SE, Fallout 3,
Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion et Morrowind (ces deux derniers
se montent et gèrent les mods ; leurs listes de plugins ordonnées par horodatage
ne sont pas encore gérées).

Ajouter une famille tient en une ligne de descripteur :
[internals/adding-games.md](docs/internals/adding-games.md).

## Travaux antérieurs et remerciements

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) et
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la sémantique qu'Eidos
  reproduit, et la base de code face à laquelle sa parité a été étudiée
- [LOOT](https://loot.github.io/) - le moteur de tri, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) et les autres gestionnaires Linux - la
  preuve qu'une communauté veut voir ce problème résolu

## Licence

GPL-3.0-or-later. La gestion de mods appartient à tout le monde.
