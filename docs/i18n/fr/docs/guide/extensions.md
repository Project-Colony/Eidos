<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# Extensions

Une extension ajoute une entrée à Eidos sans faire partie d'Eidos. C'est un
manifeste TOML nommant un programme, plus, tout au plus, ce programme.

Les manifestes vivent dans `~/.config/Colony/Eidos/addons/`, un `.toml` par
extension. Ouvrez le dossier depuis Affichage -> Extensions -> Ouvrir le dossier,
puis appuyez sur Recharger - pas de redémarrage.

`id`, `kind` et `exec` sont obligatoires ; `name` vaut l'identifiant par défaut.
`exec` nomme le programme. Un nom sans `/` est cherché dans le `PATH`. Quand le
programme manque, Eidos ignore l'extension, et sa ligne dans Affichage -> Extensions
dit pourquoi (`<exec> is not on PATH` ou `<exec> is not there`). `games` limite
l'extension à ces identifiants de jeu ; sans lui, l'extension s'applique à tous les
jeux, et sur un autre jeu sa ligne indique « pas pour ce jeu ».

Un manifeste qu'Eidos refuse est listé en premier, avec la raison : un fichier
illisible, un TOML invalide ou un champ obligatoire absent, un `id` vide ou
contenant une espace, un `exec` vide, un `id` qu'un manifeste précédent a déjà pris
(les fichiers sont lus par ordre de nom), un `kind` qu'Eidos ne connaît pas, ou un
installateur, un aperçu ou une info de sauvegarde sans `protocol = 1` et sans
nom de fichier `extensions`.

## Pourquoi rien n'est chargé dans Eidos

Mod Organizer 2 charge ses greffons comme bibliothèques partagées et héberge ceux
en Python via Qt. Ni l'un ni l'autre ne se transpose. Rust n'a pas d'ABI stable :
une bibliothèque partagée compilée avec un autre compilateur - ou un autre drapeau
d'optimisation, ou un autre jeu de fonctionnalités d'une dépendance commune - est
un comportement indéfini, et rien ne le signale comme une incompatibilité de
version. Et les widgets
d'Eidos sont génériques à la compilation, donc une bibliothèque ne pourrait même
pas en construire un à rendre, l'ABI fût-elle stable.

Une extension est un programme qu'Eidos exécute avec les droits d'accès de votre compte au système de fichiers. N'installez que des utilitaires auxquels vous faites confiance. L'isolation des processus et la vérification des réponses ne constituent pas un bac à sable du système d'exploitation.

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

Elle apparaît dans Affichage -> Extensions avec un bouton Lancer, et démarre détachée
- Eidos ne l'attend pas, et la ligne d'état donne son pid. Sa sortie est jetée.
Sans `workdir`, elle démarre dans le dossier d'où Eidos lui-même a été lancé.

## Un contrôle

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Il s'exécute à chaque rafraîchissement, dans la racine de l'instance sauf si
`workdir` dit autre chose, et écrit un constat par ligne :

```
level<TAB>title<TAB>detail
```

où `level` vaut `problem`, `advice` ou `ok`. Le détail est facultatif ; une ligne
sans titre est abandonnée. Tout ce qui ne commence pas par un niveau connu est
ignoré, si bien qu'une sortie de progression ou un avertissement égaré ne peuvent
pas produire une ligne qui ressemble à un contrôle d'Eidos lui-même. Les constats
apparaissent dans l'onglet Diagnostics sous la forme `<name> - <title>`, après les
cartes du préfixe et de Nexus et avant les autres cartes de diagnostic.

Un contrôle dispose de trois secondes. Celui qui déborde est arrêté et signalé
comme un problème contre lui-même - il s'exécute au même rafraîchissement qui suit
chaque clic, donc un contrôle bloqué figerait la fenêtre. L'horloge tourne jusqu'à
ce que le contrôle soit terminé et que ses tubes de sortie soient fermés : un
processus fils en arrière-plan qui les garde ouverts compte donc dans les trois
secondes. Un contrôle qui se termine en erreur sans rien écrire est signalé de la
même façon, avec la première ligne de sa sortie d'erreur, de même que celui qui
écrit plus de 1 Mio sur l'un ou l'autre flux. Dans les deux cas Eidos ne le relance
pas avant que vous appuyiez sur Recharger, et sa ligne dit « Elle ne sera pas
relancée avant que vous appuyiez sur Recharger. » Un contrôle ne lit
rien sur son entrée standard.

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
| `{install}`     | le dossier du jeu que l'instance exécute     |
| `{data}`        | le dossier `Data` de ce dossier              |

`{install}` est l'installation de la boutique, ou la copie du jeu quand l'instance
en exécute une (un `Stock Game` figé ou le dossier de jeu propre à une liste ;
voir [usage.md](usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie)). `{data}` la
suit.

Un contrôle reçoit aussi `{addon_dir}`, le dossier qui contient son manifeste, et
les extensions du protocole 1 ci-dessous aussi. Un outil non.

Une substitution inconnue n'est jamais vidée, ce qui transformerait `--out {typo}`
en `--out --next-flag`. Eidos refuse d'exécuter un outil ou un contrôle dont les
substitutions ne peuvent pas toutes être résolues, et dit lesquelles manquent :
dans la ligne d'état pour un outil, dans la ligne du contrôle dans l'onglet
Diagnostics pour un contrôle.

## Ce qu'une extension ne peut pas faire

Une extension ne peut pas rappeler Eidos, s'enregistrer depuis une archive ni dessiner ses propres widgets. Les installateurs structurés du protocole 1 peuvent renvoyer des plans de fichiers vérifiés et des demandes de choix ; Eidos assure la révision, la préparation temporaire et la publication. FOMOD et BAIN restent des installateurs natifs. Les extensions d'aperçu et d'informations sur les sauvegardes utilisent le même hôte aux réponses bornées. Consultez le [contrat du protocole et les exemples exécutables](../../../../guide/examples/extensions/README.md) (en anglais) pour la priorité de correspondance, les réponses CLI enregistrées, les limites et les contrôles d'identité lors de la répétition.
