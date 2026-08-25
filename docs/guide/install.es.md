<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# Instalar Eidos

Tres formas de entrar. Todas dan los mismos dos binarios - `eidos` (la línea de
órdenes) y `eidos-gui` - más el manejador `nxm://` que hace que el botón «Mod
Manager Download» de Nexus aterrice en tu instancia.

## Lo que necesitas primero

| | |
|---|---|
| **Linux con FUSE** | `fusermount3` en tu PATH. Toda distribución actual lo incluye. |
| **Un juego con Proton, lanzado una vez** | Steam sólo crea el prefijo Wine del juego en el primer lanzamiento, y Eidos trabaja dentro de él. |
| **`7z`** | Para instalar archivos de mods. `p7zip` en la mayoría de distribuciones. |

Sin root, sin demonio, sin editar `/etc/fuse.conf` y sin nada que añadir a tus
grupos. Eidos monta dentro de un espacio de nombres privado que pertenece al
proceso del juego.

## Arch

```bash
cd packaging && makepkg -si
```

## Un archivo de versión

```bash
./install.sh
```

Instala en `~/.local/bin` por defecto. `--system` lo pone en `/usr/local/bin`,
`--bindir DIR` en cualquier otro sitio. Volver a ejecutarlo es la forma prevista
de actualizar.

## Desde el código fuente

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Después: apuntar Steam hacia él

Eidos se ejecuta *como* la orden de lanzamiento de tu juego, que es como llega a
montar antes de que el juego arranque. En Steam, clic derecho en el juego ->
Propiedades -> Opciones de lanzamiento:

```
~/.local/bin/eidos-gui %command%
```

Pulsa Jugar. Eidos se abre en la instancia de ese juego; instala mods, ordena con
LOOT, pulsa Run. Al salir, el montaje se va con él y tu instalación queda
exactamente como estaba.

Usa la ruta absoluta - Steam no lee el `PATH` de tu shell.

### Si prefieres la terminal

```sh
eidos init skyrimse               # crear una instancia (añade una carpeta para hacerla portátil)
eidos install skyrimse mod.7z     # mods Simple / FOMOD / BAIN / root
eidos sort skyrimse               # ordenar la carga con LOOT
eidos play skyrimse -- %command%  # ejecutar lo que sea a través de la vista combinada
```

Toda orden que acepta un identificador de juego acepta también la carpeta de una
instancia portátil - ver [usage.es.md](usage.es.md). El recorrido completo está
ahí.

## Opcional: passthrough de FUSE

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` activa el passthrough FUSE
del núcleo. Está **desactivado por defecto y casi con seguridad lo quieres así**:
medido en Skyrim SE, impide que el juego abra sus propios archivos y complementos,
de modo que los mods no se cargan, en silencio. El interruptor existe para volver
a probar el mecanismo, no porque se recomiende.

Los detalles, y las mediciones detrás de esa decisión, en
[troubleshooting.es.md](troubleshooting.es.md).

## ¿Algo va mal ya?

[troubleshooting.es.md](troubleshooting.es.md) cubre los interruptores de
entorno, cómo leer los contadores de operaciones y todos los problemas que han
mordido a alguien hasta ahora.
