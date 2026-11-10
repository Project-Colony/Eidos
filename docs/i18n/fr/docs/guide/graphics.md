<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders, DLSS et génération d'images

Community Shaders 1.4+ livre sa propre mise à l'échelle (DLSS 4 / FSR 3.1 / XeSS,
via le paquet séparé « Upscaling - Community Shaders ») et la génération d'images
FSR 3.1. Tout cela fonctionne à travers Eidos sous Linux - CS et ses paquets
s'installent comme des mods ordinaires et l'union sert leurs DLL comme n'importe
quoi d'autre - mais trois réglages ne se voient pas depuis l'intérieur du jeu, et
chacun fait que la fonctionnalité ne fait rien, sans erreur : la variable NVAPI, le
plein écran exclusif et le plafond d'IPS de Reflex.

L'onglet Diagnostics vérifie les deux premiers, et la ligne ENB plus bas, pour un
jeu qui a un préfixe Wine (le préfixe Proton de Steam, ou celui qu'Heroic a créé).
Le plafond de Reflex n'est pas vérifié. Quand l'instance exécute le jeu depuis une
copie comme `Stock Game`, les vérifications lisent les fichiers de la copie, pas
ceux de l'installation de la boutique. `eidos check <instance>` affiche les mêmes
cartes dans un terminal, sans les boutons Fix.

- La carte NVAPI apparaît pour une copie Steam de Skyrim SE ou d'Enderal SE quand un
  pilote NVIDIA est chargé (`/proc/driver/nvidia/version` existe), que le `Data`
  fusionné contient quelque chose dans `Shaders/Upscaling`, que le Proton du jeu
  laisse NVAPI désactivé par défaut, et que ni l'option de lancement de Steam, ni
  l'environnement d'Eidos, ni le `launch-env.ini` du profil ne définit
  `PROTON_ENABLE_NVAPI=1`.
- La carte du plein écran exclusif n'apparaît qu'avec le paquet Upscaling installé,
  quand le `SkyrimPrefs.ini` du profil a `bFull Screen=1` et que
  `SKSE/Plugins/SSEDisplayTweaks.ini` ne définit pas à la fois `Fullscreen=false` et
  `Borderless=true`. Son bouton Fix écrit `bFull Screen=0` et `bBorderless=1` dans le
  `SkyrimPrefs.ini` du profil. La vérification lit aussi `SkyrimPrefs.ini` pour
  Enderal SE, alors qu'un profil Enderal SE contient `EnderalPrefs.ini` : sur
  Enderal SE, utilisez plutôt la carte Display plus bas.
- La carte ENB apparaît pour tout jeu dont l'`enblocal.ini` gagnant dans la vue root
  n'a pas `LinuxVersion=true` dans `[GLOBAL]`. Elle a aussi un bouton Fix.

## L'option de lancement dont DLSS a besoin

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Utilisez le chemin absolu de votre propre `eidos-gui` ou `eidos-linux`, comme dans
[install.md](install.md). La carte donne la ligne à définir : l'option de lancement
actuelle du jeu avec `PROTON_ENABLE_NVAPI=1` devant, ou, s'il n'y en a pas, la ligne
construite avec le chemin de l'Eidos en cours.

Sans la couche NVIDIA NVAPI (dxvk-nvapi), CS ne peut pas initialiser DLSS et se
rabat sur la mise à l'échelle FSR, sans rien à l'écran qui dise pourquoi. Si un
build de Proton active la couche de lui-même, c'est écrit dans son script `proton`,
et Eidos y lit la ligne `use_nvapi`. Un interrupteur de retrait (`disablenvapi`)
veut dire activée par défaut : GE-Proton11-9 et Proton 11.0 ont cette forme
(vérifié le 2026-11-09). Un interrupteur d'activation (`enablenvapi`) veut dire
désactivée, et seul ce build a besoin de la variable. La définir ne coûte rien sur
une machine non NVIDIA. La génération d'images elle-même est du FSR 3.1 et n'a pas
besoin de NVAPI ; seul l'upscaler DLSS en a besoin.

Pour la définir pour un seul profil, mettez `env=PROTON_ENABLE_NVAPI=1` dans le
`launch-env.ini` de ce profil (View -> INI editor, dernier onglet).

## La génération d'images exige le mode fenêtré sans bordure

La génération d'images de CS s'appuie sur un proxy de présentation D3D12 et refuse
catégoriquement le plein écran exclusif. Avec `bFull Screen=1` dans
`SkyrimPrefs.ini`, elle ne s'enclenche jamais : aucune erreur, aucun message, et le
jeu reste à sa fréquence d'images de base. SSE Display Tweaks impose le mode au
niveau du moteur, quoi que disent les INI :

```ini
[Render]
Fullscreen=false
Borderless=true
```

La fenêtre a exactement le même aspect (sans bordure, à la résolution native).
Seul le mode que le moteur annonce change, et c'est ce mode que CS vérifie.

La carte Display de Diagnostics règle la taille et le mode en une étape, pour
Skyrim SE, Enderal SE et Fallout 4. Elle montre la taille de l'écran connecté le
plus grand. Avec Borderless window sélectionné, Apply écrit `iSize W`, `iSize H`,
`bFull Screen=0` et `bBorderless=1` dans le `SkyrimPrefs.ini` du profil
(`EnderalPrefs.ini` pour Enderal SE, `Fallout4Prefs.ini` pour Fallout 4). Quand SSE
Display Tweaks règle la fenêtre, Apply écrit aussi `Resolution`, `Fullscreen=false`
et `Borderless=true` dans une copie de son INI dans l'Overwrite
(`SKSE/Plugins/SSEDisplayTweaks.ini`), et le fichier du mod reste tel quel. Avec
**Start the game at once when a store opens Eidos** activé (Preferences, General,
Running a game), la fenêtre s'ouvre quand même si le profil n'a pas encore de
taille de fenêtre et que SSE Display Tweaks ne règle pas la fenêtre, pour que la
taille puisse être réglée avant le premier démarrage.

Deux autres conditions d'activation, avec le même échec silencieux :

- L'écran se rafraîchit à 120 Hz ou plus, ou `frameGenerationForceEnable` est
  activé dans les réglages d'upscaling de CS. La génération d'images double la
  cadence présentée, donc CS refuse de l'armer sur un écran incapable d'afficher le
  résultat.
- Le paquet Upscaling est installé (son arborescence `Data/Shaders/Upscaling/`
  contient les DLL Streamline et FidelityFX). Sans lui, CS affiche les entrées de
  menu et ne peut rien activer.

## La limite d'images de Reflex peut étrangler la sortie

Les réglages Reflex de CS portent leur propre plafond d'IPS (`reflexFPSLimit`, avec
`reflexUseFPSLimit`). Un plafond resté à une ancienne valeur - le nôtre était à 79,
hérité d'un réglage précédent - se situe en aval de la génération d'images et
rogne exactement les images qu'elle produit : 60 de base doublé à 120, ramené à 79,
se lit comme « la génération d'images ne fait rien ». Sur un écran 144 Hz, le
plafond Reflex conventionnel est d'environ 138. Vérifiez-le dès que des images
générées semblent manquer.

## Interaction connue : écran noir avec SSE Display Tweaks

La combinaison FG + Display Tweaks + DXVK a un échec d'écran noir connu. Correctif,
dans l'ordre :

1. `SSEDisplayTweaks.ini` : `DisableBufferResizing=true`
2. Si cela ne suffit pas, un `dxvk.conf` à côté de l'exécutable du jeu (le dossier
   `Root/` d'un mod en place un là) avec
   `dxvk.enableGraphicsPipelineLibrary = False`

## Mods de textures mélangés : PGPatcher

Community Shaders sait rendre le parallax, le complex material et le PBR, mais
seulement là où un mesh est préparé pour. Historiquement cela voulait dire
installer des meshes pré-patchés puis un mod pour rééteindre l'effet partout où
les textures manquaient : une couverture limitée aux meshes que vous aviez, et
du CPU dépensé à l'exécution pour défaire un choix fait à l'installation.

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) inverse
la chose. Vous installez les textures que vous voulez, de n'importe quel type de
shader, et il réécrit les meshes et les plugins pour que chaque surface utilise
le bon - y compris les enregistrements de textures alternatives des plugins, que
l'ancienne méthode laissait de côté. Sans lui, une texture qui demande un shader
que son mesh ne réclame pas se charge mais ne s'affiche pas comme prévu, ce qui
arrive surtout sur une installation qui mélange les types de textures.

Deux choses à savoir avant de le lancer sous Linux :

- il lui faut l'argument `--ignore-mo2vfscheck` ou il quitte aussitôt en se
  plaignant de MO2. Eidos le fournit - voir
  [Outils](tools.md#pourquoi-pgpatcher-a-besoin-de---ignore-mo2vfscheck) pour ce qu'est ce drapeau et pourquoi la
  vérification échoue ici ;
- il modifie des meshes, donc il tourne après l'installation de tous les
  mods de textures et avant DynDOLOD, qui doit voir les meshes finaux.
  Changer vos textures ensuite veut dire relancer les deux, dans cet ordre.

Sa sortie ne va pas dans l'Overwrite. PGPatcher écrit uniquement dans le dossier
réglé dans le champ Output location de son lanceur, et il refuse un dossier
situé dans `Data`. Faites pointer ce champ vers un dossier à lui, par exemple un
nouveau dossier `PGPatcher Output` dans le `mods/` de l'instance. Gardez ce mod
désactivé pendant que PGPatcher tourne, car PGPatcher vide le dossier avant de
le réécrire et Eidos s'attend à ce que les mods activés ne changent pas tant que
la vue est montée. Activez-le ensuite avec une priorité haute : il est fait pour
gagner.

Ne donnez pas à l'entrée de PGPatcher un mod dans **Capture output into** pour
ensuite utiliser ce mod comme Output location. Eidos déplace les dossiers `cfg/` et `log/` propres à PGPatcher dans ce mod
après chaque exécution, et PGPatcher s'arrête alors avec le message
« Output directory has non-PGPatcher related files ».

## ENB sous Proton : LinuxVersion

Jackify écrit `LinuxVersion=true` dans la section `[GLOBAL]` de `enblocal.ini` pour
chaque ENB qu'il installe sous Proton. La documentation d'ENB ne décrit pas cette
clé. Une carte Health de l'onglet Diagnostics signale un `enblocal.ini` sans cette
ligne, et son bouton Fix écrit dans `overwrite/Root/` une copie de l'`enblocal.ini`
gagnant avec la ligne définie, de sorte que le mod ENB garde son fichier.

Une autre carte signale un ENB présent dans le dossier du jeu (un `enblocal.ini`,
un `enbseries.ini` ou un dossier `enbseries/`) alors qu'un mod activé livre
Community Shaders (`SKSE/Plugins/CommunityShaders.dll`). Les deux peuvent
fonctionner ensemble. Si l'image paraît fausse, désactivez l'un des deux dans son
INI.

## Lire les chiffres ensuite

Les images générées ne concernent que la présentation : le moteur simule toujours
à la cadence de base, Havok bat toujours à la cadence de base, et tout ce qui
compte les images *du moteur* (y compris les compteurs de CS) continue d'annoncer
~60 pendant que l'écran en affiche ~120. Le compteur a raison, et c'est pourquoi la génération d'images est sans danger pour la
physique là où augmenter la cadence du moteur ne l'est pas. `DXVK_HUD=fps` dans les
options de lancement affiche un compteur si vous en voulez un à l'écran.

Une règle : l'interpolation au niveau du pilote (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) et la génération d'images de CS sont des
technologies concurrentes. Utilisez l'une ou l'autre, jamais les deux.
