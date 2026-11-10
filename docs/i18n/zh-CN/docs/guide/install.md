<!-- eidos-i18n: source=docs/guide/install.md sha=d0e9082b618de5fbbdfa07c791a94bf2559b81d2 -->

# 安装 Eidos

共有四种方式。Colony 只安装一个文件 `eidos-linux`,它就是整个程序。发行版捆绑包、
Arch 软件包和源码构建则并排提供 `eidos`(命令行)、`eidos-gui` 和静态 NIF 预览辅助
程序 `eidos-nif-preview`。Arch 软件包和源码构建还会安装 `eidos-fuse` 和
`eidos-launch` 这两个更底层的命令行。每种方式都能注册 `nxm://` 处理器,让 Nexus 上的
"Mod Manager Download" 按钮把下载内容送到你的实例。

## 你需要先具备

| 需求 | 原因 |
|---|---|
| 带 FUSE 的 Linux | PATH 中要有 `fusermount3`。任何当前发行版都自带。 |
| 一款用 Proton 启动过一次的游戏 | Steam 只在首次启动时创建游戏的 Wine 前缀,而 Eidos 在其中工作。Eidos 还能识别 Skyrim Special Edition 的 GOG 和 Epic 版本、Fallout 3 与 Fallout: New Vegas 的 Epic 版本,以及 Stardew Valley 的 GOG 版本。这些游戏通过 Heroic 或 Legendary 运行,Eidos 是游戏在那里的包装程序:见 [usage.md](usage.md#启动游戏)。 |
| 一个 7-Zip 程序:`7z`、`7zz` 或 `7za` | 用于安装模组压缩包。Arch 上是 `7zip`,多数其他发行版是 `p7zip`。 |
| `par2`(可选) | 给 `eidos pack` 的备份写 PAR2 恢复数据。Arch 和 Fedora 上是 `par2cmdline`,Debian 和 Ubuntu 上是 `par2`。 |

不需要 root,不需要守护进程,不需要改 `/etc/fuse.conf`,也不需要把你加进任何用户组。
Eidos 挂载在属于游戏进程的私有命名空间里。

有些系统设置只会表现为崩溃。Diagnostics 标签页会检查 `vm.max_map_count`(1048576 或
更大)、打开文件数的硬限制(524288 或更大),以及游戏、它的前缀或实例是否位于 NTFS、
exFAT 或 FAT 上;每一项的修复方法见 [troubleshooting.md](troubleshooting.md#系统)。

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
NIF 模型预览需要 `eidos-nif-preview`,只有发行版捆绑包、Arch 软件包和源码构建才带有
它;没有它时,预览会如实说明,其他一切不受影响。`eidos-linux` 也不带捆绑包里的那些
Microsoft 运行库([见下](#捆绑包里的-microsoft-运行库)):工具需要某个运行库时,你开始
安装前置组件后,Eidos 会从 Microsoft 下载它。让 Steam 直接指向这个文件本身,如下文
所示。

## 发行版捆绑包

从[最新发行版](https://github.com/Project-Colony/Eidos/releases/latest)下载
`eidos-linux-bundle.tar.gz` 及其 `.sig`、`.meta` 和 `.meta.sig`,并按照 README 的
[代码签名策略](../../README.md#代码签名策略)所示校验它们。捆绑包顶层有 `eidos`、
`eidos-gui` 和 `eidos-nif-preview`,另有 `install.sh`、`README.md`、`LICENSE`、
`SECURITY.md`、`assets/brand/png` 里的图标,`licenses/` 里的第三方声明(nifly、omod、
microsoft-directx、microsoft-runtimes 和 Rust crate),以及下文的 `runtimes/` 文件夹。
它不带 `eidos-fuse` 和 `eidos-launch`。然后:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` 把二进制文件复制到 `~/.local/bin`;设置了 `$XDG_BIN_HOME` 时则复制到那里。
`--system` 通过 sudo 装到 `/usr/local/bin`,`--bindir DIR` 装到别处,`--from DIR` 从另一个
目录取二进制文件。这个脚本还会:

- 运行 `eidos nxm --register` 注册 `nxm://` 处理器,所以 Colony 一节里的注册步骤
  就不需要了;
- 写入 `~/.local/share/applications/eidos.desktop`,以及 `~/.local/share/icons/hicolor`
  下的 Eidos 图标;
- 把许可声明复制到 `~/.local/share/licenses/eidos`(用 `--system` 时是
  `/usr/local/share/licenses/eidos`);
- 把 `runtimes/` 复制到 `~/.local/share/Colony/Eidos/runtimes`,用 `--system` 时也是如此,
  所以这些运行库属于运行该脚本的用户;
- 最后打印 Steam 启动选项,其中带有它所安装的 `eidos-gui` 的绝对路径。

`--cap` 让 sudo 给所安装的 `eidos` 授予 `CAP_SYS_ADMIN`(见下文的 FUSE passthrough)。
`--no-cap` 是默认值,保持无 root。重新运行这个脚本就是受支持的升级方式。捆绑包和
`eidos-linux` 需要 x86_64 Linux,且 glibc 为 2.39 或更新版本。

### 捆绑包里的 Microsoft 运行库

捆绑包的 `runtimes/` 文件夹放着工具会要求的 Microsoft 文件:`VC_redist.x86.exe` 和
`VC_redist.x64.exe`(Visual C++ 2015-2022,即 `vcrun2022` 前置组件)、
`windowsdesktop-runtime-8.0.12-win-x64.exe`(.NET 8 Desktop Runtime,对应 `dotnet8` 和
`dotnetdesktop8`)以及 `dotnet-runtime-10.0.10-win-x64.zip`(.NET 10 运行时,对应
`dotnet10`)。Eidos 自己在游戏的前缀里运行 Visual C++ 和 .NET 8 的安装程序,不经
winetricks;.NET 10 的 zip 则只解包一次到 `~/.local/share/Colony/Eidos/runtimes`,所有
实例共用(解包后 78 MB)。每个文件只有在它的 SHA-256 与 Eidos 里固定的值一致时才会
使用。Synthesis 需要的 .NET 10 SDK(`dotnetsdk10`,约 300 MB)不在捆绑包里,总是
下载。

解包后的捆绑包也可以原地运行:Eidos 在上述数据文件夹之后,还会在自己二进制文件旁边
找 `runtimes/`。

## Arch

```bash
cd packaging && makepkg -si
```

这个 PKGBUILD 会从源码构建当前的 `main`。它把 `eidos`、`eidos-gui`、
`eidos-nif-preview`、`eidos-fuse` 和 `eidos-launch` 装进 `/usr/bin`,并带有系统级的
`nxm://` 处理器、桌面入口和图标。它依赖 `fuse3`、`7zip` 和 `xdg-utils`。它不带
`runtimes/` 文件夹,所以工具需要 Microsoft 运行库时会下载。`winetricks` 是可选的:
对于 Eidos 自己不安装的前置组件,例如 `dotnet48` 和 `xact`,仍然要用它。

## 从源码构建

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

从源码构建需要 Rust 1.92 或更新版本、`just`、CMake 3.20 或更高版本、C++17 编译器,以及用于原生辅助程序测试的 Python 3。`just build` 构建整个工作区,所以 `eidos-fuse` 和 `eidos-launch` 也会被构建,`just install` 对构建输出运行 `packaging/install.sh`,把它们和其余文件一起复制。不使用 `just` 时,请参阅[原生辅助程序说明](../../../../../crates/eidos-nif-preview/README.md),使用 `cargo build --release --locked` 构建 Rust 程序,将辅助程序复制到 `target/release`,然后运行 `packaging/install.sh --from target/release`。源码构建同样没有 `runtimes/` 文件夹,所以工具需要 Microsoft 运行库时会下载。

release 配置使用一个代码生成单元、fat LTO 和去除符号。用 rustc 1.99.1 和 3 个构建任务,
`eidos` 与 `eidos-gui` 的冷构建用了 334 秒(thin LTO 为 243 秒),得到 20.9 MB 的
`eidos` 和 49.7 MB 的 `eidos-gui`。每个二进制文件内嵌八个以 gzip 压缩存放的 DirectX
DLL:7.9 MB,而不是 17.9 MB。`just profile=dev build` 不做优化地构建。

## 然后:让 Steam 指向它

Eidos 是*作为*你游戏的启动命令运行的,这正是它能在游戏启动前完成挂载的原因。在 Steam
里右键点游戏 -> 属性 -> 启动选项,填入 Eidos 窗口的绝对路径,后面接 `%command%`:

```
~/.local/bin/eidos-gui %command%
```

请使用绝对路径,Steam 不读取你 shell 的 `PATH`。路径取决于你怎么安装的 Eidos,
`install.sh` 会打印它所用目录对应的那一行:

| 安装方式 | 启动选项 |
|---|---|
| 发行版捆绑包或 `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch 软件包 | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

按下开始游戏。Eidos 会在该游戏的实例上打开;安装模组、用 LOOT 排序、点 Run。退出时
挂载随之消失,你的安装目录与原先分毫不差。

游戏总是通过它的商店启动。在你自己打开的窗口里,Run 会运行 `steam -applaunch <appid>`,
Steam 通过启动选项启动 Eidos,然后打开第二个 Eidos 窗口,你在那里按 Run。在启动选项
启动 Eidos 之前,Run 什么也不会启动,状态栏会显示该设置的那一行。打开
Start the game at once when a store opens Eidos(Preferences, General, Running a game;
默认关闭)后,在 Steam 里按 Play 会在 Health 没有列出问题时不开窗口直接启动游戏。
Heroic 和 Legendary 通过它们的包装程序设置以同样方式工作;细节见
[usage.md](usage.md#启动游戏)。

Diagnostics 标签页会检查这一行。启动选项为空、没有指向任何 Eidos 二进制文件、指向的
文件不存在、Eidos 后面没有 `%command%`,或者启动的是另一份 Eidos 而不是当前运行的这
一份时,Health 卡片会显示该设置的那一行。

### 如果你更喜欢终端

```sh
eidos init skyrimse               # 创建实例(给出文件夹即为便携实例)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root 模组
eidos sort skyrimse               # 用 LOOT 排序加载顺序
eidos check skyrimse              # 不需要窗口的 Health 检查
eidos play skyrimse -- %command%  # 让任何程序穿过合并视图运行
eidos shell skyrimse              # 合并视图里的 shell,写入落进 Overwrite
```

凡是接受游戏 id 的命令,同样接受便携实例的文件夹,见
[usage.md](usage.md#实例全局与便携)。完整导览在 [usage.md](usage.md)。

## 更新

Colony 通过它的启动器更新 `eidos-linux`。捆绑包请运行新捆绑包的 `install.sh`;Arch 软件包
请再次运行 `makepkg -si`;源码构建请拉取后运行 `just install`。窗口在启动时向 GitHub
询问最新发行版,每天最多一次,有更新的版本时,会在菜单栏的 Help 之后显示指向其发行
说明的链接。它从不自行下载或替换自己。Preferences, About, Version 可以关闭这项检查,
离线模式会跳过它,装在 `/usr` 下的副本(Arch 软件包)从不询问,因为由包管理器更新它。

## 可选:FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 授予的是可选权限,不会开启
passthrough。`./install.sh --cap` 和 `just setcap` 会把它授予它们所安装或构建的
`eidos`。在 `nosuid` 挂载上内核会忽略文件权限,所以 `install.sh` 会带警告跳过授权,
`just setcap` 则报错停止。这个权限加在捆绑包、Arch 软件包或源码构建中单独的 `eidos`
二进制文件上,绝不能加在 `eidos-linux` 上,因为后者同时也是窗口
([SECURITY.md](../../../../../SECURITY.md))。重新构建或重新安装的二进制文件是没有该
权限的新文件,所以如果你用它,每次升级后要再授权一次。

`EIDOS_FUSE_PASSTHROUGH=1` 是另一个运行时开关,默认关闭。passthrough 开启时,早先版本
的守护进程会让 Skyrim SE 加载不了任何模组内容。最可能的原因是它每次打开都注册一个新的
后备文件,而内核每个 inode 只接受一个。守护进程现在让一个文件的所有 passthrough 打开
共享同一个后备,但这还没有在游戏里重新测量过。在那之前,只为了测试才打开 passthrough。
实测数据和原因见 [troubleshooting.md](troubleshooting.md)。

## 已经出问题了?

[troubleshooting.md](troubleshooting.md) 讲了环境开关、如何读操作计数器、Health 卡片,
以及迄今为止咬过人的每一个问题。
