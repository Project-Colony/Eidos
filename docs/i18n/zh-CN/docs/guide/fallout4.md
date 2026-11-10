<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# 通过 Eidos 玩 Fallout 4

用 Eidos 玩 Fallout 4,不需要特殊的启动选项,不需要改名的可执行文件,也不需要包装脚本。
它和其他所有游戏用同一个启动选项。

## 启动选项

```
~/.local/bin/eidos-gui %command%
```

Steam 为 Fallout 4 设定的启动目标是 `Fallout4Launcher.exe`,从来不是 `Fallout4.exe`,
所以让 script extender 跑起来,本质上是「怎么让 Steam 启动另一个程序」这个问题。常见
答案是用 bash 改写 `%command%`:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

或者把 `f4se_loader.exe` 覆盖到 `Fallout4Launcher.exe` 上。Steam 会在每次游戏更新时还原
它,此后你就在没有 F4SE 的情况下游玩,却没有任何提示。

Eidos 根据游戏描述符自己完成这次替换。窗口启动游戏时,如果加载器在游戏文件夹、某个
已启用模组的 `Root/` 或 `Overwrite/Root/` 里,就用 `f4se_loader.exe` 替换
`Fallout4Launcher.exe`。找不到加载器时运行 `Fallout4.exe`,状态栏写:
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
如果这两个文件都找不到,则原样运行启动器,同样会说明。若启动选项改用
`eidos play fallout4 -- %command%`,命令行只在加载器存在时才替换,否则运行
`Fallout4Launcher.exe`。

避开启动器还有第二个理由:它会重新扫描 `Data` 并重写 `plugins.txt`,把刚刚部署好的加载
顺序一笔勾销。

你自己打开窗口并按 Run 时,Eidos 会请 Steam 启动游戏(`steam -applaunch 377160`),
Steam 再通过上面的启动选项启动 Eidos。如果实例里游戏可执行文件旁边没有
`f4se_loader.exe`,Eidos 不会这么做,状态栏会要求你先安装 script extender。
Fallout 4 VR 的做法相同,对应 `Fallout4VRLauncher.exe` 和 `f4sevr_loader.exe`。

## Eidos 替你处理的事

| 范围 | Eidos 做什么 |
|---|---|
| 档案失效 | `Fallout4Custom.ini` 会被写入 `[Archive]` `bInvalidateOlderFiles=1` 和空的 `sResourceDataDirsFinal=`,这两个键让 `Data` 之外的散装文件根本能被看见。写进配置档,而不是游戏目录。 |
| INI | `Fallout4.ini`、`Fallout4Prefs.ini` 和 `Fallout4Custom.ini` 属于配置档。通过 Eidos 运行的 BethINI Pie 编辑的是配置档里的副本,Backups -> INIs 会放回 Eidos 在每次运行前保留的那份副本([tools.md](tools.md#bethini-pie))。 |
| 加载顺序 | `plugins.txt` 采用 Fallout 4 使用的星号格式(`*` 表示启用),并遵循 `Fallout4.ccc` 处理隐含的 Creation Club 插件 |
| LOOT | 排序方式与 Skyrim 相同,`eidos sort <instance>` 会取 `fallout4` 的 masterlist |
| 存档 | `.fos` 存档及其 `.f4se` 协同存档会被列出、复制并按配置档保存;详情面板会读取存档自身的插件表,所以一个需要你已禁用插件的存档会在读取前就说出来。禁用或移除某个模组时,如果配置档最新三个存档之一用到了它的插件,会先询问。 |
| Root 模组 | 模组随可执行文件一同提供的东西(F4SE 本身、ENB、`dxvk.conf`)都通过 Skyrim 同样的 `Root/` 机制落到那里 |

## 版本问题

Fallout 4 不再是 2019 到 2024 年间那个冻结的游戏。截至 2026 年 8 月,有三条活跃分支,
为其中一条构建的模组 DLL 无法在另一条上加载:

| 分支 | 版本 | F4SE |
|---|---|---|
| 经典版(old-gen) | 1.10.163 | 0.6.23 |
| 次世代版 | 1.10.984 | 0.7.2 |
| 周年版 / Creations | 1.11.137 to 1.11.240 | 0.7.4 to 0.7.9 |

搭建模组列表之前,有两点要知道:

- 游戏根目录里有 `Creations/` 和 `Mods/` 文件夹,说明你在 1.11.x 这条线上。Eidos 里
  存档的详情面板也会显示写下它的构建版本:Fallout 把它写进存档,Eidos 显示为
  「Game build」。
- F4SE 通常在 Bethesda 更新后一两天内跟上,但 *Address Library for F4SE Plugins*
  (大多数 DLL 模组靠它解析偏移)按自己的节奏走。在这两者之间,DLL 模组无法加载。
  不含 DLL 的模组(贴图、模型、插件)不受影响,所以刚打完补丁不是开始搭模组列表的好日子。

整套配置跑通之后,把游戏版本冻结进实例:在 Diagnostics 标签页的 Game files 卡片上点
Freeze this version,或者用 `eidos stock-game fallout4 freeze` 查看大小,用
`eidos stock-game fallout4 freeze --yes` 执行复制。Eidos 把商店安装复制到
`<instance>/Stock Game`,并从这份副本运行游戏,仍由 Steam 启动。之后的补丁改的是商店
安装而不是副本,所以你装的 DLL 仍对应它们的游戏版本。在 btrfs 或 XFS 上,副本与商店
共享数据块,在 Steam 改动某个文件之前几乎不占空间;在 ext4 上是完整复制,大小与游戏
文件夹相同。Steam 报告游戏正忙或没有完整安装时,Eidos 会拒绝。细节见
[usage.md](usage.md#游戏文件商店安装或副本)。

没有副本时,请在 Steam 里关闭 Fallout 4 的自动更新(属性,更新,「仅在启动游戏时更新」),
否则下一个补丁会把你装的每一个 DLL 都打碎。

## 硬件提示:NVIDIA 上的武器碎片崩溃

Fallout 4 的武器碎片效果基于 NVIDIA FleX,那是 NVIDIA 在 Pascal 之后停止支持的 PhysX
衍生物。在任何 Turing 及更新的显卡上(GTX 16、RTX 20 直到 RTX 50)它都会让游戏
崩溃。这是游戏本身的缺陷,与 Linux、Proton 或 Eidos 无关。

两个办法,任选其一:在游戏设置里关闭「Weapon Debris」,或安装
*Weapon Debris Crash Fix*(Nexus 48078),它禁用的是碎片的碰撞而不是效果本身。

## Fallout 4 的健康卡片

Diagnostics 标签页为下列每一项显示一张 Health 卡片:

- F4SE 插件装在 `F4SE/Plugins` 里,但 `f4se_loader.exe` 不在任何已启用的模组里,也不在
  游戏文件夹里。游戏启动时不会带上它们。
- 已安装 `f4se_loader.exe`,但没有对应当前游戏版本的 `f4se_<version>.dll`
  (1.10.984 对应 `f4se_1_10_984.dll`)。加载器会报错停止;这通常发生在游戏更新之后。
- 游戏可执行文件或它的 Steam 构建号,自上次经 Eidos 启动且无错误退出之后发生了变化。
  为旧版本构建的 F4SE 插件可能不再加载。
- 优先级比 F4SE 高的某个模组带有 F4SE 自身脚本的散装 `.pex` 副本。旧副本会破坏它所声明
  的 F4SE 函数,而游戏不会报错。
- 配置档的 `Fallout4Custom.ini` 在 `[General]` 里有 `sTestFile1` 到 `sTestFile10` 行。
  只要设了其中一行,游戏就会加载这些插件并忽略 `plugins.txt`。Fix 会移除它们,并把
  原文件保存为 `Fallout4Custom.ini.pre-fix`。
- 某个模组的 BA2 档案以压缩形式存放 `.wav` 或 `.xwm` 音频。游戏播放该声音时可能崩溃。
  请用未压缩音频重新打包档案。Fallout 4 VR 也有同样的卡片。
- 崩溃记录器在上次启动时写了日志:Buffout 4 在 `Documents/My Games/Fallout4/F4SE`
  里的 `crash-*.log`,或 `F4SE/Crashlogs` 里的 `.log`。卡片会点出异常,以及日志中由你的
  模组安装的 DLL 和插件。
- Display 卡片显示配置档 `Fallout4Prefs.ini` 里设定的窗口大小和模式,Apply 会把
  `iSize W`、`iSize H`、`bFull Screen` 和 `bBorderless` 写进去。
- 某条 Nexus 前置要求指向一个 DLC。Eidos 会找与 DLC 同名的插件,而 Fallout 4 的 DLC
  插件名字不同(Far Harbor 是 `DLCCoast.esm`),所以这张卡片只是建议:自己检查,然后
  按 Ignore。

两张加载器卡片带有指向 <https://f4se.silverlock.org/> 的 Open the download page 按钮,
以及 Install the downloaded archive 按钮,后者会把加载器放进模组的 `Root/`。
`eidos check fallout4` 打印同样的结论,但 `.pex` 卡片、Display 卡片和 Nexus 前置要求
卡片除外,它们只有窗口会计算。

## 如果哪里不对劲

通用排查清单在 [troubleshooting.md](troubleshooting.md);而 Fallout 特有的第一个问题
永远是*究竟启动了哪个可执行文件*。窗口启动游戏时,会把完整的启动命令写进实例的运行
日志,所以:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

如果它写的是 `f4se_loader.exe`,替换成功了。如果写的是 `Fallout4.exe`,说明 F4SE 没装在
Eidos 找得到的地方:它该待在游戏可执行文件旁边,对于用模组管理器的配置来说,那就是
某个模组的 `Root/` 目录(或者手动装进游戏目录本身)。如果写的是
`Fallout4Launcher.exe`,说明 Eidos 在根视图里既没找到加载器也没找到 `Fallout4.exe`。

可执行文件对了但游戏仍然崩溃时,Test run 和 Find the mod behind a problem 可以在不动
配置档存档的前提下把问题缩小到某一个模组([usage.md](usage.md#测试运行))。
