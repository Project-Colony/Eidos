<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# Installer Eidos

Quatre portes d'entrée. Colony installe un seul fichier, `eidos-linux`, qui est le
programme entier. Le paquet de version, le paquet Arch et une compilation depuis
les sources livrent côte à côte `eidos` (la ligne de commande), `eidos-gui` et
l'utilitaire d'aperçu NIF statique `eidos-nif-preview`. Le paquet Arch et une
compilation depuis les sources installent aussi `eidos-fuse` et `eidos-launch`, les
deux CLI de plus bas niveau. Chaque voie peut enregistrer le gestionnaire `nxm://`
qui fait atterrir le bouton « Mod Manager Download » de Nexus dans votre instance.

## Ce qu'il vous faut d'abord

| Exigence | Pourquoi |
|---|---|
| Linux avec FUSE | `fusermount3` dans votre PATH. Toutes les distributions actuelles le fournissent. |
| Un jeu Proton, lancé une fois | Steam ne crée le préfixe Wine du jeu qu'au premier lancement, et Eidos travaille dedans. Eidos détecte aussi les copies GOG et Epic de Skyrim Special Edition, les copies Epic de Fallout 3 et de Fallout: New Vegas, et la copie GOG de Stardew Valley. Celles-ci passent par Heroic ou Legendary avec Eidos comme wrapper du jeu : voir [usage.md](usage.md#lancer-le-jeu). |
| Un binaire 7-Zip : `7z`, `7zz` ou `7za` | Pour installer les archives de mods. `7zip` sur Arch, `p7zip` sur la plupart des autres distributions. |
| `par2` (facultatif) | Données de récupération PAR2 pour les sauvegardes `eidos pack`. `par2cmdline` sur Arch et Fedora, `par2` sur Debian et Ubuntu. |

Pas de root, pas de démon, aucune modification de `/etc/fuse.conf`, et rien à
ajouter à vos groupes. Eidos monte dans un espace de noms privé qui appartient au
processus du jeu.

Certains réglages système ne se manifestent que par des plantages. L'onglet
Diagnostics vérifie `vm.max_map_count` (1048576 ou plus), la limite stricte de
fichiers ouverts (524288 ou plus) et si le jeu, son préfixe ou l'instance se trouve
sur NTFS, exFAT ou FAT ; [troubleshooting.md](troubleshooting.md#le-système) donne
le correctif de chacun.

## Via Colony

Cherchez **Eidos** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le ; les mises à jour arrivent par le lanceur. Colony garde un seul
fichier, `~/.local/share/Colony/apps/Eidos/eidos-linux`. Lancez-le et la fenêtre
s'ouvre. Appelé à travers un lien nommé `eidos`, le même fichier est la ligne de
commande. Placez-en un dans votre `PATH` et enregistrez le gestionnaire de
téléchargements Nexus :

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

La fenêtre n'a besoin d'aucun lien : chaque fois qu'elle lance un jeu ou un outil,
elle exécute son propre fichier comme ligne de commande. L'aperçu des modèles NIF a
besoin de `eidos-nif-preview`, que seuls le paquet de version, le paquet Arch et
les compilations depuis les sources contiennent ; sans lui, l'aperçu le signale, et
rien d'autre n'est affecté. `eidos-linux` ne contient pas non plus les runtimes
Microsoft que livre le paquet ([plus bas](#les-runtimes-microsoft-du-paquet)) :
quand un outil en a besoin, Eidos le télécharge chez Microsoft dès que vous lancez
l'installation des prérequis. Faites pointer Steam sur le fichier lui-même, comme
ci-dessous.

## Un paquet de version

Téléchargez `eidos-linux-bundle.tar.gz` et ses `.sig`, `.meta` et `.meta.sig`
depuis la [dernière version](https://github.com/Project-Colony/Eidos/releases/latest),
et vérifiez-les comme le montre la
[politique de signature du code](../../README.md#politique-de-signature-du-code) du README. Le
paquet contient `eidos`, `eidos-gui` et `eidos-nif-preview` à sa racine, avec
`install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, les icônes dans
`assets/brand/png`, les mentions de tiers dans `licenses/` (nifly, omod,
microsoft-directx, microsoft-runtimes et les crates Rust) et le dossier `runtimes/`
décrit plus bas. Il ne contient ni `eidos-fuse` ni `eidos-launch`. Puis :

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` copie les binaires dans `~/.local/bin`, ou dans `$XDG_BIN_HOME` quand
il est défini. `--system` les place dans `/usr/local/bin` via sudo, `--bindir DIR`
ailleurs, et `--from DIR` prend les binaires dans un autre dossier. Le script fait
aussi ceci :

- il enregistre le gestionnaire `nxm://` en lançant `eidos nxm --register`, donc
  l'étape d'enregistrement de la section Colony n'est pas nécessaire ;
- il écrit `~/.local/share/applications/eidos.desktop` et les icônes d'Eidos sous
  `~/.local/share/icons/hicolor` ;
- il copie les mentions de licence dans `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` avec `--system`) ;
- il copie `runtimes/` vers `~/.local/share/Colony/Eidos/runtimes`, y compris avec
  `--system`, si bien que les runtimes appartiennent à l'utilisateur qui lance le
  script ;
- il termine en affichant l'option de lancement Steam avec le chemin absolu de
  l'`eidos-gui` qu'il a installé.

`--cap` demande à sudo d'accorder `CAP_SYS_ADMIN` à l'`eidos` installé (voir le
passthrough FUSE plus bas). `--no-cap` est la valeur par défaut et laisse tout sans
privilèges. Relancer le script est la façon prévue de mettre à jour. Le paquet et
`eidos-linux` exigent Linux x86_64 avec glibc 2.39 ou plus récent.

### Les runtimes Microsoft du paquet

Le dossier `runtimes/` du paquet contient les fichiers Microsoft que les outils
demandent : `VC_redist.x86.exe` et `VC_redist.x64.exe` (Visual C++ 2015-2022, le
prérequis `vcrun2022`), `windowsdesktop-runtime-8.0.12-win-x64.exe` (le .NET 8
Desktop Runtime, pour `dotnet8` et `dotnetdesktop8`) et
`dotnet-runtime-10.0.10-win-x64.zip` (le runtime .NET 10, pour `dotnet10`). Eidos
exécute lui-même les installateurs de Visual C++ et du .NET 8 dans le préfixe du
jeu, sans winetricks, et décompresse une seule fois le zip du .NET 10 dans
`~/.local/share/Colony/Eidos/runtimes`, où toutes les instances le partagent (78 Mo
décompressé). Chaque fichier n'est utilisé que si son SHA-256 correspond à celui
épinglé dans Eidos. Le SDK .NET 10 dont Synthesis a besoin (`dotnetsdk10`, environ
300 Mo) n'est pas dans le paquet et est toujours téléchargé.

Un paquet décompressé fonctionne aussi sur place : Eidos cherche `runtimes/` à côté
de son propre binaire, après le dossier de données ci-dessus.

## Arch

```bash
cd packaging && makepkg -si
```

Le PKGBUILD compile la branche `main` actuelle depuis les sources. Il installe
`eidos`, `eidos-gui`, `eidos-nif-preview`, `eidos-fuse` et `eidos-launch` dans
`/usr/bin`, avec un gestionnaire `nxm://` pour tout le système, une entrée de
bureau et les icônes. Il dépend de `fuse3`, `7zip` et `xdg-utils`. Il ne livre pas
de dossier `runtimes/`, donc les runtimes Microsoft sont téléchargés quand un outil
en a besoin. `winetricks` est facultatif : Eidos s'en sert toujours pour les
prérequis qu'il n'installe pas lui-même, comme `dotnet48` et `xact`.

## Depuis les sources

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

La compilation depuis les sources exige Rust 1.92 ou plus récent, `just`, CMake
3.20+, un compilateur C++17 et Python 3 pour les tests de l'utilitaire natif. `just
build` compile tout l'espace de travail, donc `eidos-fuse` et `eidos-launch` sont
compilés aussi, et `just install` lance `packaging/install.sh` sur le résultat, qui
les copie avec le reste. Pour les commandes sans `just`, consultez les
[instructions de l'utilitaire natif](../../../../../crates/eidos-nif-preview/README.md),
compilez Rust avec `cargo build --release --locked`, copiez l'utilitaire dans
`target/release`, puis lancez `packaging/install.sh --from target/release`. Une
compilation depuis les sources n'a pas non plus de dossier `runtimes/`, donc les
runtimes Microsoft sont téléchargés quand un outil en a besoin.

Le profil release utilise une seule unité de génération de code, une LTO complète et
des symboles retirés. Avec rustc 1.99.1 et 3 tâches de compilation, une compilation
à froid de `eidos` et `eidos-gui` a pris 334 s, contre 243 s avec une LTO légère, et
a donné un `eidos` de 20,9 Mo et un `eidos-gui` de 49,7 Mo. Chaque binaire embarque
huit DLL DirectX stockées compressées en gzip : 7,9 Mo au lieu de 17,9 Mo. `just
profile=dev build` compile sans optimisation.

## Ensuite : pointer Steam dessus

Eidos s'exécute *en tant que* commande de lancement de votre jeu, ce qui lui permet
de monter la vue avant que le jeu ne démarre. Dans Steam, clic droit sur le jeu ->
Propriétés -> Options de lancement, et saisissez le chemin absolu de la fenêtre
d'Eidos suivi de `%command%` :

```
~/.local/bin/eidos-gui %command%
```

Utilisez le chemin absolu - Steam ne lit pas le `PATH` de votre shell. Le chemin
dépend de la façon dont vous avez installé Eidos, et `install.sh` affiche la ligne
pour le dossier qu'il a utilisé :

| Installation | Option de lancement |
|---|---|
| Paquet de version ou `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Paquet Arch | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Appuyez sur Jouer. Eidos s'ouvre sur l'instance de ce jeu ; installez des mods,
triez avec LOOT, cliquez sur Lancer. En quittant, le montage disparaît avec lui et
votre installation est exactement dans l'état où vous l'aviez laissée.

Le jeu démarre toujours par sa boutique. Dans une fenêtre que vous avez ouverte
vous-même, Lancer exécute `steam -applaunch <appid>`, Steam démarre Eidos par
l'option de lancement, et une seconde fenêtre d'Eidos s'ouvre où vous appuyez sur
Lancer. Tant que l'option de lancement ne démarre pas Eidos, Lancer ne démarre rien
et la ligne d'état montre la ligne à définir. Avec Lancer le jeu directement quand
une boutique ouvre Eidos (Préférences, Général, Pendant le jeu ; désactivé par
défaut), Jouer dans Steam démarre le jeu sans la fenêtre quand le diagnostic ne
signale aucun problème. Heroic et
Legendary fonctionnent de la même façon par leur réglage de wrapper ;
[usage.md](usage.md#lancer-le-jeu) donne les détails.

L'onglet Diagnostics vérifie la ligne. Une carte de diagnostic montre la ligne à définir
quand l'option de lancement est vide, ne nomme aucun binaire d'Eidos, pointe vers un
fichier qui n'existe pas, n'a pas de `%command%` après Eidos, ou démarre une autre
copie d'Eidos que celle qui tourne.

### Si vous préférez le terminal

```sh
eidos init skyrimse               # créer une instance (ajoutez un dossier pour la rendre portable)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # trier l'ordre de chargement avec LOOT
eidos check skyrimse              # les vérifications du diagnostic qui n'ont pas besoin de fenêtre
eidos play skyrimse -- %command%  # exécuter n'importe quoi à travers la vue fusionnée
eidos shell skyrimse              # un shell dans la vue fusionnée, les écritures vont dans Overwrite
```

Chaque commande qui accepte un identifiant de jeu accepte aussi le dossier d'une
instance portable - voir [usage.md](usage.md#instances--globale-et-portable). La
visite complète est dans [usage.md](usage.md).

## Mises à jour

Colony met à jour `eidos-linux` par son lanceur. Pour le paquet de version, lancez
le `install.sh` du nouveau paquet ; pour le paquet Arch, relancez `makepkg -si` ;
pour une compilation depuis les sources, faites un pull et lancez `just install`. La
fenêtre demande à GitHub la dernière version au démarrage, au plus une fois par jour,
et quand une plus récente existe elle affiche un lien vers ses notes de version après
Aide dans la barre de menus. Elle ne télécharge ni ne remplace jamais rien
elle-même. Préférences, À propos, Version désactive la vérification, le mode hors ligne
l'ignore, et une copie installée sous `/usr` (le paquet Arch) ne la demande jamais,
parce que le gestionnaire de paquets la met à jour.

## Facultatif : le passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` accorde la capacité
facultative ; cette commande n'active pas le passthrough. `./install.sh --cap` et
`just setcap` l'accordent à l'`eidos` qu'ils installent ou compilent. Sur un montage
`nosuid`, le noyau ignore les capacités de fichier : `install.sh` saute alors
l'octroi avec un avertissement et `just setcap` s'arrête avec une erreur. La
capacité va sur le binaire `eidos` séparé du paquet de version, du paquet Arch ou
d'une compilation depuis les sources, jamais sur `eidos-linux`, qui est aussi la
fenêtre ([SECURITY.md](../../../../../SECURITY.md), en anglais). Un binaire recompilé
ou réinstallé est un nouveau fichier sans la capacité : accordez-la de nouveau après
chaque mise à jour si vous l'utilisez.

`EIDOS_FUSE_PASSTHROUGH=1` est le réglage d'activation distinct à l'exécution, et il
est désactivé par défaut. Passthrough activé, une version antérieure du démon
empêchait Skyrim SE de charger le moindre contenu de mod. La cause la plus probable
est qu'elle enregistrait un nouveau fichier sous-jacent à chaque ouverture, alors
que le noyau n'en accepte qu'un par inode. Le démon en partage désormais un seul
entre toutes les ouvertures en passthrough d'un fichier, mais cela n'a pas encore
été remesuré en jeu. D'ici là, n'activez le passthrough que pour le tester. Les
mesures et la cause sont dans [troubleshooting.md](troubleshooting.md).

## Quelque chose ne va déjà pas ?

[troubleshooting.md](troubleshooting.md) couvre les interrupteurs d'environnement,
la lecture des compteurs d'opérations, les cartes de diagnostic, et tous les problèmes
qui
ont mordu quelqu'un jusqu'ici.
