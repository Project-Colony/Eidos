<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# 工具:xEdit、BodySlide、DynDOLOD、FNIS

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
- 你在设置里指定的**工具文件夹**(Tools -> Tools folder),用于实例之间共享的
  那个目录 - 诸如 `/mnt/Games/Tools`。

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
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
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

## 一个工具需要哪些 DLL,由它的名字决定

这是令人意外的部分,所以值得明说:**你给一个工具起的标题,决定了 Eidos 为它
准备哪些运行时前置。**匹配是对标题做不区分大小写的子串匹配。

| 若标题中包含 | Eidos 请求 |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| 其他任何情况 | 无 |

所以注册为 **`BodySlide`** 的工具拿得到它的 DirectX DLL;同一个可执行文件
注册为 **`BS`** 就什么也拿不到,还可能带着一个只字不提 DLL 的错误启动失败。
给工具起程序本来的名字。

这份列表在 `default_prereqs`(`crates/eidos-instance/src/tools.rs`)里,而
Executables 对话框中的 `Prereqs` 字段是可以编辑的 - 检测出来的是默认值,
不是规矩。

### 三种前置

**第一类 - 自带的 DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dx11_43`)。Eidos
随包提供它们,并在启动时把它们复制进前缀。无需任何操作,不联网。

**第二类 - winetricks verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact`……)。它们要写注册表键、GAC 和 CLR host,所以没法靠复制文件
解决。它们**从 Microsoft 下载**。

**第三类 - 运行时**(`dotnet10`)。一个现代 .NET 运行时是 193 个文件,住在
它们自己的目录里,通过 `DOTNET_ROOT` 被找到:从不注册,也根本不会装进前缀,
所以另外两类都载不动它。Eidos 自己下载它,用编译进二进制里的校验和核对,
并缓存在 `~/.local/share/Colony/Eidos/runtimes/` - **在任何实例之外**,
因为 78 MB 不是按游戏算的,也不是按 profile 算的。

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
