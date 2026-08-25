<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

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
| apprendre la ligne de commande et la GUI | [guide/usage.fr.md](docs/guide/usage.fr.md) |
| configurer xEdit, BodySlide ou DynDOLOD | [guide/tools.fr.md](docs/guide/tools.fr.md) |
| jouer à Fallout 4 (F4SE, versions, le plantage des débris NVIDIA) | [guide/fallout4.fr.md](docs/guide/fallout4.fr.md) |
| faire marcher DLSS et la génération d'images (Community Shaders) | [guide/graphics.fr.md](docs/guide/graphics.fr.md) |
| réparer quelque chose qui semble anormal | [guide/troubleshooting.fr.md](docs/guide/troubleshooting.fr.md) |
| savoir pourquoi c'est rapide, et le vérifier vous-même | [internals/performance.md](docs/internals/performance.md) |
| comprendre son fonctionnement interne | [internals/architecture.md](docs/internals/architecture.md) |
| le compiler, le tester, y contribuer | [internals/contributing.md](docs/internals/contributing.md) |
| savoir pourquoi il existe | [project/landscape.md](docs/project/landscape.md) |

L'index complet est dans [docs/README.fr.md](docs/README.fr.md) ; la politique de
sécurité et comment signaler une vulnérabilité dans [SECURITY.md](SECURITY.md).

## Langue

Les pages dont un joueur a besoin sont traduites. **L'anglais fait foi** : quand une
traduction le contredit, c'est le fichier anglais qui a raison.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)

**Tout le reste est en anglais volontairement, pas par omission.** `docs/internals/`
et `docs/project/` sont lus par des gens qui lisent aussi le Rust, et `CHANGELOG.md`
est généré. Les traduire ferait 17 678 mots de plus à tenir honnêtes pour un public
qui n'en a pas besoin.

Chaque traduction porte l'empreinte du fichier anglais dont elle est issue, et la CI
échoue quand l'anglais avance - voir [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Une traduction qu'on ne peut pas remettre à jour est **supprimée**, pas laissée en
place : une page périmée garde l'air officiel et distribue les commandes du mois
dernier, ce qui est pire pour le lecteur que d'être renvoyé vers l'anglais.

Ajouter une langue, c'est neuf fichiers et une ligne dans cette liste ;
[`docs/internals/contributing.md`](docs/internals/contributing.md) donne les étapes.

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
