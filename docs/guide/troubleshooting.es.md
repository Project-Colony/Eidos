<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

# Solución de problemas y diagnóstico

Todo para el día en que el juego ve algo con lo que el sistema de archivos no está
de acuerdo: los interruptores de entorno, cómo leer los contadores de operaciones,
los problemas conocidos con su historia, y el asunto del passthrough.

### Diagnosticar el VFS

Existen dos variables de entorno para cuando el juego ve algo con lo que el sistema
de archivos no está de acuerdo:

```sh
EIDOS_FUSE_STATS=1                  # contadores de operaciones, volcados al desmontar
EIDOS_FUSE_NO_CACHE=1               # todas las cachés del lado del núcleo apagadas
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # o nómbralas una a una
```

La forma granular es lo que encontró el cierre descrito más abajo: apagar las cuatro
responde «¿es la caché?», y sólo nombrarlas responde «¿cuál». Los contadores
responden a la otra mitad: una carga que muestra `read 0` es una en la que
`FUSE_PASSTHROUGH` sirvió cada byte dentro del núcleo, así que todo lo que ibas a
afinar en la ruta de lectura ya es gratis.

## Montar una unión a mano

La primera `--layer` gana en caso de conflicto; la última son tus datos de juego
intactos. El montaje sólo necesita `/dev/fuse` y `fusermount3` (sin overlayfs, sin
Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... lee y escribe a través de /mnt/point ...
fusermount3 -u /mnt/point
```

Las escrituras aterrizan en `--overwrite <dir>` (un directorio temporal si se
omite), de modo que las propias capas siguen intactas incluso aquí.

#### Por qué el passthrough está desactivado por defecto

El passthrough entrega al núcleo el archivo de respaldo real, de forma que las
lecturas se saltan este demonio por completo. Es una ganancia de rendimiento que
aquí cuesta corrección. Medido en A/B sobre Skyrim SE 1.6.1170, proton-cachyos 11.0,
núcleo 7.1.4, el mismo orden de carga de 82 complementos, con la única variable de
si el binario llevaba la capacidad:

| passthrough | fallos de `NtCreateFile` con `STATUS_ACCESS_VIOLATION` |
|-------------|---------------------------------------------------------|
| activado    | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| desactivado | 0                                                       |

Con él activado el juego no abre ninguno de sus propios archivos ni complementos, lo
que en el juego se manifiesta como mods que simplemente no están - sin error, sin
línea de registro. Desactivado, el mismo orden de carga llega a la partida con sus
complementos, archivos y guiones Papyrus vivos.

El fallo es invisible desde dentro del demonio, que es lo que lo hizo caro de
encontrar: nuestro propio `open` tiene éxito siempre y el núcleo nunca rechaza un
archivo de respaldo (verificado a lo largo de una sesión completa fallida con
`EIDOS_FUSE_TRACE=open`: cero `open FAILED`, cero `passthrough refused`). El error
se produce después de que el demonio haya respondido `opened_passthrough`, así que
ningún registro del lado del demonio puede verlo. Tampoco depende de la extensión:
golpea archivos y complementos por igual, es decir, los ficheros que el juego
mantiene abiertos durante toda su ejecución.

`EIDOS_FUSE_PASSTHROUGH=1` lo vuelve a activar, para medir lo que aporta o para
volver a probar el mecanismo. Los avisos de capacidad en el lanzador y en la pestaña
Diagnostics sólo aparecen cuando lo has pedido.

Para lanzar el propio juego a través de Eidos, pon su opción de lanzamiento de Steam
en:

```
eidos play skyrimse -- %command%
```

Antepón `WINEDLLOVERRIDES="d3dcompiler_47=n"` si Proton necesita el d3dcompiler
nativo para compilar sombreadores; Eidos lo fusiona con cualquier anulación de DLL
que traiga un mod (cargadores ENB/ReShade/`.asi`).

### ¿Se está usando de verdad el índice de capas?

El índice es todo o nada y se construye en silencio: `LayerStack::new` obtiene o
bien un mapa completo de las capas de sólo lectura, o bien `None`, tras lo cual cada
consulta las recorre exactamente como antes. Nada en el registro de una sesión
distingue ambos casos, así que una pila que cayó calladamente al recorrido parece
idéntica a una que funciona - mientras paga el coste antiguo.

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` resuelve rutas reales con y sin el índice y compara los recorridos de
directorio. `index_agrees` comprueba que ambos respondan LO MISMO, en cada ruta y
cada listado de una instancia real. `listing_cost` mide lo que el mapa de hijos
combinados ahorra en `readdir`.

`EIDOS_NO_INDEX=1` fuerza el recorrido, para cuando lo que se depura es justamente
la diferencia entre las dos respuestas.

## Problemas conocidos

### DLSS o la generación de fotogramas no hace nada, en silencio

Tres causas distintas, cada una sin mensaje de error alguno: NVAPI no habilitado en
las opciones de lanzamiento, pantalla completa exclusiva, o un tope de FPS de Reflex
obsoleto. La lista completa está en [graphics.es.md](graphics.es.md).

**Un mod que escribe un directorio de dos maneras perdía todo lo que había bajo la
segunda.** Corregido. ext4 distingue `meshes/` de `Meshes/`; la vista combinada no
debe hacerlo, y hay mods reales que traen ambas - XP32 Maximum Skeleton tiene sus
animaciones y su archivo de comportamiento FNIS bajo la versión con mayúscula, y sus
`character assets` bajo la otra.

El resolutor tomaba la coincidencia exacta de mayúsculas para cada componente de la
ruta y se comprometía con ella: entraba en `meshes/`, no encontraba allí el resto de
la ruta y abandonaba LA CAPA ENTERA. Todo archivo bajo la otra grafía era invisible
para el juego - sin error, sin registro, sin nada en ningún diagnóstico. En una
instancia real de 50 capas fueron 74 archivos.

Un componente que coincide es ahora un candidato, no una decisión; la coincidencia
exacta se sigue probando primero, y sólo cuando el resto falla debajo el recorrido
busca hermanos equivalentes ignorando mayúsculas. Los listados tenían el mismo fallo
un directorio más arriba y ahora leen cada directorio equivalente por capa.

**El LODGen de DynDOLOD muere dejando un registro vacío.** Corregido con `dotnet10`;
ver [tools.md](tools.md). El síntoma es inconfundible:
`LODGen_SSE_<world>_log.txt` con un cartel de versión, una línea `.NET Version:` y
nada más, para cada mundo, y un diálogo que sólo dice «failed to generate object LOD
for one or more worlds». La causa es el Mono de Wine respondiendo por .NET
Framework, y ninguna cantidad de .NET Framework instalado lo arregla: Proton
reemplaza `mscoree.dll` por un enlace simbólico a su propio árbol en cada
actualización del prefijo.

**Wine no podía saber que el montaje pliega mayúsculas y minúsculas.** Corregido, y
era el que importaba.

No existe API para «¿este sistema de archivos distingue mayúsculas?», así que
`get_dir_case_sensitivity` de Wine olfatea el marcador que CIOPFS deja en los
directorios que sirve. Si falta, Wine supone que SÍ distingue, y toda búsqueda cuya
grafía no coincida byte a byte recae en leer el directorio ENTERO para encontrar una
coincidencia sin distinción de mayúsculas. Los juegos de Bethesda piden
`data/ccbgssse001-fish.bsa` cuando el archivo es `ccBGSSSE001-Fish.bsa`, así que
saltaba en casi todos los recursos: 4471 sondeos del marcador y 2236 relecturas
completas de directorio en ocho segundos, y 195796 enumeraciones de `Data` en
noventa. Skyrim SE nunca llegaba a su menú principal: se quedaba en 240 MB
residentes mientras el demonio quemaba el 92 % de un núcleo.

Eidos plegaba mayúsculas en `resolve_read` desde el principio. Todo el coste venía
de no decirlo nunca. Ahora `lookup` responde `.ciopfs`; `readdir` sigue sin
listarlo.

Dos cosas lo volvieron mortal en lugar de meramente lento. El coste escala con el
tamaño del directorio, así que instalar el contenido Anniversary (`Data` de 37 a 177
archivos) lo desbordó. Y `opendir` construía con avidez el listado combinado, lo que
es puro desperdicio cuando Wine abre un directorio sólo para hacer `stat` a ese
marcador dentro - la instantánea se toma ahora en el primer `readdir`.

Después: el menú principal, 2,1 GB residentes, demonio al 0 % de CPU.

`EIDOS_FUSE_TRACE=opendir` es lo que lo encontró, y se distribuye. Los contadores de
operaciones dicen cuántas; 195796 enumeraciones de un solo directorio son invisibles
dentro de un total.

**Que el juego reescribiera `plugins.txt` vacío** era muy probablemente lo mismo: un
`Data` que no podía enumerar en un tiempo razonable, de donde concluía que allí no
había nada y guardaba eso. No demostrado, y merece revisarse. En cualquier caso, la
guarda de captura (una captura que vacía por completo el conjunto activo se rechaza
sea cual sea su tamaño) significa que ya no puede dañar el perfil.

**`FOPEN_KEEP_CACHE` está apagado.** Corregido, y vale la pena saber por qué. Hacía
que Skyrim SE se cerrara por desreferencia nula segundos después del menú principal,
de forma determinista, sin un solo mod instalado; las otras tres cachés del lado del
núcleo se descartaron una a una por bisección y sólo ésta importaba. Su pérdida se
midió entonces como gratuita, pero esa medición se hizo con `FUSE_PASSTHROUGH`
activo, donde el demonio sirve *cero* lecturas (`EIDOS_FUSE_STATS` informó `read 0`
en una carga completa) y el núcleo ya cacheaba esas páginas contra el archivo de
respaldo. El passthrough está ahora apagado por defecto (más abajo), así que aquel
argumento ya no vale y el coste real está sin medir - el cierre basta de todos modos
para dejarlo apagado. Reactívalo con `EIDOS_FUSE_KEEP_CACHE=1` para investigar; las
dos banderas ya no están enredadas, así que ahora puede probarse por sí sola.

### El passthrough de FUSE impide que el juego cargue contenido de mods

Corregido apagándolo; `EIDOS_FUSE_PASSTHROUGH=1` lo devuelve. Con el passthrough
activo, Skyrim SE falla al abrir 152 de sus propios archivos (75 `.bsa`, 65 `.esl`,
10 `.esm`, 2 `.esp`) con `STATUS_ACCESS_VIOLATION`, frente a 0 con él apagado, en el
núcleo 7.1.4 - de modo que no se carga ningún contenido de mods, en silencio. El
núcleo levanta el error después de que el demonio haya respondido
`opened_passthrough`, así que los propios registros del demonio muestran una
ejecución limpia (cero aperturas fallidas, cero archivos de respaldo rechazados). La
causa raíz en la ruta del núcleo no está establecida; el interruptor se conserva
para poder volver a probarlo, y para que el passthrough pudiera limitarse sólo a
DLL si resultara que el mapeo de imágenes lo necesita.
