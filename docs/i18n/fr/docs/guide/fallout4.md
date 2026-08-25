<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 à travers Eidos

Fallout 4 n'a besoin d'aucune option de lancement particulière, d'aucun exécutable
renommé et d'aucun script d'enrobage. Cela mérite d'être dit clairement, parce que
tous les autres guides Linux pour F4SE affirment le contraire - et leurs conseils
cassent à la mise à jour Steam suivante.

## L'option de lancement

```
~/.local/bin/eidos-gui %command%
```

La cible de lancement de Steam pour Fallout 4 est `Fallout4Launcher.exe`, jamais
`Fallout4.exe` ; faire tourner le script extender revient donc réellement à la
question « comment faire démarrer un autre programme à Steam ». Les réponses
habituelles consistent à réécrire `%command%` en bash :

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

ou à copier `f4se_loader.exe` par-dessus `Fallout4Launcher.exe`, que Steam restaure
discrètement à chaque mise à jour du jeu - après quoi vous jouez sans F4SE et rien
ne le dit.

Eidos effectue la substitution lui-même, depuis le descripteur du jeu : il remplace
le lanceur par `f4se_loader.exe` quand il y en a un d'installé, se rabat sur
`Fallout4.exe` quand il n'y en a pas, et **vous prévient** quand il a dû se rabattre.
Un jeu qui démarre avec tous les mods F4SE inertes est pire qu'un jeu qui ne démarre
pas.

Il y a une seconde raison de ne jamais exécuter le lanceur : il rebalaye `Data` et
réécrit `plugins.txt`, annulant l'ordre de chargement qui vient d'être déployé.
Eidos ne l'exécute jamais.

## Ce qu'Eidos prend en charge pour vous

| | |
|---|---|
| Invalidation d'archives | `Fallout4Custom.ini` reçoit `[Archive]` `bInvalidateOlderFiles=1` et un `sResourceDataDirsFinal=` vide, les deux clés qui permettent aux fichiers libres hors de `Data` d'être vus tout court. Écrit dans le profil, pas dans le dossier du jeu. |
| Ordre de chargement | `plugins.txt` au format à astérisques qu'utilise Fallout 4 (`*` marque l'actif), avec `Fallout4.ccc` respecté pour les plugins Creation Club implicites |
| LOOT | Le tri fonctionne comme pour Skyrim - `eidos sort <instance>` récupère la masterlist `fallout4` |
| Sauvegardes | Les sauvegardes `.fos` et leurs cosaves `.f4se` sont listées, copiées et conservées par profil ; le panneau de détail lit la table de plugins de la sauvegarde, donc une sauvegarde qui a besoin d'un plugin désactivé le dit avant que vous la chargiez |
| Mods root | Tout ce qu'un mod livre à côté de l'exécutable (F4SE lui-même, ENB, un `dxvk.conf`) y atterrit par le même mécanisme `Root/` que Skyrim |

## La question des versions

Fallout 4 n'est plus le jeu figé qu'il était entre 2019 et 2024. En août 2026, il
existe trois branches vivantes, et une DLL de mod compilée pour l'une ne se
chargera pas sur une autre :

| Branche | Version | F4SE |
|---|---|---|
| Classique (« old-gen ») | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Deux conséquences à connaître avant de bâtir une liste de mods :

- **Vérifiez ce que vous avez vraiment.** Des dossiers `Creations/` et `Mods/` à la
  racine du jeu signifient que vous êtes sur la lignée 1.11.x. Le panneau de détail
  d'une sauvegarde dans Eidos affiche aussi le build qui l'a écrite - Fallout
  l'inscrit dans la sauvegarde, et Eidos le fait remonter sous « Game build ».
- **Un correctif tout frais n'est pas un bon jour pour commencer.** F4SE sort en
  général un ou deux jours après une mise à jour de Bethesda, mais *Address Library
  for F4SE Plugins* - par lequel la plupart des mods DLL résolvent leurs décalages -
  suit son propre calendrier. Entre les deux, la moitié DLL de l'écosystème est à
  terre. Les mods sans DLL (textures, meshes, plugins) ne sont pas touchés.

Une fois votre pile fonctionnelle, désactivez les mises à jour automatiques de Steam
pour Fallout 4 (Propriétés → Mises à jour → « Ne mettre à jour ce jeu qu'au
lancement »), sinon le prochain correctif cassera toutes les DLL installées.

## Note matérielle : plantage des débris d'armes sur NVIDIA

L'effet de débris d'armes de Fallout 4 s'appuie sur NVIDIA FleX, un dérivé de PhysX
qu'NVIDIA a cessé de prendre en charge après la génération Pascal. Sur toute carte
Turing ou plus récente - GTX 16, RTX 20 à RTX 50 - il fait planter le jeu. C'est un
bug du jeu, sans rapport avec Linux, Proton ou Eidos.

Deux correctifs, l'un ou l'autre suffit : désactivez « Débris d'armes » dans les
réglages du jeu, ou installez *Weapon Debris Crash Fix* (Nexus 48078), qui désactive
la collision des fragments plutôt que l'effet.

## Si quelque chose semble anormal

La liste de contrôle générale est dans
[troubleshooting.fr.md](troubleshooting.md) ; la première question propre à
Fallout est toujours *quel exécutable a réellement démarré*. Eidos écrit la commande
de lancement complète dans le journal d'exécution de l'instance, donc :

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

S'il nomme `f4se_loader.exe`, la substitution a eu lieu. S'il nomme
`Fallout4Launcher.exe`, F4SE n'est pas installé là où Eidos peut le trouver - sa
place est à côté de l'exécutable du jeu, ce qui pour une installation gérée par un
gestionnaire de mods signifie le dossier `Root/` d'un mod (ou le dossier du jeu
lui-même, installé à la main).
