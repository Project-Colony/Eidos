<!-- eidos-i18n: source=docs/guide/tools.md sha=37b7a3431ec6c74ca07db1fe66c105adc8eac11e -->

# 工具:xEdit、BodySlide、DynDOLOD, PGPatcher、FNIS

通过 Eidos 运行的工具看到的是合并视图,就在游戏的 Wine 前缀里面(Steam 的 Proton 前缀,
或 Heroic 建好的前缀)。它读到的正是游戏会读到的东西(每一个启用的模组,按优先级
排列),而它写出的任何东西都落进 Overwrite,在那里一次点击就把它变成一个真正的模组。
`runner=native` 的工具不经过 Proton 运行,见
[下文](#环境变量以及不经过-proton-的工具)。

## Eidos 自己能找到的那些

有些工具的名字足够独特,不必声明就能被找到,xEdit 就是最明显的例子:
Fallout 4 的 `FO4Edit.exe`、Skyrim SE 的 `SSEEdit.exe`、初代的
`TES5Edit.exe`,等等。每个都带有一个 QuickAutoClean 条目,用来清理 LOOT 警告的
dirty edits。`PGPatcher.exe` 在除 Morrowind 之外的每款 Bethesda 游戏上以同样的方式被
找到,并且带着一个缺了就无法工作的参数(见
[下文](#为什么-pgpatcher-需要---ignore-mo2vfscheck))。Eidos 按文件名在这些地方找它们:

- `Overwrite/Root/`、实例运行所用的游戏文件夹(商店安装,或它的游戏副本),以及已启用
  模组的 `Root/` 树;
- 本实例的 `mods/`,MO2 用户就是把工具装在那里;
- 你在偏好设置里指定的工具文件夹(General -> Tools),用于实例之间共享的目录,诸如
  `/mnt/Games/Tools`。

常用的生成器也用同样的方式查找,各自只在其支持的游戏中出现:Pandora、Nemesis、FNIS、BodySlide、DynDOLOD、TexGen、xLODGen 和 Synthesis。BethINI Pie(`Bethini.exe`)也是,出现在与 PGPatcher 相同的游戏上,见 [BethINI Pie](#bethini-pie)。在 Stardew Valley 上,SMAPI(`StardewModdingAPI.exe`)同样这样被找到。它也是那款游戏的 script extender loader,所以它通过商店启动游戏,见[够到一个工具](#够到一个工具隐藏置顶以及桌面快捷方式)。

这份列表按游戏区分,所以 Skyrim 实例永远不会被塞上 Fallout 的编辑器。搜索到
第四层就停,因为一个模组池有几十万个文件,而这在每次构建工具列表时都要跑一遍;
它也不跟随符号链接。以这种方式找到的工具,配置方式与你手动输入的完全一样:
它的前置组件、它的参数(PGPatcher 的 `--ignore-mo2vfscheck`)和它的捕获模组都来自它的
标题,规则和下面讲的一样。

如果工具在别的地方,或者你想要不同的参数,就手动添加。同名的用户条目会覆盖任何自动
找到的条目。

## 清理 LOOT 标记为 dirty 的插件

LOOT 排序之后，Diagnostics 标签页会列出 LOOT 标记为 dirty（需要清理）的插件，适用于同时有 xEdit 和 LOOT 清理数据的游戏（Skyrim LE、SE 和 VR，Enderal SE，Fallout 3，New Vegas，Fallout 4 和 4 VR，Oblivion，Starfield）。如果 Eidos 找到了 xEdit（在 Executables 里登记过，或者放在实例中 Eidos 查找工具的任何位置），卡片上会有 Clean all，每个 dirty 插件的行菜单里会有 Clean with QuickAutoClean；否则卡片会提示在 Executables 里添加 xEdit。Eidos 不会下载 xEdit。它通过合并视图，用 `-qac -autoexit -autoload`，一次对一个插件运行 xEdit 的 QuickAutoClean 模式。xEdit 把清理后的插件保存在它加载的那个插件上，在合并视图里这是 Overwrite 中的一个新文件；Eidos 把它移进名为 `Cleaned plugins` 的模组，这个模组创建时处于启用状态，位于列表底部，在所有冲突中胜出。原模组里的插件和游戏 Data 文件夹里的插件永远不会被写入。要撤销一次清理，停用 `Cleaned plugins`，或者隐藏其中的一个文件。位于 Overwrite 里的插件不会被清理,因为 xEdit 会自己重写那个文件;请先把 Overwrite 变成一个模组。

xEdit 的备份、缓存和日志放在合并视图之外的 `<instance>/logs/qac-<time>/`，状态栏会报告从这份日志读到的数量。清理成功后 Eidos 会删除这个文件夹，清理失败则保留。Eidos 会把每个副本所依据的插件的大小和日期记录在 `Cleaned plugins/meta.ini` 里。 该插件变更后,Diagnostics 标签页会显示 **Out-of-date copy of <plugin>**,卡片上有一个隐藏旧副本的按钮。

## 添加一个

在 GUI 里:菜单栏的 **Tools**,或工具栏上的 **Executables**,然后点 Add。在命令行里:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # 列出已注册的东西
eidos tool skyrimse run BodySlide         # 穿过合并视图运行它
eidos tool skyrimse run BodySlide --print # 只显示命令,不运行
eidos tool skyrimse rm BodySlide          # 删除你添加的条目
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # 运行一次 exe,不注册
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` 会把该次运行的输出收进 `mods/<mod>`,不论工具自身如何设置。`rm` 只删除你添加的条目;每款游戏的默认项可以隐藏,不能删除。

用 `--exe` 运行的 exe,如果它的文件名(忽略大小写)是 Eidos 为该游戏所知的某个工具,就会得到那个工具的标题,连同下面的前置组件、参数和捕获模组。`GenerateFNISforUsers.exe` 就是 FNIS,所以上面那行 `--exe` 会要求 `dotnet48`,并把输出捕获进 `FNIS Output`。任何其他 exe 以它的文件名作标题。在 Data 标签页里,`.exe` 行有 **Add tool**(打开 Executables 对话框并填好该条目)和 **Run**(像 `--exe` 那样把文件运行一次)。两者给文件起标题的方式相同。

对于不是 Windows 工具的命令,`eidos shell skyrimse` 会在挂载了合并视图的游戏文件夹里打开你的 `$SHELL`(未设置时为 `/bin/sh`)。shell 写入的内容落进 Overwrite,退出 shell 就卸载视图。`eidos shell skyrimse -- <command>` 则只运行一条命令。在交互式 shell 里按 Ctrl+C 会停止你键入的命令,shell 仍保持打开。shell 不经过 Proton 运行;Windows 程序请走 `eidos tool`。

在窗口里,**Stop** 会结束卡住的工具运行:按两次,Eidos 会结束这次运行的程序,然后像任何退出之后那样处理它的文件。打开 **Notify the desktop when a long job ends**(Preferences, General, Running a game)后,窗口在后台时结束的工具运行会发出桌面通知。

script extender、游戏本体和启动器会被自动检测;只有额外的工具需要注册。在从游戏副本运行的实例上,它们在副本里被检测,见[游戏副本上的工具](#游戏副本上的工具)。

### 让它指向真正的那个文件,不管它在哪

在可执行文件实际所在的位置注册它。如果这个工具是作为模组安装的,那就在模组
文件夹里面:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

那是全局实例的路径。便携实例适用同样的规则,只是在它自己的文件夹下,
`<instance>/mods/...`。像这样的绝对路径,是之后移动便携文件夹时唯一撑不住的东西。

Eidos 会在启动前把那个路径改写成合并视图里的路径,于是工具从
`<game>/Data/CalienteTools/BodySlide/` 运行,并且在那里也能看到其他所有模组的
文件。BodySlide 自带的 `SliderSets` 目录是空的,它能构建的每一具身体都来自 CBBE 和
服装模组。从它自己的模组文件夹启动时它什么都找不到,看起来就像坏了。

MO2 做的是同样的改写,理由也一样:它自己的注释里点名的是 FNIS。

位于已禁用模组里的工具无法被改写,因为它的文件也不在视图里。Eidos 会照实说明,并从
它自己的文件夹运行它。

## 来自 Mod Organizer 2 列表的工具

作为 Eidos 实例打开的 Wabbajack 列表或其他 MO2 实例会保留自己的 `ModOrganizer.ini`，其中配置的可执行文件可以复制到 `tools.ini`：

```sh
eidos tool /path/to/list import-mo2           # 显示将要添加的内容
eidos tool /path/to/list import-mo2 --write   # 添加
```

在窗口中,Executables 对话框有 **Import from ModOrganizer.ini** 按钮,每个条目都有一个可取消勾选的复选框。**Add selected** 把勾选的条目放进列表,点击 Save 之前不会写入任何内容。只有不是 Eidos 写的 `ModOrganizer.ini` 才算数;Eidos 自己写的那个见 [Mod Organizer 2 模式下的 PGPatcher 和 PGLite](#mod-organizer-2-模式下的-pgpatcher-和-pglite)。

`Z:` 上的路径是 Linux 路径，保持原样。其他盘符上的路径如果包含列表文件夹的名称，就会移到列表文件夹下（`D:\Lists\MyList\tools\SSEEdit.exe` 变为 `<list>/tools/SSEEdit.exe`），可执行文件、工作目录和参数都按此处理。脚本扩展器的加载器、启动器和游戏主程序会变成纯文件名,因此和内置条目一样,从 Eidos 挂载视图的游戏文件夹中运行。MO2 的 `hide` 变成 **Hide from picker**,`steamAppID` 变成 **Steam AppID**。指向其他位置的条目(Windows 系统程序、列表之外的工具)会连同原因列为已跳过,`tools.ini` 中已有的标题则永远不会被替换。

导入的条目保留 MO2 的标题和参数,Eidos 不会根据它的标题填写任何东西:没有前置组件,没有额外参数,也没有捕获模组。请在 Executables 对话框里设置它们。来自 Windows 上制作的列表的 PGPatcher 条目通常缺少 `--ignore-mo2vfscheck`;请加上(见[下文](#为什么-pgpatcher-需要---ignore-mo2vfscheck))。

指向列表 `Stock Game` 文件夹的导入路径,只有在实例从那份副本运行时,才会在运行时被移到商店安装上;见下一节。

## 游戏副本上的工具

当实例从游戏副本运行时(列表的 `Stock Game` 或 `Game Root` 文件夹,或冻结得到的副本;见
[游戏文件](usage.md#游戏文件商店安装或副本)),工具看到的是副本里的文件,而不是商店安装里的。
合并视图仍然挂载在商店安装上:根视图把副本放在最底层,Data 视图读取副本的 `Data`。
Eidos 在副本里,而不是商店安装里,查找 script extender、启动器、游戏本体和已知的工具。

指向副本的工具路径、参数或工作目录,会在运行前被移到商店安装上。它可以写成 Linux 路径,
也可以写成 Wine 的 `Z:` 路径:`-D:"Z:\...\Stock Game\Data"` 会变成商店安装下的同一路径。
从列表的 `ModOrganizer.ini` 导入的条目需要这一点。实例从商店安装运行时不会移动任何东西,
所以这样的条目读到的是不带你模组的原始副本。

前缀注册表里游戏的 `installed path`,以及 Eidos 写的 `ModOrganizer.ini` 里的
`gamePath`,仍然指向商店安装,合并视图就在那里。副本文件夹缺失或里面没有游戏本体时,
工具运行会像游戏启动一样停止,消息里点名 `eidos stock-game <instance> store`,用来回到
商店安装。

`eidos stock-game <instance>` 显示实例运行的是哪些游戏文件,以及它可以切换到哪些副本。
`use <folder>` 和 `store` 用来切换,`freeze --yes` 把商店安装复制到实例的
`Stock Game` 文件夹并从它运行。Diagnostics 标签页里的 Game files 卡片做同样的事;见
[游戏文件](usage.md#游戏文件商店安装或副本)。

## 把工具的输出送进它自己的模组

生成器(FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis)会写出几百个文件。
默认情况下它们和别的东西一起落进 Overwrite。在 Executables 编辑器里设置
**Capture output into**,这一次运行的输出就改为进入那个模组:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora、Nemesis、FNIS、BodySlide 和 Synthesis 添加时已设好捕获 mod：`Pandora Output`、`Nemesis Output`、`FNIS Output`、`BodySlide Output` 和 `Synthesis Output`。DynDOLOD、TexGen 和 xLODGen 没有预设，因为它们写入你指定的输出文件夹。

模组不存在就会被创建。只有这次运行产生的文件会被移走;原本已经在 Overwrite
里的东西留在原处,所以两个都设了捕获目标的工具不会拿走彼此的输出。什么都没写
的一次运行不会留下一个空模组。

Eidos 在运行结束之后移动文件。MO2 则是把写入层指向那个模组,这会在整次运行期间把该模组
提升到最高优先级:它牵涉的每一处冲突都翻转,事后再翻转回来。写入还会不经 copy-up 直接
写穿模组自己的文件。捕获不需要这两样,就能达到同样的最终状态。

如果目标模组是禁用的,输出照样会写出去,但游戏看不到它,于是工具在下一次运行时
会重新生成同样的文件。遇到这种情况 Eidos 会警告。

### 输出过期时

Eidos 会记录工具的输出是由哪些模组、插件顺序和配置档生成的,以及工具可执行文件的大小和日期,和 `tools.ini` 里该条目的路径与参数。其中任何一项改变,Diagnostics 标签页都会显示 **Generated output inputs changed**,卡片上还有一个 **Run <tool> again** 按钮,按当前设置重新运行该工具。

对于输出过期就会弄坏游戏的生成器(例如 Pandora 或 Nemesis),可以在 `tools.ini` 里它的条目下加上 `check_before_play=1`,或在 Executables 对话框中为它打开 **Ask before playing when its output is out of date**。之后 Run 会先询问:运行该工具,还是照常游玩。选择照常游玩后,会记住到模组、插件或配置档再次改变,或窗口关闭为止。Eidos 从不自己启动该工具。打开 **Start the game at once when a store opens Eidos** 时,这样的询问会让窗口保持打开,而不是直接启动游戏。

## 前置组件和参数由标题决定

你给一个工具起的标题,决定了 Eidos 为它准备哪些运行时前置组件。匹配是对标题做不区分
大小写的子串匹配。

| 若标题中包含 | Eidos 请求 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 其他任何情况 | 无 |

所以注册为 `BodySlide` 的工具拿得到它的 DirectX DLL;同一个可执行文件
注册为 `BS` 就什么也拿不到,还可能带着一个只字不提 DLL 的错误启动失败。
给工具起程序本来的名字。

这份列表在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)里,而
Executables 对话框中的 `Prereqs` 字段是可以编辑的:标题只是填入默认值。

标题同样决定参数,只为那一个需要谁也猜不到的参数的工具:标题中含
`pgpatcher`(或它的旧名 `parallaxgen`)就会得到 `--ignore-mo2vfscheck`。原因,
以及它为何不是可选项,见[下文](#为什么-pgpatcher-需要---ignore-mo2vfscheck)。捕获
模组同样来自标题(见[上文](#把工具的输出送进它自己的模组))。

### 三种前置

第一类是自带的 DLL:`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43` 和 `d3dx11_43`。
Eidos 把它们放在自己的二进制文件里,并在启动时复制进前缀。在 64 位前缀(Proton 的
默认)里,64 位版本进 `system32`,32 位版本进 `syswow64`。无需任何操作,不联网。每个
版本都以 gzip 压缩存放,所以这八个文件在每个二进制文件里占 7.9 MB,而不是 17.9 MB,
而且只有正在复制的那一个会被解包(最多 4.7 MB)。

第二类是安装程序:`vcrun2022`、`dotnet6`、`dotnet7`、`dotnet8`、`dotnetdesktop6`、
`dotnetdesktop8`、`dotnet48`、`xact` 和 `xact_x64`。它们要写注册表键、GAC 和 CLR host,
所以没法靠复制文件解决。其中三个由 Eidos 直接安装,不用 winetricks:`vcrun2022` 运行
Microsoft 的 `VC_redist.x86.exe` 和 `VC_redist.x64.exe`,`dotnet8` 和 `dotnetdesktop8`
运行 .NET 8.0.12 Desktop Runtime 安装程序,它也带有 .NET 运行时。发行版捆绑包
(`eidos-linux-bundle.tar.gz`)在 `runtimes/` 文件夹里附带这些安装程序,`install.sh`
会把它们复制到运行它的用户的 `~/.local/share/Colony/Eidos/runtimes/`,用 `--system`
时也是如此。Eidos 先在那里找每个文件,再到 `eidos` 二进制文件旁边的 `runtimes/` 文件夹
里找(原地运行的解包捆绑包)。两处都没有时(单文件 `eidos-linux`、源码构建,或系统级
安装的另一个用户),Eidos 会从 Microsoft 下载它们(`download.visualstudio.microsoft.com`)
到 `~/.local/share/Colony/Eidos/runtimes/` 并保留在那里。无论哪种情况,SHA-256 与 Eidos
内置值不一致的文件都会被拒绝,文件会不带窗口地(`/install /quiet /norestart`)在游戏的
前缀里运行,用的是运行该游戏工具的那个 Proton 或 Wine 版本。和 winetricks 一样,
`vcrun2022` 会先在前缀的 DLL 覆盖设置里把 Visual C++ 的 DLL 设为 `native,builtin`。
之后,对于 Proton 版本,Eidos 会检查前缀里的 C++ 运行时是不是一半来自 Microsoft、
一半来自 Proton。如果是,这次安装就失败:消息会点出来自不同版本的 DLL 对,并说明改用
winetricks 安装 `vcrun2022`,它会自己替换 `msvcp140.dll`。只安装 64 位的 .NET 8;
32 位的 .NET 8 工具仍然需要 `winetricks dotnetdesktop8`。

其余六个 verb 走系统的 `winetricks`,它必须在 `PATH` 上。Eidos 用运行该游戏工具的
Proton 或 Wine 版本的 `wine` 来运行它,针对游戏的前缀,这样就绕开了 Steam 的
pressure-vessel 容器,以及 protontricks 与 Proton-GE 不匹配的问题。有些 winetricks verb
需要 `cabextract`,缺少时 Eidos 会警告。声明了某个未安装的第二类 verb 的工具照样会
启动,只是带一条点名该 verb 和修复命令的警告,因为你可能是用别的方式装的。

当前缀自己的 `winetricks.log` 列出某个第二类 verb 时(无论是谁运行了 winetricks 或
protontricks),或者 Eidos 把它装进了那个前缀时,它就算完成。Eidos 把自己的安装记录
在实例的 `prereqs.done` 里,与前缀的路径和 inode 绑定,所以重建过的前缀会让它的
verb 再次显示为待安装。**Rebuild prefix** 是针对 Steam 副本的 Diagnostics 操作,它把
`compatdata/<appid>` 重命名为 `<appid>.eidos-backup-<time>`,不删除任何东西;Steam 会在
下次启动游戏时创建新的前缀。

第三类是运行时,`dotnet10` 和 `dotnetsdk10`。一个现代 .NET 运行时是 193 个文件,住在
它们自己的目录里,通过 `DOTNET_ROOT` 被找到。它从不注册,也不会装进前缀,所以另外
两类都载不动它。Eidos 把它放在任何实例之外,因为 78 MB 不是按游戏算的,也不是按
配置档算的:每个运行时解包到
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`(`dotnet10-10.0.10`、
`dotnetsdk10-10.0.403`)。Eidos 在与第二类安装程序相同的两个文件夹里,按下载名称查找
运行时的 zip,然后解包发行版捆绑包附带的那份(`dotnet-runtime-10.0.10-win-x64.zip`,
仅用于 `dotnet10`),或者下载它。捆绑包不带 SDK 的 `dotnet-sdk-10.0.403-win-x64.zip`,
但手动放在那里的副本会被使用,并省下下载。任一文件的 SHA-256 与 Eidos 内置值不一致都会
被拒绝。它用 `7z`(与安装模组压缩包所用的是同一个 7-Zip)解包这个 zip。

`dotnetsdk10` 是 .NET 10 SDK,下载 301 MB,解压后 808 MB。
Synthesis 编译 patcher 需要它,而且构建和运行 patcher 时调用的是不带路径的 `dotnet`,
所以声明了 `dotnetsdk10` 的工具还会在 `WINEPATH` 上得到 SDK 文件夹,外加 `NUGET_CERT_REVOCATION_MODE=offline`、`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` 和 `DOTNET_CLI_TELEMETRY_OPTOUT=1`。
构建 patcher 会留下仍在运行的 MSBuild 节点和 VBCSCompiler 服务器,在 Wine 下它们会让前缀的 wineserver 一直存活,所以该工具还会得到 `MSBUILDDISABLENODEREUSE=1`、`DOTNET_CLI_USE_MSBUILD_SERVER=0` 和 `UseSharedCompilation=false`。
`DOTNET_ROLL_FORWARD=Major` 让为 .NET 8 构建的 patcher(Mutagen 0.54 之前)在找不到 .NET 8 运行时时,使用 SDK 自带的 .NET 10 运行时运行。
`tools.ini` 中仍写着旧默认值 `dotnet8`, `vcrun2022` 的 Synthesis 条目,会在 Eidos 读取时改为 `dotnetsdk10`, `vcrun2022`。

NuGet 会对照 Windows 根证书存储检查它还原的每个包的签名,而 Wine 前缀里没有 SDK 以 `sdk/10.0.403/trustedroots/codesignctl.pem` 和 `timestampctl.pem` 附带的代码签名根证书和时间戳根证书。
没有它们,还原 `Mutagen.Bethesda.Synthesis` 0.36.6 会以 `NU3018` 和 `NU3028` 失败。
因此,在运行声明了 `dotnetsdk10` 的工具之前,只要 SDK 已经下载,Eidos 就会把这两个文件中的 385 个证书加入游戏前缀里当前用户的根证书存储,用的是写入游戏路径的同一次 `regedit` 导入(见下文)。
如果其中某个证书丢失,Eidos 会重新导入。
此后,该前缀里的每个 Windows 程序都信任这些根证书。
有了它们,在全新的前缀和空的 NuGet 缓存下,同样的还原和构建在系统 Wine 11.21 和 GE-Proton11-9 上都能成功。

在你提出要求之前,Eidos 不会安装第二类或第三类里的任何东西:

```sh
eidos prereqs skyrimse            # 显示已注册工具需要什么,以及它们的状态
eidos prereqs skyrimse --install  # 安装缺失的部分(先用捆绑包里的副本,再下载)
```

在 GUI 里同样的状态就摆在 Prereqs 字段下面,缺的那些是按钮。既不是自带、也不是运行时、
也不是第二类 verb 的东西,会被报成大概率是拼写错误,而不是作为下载项提供。

`eidos prereqs <instance> --install` 在游戏、Steam 或某个工具仍占用前缀时会拒绝。
第三类运行时甚至在前缀存在之前就能安装。Eidos 自己运行的安装程序需要一个已存在的
前缀:在商店启动过一次游戏之前,它们会以 "start the game once through its store,
then install again" 停止。第二类 verb 逐个安装,每次成功都会被记录,所以失败之后同一条
命令会接着做。对于没有 Heroic 设置的 GOG 或 Epic 副本,`--install` 复制自带的 DLL 之后
就会停在第二类 verb 之前:它说明要使用那个运行器自己的 winetricks,并在下次检查时读取
前缀的 `winetricks.log`。

### 为什么 DynDOLOD 需要 `dotnet10`

DynDOLOD 自己不构建 object LOD:它调用 LODGen,而且自带了三个。
`LODGenx64.exe` 面向 .NET Framework 4.8,在 Proton 下这会被导到 Wine 的 Mono,
而 Mono 的 `System.Uri` 初始化器会调用一个 Mono 没有实现的方法。LODGen 在干第一行活
之前就死了,留下一份只有版本横幅、别的什么都没有的日志,以及一个只说
"failed for one or more worlds" 的 DynDOLOD 对话框。

装上真正的 .NET Framework 也修不好:Proton 把 `mscoree.dll`(那个本该找到它的加载器)
换成了指向自己那套目录的符号链接,并且每次前缀更新都会再做一遍。

能用的那个构建是 `LODGenx64Win10.exe`,它面向现代 .NET,完全不碰 `mscoree`。
把 `DOTNET_ROOT` 指向一个 .NET 10 运行时,它就能跑。`dotnet10` 准备的就是
这个,而 Eidos 在启动任何声明了它的工具时都会设置这个变量。

### 为什么 Pandora 需要 `dotnet10`

Pandora 是 Avalonia 应用,不是 WPF 应用,从 Pandora 4.1.0 起它面向 `net10.0`,所以只靠
.NET 10 运行时就能运行。找不到运行时时,它会说要安装 .NET Desktop Runtime。那是 .NET
启动器对任何找不到运行时的 GUI 程序都会显示的通用文字,`dotnetdesktop8` 修不好它。

Eidos 1.21.0 及更早版本给 Pandora 的是 `dotnetdesktop8`,而条目会保留自己的 `prereqs=`
行。Eidos 读到仍然只列出 `dotnetdesktop8` 的 Pandora 条目,并且它可执行文件旁的
`.runtimeconfig.json` 写着 `net10.0` 时,该条目会改用 `dotnet10`。Pandora 2.x 面向
.NET 8,保持原样。

### 为什么 PGPatcher 需要 `--ignore-mo2vfscheck`

PGPatcher 会重写网格和插件,让你装的贴图模组(原版、视差、complex material、PBR)
在每个表面上都用对的着色器。它拒绝在模组管理器之外运行,因为对一个未经管理的游戏
目录打补丁是破坏性的。

PGPatcher 会去找 usvfs,也就是 MO2 在 Windows 下注入游戏的 DLL 挂钩层。Eidos 没有:
合并视图是一个 FUSE 挂载,Wine 走进去时根本不知道它存在。所以检查什么也找不到,工具在
做任何事之前就立刻退出:

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

而且它点名的还是用户根本没在运行的程序。作者本人对两份 Linux 报告
（[#716][pg716] 与 [#725][pg725]，都来自 Fluorine，即 MO2 的 Linux 分支，出于
同样的原因基于 FUSE）给出的答案，就是一个跳过该检查的开关。

因此 Eidos 会替你预填。在模组池中被找到的 PGPatcher 出现时参数已经设好；在
Executables 对话框的标题里输入 `PGPatcher`，Arguments 栏也会以同样方式被填上。
你输入的内容永远优先：预填只会填你留空的栏位，和上面的前置条件规则一致。

清单在 `default_args`(`crates/eidos-instance/src/tools.rs`),紧挨着
`default_prereqs`。

运行时它什么都不需要，这对 .NET 工具来说少见：它自带运行时（`dotnetlib/`，附有
声明 `includedFrameworks` 的 `runtimeconfig.json`），并把 Microsoft CRT 的 DLL
放在可执行文件旁边。Prereqs 留空即可。

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### Mod Organizer 2 模式下的 PGPatcher 和 PGLite

Mod Manager 设为 `None` 时，PGPatcher 只能看到一个合并后的 `Data` 文件夹，无法
得知某个网格或贴图来自哪个模组，因此既没有冲突可显示，也没有模组顺序可遵循。
它的 `Mod Organizer 2` 模式直接读取实例本身：配置文件的 `modlist.txt`、按优先级
排列的已启用模组，以及每个模组文件夹里的文件。游戏内版本 PGLite 也以同样方式
读取实例。Eidos 实例本来就是 MO2 的布局（`mods/`、`profiles/<name>/modlist.txt`、
`overwrite/`），所以这个模式可以直接用于它。

在 PGPatcher 的启动器中：

1. 把 **Mod Manager** 设为 `Mod Organizer 2`。
2. 把 **MO2 Options -> Instance Location** 设为 Eidos 实例文件夹，写成 Wine
   `Z:` 盘（即 `/`）上的 Windows 路径。位于 `/mnt/Games/Eidos-Skyrim` 的便携
   实例写作 `Z:\mnt\Games\Eidos-Skyrim`；全局实例写作
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`。
3. 在工具的参数中保留 `--ignore-mo2vfscheck`。在这个模式下，PGPatcher 会查找
   usvfs，找不到就停止。

PGPatcher 只接受含有 `modorganizer.ini` 的文件夹，所以 Eidos 每次启动游戏或
工具时，都会为所启动的配置文件在实例根目录写入 `ModOrganizer.ini`。只有内容
变化时才会重写。文件包含：

- `[General]` 中：`gameName`（MO2 中的游戏名，因此 Fallout: New Vegas 写作 `New Vegas`）、`gamePath`(游戏的商店安装文件夹,以 `Z:` 路径表示,实例从游戏副本运行时也是如此)和
  `selected_profile`;
- `[Settings]` 中：以绝对 `Z:` 路径表示的 `mod_directory`、`profiles_directory`
  和 `overwrite_directory`，实例有 `downloads/` 文件夹时还有
  `download_directory`。

文件第一行是一条注释，说明它由 Eidos 写入。没有这一行的 `ModOrganizer.ini`，
无论文件名大小写如何，Eidos 都不会修改或删除：你在 Eidos 中打开的 MO2 实例保留
它自己的文件，PGPatcher 按原样读取。MO2 保存设置时会丢弃注释，所以被 MO2 覆盖
保存过的文件从此归 MO2 所有。空文件算作 Eidos 的文件。

这样的文件记录的是 MO2 最后选择的配置文件，PGPatcher 使用这个配置文件，而不是 Eidos
启动的那个。两者不同时，Eidos 会在启动时记录一条警告。请在 MO2 中选择同一个配置
文件，或者重命名或删除该文件，让 Eidos 写入自己的文件。

使用 Mod Organizer 2 模式的工具也从该文件的 `gamePath` 读取游戏。当它既不是实例运行所用
的游戏文件夹、也不是商店安装时,Diagnostics 标签页会显示 **ModOrganizer.ini points tools
at another game folder**,附上那个路径和本实例运行的游戏文件夹。请修改文件里的
`gamePath`,或删除该文件让 Eidos 写自己的。

Eidos 只在启动某个东西时写它自己的文件，所以在两次启动之间，它记录的是上一次启动的
配置文件。由 Eidos 启动的工具能在其中找到自己的配置文件。

当配置文件中没有任何已启用的模组，或输出文件夹就是某个已启用模组的文件夹时，
PGPatcher 会报错并停止，和在 MO2 下一样。

## 游戏前缀中的 Wine 自带程序

File 菜单会在游戏的前缀中运行 Wine 的四个程序:winecfg、regedit、Wine 文件管理器和卸载程序。它们和其他工具一样启动,使用运行游戏工具的 Proton 或 Wine 版本,并挂载好模组,窗口会等它们结束。**Prefix drive_c** 会在文件管理器中打开前缀的 `C:` 盘。前缀里还没有某个程序时,对应的行显示为灰色,商店第一次启动游戏后就会有。在终端里,`eidos tool <instance> run winecfg` 做同样的事,`regedit`、`explorer` 和 `uninstaller` 也一样,只要列表里没有工具使用那个标题。

你在 regedit 里对游戏的 `installed path` 所做的修改,会在下一次运行工具时被改回来;见下一节。

## 前缀里的游戏路径

Windows 工具靠读 `HKLM\Software\Bethesda Softworks\<game>` 的
`installed path` 找到自己的游戏,这个键由游戏自己的安装程序写入。Steam 在 Proton 下从不
运行那个安装程序。没有这个键,xEdit、Wrye Bash 和 DynDOLOD 打开时路径是空的。

运行工具之前,Eidos 会在游戏的前缀里用 `regedit` 导入来写这个键,除非先前某次导入
已经写入了同样的路径、而前缀仍然保持着它。这个键指向商店安装,实例从游戏副本运行
时也是如此,因为合并视图挂载在那里。对于声明了 `dotnetsdk10` 的工具,同一次导入还会
加入 NuGet 签名根证书(见[上文](#三种前置))。`runner=native` 的工具会跳过这一步。
前缀尚不存在,或者另一个程序占用着它时,工具照样运行,Eidos 会打印一条警告,点出该从
哪个商店启动一次游戏,或该关闭哪个程序。键指向的不是商店安装的文件夹时,Diagnostics
标签页会显示 **Windows tools cannot find <game>**,直到下一次运行工具再次写入它。

## 够到一个工具:隐藏、置顶,以及桌面快捷方式

一款游戏的默认项里包含你可能永远用不上的工具。在 Executables 对话框里:

- **Pin to top** 把一个条目放到 Run 列表的最前面。
- **Hide from picker** 把一个条目拿出去而不删掉它。
- **Desktop shortcut** 会往 `~/.local/share/applications` 里写一个
  `.desktop`,所以它出现在你的应用菜单和搜索里,而不是出现在桌面上。它直接运行
  `eidos tool <instance> run <title>`,所以工具是穿过合并视图、带着这个实例的
  配置档起来的,而且不需要打开 Eidos 窗口。

隐藏和置顶只改变工具怎么被够到,所以它们对每款游戏的默认项和你自己的条目一样适用。

script extender 的 loader、启动器和游戏本体没有 Desktop shortcut 按钮。它们会
启动游戏,而游戏总是通过它的商店启动,这件事商店自己的快捷方式已经在做。用
Eidos 1.22 或更早版本给其中某一项建的快捷方式仍然能用:和在终端里输入的
`eidos tool <instance> run <title>` 一样,它会请商店启动游戏,就像 Run 在不是
由商店打开的窗口里做的那样。商店的设置缺失时,这条命令什么也不启动,并打印要
设置什么。快捷方式不显示任何消息,所以要看它,就在终端里运行同一条命令。

Eidos 在以下情况下会这样把任何条目交给商店:它的可执行文件名(忽略大小写)是 script
extender loader、启动器或游戏本体的文件名,无论是默认条目、你添加的条目、从 MO2 导入的
条目,还是 `--exe` 运行。它先把本实例标记为最近使用的实例,这样商店打开的窗口显示的就是
这个实例,然后运行 `steam -applaunch <id>`,对于 Heroic 运行
`xdg-open heroic://launch?appName=<app>&runner=<runner>`,或者 `legendary launch <app>`。
加上 `--print` 时它打印那条商店命令,什么也不启动。`steam` 命令必须在 `PATH` 上:Flatpak
版 Steam 不会把它放在那里,所以启动会失败,消息说明要从 Steam 启动游戏。

当商店已经启动了进程时(`SteamGameId`、`SteamAppId`、`HEROIC_APP_NAME`、
`HEROIC_APP_RUNNER` 或 `EIDOS_STORE_STARTED` 之一已设置),该条目像任何工具一样直接穿过
合并视图运行。没有这些变量之一时,Eidos 不会自己用 Proton 启动游戏。实例没有 script
extender loader(在它所运行的游戏文件夹、已启用模组的 `Root/` 或 `Overwrite/Root/` 里)时,
什么也不会启动,消息说明要先安装 script extender。对于 Heroic 之外的 GOG 副本,或者
Eidos 无法转交的商店 id,它说明要从那个商店启动游戏。

每个配置文件都可以有自己的游戏快捷方式。右键点击配置文件并选择
**Desktop shortcut**：这个条目运行
`eidos tool <instance> run "<game>" --profile <name>`，它会像在窗口里选中该配置
文件那样把它设为当前配置文件，然后请商店启动游戏。商店打开的 Eidos 窗口显示的就
是这个配置文件。`--profile` 适用于任何 `eidos tool ... run`，会在工具启动前选中
配置文件；与 `--print` 一起使用时，它什么也不选。

## 自成一个 Steam 应用的工具

Creation Kit 是一个独立的 Steam 应用,要用它自己的 AppID;另外几个通过 Steam
发行的模组工具也一样。在条目上设置 **Steam AppID**,Eidos 就会用那个 id 而不是
游戏的 id 来启动它。

在 Windows 上这意味着换一个启动器。在这里,这是加在同一次运行上的两个环境变量,
`SteamAppId` 和 `SteamGameId`。Eidos 两个都设,因为 Proton 读其中一个,而 Steam 自己的
库读另一个,一个工具看到两者不一致时会失败,却不给出清楚的错误。`eidos tool ... --print` 会准确显示真实运行会拿到什么。

## 环境变量,以及不经过 Proton 的工具

`<instance>/tools.ini` 里有三个键在 Executables 对话框中还没有对应的输入框,
需要手动写进工具所在的节。每一行 `env=NAME=value` 设置一个只对这个工具的运行
生效的变量:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos 在设置完它自己的所有变量之后、并且在配置档 `launch-env.ini` 里的变量之后才应用
这些行,所以这里写的值会替换掉两者。这也包括 Eidos 根据模组自带的 DLL 拼出来的
`WINEDLLOVERRIDES`:设置了它的工具,必须把自己需要的每个 DLL 都列进去。配置档的包装
程序会像包在游戏命令外那样包在工具的命令外;见
[每个配置档的变量与包装程序](usage.md#每个配置档的变量与包装程序)。变量名由字母、数字和 `_`
组成,不以数字开头。名字不合规则的行会被忽略。

`runner=native` 不经过 Proton,直接运行 `exe` 本身。合并视图照常挂载,
`arg0=`、`arg1=` 这些行就是它的参数。Linux 程序,或者 Steam 不管理、Heroic
也没有记录运行器的副本上的工具,都用这种方式:`exe` 是那个副本的 Wine,Windows
程序是第一个参数,前缀由 `env=` 指定。

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` 必须是绝对路径。工作目录默认是 `exe` 所在的文件夹,在这里就是运行器的
文件夹,所以要设置 `workdir=`。只有 `exe` 会被改写成合并后的路径:模组里的
Windows 程序,要写它在游戏 `Data` 文件夹下的路径,合并视图就是在那里显示它的。
原生工具会跳过 Proton 的步骤:上面讲的注册表键、Steam 的着色器缓存、空的 .NET
根目录,以及从 `LD_PRELOAD` 中去掉 Steam 覆盖层。

用 Heroic 安装的 GOG 或 Epic 副本,如果游戏的 Heroic 设置
(`GamesConfig/<appName>.json`)指定了 Wine 或 Proton 版本,工具就不需要这样的
条目:Eidos 用那个版本在游戏的 Heroic 前缀里运行它们,
`eidos prereqs <instance> --install` 也把它用于安装程序和 winetricks。只用 Legendary
安装的副本没有 Heroic 设置。

`slr=1` 会让工具及其之前的注册表导入,在工具的 Proton 版本所指定的 Steam Linux Runtime 容器里启动(见其 `toolmanifest.vdf` 中的 `require_tool_appid`),与 Steam 启动游戏的方式相同。
这个运行时必须装在某个 Steam 库中;没有装的话,运行会停下,并给出需要安装的 Steam 应用。
`eidos prereqs` 仍在主机上运行它的安装程序和 winetricks,`runner=native` 的工具会忽略这个键。
容器能看到你的主目录和游戏所在的 Steam 库。
放在其他地方(例如另一块磁盘上)的工具、mod 或运行时,需要 `env=STEAM_COMPAT_MOUNTS=/那个/路径`(多个路径用 `:` 分隔)。
装在 `/usr/share/steam/compatibilitytools.d` 下的 Proton 很可能无法配合 `slr=1` 使用,因为容器里的 `/usr` 是运行时自己的。
目前还没有在容器里实际运行过工具。

`eidos tool <instance> run <title> --print` 会显示最终的命令,以及这次运行在继承的
环境之上设置的每一个变量,包括这些行。对于启动游戏的条目,它改为打印商店命令(见
[够到一个工具](#够到一个工具隐藏置顶以及桌面快捷方式))。

## BethINI Pie

Eidos 不附带 BethINI Pie。把从它的 Nexus 页面下载的版本放进此实例的 `mods/` 或你的工具文件夹,`Bethini.exe` 就会出现在工具列表中,出现在与 PGPatcher 相同的游戏上。通过 Eidos 运行时,它编辑的是配置档自己的游戏 INI 副本。每次运行标题含 `bethini` 的工具之前,Eidos 会保留这些 INI 的一个还原点;工具栏上的 **Backups**,然后 **INIs**,可以还原其中一个。副本生成时尚不存在的 INI 保持原样。该对话框为每种类型保留最新的 10 个还原点,而还原其中一个时会先备份当前的 INI,所以选错了也能撤销。

## 工具自己的设置终归是它自己的

Eidos 不检查大多数工具设置;设错了通常不会有任何报错。

BodySlide 的 **Game Data Path**(Settings)必须指向游戏的 `Data` 目录,而不是它上面那层
游戏文件夹。设高了一层,批量构建会报 "All sets processed successfully",并把 1439 个
mesh 写到游戏永远不会去找的地方。Eidos 在 `Overwrite/Root/` 里接住了它们,所以你的游戏
安装目录不受影响,而在游戏里唯一的迹象就是你的身体没有被构建出来。

在运行 BodySlide 或 Outfit Studio 之前,如果胜出的 `CalienteTools/BodySlide/Config.xml`
里的 `GameDataPath` 或 `OutputDataPath` 在运行期间没有通向游戏的 `Data` 文件夹,状态栏会
发出警告,Diagnostics 标签页也会把同一个警告显示为一张卡片,并给出应当设置的值。
`eidos check <instance>` 也会报告它。`S:` 路径按照 Eidos 为工具映射 `S:` 的方式来读,即
Steam 库文件夹。位于实例 `mods/` 文件夹内、用于输出模组的 `OutputDataPath` 会被接受。
当 `Overwrite/Root/` 里有 `meshes` 或 `textures` 文件夹时,Diagnostics 也会显示一张卡片。

工具的输出属于 Overwrite。当一次运行产出了值得留下的东西,
**Overwrite -> Create mod...** 会把它变成一个普通模组,可以像其他模组一样
排序、禁用和移除。
