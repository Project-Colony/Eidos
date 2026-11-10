<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=af0c14c5192ccae592dd1c1661fd02d5242ecd5a -->

# 排查与诊断

环境开关、如何读操作计数器、Diagnostics 标签页的 Health 卡片、如何找到崩溃背后的模组、
提交缺陷报告时该附上什么,以及已知问题及其来龙去脉。

## 诊断 VFS

当游戏看到的东西和文件系统说的不一致时,有两个环境变量能覆盖大多数情况:

```sh
EIDOS_FUSE_STATS=1                  # 操作计数器,卸载时输出
EIDOS_FUSE_NO_CACHE=1               # 关掉内核侧的每一个缓存
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # 或者逐个点名
```

正是逐个点名的形式找出了已知问题里的 `FOPEN_KEEP_CACHE` 崩溃:四个全关回答的是
「是不是缓存的问题」,只有点名才回答「是哪一个」。计数器回答另一半。开启 passthrough
时(默认关闭),一次显示 `read 0` 的加载,意味着 `FUSE_PASSTHROUGH` 在内核里供上了
每一个字节,那么你打算在读取路径上做的一切调优本来就已经免费了。

还有几个开关可以进一步缩小故障范围:

```sh
EIDOS_FUSE_TRACE=open,opendir       # 记录每次 open / opendir(1 = 每个通道)
EIDOS_FUSE_THREADS=1                # 一个守护线程(默认:CPU 数,最多 4)
EIDOS_FUSE_OPENDIR=1                # 即使内核可以跳过,也保留 opendir 句柄
EIDOS_NO_INDEX=1                    # 遍历各层,而不是使用层索引
EIDOS_NO_OW_INDEX=1                 # 对 Overwrite 的成员索引做同样的事
EIDOS_FUSE_KEEP_CACHE=1             # 重新打开 FOPEN_KEEP_CACHE,见已知问题
```

从 Steam 启动时,把它们放在启动选项里命令的前面,例如
`EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`。输出进入本次运行的日志:从窗口
启动时是实例的 `logs/run-*.log`,从 Steam 启动时是 Steam 的控制台。

想看合并视图本身,`eidos shell <instance>` 会在游戏看到的那个游戏文件夹里打开一个
shell,`eidos shell <instance> -- ls Data/meshes` 则在那里运行一条命令;写入的内容都落进
Overwrite([usage.md](usage.md#合并视图里的-shell))。Data 标签页的 Save tree 按钮把整个
合并后的 `Data` 树写成文本文件,每行一个文件并注明它来自哪个模组,你的主目录会被隐去。

## 手动挂载一个合并视图

`eidos-fuse` 随 Arch 软件包和源码构建一同提供:`just build` 会构建它,`just install` 或
`install.sh` 在它存在时会复制它。发行版捆绑包和 `eidos-linux` 不带它。它的用法行:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

冲突时第一个 `--layer` 获胜;最后一个是你原封不动的游戏数据。挂载只需要 `/dev/fuse`
和 `fusermount3`(不用 overlayfs,也不用 Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... 通过 /mnt/point 读写 ...
fusermount3 -u /mnt/point
```

写入落到 `--overwrite <dir>`(省略时为临时目录),所以即便在这里,各层本身也保持
原样。

## 为什么 passthrough 默认关闭

passthrough 把真正的后备文件交给内核,于是读取完全绕开这个守护进程。下面的 A/B 是
用较早版本的守护进程测的,早于后文已知问题「FUSE passthrough 让游戏加载不了任何模组
内容」里描述的改动。这个改动避开了最可能的原因,但还没有在游戏里重新测量过,所以
在重新测量之前 passthrough 默认保持关闭。在 Skyrim SE 1.6.1170、proton-cachyos
11.0、内核 7.1.4 上做的 A/B 实测,同一份 82 个插件的加载顺序,唯一变量是二进制文件
是否带着该能力:

| passthrough | `NtCreateFile` 以 `STATUS_ACCESS_VIOLATION` 失败的次数 |
|-------------|--------------------------------------------------------|
| 开          | 152 - 75 个 `.bsa`、65 个 `.esl`、10 个 `.esm`、2 个 `.esp` |
| 关          | 0                                                      |

开着的时候,游戏打不开自己的任何档案与插件,在游戏里表现为模组不在:没有报错,没有
日志行。关掉之后,同一份加载顺序带着它的插件、档案和 Papyrus 脚本进入了游戏。

这个失败从守护进程内部是看不见的:守护进程自己的 `open` 每次都成功,内核也从未拒绝
后备文件(用 `EIDOS_FUSE_TRACE=open` 跑完一整场失败会话验证过:零次 `open FAILED`,
零次 `passthrough refused`)。错误发生在守护进程回答 `opened_passthrough` 之后,所以
守护进程侧的任何日志都看不到它。它也与扩展名无关,档案和插件同样中招,也就是游戏整场
都保持打开的那些文件。

`EIDOS_FUSE_PASSTHROUGH=1` 会把它开回来,用于测量它带来什么,或重新验证该机制。
启动器和 Diagnostics 标签页里的能力警告,只在你主动要求它时才出现。

## 层索引真的在起作用吗?

`LayerStack::new` 要么为只读各层建出完整映射,要么一个也不建。没有映射时,每次查询
都和从前一样逐层遍历:答案相同,读一次存档大约要三倍的时间,其他方面看不出任何区别。

索引关闭时,挂载会在启动时说明一次原因:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

原因会指出一个无法读取的目录、一个超过 64 层目录深的层、超过 4,000,000 个条目,
或 `EIDOS_NO_INDEX`。这一行写进本次运行的日志:从窗口启动时是实例的
`logs/run-*.log`,从 Steam 启动时是 Steam 的控制台。设置 `EIDOS_FUSE_STATS=1` 后,
卸载时的输出里还有一行 `layer index:`,普通挂载上 `unindexed` 不为零,就说明索引
当时是关闭的。

下面的示例离线检查同一件事,针对实例的文件夹,不需要挂载:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` 用索引和不用索引分别解析真实路径,并比较目录扫描结果。`index_agrees`
检查两者给出的是同一个答案,覆盖真实实例的每条路径与每次列目录。`listing_cost`
测量合并子项映射在 `readdir` 上省下了多少。

`EIDOS_NO_INDEX=1` 强制走遍历,适用于正在调试的恰恰就是两种答案之间差异的时候。

## 用命令行启动游戏

要通过 `eidos` 命令行而不是窗口启动游戏,把它的 Steam 启动选项设为 `eidos` 的绝对
路径,例如:

```
~/.local/bin/eidos play skyrimse -- %command%
```

不需要加 `WINEDLLOVERRIDES` 前缀。当某个已启用的模组导入 `d3dcompiler_47` 时,Eidos
会把自带的副本部署进前缀,强制它以原生方式加载,并与模组自带的任何 DLL 覆盖
(ENB/ReShade/`.asi` 加载器)合并。

Eidos 从不自己通过 Proton 启动游戏。对一个会启动游戏的条目(script extender 的加载器、
启动器或游戏本体)执行 `eidos tool <instance> run <title>`,是请拥有该副本的商店来启动
它,与没有商店打开过的窗口里按 Run 一样;商店的设置没有启动 Eidos 时,它会打印该设置
什么。`--print` 打印商店的命令而不是运行它。

## 无法关闭的游戏或工具

游戏或工具运行时,Run 按钮会变成 Stop,锁定卡片上也有一个 Stop 按钮。按两次。第二次
按下会结束这次运行启动的所有程序:先发 SIGTERM,10 秒后对剩下的发 SIGKILL。游戏没有
保存的内容会丢失。之后 Eidos 做的事与任何退出之后相同:INI 和插件的改动回到配置档,
工具的输出被记录,模组被卸载。无论程序返回了什么,这次运行都记录为退出码 143,所以
`eidos sessions <instance> diff` 绝不会把它当作上一次正常的运行。Unlock 不一样:它把
窗口交还给你,程序继续运行。在终端里对 `eidos` 进程执行 `kill <pid>`(pid 显示在锁定
卡片上)与 Stop 效果相同。

## 找出是什么坏了

要找出是哪个改动弄坏了游戏:

- `eidos sessions <instance> diff` 列出自上一次以退出码 0 结束的游戏运行以来改变的内容:
  模组、版本、插件顺序、INI 和游戏可执行文件
  ([usage.md](usage.md#自上次正常运行以来改变了什么))。
- Run 旁边的 Test run,用配置档的模组、不带模组,或只带选中的模组及其所需的主文件
  来玩,并把游戏写下的一切送到 `<instance>/test-run/`。运行之后,你对每一组文件决定
  保留还是丢弃:存档、INI 改动、插件状态、Overwrite 输出和日志
  ([usage.md](usage.md#测试运行))。
- Diagnostics 标签页上的 Find the mod behind a problem,每次用剩余嫌疑对象的一半做测试
  运行,并问你问题是否出现。200 个嫌疑对象大约需要 8 次运行,而且搜索在关闭 Eidos 后
  仍然保留([usage.md](usage.md#找出问题背后的模组))。

被停止的运行绝不会在那个搜索里预先填上 Yes,因为它记录为退出码 143(见上)。

## Health 卡片

Diagnostics 标签页为它找到的每个问题显示一张 Health 卡片,带有原因,有些还带一个修复
按钮。启动选项卡片在 [install.md](install.md) 里说明,NVAPI、全屏和 ENB 卡片在
[graphics.md](graphics.md) 里,Proton 和前缀卡片在下一节。`eidos check <instance>`
打印不需要窗口的结论([usage.md](usage.md#从终端做健康检查)列出了它们),其中有
前缀、注册表和 `S:` 盘符卡片。只有窗口会显示 Proton 卡片及其 Flatpak、未知版本和
较旧版本的变体、补丁顺序、扩展器脚本、没有插件加载的档案、Nexus 前置要求、过期的
插件副本和 Display 卡片。

### 系统

- "vm.max_map_count is <value>" 在低于 1048576 时出现,Arch Linux、Fedora 39 及更新版本、
  Ubuntu 24.04 及更新版本的默认值就是它。Proton 下的一些游戏会映射更多内存区域,然后
  毫无报错地崩溃。在 `/etc/sysctl.d` 下的某个文件里写入 `vm.max_map_count = 1048576`,
  例如 `80-gamecompatibility.conf`,然后运行 `sudo sysctl --system`。
- "The open files limit is <value>" 在硬限制低于 524288 时出现,Proton 的 esync 需要它;
  低于它时游戏可能无法启动或崩溃。systemd 240 及更新版本会设置它。在较旧的系统上,
  在 `/etc/systemd/system.conf` 和 `/etc/systemd/user.conf` 里设置
  `DefaultLimitNOFILE=524288`,然后重新登录。
- "Game files on a Windows file system" 会点出位于 `ntfs`、`ntfs3`、`fuseblk`(ntfs-3g、
  exfat-fuse)、`exfat` 或 `vfat` 上的游戏文件夹、前缀或实例根目录。Proton 需要符号
  链接、Unix 权限和区分大小写的名字。请把游戏、它的前缀和实例移到 ext4、btrfs 或
  xfs 这类 Linux 文件系统上。

### 游戏文件

标签页顶部的 Game files 卡片显示实例运行的是商店安装,还是实例里的游戏副本
([usage.md](usage.md#游戏文件商店安装或副本))。在三种情况下启动会被拒绝,并有一张卡片
说明原因:

- "The game copy '<folder>' is missing":实例运行的副本已经不在了。把它放回去,或者
  回到商店版本。
- "'<folder>' has no <binary>":实例运行的文件夹里没有游戏可执行文件,所以它不是游戏的
  副本。
- "<game> cannot run from a copy yet":Morrowind,以及 `bUseMyGamesDirectory=0` 的
  Oblivion,把插件列表放在游戏文件夹里。

"ModOrganizer.ini points tools at another game folder" 出现在列表自带的、并非 Eidos 写
的 `ModOrganizer.ini` 里 `gamePath` 与实例所运行的游戏文件不同时。PGPatcher 这类 Mod
Organizer 2 模式的工具从那里读取游戏。请修改该文件里的 `gamePath`,或删除该文件让
Eidos 写自己的。

### 加载顺序

- "sTestFile lines make the game ignore plugins.txt" 出现在 Fallout 4、Fallout 4 VR 和
  Starfield 上,条件是配置档的 `Fallout4Custom.ini` 或 `StarfieldCustom.ini` 在
  `[General]` 里有 `sTestFile1` 到 `sTestFile10` 行。只要设了其中一行,游戏就会加载这些
  插件,Eidos 写出的加载顺序不起作用。Fix 从配置档的副本里移除这些行,并把原文件保存为
  `<ini>.pre-fix`。
- Starfield:已启用的、没有 master 标志的 `.esp` 插件会得到一张卡片,因为 Starfield
  无法把它们排在 `.esm` 和带 master 标志的插件之间,并且会让它们的一些记录在整场会话
  中一直保持加载。作者发布 `.esm` 版本可以避免这两个问题。
- Starfield:"Invalid blueprint plugins" 列出具备 blueprint 标志中的一部分但不是全部
  的插件,即名字以 `BlueprintShips-` 开头且扩展名为 `.esm` 的插件。"Blueprint plugins
  without their main plugin" 列出主插件没有启用的那些:`BlueprintShips-Example.esm`
  需要 `Example.esm`、`.esp` 或 `.esl`。
- "<patch> loses <n> file(s) to <master>" 出现在某个模组里的插件需要另一个模组里的
  master,而那个模组优先级更高,所以两者都带有的文件以它的副本为准时。补丁通常应放在它
  所修补的模组之下,卡片的 Move 按钮会把它放过去。如果补丁本来就该输掉这些文件,就保持
  现有顺序。
- "Out-of-date copy of <plugin>" 出现在某个模组里有某个插件的清理过或带轻量标志的副本,
  而原插件此后已经改变(模组或游戏更新)时。游戏加载的是旧副本。把它隐藏,然后重新清理
  或标记新的插件。

### 模组列表

- "Order rules of the collection <name> that no longer hold" 出现在已安装集合的模组
  不再处于其规则所要求的顺序时。Restore collection order 只把该集合的模组移回去,
  Ctrl+Z 可以撤销。
- "<mod> requires <mod>, which is not installed"(或 "which is disabled")来自 Check for
  updates 读取的 Nexus 前置要求。Open on Nexus 打开所需模组的页面,Download main file
  在高级会员账户上下载它,Ignore 则隐藏另一个模组已经满足的前置要求。DLC 前置要求只是
  建议:Eidos 会找与 DLC 同名的插件。
- "fontconfig.txt names a font library that is missing"(Skyrim 引擎)会点出没有任何层
  和任何档案包含的字体,以及 `Interface/fontconfig.txt` 胜出的那个模组。在安装该字体
  或删除那一行之前,菜单文字可能显示为方框。
- "BA2 archive stores audio compressed"(Fallout 4 和 Fallout 4 VR)统计某个模组的 BA2
  以压缩形式存放的 `.wav` 和 `.xwm` 文件数。游戏播放它们时可能崩溃。请用未压缩音频
  重新打包档案。

### script extender 与游戏更新

- "<loader> is not installed" 出现在装了 script extender 插件(位于 `SKSE/Plugins`、
  `F4SE/Plugins` 等)而加载器不在任何已启用的模组里、也不在游戏文件夹里时。游戏启动时
  不会带上它们。
- "The script extender does not match game version <version>" 出现在已安装加载器,但没有
  对应当前游戏版本的运行时 DLL 时,例如 `skse64_1_6_1170.dll`。加载器会报错停止。这通常
  发生在游戏更新之后。这张卡片和上面那张都带有指向 extender 网站的 Open the download
  page 按钮和 Install the downloaded archive 按钮,后者会把加载器放进模组的 `Root/`
  文件夹。
- "<game> changed since the last clean launch" 出现在游戏可执行文件(Steam 副本则是它的
  build id)自上次经 Eidos 启动且无错误退出之后发生了变化时。Steam 更新若没有改变可执行
  文件,就不会触发它。为旧版本构建的 script extender 插件可能不再加载,它下面的卡片会
  点出声明了这一点的那些。要保留列表所基于的游戏版本,可以用 Game files 卡片上的
  Freeze this version 把它冻结进实例,或者在 `freeze` 单独运行显示大小之后,用
  `eidos stock-game <instance> freeze --yes`:之后商店的更新改变的是商店安装,而不是
  副本。
- "<mod> replaces <n> <extender> script(s)" 出现在优先级比 script extender 高的某个模组
  带有 extender 自身脚本的散装 `.pex` 副本时。旧副本会破坏它所声明的函数,而游戏不会
  报错。把 extender 移到那个模组之下(两者都是模组时,卡片的 Move 按钮会做),或在模组
  里隐藏那些文件。extender 的脚本散放在游戏的 `Data` 里时,请把 extender 安装成模组,
  这样才能排序。只检查散装文件,不检查档案。

### 工具输出

- "Overwrite/Root holds meshes"(或 textures)表示某个数据路径指向游戏文件夹的工具把
  这些文件夹写到了那里,游戏从不读取那里。修正工具的路径,删除这些文件夹,然后重新
  运行工具。Open folder 按钮会打开 `Overwrite/Root`。
- "BodySlide's GameDataPath is not the game's Data folder"(或 `OutputDataPath`):见
  [工具](tools.md#工具自己的设置终归是它自己的)。

### 上一次会话中的崩溃

"The game crashed in the last session" 出现在崩溃记录器在上次启动时写了日志时。卡片会
点出异常,以及日志中由你的模组或 Overwrite 安装的 DLL 和插件。把这些当作线索:崩溃日志
里点名的模组,往往只是崩溃显现的地方,而不是原因。日志里没有点到你模组里的任何文件时,
卡片会这么说。Open the log 打开日志,Copy the log (home path hidden) 复制日志并隐去你的
主目录。

### 窗口大小

对于 Skyrim SE、Enderal SE 和 Fallout 4,Display 卡片显示配置档启动游戏所用的窗口大小
和模式,当配置档的 Prefs INI 和前缀的都没有设置时,显示 "No window size set for
this profile"。Apply 把大小和模式写进配置档的副本;当由 SSE Display Tweaks 设定窗口
时,还会写进它的 INI 在 Overwrite 里的副本([usage.md](usage.md#diagnostics))。

## 游戏由哪个 Proton 运行,以及损坏的前缀

Diagnostics 标签页有一张 Proton 卡片:Eidos 和 Steam 为该游戏使用的版本、Steam 自己的
设置,以及最后运行该前缀的版本。如果最后准备前缀的 Proton 比所选的更新,卡片
会变成建议,因为在较新的前缀上用较旧的 Proton 可能会重置其中的 Wine 文件,或者
无法启动。请在 Steam 中游戏的属性、兼容性里选择一个至少同样新的版本,或者重建
前缀。

其他 Proton 和前缀卡片:

- "Proton comes from the Flatpak Steam install":那个 Proton 把它的运行时和 steamclient
  库放在 Flatpak 沙箱里,所以从宿主机运行它可能失败。Eidos 不会通过 `flatpak run` 启动
  它,那会把游戏放进沙箱,置于合并视图之外。请在
  `~/.steam/root/compatibilitytools.d/` 里安装一个 Proton 并为该游戏选用它。
- "Eidos cannot tell which Proton runs this game":Steam 的 `config.vdf` 没有为它指定
  兼容性工具,前缀也没有记录任何已安装的版本。在 Steam 里选定一个之前(游戏的属性、
  兼容性),工具不会启动。
- "<game> has no Proton 前缀 yet":先通过 Steam 启动一次游戏,然后按 F5。如果 Steam
  没有创建前缀,请检查 `steamapps/compatdata` 下该游戏的文件夹是否可写;一个由 root
  拥有的空文件夹会悄无声息地阻止它。
- "Windows tools cannot find <game>":前缀里记录的安装路径是错的或缺失,而 xEdit、
  DynDOLOD、Wrye Bash 和 Creation Kit 会读取它。下次你运行工具时,Eidos 会写入正确的
  路径。
- "The prefix's S: drive points outside the Steam library":`S:` 既不匹配库根目录(Eidos
  为工具把它指到那里),也不匹配 `<library>/steamapps`(Steam 为游戏把它指到那里)。
  库可能被移动过,或者前缀来自另一台机器。前缀里存放的每个相对于 `S:` 的路径,
  在下次通过 Eidos 或 Steam 启动、让它重新指向之前,都解析不到任何地方。
- "<store> installation: started through its store" 是 GOG 或 Epic 副本的卡片。当 Eidos
  是游戏在 Heroic 或 Legendary 里的包装程序时,Run 请它们启动游戏,工具则使用 Heroic
  为该副本记录的 Wine 或 Proton 版本运行。

File 菜单会在游戏的前缀里运行 winecfg、regedit、Wine 文件资源管理器和卸载程序,
使用运行该游戏工具的那个 Proton([tools.md](tools.md#游戏前缀中的-wine-自带程序))。
在前缀里有这些程序之前,它们的行保持灰色。

Proton 卡片上的 Rebuild prefix 按钮用于已经无法使用的前缀。按两次。Eidos 不删除任何
东西:它在同一文件夹里把 `steamapps/compatdata/<appid>` 重命名为
`<appid>.eidos-backup-<date>-<time>`,下次你从 Steam 启动游戏时,Steam 会创建一个新
前缀。只要有游戏、工具或任何 Wine 程序在使用该前缀,它就会拒绝。在新前缀出现
之前,窗口里的 Run 无法把游戏交给 Steam,而是显示启动选项,所以请在 Steam 里按 Play。
只保存在旧前缀里的存档和设置会留在重命名后的文件夹中:把需要的复制回来,不再需要
时请自己删除那个文件夹。

## 提交缺陷报告时附上什么

- Diagnostics 标签页上的 Copy report 会复制 Eidos 版本、游戏、Proton 版本、Steam 是否
  是 Flatpak、passthrough 状态、会话类型、发行版、内核、`vm.max_map_count`、打开文件数
  限制、`/dev/fuse` 和 `fusermount3` 是否存在、配置档的启动变量和包装程序的名字,以及
  每一张 Health 卡片。你的主目录显示为 `~`,登录名显示为 `<user>`。
  `eidos check <instance>` 在终端里打印同样的头部,`--json` 以 JSON 给出。
- 运行日志 `<instance>/logs/run-*.log` 以 Eidos 所运行的确切命令开头。File, Eidos logs
  会打开 Eidos 自己日志的文件夹 `~/.local/share/Colony/Eidos/logs`。
- 崩溃卡片上的 Copy the log (home path hidden) 复制崩溃日志。
- Data 标签页上的 Save tree 保存合并后的 `Data` 树,并注明每个文件的来源。

## 已知问题

### DLSS 或帧生成静悄悄地什么都不做

三个各自独立的原因,每一个都没有任何错误提示:启动选项里没开 NVAPI、独占全屏,或者
一个陈旧的 Reflex 帧率上限。完整清单在 [graphics.md](graphics.md)。

### 用 Flatpak Steam 时 Run 没有反应

在没有商店打开过的窗口里,Run 通过运行 `steam -applaunch <appid>` 请 Steam 启动游戏。
Flatpak 版 Steam 不会在宿主机的 `PATH` 上放 `steam`,所以状态栏会显示 "Could not start
Steam" 和错误信息。请改在 Steam 里按 Play。

### 来自另一台设备的存档没有出现

不经 Eidos 进入前缀的存档,例如从 Steam Deck 通过 Steam Cloud 下载的,在配置档的
存档被挂载期间保持隐藏。没有任何东西会删除它。Saves 标签页在标题下统计这些存档,
Import 会把它们复制进配置档,而不会替换配置档里的存档
([usage.md](usage.md#存档属于配置档))。

### 一个把同一个目录写成两种拼法的模组,会丢掉第二种拼法下的全部内容

已修复。ext4 把 `meshes/` 和 `Meshes/` 当作两个目录;合并视图不能这样,而真实的模组确实
两种都用,XP32 Maximum Skeleton 的动画和 FNIS 行为文件在首字母大写的那个下面,
`character assets` 在另一个下面。

解析器对每个路径分量取大小写完全匹配的那一个,并就此认定:它进了 `meshes/`,在里面
找不到路径的其余部分,于是放弃整个层。另一种拼法下的每个文件对游戏都是隐形的,没有
报错,没有日志,任何诊断里都没有。在一个真实的 50 层实例上,那是 74 个文件。

现在匹配上的分量只是候选,不是决定;仍然先试完全匹配的大小写,只有当其下的剩余部分
失败时,扫描才去找大小写折叠后相等的同级目录。列目录在上一层目录有同样的毛病,现在
每层都会读取每一个折叠后相等的目录。

路径索引从来没有这个缺陷,因为它会遍历找到的每个目录。所以有一段时间,索引返回了回退
路径找不到的文件,尽管回退路径本是永远不该出错的那个答案。

### DynDOLOD 的 LODGen 死掉,只留下一个空日志

由 `dotnet10` 修复;见 [tools.md](tools.md)。每个世界的 `LODGen_SSE_<world>_log.txt`
里只有一行版本横幅、一行 `.NET Version:`,再无其他,加上一个只说
"failed to generate object LOD for one or more worlds" 的对话框。原因是 Wine 的 Mono
代替 .NET Framework 应答,而且装多少 .NET Framework 都没用,Proton 会在每次前缀
更新时把 `mscoree.dll` 换成指向它自己目录树的符号链接。发行版捆绑包带有 .NET 10
运行时;其他安装方式则下载一次。

### Wine 无法得知这个挂载会折叠大小写

已修复。

Wine 的 `get_dir_case_sensitivity` 先用 `FS_IOC_GETFLAGS` 询问目录,再嗅探 CIOPFS
留在其所服务目录里的标记。两者都没有,Wine 就假定区分大小写,于是每一次拼写不能逐字节
吻合的查找,都退化成读取整个目录来寻找大小写无关的匹配。Bethesda 的游戏请求
`data/ccbgssse001-fish.bsa`,而文件名是 `ccBGSSSE001-Fish.bsa`,所以几乎每个资源都会
触发:八秒内 4471 次标记探测、2236 次完整重读目录,九十秒内 195796 次枚举 `Data`。
Skyrim SE 从未走到主菜单,它停在 240 MB 常驻内存,而守护进程烧掉了一个核心的 92%。

Eidos 从一开始就在 `resolve_read` 里折叠大小写。全部代价只来自它从不说出这一点。
现在 `lookup` 会回答 `.ciopfs`;`readdir` 依旧不列出它。`ioctl` 也会对目录回答
`FS_IOC_GETFLAGS` 并带上 casefold 标志,当前的 Wine 会在查找标记之前先问这个。

有两件事让它从「慢」变成「致命」。代价随目录大小增长,所以装上 Anniversary 内容
(`Data` 从 37 个文件变成 177 个)就压垮了它。而且 `opendir` 会急切地构建合并列表,
当 Wine 打开一个目录只是为了 `stat` 里面那个标记时,这纯属浪费,现在快照改为在第一次
`readdir` 时才拍。

之后:主菜单、2.1 GB 常驻、守护进程 0% CPU。

`EIDOS_FUSE_TRACE=opendir` 就是找出它的工具,并且随程序附带。操作计数器只说「有多少
次」;一个目录被枚举 195796 次,在总数里是看不出来的。

### 游戏把 `plugins.txt` 重写成空的

很可能与上面的大小写折叠是同一回事:一个它在合理时间内枚举不完的 `Data`,于是它断定
那里什么都没有,并把这个结论存了下来。未经证实,值得复查。无论如何,捕获保护(任何会把
活动集合整个清空的捕获,无论规模一律拒绝)意味着它再也损坏不了配置档。

### `FOPEN_KEEP_CACHE` 是关闭的

已修复。它会在主菜单出现几秒后让 Skyrim SE 因空指针解引用而崩溃,可稳定复现,且一个
模组都没装;另外三个内核侧缓存被逐个二分排除,只有这一个有影响。那次二分是在
`FUSE_PASSTHROUGH` 开启的状态下做的,而内核会让带着 `FOPEN_KEEP_CACHE` 的 passthrough
打开以 EIO 失败,所以开着这个标志时,每一次 passthrough 打开都失败了。让游戏崩溃的也许
正是这些失败的打开,而在 passthrough 关闭的情况下单独测试这个标志,从来没做过。
passthrough 回复现在不再带它。当时测量认为失去它是免费的,但 passthrough 开启时守护
进程服务的读取是零(`EIDOS_FUSE_STATS` 在一次完整加载中报告 `read 0`),所以在
passthrough 默认关闭的现在,真实代价尚未测量。要单独测试的话用
`EIDOS_FUSE_KEEP_CACHE=1` 打开。

### FUSE passthrough 让游戏加载不了任何模组内容

守护进程现在避开了最可能的原因(见下),但这还没有在游戏里重新测量过,所以
passthrough 默认保持关闭;`EIDOS_FUSE_PASSTHROUGH=1` 可以打开它。这次改动之前,
passthrough 开启时,Skyrim SE 以 `STATUS_ACCESS_VIOLATION` 打不开自己的 152 个文件,
所以任何模组内容都不会加载,而且悄无声息。测量结果,以及守护进程自己的日志为什么显示
一次干净的运行,见上面的「为什么 passthrough 默认关闭」。

内核给每个 inode 只绑定一个后备文件(`fs/fuse/iomode.c`)。如果一个 inode 已经有后备
文件,再带来第二个后备文件 ID 的打开会以 EIO 失败;对已有后备文件的 inode 做普通打开
(不带 `FOPEN_PASSTHROUGH`)也一样。带着 `FOPEN_KEEP_CACHE` 的 passthrough 打开同样以
EIO 失败。以前的守护进程每次打开都注册一个新的后备 ID,所以游戏已经开着的文件被第二次
打开时就会失败。Wine 把 EIO 报告为 `STATUS_ACCESS_VIOLATION`,这正是那 152 次打开失败
背后的错误。这条规则是从内核源码读出来的,并且与那 152 次失败吻合,所以它是最可能的
原因。

现在守护进程每个 inode 只注册一个后备文件,由它的所有 passthrough 打开共享,并且从不在
passthrough 打开上设置 `FOPEN_KEEP_CACHE`。passthrough 只用于游戏层和模组层中大小达到
1 MiB 或以 `.dll`、`.exe`、`.asi` 结尾的文件的只读打开。文件一旦有了后备,它的每一次
只读打开都使用该后备;只要它还有普通打开,新的读者拿到的也是普通打开。Overwrite 里的
文件从不走 passthrough,投射插件时间戳的挂载则完全不使用它。当 passthrough 读者还占着
一个文件时,以写入方式打开它会在任何 copy-up 之前得到 ETXTBSY(在 Wine 中是
`STATUS_SHARING_VIOLATION`),因此 `O_TRUNC` 打开不会在 Overwrite 里留下一个遮住模组
文件的空副本。

### Mod Manager Download 按钮没有反应

Preferences, Nexus, Downloads 显示由哪个程序接收 `nxm://` 链接。如果该程序不是 Eidos、
已不存在或尚未设置,会出现一张 Health 卡片。Register 按钮会写入 `eidos-nxm.desktop`,
并把它设为 `x-scheme-handler/nxm` 的默认处理程序。在终端中,`eidos nxm --register`
的效果相同。

如果状态已显示 Eidos,按钮仍然没有反应,可能是浏览器沿用了之前的选择。请在浏览器设置
中清除 nxm 链接的已保存选择,让它重新询问。Flatpak 版浏览器无法直接启动宿主机上的
程序,而是通过桌面门户转交链接,门户读取的是在宿主机上注册的处理程序。
