<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders, DLSS et génération d'images

Community Shaders 1.4+ livre sa propre mise à l'échelle (DLSS 4 / FSR 3.1 / XeSS,
via le paquet séparé « Upscaling - Community Shaders ») et la génération d'images
FSR 3.1. Tout cela fonctionne à travers Eidos sous Linux - CS et ses paquets
s'installent comme des mods ordinaires et l'union sert leurs DLL comme n'importe
quoi d'autre - mais trois choses ne sont **pas** découvrables depuis l'intérieur du
jeu, et chacune fait que la fonctionnalité ne fait rien, en silence. Cette page en
est la liste, apprise à la dure sur une installation réelle.

## L'option de lancement dont DLSS a besoin

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton désactive sa couche NVIDIA NVAPI (dxvk-nvapi) sauf si le jeu figure sur la
liste blanche de Valve, et Skyrim n'y est pas. Sans elle, CS ne peut pas
initialiser DLSS et se rabat sur la mise à l'échelle FSR - discrètement, sans rien
à l'écran qui dise pourquoi. Définir la variable ne coûte rien sur une machine non
NVIDIA, donc l'option de lancement sûre est simplement la ligne ci-dessus. La
génération d'images elle-même est du FSR 3.1 et n'a pas besoin de NVAPI ; seul
l'upscaler DLSS en a besoin.

## La génération d'images exige le mode fenêtré sans bordure

La génération d'images de CS s'appuie sur un proxy de présentation D3D12 et refuse
catégoriquement le plein écran exclusif. `bFull Screen=1` dans `SkyrimPrefs.ini`
signifie qu'elle ne s'enclenche jamais - aucune erreur, aucun message, juste la
fréquence d'images de base. Le correctif robuste est SSE Display Tweaks, qui impose
le mode au niveau du moteur quoi que disent les INI :

```ini
[Render]
Fullscreen=false
Borderless=true
```

La fenêtre a exactement le même aspect (sans bordure, à la résolution native) ;
seul ce que le moteur croit change - et ce que le moteur croit est ce que CS
vérifie.

Deux autres conditions d'activation, avec le même échec silencieux :

- **Un écran à 120 Hz ou plus**, ou bien activez `frameGenerationForceEnable` dans
  les réglages d'upscaling de CS. La génération d'images double la cadence
  présentée, donc CS refuse de l'armer sur un écran incapable d'afficher le
  résultat.
- **Le paquet Upscaling installé** (son arborescence `Data/Shaders/Upscaling/`
  contient les DLL Streamline et FidelityFX). Sans lui, CS affiche les entrées de
  menu et ne peut rien activer.

## La limite d'images de Reflex peut étrangler la sortie

Les réglages Reflex de CS portent leur propre plafond d'IPS (`reflexFPSLimit`, avec
`reflexUseFPSLimit`). Un plafond resté à une ancienne valeur - le nôtre était à 79,
hérité d'un réglage précédent - se situe en aval de la génération d'images et
rogne exactement les images qu'elle produit : 60 de base doublé à 120, ramené à 79,
se lit comme « la génération d'images ne fait rien ». Sur un écran 144 Hz, le
plafond Reflex conventionnel est d'environ 138. Vérifiez-le dès que la sortie
générée semble absente ; c'est le deuxième tueur silencieux après le plein écran
exclusif.

## Interaction connue : écran noir avec SSE Display Tweaks

La combinaison FG + Display Tweaks + DXVK a un échec d'écran noir connu. Correctif,
dans l'ordre :

1. `SSEDisplayTweaks.ini` : `DisableBufferResizing=true`
2. Si cela ne suffit pas, un `dxvk.conf` à côté de l'exécutable du jeu (le dossier
   `Root/` d'un mod en place un là) avec
   `dxvk.enableGraphicsPipelineLibrary = False`

## Lire les chiffres ensuite

Les images générées ne concernent que la présentation : le moteur simule toujours
à la cadence de base, Havok bat toujours à la cadence de base, et tout ce qui
compte les images *du moteur* (y compris les compteurs de CS) continue d'annoncer
~60 pendant que l'écran en affiche ~120. C'est le comportement correct, pas un
compteur cassé - et c'est pourquoi la génération d'images est sans danger pour la
physique là où augmenter la cadence du moteur ne l'est pas. `DXVK_HUD=fps` dans les
options de lancement affiche un compteur si vous en voulez un à l'écran.

Une règle : l'interpolation au niveau du pilote (NVIDIA Smooth Motion,
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`) et la génération d'images de CS sont des
technologies concurrentes. Utilisez l'une ou l'autre, jamais les deux.
