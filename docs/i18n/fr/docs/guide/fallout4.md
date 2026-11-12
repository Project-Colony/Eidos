<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Fallout 4 à travers Eidos

Avec Eidos, Fallout 4 n'a besoin d'aucune option de lancement particulière,
d'aucun exécutable renommé et d'aucun script d'enrobage. Il prend la même option
de lancement que tous les autres jeux.

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
à chaque mise à jour du jeu - après quoi vous jouez sans F4SE et rien ne le dit.

Eidos effectue la substitution lui-même, depuis le descripteur du jeu. Quand la
fenêtre démarre le jeu, elle remplace `Fallout4Launcher.exe` par `f4se_loader.exe`
si le chargeur est dans le dossier du jeu, dans le `Root/` d'un mod activé ou dans
`Overwrite/Root/`. Sans le chargeur, elle exécute `Fallout4.exe`, et la ligne
d'état dit
`f4se_loader.exe n'est pas installé : Fallout4.exe est lancé directement, donc les mods qui demandent le script extender ne se chargeront pas.`
Quand aucun des deux fichiers n'est trouvé, elle exécute le lanceur tel quel et le
dit aussi. Avec `eidos play fallout4 -- %command%` comme option de lancement à la
place, la CLI ne substitue que si le chargeur existe, et exécute sinon
`Fallout4Launcher.exe`.

Il y a une seconde raison d'éviter le lanceur : il rebalaye `Data` et réécrit
`plugins.txt`, annulant l'ordre de chargement qui vient d'être déployé.

Quand vous ouvrez la fenêtre vous-même et appuyez sur Lancer, Eidos demande à Steam de
démarrer le jeu (`steam -applaunch 377160`), et Steam démarre Eidos par l'option de
lancement ci-dessus. Si `f4se_loader.exe` n'est pas à côté de l'exécutable du jeu
dans l'instance, il ne le fait pas, et la ligne d'état vous demande d'installer
d'abord le script extender. Fallout 4 VR fonctionne de la même façon, avec
`Fallout4VRLauncher.exe` et `f4sevr_loader.exe`.

## Ce qu'Eidos prend en charge pour vous

| Domaine | Ce que fait Eidos |
|---|---|
| Invalidation d'archives | `Fallout4Custom.ini` reçoit `[Archive]` `bInvalidateOlderFiles=1` et un `sResourceDataDirsFinal=` vide, les deux clés qui permettent aux fichiers libres hors de `Data` d'être vus tout court. Écrit dans le profil, pas dans le dossier du jeu. |
| INI | `Fallout4.ini`, `Fallout4Prefs.ini` et `Fallout4Custom.ini` appartiennent au profil. BethINI Pie lancé par Eidos modifie les copies du profil, et Sauvegardes -> INI remet la copie qu'Eidos garde avant chacune de ses exécutions ([tools.md](tools.md#bethini-pie)). |
| Ordre de chargement | `plugins.txt` au format à astérisques qu'utilise Fallout 4 (`*` marque l'actif), avec `Fallout4.ccc` respecté pour les plugins Creation Club implicites |
| LOOT | Le tri fonctionne comme pour Skyrim - `eidos sort <instance>` récupère la masterlist `fallout4` |
| Sauvegardes | Les sauvegardes `.fos` et leurs cosaves `.f4se` sont listées, copiées et conservées par profil ; le panneau de détail lit la table de plugins de la sauvegarde, donc une sauvegarde qui a besoin d'un plugin désactivé le dit avant que vous la chargiez. Désactiver ou supprimer un mod dont le plugin est utilisé par l'une des trois sauvegardes les plus récentes du profil demande d'abord confirmation. |
| Mods root | Tout ce qu'un mod livre à côté de l'exécutable (F4SE lui-même, ENB, un `dxvk.conf`) y atterrit par le même mécanisme `Root/` que Skyrim |

## La question des versions

Fallout 4 n'est plus le jeu figé qu'il était entre 2019 et 2024. En août 2026, il
existe trois branches vivantes, et une DLL de mod compilée pour l'une ne se
chargera pas sur une autre :

| Branche | Version | F4SE |
|---|---|---|
| Classique (« old-gen ») | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 à 1.11.240 | 0.7.4 à 0.7.9 |

Deux conséquences à connaître avant de bâtir une liste de mods :

- Des dossiers `Creations/` et `Mods/` à la racine du jeu signifient que vous êtes
  sur la lignée 1.11.x. Le panneau de détail d'une sauvegarde dans Eidos affiche
  aussi le build qui l'a écrite : Fallout l'inscrit dans la sauvegarde, et Eidos
  l'affiche sous « Version du jeu ».
- F4SE sort en général un ou deux jours après une mise à jour de Bethesda, mais
  *Address Library for F4SE Plugins* - par lequel la plupart des mods DLL
  résolvent leurs décalages - suit son propre calendrier. Entre les deux, les mods
  DLL ne se chargent pas. Les mods sans DLL (textures, meshes, plugins) ne sont pas
  touchés, donc un correctif tout frais est un mauvais jour pour commencer une
  liste de mods.

Une fois votre pile fonctionnelle, figez la version du jeu dans l'instance : Figer
cette version sur la carte Fichiers du jeu de l'onglet Diagnostics, ou
`eidos stock-game fallout4 freeze` pour voir la taille et
`eidos stock-game fallout4 freeze --yes` pour copier. Eidos copie l'installation de
la boutique dans `<instance>/Stock Game` et exécute le jeu depuis cette copie, que
Steam démarre toujours. Un correctif ultérieur change l'installation de la boutique
et pas la copie, si bien que les DLL que vous avez installées gardent leur version
du jeu. Sur btrfs ou XFS, la copie partage les blocs de la boutique et ne prend
presque pas de place tant que Steam ne modifie pas un fichier ; sur ext4, c'est une
copie complète aussi grande que le dossier du jeu. Eidos refuse tant que Steam
signale le jeu occupé ou pas entièrement installé. Les détails sont dans
[usage.md](usage.md#fichiers-du-jeu--linstallation-de-la-boutique-ou-une-copie).

Sans copie, désactivez les mises à jour automatiques de Steam pour Fallout 4
(Propriétés, Mises à jour, « Ne mettre à jour ce jeu qu'au lancement »), sinon le
prochain correctif cassera toutes les DLL installées.

## Note matérielle : plantage des débris d'armes sur NVIDIA

L'effet de débris d'armes de Fallout 4 s'appuie sur NVIDIA FleX, un dérivé de PhysX
qu'NVIDIA a cessé de prendre en charge après la génération Pascal. Sur toute carte
Turing ou plus récente - GTX 16, RTX 20 à RTX 50 - il fait planter le jeu. C'est un
bug du jeu, sans rapport avec Linux, Proton ou Eidos.

Deux correctifs, l'un ou l'autre suffit : désactivez « Débris d'armes » dans les
réglages du jeu, ou installez *Weapon Debris Crash Fix* (Nexus 48078), qui désactive
la collision des fragments plutôt que l'effet.

## Cartes de diagnostic pour Fallout 4

L'onglet Diagnostics montre une carte pour chacun de ces cas :

- Des plugins F4SE sont installés dans `F4SE/Plugins`, mais `f4se_loader.exe` n'est
  dans aucun mod activé ni dans le dossier du jeu. Le jeu démarre sans eux.
- `f4se_loader.exe` est installé, mais pas le `f4se_<version>.dll` de la version du
  jeu en cours (`f4se_1_10_984.dll` pour 1.10.984). Le chargeur s'arrête avec une
  erreur ; cela suit en général une mise à jour du jeu.
- L'exécutable du jeu ou son build Steam a changé depuis le dernier lancement par
  Eidos qui s'est terminé sans erreur. Les plugins F4SE compilés pour l'ancienne
  version peuvent ne plus se charger.
- Un mod de priorité plus haute que F4SE contient des copies libres `.pex` des
  scripts propres à F4SE. Une copie ancienne casse les fonctions F4SE qu'elle
  déclare, et le jeu n'affiche aucune erreur.
- Le `Fallout4Custom.ini` du profil a des lignes `sTestFile1` à `sTestFile10` dans
  `[General]`. Tant qu'une est définie, le jeu charge ces plugins et ignore
  `plugins.txt`. Corriger les retire et garde le fichier précédent sous le nom
  `Fallout4Custom.ini.pre-fix`.
- L'archive BA2 d'un mod stocke de l'audio `.wav` ou `.xwm` compressé. Le jeu peut
  planter quand il joue ce son. Réempaquetez l'archive avec l'audio non compressé.
  Fallout 4 VR reçoit la même carte.
- Un enregistreur de plantages a écrit un journal pendant le dernier lancement : le
  `crash-*.log` de Buffout 4 dans `Documents/My Games/Fallout4/F4SE`, ou un `.log`
  dans `F4SE/Crashlogs`. La carte nomme l'exception et les DLL et plugins du
  journal que vos mods installent.
- La carte de la fenêtre montre la taille et le mode de fenêtre réglés dans le
  `Fallout4Prefs.ini` du profil, et Appliquer y écrit `iSize W`, `iSize H`,
  `bFull Screen` et `bBorderless`.
- Un prérequis Nexus nomme un DLC. Eidos cherche un plugin portant le nom du DLC,
  et les plugins de DLC de Fallout 4 ont d'autres noms (`DLCCoast.esm` pour Far
  Harbor), donc cette carte est un conseil : vérifiez à la main, puis appuyez sur
  Ignorer.

Les deux cartes du chargeur ont un bouton Ouvrir la page de téléchargement pour
<https://f4se.silverlock.org/> et un bouton Installer l'archive téléchargée, qui
place le chargeur dans le `Root/` du mod. `eidos check fallout4` affiche les mêmes
constats, sauf la carte `.pex`, la carte de la fenêtre et les cartes de prérequis Nexus,
que seule la fenêtre calcule.

## Si quelque chose semble anormal

La liste de contrôle générale est dans [troubleshooting.md](troubleshooting.md) ;
la première question propre à Fallout est toujours *quel exécutable a réellement
démarré*. Quand la fenêtre démarre le jeu, elle écrit la commande de lancement
complète dans le journal d'exécution de l'instance, donc :

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

S'il nomme `f4se_loader.exe`, la substitution a eu lieu. S'il nomme
`Fallout4.exe`, F4SE n'est pas installé là où Eidos peut le trouver - sa place est
à côté de l'exécutable du jeu, ce qui pour une installation gérée par un
gestionnaire de mods signifie le dossier `Root/` d'un mod (ou le dossier du jeu
lui-même, installé à la main). S'il nomme `Fallout4Launcher.exe`, Eidos n'a trouvé
ni le chargeur ni `Fallout4.exe` dans la vue root.

Quand le bon exécutable démarre et que le jeu plante quand même, Partie test et
Trouver le mod à l'origine d'un problème le réduisent à
un seul mod sans toucher aux sauvegardes du
profil ([usage.md](usage.md#parties-test)).
