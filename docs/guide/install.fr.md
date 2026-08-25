<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Installer Eidos

Trois portes d'entrée. Toutes livrent les deux mêmes binaires - `eidos` (la ligne
de commande) et `eidos-gui` - plus le gestionnaire `nxm://` qui fait atterrir le
bouton « Mod Manager Download » de Nexus dans votre instance.

## Ce qu'il vous faut d'abord

| | |
|---|---|
| **Linux avec FUSE** | `fusermount3` dans votre PATH. Toutes les distributions actuelles le fournissent. |
| **Un jeu Proton, lancé une fois** | Steam ne crée le préfixe Wine du jeu qu'au premier lancement, et Eidos travaille dedans. |
| **`7z`** | Pour installer les archives de mods. `p7zip` sur la plupart des distributions. |

Pas de root, pas de démon, aucune modification de `/etc/fuse.conf`, et rien à
ajouter à vos groupes. Eidos monte dans un espace de noms privé qui appartient au
processus du jeu.

## Arch

```bash
cd packaging && makepkg -si
```

## Une archive de version

```bash
./install.sh
```

Installe dans `~/.local/bin` par défaut. `--system` place les binaires dans
`/usr/local/bin`, `--bindir DIR` ailleurs. Relancer ce script est la façon
prévue de mettre à jour.

## Depuis les sources

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Ensuite : pointer Steam dessus

Eidos s'exécute *en tant que* commande de lancement de votre jeu, ce qui lui
permet de monter la vue avant que le jeu ne démarre. Dans Steam, clic droit sur
le jeu -> Propriétés -> Options de lancement :

```
~/.local/bin/eidos-gui %command%
```

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
instance portable - voir [usage.md](usage.md#instances-global-and-portable) 🇬🇧.
La visite complète est dans [usage.md](usage.md) 🇬🇧.

## Facultatif : le passthrough FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` active le passthrough FUSE
du noyau. Il est **désactivé par défaut et vous voulez presque certainement le
laisser ainsi** : mesuré sur Skyrim SE, il empêche le jeu d'ouvrir ses propres
archives et plugins, si bien que les mods ne se chargent pas, en silence.
L'interrupteur existe pour retester le mécanisme, pas parce qu'il est recommandé.

Les détails, et les mesures derrière cette décision, dans
[troubleshooting.fr.md](troubleshooting.fr.md).

## Quelque chose ne va déjà pas ?

[troubleshooting.fr.md](troubleshooting.fr.md) couvre les interrupteurs
d'environnement, la lecture des compteurs d'opérations, et tous les problèmes qui
ont mordu quelqu'un jusqu'ici.
