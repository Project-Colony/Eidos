<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# 通过 Eidos 玩 Fallout 4

Fallout 4 不需要任何特殊启动选项、不需要改名的可执行文件、也不需要包装脚本。这话值得
说明白,因为其它所有 Linux 上的 F4SE 教程都说反了 —— 而它们的建议会在下一次 Steam
更新时崩掉。

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

或者把 `f4se_loader.exe` 覆盖到 `Fallout4Launcher.exe` 上 —— 而 Steam 会在每次游戏
更新时悄悄还原它,此后你就在没有 F4SE 的情况下游玩,却没有任何提示。

Eidos 自己完成这次替换,依据是游戏描述符:装了 `f4se_loader.exe` 就用它替换启动器,
没装则退回 `Fallout4.exe`,而且在不得不退回时**会告诉你**。一个所有 F4SE 模组都失效
却照常启动的游戏,比根本启动不了更糟。

还有第二个绝不该运行启动器的理由:它会重新扫描 `Data` 并重写 `plugins.txt`,把刚刚
部署好的加载顺序一笔勾销。Eidos 从不执行它。

## Eidos 替你处理的事

| | |
|---|---|
| 档案失效 | `Fallout4Custom.ini` 会被写入 `[Archive]` `bInvalidateOlderFiles=1` 和空的 `sResourceDataDirsFinal=`,这两个键让 `Data` 之外的散装文件根本能被看见。写进配置档,而不是游戏目录。 |
| 加载顺序 | `plugins.txt` 采用 Fallout 4 使用的星号格式(`*` 表示启用),并遵循 `Fallout4.ccc` 处理隐含的 Creation Club 插件 |
| LOOT | 排序方式与 Skyrim 相同 —— `eidos sort <instance>` 会取 `fallout4` 的 masterlist |
| 存档 | `.fos` 存档及其 `.f4se` 协同存档会被列出、复制并按配置档保存;详情面板会读取存档自身的插件表,所以一个需要你已禁用插件的存档会在读取前就说出来 |
| Root 模组 | 模组随可执行文件一同提供的东西(F4SE 本身、ENB、`dxvk.conf`)都通过 Skyrim 同样的 `Root/` 机制落到那里 |

## 版本问题

Fallout 4 不再是 2019 到 2024 年间那个冻结的游戏。截至 2026 年 8 月,有三条活跃分支,
为其中一条构建的模组 DLL 无法在另一条上加载:

| 分支 | 版本 | F4SE |
|---|---|---|
| 经典版(old-gen) | 1.10.163 | 0.6.23 |
| 次世代版 | 1.10.984 | 0.7.2 |
| 周年版 / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

在搭建模组列表之前值得知道的两个后果:

- **确认你手上到底是哪一个。** 游戏根目录里有 `Creations/` 和 `Mods/` 文件夹,说明你
  在 1.11.x 这条线上。Eidos 里存档的详情面板也会显示写下它的构建版本 —— Fallout 会把
  它写进存档,Eidos 以「Game build」呈现。
- **刚打完补丁不是开工的好日子。** F4SE 通常在 Bethesda 更新后一两天内跟上,但
  *Address Library for F4SE Plugins* —— 大多数 DLL 模组靠它解析偏移 —— 按自己的节奏
  走。在这两者之间,生态里 DLL 的那一半是瘫的。不含 DLL 的模组(贴图、模型、插件)
  不受影响。

一旦你的整套配置跑通,就把 Fallout 4 的 Steam 自动更新关掉(属性 → 更新 →「仅在启动
游戏时更新」),否则下一个补丁会把你装的每一个 DLL 都打碎。

## 硬件提示:NVIDIA 上的武器碎片崩溃

Fallout 4 的武器碎片效果基于 NVIDIA FleX,那是 NVIDIA 在 Pascal 之后停止支持的 PhysX
衍生物。在任何 Turing 及更新的显卡上 —— GTX 16、RTX 20 直到 RTX 50 —— 它都会让游戏
崩溃。这是游戏本身的缺陷,与 Linux、Proton 或 Eidos 无关。

两个办法,任选其一:在游戏设置里关闭「Weapon Debris」,或安装
*Weapon Debris Crash Fix*(Nexus 48078),它禁用的是碎片的碰撞而不是效果本身。

## 如果哪里不对劲

通用排查清单在 [troubleshooting.zh-CN.md](troubleshooting.md);而 Fallout 特有的
第一个问题永远是*究竟启动了哪个可执行文件*。Eidos 会把完整的启动命令写进实例的运行
日志,所以:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

如果它写的是 `f4se_loader.exe`,替换成功了。如果写的是 `Fallout4Launcher.exe`,说明
F4SE 没装在 Eidos 找得到的地方 —— 它该待在游戏可执行文件旁边,对于用模组管理器的配置
来说,那就是某个模组的 `Root/` 目录(或者手动装进游戏目录本身)。
