<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Extensions

Une extension ajoute une entrée à Eidos sans faire partie d'Eidos. C'est un
manifeste TOML nommant un programme, plus, tout au plus, ce programme.

Les manifestes vivent dans `~/.config/Colony/Eidos/addons/`, un `.toml` par
extension. Ouvrez le dossier depuis **View -> Extensions -> Open folder**, puis
appuyez sur **Reload** - pas de redémarrage.

## Pourquoi rien n'est chargé dans Eidos

Mod Organizer 2 charge ses greffons comme bibliothèques partagées et héberge ceux
en Python via Qt. Ni l'un ni l'autre ne se transpose. Rust n'a pas d'ABI stable :
une bibliothèque partagée compilée avec un autre compilateur - ou un autre drapeau
d'optimisation, ou un autre jeu de fonctionnalités d'une dépendance commune - est
un comportement indéfini, pas une incompatibilité de version. Et les widgets
d'Eidos sont génériques à la compilation, donc une bibliothèque ne pourrait même
pas en construire un à rendre, l'ABI fût-elle stable.

Une extension est donc un programme qu'Eidos *exécute*. Elle ne peut pas faire
planter la fenêtre, ne peut pas corrompre une liste de mods, et continue de
fonctionner d'une version d'Eidos à l'autre.

## Un outil

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # omettre pour tous les jeux
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Elle apparaît dans **View -> Extensions** avec un bouton Run, et démarre détachée
- Eidos ne l'attend pas.

## Un contrôle

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Il s'exécute à chaque rafraîchissement et écrit un constat par ligne :

```
level<TAB>title<TAB>detail
```

où `level` vaut `problem`, `advice` ou `ok`. Le détail est facultatif. Tout ce qui
ne commence pas par un niveau connu est ignoré, si bien qu'une sortie de
progression ou un avertissement égaré ne peuvent pas produire une ligne qui
ressemble à un contrôle d'Eidos lui-même. Les constats apparaissent dans l'onglet
**Health**, préfixés du nom de l'extension.

Un contrôle dispose de trois secondes. Celui qui déborde est arrêté et signalé
comme un problème contre lui-même - il s'exécute au même rafraîchissement qui suit
chaque clic, donc un contrôle bloqué figerait la fenêtre.

## Substitutions

`args` et `workdir` développent celles-ci :

| Substitution    | Ce que c'est                                 |
| --------------- | -------------------------------------------- |
| `{instance}`    | la racine de l'instance                      |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | le nom du profil actif                       |
| `{profile_dir}` | le dossier du profil actif                   |
| `{game}`        | l'identifiant du jeu, p. ex. `skyrimse`      |
| `{game_name}`   | le nom d'affichage du jeu                    |
| `{install}`     | le dossier d'installation du jeu             |
| `{data}`        | le dossier `Data` du jeu                     |

Une substitution inconnue est laissée exactement telle qu'écrite plutôt que vidée,
pour qu'une faute échoue visiblement au lieu de transformer `--out {typo}` en
`--out --next-flag`. Lancer un outil dont toutes les substitutions ne peuvent pas
être résolues est refusé, et Eidos dit lesquelles manquent.

## Ce qu'une extension ne peut pas faire

Elle reçoit des valeurs et s'exécute ; elle ne peut pas rappeler Eidos, changer la
liste de mods, ni dessiner quoi que ce soit dans la fenêtre. C'est délibéré. Ce
pour quoi MO2 utilise des greffons et qui DOIT atteindre l'intérieur - la prise en
charge des jeux, les installateurs, le moteur de conflits - est intégré ici plutôt
que rapporté : une définition de jeu est son propre TOML dans
`~/.config/Colony/Eidos/games/`, et les installateurs FOMOD et BAIN sont natifs.
