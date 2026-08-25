<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**从不碰你游戏的那个原生 Linux 模组管理器。**

</div>

Eidos 给 Linux 上的 Bethesda 游戏带来 Mod Organizer 2 在 Windows 上给它们的东西 -
一份虚拟的、每次启动时合成的模组视图 - 但它由 Linux 原语构建,而不是靠挂钩
Windows API。管理器不需要 Wine。不往游戏目录里复制文件。没有清理流程,因为
根本没有东西要清理。

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **状态:** Skyrim SE 每天都在通过 Eidos 游玩 - SKSE、script extender 预加载器、
> Creation Club、LOOT 排过序的加载顺序、按配置档案分开的存档,一整套。目前只有
> 一个游戏系列在真实游玩中得到验证;另外十个已经接好线,等着测试者。

## 为什么是 Eidos

- 🔒 **一个只有你的游戏能看见的挂载。** 合并视图存在于一个私有挂载命名空间里:
  你的文件管理器、你的备份任务、另一个游戏 - 谁都看不见它,谁也不需要为它授权。
  杀掉游戏,拔掉电源:命名空间随进程树一起消失,你的安装目录与原先分毫不差。
  没有残留是*构造上*如此。
- 🧾 **只有一份事实。** 你的配置档案拥有自己的模组列表、插件顺序、INI 和存档。
  插件文件和存档目录在启动时被 bind-mount 到游戏自己的路径上,所以连游戏自己写
  出来的东西也落进你的配置档案。切换配置档案就切换了一切。
- 🐧 **完全 rootless。** 没有 setuid 辅助程序,没有守护进程,不用 `sudo setcap`,
  也不用改 `/etc/fuse.conf`。一个可执行文件,一条 Steam 启动选项。
- 🛡️ **有凭据的守卫。** 一次毁掉你插件列表的崩溃,会对照会话前的快照被标出来,
  一键还原。一次会抹掉你加载顺序的捕获会被拒绝,并说明原因。

## 它做什么

**模组。** 简单压缩包、FOMOD 向导、Wrye Bash 的 BAIN 包,剩下的交给手动选择器 -
以及**原生支持 root 模组**(script extender 预加载器、ENB、Engine Fixes),不需要
Root Builder 插件,也不往你的安装目录里复制任何东西。隐藏单个文件、用分隔符分组、
定点移动、每个模组的备注与分类,还有一个 MO2 配置档案导入器。

这份列表就是 MO2 的那份,连习惯都一样:八个可选列、可按其中任意一列排序、按分类
或按来源分组、双击手势、输入即跳转、每个模组各自的备份(在你还原之前它们是惰性的),
以及对布局不会被本游戏加载、或是为另一个游戏下载的模组给出的提示标记。它的文件树
能做那些寻常操作 - 新建文件夹、重命名、删除、打开 - 并且不用启动任何东西就能预览
图片和文本。

**插件。** 内置 LOOT 排序的加载顺序、与游戏计算方式一致的模组索引、缺失 master 的
警告,以及把你的 DLC 和 Creation Club 内容按其本来面目显示为未管理的条目。

**实例。** 全局 - 集中管理在 `~/.local/share/eidos` 下 - 或者便携:一个放在任何地方
的自包含文件夹(第二块硬盘、一个游戏分区),可移动、相互隔离,和 MO2 的一样。便携
实例会跨会话被记住;GUI、Steam 启动和每一条 CLI 命令都跟随你上次用过的那个,而且
凡是接受游戏 id 的命令,同样接受这个文件夹。细节见
[usage.zh-CN.md](docs/guide/usage.md#实例全局与便携)。

**配置档案。** 每个配置档案各有自己的模组顺序、插件状态、INI 和存档。存档会被解析、
与你当前的插件比对 - 还有一个按钮可以启用某个存档所需要的东西 - 并在每次会话之后
同步回去供 Steam Cloud 使用。

**Nexus。** 连接一个账号,网站上的 "Mod Manager Download" 按钮就会直接落进你的实例,
并对照你已安装的东西检查更新,显示每个模组的作者和通往其个人页的链接。一个
**collection** 链接会列出它的成员并与你的实例做联结 - 已安装、已下载、缺失 - 这是
在读一个 collection,而不是在安装一个,面板里会说明原因。Downloads 标签页是一座
压缩包库:过滤、排序、不删除地隐藏,以及清除那些已经安装过的。一个**离线**开关可以
把这一切全部停掉。

**工具。** xEdit、BodySlide、DynDOLOD 和它们的同伴都在游戏的 Proton 前缀里*穿过合并
视图*运行 - 它们看得见你的模组,它们的输出落进 Overwrite,一次点击就能把它变成一个
真正的模组。每个工具需要的运行时会按需获取,所以缺一个 DLL 是一个按钮,而不是一个
下午。xEdit 和它的 QuickAutoClean 孪生兄弟会被自动找到 - 在游戏文件夹里、在某个模组
里,或者在你放在游戏旁边的工具目录里 - 并且已经替你选好了正确的运行时。把常用的钉住,
把不用的隐藏,当某个工具本身就是一个 Steam 应用时给它自己的 Steam AppID,还可以写一个
`.desktop` 快捷方式,让它穿过合并视图启动,完全不用打开 Eidos。

**诊断。** 缺失的 master、无主的压缩包、模组列表漂移、损坏的插件集合 - 以及在一次运行
之后,script extender 自己的日志说实际加载了什么。

**它把自己的文件放在哪。** `~/.config/Colony/Eidos/` 存放你选择的东西 - 偏好设置、你的
Nexus 会话、你的实例列表、你写的游戏与附加定义 - 日志在 `~/.local/state/Colony/Eidos/`
下。Colony 家族里每个程序都用这套布局。更早的 Eidos 把这些放在 `~/.config/eidos/`;
升级后的第一次启动会把它们复制过来,在日志里说明,并让旧目录保持原样。

## 它与其他方案的对比

| | Eidos | Wine 里的 MO2 | Fluorine-Manager | Limo / 链接部署类 |
|---|---|---|---|---|
| 管理器原生运行 | ✅ | ❌ Wine 里的 Windows 程序 | ✅(Qt 移植) | ✅ |
| 游戏目录不受触碰 | ✅ 始终 | ✅ | ✅ | ❌ 会往里写链接 |
| 挂载对谁可见 | 只有游戏 | 只有游戏 | **整个系统** | 不适用 |
| 崩溃后需要清理 | 无,设计使然 | 无 | 残留挂载恢复 | 手动取消部署 |
| Root 模组(ENB、预加载器) | ✅ 原生 | 需要插件 | 需要插件 | 部分支持 |
| 需要的权限 | 无 | 无 | 改 `/etc/fuse.conf` | 无 |

## 它有多快

| | 之前 | 现在 |
|---|---|---|
| 读取一个存档 | ~20 秒 | **6-7 秒** |
| 单次会话中的目录读取 | 560 万 | 46.5 万 |

切换单元格是即时的。提速来自少问你的模组几个问题:找一个文件以前要挨个盘问全部五十个,
列一个文件夹以前要重复干五十遍。现在两件事都不做了。这是在一个正常游玩的真实实例上
测出来的,不是跑分。

## 开始上手

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

然后把你游戏的 Steam 启动选项设为 `~/.local/bin/eidos-gui %command%`,按下开始游戏。

Arch 软件包和发行版压缩包、你需要先装什么,以及命令行路线:
**[docs/guide/install.zh-CN.md](docs/guide/install.md)**。

## Steam 启动选项

基础的这一行就是绝大多数配置所需要的全部:

```
~/.local/bin/eidos-gui %command%
```

其他一切都是叠在它前面的环境变量,而且它们可以自由组合:

| 你想要... | 放在前面 |
|---|---|
| 配合 Community Shaders 的 DLSS | `PROTON_ENABLE_NVAPI=1` - 没有它,DLSS 会悄无声息地永远初始化不了;完整清单见 [guide/graphics.zh-CN.md](docs/guide/graphics.md) |
| 屏幕上的 FPS 计数 | `DXVK_HUD=fps` |
| 驱动级插帧,零模组(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - 绝不要和 Community Shaders 自带的帧生成一起用 |
| 用于 bug 报告的详细日志 | `EIDOS_LOG=debug`(会话日志落在 `~/.local/state/Colony/Eidos/logs/`) |
| 来自挂载的单次会话 I/O 报告 | `EIDOS_FUSE_STATS=1` |
| 不同的 FUSE 工作线程数 | `EIDOS_FUSE_THREADS=8`(默认 4;追查并发 bug 时,`1` 是第一个该试的值) |
| 把这次启动钉在某个便携实例上 | `EIDOS_INSTANCE=/path/to/folder` - 没有它,Eidos 会打开你上次用过的实例,而那通常正是你想要的 |

一套现代模组配置(Community Shaders、DLSS、帧生成)该保留的那一行 - 这就是最终命令,
不是示例:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

在验证配置是否可用期间,把 `DXVK_HUD=fps` 加在前面,能用了就去掉。

更深层的诊断开关(`EIDOS_FUSE_TRACE`、缓存与索引的二分排查开关、以及
`EIDOS_FUSE_PASSTHROUGH` 为什么默认关闭)在
[guide/troubleshooting.zh-CN.md](docs/guide/troubleshooting.md)。

## 接下来去哪

| 如果你想... | |
|---|---|
| 安装它 | [guide/install.zh-CN.md](docs/guide/install.md) |
| 学会命令行和图形界面 | [guide/usage.zh-CN.md](docs/guide/usage.md) |
| 配置 xEdit、BodySlide 或 DynDOLOD | [guide/tools.zh-CN.md](docs/guide/tools.md) |
| 玩 Fallout 4(F4SE、版本、NVIDIA 碎片崩溃) | [guide/fallout4.zh-CN.md](docs/guide/fallout4.md) |
| 让 DLSS / 帧生成跑起来(Community Shaders) | [guide/graphics.zh-CN.md](docs/guide/graphics.md) |
| 修好看起来不对劲的东西 | [guide/troubleshooting.zh-CN.md](docs/guide/troubleshooting.md) |
| 知道它为什么快,并自己验证 | [internals/performance.md](../../internals/performance.md) |
| 理解它内部怎么工作 | [internals/architecture.md](../../internals/architecture.md) |
| 构建它、测试它、参与贡献 | [internals/contributing.md](../../internals/contributing.md) |
| 知道它究竟为何存在 | [project/landscape.md](../../project/landscape.md) |

一种语言就是一个目录:`docs/i18n/zh-CN/` 镜像了仓库根目录的结构,因此两个译文页面
之间的链接与它们英文原文之间的链接是同一串字符。

## 语言

玩家需要的页面都翻译了。**英文是准绳**:当译文与它不一致时,以英文文件为准。

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

**其余的一切是刻意用英文,而不是漏掉了。** `docs/internals/` 和 `docs/project/` 是给
那些同时也在读 Rust 的人看的,而 `CHANGELOG.md` 是生成的。翻译它们意味着还要为一批
并不需要的读者,再维持 17,678 个词的诚实。

每份译文都带着它所依据的英文文件的哈希,当英文往前走了,CI 就会失败 - 见
[`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh)。一份没法被重新更新到位的译文会被
**删除**,而不是留在原地:过期的页面看上去照样权威,却在发放上个月的命令,这对读者
比把他送去看英文更糟。

增加一门语言等于四个文件加这张表里的一行;
[`docs/internals/contributing.md`](../../internals/contributing.md) 里有步骤。

## 支持的游戏

**Skyrim SE/AE** - 已在真实游玩中验证。**Fallout 4** 也已经端到端接好(自动换用 F4SE、
archive invalidation、星号加载顺序、LOOT、`.fos` 存档) - 见
[guide/fallout4.zh-CN.md](docs/guide/fallout4.md)。按共享的游戏描述符接好线、
正在找测试者的有:Skyrim LE、Skyrim VR、Enderal SE、Fallout 3、Fallout NV、
Fallout 4(+ VR)、Starfield、Oblivion 和 Morrowind(后两者能挂载并管理模组;它们按
时间戳排序的插件列表还没有被管理起来)。

增加一个游戏系列就是一行描述符:
[internals/adding-games.md](../../internals/adding-games.md)。

## 前人的工作与致谢

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) 和
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos 复现的那套语义,以及它对照
  研究其行为一致性的代码库
- [LOOT](https://loot.github.io/) - 排序引擎,经由 libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) 以及其他 Linux 管理器 - 证明有一个社区
  希望这件事被解决

## 许可

GPL-3.0-or-later。模组管理属于所有人。
