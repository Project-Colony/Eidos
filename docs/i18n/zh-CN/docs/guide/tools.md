<!-- eidos-i18n: source=docs/guide/tools.md sha=bc1c18f7029d1b12bcae46d861626848edfafb5a -->

# 工具:xEdit、BodySlide、DynDOLOD, PGPatcher、FNIS

通过 Eidos 运行的工具看到的是**合并视图**,就在游戏自己的 Proton 前缀里面。
它读到的正是游戏会读到的东西 - 每一个启用的模组,按优先级排列 - 而它写出的
任何东西都落进 Overwrite,在那里一次点击就把它变成一个真正的模组。

## Eidos 自己能找到的那些

有些工具的名字足够独特,不必声明就能被找到,xEdit 就是最明显的例子:
Fallout 4 的 `FO4Edit.exe`、Skyrim SE 的 `SSEEdit.exe`、初代的
`TES5Edit.exe`,等等 - 连同它们各自的 **QuickAutoClean** 双胞胎,那正是
LOOT 一直在警告的 dirty edits 所对应的按钮。Eidos 按文件名在这些地方找它们:

- 游戏的安装目录,以及已启用模组的 `Root/` 树;
- **本实例的 `mods/`**,MO2 用户就是把工具装在那里;
- 你在偏好设置里指定的**工具文件夹**(General -> Tools),用于实例之间共享的
  那个目录 - 诸如 `/mnt/Games/Tools`。

`PGPatcher.exe` 在 Creation Engine 系列游戏中以同样的方式被找到，并且带着一个缺了就无法工作的参数 - 见[下文](#为什么-pgpatcher-需要---ignore-mo2vfscheck)。

这份列表按游戏区分,所以 Skyrim 实例永远不会被塞上 Fallout 的编辑器。搜索到
第四层就停,因为一个模组池有几十万个文件,而这在每次构建工具列表时都要跑一遍;
它也不跟随符号链接。以这种方式找到的工具,配置方式与你手动输入的完全一样:
它的运行时来自它的名字,规则和下面讲的一样。

如果工具在别的地方,或者你想要不同的参数,就手动添加 - 同名的用户条目会覆盖
任何自动找到的条目。

## 添加一个

在 GUI 里:**Tools -> Executables**,然后点 Add。在命令行里:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # 列出已注册的东西
eidos tool skyrimse run BodySlide         # 穿过合并视图运行它
eidos tool skyrimse run BodySlide --print # 只显示命令,不运行
```

script extender、游戏本体和启动器会被自动检测;只有额外的工具需要注册。

### 让它指向真正的那个文件,不管它在哪

在可执行文件实际所在的位置注册它。如果这个工具是作为模组安装的,那就在模组
文件夹里面:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(那是全局实例的路径 - 便携实例适用同样的规则,只是在它自己的文件夹下,
`<instance>/mods/...`;注意像这样的绝对路径,是之后移动便携文件夹时唯一撑不住
的东西)。

Eidos 会在启动前把那个路径改写成合并视图里的路径,于是工具从
`<game>/Data/CalienteTools/BodySlide/` 运行,并且在那里也能看到其他所有模组的
文件。这件事比听上去更要紧:BodySlide 自带的 `SliderSets` 目录是**空的**,
它能构建的每一具身体都来自 CBBE 和服装模组。从它自己的模组文件夹启动时它什么
都找不到,看起来就像坏了。

MO2 做的是同样的改写,理由也一样 - 它自己的注释里点名的是 FNIS。

位于**已禁用**模组里的工具无法被改写,因为它的文件也不在视图里。Eidos 会照实
说明,并从它自己的文件夹运行它,而不是假装。

## 把工具的输出送进它自己的模组

生成器 - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - 会写出几百个文件。
默认情况下它们和别的东西一起落进 Overwrite。在 Executables 编辑器里设置
**Capture output into**,这一次运行的输出就改为进入那个模组:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

模组不存在就会被创建。只有这次运行产生的文件会被移走;原本已经在 Overwrite
里的东西留在原处,所以两个都设了捕获目标的工具不会偷走彼此的输出。什么都没写
的一次运行不会留下一个空模组。

这是在运行结束之后做的,而不是把写入层指向那个模组 - MO2 是后面这种做法。
把写入层指向一个模组,会在整次运行期间把它提升到最高优先级 - 让它牵涉的每一处
冲突翻转,事后再翻转回来 - 并且会不经 copy-up 直接写穿模组自己的文件。捕获
不需要这两样,就能达到同样的最终状态。

如果目标模组是禁用的,输出照样会写出去,但游戏看不到它,于是工具在下一次运行时
会重新生成同样的文件。遇到这种情况 Eidos 会警告。

### 输出过期时

Eidos 会记录工具的输出是由哪些模组、插件顺序、配置档和可执行文件生成的。其中任何一项改变,Diagnostics 标签页都会提示,卡片上还有一个 **Run <tool> again** 按钮,按当前设置重新运行该工具。

对于输出过期就会弄坏游戏的生成器(例如 Pandora 或 Nemesis),可以在 `tools.ini` 里它的条目下加上 `check_before_play=1`,或在 Executables 对话框中为它打开 **Ask before playing when its output is out of date**。之后 Run 会先询问:运行该工具,还是照常游玩。选择照常游玩后,会记住到模组、插件或配置档再次改变,或窗口关闭为止。Eidos 从不自己启动该工具。

## 一个工具需要哪些 DLL,由它的名字决定

这是令人意外的部分,所以值得明说:**你给一个工具起的标题,决定了 Eidos 为它
准备哪些运行时前置。**匹配是对标题做不区分大小写的子串匹配。

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

所以注册为 **`BodySlide`** 的工具拿得到它的 DirectX DLL;同一个可执行文件
注册为 **`BS`** 就什么也拿不到,还可能带着一个只字不提 DLL 的错误启动失败。
给工具起程序本来的名字。

这份列表在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)里,而
Executables 对话框中的 `Prereqs` 字段是可以编辑的 - 检测出来的是默认值,
不是规矩。

标题同样决定**参数**，只为那一个需要谁也猜不到的参数的工具：标题中含
`pgpatcher`（或它的旧名 `parallaxgen`）就会得到 `--ignore-mo2vfscheck`。原因，
以及它为何不是可选项，见[下文](#为什么-pgpatcher-需要---ignore-mo2vfscheck)。

### 三种前置

**第一类 - 自带的 DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dcompiler_43`、
`d3dx11_43`)。Eidos
随包提供它们,并在启动时把它们复制进前缀。无需任何操作,不联网。

**第二类 - winetricks verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact`……)。它们要写注册表键、GAC 和 CLR host,所以没法靠复制文件
解决。它们**从 Microsoft 下载**。

**第三类 - 运行时**(`dotnet10`, `dotnetsdk10`)。一个现代 .NET 运行时是 193 个文件,住在
它们自己的目录里,通过 `DOTNET_ROOT` 被找到:从不注册,也根本不会装进前缀,
所以另外两类都载不动它。Eidos 自己下载它,用编译进二进制里的校验和核对,
并缓存在 `~/.local/share/Colony/Eidos/runtimes/` - **在任何实例之外**,
因为 78 MB 不是按游戏算的,也不是按 profile 算的。

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

第二类和第三类里没有任何东西是悄悄跑的:

```sh
eidos prereqs skyrimse            # 显示已注册工具需要什么,以及它们的状态
eidos prereqs skyrimse --install  # 取回缺失的部分(会下载)
```

在 GUI 里同样的状态就摆在 Prereqs 字段下面,缺的那些是按钮。既不是自带、
也不是运行时、也不是已知 winetricks verb 的东西,会被报成大概率是拼写错误,
而不是作为下载项提供。

### 为什么 DynDOLOD 需要 `dotnet10`

DynDOLOD 自己不构建 object LOD:它调用 LODGen,而且自带了三个。
`LODGenx64.exe` 面向 .NET Framework 4.8,在 Proton 下这会被导到 Wine 的 Mono -
而它的 `System.Uri` 初始化器会调用一个 Mono 没有实现的方法。它在干第一行活
之前就死了,留下一份只有版本横幅、别的什么都没有的日志,以及一个只说
"failed for one or more worlds" 的 DynDOLOD 对话框。

装上真正的 .NET Framework 也修不好:Proton 把 `mscoree.dll` - 那个本该找到它
的加载器 - 换成了指向自己那套目录的符号链接,并且每次前缀更新都会重做一遍。

能用的那个构建是 `LODGenx64Win10.exe`,它面向现代 .NET,完全不碰 `mscoree`。
把 `DOTNET_ROOT` 指向一个 .NET 10 运行时,它就能跑。`dotnet10` 准备的就是
这个,而 Eidos 在启动任何声明了它的工具时都会设置这个变量。

Eidos 用系统的 `winetricks`,配上 Proton 自己的 `wine` 和游戏前缀来跑,
这样就绕开了 Steam 的 pressure-vessel 容器,以及 protontricks 与 Proton-GE
不匹配的问题。声明了某个未安装的第二类 verb 的工具照样会启动,只是带一条
点名该 verb 和修复命令的警告 - 用户可能从别处已经有了。

### 为什么 PGPatcher 需要 `--ignore-mo2vfscheck`

PGPatcher 会重写网格和插件，让你装的贴图模组——原版、视差、complex material、
PBR——在每个表面上都用对的着色器。它拒绝在模组管理器之外运行，这是对的：对一个
未经管理的游戏目录打补丁是破坏性的。

问题出在它「怎么」检查：它去找 **usvfs**，也就是 MO2 在 Windows 下注入游戏的
DLL 挂钩层。Eidos 没有，也永远不会有——合并视图是一个 FUSE 挂载，Wine 走进去时
根本不知道它存在——所以检查什么也找不到，工具在做任何事之前就立刻退出：

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

而且它点名的还是用户根本没在运行的程序。作者本人对两份 Linux 报告
（[#716][pg716] 与 [#725][pg725]，都来自 Fluorine，即 MO2 的 Linux 分支，出于
同样的原因基于 FUSE）给出的答案，就是一个跳过该检查的开关。

因此 Eidos 会替你预填。在模组池中被找到的 PGPatcher 出现时参数已经设好；在
Executables 对话框的标题里输入 `PGPatcher`，Arguments 栏也会以同样方式被填上。
你输入的内容永远优先：预填只会填你留空的栏位，和上面的前置条件规则一致。

清单在 `default_args`（`crates/eidos-instance/src/tools.rs`），紧挨着
`default_prereqs`。它只有一条，但一条就足以让它成为一个函数：开关谁也猜不到，
而它所避免的失败点的还是别的程序的名字。

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

- `[General]` 中：`gameName`（MO2 中的游戏名，因此 Fallout: New Vegas 写作 `New Vegas`）、`gamePath`（游戏安装文件夹，以 `Z:` 路径表示）和
  `selected_profile`；
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

Eidos 只在启动某个东西时写它自己的文件，所以在两次启动之间，它记录的是上一次启动的
配置文件。由 Eidos 启动的工具能在其中找到自己的配置文件。

当配置文件中没有任何已启用的模组，或输出文件夹就是某个已启用模组的文件夹时，
PGPatcher 会报错并停止，和在 MO2 下一样。

## 前缀里的游戏路径

Windows 工具靠读 `HKLM\Software\Bethesda Softworks\<game>` 的
`installed path` 找到自己的游戏,这个键由游戏自己的安装程序写入 - 而 Steam
在 Proton 下从不运行它。没有这个键,xEdit、Wrye Bash 和 DynDOLOD 打开时路径
是空的。Eidos 在运行工具之前写好它:幂等、只增不改,并且在前缀未初始化或
正在使用时跳过。

## 够到一个工具:隐藏、置顶,以及桌面快捷方式

一款游戏的默认项里包含你可能永远用不上的工具,而一个列着八个条目、只为够到
第二个的选择器,是没人会看的选择器。在 Executables 对话框里:

- **Pin to top** 把一个条目放到 Run 列表的最前面。
- **Hide from picker** 把一个条目拿出去而不删掉它。
- **Desktop shortcut** 会往 `~/.local/share/applications` 里写一个
  `.desktop` - 在 freedesktop 系统上启动器本该待的地方,所以它出现在你的
  应用菜单和搜索里,而不是出现在桌面上。它直接运行
  `eidos tool <instance> run <title>`,这意味着这个工具是**穿过合并视图、
  带着这个实例的 profile** 起来的,而且完全不需要打开 Eidos 窗口。

隐藏和置顶关乎一个工具*怎么被够到*,而不是它运行什么,所以它们对每款游戏的
默认项和你自己的条目一样适用。

## 自成一个 Steam 应用的工具

Creation Kit 是一个独立的 Steam 应用,要用它自己的 AppID;另外几个通过 Steam
发行的模组工具也一样。在条目上设置 **Steam AppID**,Eidos 就会用那个 id 而不是
游戏的 id 来启动它。

在 Windows 上这意味着换一个启动器。在这里,这是加在本来就要构建的那次运行上的
两个环境变量 - `SteamAppId` 和 `SteamGameId`,两个都要,因为 Proton 读其中
一个,而 Steam 自己的库读另一个,一个工具看到两者不一致时会以奇怪而非清晰的
方式失败。`eidos tool ... --print` 会准确显示真实运行会拿到什么。

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

Eidos 在设置完它自己的所有变量之后才应用这些行,所以这里写的值会替换掉 Eidos
自己的值。这也包括 Eidos 根据模组自带的 DLL 拼出来的 `WINEDLLOVERRIDES`:
设置了它的工具,必须把自己需要的每个 DLL 都列进去。变量名由字母、数字和 `_`
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
`eidos prereqs <instance> --install` 也把它交给 winetricks。只用 Legendary
安装的副本没有 Heroic 设置。

`slr=1` 会让工具及其之前的注册表导入,在工具的 Proton 版本所指定的 Steam Linux Runtime 容器里启动(见其 `toolmanifest.vdf` 中的 `require_tool_appid`),与 Steam 启动游戏的方式相同。
这个运行时必须装在某个 Steam 库中;没有装的话,运行会停下,并给出需要安装的 Steam 应用。
`eidos prereqs` 仍在主机上运行 winetricks,`runner=native` 的工具会忽略这个键。
容器能看到你的主目录和游戏所在的 Steam 库。
放在其他地方(例如另一块磁盘上)的工具、mod 或运行时,需要 `env=STEAM_COMPAT_MOUNTS=/那个/路径`(多个路径用 `:` 分隔)。
装在 `/usr/share/steam/compatibilitytools.d` 下的 Proton 很可能无法配合 `slr=1` 使用,因为容器里的 `/usr` 是运行时自己的。
目前还没有在容器里实际运行过工具。

`eidos tool <instance> run <title> --print` 会显示最终的命令,以及这次运行在继承的
环境之上设置的每一个变量,包括这些行。

## 工具自己的设置终归是它自己的

Eidos 把工具放到对的位置,配上对的 DLL。工具接下来拿它的配置做什么,是你和
这个工具之间的事,而且失败通常是无声的。

举个实打实的例子,因为不这样就要搭进去一小时:BodySlide 的
**Game Data Path**(Settings)必须指向游戏的 `Data` 目录,而不是它上面那层
游戏文件夹。设高了一层,批量构建会报 "All sets processed successfully",
并把 1439 个 mesh 写到游戏永远不会去找的地方。Eidos 接住了它们 - 它们落在
`Overwrite/Root/` 而不是你的安装目录里 - 但从游戏的角度看一切正常,
只不过你的身体没有被构建出来。

工具的输出属于 Overwrite。当一次运行产出了值得留下的东西,
**Overwrite -> Create mod...** 会把它变成一个普通模组,可以像其他模组一样
排序、禁用和移除。
