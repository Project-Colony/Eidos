<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**El gestor de mods nativo de Linux que nunca toca tu juego.**

</div>

Eidos da a los juegos de Bethesda en Linux lo que Mod Organizer 2 les da en
Windows - una vista combinada de tus mods, virtual y rehecha en cada
lanzamiento - construida sobre primitivas de Linux en vez de sobre el enganche
de la API de Windows. Nada de Wine para el gestor. Ningún archivo copiado en el directorio
del juego. Ningún procedimiento de limpieza, porque no hay nada que limpiar.

```
Steam ──> eidos-gui %command% ──> [ espacio de nombres privado ]
                                  │  mods ⊕ juego  ──> lo que el juego ve
                                  └─ muere con el juego; la instalación queda intacta
```

> **Estado:** Skyrim SE se juega a través de Eidos a diario - SKSE,
> precargadores de script extender, Creation Club, órdenes de carga ordenados
> con LOOT, partidas por perfil, todo. Una familia de juegos probada en juego
> real hasta ahora; otras diez están cableadas y esperando probadores.

## Por qué Eidos

- 🔒 **Un montaje que sólo tu juego ve.** La vista combinada vive en un espacio
  de nombres de montaje privado: tu gestor de archivos, tu copia de seguridad,
  un segundo juego - ninguno de ellos lo ve, ninguno necesita permiso para él.
  Mata el juego, corta la corriente: el espacio de nombres muere con el árbol de
  procesos y tu instalación queda exactamente como estaba. No hay residuo *por
  construcción*.
- 🧾 **Una sola copia de la verdad.** Tu perfil es dueño de su lista de mods, su
  orden de plugins, sus INI y sus partidas. Los archivos de plugins y el
  directorio de partidas se montan con bind sobre las rutas propias del juego al
  lanzar, así que incluso las escrituras del propio juego aterrizan en tu perfil.
  Cambiar de perfil lo cambia todo.
- 🐧 **Totalmente sin root.** Sin ayudante setuid, sin demonio, sin
  `sudo setcap`, sin editar `/etc/fuse.conf`. Un binario, una opción de
  lanzamiento de Steam.
- 🛡️ **Salvaguardas con justificante.** Un cierre inesperado que destroza tu
  lista de plugins se señala frente a una instantánea previa a la sesión, con
  restauración en un clic. Una captura que borraría tu orden de carga se rechaza
  y dice por qué.

## Qué hace

**Mods.** Archivos simples, asistentes FOMOD, paquetes BAIN de Wrye Bash, un
selector manual para el resto - y **los mods root de forma nativa**
(precargadores de script extender, ENB, Engine Fixes), sin el complemento Root
Builder y sin copiar nada en tu instalación. Oculta archivos sueltos, agrupa con
separadores, movimientos dirigidos, notas y categorías por mod, y un importador
de perfiles MO2.

La lista es la de MO2, con sus costumbres: ocho columnas opcionales y ordenación
por cualquiera de ellas, agrupación por categoría o por origen, gestos de doble
clic, escribir para saltar, copias de seguridad por mod inertes hasta que las
restauras, y avisos consultivos para un mod cuya disposición este juego no
cargará o que se descargó para otro. Su árbol de archivos hace las operaciones
ordinarias - carpeta nueva, renombrar, borrar, abrir - y previsualiza imágenes y
texto sin lanzar nada.

**Plugins.** El orden de carga con la ordenación de LOOT integrada, los índices
de mods tal como los calcula el juego, avisos de masters ausentes, y tus DLC y
contenidos de Creation Club mostrados como las filas no gestionadas que son.

**Instancias.** Globales - gestionadas de forma centralizada bajo
`~/.local/share/eidos` - o portátiles: una carpeta autónoma donde quieras (un
segundo disco, una partición de juegos), movible y aislada, como las de MO2. Las
instancias portátiles se recuerdan entre sesiones; la GUI, el lanzamiento de
Steam y toda orden de la línea de órdenes siguen la que usaste por última vez, y
cualquier orden acepta la carpeta allí donde acepta un identificador de juego.
Los detalles en [usage.es.md](docs/guide/usage.es.md#instancias-globales-y-portátiles).

**Perfiles.** Orden de mods, estado de plugins, INI y partidas por perfil. Las
partidas se analizan, se comparan con tus plugins actuales - con un botón que
activa lo que una partida necesita - y se resincronizan para Steam Cloud después
de cada sesión.

**Nexus.** Conecta una cuenta y el botón «Mod Manager Download» del sitio
aterriza directamente en tu instancia, con comprobación de actualizaciones
frente a lo que tienes instalado, quién hizo cada mod y un enlace a su perfil. Un
enlace de **colección** lista sus miembros cruzados con tu instancia -
instalados, descargados, ausentes - lo que es leer una colección en vez de
instalarla, y el panel dice por qué. La pestaña Downloads es una biblioteca de
archivos: filtrar, ordenar, ocultar sin borrar, y purgar los que ya están
instalados. Un interruptor **sin conexión** lo detiene todo.

**Herramientas.** xEdit, BodySlide, DynDOLOD y compañía se ejecutan *a través de
la vista combinada* dentro del prefijo Proton del juego - ven tus mods, su
salida aterriza en Overwrite, y un clic la convierte en un mod real. El runtime
que cada una necesita se obtiene a petición, así que una DLL ausente es un botón
y no una tarde entera. xEdit y su gemelo QuickAutoClean se encuentran por ti - en
la carpeta del juego, dentro de un mod, o en el directorio de herramientas que
guardas junto a tus juegos - con los runtimes correctos ya elegidos. Fija las que
usas, oculta las que no, da a una herramienta su propio
AppID de Steam cuando es su propia aplicación de Steam, y escribe un acceso
directo `.desktop` que la lance a través de la vista combinada sin abrir Eidos en
absoluto.

**Diagnóstico.** Masters ausentes, archivos huérfanos, deriva de la lista de
mods, conjuntos de plugins dañados - y, tras una ejecución, lo que el propio
registro del script extender dice que se cargó realmente.

**Dónde guarda sus propios archivos.** `~/.config/Colony/Eidos/` para lo que has
elegido - preferencias, tu sesión de Nexus, tu lista de instancias, las
definiciones de juegos y de extensiones que hayas escrito - con los registros
bajo `~/.local/state/Colony/Eidos/`. La disposición que usa todo programa de la
familia Colony. Un Eidos más antiguo los guardaba en `~/.config/eidos/`; el
primer lanzamiento tras actualizar los copia, lo dice en el registro, y deja el
directorio antiguo exactamente como estaba.

## Cómo se compara

| | Eidos | MO2 con Wine | Fluorine-Manager | Limo / desplegadores por enlaces |
|---|---|---|---|---|
| El gestor se ejecuta de forma nativa | ✅ | ❌ aplicación Windows en Wine | ✅ (port a Qt) | ✅ |
| Directorio del juego intacto | ✅ siempre | ✅ | ✅ | ❌ se escriben enlaces dentro |
| El montaje lo ve | sólo el juego | sólo el juego | **todo el sistema** | n/a |
| Limpieza necesaria tras un cierre inesperado | ninguna, por diseño | ninguna | recuperación de montajes obsoletos | retirada manual |
| Mods root (ENB, precargadores) | ✅ nativo | requiere complemento | requiere complemento | parcial |
| Privilegios necesarios | ninguno | ninguno | editar `/etc/fuse.conf` | ninguno |

## Su rapidez

| | antes | ahora |
|---|---|---|
| cargar una partida | ~20 segundos | **6-7 segundos** |
| lecturas de directorio en una sesión | 5,6 millones | 465 mil |

Los cambios de celda son inmediatos. La ganancia vino de hacerles menos
preguntas a tus mods: encontrar un archivo interrogaba antes a los cincuenta por
turno, y listar una carpeta lo hacía cincuenta veces seguidas. Ninguna de las dos
cosas lo hace ya. Medido en una instancia real jugada con normalidad, no en un
banco de pruebas.

## Empezar

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Luego pon la opción de lanzamiento de Steam de tu juego a
`~/.local/bin/eidos-gui %command%` y pulsa Jugar.

Paquetes de Arch y archivos de versión, lo que necesitas instalado primero, y la
vía de la línea de órdenes: **[docs/guide/install.es.md](docs/guide/install.es.md)**.

## Opciones de lanzamiento de Steam

La línea base es todo lo que necesitan la mayoría de las instalaciones:

```
~/.local/bin/eidos-gui %command%
```

Todo lo demás son variables de entorno apiladas por delante, y se combinan
libremente:

| Quieres... | Pon delante |
|---|---|
| DLSS con Community Shaders | `PROTON_ENABLE_NVAPI=1` - sin ella DLSS nunca se inicializa, en silencio; la lista completa está en [guide/graphics.es.md](docs/guide/graphics.es.md) |
| un contador de FPS en pantalla | `DXVK_HUD=fps` |
| interpolación de fotogramas a nivel de controlador, cero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - nunca junto con la generación de fotogramas de Community Shaders |
| registros detallados para un informe de fallo | `EIDOS_LOG=debug` (los registros de sesión aterrizan en `~/.local/state/Colony/Eidos/logs/`) |
| un informe de E/S por sesión desde el montaje | `EIDOS_FUSE_STATS=1` |
| otro número de hilos de FUSE | `EIDOS_FUSE_THREADS=8` (4 por defecto; `1` es lo primero que probar cuando persigues un fallo de concurrencia) |
| este lanzamiento fijado a una instancia portátil | `EIDOS_INSTANCE=/path/to/folder` - sin ella Eidos abre la instancia que usaste por última vez, que suele ser lo que quieres |

La línea que conservar para una instalación moderna con mods (Community Shaders,
DLSS, generación de fotogramas) - esta es la orden final, no un ejemplo:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Añade `DXVK_HUD=fps` delante mientras compruebas que la instalación funciona, y
quítalo cuando lo haga.

Los interruptores de diagnóstico más profundos (`EIDOS_FUSE_TRACE`, los
conmutadores de bisección de caché e índice, por qué `EIDOS_FUSE_PASSTHROUGH`
está desactivado por defecto) están en
[guide/troubleshooting.es.md](docs/guide/troubleshooting.es.md).

## A dónde ir ahora

| Si quieres... | |
|---|---|
| instalarlo | [guide/install.es.md](docs/guide/install.es.md) |
| aprender la línea de órdenes y la GUI | [guide/usage.es.md](docs/guide/usage.es.md) |
| configurar xEdit, BodySlide o DynDOLOD | [guide/tools.es.md](docs/guide/tools.es.md) |
| jugar a Fallout 4 (F4SE, versiones, el cierre por restos en NVIDIA) | [guide/fallout4.es.md](docs/guide/fallout4.es.md) |
| hacer funcionar DLSS / la generación de fotogramas (Community Shaders) | [guide/graphics.es.md](docs/guide/graphics.es.md) |
| arreglar algo que parece ir mal | [guide/troubleshooting.es.md](docs/guide/troubleshooting.es.md) |
| saber por qué es rápido, y comprobarlo tú mismo | [internals/performance.md](docs/internals/performance.md) |
| entender cómo funciona por dentro | [internals/architecture.md](docs/internals/architecture.md) |
| compilarlo, probarlo, contribuir | [internals/contributing.md](docs/internals/contributing.md) |
| saber por qué existe siquiera | [project/landscape.md](docs/project/landscape.md) |

El índice completo está en [docs/README.es.md](docs/README.es.md); la política de
seguridad y cómo informar de una vulnerabilidad, en [SECURITY.md](SECURITY.md).

## Idioma

Las páginas que un jugador necesita están traducidas. **El inglés es canónico**:
cuando una traducción no concuerda con él, el archivo en inglés es el que tiene
razón.

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**Todo lo demás está en inglés a propósito, no por omisión.** `docs/internals/` y
`docs/project/` los leen personas que además leen el Rust, y `CHANGELOG.md` se
genera. Traducirlos serían 17.678 palabras más que mantener honestas para un
público que no las necesita.

Cada traducción lleva el hash del archivo en inglés del que se hizo, y CI falla
cuando el inglés avanza - ver [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
Una traducción que no se puede volver a poner al día se **borra**, no se deja
donde está: una página caduca sigue pareciendo autoritativa y reparte las órdenes
del mes pasado, lo que para el lector es peor que ser enviado al inglés.

Añadir un idioma son cuatro archivos y una fila en esta tabla;
[`docs/internals/contributing.md`](docs/internals/contributing.md) tiene los pasos.

## Juegos compatibles

**Skyrim SE/AE** - probado en juego real. **Fallout 4** también está cableado de
punta a punta (F4SE sustituido automáticamente, invalidación de archivos, orden
de carga con asteriscos, LOOT, partidas `.fos`) - ver
[guide/fallout4.es.md](docs/guide/fallout4.es.md). Cableados según el descriptor
de juego compartido y a la espera de probadores: Skyrim LE, Skyrim VR, Enderal
SE, Fallout 3, Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion y Morrowind
(estos dos últimos montan y gestionan mods; sus listas de plugins ordenadas por
marca de tiempo todavía no se gestionan).

Añadir una familia es una fila de descriptor:
[internals/adding-games.md](docs/internals/adding-games.md).

## Trabajos previos y agradecimientos

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) y
  [usvfs](https://github.com/ModOrganizer2/usvfs) - la semántica que Eidos
  reproduce, y el código con el que se estudió su paridad
- [LOOT](https://loot.github.io/) - el motor de ordenación, vía libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) y los demás gestores de Linux -
  prueba de que hay una comunidad que quiere esto resuelto

## Licencia

GPL-3.0-or-later. La gestión de mods es de todos.
