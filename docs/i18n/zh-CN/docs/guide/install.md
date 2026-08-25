<!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b21c7e98ce19d35d4780b65daef317b4a -->

# 安装 Eidos

三条路。它们都会给你同样的两个可执行文件 - `eidos`(命令行)与 `eidos-gui` -
以及 `nxm://` 处理器,让 Nexus 上的 "Mod Manager Download" 按钮直接落进你的实例。

## 你需要先具备

| | |
|---|---|
| **带 FUSE 的 Linux** | PATH 中要有 `fusermount3`。任何当前发行版都自带。 |
| **一款用 Proton 启动过一次的游戏** | Steam 只在首次启动时创建游戏的 Wine 前缀,而 Eidos 在其中工作。 |
| **`7z`** | 用于安装模组压缩包。多数发行版里叫 `p7zip`。 |

不需要 root,不需要守护进程,不需要改 `/etc/fuse.conf`,也不需要把你加进任何用户组。
Eidos 挂载在属于游戏进程的私有命名空间里。

## Arch

```bash
cd packaging && makepkg -si
```

## 发行版压缩包

```bash
./install.sh
```

默认装到 `~/.local/bin`。`--system` 装到 `/usr/local/bin`,`--bindir DIR` 装到别处。
重新运行它就是受支持的升级方式。

## 从源码构建

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## 然后:让 Steam 指向它

Eidos 是*作为*你游戏的启动命令运行的,这正是它能在游戏启动前完成挂载的原因。
在 Steam 里右键点游戏 -> 属性 -> 启动选项:

```
~/.local/bin/eidos-gui %command%
```

按下开始游戏。Eidos 会在该游戏的实例上打开;安装模组、用 LOOT 排序、点 Run。
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
见 [usage.zh-CN.md](usage.md),完整导览也在那里。

## 可选:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 会开启内核 FUSE passthrough。
它**默认关闭,而且你几乎肯定应该让它保持关闭**:在 Skyrim SE 上实测,它会让游戏
打不开自己的档案与插件,于是模组静悄悄地不加载。这个开关的存在是为了重新测试该
机制,而不是因为推荐使用。

细节以及支撑该决定的实测数据,见
[troubleshooting.zh-CN.md](troubleshooting.md)。

## 已经出问题了?

[troubleshooting.zh-CN.md](troubleshooting.md) 讲了环境开关、如何读操作
计数器,以及迄今为止咬过人的每一个问题。
