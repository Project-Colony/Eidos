<!-- eidos-i18n: source=docs/guide/install.md sha=40f02a87874c44292179b6dff322d0f623c307ff -->

# 安装 Eidos

共有四种安装方式。Colony 只安装一个文件 `eidos-linux`,它就是整个程序;其他方式则并排提供 `eidos`（命令行）、`eidos-gui` 和静态 NIF 预览辅助程序，以及 `nxm://` 处理器，让 Nexus 上的“Mod Manager Download”按钮将下载内容送到你的实例。

## 你需要先具备

| | |
|---|---|
| **带 FUSE 的 Linux** | PATH 中要有 `fusermount3`。任何当前发行版都自带。 |
| **一款用 Proton 启动过一次的游戏** | Steam 只在首次启动时创建游戏的 Wine 前缀,而 Eidos 在其中工作。 |
| **`7z`** | 用于安装模组压缩包。多数发行版里叫 `p7zip`。 |

不需要 root,不需要守护进程,不需要改 `/etc/fuse.conf`,也不需要把你加进任何用户组。
Eidos 挂载在属于游戏进程的私有命名空间里。

## 通过 Colony

在 [Colony](https://github.com/Project-Colony/Colony) 里搜索 **Eidos** 并安装;
更新通过启动器送达。Colony 只保留一个文件,
`~/.local/share/Colony/apps/Eidos/eidos-linux`。运行它,窗口就会打开。
通过名为 `eidos` 的链接调用时,同一个文件就是命令行。在你的 `PATH` 上放一个这样的
链接,并注册 Nexus 下载处理器:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # 网站上的 Mod Manager Download 按钮现在会交给 Eidos
```

窗口本身不需要这个链接:每当它启动游戏或工具时,都会把自己的文件当作命令行运行。
NIF 模型预览需要 `eidos-nif-preview`,只有发行版捆绑包和源码构建才带有它;没有它时,
预览会如实说明,其他一切不受影响。让 Steam 直接指向这个文件本身,如下文所示。

## 发行版捆绑包

从[最新发行版](https://github.com/Project-Colony/Eidos/releases/latest)下载
`eidos-linux-bundle.tar.gz` 及其 `.sig`、`.meta` 和 `.meta.sig`,并按照 README 的
[代码签名策略](../../README.md#代码签名策略)所示校验它们。然后:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

默认装到 `~/.local/bin`。`--system` 装到 `/usr/local/bin`,`--bindir DIR` 装到别处。
重新运行它就是受支持的升级方式。
捆绑包和 `eidos-linux` 需要 x86_64 Linux,且 glibc 为 2.39 或更新版本。

## Arch

```bash
cd packaging && makepkg -si
```

这个 PKGBUILD 会从源码构建当前的 `main`。

## 从源码构建

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

从源码构建需要 Rust 1.89 或更新版本、`just`、CMake 3.20 或更高版本、C++17 编译器，以及用于原生辅助程序测试的 Python 3。不使用 `just` 时，请参阅[原生辅助程序说明](../../../../../crates/eidos-nif-preview/README.md)，使用 `cargo build --release --locked` 构建 Rust 程序，将辅助程序复制到 `target/release`，然后运行 `packaging/install.sh --from target/release`。

## 然后:让 Steam 指向它

Eidos 是*作为*你游戏的启动命令运行的,这正是它能在游戏启动前完成挂载的原因。
在 Steam 里右键点游戏 -> 属性 -> 启动选项:

```
~/.local/bin/eidos-gui %command%
```

若通过 Colony 安装,路径是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。按下开始游戏。Eidos 会在该游戏的实例上打开;安装模组、用 LOOT 排序、点 Run。
退出时挂载随之消失,你的安装目录与原先分毫不差。

请使用绝对路径 - Steam 不读取你 shell 的 `PATH`。

### 如果你更喜欢终端

```sh
eidos init skyrimse               # 创建实例(给出文件夹即为便携实例)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 模组
eidos sort skyrimse               # 用 LOOT 排序加载顺序
eidos play skyrimse -- %command%  # 让任何程序穿过合并视图运行
```

凡是接受游戏 id 的命令,同样接受便携实例的文件夹 -
见 [usage.zh-CN.md](usage.md#实例全局与便携)。完整导览在
[usage.zh-CN.md](usage.md)。

## 可选:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 仅授予可选权限，不会开启 passthrough。
它应当加在捆绑包或源码构建中单独的 `eidos` 二进制文件上,绝不能加在 `eidos-linux` 上,
因为后者同时也是窗口([SECURITY.md](../../../../../SECURITY.md))。
运行时需另行设置 `EIDOS_FUSE_PASSTHROUGH=1` 才会启用。
它**默认关闭,而且你几乎肯定应该让它保持关闭**:在 Skyrim SE 上实测,它会让游戏
打不开自己的档案与插件,于是模组静悄悄地不加载。这个开关的存在是为了重新测试该
机制,而不是因为推荐使用。

细节以及支撑该决定的实测数据,见
[troubleshooting.zh-CN.md](troubleshooting.md)。

## 已经出问题了?

[troubleshooting.zh-CN.md](troubleshooting.md) 讲了环境开关、如何读操作
计数器,以及迄今为止咬过人的每一个问题。
