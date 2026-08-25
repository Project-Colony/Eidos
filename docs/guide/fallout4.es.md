<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 a través de Eidos

Fallout 4 no necesita ninguna opción de lanzamiento especial, ningún ejecutable
renombrado y ningún script envoltorio. Vale la pena decirlo con claridad, porque
todas las demás guías de Linux para F4SE dicen lo contrario - y sus consejos se
rompen en la siguiente actualización de Steam.

## La opción de lanzamiento

```
~/.local/bin/eidos-gui %command%
```

El objetivo de lanzamiento de Steam para Fallout 4 es `Fallout4Launcher.exe`, nunca
`Fallout4.exe`, así que conseguir que el script extender siquiera se ejecute es en
realidad la pregunta «cómo hago que Steam arranque otro programa». Las respuestas
habituales reescriben `%command%` en bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

o copian `f4se_loader.exe` sobre `Fallout4Launcher.exe`, que Steam restaura
calladamente en cada actualización del juego - tras lo cual estás jugando sin F4SE y
nada lo dice.

Eidos hace el cambio él mismo, a partir del descriptor del juego: sustituye el
lanzador por `f4se_loader.exe` cuando hay uno instalado, recae en `Fallout4.exe`
cuando no lo hay, y **te avisa** cuando ha tenido que recaer. Un juego que arranca
con todos los mods de F4SE inertes es peor que un juego que no arranca.

Hay una segunda razón para no ejecutar nunca el lanzador: reexamina `Data` y
reescribe `plugins.txt`, deshaciendo el orden de carga recién desplegado. Eidos no
lo ejecuta jamás.

## De qué se encarga Eidos por ti

| | |
|---|---|
| Invalidación de archivos | `Fallout4Custom.ini` recibe `[Archive]` `bInvalidateOlderFiles=1` y un `sResourceDataDirsFinal=` vacío, las dos claves que permiten que los archivos sueltos fuera de `Data` se vean siquiera. Escrito en el perfil, no en la carpeta del juego. |
| Orden de carga | `plugins.txt` en el formato de asterisco que usa Fallout 4 (`*` marca activo), con `Fallout4.ccc` respetado para los complementos implícitos de Creation Club |
| LOOT | La ordenación funciona igual que en Skyrim - `eidos sort <instance>` obtiene la masterlist de `fallout4` |
| Partidas | Las partidas `.fos` y sus cosaves `.f4se` se listan, copian y guardan por perfil; el panel de detalle lee la tabla de complementos de la propia partida, así que una partida que necesita un complemento que has desactivado lo dice antes de cargarla |
| Mods root | Todo lo que un mod trae junto al ejecutable (el propio F4SE, ENB, un `dxvk.conf`) aterriza ahí por el mismo mecanismo `Root/` que usa Skyrim |

## La cuestión de las versiones

Fallout 4 ya no es el juego congelado que fue entre 2019 y 2024. A agosto de 2026
hay tres ramas vivas, y una DLL de mod compilada para una no cargará en otra:

| Rama | Versión | F4SE |
|---|---|---|
| Clásica («old-gen») | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Dos consecuencias que conviene conocer antes de montar una lista de mods:

- **Comprueba qué tienes realmente.** Carpetas `Creations/` y `Mods/` en la raíz del
  juego significan que estás en la línea 1.11.x. El panel de detalle de una partida
  en Eidos también muestra la compilación que la escribió - Fallout lo graba en la
  partida, y Eidos lo saca como «Game build».
- **Un parche recién salido no es buen día para empezar.** F4SE suele aparecer uno o
  dos días después de una actualización de Bethesda, pero *Address Library for F4SE
  Plugins* - por donde la mayoría de mods DLL resuelven sus desplazamientos - va a
  su propio ritmo. Entre ambos, la mitad DLL del ecosistema está caída. Los mods sin
  DLL (texturas, mallas, complementos) no se ven afectados.

Cuando tu montaje funcione, desactiva las actualizaciones automáticas de Steam para
Fallout 4 (Propiedades → Actualizaciones → «Actualizar este juego sólo al
iniciarlo»), o el siguiente parche romperá todas las DLL que instalaste.

## Nota de hardware: los restos de armas provocan cierres en NVIDIA

El efecto de restos de armas de Fallout 4 corre sobre NVIDIA FleX, un derivado de
PhysX que NVIDIA dejó de mantener tras la generación Pascal. En cualquier tarjeta
Turing o posterior - GTX 16, RTX 20 hasta RTX 50 - cierra el juego. Es un fallo del
juego, nada que ver con Linux, Proton ni Eidos.

Dos arreglos, cualquiera sirve: desactiva «Weapon Debris» en los ajustes del juego,
o instala *Weapon Debris Crash Fix* (Nexus 48078), que desactiva la colisión de los
fragmentos en lugar del efecto.

## Si algo parece ir mal

La lista general está en [troubleshooting.es.md](troubleshooting.es.md); la primera
pregunta propia de Fallout es siempre *qué ejecutable arrancó realmente*. Eidos
escribe la orden de lanzamiento completa en el registro de ejecución de la
instancia, así que:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Si nombra `f4se_loader.exe`, el cambio ocurrió. Si nombra `Fallout4Launcher.exe`,
F4SE no está instalado donde Eidos pueda encontrarlo - le corresponde estar junto al
ejecutable del juego, lo que en un montaje gestionado significa el directorio
`Root/` de un mod (o la propia carpeta del juego, instalado a mano).
