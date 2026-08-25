<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
ligne de commande : **[docs/guide/install.fr.md](docs/guide/install.md)**.

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
[guide/troubleshooting.fr.md](docs/guide/troubleshooting.md).

## Où aller ensuite

| Si vous voulez... | |
|---|---|
| l'installer | [guide/install.fr.md](docs/guide/install.md) |
| apprendre la ligne de commande et la GUI | [guide/usage.fr.md](docs/guide/usage.md) |
| configurer xEdit, BodySlide ou DynDOLOD | [guide/tools.fr.md](docs/guide/tools.md) |
| jouer à Fallout 4 (F4SE, versions, le plantage des débris NVIDIA) | [guide/fallout4.fr.md](docs/guide/fallout4.md) |
| faire marcher DLSS et la génération d'images (Community Shaders) | [guide/graphics.fr.md](docs/guide/graphics.md) |
| réparer quelque chose qui semble anormal | [guide/troubleshooting.fr.md](docs/guide/troubleshooting.md) |
| savoir pourquoi c'est rapide, et le vérifier vous-même | [internals/performance.md](../../internals/performance.md) |
| comprendre son fonctionnement interne | [internals/architecture.md](../../internals/architecture.md) |
| le compiler, le tester, y contribuer | [internals/contributing.md](../../internals/contributing.md) |
| savoir pourquoi il existe | [project/landscape.md](../../project/landscape.md) |

Une langue tient dans un seul dossier : `docs/i18n/fr/` reproduit l'arborescence
de la racine du dépôt, ce qui rend un lien entre deux pages traduites identique au
lien entre leurs originaux anglais.

## Langue

Les pages dont un joueur a besoin sont traduites. **L'anglais fait foi** : quand une
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

**Tout le reste est en anglais volontairement, pas par omission.** `docs/internals/`
et `docs/project/` sont lus par des gens qui lisent aussi le Rust, et `CHANGELOG.md`
est généré. Les traduire ferait 17 678 mots de plus à tenir honnêtes pour un public
qui n'en a pas besoin.

Chaque traduction porte l'empreinte du fichier anglais dont elle est issue, et la CI
échoue quand l'anglais avance - voir [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
Une traduction qu'on ne peut pas remettre à jour est **supprimée**, pas laissée en
place : une page périmée garde l'air officiel et distribue les commandes du mois
dernier, ce qui est pire pour le lecteur que d'être renvoyé vers l'anglais.

Ajouter une langue, c'est neuf fichiers et une ligne dans cette liste ;
[`docs/internals/contributing.md`](../../internals/contributing.md) donne les étapes.

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
[internals/adding-games.md](../../internals/adding-games.md).

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
