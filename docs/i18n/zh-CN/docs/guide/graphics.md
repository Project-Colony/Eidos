<!-- eidos-i18n: source=docs/guide/graphics.md sha=62294f58f6a6524b75b1424aace480fccaa6de22 -->

# Community Shaders、DLSS 与帧生成

Community Shaders 1.4+ 自带超分(DLSS 4 / FSR 3.1 / XeSS,通过单独的
"Upscaling - Community Shaders" 包)以及 FSR 3.1 帧生成。这些在 Linux 上都能穿过
Eidos 工作:CS 及其附加包按普通模组安装,合并视图像对待其他文件一样提供它们的
DLL。但有三项设置在游戏里看不出来,而且每一项都会让功能不报错地什么也不做:NVAPI
变量、独占全屏和 Reflex 的 FPS 上限。

对于有 Wine 前缀的游戏(Steam 的 Proton 前缀,或 Heroic 建好的那个),Diagnostics
标签页会检查前两项,以及下面的 ENB 那一行。Reflex 上限不检查。实例从 `Stock Game`
这类副本运行游戏时,这些检查读的是副本里的文件,而不是商店安装。`eidos check <instance>`
在终端里打印同样的卡片,只是没有 Fix 按钮。

- NVAPI 卡片出现的条件:Skyrim SE 或 Enderal SE 的 Steam 副本,已加载 NVIDIA 驱动
  (`/proc/driver/nvidia/version` 存在),合并后的 `Data` 在 `Shaders/Upscaling` 里
  有内容,游戏所用的 Proton 版本默认关闭 NVAPI,并且 Steam 的启动选项、Eidos 自己的
  环境和配置档的 `launch-env.ini` 都没有设置 `PROTON_ENABLE_NVAPI=1`。
- 独占全屏卡片只在装了 Upscaling 包时出现,条件是配置档的 `SkyrimPrefs.ini` 里有
  `bFull Screen=1`,而 `SKSE/Plugins/SSEDisplayTweaks.ini` 没有同时设置
  `Fullscreen=false` 和 `Borderless=true`。它的 Fix 按钮把 `bFull Screen=0` 和
  `bBorderless=1` 写进配置档的 `SkyrimPrefs.ini`。这项检查对 Enderal SE 也读
  `SkyrimPrefs.ini`,而 Enderal SE 的配置档里是 `EnderalPrefs.ini`,所以 Enderal SE
  请改用下面的 Display 卡片。
- ENB 卡片出现的条件:任何游戏,只要根视图里生效的 `enblocal.ini` 在 `[GLOBAL]` 里
  没有 `LinuxVersion=true`。它也有 Fix 按钮。

## DLSS 需要的启动选项

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

请使用你自己的 `eidos-gui` 或 `eidos-linux` 的绝对路径,见 [install.md](install.md)。
卡片会给出该设置的那一行:游戏当前的启动选项,前面加上 `PROTON_ENABLE_NVAPI=1`;
没有启动选项时,则用正在运行的 Eidos 的路径拼出那一行。

没有 NVIDIA 的 NVAPI 层(dxvk-nvapi),CS 无法初始化 DLSS,会退回 FSR 超分,屏幕上不会
有任何提示说明原因。某个 Proton 版本是否自己开启这一层,写在它的 `proton` 脚本里,
Eidos 读取其中的 `use_nvapi` 一行。退出式开关(`disablenvapi`)表示默认开启:
GE-Proton11-9 和 Proton 11.0 是这种形式(2026-11-09 核对)。加入式开关(`enablenvapi`)
表示默认关闭,只有这样的版本才需要这个变量。在非 NVIDIA 机器上设置它没有代价。帧生成
本身是 FSR 3.1,不需要 NVAPI;只有 DLSS 超分需要。

只想为一个配置档设置时,把 `env=PROTON_ENABLE_NVAPI=1` 写进该配置档的
`launch-env.ini`(View -> INI editor,最后一个标签页)。

## 帧生成要求无边框窗口

CS 的帧生成建立在 D3D12 呈现代理之上,并且干脆拒绝独占全屏。`SkyrimPrefs.ini` 里有
`bFull Screen=1` 时它永远不会启动:没有报错,没有提示,游戏只保持基础帧率。SSE Display
Tweaks 在引擎层面设定模式,不管 INI 怎么写:

```ini
[Render]
Fullscreen=false
Borderless=true
```

窗口看起来一模一样(无边框、原生分辨率)。变的只是引擎报告的模式,而 CS 检查的正是
这个模式。

Diagnostics 里的 Display 卡片一步设好大小和模式,适用于 Skyrim SE、Enderal SE 和
Fallout 4。它显示已连接的最大屏幕的尺寸。选中 Borderless window 后,Apply 会把
`iSize W`、`iSize H`、`bFull Screen=0` 和 `bBorderless=1` 写进配置档的
`SkyrimPrefs.ini`(Enderal SE 是 `EnderalPrefs.ini`,Fallout 4 是
`Fallout4Prefs.ini`)。窗口由 SSE Display Tweaks 设定时,Apply 还会把 `Resolution`、
`Fullscreen=false` 和 `Borderless=true` 写进它的 INI 在 Overwrite 里的副本
(`SKSE/Plugins/SSEDisplayTweaks.ini`),模组自己的文件保持原样。打开了
**Start the game at once when a store opens Eidos**(Preferences, General, Running a
game)时,如果配置档还没有窗口大小、SSE Display Tweaks 也没有设定窗口,窗口仍会打开,
这样可以在第一次启动之前设好大小。

还有两个启用条件,失败方式同样安静:

- 显示器刷新率 120 Hz 或更高,或者在 CS 的超分设置里打开 `frameGenerationForceEnable`。
  帧生成会把呈现帧率翻倍,所以 CS 拒绝在显示不出结果的显示器上启用它。
- 已安装 Upscaling 包(它的 `Data/Shaders/Upscaling/` 目录里放着 Streamline 与
  FidelityFX 的 DLL)。没有它,CS 会显示菜单项却什么也开不了。

## Reflex 的帧率上限可能把输出掐死

CS 的 Reflex 设置自带 FPS 上限(`reflexFPSLimit`,配合 `reflexUseFPSLimit`)。停留
在旧值的上限(我们的是 79,来自很久以前的一次调校)位于帧生成的下游,恰好把它产出的帧
砍掉:基础 60 翻倍到 120,再被压回 79,看上去就是「帧生成没起作用」。144 Hz 显示器上
常规的 Reflex 上限约为 138。只要觉得生成的帧不见了,就查一下它。

## 已知交互:与 SSE Display Tweaks 一起出现黑屏

FG + Display Tweaks + DXVK 这个组合有已知的黑屏故障。按顺序修:

1. `SSEDisplayTweaks.ini`:`DisableBufferResizing=true`
2. 若仍不行,在游戏可执行文件旁放一个 `dxvk.conf`(模组的 `Root/` 目录就能放到那里),
   内容为 `dxvk.enableGraphicsPipelineLibrary = False`

## 混用贴图模组:PGPatcher

Community Shaders 能渲染视差、complex material 和 PBR,但只在网格已为此设置好的
地方。过去这意味着先装一堆预打补丁的网格,再装一个模组,在贴图缺失的地方把效果
重新关掉:覆盖范围受限于你手头恰好有的网格,还要在运行时花 CPU 去撤销安装时做出的
选择。

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) 把它反了
过来。你随意安装任何着色器类型的贴图,它会重写网格和插件,让每个表面都用上对的
那一种,包括插件里的备用贴图记录,那是旧方法覆盖不到的。没有它,需要某种着色器、而
网格并未请求该着色器的贴图只会被加载,却不会按预期渲染,这在混用多种贴图类型的配置
里最常见。

在 Linux 上运行前有两点:

- 它需要参数 `--ignore-mo2vfscheck`,否则会立刻退出并抱怨 MO2。Eidos 会提供。这个
  开关是什么、检查为何在这里失败,见[工具](tools.md#为什么-pgpatcher-需要---ignore-mo2vfscheck);
- 它会修改网格,所以要在所有贴图模组装完之后、在需要看到最终网格的 DynDOLOD
  之前运行。之后再改贴图,就得按这个顺序把两者都重跑一遍。

它的输出不进入 Overwrite。PGPatcher 只写入其启动器里 Output location 指定的文件夹,
并且不接受 `Data` 里面的文件夹。把这个位置指向它专用的文件夹,比如在实例的 `mods/`
下新建一个 `PGPatcher Output` 文件夹。PGPatcher 运行期间让那个模组保持禁用,因为
PGPatcher 会先清空文件夹再重新写入,而 Eidos 假定视图挂载期间启用的模组不会变化。
运行结束后再以高优先级启用它:它本来就是要赢的。

不要给 PGPatcher 的条目设置 **Capture output into** 模组,再把那个模组用作 Output
location。每次运行之后,Eidos 会把 PGPatcher 自己的 `cfg/` 和 `log/` 移进那个模组,
PGPatcher 随后就会报 "Output directory has non-PGPatcher related files" 并停止。

## Proton 下的 ENB:LinuxVersion

Jackify 为它在 Proton 下安装的每个 ENB,都会在 `enblocal.ini` 的 `[GLOBAL]` 段写入
`LinuxVersion=true`。ENB 自己的文档没有说明这个键。Diagnostics 标签页里的一张 Health
卡片会提示缺少这一行的 `enblocal.ini`,它的 Fix 按钮把生效的 `enblocal.ini` 加上这一行
后复制一份写入 `overwrite/Root/`,ENB 模组的文件保持不变。

另一张卡片会在游戏文件夹里有 ENB(`enblocal.ini`、`enbseries.ini` 或 `enbseries/`
文件夹),同时某个已启用的模组带有 Community Shaders
(`SKSE/Plugins/CommunityShaders.dll`)时提示。两者可以一起运行。如果画面不对,在它们
各自的 INI 里关掉其中一个。

## 事后怎么读这些数字

生成的帧只存在于呈现侧:引擎仍以基础帧率模拟,Havok 仍以基础帧率步进,一切统计*引擎*
帧的东西(包括 CS 自己的计数器)都会继续报 ~60,而显示器显示 ~120。计数器没有错,也正
因如此,帧生成对物理是安全的,而抬高引擎自身帧率则不然。启动选项里的 `DXVK_HUD=fps`
可以在屏幕上给你一个计数器。

一条规则:驱动级插帧(NVIDIA Smooth Motion,`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)与
CS 的帧生成是互相竞争的技术。二选一,绝不要同时开。
