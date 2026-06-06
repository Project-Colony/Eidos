from lzokay._lzokay import compress, compress_worst_size, decompress

from . import version

__all__ = [
    "decompress",
    "compress",
    "compress_worst_size",
    "VERSION",
    "__version__",
]


__version__ = version.version
VERSION = version.version
