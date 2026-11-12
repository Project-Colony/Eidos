<!-- eidos-i18n: source=docs/guide/usage.md sha=6459b22b0401855f4cacc644cb35eade2e2a023d -->

# 使用 Eidos

怎么使用 Eidos:实例与游戏副本、命令行、测试运行、窗口、Nexus 集合与更新、把实例搬到另一台
机器、工具和扩展。遇到问题请看 [troubleshooting.md](troubleshooting.md);关于从源码构建
和仓库布局,见 [../internals/contributing.md](../../../../internals/contributing.md)。

## 实例:全局与便携

接受实例的命令,既可以给游戏 id,也可以给便携实例的文件夹。`skyrimse` 指全局实例,Eidos
把它放在 `~/.local/share/Colony/Eidos/instances/skyrimse`。便携实例是一个自包含的文件夹,
可以放在你想放的任何地方(第二块硬盘、游戏分区),并且可以移动,和 MO2 的便携实例一样:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # 在那里创建一个便携实例
eidos install /mnt/games/EidosSkyrim mod.7z  # 每个命令都接受这个文件夹
eidos play /mnt/games/EidosSkyrim -- %command%
```

文件夹里的 `eidos-instance.ini` 写明了游戏,所以这个文件夹不需要别的东西。环境中的
`EIDOS_INSTANCE=<folder>` 会把游戏 id 重定向到那个文件夹,例如用在 Steam 启动选项里。你
创建或打开过的便携实例会记录在 `~/.config/Colony/Eidos/instances.ini`,最近使用的排在前面:
窗口的欢迎页会列出它们,从 Steam 启动时会进入你上次玩的那个,`nxm://` 处理器则把下载放进
它。

手动移动便携文件夹,除了你用绝对路径登记到旧位置的工具条目之外,一切都会保留。
`eidos pack` 和 `eidos unpack`([见下](#把实例搬到另一台机器))会修正这些条目,单纯的 `mv`
不会。`~/.local/share/Colony/Eidos/runtimes/` 里的运行时缓存由这台机器上的所有实例共用,
所以 78 MB 的 .NET 宿主不会被复制到每个实例里。

有一个位置会被拒绝:游戏的安装文件夹之内,MO2 用户常把实例放在那里。那棵目录树归 Steam
所有,一次更新、一次「验证完整性」或一次卸载,都可能连同你的配置一起改写或删除它。Eidos
还会挂载到游戏根目录之上,所以放在里面的实例会落在它自己的挂载目标里。向导、
`eidos init`、`eidos play` 和 `eidos shell` 都会拒绝它。请改为放在游戏旁边;同一块硬盘上的
同级文件夹一样方便。

### Eidos 把文件放在哪里

Eidos 把自己的文件放在 `Colony/Eidos` 之下,这是 Colony 家族里每个程序都使用的布局:
`~/.config/Colony/Eidos/` 放你选定的内容(偏好设置、你的 Nexus 会话、你的实例列表、你写的
游戏和扩展定义),`~/.local/share/Colony/Eidos/` 放全局实例(`instances/`)、下载的运行时和
会话日志(`logs/`),`~/.cache/Colony/Eidos/` 放 LOOT 的 masterlist,缺失时会重新下载。

当它发现旧版 Eidos 布局的文件(`~/.config/eidos/`、`~/.local/share/eidos/` 和
`~/.local/state/`)时,第一次启动会把它们复制过来,并在日志里说明。全局实例用硬链接而不是
逐字节复制来迁移,所以它的模组不占额外空间,它的工具条目也会改指向新文件夹。旧目录里的
任何东西都不会被改动或删除;新布局运行正常之后,请自己删除它们。如果复制无法完成(磁盘已满
或只读、旧实例文件夹在另一块硬盘上、游戏仍在从中运行),Eidos 会继续使用旧位置,并在下次
启动时再试。仍然写着旧实例文件夹的 Steam 启动选项,会打开已迁移的那个。

硬链接只在一开始省空间。之后你移除或重装的模组或下载,会通过旧实例文件夹让它的旧副本留在
磁盘上,删除 `~/.local/share/eidos/<game>/` 才能把这部分空间还回来。

便携实例从不被迁移:它留在你放的地方。

## 命令行

```sh
eidos games                                # 这里已安装的受支持游戏,像 MO2 的列表
eidos init skyrimse [folder]               # 创建实例;给出文件夹就是便携实例
eidos init skyrimse --game-path <folder>   # 安装了多份时,使用安装在 <folder> 里的那份
eidos install skyrimse mod.7z [name]       # 安装压缩包(Simple、FOMOD、BAIN、OMOD)
eidos install skyrimse mod.7z --replace --backup  # 覆盖重装该模组(--merge 则装到它的文件之上)
eidos import skyrimse <mo2-profile>        # 接管 MO2 配置档的模组顺序和插件状态
eidos sort skyrimse [--dry-run]            # 用 LOOT 排序加载顺序(--update-masterlist 先刷新它)
eidos play skyrimse                        # 显示将要挂载的内容
eidos play skyrimse -- <command>           # 在模组挂载到游戏之上的状态下运行 <command>
eidos shell skyrimse                       # 合并视图里的 shell;写入落进 Overwrite
eidos check skyrimse [--json]              # 不需要窗口的 Health 检查
eidos sessions skyrimse                    # 已记录的游戏和工具运行,最新的在前
eidos sessions skyrimse diff [<id>]        # 自上一次以退出码 0 结束的运行以来改变了什么
eidos stock-game skyrimse                  # 实例运行的是哪些游戏文件;冻结一份副本或切换
eidos tool skyrimse                        # 列出工具(add、rm 和 run:见下面的工具一节)
eidos tool skyrimse run --exe <path>       # 穿过合并视图把某个 exe 运行一次
eidos tool skyrimse import-mo2 [--write]   # ModOrganizer.ini 里的可执行文件;--write 把它们加进来
eidos prereqs skyrimse [--install]         # 工具需要什么;--install 取回缺失的部分
eidos collection skyrimse <link>           # 安装 Nexus 集合(--dry-run、--no-optional)
eidos nexus status                         # 检查已保存的 Nexus 登录
eidos nexus update skyrimse [--apply]      # 检查模组更新;--apply 安装它们
eidos nxm <link>                           # 下载一个 Mod Manager Download 链接
eidos nxm --register                       # 让网站上的 Mod Manager Download 按钮打开 Eidos
eidos export skyrimse -o mods.csv --active # MO2 的 CSV 格式;--active 只列出已启用的模组
eidos pack skyrimse backup.eidos           # 把整个实例装进一个文件
eidos unpack backup.eidos [folder]         # 不给文件夹时,全局实例回到它的中心位置
eidos --version                            # 打印版本
```

不带参数的 `eidos` 会打印命令列表。大多数命令在缺少参数时会打印它们自己的选项(`eidos init`
和 `eidos import` 则打印完整列表),`eidos tool --help` 列出 tool 的子命令。`eidos pack`
接受 `--dry-run`、`--level`、`--no-downloads`、`--no-recovery` 和 `--force`,`eidos unpack`
接受 `--info` 和 `--force`;两者在[下文](#把实例搬到另一台机器)里说明。`eidos stock-game`
在[游戏文件](#游戏文件商店安装或副本)一节说明。

### 通过合并视图运行游戏

`eidos play` 在一个私有命名空间里,把实例的模组挂载到游戏自己的 `Data` 目录之上,然后让
命令穿过这个视图运行。游戏自己的文件会先被 bind-stash,所以守护进程仍然原样读取它们。
写入(存档、重新生成的配置)落进实例的 `overwrite/` 层;游戏安装目录和每个模组文件夹都保持
逐字节原样。

Eidos 不需要 root。它挂载在私有的用户与挂载命名空间里,没有 setuid 辅助程序,没有系统服务,
也不需要授予任何东西。`eidos` 二进制文件上的一个可选权限允许 FUSE passthrough,它保持关闭,
除非设置了 `EIDOS_FUSE_PASSTHROUGH=1`;见
[install.md](install.md#可选fuse-passthrough)。有这个权限时,Eidos 使用普通的挂载命名空间而不是
用户命名空间,两种方式下模组的部署结果相同。

### 合并视图里的 shell

`eidos shell <instance>` 在游戏会看到的那个游戏文件夹里启动你的 `$SHELL`(或 `/bin/sh`):每个已启用的模组都挂载在 `Data` 之上,Root 模组挂载在游戏根目录之上,在那里写入的一切都落进 Overwrite。`eidos shell <instance> -- <command>` 则只运行一条命令,例如 `eidos shell skyrimse -- ls Data/meshes`,或者 NifSkope 这样的原生工具。这次运行像游戏会话一样持有实例锁,所以在你退出之前窗口无法保存更改,它在 `eidos sessions` 里以 `shell` 为名被记录,绝不会记为游戏启动。对一条命令按 Ctrl+C,会像 Stop 那样停止这次运行:Eidos 结束命令、捕获会话并卸载。在交互式 shell 里 Ctrl+C 照常属于 shell;输入 `exit` 结束这次运行。

### 从终端做健康检查

`eidos check <instance>` 运行不需要窗口的 Health 检查,所以可以通过 SSH 或在 Game Mode
里运行,并放进缺陷报告:

- Proton 前缀及其注册表;
- 插件诊断和缺失的 master;
- script extender 及其插件;
- 自上次正常启动以来的游戏更新;
- Steam 启动选项;
- 图形设置(NVAPI、全屏、ENB);
- 上一次运行的崩溃日志;
- 配置档 INI 里的 `sTestFile` 行;
- Starfield 的 `.esp` 和 blueprint 插件;
- `vm.max_map_count` 和打开文件数限制;
- 实例所运行的游戏副本,以及列表的 `ModOrganizer.ini` 里指向另一个游戏文件夹的
  `gamePath`;
- 不再成立的集合顺序规则;
- 位于 NTFS、exFAT 或 FAT 上的游戏文件;
- BodySlide 的数据路径;
- `Overwrite/Root` 里遗留的 `Data` 文件夹;
- `Interface/fontconfig.txt` 点名但没有任何模组提供的字体库;
- Fallout 4 和 Fallout 4 VR 模组的 BA2 档案里的压缩音频。

有些卡片只由窗口在 Diagnostics 标签页上计算:补丁顺序、模组替换的 extender 脚本、没有任何东西
加载的档案、Nexus 前置要求、LOOT 标记为 dirty 的插件、运行游戏的 Proton 版本、Display
卡片,以及比它的输入更旧的工具输出。

命令会打印与 Copy report 相同的系统行,然后每个结论一行,你的主目录显示为 `~`,登录名显示为
`<user>`。没有发现问题时退出码为 0(只有建议也是 0),至少发现一个问题时为 1,Eidos 无法检查时
为 2:参数错误、打不开的实例,或者检测不到的游戏。

`eidos check <instance> --json` 改为打印一个 JSON 对象。它的顶层字段是 `eidos`(版本)、
`game`、`instance`、`system`(`proton`、`flatpak_steam`、`passthrough`、`session`、`distro`、
`kernel`、`dev_fuse`、`fusermount3`、`max_map_count`、`open_files_hard`)和 `findings`。
`max_map_count` 和 `open_files_hard` 是数字,Eidos 读不到时为 `null`。每个结论是一个 `code`、
一个 `level`(`problem` 或 `advice`)和它的 `args`;Diagnostics 标签页会为每个 code 显示对应的句子。

### 自上次正常运行以来改变了什么

每一次通过 `eidos play`、`eidos shell` 或 `eidos tool run` 的运行,无论从窗口、终端还是 Steam 启动,都会往 `<instance>/logs/sessions/` 写一条记录:配置档、每个已启用的模组及其版本和 Nexus 文件 id、插件顺序、每个配置档 INI 的校验和、游戏可执行文件的版本、Eidos 版本、退出码,以及这次运行创建的存档。保留最新的 50 条,`logs/` 不会进入打包的实例。`eidos sessions <instance> diff` 把最新的、以退出码 0 结束的游戏运行与当前状态比较;`diff <id>` 从列表里选另一条记录。列表把[测试运行](#测试运行)标为 `test`,测试运行绝不会被选为上一次正常的运行。被你停止的运行记录为退出码 143,所以同样不会被选中。Eidos 在这里还不检测崩溃,而在 Proton 下崩溃的游戏仍可能以退出码 0 退出:如果它选中的运行是这种情况,请用 `diff <id>` 指定更早的一条。

## 模组列表

和 MO2 一样,每个配置档都有自己的 `modlist.txt`(哪些模组已启用、优先级如何),而模组文件夹
放在实例共享的 `mods/` 里。你手动放进 `mods/` 的文件夹还不会被列出:它会以禁用状态、最高
优先级出现在窗口列表的底部,和 MO2 一样,所以你启用它之后它会赢得它的冲突。

由于文件夹是共享的,移除或重命名一个模组会影响每个配置档:它的那一行会从每个配置档的
`modlist.txt` 里删掉或改名,而每个配置档对其余模组保留各自的状态和顺序。一个没有任何模组的
配置档根本没有 `modlist.txt`。

窗口使用它自己持有的模组列表和加载顺序副本。窗口打开期间,如果另一个 Eidos 进程写了其中任何
一个(终端里的 `eidos install` 或 `eidos sort`、一个集合、Steam 启动的会话、第二个窗口),窗口
的下一次编辑会被拒绝,而不是覆盖那份工作。它会从磁盘重新载入,并提示 *Not saved: another Eidos
process changed the mod list. It has been reloaded - redo the change.*(加载顺序则是对应的同样
提示);只会丢失那一次点击,重做一遍即可。Rename、New mod、Add separator、Remove 和把
Overwrite 变成模组,都会在动 `mods/` 之前先检查,所以被拒绝的编辑不会改动磁盘上的任何东西。
INI 编辑器同样拒绝覆盖保存一个在它打开之后已经改变的文件。

## 存档属于配置档

每个配置档在 `profiles/<name>/saves/` 里保存自己的存档,运行期间这个文件夹会挂载到 Proton
前缀里游戏的存档文件夹之上。配置档第一次启动时,会把前缀里已有的存档复制进去,仅此
一次,所以你之后从配置档里删掉的存档会一直保持删除。每次运行之后,配置档最新的存档会被复制
回前缀,供 Steam Cloud 使用;前缀里不是 Eidos 放进去的存档,会先以
`orphan-<time>-<name>` 保存在配置档里,绝不会被覆盖。

在那次首次复制之后,不经 Eidos 进入前缀的存档(从 Steam Deck 通过 Steam Cloud 下载的、
不用 Eidos 启动游戏产生的),在配置档的存档被挂载期间保持隐藏。没有任何东西会删除它:启动时
会在运行日志里点出它(从窗口启动时是实例的 `logs/run-*.log`,从 Steam 启动时是 Steam 的
控制台),这组文件每变化一次就记一次,Saves 标签页则在标题下的那一行统计这些存档。要玩其中
一个,请在 Saves 标签页按 Import。Import 把每个存档连同它的协同存档复制进配置档,绝不会替换
配置档里的存档:名字已被配置档使用的存档,会以 `orphan-<time>-<name>` 导入。

## 测试运行

Run 旁边的 Test run 按钮像 Run 一样启动游戏,但游戏写下的一切都进入 `<instance>/test-run/`,而不是你的配置档、Overwrite 和前缀:新的和改动的存档、INI 改动、`plugins.txt`、写进 Data 或游戏文件夹的文件,以及 `Documents/My Games/<game>` 里的日志。你可以用配置档的模组测试、不带任何模组测试,或者只带选中且已启用的模组,外加它们的插件所需的、作为 master 的那些模组。

游戏退出时,Eidos 把这些文件列成五组(存档、INI 改动、插件状态、Overwrite 输出、日志),并请你对每一组决定保留还是丢弃。要等每个有文件的组都有了回答,Apply 才可用。运行所用的模组集合与配置档的不同时,不会提供插件状态。保留存档绝不会从配置档里删除任何存档,即使是游戏在运行中删除的存档。保留 Overwrite 输出,还会把游戏删除的文件也从 Overwrite 里删除,列表会先统计它们。丢弃任何东西都要按第二次。Decide later 关闭列表并把文件留在 `test-run/` 里,下次你打开实例或按 F5 时列表会再次打开。如果 Eidos 或电脑在测试运行期间停止,文件同样会留在那里,列表以同样的方式打开。

当窗口不是由游戏的商店启动的时,Test run 会像 Run 一样把游戏交给那个商店(Steam、Heroic 或 Legendary),商店启动的那次运行就是测试运行。如果商店没有启动游戏,这个请求会保持等待:该配置档的下一次游戏运行(无论来自 Run 还是来自商店)就是测试运行,Cancel the waiting test run 会丢弃它。测试运行只覆盖上面列出的文件夹:游戏的其他写入,例如 Wine 注册表,仍去它们一贯去的地方。测试运行的存档只有在你保留它们、并进行一次正常运行之后,才会到达 Steam Cloud。

### 找出问题背后的模组

Diagnostics 标签页上的 Find the mod behind a problem,用来找出导致某个你能复现的问题(例如在某处崩溃)的模组。每一步都是一次测试运行,带上剩余嫌疑对象的一半、它们所需的 master,以及你保持始终开启的模组:开始时在列表里选中且已启用的模组,和提供 script extender 的模组。每次运行之后,你回答问题是否出现。Not sure 会用不同方式拆分嫌疑对象,Run it again 重复这一步。回答或按 Run it again 时,会在第二次按下后丢弃这一步写下的文件。想保留其中一些,请在回答之前按 See the files:它打开与测试运行之后相同的列表。当游戏崩溃且崩溃记录器写了日志,或游戏以错误码退出时,Eidos 会把 Yes 标为可能的答案。200 个嫌疑对象大约需要 8 次运行。

这次搜索保存在 `<instance>/bisect.json` 里,所以你可以在两步之间关闭 Eidos,而且它属于开始时所在的配置档。Stop the search 在第二次按下后丢弃它。结果会点出那个模组,或者彼此的 master 互相需要的一组模组,并保存在 `<instance>/logs/bisect-<time>.txt` 里。生成的模组(Nemesis 或 DynDOLOD 的输出、Bashed Patch)是为完整列表构建的,只带一半时可能失败:要么让它们始终开启,要么在整个搜索期间保持关闭。

## Wabbajack 列表

Eidos 没有 Wabbajack 安装器。请在 Eidos 之外用 CLF3 或 Jackify 安装列表,然后在向导里把安装文件夹选为便携实例。Eidos 按 MO2 的布局读取该文件夹:`mods/`、`profiles/<name>/modlist.txt` 和插件列表。它保留安装器写下的 `ModOrganizer.ini` 并且从不覆盖它,所以列表自带的可执行文件仍保持登记。

许多列表会加一个 `Stock Game` 或 `Game Root` 文件夹,里面放着打过补丁的游戏副本,MO2 从那里运行游戏。当向导或 `eidos init` 第一次接管一个列表文件夹,且该文件夹里有游戏的可执行文件时,实例也从它运行游戏。游戏仍然通过 Steam 或拥有它的商店启动,所以什么都不必搬动。Eidos 根据列表的 `ModOrganizer.ini` 里的 `gamePath`,或者根据这两个名字找到该文件夹。用较早版本的 Eidos 接管的实例会保留商店安装,直到你切换它(见下一节)。当那个 `gamePath` 指向实例并不运行的游戏文件夹时,Diagnostics 会说明,因为 PGPatcher 这类 Mod Organizer 2 模式的工具从那里读取游戏。CLF3 用它自己的工具重建 BSA 和 DDS 文件,所以结果可能与 Wabbajack 安装不同,列表作者也不支持它。

## 游戏文件:商店安装或副本

默认情况下,游戏从它的商店安装运行,也就是 Steam、Heroic 或 Legendary 存放它的文件夹。实例也可以改为从实例里那个文件夹的副本运行。模组叠在副本之上,商店安装只是 Eidos 挂载结果的地方。游戏仍由商店启动,所以 Steam 保留它的覆盖层、着色器缓存和云存档。

Diagnostics 标签页顶部的 Game files 卡片显示实例运行的是哪些文件,附上副本和商店安装的游戏版本,并在两者之间切换。在终端里,`eidos stock-game <instance>` 打印同样的信息,`use <folder>`、`store`、`freeze` 和 `delete --yes` 做卡片按钮所做的事。`freeze` 打印大小和副本的类型,`freeze --yes` 执行复制。

**Freeze this version** 把商店安装复制到 `<instance>/Stock Game` 并从它运行游戏,所以商店更新不再弄坏 script extender 插件:更新改变的是商店安装,而不是副本。第一次按下显示大小以及文件系统会怎么处理它,第二次按下才复制。在能共享数据块(reflink)的文件系统上,例如 btrfs 或 XFS,副本在商店改动某个文件之前几乎不占空间。在 ext4 上,或者游戏和实例位于不同文件系统时,它是与游戏文件夹一样大的完整复制。在 Steam 报告游戏正忙或没有完整安装时,在实例的某个游戏或工具正在运行时,在实例已经持有列表的游戏副本、冻结的副本或未完成的副本时,以及在文件系统的剩余空间小于完整复制所需时,Eidos 会拒绝。Heroic 和 Legendary 的副本不检查是否有更新正在进行。复制过程中停止、失败,或 Steam 更新了游戏,会留下 `Stock Game.eidos-tmp`,卡片会提示删除它;它绝不会被使用。

副本会隐藏商店在复制之后对游戏文件夹所做的每一处新增、改动或删除:新的 Creation Club 文件、新的可执行文件、新的 DLL。游戏、插件列表、LOOT 和 Health 检查读取的都是副本。它不会隐藏游戏文件夹之外发生的事:更新仍可能在下次启动时把运行时装进 Proton 前缀,Steam 也可能把游戏切换到另一个 Proton。

**Return to the store version** 在第二次按下后,把副本从下一次启动中拿掉。副本仍留在磁盘上,**Delete the copy** 会移除它,同样要按第二次。实例仍在运行这份副本时,Delete 会被拒绝。Eidos 绝不会删除列表自带的 `Stock Game` 或 `Game Root` 文件夹。如果副本文件夹丢失,或者里面不再有游戏的可执行文件,启动会被拒绝,Diagnostics 会点出那个文件夹,所以商店版本不会出人意料地运行。

Morrowind,以及 `bUseMyGamesDirectory=0` 的 Oblivion,把插件列表放在游戏文件夹里,Eidos 目前不从副本运行它们。

## 安装 Nexus 集合

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # 读取它并说明将会发生什么
eidos collection skyrimse rqhcxy --no-optional  # 跳过它标为可选的成员
```

在窗口里,把链接粘贴到 **File -> Open a Nexus collection...**,或者在网站上点一个集合。
Eidos 列出该修订版的成员,并逐个与本实例比对:已安装、已下载或缺失。*Install this
collection* 在工作线程上运行完整的配方,使用打包和解包任务所用的进度条,完成时进度条会变成
报告。窗口和 `eidos collection` 运行的是同一个安装过程。

不给链接时,**File -> Open a Nexus collection...** 会在 **Browse** 标签页打开。它列出为打开
的实例的游戏发布的集合,每次 20 个,可按下载最多、最近更新或最新排序,搜索框把列表缩小到名称
或文字匹配的集合。每一行给出作者、最新修订版、模组数量、下载量和日期。**View** 在 **Link**
标签页打开该集合的最新修订版,与粘贴它的链接相同。标为成人内容的集合会被排除,除非你的
Nexus 账户显示成人内容;没有评分的集合总是被排除,列表下面的那一行会把两者都统计出来。浏览
需要已登录的 Nexus 账户,和这个窗格的其余部分一样。

集合只能对它自己游戏的实例打开。加载着 Fallout 4 实例时打开 Skyrim 集合会按名字被拒绝,而不是
拿它的成员去匹配错误的模组列表。

### Eidos 从集合里读取什么

集合的内容几乎都无法通过 Nexus API 看到。安装顺序、作者为每个模组的脚本安装器给出的回答、
哪个模组赢得文件冲突、哪些插件加载在哪里,以及二进制补丁,全都在集合自己档案里的一个
`collection.json` 中。下载同样的模组并用默认选项安装,得到的是另一个游戏。

Eidos 下载这个档案,读取 `collection.json` 并应用它:

| 集合写的是 | Eidos 做的是 |
| --- | --- |
| `phase` | 按该顺序安装,可选成员最后 |
| `choices` | 重放作者对每个 FOMOD 的回答,无法重放时会说明 |
| `modRules` | 重新排列模组列表,让正确的模组赢得每个文件 |
| `plugins` / `pluginRules` | 把 LOOT 规则合并进你的 userlist 并排序 |
| `INI Tweaks/` | 把它们安装成一个独立的模组 |
| `tools` | 告诉你它期望哪些工具;不创建任何东西 |

成员在安装时就被启用,按安装顺序放在模组列表末尾,然后由 `modRules` 来移动它们。被排除在
模组列表之外的成员不会加载,而报告也不会显示它。

每装完一个成员,状态就会写入 `<instance>/collections/<slug>-<revision>.state.json`。它放在
该修订版文件夹的旁边,绝不在里面,所以集合无法自带一个状态文件,而在 200 个成员中的第 147 个
被中断的安装,会从 147 继续。它先写成临时名字再重命名到位,所以中断留下的是旧记录或新记录,
绝不是任何一个的一半。如果它哪天读不出来,Eidos 会停下并说明,而不是把整个集合从头再来。再运行
一次同样的命令即可。

安装一个你已有集合的较新修订版时,会接管上一个修订版的文件夹:没有变化的成员原地校验,作者
更新了文件的成员在它旧文件夹里被替换,而新修订版丢弃的成员会保持已安装,并在报告里点出,让你
自己禁用或移除。另一个配置档启用着的文件夹永远不会被接管,因为模组文件夹是共享的,那个配置档
会在你脚下发生变化:新修订版会在它旁边安装自己的副本,而报告会请你在这个配置档里禁用旧的,
绝不是移除它。回到较旧的修订版,会用同样的方式拿回它的文件夹,或者为拿不回来的安装全新的
副本。每当一个成员以这种方式被替换、而你曾在它的文件夹里改动或添加过文件时,旧文件夹会先以
`<name>_backup` 的名字保留在它旁边,它绝不会加载进游戏,而报告会点出它。

来自较早修订版、仍保持启用的残留,会让这次安装不算作作者构建的那一个;你把它禁用之后,这条说明
就会消失。用 `--no-optional` 跳过的成员不会被接管,并以同样方式被点出。如果接管本身失败,运行会
以 *previous revision: its folders could not be taken over* 和原因停止;修好之后再运行一次命令。

### 免费账户与需要你自己获取的成员

免费的 Nexus 账户无法获取成员模组。Nexus 不通过 API 给免费账户下载链接:每个文件都需要网站自己
的 "Mod Manager Download" 按钮。集合的档案在免费账户上可以下载,所以你能读到整份配方,但每个成员
都会连同它确切的页面 URL 被报告出来,让你自己去获取,重试也不会改变这一点。请通过网站的按钮获取
它们,然后再运行一次命令:每次运行都会在 `downloads/` 里查找这些成员,所以集合会随着你提供它们
而完成。

在窗口里,免费账户在集合窗格里得到 *Fetch the N missing required files, one by one*,以代替批量
获取。它打开第一个缺失必需文件在 Nexus 上的页面,等待那个文件落进 `downloads/`,然后打开下一个。
你仍然要为每个文件按页面上的下载按钮。窗格显示进度(*3 of 12*),带有 *Skip* 和 *Stop*。落下来
的是同一模组的另一个文件时,窗格会点出两个文件 id,并继续等待指定的那个。可选成员不属于它,保留
各自的 *Open* 按钮。

Eidos 还会应用二进制补丁(每个都按记录的 CRC 校验)、文件覆盖(被点名的文件用 `.mohidden` 隐藏,
与 MO2 相同),以及文件随集合档案一起带来的成员(`bundle`)。集合期望你手动获取的成员(`browse`、
`manual`)会把作者自己的说明带到报告里。名字已经是你某个模组的成员,会以一个空闲的名字安装在它
旁边,绝不覆盖它,报告会说明是哪一个。

### 报告

安装器回答没能全部重放的成员,会单独得到一行 *installed, but not the way the collection asks*,
与按作者构建方式安装的成员分开,因为它在磁盘上的文件与作者的不同。报告还会列出没有指向集合中
任何东西的排序规则、陷入互相矛盾的规则循环中的模组、不存在的 LOOT 组,以及这个版本的 Eidos 不
理解的清单部分。

### 保持集合顺序

安装之后,Eidos 会保留每个已安装集合的规则。当你之后违背某条 before 或 after 规则移动了它的
两个模组时,Diagnostics 标签页会为该集合显示一张卡片,点出第一对这样的模组,并统计不再成立的
规则数量。只有当前配置档里启用的模组才算数,陷入循环的规则会被跳过,因为没有任何顺序能满足它们。
**Restore collection order** 会把集合的模组移回一个满足每条规则的顺序,在规则允许的地方保留你
自己的顺序,并让不在集合里的模组留在原处。Ctrl+Z 可以撤销。`eidos check` 把同样的结论列为
`collection_order_broken`。这项检查读取 `collections/` 里该修订版的配方,所以缓存的配方已被移除
的集合不会被检查。

## 把实例搬到另一台机器

`eidos pack` 把整个实例(每个模组、加载顺序、所有配置档、带着你存档的 Overwrite,以及所有东西
安装自的档案)写进一个文件。`eidos unpack` 把它放回去:

```sh
eidos pack skyrimse ~/backup.eidos                  # 或者给一个文件夹:以游戏和日期命名
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # 在另一台机器上
```

`eidos pack --dry-run` 精确列出会放进去什么、不会放进去什么及原因,以及需要多少空间,而不写任何
东西。`eidos unpack --info` 对你已有的文件做同样的事。

如果 7-Zip 有东西没读出来,或者遍历时不得不跳过你本期望出现在备份里的东西(符号链接、读不了
的文件夹、无法安全地交给 7-Zip 的名字),档案仍会以你选的名字写出,但 `eidos pack` 会以 1 退出
并说明缺了什么,所以运行 `eidos pack ... && ...` 的脚本会停在那里。

`.eidos` 文件是一个用自己扩展名的 7-Zip 档案。安装模组本来就需要 7-Zip,所以这没有增加依赖,而且
备份在任何能读 7-Zip 的压缩软件里不用 Eidos 也能打开。它用 LZMA2 以 `-mx1` 压缩,在一个贴图
很多的列表上压到原来的大约 43%;`-mx9` 要慢得多,只多省几个百分点。贴图和网格本来就是已压缩的
格式,所以更大的字典几乎找不到还能压缩的东西。`--level`(0、1、3、5、7 或 9)选择另一个级别,
`--no-downloads` 把 `downloads/` 排除在外。

在默认的级别 1,文件进入最大 16 MB 的固实块。从一个 70 GB 的备份里取出一个文件,最多只解码它所在
的那个块,绝不解码它前面的所有东西,而且打包游戏内容比每个文件单独压缩快 2 到 5 倍(解包快 2 到
4 倍)。代价在于损坏:一个损坏的字节可能带走它所在块里的每个文件,最多是 16 MB 的相邻文件。下面
说的恢复数据就覆盖这种情况。其他每个级别都保持非固实:每个文件是自己的流,一个损坏的字节只损失
那一个文件。级别 0 只存储文件。从级别 3 起,LZMA2 已经以 16 MB 或更大的块工作,而固实块在那里
测得更慢:在级别 5,打包网格用了 1.4 倍的时间,解包贴图用了 6.8 倍的时间。

打包会取得实例锁,所以游戏或另一个 Eidos 正在写这个实例时无法运行,而且读取期间没有任何东西
会改动实例。备份先以 `.part` 的名字写出,刷到磁盘,并用 `7z t` 整个读回,然后才得到你选的名字。
这个检查会解码每一个块并比对每一个校验和,与另一台机器上的解包做的是同样的工作。它显示的是写入
之后磁盘立刻交还的内容,而不是驱动器会不会一直保存它;下面的恢复数据就是为此准备的。如果它失败,
`.part` 会被删除,你的名字下不会写入任何东西,`eidos pack` 会引用 7-Zip 发现损坏的那一行。被中断
或失败的打包留下的是一个 `.part` 文件或什么也没有,绝不会是一个看起来完整其实不完整的 `.eidos`
文件。

安装了 par2cmdline 时(Arch 上 `sudo pacman -S par2cmdline`,Debian 和 Ubuntu 上
`sudo apt install par2`,Fedora 上 `sudo dnf install par2cmdline`),`eidos pack` 还会在档案旁边
写 PAR2 恢复数据:`backup.eidos.par2` 和几个 `backup.eidos.volNN+MM.par2` 文件,是档案大小的 5%
外加一个小索引。请把它们和 `.eidos` 文件放在同一个文件夹里,保持写出时的名字,并一起复制:没有
它们而被搬走的备份无从修复,而且恢复数据记录着档案的名字,所以改过名的备份就不再与它匹配。
`eidos unpack` 按名字找到它们,并在解压任何东西之前用它们检查档案,包括尾部被截断、导致 7-Zip
打不开的备份。如果有几个字节坏了,par2 会先修复档案,并把损坏的那份保留为 `backup.eidos.1`,
这需要该文件夹里有与档案同样大小的空闲空间;超出 5% 所能覆盖的损坏,会在写出任何一个文件之前
就停止解包。与档案不匹配的恢复文件(例如同名的旧备份留下的),在 7-Zip 读取档案并发现它完好
时不会被使用,解包会说明这一点。在没有 par2 的机器上,解包仍会运行,并警告没有使用恢复数据。
`--no-recovery` 不带它打包,`eidos pack --dry-run` 会说明它是否会被写出。`--force` 替换同名的
现有文件。

### 其中不包括什么,以及原因

| 被排除的 | 原因 |
| --- | --- |
| Proton 前缀 | 它与机器相关(驱动器映射、*这个*文件系统的 `Z:` 视图),而 Eidos 用 `eidos prereqs <instance> --install` 一分钟就能重建它。带上它会让档案大约翻倍,只为运送另一台机器反正要重新生成的东西。 |
| 游戏 | 实例修改的是它自己并不包含的游戏。请在那边从 Steam 安装。 |
| 实例之外的工具 | xEdit、DynDOLOD 和其他工具是带有自己安装程序的第三方二进制文件。它们的名字会写在档案里,所以解包会准确告诉你新机器上缺哪些。 |
| `logs/`、`.eidos.lock`、`prereqs.done`、`prereqs.log` | 制作备份那台机器的记录。尤其是 `prereqs.done`,它会声称运行库已经装在一个并不存在的前缀里。 |
| `loot/`(`userlist.yaml` 除外) | Eidos 会按需重新获取的 masterlist 缓存。你自己写在 `userlist.yaml` 里的 LOOT 规则无法重新获取,所以这一个文件会放进去。 |
| `.base/`、`.base-root/` | 会话期间用来存放游戏自己文件的空挂载点。 |
| `test-run/`、`bisect.json` | 等待复查的[测试运行](#测试运行),以及两步之间的、寻找问题背后模组的搜索。两者都属于这台机器上的运行。 |
| 写了一半的文件 | 暂停的下载(`*.unfinished`)、进行中的原子写入(`*.eidos-tmp*`)。 |
| 符号链接 | 7-Zip 会跟随它并复制它指向的任何东西,对于绝对链接,这意味着把别处的目录树拉进你的备份。它们改为被报告出来,并且这份备份算作不完整:链接到另一块硬盘的 `mods/` 或 `downloads/` 不会被打包。要有意在没有链接的 `downloads/` 的情况下备份,请传 `--no-downloads`(或在窗口里取消勾选 *Include downloads/*);有意的排除不算缺失。 |

实例里的游戏副本(`Stock Game`)和其他文件夹一样被打包,所以冻结过的实例的备份大约要大出一个游戏。
未完成的副本(`Stock Game.eidos-tmp`)被排除。

每一项都连同原因列在档案根目录的 `eidos-backup.ini` 里,所以读那个文件就能看到什么被排除了。同一个
文件还记录实例原来所在的位置、哪个配置档是活动的、解包后有多大,以及新机器会需要哪些工具。

下载记录(`downloads/*.meta`)放进去时会清空它的 `url=` 行。那个 URL 是一个签名的 Nexus 链接,几个小时
内就会失效,并带着下载该文件的账户的 `user_id`,而备份可能会交给别人。模组 id、文件 id 和版本会保留,
所以下载可以再找到。

### 在窗口里

两者都在 **File** 菜单里:*Pack this instance...* 和 *Unpack a backup...*。解包也在欢迎页的实例列表
下面,所以还没有任何实例的机器也能解出一个。

在写任何东西之前,Pack 对话框会显示有多少文件、多大、其中 `downloads/` 占多少、档案大约会是多少,
以及哪些工具被点名但没有携带。它的 *Write recovery data* 复选框在安装了 par2 时是勾选的,没有时则
灰掉并给出该安装的软件包。Unpack 对话框在你选好文件后就读取备份的清单,所以在你选择文件夹之前,它
就能告诉你它装的是哪个游戏、何时制作、原来在哪里,以及它的哪些工具在这里缺失。

两者都在工作线程上带着进度条运行,完成时进度条会变成报告。

### 解包会修复什么

直接复制文件夹,会让每个工具按钮都指向旧机器上的路径。`eidos unpack` 只改写那些写着旧实例根目录的
值:`tools.ini` 里的 `exe`、`workdir` 和带编号的 `arg` 键,以及每个模组 `meta.ini` 里的
`installationFile`。从未在实例里面的工具(`/opt/xedit/SSEEdit.exe`)会保持原样,并被列为需要重新安装
的东西;Eidos 不会去猜那个工具在新机器上的位置。它还会修正实例自称是中心实例还是便携实例,这样之后
的命令会去它现在所在的地方找它。

解包会拒绝非空的文件夹(如果你确实想覆盖,传 `--force`)、小于清单自己所给数字的磁盘、由比你的更新的
Eidos 制作的档案,以及任何含有会写到你所选文件夹*之外*的条目的档案。不给文件夹时,全局实例的备份
会回到它的游戏对应的中心位置;便携实例的备份则需要一个文件夹。

之后:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # 重建 Proton 前缀
eidos play /mnt/games/EidosSkyrim
```

## 窗口

启动 `eidos-gui`,或 Colony 安装的 `eidos-linux` 文件。

第一次启动会打开一个像 MO2 那样的向导:欢迎、实例类型(便携或全局)、游戏、名称和位置、摘要,然后
是主窗口。欢迎页还列出每个已知的实例,全局的和便携的,最近使用的在前,一次点击即可打开,所以它同时
也是实例切换器。把向导指向一个已经有实例的文件夹,会按原样打开那个实例,而不是在它上面再创建一个,
属于另一款游戏的文件夹会被拒绝。

主窗口有两个窗格:模组列表,以及 Data、Plugins、Conflicts、Overwrite、Archives、Saves、Downloads
和 Diagnostics 标签页,另有一个 Run 按钮、一个 Test run 按钮([测试运行](#测试运行))和一个运行目标
选择器。你可以筛选模组列表、选择、重新排序、用分隔符分组、按类别缩小范围,并右键执行操作。除了
置顶和置底,模组菜单还有 MO2 那些有针对性的移动:移到第一个冲突的模组之上、最后一个之下、移到指定
优先级,或移进某个分隔符的组里。

配置档选择器用来切换配置档。右键点配置档可以 Rename、Copy to new...、Desktop shortcut、
Import from MO2... 和删除,删除要点两下,并拒绝删除当前的或最后一个配置档。Import from MO2... 会接管
MO2 配置档的模组顺序和加载顺序。Desktop shortcut 写一个桌面入口,运行
`eidos tool <instance> run "<game>" --profile <name>`:它把那个配置档设为当前配置档,并请商店启动
游戏(细节见 [tools.md](tools.md#够到一个工具隐藏置顶以及桌面快捷方式))。

**File** 菜单还会在游戏的前缀里运行 winecfg、regedit、Wine 文件管理器和卸载程序,Prefix drive_c
打开前缀的 `C:` 盘;见 [tools.md](tools.md#游戏前缀中的-wine-自带程序)。
*Instances...* 列出 Eidos 知道的实例;Forget 把其中一个从列表里去掉,不动磁盘上的任何东西。
*Export the mod list...* 以 MO2 的 CSV 格式写出列表,与 `eidos export` 相同。

### 偏好设置

点击左上角的 **Eidos**,就用偏好设置取代两个窗格,再点一次(或点 **Close**)返回。General、
Appearance 和 Accessibility 排在最前,和每个 Colony 程序一样,然后是 Mod list、Nexus 和 About。
每个改动都立即生效并保存;没有 Save 按钮。

- General:Eidos 打开的游戏、窗口大小、界面语言、工具文件夹,以及游戏怎么运行。游戏运行时锁定窗口
  默认开启。Start the game at once when a store opens Eidos,以及在窗口处于后台时告知长时间任务何时
  结束的桌面通知,默认关闭。窗口使用英语和法语;System 跟随 `LC_ALL`、`LC_MESSAGES` 或 `LANG`,
  改动立即生效。`eidos` 命令行始终用英语回答。
- Appearance:主题(26 个系列、59 个配色,默认是 Eidos 的羊皮纸)、强调色、文字大小、效果和实时
  预览。
- Accessibility:高对比度、适合阅读障碍者的字体、减少动效,以及与 Appearance 里那个相乘的阅读文字
  大小。
- Mod list:把模组拖到列表顶部或底部边缘时的自动滚动速度。
- Nexus:登录、下载文件夹、`nxm://` 处理器及其 Register 按钮、首选服务器、Install downloads when
  they finish,以及 Offline mode。
- About:文件放在哪里、键盘快捷键、版本和发行版检查。

外观和界面语言保存在 `~/.config/Colony/Eidos/preferences.json`。其他设置(命令行也会读取:默认游戏、
工具文件夹、Nexus 服务器、Offline mode 等)保存在它旁边的 `settings.ini`,Nexus 会话则在 `nexus.ini`。
旧版 Eidos 放在 `settings.ini` 里的主题会在第一次时被带过来。

**About -> Version** 里有发行版检查,默认开启。启动时,每天最多一次,Eidos 向 GitHub 询问
`Project-Colony/Eidos` 的最新发行版。当它比正在运行的副本更新时,菜单栏会在 Help 之后显示
`Eidos <version> is available: release notes`,该链接会打开那个发行版的页面及其更新日志。Eidos 从不
自行下载或替换自己。离线模式下会跳过这项检查。由包管理器装在 `/usr` 下的副本会关闭它,因为包管理器会
更新它;`install.sh --system` 装 Eidos 的 `/usr/local` 算作你自己的安装。

### 列、排序和分组

列表默认显示四列(Category、Content、Version 和 Flags,与 MO2 显示的相同),可提供八列:Category、
Content、Version、Author、Installed、Nexus id、Game、Flags。在 View 菜单里勾选它们。八列全开时,
名称列所剩的空间很小。

点击列标题按它排序,再点一次反向,第三次点击则回到加载顺序。列表只能在加载顺序下拖动:放置位置指向的
是真正的列表,而在排序状态下,一行在屏幕上的位置并不是它在那个列表里的位置。排序开启时,不绘制插入
条,拖动也会被拒绝,与 MO2 一样。View 菜单会这么说,并给出返回的办法。

View 菜单还能给整个列表分组,按类别或按来源(来自 Nexus,或手动安装)。分组标题不是分隔符:它们不能
重命名、着色或移动。它们可以折叠,折叠的标题保留它的计数。排序或分组开启时分隔符会被隐藏,因为分隔符
统领的是加载顺序里跟在它后面的那些行,而两者都会改变屏幕上的这个顺序。

### 鼠标和键盘

双击模组打开 Information,Ctrl+双击打开它的文件夹,Shift+双击打开它的 Nexus 页面。Ctrl+F 把光标
放进筛选框。键入一个字母会跳到下一个以它开头的模组,再按一次会继续走过其余的,而不是停在第一个上。
它们都不会落在被筛选、折叠的分隔符或折叠的分组隐藏的行上,所以下一次 Space 绝不会切换一个你看不见的
模组。

分隔符菜单上的 "Collapse others" 会折叠除了它之外的所有组。拖动期间,让指针在一个折叠的组上方保持
不动会把它展开,这样模组就能放进里面;只是经过它不会。

Ctrl+Z 撤销对模组列表或加载顺序的上一次改动:一次勾选、一次移动、一次批量禁用、一次 LOOT 排序。
Ctrl+Shift+Z 重做。窗口保留 50 步,并在 F5、游戏或工具运行结束、切换配置档,以及另一个 Eidos 进程
改动列表时忘掉它们。安装、重命名或移除模组会改变磁盘上的文件夹,所以越过其中任何一步的撤销会被拒绝。

### 筛选列表

Filters 按钮为每个条件打开一行:active、conflicted、update available、no category、endorsed、
tracked、has a Nexus id、has hidden files、no valid game data、for another game、has notes,以及每种内容
一行(插件、BSA 或 BA2 档案、贴图、网格、脚本、script extender 插件、界面文件、声音、FaceGen)。点击
会让一行在 off、only 和 except 之间循环。最上面那一行在匹配所有开启的条件与匹配其中任何一个之间切换。
名称框和类别列表总会进一步缩小结果。has hidden files 这一行只看已启用的模组。

### 列表告诉你关于模组的什么

两个建议性标记,都是一个图标,悬停时显示说明。No valid game data 表示模组顶层没有任何东西像是这款游戏会
加载的;它可能需要把文件夹上移一层,也可能根本不是这款游戏的模组。Another game 表示模组自己的
`meta.ini` 写的是另一款游戏。两者都不阻止任何事:模组仍会部署。行菜单上的 "Mark as valid" 通过 MO2 自己的
`validated=` 键让两者都不再提示,所以在一个管理器里标为有效的模组,在另一个里不会被标记。

布局检查只在确定时才标记模组:`Root/` 树、读不了的文件夹和空文件夹都算作有效的游戏数据。

### 禁用你的存档或其他模组需要的模组

在禁用或移除一个模组之前,Eidos 会读取配置档最新三个存档的插件列表。当其中一个用到了该模组的插件时,
列表上方的一条栏会点出这些插件,并等待 Disable anyway 或 Cancel(Escape 也是取消)。Remove 会把同样的
名字加进它那条再点一次的消息里。只读取 Skyrim 和 Fallout 4 的存档;其他游戏会不询问就禁用。

模组 Information 的 General 标签页,列出它需要的模组和需要它的模组,依据已启用插件的 master 读出。禁用
一个被其他模组需要的模组时,会显示同样的栏,带有 Disable them too(它还会禁用每个需要它的模组,以及需要
那些模组的模组)或 Keep them enabled。Diagnostics 标签页上的缺失 master 卡片有 Enable what provides them:
它启用持有每个缺失 master 的模组,并勾选一个存在但未勾选的 master 插件。

### 动手之前先备份一个模组

"Back up this mod" 把它的文件夹复制到旁边,名为 `<name>_backup`(然后是 `_backup2`,依此类推;备份绝不会
替换上一个)。这份副本不是模组:它的复选框不起作用,也不会向合并视图添加任何东西,因为勾选它会把同一个
模组的两份副本部署到一起互相覆盖。"Restore this backup over the mod" 把它放回去,要点两下;当前内容会先被
移到一边,复制成功之后才丢弃。

工具栏上的 **Backups** 为配置档的模组列表、加载顺序(`plugins.txt` 和 `loadorder.txt` 一起)和它的游戏
INI 保留还原点,每种 10 个,放在 `<profile>/backups/`。Back up now 添加一个,Restore 会先备份当前状态,所以选
错了也能撤销。Eidos 还会在每次 BethINI 运行之前保留一个 INI 还原点(见
[tools.md](tools.md#bethini-pie))。

### 标签页

**Data** 是合并视图的树,一次展开一层:打开一个文件夹,只读取每个有它的层里的那个文件夹,绝不一次读取每个
已启用的模组。它由挂载所使用的同一个层栈构建,所以 whiteout 和隐藏文件都会被遵守,这个标签页显示的就是
游戏会看到的东西。可以按名称筛选、缩小到只看有争议的文件、用 Size 和 Modified 列理清什么在哪里,并在文件
管理器里 Reveal 任何一行。悬停在一个文件上会列出提供它的其他模组。模组顶层的 `meta.ini`、`readme.txt`、
`fomod/` 和 `Root/` 不在 Data 里,无论在这个标签页还是挂载中:前三个是给模组管理器用的,`Root/` 则改为进入游戏
文件夹。游戏或工具写在这些名字之下的文件会落进 Overwrite 并保持可见。

Data 里的 `.exe` 行有 Add tool(打开填好该文件的 Executables 编辑器,按 Save 之前不会保存任何东西)和 Run
(像登记过的工具那样穿过合并视图把它运行一次,见[工具](#工具))。Save tree 把整个合并后的 Data 树写成文本
文件,每个文件一行并注明提供它的模组,你的主目录和用户名会被去掉,用于缺陷报告。

**Plugins** 是 ESP/ESM/ESL 加载顺序:切换、手动重新排序,或用 LOOT 排序并阅读排序后的报告,其中的建议链接会
在你的浏览器里打开。它的标题统计每个索引空间里已启用的插件数量与游戏上限,例如
`full 231/254, light 512/4096`,有一个已满时变成橙色。筛选框按名称缩小列表。Space、Ctrl+A、Shift 范围选择,
以及行菜单的 Activate all shown 和 Deactivate all shown,都只作用于显示出来的行,而且在清除筛选之前拖动会被
拒绝。选中一个插件会给插件列表里它的 master 着色,并给模组列表里提供它的模组着色。被 LOOT 标记为 dirty 的插件
会得到一张可以用 xEdit 清理它们的 Diagnostics 卡片([tools.md](tools.md#清理-loot-标记为-dirty-的插件))。

LOOT 排序之后,新记录都落在 light 范围内的 full 插件会在悬停时这样提示,它的行菜单里有 Flag as light。Eidos 把
带有 light 标志的副本写进列表底部(最高优先级)一个名为 `<plugin> (light)` 的新模组里,并保持原件不动,所以禁用
那个模组就能撤销。当配置档最新三个存档之一把该插件作为 full 插件加载时,它会拒绝。只读取 Skyrim 和 Fallout 4
的存档,所以 Starfield 上不检查任何存档。LOOT 只检查记录范围:标记一个添加了 cell 或 worldspace 的插件之前,请先
在 xEdit 里看看它。

悬停在插件上会显示它来自哪里、它的作者、它的头部和表单版本、描述的前 300 个字符、它的 master,以及在 LOOT
排序之后 LOOT 的消息。在 Skyrim、Skyrim SE、Skyrim VR 和 Enderal SE 上,游戏还会读取以已加载插件命名的 INI;INI
列会标出这些插件,点击会以只读方式打开那个文件。

**Conflicts** 解释每个文件的赢家和输家。**Overwrite** 一步就把游戏写下的东西变成一个真正的模组。

**Archives** 列出已启用模组带有的每个 BSA 或 BA2、它来自哪个模组,以及它为何加载:同名的活动插件,或者 INI
注册了它。没有任何东西加载的会被标为累赘。当另一个模组带有同名并赢得冲突的档案时,该行会点出那个模组。
Extract 把档案解包到一个以它去掉扩展名后的名字命名的新模组里(`Foo.ba2` 得到 `Foo (extracted)`),并把档案
留在它的模组里;散装文件会胜过任何档案里的文件,不论模组顺序如何。

**Saves** 读取每个存档的头部(角色、等级、位置、游戏时长),把其中存储的插件列表与当前的比较,并有一个启用它所需
模组的按钮。不经 Eidos 进入前缀的存档,在[存档属于配置档](#存档属于配置档)里说明。

**Downloads** 和 **Diagnostics** 在下面各有自己的一节。

### 模组信息

"Information..." 打开一个按模组的对话框,带有 General、Conflicts、Filetree、INI Tweaks、Notes、Nexus files 和
Nexus page 标签页。在 filetree(以及 Data 树)里,任何文件都可以被隐藏:它被重命名为 `<name>.mohidden`,
这会把它从合并视图里拿掉而不删除它,所以可以在不改变优先级的情况下压制某个模组的几个散落文件。filetree 也做普通的
文件操作:新建文件夹、重命名、删除、打开。每一个都会拒绝不是模组内部普通路径的路径:不能有 `..`,不能是绝对路径,
路径途中不能有符号链接,因为跟随一个可能会删掉模组文件夹之外的文件。Rename 只改路径的最后一部分,所以绝不移动
文件,并且拒绝已被占用的名字。Delete 要点两下,且无法撤销。

filetree 或 Data 树里任何一行上的 **View** 会预览该文件:图片、文本、DDS 贴图(选择 mip、层、面和通道)以及带有
漫反射贴图的静态 NIF 模型。NIF 预览需要 Eidos 旁边或 `PATH` 上有 `eidos-nif-preview`;发行版捆绑包、Arch 软件包
和源码构建有它,`eidos-linux` 没有。文本最多读取 64 KB,并说明停在哪里。

**INI Tweaks** 列出模组在它的 `INI Tweaks/` 文件夹里带的片段;已启用的会在启动时按优先级顺序合并进配置档的游戏
INI,并在捕获这次运行的 INI 时被拿出来,所以禁用一个 tweak 就会移除它的设置。

**Nexus files** 标签页按最新在前列出该模组在 Nexus 上的文件,并标出这里安装的那个和替换它的那个。第二行有一个
**Update** 按钮;见[模组更新](#模组更新)。

**Nexus page** 标签页显示模组的描述,以及它页面上每个文件版本的更新日志,最新在前。Eidos 在你第一次打开这个
标签页时,通过 Nexus 的 GraphQL API 读取两者,这不计入每小时的请求额度。它保留它们,直到 Eidos 关闭或你按
**Refresh**。它们以纯文本显示:BBCode 和 HTML 格式被去掉,链接的地址写在它的文字之后,图片和视频被省略。文字上方
的 **Endorse** 和 **Track** 做的事与模组右键菜单里的相同。成人模组的页面保持隐藏,除非你的 Nexus 账户显示成人内容,
没有评分的页面在任何情况下都保持隐藏。

### 下载

可以把下载从 Downloads 列表拖到模组列表中的某个位置,按那个优先级安装它,从文件管理器拖到窗口上的档案或
文件夹同样会安装。文件拖放需要 X11 或 XWayland 会话:winit 只对 X11 实现了它。下载可以暂停和继续:暂停会停止
传输并保留部分文件,Resume 会向 Nexus 要一个新链接并从中断处继续。

Downloads 标签页保存你下载的档案。可以按名称筛选(友好的模组名也行,所以 "skyui" 能找到
`SkyUI_5_2_SE-12604-5-2SE.7z`),按最新、名称、大小或状态排序,并隐藏你用完的档案:文件仍在,只是那一行消失。
"Show hidden" 把它们带回来,同一个按钮会取消隐藏。"Remove N installed" 删除你已经安装的模组的档案,要点两下,
而且只删除筛选所显示的那些。

**Preferences -> Nexus -> Install downloads when they finish** 默认关闭。打开后,窗口开着时完成的 Nexus 下载会
不需要你去 Downloads 标签页就自行安装。它是通常的安装:FOMOD 会打开它的安装器,名字已在模组列表里的会询问怎么
处理。窗口打开之前、或这个选项打开之前就在 `downloads/` 里的档案不会被动,你已取消安装的下载也不会。集合窗格里
打开着集合时,没有任何下载会自行安装,期间完成的下载之后也不会自行安装;集合会安装它的成员。待处理的 Update 所等
的那个文件由 Update 安装。只有 Nexus 下载会这样安装,被 Downloads 筛选隐藏的下载会等到筛选被清除。拖动进行期间,
或游戏或另一个 Eidos 持有实例时,安装会等待。

### 模组更新

工具栏右端附近的更新按钮会检查每个有 Nexus id 的模组。当模组页面显示的是你已安装的版本时,不再多问。否则 Eidos
读取该模组的文件列表(多一次请求,在这次检查的其余时间里保留),并从模组的 `meta.ini` 所记录的文件 id 沿着更新链
走下去:作者把新上传标为旧上传的后继,MO2 读取的是同一条链。链中更新的文件就是更新。当链里没有后继,而已安装的文件
仍列在页面上时,即使页面显示的是另一个版本,这个模组也算作最新。只有在没有记录文件 id、已安装的文件被移除或归档,
或者 Nexus 没有发来文件列表时,才由页面的版本决定。

每个模组使用它自己的 `meta.ini` 所写的游戏的 Nexus 域名(`gameName`),而不是实例的:更新检查、Endorse、Track 和
Visit on Nexus 都去那里。Enderal SE 还接受 `skyrimspecialedition` 的模组,所以装在 Enderal SE 实例里的 Skyrim SE
模组会在它的 Skyrim SE 页面上被检查。当游戏接受链接的域名时,Mod Manager Download 链接会去你上次使用的实例。否则
它会先去域名属于它自己的那款游戏的实例,再去仅仅接受该域名的游戏的实例。`EIDOS_INSTANCE=<folder>` 会覆盖两者。

模组右键菜单里的 Track 和 Untrack 把模组加入你在 Nexus 上的追踪列表,或从中移除,所以网站的通知会跟随你在这里追踪
的东西。同一个菜单里的 Endorse 和 Abstain 在 Nexus 上点赞该模组或撤回点赞;更新按钮旁边的点赞按钮会给选中的模组
点赞。每次更新检查还会从 Nexus 读取你的点赞和追踪的模组(两次请求),并把每个模组的 Endorsed 和 Tracked 标记设为一致,
使用该模组自己的 Nexus 域名。两者不一致时,以 Nexus 为准。如果其中任一列表读不出来,检查会说明,并让那些标记保持原样。

"Information..." 的 Nexus files 标签页上的 **Update** 按钮会安装更新的文件。有高级账户时,Eidos 自己下载文件。免费账户
则会打开文件的页面,由 Mod Manager Download 把文件送回来,档案到达 `downloads/` 时安装开始。从那里起它就是通常的安装:
FOMOD 会按该模组安装时的回答打开,然后提示给出 Replace,并勾选 "Keep a backup of the existing mod",无论你上一次提示时
选了什么。Replace 保留模组的优先级和启用状态。

在 Replace 交换模组的文件夹之前,Eidos 会按文件名和大小比较旧文件夹与新文件夹,并读取两边插件的头部。这涵盖 Update、
集合更新,以及在安装提示处选择的 Replace。状态栏随后统计新增、删除和大小改变的文件数,并点出新版本需要而旧版本不需要的
每个 master。"Information..." 的 **General** 标签页末尾有一个 **Last update** 部分,列出版本、新增或移除的插件、新增或移除
的 master、ESM、ESL 或 ESH 标志改变了的插件、该模组现在与哪些模组共享文件或不再共享,以及这些文件。大小不变而被重写的
文件不会列出。这份报告保存在实例的 `logs/updates/<mod>.json` 里,`eidos pack` 会把它排除;全新安装或 Merge 到该文件夹会把它
移除。`eidos install --replace` 在一行 `changes:` 上打印同样的计数。

含有多个 Nexus 文件的文件夹(例如合并了一个可选文件的主文件)会被拒绝:用新的主文件替换它会丢掉其他文件。请手动更新这些。

`eidos nexus update skyrimse --apply` 运行同样的检查,然后在高级账户上下载每个新文件,并用 `--replace --backup` 把它安装到它
的文件夹之上。免费账户则改为为每个文件页面打印 `OPEN <url>`。合并了多个文件的文件夹会连同原因被跳过。没有记录回答的
FOMOD 模组(例如在 Eidos 保存回答之前安装的)不会用安装器的默认值重新安装:那次更新会失败,并提示先在窗口里更新这个模组一次,
这会记录回答。安装器无需选择的 FOMOD 照常安装。只要有任何更新失败,该命令以 1 退出。

### 集合更新

当你运行 Check for updates 时,Eidos 还会向 Nexus 询问本实例里安装的每个集合的最新修订版。模组列表上方的一行会点出每个有较新修订版的集合,并统计新增(+)、更新(~)和丢弃(-)的模组数;悬停可以看到它们的名字。已安装修订版的配方不再被缓存时,没有这些计数。**Update** 在集合窗格里打开较新的修订版,在那里 *Install this collection* 会运行它。这个列表在每次检查时重建,不会被保存。

### Nexus 设置

**Preferences -> Nexus -> Offline** 会让 Eidos 完全不联系 Nexus,并跳过 GitHub 上每天一次的发行版检查。更新检查、登录、下载和集合会说明
这一点,而不是以连接错误失败。它默认关闭。

**Preferred servers** 为下载偏好的 CDN 节点排名,最优的在前。只有高级账户才会得到多个可选的镜像,所以对其他所有人,是 Nexus 来选,这个设置
不起作用。当你指定的服务器都没有被提供时,下载仍会进行,使用 Nexus 提供的第一个节点。

### 类别

类别可以分配给一个模组或整个选择,目录可以在同一个对话框里编辑,游戏的官方类别列表可以从 Nexus 拉取。两个目录文件都是 MO2 自己的
(`categories.dat` 和 `nexuscatmap.dat`),所以共享的实例保持一个目录。从 Nexus 下载的模组会在安装时归类,前提是 Fetch from Nexus 已经映射了
目录。

### INI 编辑器、日志和扩展

**View -> INI editor** 编辑配置档的游戏 INI,即 Eidos 保留的那份副本;Proton 前缀里的副本在每次启动时都会被覆盖。它的最后一个标签页是配置档的
`launch-env.ini`([每个配置档的变量与包装程序](#每个配置档的变量与包装程序))。**View -> Log** 读取会话日志。**View -> Extensions** 列出你自己的
扩展;见 [extensions.md](extensions.md)。

当一次启动发现游戏的主 INI(`Skyrim.ini`、`Fallout4.ini`、`Oblivion.ini`...)既不在配置档里也不在前缀里时,Eidos 会先把游戏文件夹的默认文件
(`Skyrim_Default.ini`、`Fallout4_Default.ini`...)复制进配置档,和 MO2 一样。实例运行的是游戏副本时,默认文件取自副本。前缀和游戏文件夹不会
被改动。

### 安装模组

安装器处理 Simple 档案、FOMOD、Wrye Bash 的 BAIN 包(勾选子包,它们按顺序合并)以及 Oblivion 的 OMOD 包。OMOD 脚本在它使用 Eidos 支持的
OBMM 命令,或者是 Eidos 认识的三个 C# 安装器之一时运行;Eidos 不运行任何其他 C# 或 Visual Basic 代码,并会说明这一点。当没有任何启发式识别
出布局时,手动选择器会显示档案的树,让你指出数据根目录。

重新安装或更新一个 FOMOD 模组时,向导会在自上次以来选项发生变化的第一步,勾选上次安装的回答打开,并列出这些变化。**Reinstall with new choices**
则改为从安装器的默认值开始。`eidos install --replace` 或 `--merge` 会重用已存储的回答,当它们不再适用时会拒绝并列出这些变化。

### Diagnostics

Diagnostics 标签页运行实时健康检查。它报告缺失的 master、被 LOOT 标记为 dirty 的插件、没有活动插件会加载的档案、模组列表是否仍与模组文件夹
一致、FUSE passthrough 是否开启,以及在请求它时它所需的启动权限是否存在。运行之后,它显示 script extender 自己的日志对它每个插件 DLL 的说法,
所以你能看到你的 SKSE 插件是否加载。它还报告自上次正常启动以来的游戏更新、缺失的 script extender 或与游戏版本不匹配的(附 extender 的下载页和
安装你所下载档案的按钮)、不用 `%command%` 启动 Eidos 的 Steam 启动选项、为 DLSS 而关闭的 NVAPI、阻止帧生成的独占全屏、没有 `LinuxVersion=true`
的 ENB `enblocal.ini`、F4SE 和 SFSE 插件的插件声明,以及上一次会话的崩溃日志及其点名的模组。

配置档 INI 和插件的检查会查找 Fallout 4、Fallout 4 VR 和 Starfield 自定义 INI 的 `[General]` 里的 `sTestFile` 行,仅限这三款游戏,它们会让游戏忽略
`plugins.txt`(Fix 会把它们从配置档的副本里移除,并把原文件保存为 `<ini>.pre-fix`);还检查 Starfield 里已启用的 `.esp` 插件,以及缺少
blueprint 标志、`BlueprintShips-` 前缀或 `.esm` 扩展名,或主插件未启用的 blueprint 插件。系统检查会标出低于 Proton 所需的
`vm.max_map_count` 或打开文件数硬限制,以及位于 NTFS、exFAT 或 FAT 上的游戏、前缀或实例。模组检查会点出没有指向游戏 `Data` 的 BodySlide 数据路径、工具
写进 `Overwrite/Root` 的 `Data` 文件夹、散装文件输给它所修补的模组的补丁,以及替换了 script extender 的 Papyrus 脚本的模组。

Check for updates 还会读取每个模组的 Nexus 前置要求。Diagnostics 列出缺失或被禁用的所需模组,对于以其他方式满足的前置要求有 Ignore。

这个标签页还显示哪个 Proton 版本运行游戏,并在较旧的 Proton 运行较新 Proton 设置的前缀之前发出警告;Rebuild 会把损坏的前缀移到一边,让 Steam 创建新的
([troubleshooting.md](troubleshooting.md#游戏由哪个-proton-运行以及损坏的前缀))。其他卡片标出从未设置或注册表陈旧的前缀、由 Eidos 之外的程序接收
Mod Manager Download 链接(Register 可以修复)、比它的输入更旧的工具输出(带一个再次运行该工具的按钮),以及原插件此后已改变的清理过或 light 的副本。每张卡片都在
[troubleshooting.md](troubleshooting.md#health-卡片) 里说明。**Copy report** 把系统和每个结论的报告放到剪贴板上,你的主目录和登录名会被隐去,用于缺陷报告。

对于 Skyrim SE、Enderal SE 和 Fallout 4,Diagnostics 还有一张 Display 卡片。它显示配置档的 `SkyrimPrefs.ini`(`EnderalPrefs.ini`、`Fallout4Prefs.ini`)里设定的窗口大小和模式,
配置档还没有时则取前缀里的副本。两者都没设置大小时,它的字段从 `/sys/class/drm` 里列出的最大已连接屏幕开始,所以 Steam Deck 读到的是 1280x800。宽和高是 320 到 16384 之间的整数。
Apply 把 `iSize W`、`iSize H`、`bFull Screen` 和 `bBorderless` 写进配置档的副本,如果配置档没有,会先接管前缀的副本。当由 SSE Display Tweaks 设定窗口时,Apply 把同样的大小和模式写进它在
Overwrite 里的副本;模组自己的文件不会改变。

标签页顶部的 Game files 卡片在[游戏文件](#游戏文件商店安装或副本)里说明,集合顺序卡片在[保持集合顺序](#保持集合顺序)里,Find the mod behind a problem 在[测试运行](#找出问题背后的模组)里。

### 启动游戏

要通过窗口启动游戏,把游戏的 Steam 启动选项设为程序的绝对路径,因为 Steam 不读取你 shell 的 `PATH`
([install.md](install.md#然后让-steam-指向它)):

```
~/.local/bin/eidos-gui %command%
```

从 Colony 安装时,路径是 `~/.local/share/Colony/apps/Eidos/eidos-linux`。Eidos 会在游戏的实例上打开,即你上次使用的那个,所以便携实例也像全局实例一样能被再次找到;点击 Run 就会穿过合并视图启动它。

打开 **Start the game at once when a store opens Eidos**(Preferences, General, Running a game;默认关闭)后,Eidos 会跳过那次点击:它检查 Health,并在不显示窗口的情况下启动游戏。当 Health 列出问题、模组列表需要处理、设为在游玩前运行的工具的输出已过期,或者对于 Skyrim SE、Enderal SE 和 Fallout 4,配置档还没有窗口大小时(窗口随后会在 Diagnostics 上打开),窗口会照常打开。要为某一次启动得到窗口,请在命令前放 `EIDOS_WINDOW=1`:`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`。

在没有商店打开过的窗口里,Run 不会自己启动游戏,Eidos 也从不直接通过 Proton 或 Wine 启动游戏。Run 请拥有该副本的商店来启动它,然后由商店启动 Eidos:

- 设置了上面的启动选项、并且游戏有 Proton 前缀(Steam 在第一次启动游戏时创建)时,Steam 运行 `steam -applaunch <appid>`。Steam 关闭时会先启动。
- 当 Eidos 在 Heroic 里是游戏设置中的包装程序,以 `eidos-gui`(或 `eidos-linux`)的绝对路径作为可执行文件且不带参数时,Heroic(GOG 或 Epic 副本,原生或 Flatpak)会得到一个 `heroic://launch` 链接。Heroic 关闭时会先启动。
- 单独的 Legendary 会运行 `legendary launch <app>`,前提是 `~/.config/legendary/config.ini` 在游戏的 `[<app>]` 段下,或在 `[default]` 下,有 `wrapper =` 和 `eidos-gui`(或 `eidos-linux`)的绝对路径。

Run 列表里的 script extender loader、启动器和游戏的可执行文件都这样交给商店,在终端里对其中之一执行 `eidos tool <instance> run <entry>` 也一样。Eidos 无法把某些副本交给它们的商店,例如 Heroic 之外的 GOG 副本:这时 Run 会说明要从那个商店启动游戏。

这次交接会把本实例记录为你上次使用的实例,然后商店在它上面打开第二个 Eidos 窗口。Start the game at once 关闭时,你在那里点击 Run;打开时,那个窗口会在不显示自己的情况下启动游戏,除非上面的某种情况让它打开。第一个窗口不会跟随这次运行,所以游玩之后请在它里面按 F5 重新载入它的列表。商店的设置缺失,或 script extender 的 loader 不在游戏可执行文件旁边时,Run 什么也不启动,并显示该设置什么、在哪里设置,附上正在运行的二进制文件的真实路径。

游戏或工具运行时,Run 会变成 Stop。按两次会结束这次运行启动的所有程序,然后 Eidos 像任何退出之后那样捕获会话并卸载;被停止的运行绝不会算作一次正常的启动([troubleshooting.md](troubleshooting.md#无法关闭的游戏或工具))。

Bethesda 游戏的 Steam `%command%` 通常指向 `<Game>Launcher.exe`。Eidos 从不运行它:启动器是单独的设置应用,会重新扫描 `Data` 并重写 `plugins.txt`,把刚刚部署好的加载顺序一笔勾销。Eidos 改为运行 script extender 的 loader(如果装了),否则运行游戏本体,并在不得不退回到游戏本体时说明。

Eidos 会扫描已启用模组的 DLL 里有没有对 `d3dcompiler_47.dll` 的导入,把 Microsoft 的 DLL 复制进前缀,然后才设置 `WINEDLLOVERRIDES=d3dcompiler_47=n,b`;你不需要自己设置它。

### 每个配置档的变量与包装程序

配置档可以在 `profiles/<name>/launch-env.ini` 里带上自己的环境变量和包装命令,每行一项:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

它们适用于该配置档的每一次 `eidos play`(由 Steam 的 `%command%` 启动的游戏,或商店打开的窗口里的 Run),以及每一次工具运行。`eidos shell` 得到变量但没有包装程序。第一个 `wrapper=` 行在最外层。含有 `%command%` 的 `wrapper=` 行会把命令放在那个位置,和 Steam 的启动选项一样;不含它的行则把命令放在末尾。包装程序在 Eidos 的私有命名空间里运行,所以它们能看到模组。

这里的变量会替换 Eidos 为同名变量设置的值,而工具在 `tools.ini` 里自己的 `env=` 行会替换两者。`WINEDLLOVERRIDES` 则是加在 Eidos 为 ENB、ReShade 和自带 DLL 设置的覆盖之后,所以两处都点名的 DLL 采用配置档的设置,其余的保持自己的。

View -> INI editor 把这个文件作为最后一个标签页打开,对每款游戏都是如此。空行和以 `#` 或 `;` 开头的行会被跳过。保存时会列出 Eidos 忽略的行号:既不是 `env=NAME=value` 也不是 `wrapper=command` 的行,以及首个单词是 `NAME=value` 的 `wrapper=` 行(Steam 的启动选项允许这样写);请把那个变量改放到 `env=` 行上。不带命令的 `eidos play <instance>` 会列出一次运行得到的变量和包装程序。Diagnostics 里的 Copy report 会点出变量和包装程序的名字,不带它们的值。

## 工具

xEdit、BodySlide、DynDOLOD 和其他工具穿过合并视图,在游戏的 Wine 前缀里运行(Steam 的 Proton 前缀,或 Heroic 为 GOG 或 Epic 副本建好的前缀):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # 不登记就把 exe 运行一次
eidos prereqs skyrimse            # 已登记的工具需要什么,以及它的状态
eidos prereqs skyrimse --install  # 取回缺失的任何东西
```

Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、xLODGen 和 Synthesis 不用登记就能被找到。Pandora、Nemesis、FNIS、BodySlide 和 Synthesis 出现时已设好捕获模组,所以它们的输出会进入各自的模组(细节见 [tools.md](tools.md#把工具的输出送进它自己的模组))。

你给工具起的标题,决定了 Eidos 为它提供哪些运行时和 DLL:`BodySlide` 得到它的 DirectX 库,`BS` 什么也得不到。在窗口里,Executables 对话框在字段下面显示每个前置组件的真实状态,缺失的那些是按钮。

发行版捆绑包在 `runtimes/` 里带有 vcrun2022 安装程序(x86 和 x64)、.NET 8 Desktop Runtime 8.0.12 和 .NET 10 运行时 10.0.10,`install.sh` 会把它们复制到 `~/.local/share/Colony/Eidos/runtimes/`,所以 `eidos prereqs <instance> --install` 不用下载就能安装它们。Eidos 不经 winetricks,在游戏的前缀里静默运行 Microsoft 的 vcrun2022 和 .NET 8 安装程序。`eidos-linux` 和源码构建没有 `runtimes/` 文件夹,所以它们从 Microsoft 下载同样的固定文件。Synthesis 需要的 .NET 10 SDK(约 300 MB)总是需要下载。

对照表、三类前置组件、为什么 DynDOLOD 需要 winetricks 装不了的 .NET 运行时,以及为什么作为模组安装的工具会从合并后的路径而不是它自己的文件夹启动,都在 [tools.md](tools.md) 里。

## 扩展

Eidos 不必重新构建就能扩展:`~/.config/Colony/Eidos/addons/` 里的一份 TOML 清单会向 Extensions 列表添加一个工具,或向 Diagnostics 标签页添加一项检查。没有任何东西被加载进 Eidos:扩展是它运行的一个程序。见 [extensions.md](extensions.md)。

## 试试概念验证

这个脚本不需要游戏。它展示合并视图、写时复制、一个原封不动的游戏文件夹,以及一个只存在于它自己命名空间里的合并,只使用用户命名空间里无特权的 OverlayFS(Linux 5.11 或更新):

```sh
./scripts/poc-overlay.sh
```
