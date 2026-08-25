<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders、DLSS 与帧生成

Community Shaders 1.4+ 自带超分(DLSS 4 / FSR 3.1 / XeSS,通过单独的
"Upscaling - Community Shaders" 包)以及 FSR 3.1 帧生成。这些在 Linux 上都能穿过
Eidos 工作 —— CS 及其附加包按普通模组安装,合并视图像对待其它文件一样提供它们的
DLL —— 但有三件事在游戏里**看不出来**,而且每一件都会让功能静悄悄地什么也不做。
本页就是这份清单,是在真实环境里吃过苦头换来的。

## DLSS 需要的启动选项

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

除非游戏在 Valve 的白名单上,否则 Proton 会关闭它的 NVIDIA NVAPI 层(dxvk-nvapi),
而 Skyrim 不在名单里。没有它,CS 无法初始化 DLSS,会悄悄退回 FSR 超分,屏幕上不会
有任何提示说明原因。在非 NVIDIA 机器上设置这个变量毫无代价,所以放心用的启动选项就是
上面那一行。帧生成本身是 FSR 3.1,不需要 NVAPI;只有 DLSS 超分需要。

## 帧生成要求无边框窗口

CS 的帧生成建立在 D3D12 呈现代理之上,并且干脆拒绝独占全屏。`SkyrimPrefs.ini` 里的
`bFull Screen=1` 意味着它永远不会启动 —— 没有报错,没有提示,只有基础帧率。稳妥的
办法是 SSE Display Tweaks,它在引擎层面强制模式,不管 INI 怎么写:

```ini
[Render]
Fullscreen=false
Borderless=true
```

窗口看起来一模一样(无边框、原生分辨率);变的只是引擎的认知 —— 而引擎的认知正是
CS 检查的东西。

还有两个启用条件,失败方式同样安静:

- **显示器刷新率 120 Hz 或更高**,或者在 CS 的超分设置里打开
  `frameGenerationForceEnable`。帧生成会把呈现帧率翻倍,所以 CS 拒绝在显示不出结果
  的显示器上启用它。
- **已安装 Upscaling 包**(它的 `Data/Shaders/Upscaling/` 目录里放着 Streamline 与
  FidelityFX 的 DLL)。没有它,CS 会显示菜单项却什么也开不了。

## Reflex 的帧率上限可能把输出掐死

CS 的 Reflex 设置自带 FPS 上限(`reflexFPSLimit`,配合 `reflexUseFPSLimit`)。停留
在旧值的上限 —— 我们的是 79,来自很久以前的一次调校 —— 位于帧生成的下游,恰好把它
产出的帧砍掉:基础 60 翻倍到 120,再被压回 79,看上去就是「帧生成没起作用」。144 Hz
显示器上常规的 Reflex 上限约为 138。只要觉得生成的画面不见了就查它;这是继独占全屏
之后的第二个无声杀手。

## 已知交互:与 SSE Display Tweaks 一起出现黑屏

FG + Display Tweaks + DXVK 这个组合有已知的黑屏故障。按顺序修:

1. `SSEDisplayTweaks.ini`:`DisableBufferResizing=true`
2. 若仍不行,在游戏可执行文件旁放一个 `dxvk.conf`(模组的 `Root/` 目录就能放到那里),
   内容为 `dxvk.enableGraphicsPipelineLibrary = False`

## 事后怎么读这些数字

生成的帧只存在于呈现侧:引擎仍以基础帧率模拟,Havok 仍以基础帧率步进,一切统计*引擎*
帧的东西(包括 CS 自己的计数器)都会继续报 ~60,而显示器显示 ~120。这是正确行为,不是
计数器坏了 —— 也正因如此,帧生成对物理是安全的,而抬高引擎自身帧率则不然。启动选项里
的 `DXVK_HUD=fps` 可以在屏幕上给你一个计数器。

一条规则:驱动级插帧(NVIDIA Smooth Motion,`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)与
CS 的帧生成是互相竞争的技术。二选一,绝不要同时开。
