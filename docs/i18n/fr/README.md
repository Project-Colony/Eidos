<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**Le gestionnaire de mods natif Linux qui ne touche jamais à votre jeu.**

</div>

> 🇬🇧 L'anglais fait foi. En cas de désaccord entre cette page et
> [README.md](../../../README.md), c'est l'anglais qui a raison.

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

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

### Comment il se compare

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / déployeurs par liens |
|---|---|---|---|---|
| Gestionnaire natif | ✅ | ❌ appli Windows dans Wine | ✅ (portage Qt) | ✅ |
| Dossier du jeu intact | ✅ toujours | ✅ | ✅ | ❌ des liens y sont écrits |
| Montage visible par | le jeu seul | le jeu seul | **tout le système** | sans objet |
| Nettoyage après plantage | aucun, par conception | aucun | récupération de montage mort | dé-déploiement manuel |
| Mods root (ENB, préchargeurs) | ✅ natif | greffon requis | greffon requis | partiel |
| Privilèges requis | aucun | aucun | modifier `/etc/fuse.conf` | aucun |

### Sa rapidité

| | avant | maintenant |
|---|---|---|
| charger une sauvegarde | ~20 secondes | **6-7 secondes** |
| lectures de dossiers dans une session | 5,6 millions | 465 mille |

Les changements de cellule sont immédiats. Le gain vient d'avoir posé moins de
questions à vos mods : trouver un fichier interrogeait auparavant les cinquante
couches l'une après l'autre, et lister un dossier le faisait cinquante fois. Ni
l'un ni l'autre ne le fait plus. Mesuré sur une instance réelle jouée normalement,
pas sur un banc d'essai.

### Travaux antérieurs et remerciements

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) et
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la sémantique qu'Eidos
  reproduit, et la base de code face à laquelle sa parité a été étudiée
- [LOOT](https://loot.github.io/) - le moteur de tri, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) et les autres gestionnaires Linux - la
  preuve qu'une communauté veut voir ce problème résolu

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

**Instances.** Globales - gérées centralement sous `~/.local/share/Colony/Eidos/instances/` - ou
portables : un dossier autonome où vous voulez (un second disque, une partition de
jeux), déplaçable et isolé, comme celles de MO2. Les instances portables sont
mémorisées d'une session à l'autre ; la GUI, le lancement Steam et chaque commande
suivent celle que vous avez utilisée en dernier, et toute commande accepte le
dossier partout où elle accepte un identifiant de jeu. Détails dans
[usage.md](docs/guide/usage.md).

**Profils.** Ordre des mods, état des plugins, INI et sauvegardes par profil. Les
sauvegardes sont analysées, comparées à vos plugins actuels - avec un bouton qui
active ce dont une sauvegarde a besoin - et resynchronisées pour Steam Cloud après
chaque session.

**Nexus.** Connectez un compte pour recevoir les liens « Mod Manager Download », vérifier les mises à jour et consulter les auteurs et leurs profils. Les collections s'installent à partir de leurs recettes, avec une progression reprenable et des rapports explicites sur les étapes non prises en charge. La correspondance vérifie les fichiers source exacts ; les anciennes métadonnées incomplètes restent non vérifiées. L'onglet Téléchargements permet de filtrer, trier, masquer et supprimer les archives. Un interrupteur **hors ligne** bloque l'accès à Nexus.

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

**Où il range ses propres fichiers.** La disposition qu'utilise chaque programme
de la famille Colony : `~/.config/Colony/Eidos/` pour ce que vous avez choisi -
préférences, votre session Nexus, votre liste d'instances, les définitions de
jeux et d'extensions que vous avez écrites - `~/.local/share/Colony/Eidos/` pour
les instances globales, les runtimes téléchargés et les journaux de session, et
`~/.cache/Colony/Eidos/` pour les masterlists LOOT. Un Eidos plus ancien
rangeait cela dans `~/.config/eidos/`, `~/.local/share/eidos/` et
`~/.local/state/` ; le premier lancement après mise à jour les transfère, le dit
dans le journal, et laisse les anciens dossiers exactement tels qu'ils étaient.
Tant qu'un transfert n'est pas terminé, Eidos continue d'utiliser les anciens
fichiers. Détails dans [internals/paths.md](../../internals/paths.md) (en
anglais).

Consultez [Contrôles d'intégrité et fichiers générés](../../../docs/internals/integrity.md) (en anglais) pour la protection des remplacements, les conflits entre fichiers BSA/BA2, les diagnostics statiques SKSE et les traces de génération, ainsi que leurs limites.

### Jeux pris en charge

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

## Installation

### Via Colony (recommandé)

Cherchez **Eidos** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le. Les mises à jour arrivent par le lanceur. Colony installe un seul
fichier, `eidos-linux` : la fenêtre, et aussi la ligne de commande `eidos`
quand il est appelé à travers un lien nommé `eidos`. L'aperçu des modèles NIF a
besoin d'un utilitaire que seul le paquet ci-dessous contient ; sans lui,
l'aperçu le signale et tout le reste fonctionne. Que faire après l'installation,
et comment y faire pointer Steam :
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Téléchargement direct du binaire

Prenez un fichier dans la [dernière version](https://github.com/Project-Colony/Eidos/releases/latest) :

| Fichier | Ce que c'est |
|---|---|
| `eidos-linux` | tout le programme en un seul fichier, tel que Colony l'installe |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, l'utilitaire d'aperçu NIF, les icônes et un installateur qui enregistre l'entrée de bureau et le gestionnaire de téléchargements Nexus |

Vérifiez d'abord le téléchargement, comme le montre la
[politique de signature du code](#politique-de-signature-du-code), puis :

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

L'installateur met tout dans `~/.local/bin` sans sudo ; `--system` et
`--bindir DIR` choisissent un autre emplacement. Les deux fichiers exigent
Linux x86_64 avec glibc 2.39 ou plus récent, FUSE (`fusermount3`) et 7-Zip pour
les archives de mods.

### Compiler depuis les sources

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Exige Rust 1.89 ou plus récent (la `rust-version` que vérifie la CI ; la CI et
les versions publiées compilent avec 1.99.0), ainsi que `just`, CMake 3.20+ et
un compilateur C++17 pour l'utilitaire d'aperçu NIF. Un paquet Arch se compile
depuis `packaging/PKGBUILD`. Prérequis et commandes sans `just` :
[docs/guide/install.md](docs/guide/install.md).

### Options de lancement Steam

Eidos s'exécute *en tant que* commande de lancement du jeu, et c'est ainsi qu'il
monte avant que le jeu démarre. La ligne de base suffit à la plupart des
configurations, avec le chemin absolu de votre installation (Steam ne lit pas
votre `PATH`) :

```
~/.local/bin/eidos-gui %command%
```

Depuis Colony, ce chemin est `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Appuyez ensuite sur Jouer : Eidos s'ouvre sur l'instance de ce jeu, et Run
démarre le jeu à travers la vue fusionnée.

Tout le reste est constitué de variables d'environnement empilées devant, et elles
se combinent librement :

| Vous voulez... | Mettez devant |
|---|---|
| DLSS avec Community Shaders | `PROTON_ENABLE_NVAPI=1` - sans elle, DLSS ne s'initialise jamais, en silence ; la liste complète est dans [guide/graphics.md](docs/guide/graphics.md) |
| un compteur d'IPS à l'écran | `DXVK_HUD=fps` |
| l'interpolation d'images au niveau du pilote, sans mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - jamais en même temps que la génération d'images de Community Shaders |
| des journaux détaillés pour un rapport de bug | `EIDOS_LOG=debug` (les journaux de session vont dans `~/.local/share/Colony/Eidos/logs/`) |
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
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentation

Tout est dans [docs/](../../), indexé selon le lecteur dans
[docs/README.md](docs/README.md) : le guide de l'utilisateur (installation, la
ligne de commande et la GUI, des outils comme xEdit et DynDOLOD, le graphisme,
le dépannage), le fonctionnement interne d'Eidos, et pourquoi il existe.
Politique de sécurité et comment signaler une vulnérabilité :
[SECURITY.md](../../../SECURITY.md) (en anglais).

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

Une langue tient dans un seul dossier. `docs/i18n/<lang>/` reproduit
l'arborescence de la racine du dépôt, si bien que
`docs/i18n/de/docs/guide/install.md` est le `docs/guide/install.md` allemand -
c'est ce qui rend un lien entre deux pages traduites identique au lien entre
leurs originaux anglais, et ce qui permet de retirer une langue avec un seul
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
plantage. Il ne se connecte à aucun serveur en dehors de ceux de ce tableau, et
seulement pour l'action qui y est indiquée. Le mode **hors ligne**
(Preferences -> Nexus) coupe toute connexion à Nexus.

| Donnée | Stockée ou envoyée | Où, et pourquoi |
|---|---|---|
| Préférences, liste d'instances, définitions de jeux et d'extensions | stockées | `~/.config/Colony/Eidos/` |
| Instances, runtimes téléchargés, journaux de session | stockés | `~/.local/share/Colony/Eidos/`, ou le dossier choisi pour une instance portable |
| Masterlists LOOT | stockées | `~/.cache/Colony/Eidos/loot/` |
| Connexion à Nexus | envoyée | Se connecter ouvre `users.nexusmods.com` dans votre navigateur, qui renvoie le résultat à un écouteur qu'Eidos ouvre sur `127.0.0.1` pour cette seule connexion. Eidos l'échange ensuite contre des jetons sur `users.nexusmods.com/oauth/token` et récupère les clés publiques avec lesquelles Nexus les signe. [Politique de confidentialité de Nexus Mods](https://help.nexusmods.com/article/20-privacy-policy) |
| Identifiants Nexus | stockés | Les jetons d'accès et de renouvellement, dans `~/.config/Colony/Eidos/nexus.ini` avec le mode 0600. Une ligne `api_key=` écrite là par un Eidos plus ancien est conservée mais jamais lue ; supprimez-la si vous le souhaitez. |
| Requêtes Nexus | envoyées | `api.nexusmods.com`, quand vous utilisez une fonction Nexus : le nom de votre compte et votre statut premium, les détails des mods et des fichiers, les vérifications de mises à jour, les collections et les liens de téléchargement. Chaque requête porte votre jeton d'accès et indique Eidos et sa version. |
| Téléchargements Nexus | envoyés | Les fichiers que vous demandez, depuis l'hôte CDN de Nexus que nomme le lien de téléchargement, enregistrés dans le dossier de téléchargements de l'instance avec un `.meta` au format MO2 à côté de chacun. |
| Liens `nxm://` | stockés | `eidos nxm --register`, que `install.sh` lance pour vous, écrit `~/.local/share/applications/eidos-nxm.desktop` et en fait le gestionnaire `nxm://` via `xdg-mime`, si bien que le bouton Mod Manager Download du site transmet son lien à Eidos. Le paquet Arch installe la même entrée pour tout le système. |
| Masterlists LOOT | envoyées | `raw.githubusercontent.com/loot/...`, au premier tri LOOT d'un jeu et chaque fois que vous demandez une mise à jour. Rien de ce qui vous concerne n'est envoyé. [Déclaration de confidentialité de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Runtimes des outils | envoyés | Seulement quand vous installez un prérequis d'outil ou qu'un mod en a besoin : le runtime .NET depuis `builds.dotnet.microsoft.com`, refusé s'il ne correspond pas au SHA-256 intégré à Eidos, et `winetricks`, qui télécharge ce qu'il installe dans le préfixe Proton du jeu (redistribuables Visual C++ et .NET) depuis Microsoft et les autres hôtes qu'utilise winetricks. [Déclaration de confidentialité de Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Journaux de session | stockés | `~/.local/share/Colony/Eidos/logs/`, mode 0600 : dix sessions par instance, effacées au bout de 30 jours. Votre dossier personnel y est écrit `~` et votre nom d'utilisateur `<user>`, si bien qu'un journal peut être collé tel quel dans un ticket. Ils ne quittent jamais votre machine à moins que vous ne les envoyiez. |
| Raccourcis | stockés | `install.sh` écrit `eidos.desktop` et ses icônes sous `~/.local/share` ; un raccourci d'outil que vous créez va dans `~/.local/share/applications`. |
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
32 et 64 bits), qu'Eidos déploie dans le préfixe du jeu quand un mod ou un outil
en a besoin. Elles restent sous les conditions de redistribution de Microsoft ;
voir leur [provenance](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md).

La gestion de mods appartient à tout le monde.
