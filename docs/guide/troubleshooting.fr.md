<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Dépannage et diagnostics

Tout ce qu'il faut pour le jour où le jeu voit quelque chose que le système de
fichiers ne confirme pas : les interrupteurs d'environnement, la lecture des
compteurs d'opérations, les problèmes connus avec leur histoire, et l'affaire du
passthrough.

### Diagnostiquer le VFS

Deux variables d'environnement existent pour le moment où le jeu voit quelque
chose que le système de fichiers ne confirme pas :

```sh
EIDOS_FUSE_STATS=1                  # compteurs d'opérations, vidés au démontage
EIDOS_FUSE_NO_CACHE=1               # tous les caches côté noyau désactivés
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # ou nommez-les un par un
```

La forme granulaire est ce qui a trouvé le plantage décrit plus bas : tout couper
répond à « est-ce le cache ? », et seul le fait de les nommer répond à « lequel ».
Les compteurs répondent à l'autre moitié - un chargement qui affiche `read 0` est
un chargement où `FUSE_PASSTHROUGH` a servi chaque octet dans le noyau, donc tout
ce que vous vous apprêtiez à régler sur le chemin de lecture est déjà gratuit.

## Monter une union à la main

La première `--layer` gagne en cas de conflit ; la dernière est vos données de jeu
intactes. Le montage n'a besoin que de `/dev/fuse` et de `fusermount3` (pas
d'overlayfs, pas de Wine) :

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... lisez et écrivez à travers /mnt/point ...
fusermount3 -u /mnt/point
```

Les écritures atterrissent dans `--overwrite <dir>` (un dossier temporaire si
omis), donc les couches elles-mêmes restent intactes, même ici.

#### Pourquoi le passthrough est désactivé par défaut

Le passthrough donne au noyau le vrai fichier sous-jacent, si bien que les
lectures contournent entièrement ce démon. C'est un gain de débit qui coûte la
correction ici. Mesuré en A/B sur Skyrim SE 1.6.1170, proton-cachyos 11.0, noyau
7.1.4, le même ordre de chargement de 82 plugins, la seule variable étant la
présence de la capacité sur le binaire :

| passthrough | échecs `NtCreateFile` en `STATUS_ACCESS_VIOLATION` |
|-------------|-----------------------------------------------------|
| activé      | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`     |
| désactivé   | 0                                                    |

Activé, le jeu n'ouvre aucune de ses propres archives ni aucun de ses plugins, ce
qui se manifeste en jeu par des mods qui ne sont tout simplement pas là - aucune
erreur, aucune ligne de log. Désactivé, le même ordre de chargement arrive en jeu
avec ses plugins, ses archives et ses scripts Papyrus vivants.

L'échec est invisible depuis l'intérieur du démon, et c'est ce qui l'a rendu
coûteux à trouver : notre propre `open` réussit à chaque fois et le noyau ne
refuse jamais un fichier sous-jacent (vérifié sur une session complète en échec
avec `EIDOS_FUSE_TRACE=open` : zéro `open FAILED`, zéro `passthrough refused`).
L'erreur est produite après que le démon a répondu `opened_passthrough`, donc
aucune journalisation côté démon ne peut la voir. Ce n'est pas non plus lié à
l'extension - cela touche archives et plugins pareillement, c'est-à-dire les
fichiers que le jeu garde ouverts pendant toute son exécution.

`EIDOS_FUSE_PASSTHROUGH=1` le réactive, pour mesurer ce qu'il apporte ou pour
retester le mécanisme. Les avertissements de capacité dans le lanceur et dans
l'onglet Diagnostics n'apparaissent que si vous l'avez demandé.

Pour lancer le jeu lui-même à travers Eidos, mettez son option de lancement Steam
à :

```
eidos play skyrimse -- %command%
```

Préfixez-la de `WINEDLLOVERRIDES="d3dcompiler_47=n"` si Proton a besoin du
d3dcompiler natif pour compiler les shaders ; Eidos fusionne cela avec les
surcharges de DLL qu'un mod embarque (chargeurs ENB/ReShade/`.asi`).

### L'index des couches est-il réellement utilisé ?

L'index est tout ou rien, et il se construit en silence : `LayerStack::new`
obtient soit une carte complète des couches en lecture seule, soit `None`, après
quoi chaque requête les parcourt exactement comme avant. Rien dans un journal de
session ne distingue les deux, donc une pile qui a discrètement basculé sur le
repli ressemble à une pile qui fonctionne - tout en payant l'ancien coût.

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` résout de vrais chemins avec et sans l'index et compare les
parcours de dossiers. `index_agrees` vérifie que les deux répondent LA MÊME chose,
sur chaque chemin et chaque listage d'une instance réelle. `listing_cost` mesure
ce que la carte des enfants fusionnés économise sur `readdir`.

`EIDOS_NO_INDEX=1` force le parcours, pour quand la différence entre les deux
réponses est précisément ce qu'on débogue.

## Problèmes connus

### DLSS ou la génération d'images ne fait rien, en silence

Trois causes distinctes, chacune sans le moindre message d'erreur : NVAPI non
activé dans les options de lancement, plein écran exclusif, ou une limite d'IPS
Reflex périmée. La liste complète est dans [graphics.md](graphics.md).

**Un mod qui écrit un dossier de deux façons perdait tout ce qui était sous la
seconde.** Corrigé. ext4 distingue `meshes/` de `Meshes/` ; la vue fusionnée ne
doit pas, et de vrais mods livrent les deux - XP32 Maximum Skeleton a ses
animations et son fichier de comportement FNIS sous la version capitalisée, ses
`character assets` sous l'autre.

Le résolveur prenait la correspondance de casse exacte pour chaque composant de
chemin et s'y tenait : il entrait dans `meshes/`, n'y trouvait pas la suite du
chemin, et abandonnait LA COUCHE ENTIÈRE. Tout fichier sous l'autre orthographe
était invisible pour le jeu - aucune erreur, aucun log, rien dans aucun
diagnostic. Sur une vraie instance de 50 couches, cela faisait 74 fichiers.

Un composant qui correspond est désormais un candidat, pas une décision ; la casse
exacte est toujours essayée en premier, et ce n'est que lorsque le reste échoue
en dessous que le parcours cherche des voisins équivalents à la casse près. Les
listages avaient le même défaut un dossier plus haut et lisent maintenant chaque
dossier équivalent à la casse près, par couche.

Bon à savoir pour la forme : l'index de chemins n'a jamais eu ce bug, parce qu'il
parcourt tous les dossiers qu'il trouve. Il rendait discrètement des fichiers que
le repli ne pouvait pas rendre, ce qui est à l'envers - le repli est censé être
la réponse qui ne se trompe jamais.

**LODGen de DynDOLOD meurt en laissant un journal vide.** Corrigé par `dotnet10` ;
voir [tools.md](tools.md). Le symptôme est sans ambiguïté :
`LODGen_SSE_<world>_log.txt` contenant une bannière de version, une ligne
`.NET Version:` et rien d'autre, pour chaque monde, et une boîte de dialogue
disant seulement « failed to generate object LOD for one or more worlds ». La
cause est le Mono de Wine qui répond à la place de .NET Framework, et aucune
installation de .NET Framework n'y change rien - Proton remplace `mscoree.dll`
par un lien symbolique dans son propre arbre à chaque mise à jour du préfixe.

**Wine ne pouvait pas savoir que le montage replie la casse.** Corrigé, et c'est
celui qui comptait.

Il n'existe aucune API pour « ce système de fichiers est-il insensible à la
casse » : le `get_dir_case_sensitivity` de Wine renifle donc le marqueur que
CIOPFS laisse dans les dossiers qu'il sert. Absent, Wine suppose SENSIBLE à la
casse, et chaque recherche dont l'orthographe ne correspond pas octet pour octet
se rabat sur la lecture du dossier ENTIER pour trouver une correspondance
insensible à la casse. Les jeux Bethesda demandent
`data/ccbgssse001-fish.bsa` alors que le fichier s'appelle
`ccBGSSSE001-Fish.bsa`, donc cela se déclenchait sur presque chaque asset : 4471
sondages de marqueur et 2236 relectures complètes de dossier en huit secondes, et
195796 énumérations de `Data` en quatre-vingt-dix. Skyrim SE n'atteignait jamais
son menu principal - il restait à 240 Mo résidents pendant que le démon brûlait
92 % d'un cœur.

Eidos repliait la casse dans `resolve_read` depuis le début. Tout le coût venait
de ne jamais le dire. `lookup` répond désormais `.ciopfs` ; `readdir` continue de
ne pas le lister.

Deux choses l'ont rendu fatal plutôt que simplement lent. Le coût croît avec la
taille du dossier, donc installer le contenu Anniversary (`Data` passant de 37 à
177 fichiers) a fait basculer la chose. Et `opendir` construisait avidement le
listage fusionné, ce qui est du pur gaspillage quand Wine ouvre un dossier
uniquement pour faire un `stat` sur ce marqueur - l'instantané est désormais pris
au premier `readdir`.

Après : le menu principal, 2,1 Go résidents, démon à 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` est ce qui l'a trouvé, et il est livré. Les compteurs
d'opérations disent combien ; 195796 énumérations d'un seul dossier sont
invisibles dans un total.

**Le jeu réécrivant `plugins.txt` à vide** était très probablement la même chose -
un `Data` qu'il ne pouvait pas énumérer en un temps raisonnable, d'où il concluait
qu'il n'y avait rien et sauvegardait cela. Non prouvé, et digne d'être revérifié.
Quoi qu'il en soit, la garde de capture (une capture qui vide entièrement
l'ensemble actif est refusée, quelle que soit sa taille) fait qu'elle ne peut plus
abîmer le profil.

**`FOPEN_KEEP_CACHE` est désactivé.** Corrigé, et il vaut la peine de savoir
pourquoi. Il faisait planter Skyrim SE sur un déréférencement nul quelques
secondes après le menu principal, de façon déterministe, sans aucun mod installé ;
les trois autres caches côté noyau ont été éliminés un par un par bissection et
seul celui-ci comptait. Sa perte avait été mesurée comme gratuite à l'époque, mais
cette mesure avait été prise avec `FUSE_PASSTHROUGH` actif, où le démon sert *zéro*
lecture (`EIDOS_FUSE_STATS` rapportait `read 0` pour un chargement complet) et où
le noyau mettait déjà ces pages en cache contre le fichier sous-jacent. Le
passthrough est désormais désactivé par défaut (ci-dessous), donc cet argument ne
tient plus et le vrai coût n'est pas mesuré - le plantage suffit de toute façon à
le laisser désactivé. Réactivez avec `EIDOS_FUSE_KEEP_CACHE=1` pour enquêter ; les
deux drapeaux ne sont plus liés, il peut donc maintenant être testé seul.

### Le passthrough FUSE empêche le jeu de charger le moindre contenu de mod

Corrigé en le désactivant ; `EIDOS_FUSE_PASSTHROUGH=1` le ramène. Passthrough
activé, Skyrim SE échoue à ouvrir 152 de ses propres fichiers (75 `.bsa`, 65
`.esl`, 10 `.esm`, 2 `.esp`) en `STATUS_ACCESS_VIOLATION`, contre 0 désactivé, sur
le noyau 7.1.4 - donc aucun contenu de mod ne se charge, en silence. Le noyau lève
l'erreur après que le démon a répondu `opened_passthrough`, si bien que les
journaux du démon montrent une exécution propre (zéro ouverture échouée, zéro
fichier sous-jacent refusé). La cause racine dans le chemin noyau n'est pas
établie ; l'interrupteur est conservé pour pouvoir retester, et pour que le
passthrough puisse être restreint aux seules DLL si le mappage d'images se
révélait en avoir besoin.
