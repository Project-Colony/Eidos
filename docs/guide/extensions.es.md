<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# Extensiones

Una extensión añade una entrada a Eidos sin formar parte de Eidos. Es un
manifiesto TOML que nombra un programa y, como mucho, ese programa.

Los manifiestos viven en `~/.config/Colony/Eidos/addons/`, un `.toml` por
extensión. Abre la carpeta desde **View -> Extensions -> Open folder** y pulsa
**Reload** - sin reiniciar.

## Por qué no se carga nada dentro de Eidos

Mod Organizer 2 carga sus complementos como bibliotecas compartidas y aloja los de
Python mediante Qt. Ninguna de las dos cosas se traslada. Rust no tiene una ABI
estable, así que una biblioteca compartida construida con otro compilador - u otra
bandera de optimización, u otro conjunto de características de una dependencia
común - es comportamiento indefinido, no un desajuste de versiones. Y los widgets
de Eidos son genéricos en tiempo de compilación, de modo que una biblioteca no
podría construir uno para devolverlo aunque la ABI fuese estable.

Así que una extensión es un programa que Eidos *ejecuta*. No puede tirar la
ventana, no puede corromper una lista de mods, y sigue funcionando a través de las
actualizaciones de Eidos.

## Una herramienta

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # omitir para todos los juegos
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

Aparece en **View -> Extensions** con un botón Run y arranca desacoplada - Eidos
no la espera.

## Una comprobación

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

Se ejecuta en cada refresco e imprime un hallazgo por línea:

```
level<TAB>title<TAB>detail
```

donde `level` es `problem`, `advice` u `ok`. El detalle es opcional. Todo lo que
no empiece por un nivel conocido se ignora, de modo que la salida de progreso y
los avisos sueltos no pueden levantar una fila que parezca una de las
comprobaciones propias de Eidos. Los hallazgos aparecen en la pestaña **Health**,
con el nombre de la extensión como prefijo.

Una comprobación dispone de tres segundos. La que se pasa se detiene y se informa
como un problema contra sí misma - se ejecuta en el mismo refresco que sigue a
cada clic, así que una colgada congelaría la ventana.

## Marcadores

Tanto `args` como `workdir` expanden estos:

| Marcador        | Qué es                                       |
| --------------- | -------------------------------------------- |
| `{instance}`    | la raíz de la instancia                      |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | el nombre del perfil activo                  |
| `{profile_dir}` | el directorio del perfil activo              |
| `{game}`        | el identificador del juego, p. ej. `skyrimse`|
| `{game_name}`   | el nombre visible del juego                  |
| `{install}`     | el directorio de instalación del juego       |
| `{data}`        | el directorio `Data` del juego               |

Un marcador desconocido se deja exactamente como se escribió en lugar de vaciarse,
para que un error falle a la vista en vez de convertir `--out {typo}` en
`--out --next-flag`. Ejecutar una herramienta cuyos marcadores no se resuelven
todos se rechaza, y Eidos dice cuáles faltan.

## Lo que una extensión no puede hacer

Recibe valores y se ejecuta; no puede llamar de vuelta a Eidos, cambiar la lista
de mods ni dibujar nada en la ventana. Es deliberado. Aquello para lo que MO2 usa
complementos y que SÍ necesita llegar dentro - soporte de juegos, instaladores, el
motor de conflictos - aquí está integrado en vez de atornillado: una definición de
juego es su propio TOML en `~/.config/Colony/Eidos/games/`, y los instaladores
FOMOD y BAIN son nativos.
