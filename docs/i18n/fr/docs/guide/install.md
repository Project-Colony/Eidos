<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# Installer Eidos

Quatre portes d'entrée. Colony installe un seul fichier, `eidos-linux`, qui est
le programme entier ; les autres livrent côte à côte `eidos` (la ligne de
commande), `eidos-gui` et l'utilitaire d'aperçu statique NIF, ainsi que le gestionnaire `nxm://` qui fait atterrir le bouton « Mod Manager Download » de Nexus dans votre instance.

## Ce qu'il vous faut d'abord

| | |
|---|---|
| **Linux avec FUSE** | `fusermount3` dans votre PATH. Toutes les distributions actuelles le fournissent. |
| **Un jeu Proton, lancé une fois** | Steam ne crée le préfixe Wine du jeu qu'au premier lancement, et Eidos travaille dedans. |
| **`7z`** | Pour installer les archives de mods. `p7zip` sur la plupart des distributions. |

Pas de root, pas de démon, aucune modification de `/etc/fuse.conf`, et rien à
ajouter à vos groupes. Eidos monte dans un espace de noms privé qui appartient au
processus du jeu.

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

La fenêtre n'a besoin d'aucun lien : chaque fois qu'elle lance un jeu ou un
outil, elle exécute son propre fichier comme ligne de commande. L'aperçu des
modèles NIF a besoin de `eidos-nif-preview`, que seuls le paquet de version et
les compilations depuis les sources contiennent ; sans lui, l'aperçu le signale,
et rien d'autre n'est affecté. Faites pointer Steam sur le fichier lui-même,
comme ci-dessous.

## Un paquet de version

Téléchargez `eidos-linux-bundle.tar.gz` et ses `.sig`, `.meta` et `.meta.sig`
depuis la [dernière version](https://github.com/Project-Colony/Eidos/releases/latest),
et vérifiez-les comme le montre la
[politique de signature du code](../../README.md#politique-de-signature-du-code)
du README. Puis :

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Installe dans `~/.local/bin` par défaut. `--system` place les binaires dans
`/usr/local/bin`, `--bindir DIR` ailleurs. Relancer ce script est la façon
prévue de mettre à jour.
Le paquet et `eidos-linux` exigent Linux x86_64 avec glibc 2.39 ou plus récent.

## Arch

```bash
cd packaging && makepkg -si
```

Le PKGBUILD compile la branche `main` actuelle depuis les sources.

## Depuis les sources

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

La compilation depuis les sources nécessite Rust 1.89 ou plus récent, `just`, CMake 3.20+, un compilateur C++17 et Python 3 pour les tests de l'utilitaire natif. Pour les commandes sans `just`, consultez les [instructions de l'utilitaire natif](../../../../../crates/eidos-nif-preview/README.md), compilez Rust avec `cargo build --release --locked`, copiez l'utilitaire dans `target/release`, puis lancez `packaging/install.sh --from target/release`.

## Ensuite : pointer Steam dessus

Eidos s'exécute *en tant que* commande de lancement de votre jeu, ce qui lui
permet de monter la vue avant que le jeu ne démarre. Dans Steam, clic droit sur
le jeu -> Propriétés -> Options de lancement :

```
~/.local/bin/eidos-gui %command%
```

Depuis Colony, le chemin est `~/.local/share/Colony/apps/Eidos/eidos-linux`.
Appuyez sur Jouer. Eidos s'ouvre sur l'instance de ce jeu ; installez des mods,
triez avec LOOT, cliquez sur Run. En quittant, le montage disparaît avec lui et
votre installation est exactement dans l'état où vous l'aviez laissée.

Utilisez le chemin absolu - Steam ne lit pas le `PATH` de votre shell.

### Si vous préférez le terminal

```sh
eidos init skyrimse               # créer une instance (ajoutez un dossier pour la rendre portable)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # trier l'ordre de chargement avec LOOT
eidos play skyrimse -- %command%  # exécuter n'importe quoi à travers la vue fusionnée
```

Chaque commande qui accepte un identifiant de jeu accepte aussi le dossier d'une
instance portable - voir [usage.md](usage.md#instances--globale-et-portable).
La visite complète est dans [usage.md](usage.md).

## Facultatif : le passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` accorde la capacité facultative ; cette commande n’active pas le passthrough.
Elle s'applique au binaire `eidos` séparé du paquet ou d'une compilation depuis
les sources, jamais à `eidos-linux`, qui est aussi la fenêtre
([SECURITY.md](../../../../../SECURITY.md), en anglais).
`EIDOS_FUSE_PASSTHROUGH=1` est le réglage d’activation distinct à l’exécution. Il est **désactivé par défaut et vous voulez presque certainement le
laisser ainsi** : mesuré sur Skyrim SE, il empêche le jeu d'ouvrir ses propres
archives et plugins, si bien que les mods ne se chargent pas, en silence.
L'interrupteur existe pour retester le mécanisme, pas parce qu'il est recommandé.

Les détails, et les mesures derrière cette décision, dans
[troubleshooting.md](troubleshooting.md).

## Quelque chose ne va déjà pas ?

[troubleshooting.md](troubleshooting.md) couvre les interrupteurs
d'environnement, la lecture des compteurs d'opérations, et tous les problèmes qui
ont mordu quelqu'un jusqu'ici.
