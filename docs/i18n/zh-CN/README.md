<!-- eidos-i18n: source=README.md sha=31c66f68ad4e47e896eeb2256fd8f89c68cfdbef -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**从不碰你游戏文件夹的原生 Linux 模组管理器。**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#安装)

Eidos 给 Linux 上的 Bethesda 游戏带来 Mod Organizer 2 在 Windows 上给它们的东西:
一份每次启动时合成的模组合并视图。它用 Linux 的挂载命名空间和 FUSE,而不是 Windows API
挂钩,管理器本身不需要 Wine。不往游戏文件夹里复制任何文件,所以事后没有什么要清理。Eidos 还带有
Stellar Blade、Stardew Valley、7 Days to Die 和 Crimson Desert 的定义。

Steam 版本从游戏的启动选项运行 Eidos。Heroic 或 Legendary 里的 GOG 和 Epic 版本则把它当作
包装程序运行。游戏总是通过拥有该副本的商店启动;Eidos 从不自己用 Proton 启动它。

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **状态:** Skyrim SE 每天都在通过 Eidos 游玩,带着 SKSE、script extender 预加载器、
> Creation Club、LOOT 排过序的加载顺序和按配置档案分开的存档。目前只有一个游戏系列在真实
> 游玩中得到验证。另外十款 Bethesda 游戏和四款其他游戏已经接好线,等着测试者。

## 为什么是 Eidos

合并视图挂载在一个私有的挂载命名空间里,只有游戏的进程树看得见它。其他程序,包括第二个游戏,
永远看不到它,也不需要为它授权。游戏退出或被杀掉,或者机器断电时,命名空间随它的进程一起消失,
安装目录与原先一样。

每个配置档案拥有自己的模组列表、插件顺序、INI 和存档。启动时,插件文件和存档文件夹会被
bind-mount 到游戏自己的路径上,所以游戏写在那里的东西落进配置档案。切换配置档案就切换了
这一切。

Eidos 不需要 setuid 辅助程序,不需要系统守护进程,不用 `sudo setcap`,也不用改
`/etc/fuse.conf`。FUSE 守护进程以你的用户身份为一次启动而运行,并随游戏退出。你只需安装一个程序,
设置一条 Steam 启动选项。

每次运行之前,Eidos 都会保留一份插件列表的快照。如果一次崩溃损坏了列表,Diagnostics 会把它与那份
快照比较,Restore the pre-session order 会把旧顺序放回去。会抹掉你加载顺序的捕获会被拒绝,并说明
原因。

Test run 用配置档案的模组、不带模组,或只带选中且已启用的模组及其 master 来启动游戏。这次运行的存档、INI
改动、插件状态、Overwrite 输出,以及其他 My Games 文件(包括日志),都进入 `<instance>/test-run/`。
游戏退出时,Eidos 列出这五组,你对每一组决定保留还是丢弃。其他写入(例如 Wine 注册表)去它们一贯去的
地方。Diagnostics 里的 Find the mod behind a problem 每次用一半的嫌疑模组外加它们所需的 master 做
测试运行。每次运行之后你回答问题是否出现,Eidos 逐步缩小嫌疑范围,直到点出那个模组,或者彼此的
master 互相需要的那些模组;200 个嫌疑对象大约需要 8 次运行。开始之前在列表里选中且已启用的模组在每一步都
保持开启。细节见 [测试运行](docs/guide/usage.md#测试运行) 和
[找出问题背后的模组](docs/guide/usage.md#找出问题背后的模组)。

### 它与其他方案的对比

| | Eidos | Wine 里的 MO2 | Fluorine-Manager | Limo / 链接部署类 |
|---|---|---|---|---|
| 管理器原生运行 | 是 | 否,Wine 里的 Windows 程序 | 是(Qt 移植) | 是 |
| 游戏目录不受触碰 | 是,始终 | 是 | 是 | 否,会往里写链接 |
| 挂载对谁可见 | 只有游戏 | 只有游戏 | 整个系统 | 不适用 |
| 崩溃后需要清理 | 无 | 无 | 残留挂载恢复 | 手动取消部署 |
| Root 模组(ENB、预加载器) | 是,原生 | 需要插件 | 需要插件 | 部分支持 |
| 需要的权限 | 无 | 无 | 改 `/etc/fuse.conf` | 无 |

### 它有多快

| | 之前 | 现在 |
|---|---|---|
| 读取一个存档 | ~20 秒 | 6-7 秒 |
| 单次会话中的目录读取 | 560 万 | 33.5 万 |

切换单元格是即时的。找一个文件以前要挨个问五十个模组,列一个文件夹要把这件事重复五十遍。现在只读的层
在挂载时只索引一次,所以找一个文件只是一次哈希查找,文件夹合并后的列表也在挂载时只计算一次。这些数字来自
一个正常游玩的真实实例,不是跑分:[internals/performance.md](../../internals/performance.md)。

### 前人的工作与致谢

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) 和
  [usvfs](https://github.com/ModOrganizer2/usvfs):Eidos 复现的那套语义,以及它对照研究其行为
  一致性的代码库
- [LOOT](https://loot.github.io/):排序引擎,经由 libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) 以及其他 Linux 管理器,它们证明有一个社区希望这件事
  被解决

## 它做什么

### 模组

Eidos 安装简单压缩包、FOMOD 向导、Wrye Bash 的 BAIN 包、带有受支持 OBMM 脚本的 OMOD 容器,
其余的都交给手动选择器。Root 模组(script extender 预加载器、ENB、Engine Fixes)原生支持,
不需要 Root Builder 插件,也不往你的安装目录里复制任何东西。你可以隐藏单个文件、用分隔符给模组分组、
把一个选择移到目标位置、为每个模组保留备注和分类,并导入 MO2 配置档案。

这份列表的用法和 MO2 的一样:八个可选列并可按其中任意一列排序、按分类或按来源分组、双击手势、输入即
跳转、每个模组各自的备份(在你还原之前它们不起作用),以及对布局不会被本游戏加载、或是为另一个游戏下载
的模组给出的标记。筛选器按 Nexus 状态(已点赞、已追踪、有更新)、备注、隐藏文件和内容类型(插件、档案、
贴图、网格、脚本、声音等)挑出模组。文件树可以新建文件夹、重命名、删除和打开文件,并且不用启动任何东西
就能预览图片和文本。

Ctrl+Z 和 Ctrl+Shift+Z 撤销和重做对模组列表和加载顺序的改动。禁用或移除一个被配置档案最新三个存档之一
用到的模组时,会先询问并点出插件(Skyrim 和 Fallout 4 的存档)。重新安装或更新 FOMOD 模组时,向导会按
上次安装的回答打开。模组的 Information 会列出它需要的模组和需要它的模组,依据已启用插件的 master 读出。

### 插件

加载顺序内置 LOOT 排序、与游戏计算方式一致的模组索引、缺失 master 的警告,并把你的 DLC 和 Creation Club
内容显示为未管理的条目。标题统计每个索引空间(full、medium、light)已用的槽位。悬停在插件上会显示它的作者、
描述和头部版本。在 Skyrim 引擎上,游戏会加载同名 INI 的插件会被标出。

被 LOOT 标记为 dirty 的插件,由 xEdit 的 QuickAutoClean 清理进一个 `Cleaned plugins` 模组,绝不原地修改。
Flag as light 会把插件的 light 副本写进一个新模组,并在最新存档之一把该插件作为 full 插件加载时拒绝
(Skyrim 和 Fallout 4 的存档)。

### 实例

实例可以是全局的,放在 `~/.local/share/Colony/Eidos/instances/` 下,也可以是便携的:一个放在你想要的任何
地方(第二块硬盘、游戏分区)的自包含文件夹,可移动、相互隔离,和 MO2 的一样。便携实例会跨会话被记住。GUI、
Steam 启动和每条 CLI 命令都跟随你上次用过的那个,而且凡是接受游戏 id 的命令,同样接受这个文件夹。细节见
[usage.md](docs/guide/usage.md#实例全局与便携)。

实例也可以从实例里游戏文件夹的副本运行游戏。Eidos 第一次打开 Wabbajack 列表时,会运行列表自己的副本:它的
`ModOrganizer.ini` 指明的文件夹,否则是 `Stock Game` 或 `Game Root` 文件夹。Diagnostics 里 Game files 卡片上的
Freeze this version 会把商店安装复制到 `<instance>/Stock Game`,所以商店更新不再改变游戏所运行的文件。两个
文件夹位于同一个能共享数据块的文件系统上(例如 btrfs 或 XFS)时,副本与商店安装共享这些块(reflink)。在 ext4
上,或跨两个文件系统时,它是完整复制,会先显示大小和剩余空间。商店仍然启动游戏。Return to the store version
和 Delete the copy 可以撤销它,终端里的
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]` 做同样的事。Morrowind,
以及 `bUseMyGamesDirectory=0` 的 Oblivion,不能从副本运行。细节见
[usage.md](docs/guide/usage.md#游戏文件商店安装或副本)。

### 配置档案

每个配置档案各有自己的模组顺序、插件状态、INI 和存档。存档会被解析并与你当前的插件比对,还有一个按钮可以
启用某个存档所需要的东西,并在每次会话之后复制回前缀供 Steam Cloud 使用。Steam Cloud 从另一台设备带来
的存档,或者不经 Eidos 的启动留在前缀里的存档,会列在 Saves 标签页上。Import 把它们复制进配置档案,名字
已被配置档案使用的存档会以新的 `orphan-` 名字进来,而不是替换它。

配置档案可以为它的游戏和工具运行带上启动变量和包装程序(在 INI 编辑器里,每行一个 `env=NAME=value` 和
`wrapper=command`)。对于 Skyrim SE、Enderal SE 和 Fallout 4,Diagnostics 里的 Display 卡片会在配置档案的
INI 副本里设置窗口大小和模式。Backups 保留配置档案 INI 的还原点,Eidos 在每次 BethINI Pie 运行之前也会做
一个。配置档案的桌面快捷方式通过它的商店启动游戏。`eidos sessions` 列出已记录的运行,并把其中一个与当前
状态比较。

### Nexus

连接账号后,可以接收 Mod Manager Download 链接、检查更新,并查看作者及其个人资料。每个模组都在它的文件来自
的那个 Nexus 域名上被检查、点赞和链接,所以 Enderal SE 实例里的 Skyrim SE 模组仍留在 Skyrim SE 的网站上。
更新沿着作者在 Nexus 上设定的后继链走,用新文件替换模组的文件夹(会提供旧文件夹的备份),保留它的优先级
和启用状态,并重放安装时存储的 FOMOD 回答。在高级账户上,终端里的 `eidos nexus update <instance> --apply`
做同样的事,免费账户上则打印文件页面。Track 和 Endorse 在模组菜单里,每次更新检查都会把它们与你的 Nexus 账户
同步。模组的 Information 有一个 Nexus page 标签页,以纯文本显示它的描述和更新日志,它的 General 标签页末尾
则是上一次更新改变了什么:新增、删除和大小改变的文件,以及新增或移除的 master。Install downloads when they
finish(Preferences, Nexus,默认关闭)会在窗口开着时,下载一完成就安装它。

合集按它们的配方安装,可以中断后继续,并明确报告不支持的步骤。匹配时会核对确切的来源文件;不完整的旧元数据
仍标记为未验证。集合窗格可以浏览和搜索为某款游戏发布的集合,显示已安装集合是否有较新的修订版,并带免费账户
一页一页地走完缺失的文件。当一次拖动破坏了集合的顺序规则时,Diagnostics 会说明,Restore collection order 把
它的模组放回去。Downloads 标签页可以筛选、排序、隐藏和删除压缩包。Offline mode(Preferences, Nexus)会停止
访问 Nexus,以及每天一次的发行版检查。

### 工具

xEdit、BodySlide、DynDOLOD 和其他模组工具穿过合并视图,在游戏的 Wine 前缀里运行(Steam 的 Proton 前缀,
或 Heroic 建好的那个)。它们看得见你的模组,它们的输出落进 Overwrite,一次点击就能把它变成一个真正的模组。对于
Skyrim SE,Eidos 会在游戏文件夹里、某个模组里,或者你放在游戏旁边的工具文件夹里找到 SSEEdit、SSEEdit
QuickAutoClean、PGPatcher、Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、xLODGen、Synthesis 和
BethINI Pie,并为每个配上它需要的运行时。其他游戏有各自的列表。Pandora、Nemesis、FNIS、BodySlide 和 Synthesis
把输出写进一个具名的模组(`Nemesis Output` 等)。

发行版捆绑包带有 Microsoft 的 Visual C++ 2015-2022 和 .NET 8 Desktop Runtime 安装程序,以及 .NET 10 运行时。
Eidos 用这些安装程序安装 `vcrun2022` 和 .NET 8,不经 winetricks;没有捆绑包时,它从 Microsoft 下载同样的文件,
每个都按 Eidos 内置的 SHA-256 核对。Synthesis 需要 .NET 10 SDK,那是约 300 MB 的下载,捆绑包不附带。其他前置
组件(.NET Framework 4.8、XACT 和较旧的 .NET 版本)仍走 winetricks。在你要求安装某个工具的前置组件之前,什么也
不会下载。

把你用的工具钉住,把不用的隐藏,当某个工具本身就是一个 Steam 应用时给它自己的 Steam AppID,还可以写一个
`.desktop` 快捷方式,让它穿过合并视图启动,完全不用打开 Eidos。`eidos tool <game> import-mo2` 列出列表文件夹的
`ModOrganizer.ini` 里的可执行文件,`--write` 把它们加进来。Data 标签页里的 exe 可以运行一次或添加为工具,
`eidos tool <instance> run --exe <path>` 在终端里运行一个。打开 Ask before playing 后,当生成器的输出已过期时
Run 会先询问。

Stop 结束正在运行的游戏或工具。File 菜单在游戏的前缀里运行 winecfg、regedit、Wine 文件管理器和 Windows 卸载
程序。Diagnostics 里的 Rebuild prefix 会重命名前缀,让 Steam 创建一个新的,不删除任何东西。
`eidos shell <instance>` 在合并视图里打开一个 shell,或运行 `--` 之后给出的命令,它写入的内容落进 Overwrite。

窗口处于后台时,Eidos 可以在工具运行、集合安装、打包或解包,或者最后一个正在运行的下载结束,以及 LOOT 排序失败
时发送桌面通知。Preferences 里的 Notify the desktop when a long job ends 会打开它;默认关闭。

### 诊断

Diagnostics 标签页列出缺失的 master、无主的档案、模组列表漂移和损坏的插件集合,以及一次运行之后 script extender
自己的日志说实际加载了什么。它还报告:

- 缺失的、或与游戏版本不匹配的 script extender
- 自上次正常启动以来发生了变化的游戏
- 没有启动 Eidos、或后面没有 `%command%` 的 Steam 启动选项
- 比最后设置前缀的那个更旧的 Proton,或来自 Flatpak 版 Steam 的 Proton
- DLSS 缺少的 `PROTON_ENABLE_NVAPI`,以及会让 Community Shaders 的帧生成失效的独占全屏
- 没有 `LinuxVersion=true` 的 `enblocal.ini`
- 上一次会话的崩溃日志及其点名的模组
- 缺失或被禁用的 Nexus 前置要求
- 是另一个程序或缺失文件的 `nxm://` 处理器
- 会让游戏忽略 `plugins.txt` 的 `sTestFile` 行
- Starfield 里已启用的 `.esp` 插件,以及无效或不成对的 blueprint 插件
- 散装文件输给它所修补的模组的补丁
- 其 script extender 的 Papyrus 脚本副本胜过 extender 自带脚本的模组
- 过低的 `vm.max_map_count` 或打开文件数限制
- 位于 Windows 文件系统上的游戏文件
- 不是游戏 Data 文件夹的 BodySlide 数据路径,以及 `Overwrite/Root` 里本该在 Data 里的文件夹
- 把工具指向另一个游戏文件夹的列表 `ModOrganizer.ini`
- 缺失的、或没有游戏可执行文件的游戏副本

`eidos check <instance> [--json]` 运行不需要窗口的检查,发现问题时以 1 退出。Copy report 复制一份报告,你的
主目录写作 `~`,用户名写作 `<user>`;Data 标签页上的 Save tree 写出合并后的 Data 树并注明每个文件的来源,按同样
方式脱敏。

有关替换保护、BSA/BA2 内部文件冲突、SKSE 静态诊断、生成记录及其局限,请参阅
[完整性检查与生成文件](../../internals/integrity.md)(英文)。

### 它把自己的文件放在哪

Eidos 使用 Colony 家族里每个程序都用的布局:`~/.config/Colony/Eidos/` 存放你选择的东西(偏好设置、你的 Nexus
会话、你的实例列表、你写的游戏与附加定义),`~/.local/share/Colony/Eidos/` 存放全局实例、工具运行时(下载的,
或由 `install.sh` 从捆绑包复制来的)和会话日志,`~/.cache/Colony/Eidos/` 存放 LOOT 主列表。在实例内部,
`Stock Game` 放着冻结的游戏副本,`test-run/` 放着等待复查的测试运行。

更早的 Eidos 把文件放在 `~/.config/eidos/`、`~/.local/share/eidos/` 和 `~/.local/state/`。升级后的第一次启动会把
它们迁移过来,在日志里说明,并让旧目录保持原样。迁移完成之前,Eidos 会继续使用旧文件。详情见
[internals/paths.md](../../internals/paths.md)(英文)。

### 支持的游戏

Skyrim SE/AE 已在真实游玩中验证。Fallout 4 也已经端到端接好(自动换用 F4SE、archive invalidation、星号加载顺序、
LOOT、`.fos` 存档);见 [guide/fallout4.md](docs/guide/fallout4.md)。已接好线、正在找测试者的有:Skyrim LE、
Skyrim VR、Enderal SE、Fallout 4 VR、Fallout 3、Fallout NV、Starfield、Oblivion 和 Morrowind。Fallout 3、
Fallout NV、Oblivion 和 Morrowind 按文件的日期排列插件:Eidos 每个配置档案保存一份顺序,并在挂载内显示这些日期,
不改动磁盘上的文件。

另外四款游戏也已接好线,同样在找测试者:Stellar Blade(`~mods` 里的 Unreal `.pak` 模组)、Stardew Valley(SMAPI
模组,SMAPI 需单独安装)、7 Days to Die(游戏旁边的 `Mods/` 文件夹)和 Crimson Desert。Crimson Desert 只支持代码
模组:`bin64` 里的 ASI 插件、ReShade 和加载器 DLL。它位于 PAZ 组里的数据模组不受支持。

一个游戏就是一个 TOML 文件,你自己的可以放进 `~/.config/Colony/Eidos/games/` 而无需重新构建:
[internals/adding-games.md](../../internals/adding-games.md)。

## 安装

### 通过 Colony(推荐)

在 [Colony](https://github.com/Project-Colony/Colony) 里搜索 **Eidos** 并安装。更新通过启动器送达。Colony 只安装
一个文件 `eidos-linux`:它是窗口,而通过名为 `eidos` 的链接调用时,它也是 `eidos` 命令行。该文件既不带 NIF 模型
预览辅助程序,也不带下面捆绑包里的 Microsoft 运行库。没有辅助程序时,预览会如实说明,其他一切照常工作;你安装
某个工具的前置组件时,Eidos 会从 Microsoft 下载运行库。安装之后要做什么,以及如何让 Steam 指向它:
[docs/guide/install.md](docs/guide/install.md#通过-colony)。

### 直接下载二进制文件

从[最新发行版](https://github.com/Project-Colony/Eidos/releases/latest)获取一个资产:

| 资产 | 它是什么 |
|---|---|
| `eidos-linux` | 整个程序合为一个文件,与 Colony 安装的相同 |
| `eidos-linux-bundle.tar.gz` | `eidos`、`eidos-gui`、NIF 预览辅助程序、Visual C++ 2015-2022 和 .NET 8 Desktop Runtime 安装程序以及 .NET 10 运行时、图标、许可声明,以及一个注册桌面条目和 Nexus 下载处理器的安装程序 |

先按[代码签名策略](#代码签名策略)所示校验下载的文件,然后:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

安装程序不需要 sudo,会把二进制文件放进 `~/.local/bin`,并把捆绑包的 `runtimes/` 复制到
`~/.local/share/Colony/Eidos/runtimes`。`--system` 和 `--bindir DIR` 把二进制文件放到别处,`--from DIR` 从另一个
文件夹取它们。`--cap` 请 sudo 授予 `CAP_SYS_ADMIN`,只有实验性的 FUSE passthrough 需要它,而 passthrough 仍保持
关闭;`--no-cap` 是默认值。两种资产都需要 x86_64 Linux、glibc 2.39 或更新版本、FUSE(`fusermount3`),以及用于
模组压缩包的 7-Zip。

Eidos 不会自行更新。窗口在启动时每天一次向 GitHub 询问最新发行版,有更新的版本时,会在菜单栏的 Help 之后显示指向其
发行说明的链接。Preferences 可以关闭这项检查,装在 `/usr` 下(`/usr/local` 之外)、由包管理器更新的副本则从不询问。

### 从源码构建

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

需要 Rust 1.92 或更新版本(即 CI 检查的 `rust-version`;CI 和发行版用 1.99.1 构建),外加 `just`、CMake 3.20 或更高
版本,以及用于 NIF 预览辅助程序的 C++17 编译器。`just build` 使用 release 配置:一个代码生成单元和 fat LTO。用 3 个
构建任务对 `eidos` 和 `eidos-gui` 做一次冷构建,这样用了 334 秒,用 thin LTO 则是 243 秒,而 fat LTO 让 `eidos`
小 1.2 MB、`eidos-gui` 小 3.8 MB(测量数据在 `Cargo.toml` 里)。`just profile=dev build` 编译更快。Arch 软件包由
`packaging/PKGBUILD` 构建。前置条件以及不用 `just` 的命令:[docs/guide/install.md](docs/guide/install.md)。

### Steam 启动选项

Eidos 是作为游戏的启动命令运行的,这正是它能在游戏启动前完成挂载的原因。基础的这一行就是绝大多数配置所需要的
全部,写上你安装位置的绝对路径(Steam 不读取你的 `PATH`):

```
~/.local/bin/eidos-gui %command%
```

若通过 Colony 安装,这个路径是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。然后按下开始游戏:Eidos 会在该游戏的
实例上打开,而 Run 会让游戏穿过合并视图启动。

在你自己打开的窗口里,Run 会请 Steam 启动游戏(`steam -applaunch <appid>`),所以启动选项仍然适用,你在 Steam 打开的
窗口里按 Run。一旦 Eidos 被设为游戏在 Heroic 里的包装程序,Heroic 会得到一个 `heroic://launch` 链接;Legendary 则
运行 `legendary launch <app>`。商店的设置缺失时,Run 什么也不启动,并说明该设置什么:
[guide/usage.md](docs/guide/usage.md#启动游戏)。打开 Start the game at once when a store opens Eidos(Preferences,
默认关闭)后,Eidos 会运行 Diagnostics 检查,并在没有需要处理的事情时不显示窗口直接启动游戏。

其他一切都是叠在它前面的环境变量,而且它们可以自由组合:

| 你想要... | 放在前面 |
|---|---|
| 配合 Community Shaders 的 DLSS | `PROTON_ENABLE_NVAPI=1`。没有它,DLSS 永远初始化不了,而且不说任何话;完整清单见 [guide/graphics.md](docs/guide/graphics.md) |
| 屏幕上的 FPS 计数 | `DXVK_HUD=fps` |
| 驱动级插帧,零模组(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`,绝不要和 Community Shaders 自带的帧生成一起用 |
| 用于缺陷报告的详细日志 | `EIDOS_LOG=debug`(会话日志落在 `~/.local/share/Colony/Eidos/logs/`) |
| 来自挂载的单次会话 I/O 报告 | `EIDOS_FUSE_STATS=1` |
| 不同的 FUSE 工作线程数 | `EIDOS_FUSE_THREADS=8`(默认 4;追查并发缺陷时,`1` 是第一个该试的值) |
| 把这次启动钉在某个便携实例上 | `EIDOS_INSTANCE=/path/to/folder`。没有它,Eidos 会打开你上次用过的实例 |
| 开着 Start the game at once 时,为某一次启动显示窗口 | `EIDOS_WINDOW=1` |

一套现代模组配置(Community Shaders、DLSS、帧生成)该保留的那一行,就是这条最终命令,不是示例:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

在验证配置是否可用期间,把 `DXVK_HUD=fps` 加在前面,能用了就去掉。

同样的变量也可以放进配置档案的启动变量,而不是 Steam 那一行,这样它们按配置档案保存,并且也适用于工具运行:
[guide/usage.md](docs/guide/usage.md#每个配置档的变量与包装程序)。

更深层的诊断开关(`EIDOS_FUSE_TRACE`、缓存与索引的二分排查开关,以及 `EIDOS_FUSE_PASSTHROUGH` 为什么默认关闭)
在 [guide/troubleshooting.md](docs/guide/troubleshooting.md)。

## 文档

一切都在 [docs/](docs/) 里,索引按读者划分,见 [docs/README.md](docs/README.md):用户指南(安装、命令行与图形界面、
xEdit 和 DynDOLOD 等工具、图形、故障排查)、Eidos 内部如何工作,以及它为何存在。安全策略以及如何上报漏洞:
[SECURITY.md](../../../SECURITY.md)。

## 语言

玩家需要的页面都翻译了。英文是准绳:当译文与它不一致时,以英文文件为准。

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](../ja/README.md) · [index](../ja/docs/README.md) · [install](../ja/docs/guide/install.md) · [usage](../ja/docs/guide/usage.md) · [tools](../ja/docs/guide/tools.md) · [fallout4](../ja/docs/guide/fallout4.md) · [graphics](../ja/docs/guide/graphics.md) · [troubleshooting](../ja/docs/guide/troubleshooting.md) · [extensions](../ja/docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

其余的一切是英文。`docs/internals/` 和 `docs/project/` 是给那些同时也在读 Rust 的人看的,`CHANGELOG.md` 是生成的,
`SECURITY.md` 也不翻译。翻译前两者会为并不需要它们的读者,增加超过 28,000 个词要保持同步。

每份译文都带着它所依据的英文文件的哈希,当这个哈希不再与英文一致时,CI 就会失败,见
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)。随后译文被更新,并用 `scripts/i18n-check.sh --fix` 重新
盖章。过期的译文会被更新,而不是被删除。

一种语言就是一个目录。`docs/i18n/<lang>/` 镜像了仓库根目录的结构,所以 `docs/i18n/de/docs/guide/install.md` 就是
德文版的 `docs/guide/install.md`。正因如此,两个译文页面之间的链接与它们英文原文之间的链接是同一串字符,而撤下一门
语言只需要一条 `rm -r`。

## 代码签名策略

每一个发行版资产都由 Project Colony 组织在 CI 中签名,从不在开发者的机器上签名。在
发行版页面上,每个资产旁边都有:

| 文件 | 它是什么 |
|---|---|
| `<asset>.sig` | 用组织的发行密钥对资产做出的 ed25519 签名 |
| `<asset>.meta` | 三行内容,把资产与它的文件名、sha256 和发行版本绑定在一起 |
| `<asset>.meta.sig` | 对 `.meta` 的 ed25519 签名 |

私钥是组织的机密,只由共享的
[sign-and-publish 工作流](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
在一个什么都不构建的作业里使用;编译 Eidos 的那些作业永远见不到它。
Colony 在安装或更新 Eidos 之前会检查全部三个文件,并拒绝比已安装版本更旧的发行版。
要用 OpenSSL 3 自己校验下载的文件(同样的命令适用于每一个资产):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

发行版如何构建、签名和发布:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)。

## 隐私

Eidos 不发送遥测、不做分析,也不发送崩溃报告。除了下表列出的服务器之外,它不连接任何服务器,而且只为表中所写的那个
操作去连接。Offline mode(Preferences, Nexus)会停止与 Nexus 的一切连接,以及每天一次的发行版检查。

| 数据 | 存储或发送 | 在哪里,以及为什么 |
|---|---|---|
| 偏好设置、实例列表、游戏与附加定义 | 存储 | `~/.config/Colony/Eidos/` |
| 实例、工具运行时、会话日志 | 存储 | `~/.local/share/Colony/Eidos/`,或你为便携实例选择的文件夹 |
| LOOT 主列表 | 存储 | `~/.cache/Colony/Eidos/loot/` |
| 发行版检查 | 发送 | 启动时,每天最多一次,向 `api.github.com/repos/Project-Colony/Eidos/releases/latest` 发一个请求。Eidos 只读取发行版标签,什么也不下载。它默认开启,可在 Preferences 里关闭,在 Offline mode 下以及对装在 `/usr` 下(`/usr/local` 之外)、由包管理器更新的副本会被跳过。[GitHub 隐私声明](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus 登录 | 发送 | 登录会在你的浏览器里打开 `users.nexusmods.com`,它把结果交回给 Eidos 仅为这次登录而在 `127.0.0.1` 上开启的监听端。随后 Eidos 在 `users.nexusmods.com/oauth/token` 把它换成令牌,并获取 Nexus 用来签署这些令牌的公钥。[Nexus Mods 隐私政策](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus 凭据 | 存储 | 访问令牌和刷新令牌,保存在 `~/.config/Colony/Eidos/nexus.ini` 中,权限为 0600。更早的 Eidos 写在那里的 `api_key=` 行会被保留,但永远不会被读取;你愿意的话可以删掉它。 |
| Nexus 请求 | 发送 | `api.nexusmods.com`,在你使用某个 Nexus 功能时:你的账号名和高级会员状态、模组与文件详情、更新检查、合集和下载链接。你的点赞和追踪的模组在每次更新检查时读取,在你按 Endorse 或 Track 时写入。模组的前置要求、描述和更新日志,以及集合搜索,发往 `api.nexusmods.com/v2/graphql`。每个请求都带着你的访问令牌,并写明 Eidos 及其版本。 |
| Nexus 下载 | 发送 | 你请求的文件,来自下载链接指明的 Nexus CDN 主机,保存在实例的下载文件夹里,每个文件旁边都有一个 MO2 格式的 `.meta`。 |
| `nxm://` 链接 | 存储 | `eidos nxm --register`(`install.sh` 会替你运行)写入 `~/.local/share/applications/eidos-nxm.desktop`,并通过 `xdg-mime` 让它成为 `nxm://` 处理器,这样网站上的 Mod Manager Download 按钮就会把链接交给 Eidos。Arch 软件包会在系统范围内安装同样的条目。 |
| LOOT 主列表 | 发送 | `raw.githubusercontent.com/loot/...`,在某个游戏第一次 LOOT 排序时,以及每当你请求更新时。不会发送任何属于你的东西。[GitHub 隐私声明](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| 工具运行时 | 发送 | 只在你安装某个工具的前置组件时,而且只针对发行版捆绑包没有附带的东西(单文件 `eidos-linux` 和源码构建都不附带任何东西):来自 `builds.dotnet.microsoft.com` 的 .NET 10 运行时,以及来自 `download.visualstudio.microsoft.com` 的 Visual C++ 和 .NET 8 安装程序,与 Eidos 内置的 SHA-256 不一致即拒绝;.NET 10 SDK,始终来自 `builds.dotnet.microsoft.com`,并按同样方式核对;以及用于其他前置组件(.NET Framework 4.8、XACT、.NET 6 和 7)的 `winetricks`,它会从 Microsoft 和 winetricks 使用的其他主机下载它要安装的东西。[Microsoft 隐私声明](https://www.microsoft.com/en-us/privacy/privacystatement) |
| 会话日志 | 存储 | `~/.local/share/Colony/Eidos/logs/`,权限 0600:每个实例保留十次会话,30 天后清除。你的主目录写作 `~`,用户名写作 `<user>`,所以日志可以原样贴进 issue。除非你自己发送,它们永远不会离开你的机器。 |
| 运行记录 | 存储 | `<instance>/logs/sessions/`:每次游戏或工具运行的配置档案、已启用的模组及其版本和 Nexus 文件 id、插件顺序、每个配置档案 INI 的校验和、游戏和 Eidos 的版本、退出码,以及这次运行创建的存档。保留最新的 50 条,`eidos sessions` 读取它们,`eidos pack` 把它们排除。它们留在你的机器上。 |
| 更新报告 | 存储 | `<instance>/logs/updates/<mod>.json`:上一次更新或 Replace 对该模组文件夹做了什么改动。`eidos pack` 把它们排除。 |
| 快捷方式 | 存储 | `install.sh` 把 `eidos.desktop` 及其图标写到 `~/.local/share` 下;你创建的工具或配置档案快捷方式放在 `~/.local/share/applications`。 |
| 桌面通知 | 都不 | 默认关闭。打开时,它们通过 D-Bus(`org.freedesktop.Notifications`)发给你桌面的通知服务,并点出结束的那项任务。 |
| 你打开的链接 | 都不 | 模组页面和其他链接通过 `xdg-open` 在你自己的浏览器里打开。 |
| 附加组件 | 都不 | 你安装的扩展作为它们自己的程序在你的机器上运行;它们做什么由它们自己负责。 |

## 许可

Eidos 的原创代码采用 GPL-3.0-or-later 许可证;见 [LICENSE](../../../LICENSE)。以下部分采用不带“或更新版本”选项的
GPL-3.0:

- `eidos-install` 中的 OMOD 脚本转译;见其
  [上游作者致谢](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- 基于内置的 nifly 库构建的 `eidos-nif-preview` 辅助程序;见其
  [组件许可证和作者致谢](../../../crates/eidos-nif-preview/UPSTREAM.md);
- 加载顺序代码所链接的 `esplugin` 和 `libloadorder` crate,它们声明为 `GPL-3.0`。

二进制文件中的其他 Rust crate 保留各自的许可证(MIT、Apache-2.0、BSD 等)。发行版捆绑包在 `licenses/crates/` 中附带它们的
声明,并附有每个 crate 及其许可证的索引。

二进制文件还内嵌了八个正版 Microsoft DirectX 可再发行 DLL(`d3dcompiler_47`、`d3dcompiler_43`、`d3dx9_43` 和
`d3dx11_43`,32 位和 64 位各一份),以 gzip 压缩存放(7.9 MB,而不是 17.9 MB),当某个模组或工具需要它们时,Eidos 会
把它们解包并部署进游戏的前缀。它们仍受 Microsoft 可再发行条款约束;见其
[来源说明](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)。

发行版捆绑包还在 `runtimes/` 下带有 Microsoft 的 Visual C++ 2015-2022 安装程序、.NET 8 Desktop Runtime 安装程序和
.NET 10 运行时 zip,保持 Microsoft 发布时的样子。它们的条款在 `licenses/microsoft-runtimes/`
(`vc_redist.txt`、`dotnet.txt`)里。
