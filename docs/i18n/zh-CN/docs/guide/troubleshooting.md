<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=55e68d8b6ee830b2d336ce1e96db32155a304c57 -->

# 排查与诊断

为「游戏看到的东西和文件系统说的不一致」那一天准备的全部内容:环境开关、如何读操作
计数器、已知问题及其来龙去脉,还有 passthrough 那笔账。

### 诊断 VFS

当游戏看到的东西和文件系统说的不一致时,有两个环境变量可用:

```sh
EIDOS_FUSE_STATS=1                  # 操作计数器,卸载时输出
EIDOS_FUSE_NO_CACHE=1               # 关掉内核侧的每一个缓存
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # 或者逐个点名
```

正是逐个点名的形式找出了下面描述的崩溃:四个全关回答的是「是不是缓存的问题」,只有
点名才回答「是哪一个」。计数器回答另一半 —— 一次显示 `read 0` 的加载,意味着
`FUSE_PASSTHROUGH` 在内核里供上了每一个字节,那么你打算在读取路径上做的一切调优
本来就已经免费了。

## 手动挂载一个合并视图

冲突时第一个 `--layer` 获胜;最后一个是你原封不动的游戏数据。挂载只需要 `/dev/fuse`
和 `fusermount3`(不用 overlayfs,也不用 Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... 通过 /mnt/point 读写 ...
fusermount3 -u /mnt/point
```

写入落到 `--overwrite <dir>`(省略时为临时目录),所以即便在这里,各层本身也保持
原样。

#### 为什么 passthrough 默认关闭

passthrough 把真正的后备文件交给内核,于是读取完全绕开这个守护进程。下面的 A/B 是
用较早版本的守护进程测的,早于后文已知问题「FUSE passthrough 让游戏加载不了任何模
组内容」里描述的改动。这个改动避开了最可能的原因,但还没有在游戏里重新测量过,所以
在重新测量之前 passthrough 默认保持关闭。在 Skyrim SE 1.6.1170、proton-cachyos
11.0、内核 7.1.4 上做的 A/B 实测,同一份 82 个插件的加载顺序,唯一变量是二进制文件
是否带着该能力:

| passthrough | `NtCreateFile` 以 `STATUS_ACCESS_VIOLATION` 失败的次数 |
|-------------|--------------------------------------------------------|
| 开          | 152 —— 75 个 `.bsa`、65 个 `.esl`、10 个 `.esm`、2 个 `.esp` |
| 关          | 0                                                      |

开着的时候,游戏当时打不开自己的任何封装档与插件,在游戏里表现为模组不在:没有报
错,没有日志。关掉之后,同一份加载顺序带着它的插件、封装档和 Papyrus 脚本活着进入
了游戏。

这个失败当时从守护进程内部是看不见的,这正是它难找的原因:守护进程自己的 `open` 每
次都成功,内核也从未拒绝后备文件(用 `EIDOS_FUSE_TRACE=open` 跑完一整场失败会话验
证过:零次 `open FAILED`,零次 `passthrough refused`)。错误发生在守护进程回答
`opened_passthrough` 之后,所以守护进程侧的任何日志都看不到它。它也与扩展名无关 ——
封装档和插件同样中招,也就是游戏整场都保持打开的那些文件。

`EIDOS_FUSE_PASSTHROUGH=1` 会把它开回来,用于测量它带来什么,或重新验证该机制。
启动器和 Diagnostics 标签页里的能力警告,只在你主动要求它时才出现。

若要通过 Eidos 启动游戏本身,把它的 Steam 启动选项设为:

```
eidos play skyrimse -- %command%
```

不需要加 `WINEDLLOVERRIDES` 前缀。当某个已启用的模组导入
`d3dcompiler_47` 时,Eidos 会把自带的副本部署进前缀,强制它以原生方式加载,
并与模组自带的任何 DLL 覆盖(ENB/ReShade/`.asi` 加载器)合并。

### 层索引真的在起作用吗?

`LayerStack::new` 要么为只读各层建出完整映射,要么一个也不建。没有映射时,每次
查询都和从前一样逐层遍历:答案相同,读一次存档大约要三倍的时间,其他方面看不出任何
区别。

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
检查两者给出的是**同一个**答案,覆盖真实实例的每条路径与每次列目录。`listing_cost`
测量合并子项映射在 `readdir` 上省下了多少。

`EIDOS_NO_INDEX=1` 强制走遍历,适用于正在调试的恰恰就是两种答案之间差异的时候。

## 无法关闭的游戏或工具

游戏或工具运行时，运行按钮 Run 会变成 **Stop**，锁定卡片上也有一个 Stop 按钮。按两次。第二次按下会结束这次运行启动的所有程序：先发 SIGTERM，10 秒后对剩下的发 SIGKILL。游戏没有保存的内容会丢失。之后 Eidos 做的事与任何退出之后相同：INI 和插件的改动回到配置档，工具的输出被记录，模组被卸载。**Unlock** 不一样：它把窗口交还给你，程序继续运行。在终端里对 `eidos` 进程执行 `kill <pid>`（pid 显示在锁定卡片上）与 Stop 效果相同。

## 游戏由哪个 Proton 运行，以及损坏的前缀

Diagnostics 标签页有一张 **Proton** 卡片：Eidos 和 Steam 为该游戏使用的版本、Steam 自己的设置，以及最后运行该前缀的版本。如果最后准备前缀的 Proton 比所选的更新，卡片会变成建议，因为在较新的前缀上用较旧的 Proton 可能会重置其中的 Wine 文件，或者无法启动。请在 Steam 中游戏的属性、兼容性里选择一个至少同样新的版本。

该卡片上的 **Rebuild prefix** 用于已经无法使用的前缀。按两次。Eidos 不删除任何东西：它在同一文件夹里把 `steamapps/compatdata/<appid>` 重命名为 `<appid>.eidos-backup-<date>-<time>`，下次你从 Steam 启动游戏时，Steam 会创建一个新前缀。只要有游戏、工具或任何 Wine 程序在使用该前缀，它就会拒绝。只保存在旧前缀里的存档和设置会留在重命名后的文件夹中：把需要的复制回来，不再需要时请自己删除那个文件夹。

## 已知问题

### DLSS 或帧生成静悄悄地什么都不做

三个各自独立的原因,每一个都没有任何错误提示:启动选项里没开 NVAPI、独占全屏,或者
一个陈旧的 Reflex 帧率上限。完整清单在 [graphics.zh-CN.md](graphics.md)。

**一个把同一个目录写成两种拼法的模组,会丢掉第二种拼法下的全部内容。** 已修复。ext4
把 `meshes/` 和 `Meshes/` 当作两个目录;合并视图不能这样,而真实的模组确实两种都用 ——
XP32 Maximum Skeleton 的动画和 FNIS 行为文件在首字母大写的那个下面,`character
assets` 在另一个下面。

解析器对每个路径分量取大小写完全匹配的那一个,并就此认定:它进了 `meshes/`,在里面
找不到路径的其余部分,于是**放弃整个层**。另一种拼法下的每个文件对游戏都是隐形的 ——
没有报错,没有日志,任何诊断里都没有。在一个真实的 50 层实例上,那是 74 个文件。

现在匹配上的分量只是候选,不是决定;仍然先试完全匹配的大小写,只有当其下的剩余部分
失败时,扫描才去找大小写折叠后相等的同级目录。列目录在上一层目录有同样的毛病,现在
每层都会读取每一个折叠后相等的目录。

**DynDOLOD 的 LODGen 死掉,只留下一个空日志。** 由 `dotnet10` 修复;见
[tools.md](tools.md)。症状不会认错:每个世界的 `LODGen_SSE_<world>_log.txt` 里只有
一行版本横幅、一行 `.NET Version:`,再无其它,加上一个只说
"failed to generate object LOD for one or more worlds" 的对话框。原因是 Wine 的 Mono
代替 .NET Framework 应答,而且装多少 .NET Framework 都没用 —— Proton 会在每次前缀
更新时把 `mscoree.dll` 换成指向它自己目录树的符号链接。

**Wine 无法得知这个挂载会折叠大小写。** 已修复,而且这是最要命的一个。

Wine 的 `get_dir_case_sensitivity` 先用 `FS_IOC_GETFLAGS` 询问目录,再嗅探 CIOPFS
留在其所服务目录里的标记。两者都没有,Wine 就假定**大小写敏感**,于是每一次拼写不
能逐字节吻合的查找,都退化成读取**整个**目录来寻找大小写无关的匹配。Bethesda 的游
戏请求 `data/ccbgssse001-fish.bsa`,而文件名是 `ccBGSSSE001-Fish.bsa`,所以几乎每
个资源都会触发:八秒内 4471 次标记探测、2236 次完整重读目录,九十秒内 195796 次枚
举 `Data`。Skyrim SE 从未走到主菜单 - 它停在 240 MB 常驻内存,而守护进程烧掉了一
个核心的 92%。

Eidos 从一开始就在 `resolve_read` 里折叠大小写。全部代价只来自它从不说出这一点。
现在 `lookup` 会回答 `.ciopfs`;`readdir` 依旧不列出它。`ioctl` 也会对目录回答
`FS_IOC_GETFLAGS` 并带上 casefold 标志,当前的 Wine 会在查找标记之前先问这个。

有两件事让它从「慢」变成「致命」。代价随目录大小增长,所以装上 Anniversary 内容
(`Data` 从 37 个文件变成 177 个)就压垮了它。而且 `opendir` 会急切地构建合并列表,
当 Wine 打开一个目录只是为了 `stat` 里面那个标记时,这纯属浪费 —— 现在快照改为在第一次
`readdir` 时才拍。

之后:主菜单、2.1 GB 常驻、守护进程 0% CPU。

`EIDOS_FUSE_TRACE=opendir` 就是找出它的工具,并且随程序附带。操作计数器只说「有多少
次」;一个目录被枚举 195796 次,在总数里是看不出来的。

**游戏把 `plugins.txt` 重写成空的**,很可能是同一回事 —— 一个它在合理时间内枚举不完的
`Data`,于是它断定那里什么都没有,并把这个结论存了下来。未经证实,值得复查。无论如何,
捕获保护(任何会把活动集合整个清空的捕获,无论规模一律拒绝)意味着它再也损坏不了
配置档。

**`FOPEN_KEEP_CACHE` 是关闭的。** 已修复,而且值得知道原因。它会在主菜单出现几秒
后让 Skyrim SE 因空指针解引用而崩溃,可稳定复现,且一个模组都没装;另外三个内核侧缓
存被逐个二分排除,只有这一个有影响。那次二分是在 `FUSE_PASSTHROUGH` 开启的状态下
做的,而内核会让带着 `FOPEN_KEEP_CACHE` 的 passthrough 打开以 EIO 失败,所以开着这
个标志时,每一次 passthrough 打开都失败了。让游戏崩溃的也许正是这些失败的打开,而
在 passthrough 关闭的情况下单独测试这个标志,从来没做过。passthrough 回复现在不再
带它。当时测量认为失去它是免费的,但 passthrough 开启时守护进程服务的读取是零
(`EIDOS_FUSE_STATS` 在一次完整加载中报告 `read 0`),所以在 passthrough 默认关闭的
现在(见下),真实代价尚未测量。要单独测试的话用 `EIDOS_FUSE_KEEP_CACHE=1` 打开。

### FUSE passthrough 让游戏加载不了任何模组内容

守护进程现在避开了最可能的原因(见下),但这还没有在游戏里重新测量过,所以
passthrough 默认保持关闭;`EIDOS_FUSE_PASSTHROUGH=1` 可以打开它。这次改动之前,
passthrough 开启时,Skyrim SE 在内核 7.1.4 上会以 `STATUS_ACCESS_VIOLATION` 打不
开自己的 152 个文件(75 个 `.bsa`、65 个 `.esl`、10 个 `.esm`、2 个 `.esp`),关闭
时是 0 个 - 也就是说,任何模组内容都不会加载,而且悄无声息。内核是在守护进程回答
`opened_passthrough` 之后才抛出错误的,所以守护进程自己的日志显示的是一次干净的运
行(零次打开失败,零次后备文件被拒)。

内核给每个 inode 只绑定一个后备文件(`fs/fuse/iomode.c`)。如果一个 inode 已经有后
备文件,再带来第二个后备文件 ID 的打开会以 EIO 失败;对已有后备文件的 inode 做普通
打开(不带 `FOPEN_PASSTHROUGH`)也一样。带着 `FOPEN_KEEP_CACHE` 的 passthrough 打
开同样以 EIO 失败。以前的守护进程每次打开都注册一个新的 ID,所以游戏已经开着的文
件被第二次打开时就会失败。Wine 把 EIO 报告为 `STATUS_ACCESS_VIOLATION`,这正是那
152 次打开失败背后的错误。这条规则是从内核源码读出来的,并且与那 152 次失败吻合,
所以它是最可能的原因。

现在守护进程每个 inode 只注册一个后备文件,由该 inode 的所有 passthrough 打开共享,
并且从不在 passthrough 打开上设置 `FOPEN_KEEP_CACHE`。passthrough 只用于游戏层和
模组层中大小达到 1 MiB 或以 `.dll`、`.exe`、`.asi` 结尾的文件的只读打开。这个后
备文件一旦注册,该文件的每一次只读打开都使用它;只要该文件还有普通打开,新的读者拿
到的也是普通打开。Overwrite 里的文件从不走 passthrough,投射插件时间戳的挂载则完
全不使用它。当 passthrough 读者还占着一个文件时,以写入方式打开它会在任何 copy-up
之前得到 ETXTBSY(在 Wine 中是 `STATUS_SHARING_VIOLATION`),因此 `O_TRUNC` 打开不
会在 Overwrite 里留下一个遮住模组文件的空副本。

### Mod Manager Download 按钮没有反应

“偏好设置 > 下载”会显示由哪个程序接收 `nxm://` 链接。如果该程序不是 Eidos、已不存在或尚未设置，“健康”页会多出一张卡片。“注册”按钮会写入 `eidos-nxm.desktop`，并把它设为 `x-scheme-handler/nxm` 的默认处理程序。在终端中，`eidos nxm --register` 的效果相同。

如果状态已显示 Eidos，按钮仍然没有反应，可能是浏览器沿用了之前的选择。请在浏览器设置中清除 nxm 链接的已保存选择，让它重新询问。Flatpak 版浏览器无法直接启动宿主机上的程序，而是通过桌面门户转交链接，门户读取的是在宿主机上注册的处理程序。
