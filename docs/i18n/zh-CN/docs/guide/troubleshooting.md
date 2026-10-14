<!-- eidos-i18n: source=docs/guide/troubleshooting.md sha=427084e50a9961f690747ca6fe98c2f1725defe9 -->

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

passthrough 把真正的后备文件交给内核,于是读取完全绕开这个守护进程。这是以正确性
为代价换来的吞吐提升。在 Skyrim SE 1.6.1170、proton-cachyos 11.0、内核 7.1.4 上做的
A/B 实测,同一份 82 个插件的加载顺序,唯一变量是二进制文件是否带着该能力:

| passthrough | `NtCreateFile` 以 `STATUS_ACCESS_VIOLATION` 失败的次数 |
|-------------|--------------------------------------------------------|
| 开          | 152 —— 75 个 `.bsa`、65 个 `.esl`、10 个 `.esm`、2 个 `.esp` |
| 关          | 0                                                      |

开着的时候,游戏打不开自己的任何封装档与插件,在游戏里表现为模组就是不在 —— 没有
报错,没有日志。关掉之后,同一份加载顺序带着它的插件、封装档和 Papyrus 脚本活着进入
游戏。

这个失败从守护进程内部是看不见的,这正是它难找的原因:我们自己的 `open` 每次都成功,
内核也从不拒绝后备文件(用 `EIDOS_FUSE_TRACE=open` 跑完一整场失败会话验证过:零次
`open FAILED`,零次 `passthrough refused`)。错误发生在守护进程回答
`opened_passthrough` 之后,所以守护进程侧的任何日志都看不到它。它也与扩展名无关 ——
封装档和插件同样中招,也就是游戏整场都保持打开的那些文件。

`EIDOS_FUSE_PASSTHROUGH=1` 会把它开回来,用于测量它带来什么,或重新验证该机制。
启动器和 Diagnostics 标签页里的能力警告,只在你主动要求它时才出现。

若要通过 Eidos 启动游戏本身,把它的 Steam 启动选项设为:

```
eidos play skyrimse -- %command%
```

如果 Proton 需要原生 d3dcompiler 来编译着色器,就在前面加上
`WINEDLLOVERRIDES="d3dcompiler_47=n"`;Eidos 会把它与模组自带的任何 DLL 覆盖
(ENB/ReShade/`.asi` 加载器)合并。

### 层索引真的在起作用吗?

索引是全有或全无,而且建立时悄无声息:`LayerStack::new` 要么拿到只读各层的完整映射,
要么拿到 `None`,此后每次查询都和从前一模一样地逐层遍历。会话日志里没有任何东西能
区分这两种情况,于是一个悄悄退回遍历的层栈,看上去和正常工作的一模一样 —— 却在付
旧账。

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` 用索引和不用索引分别解析真实路径,并比较目录扫描结果。`index_agrees`
检查两者给出的是**同一个**答案,覆盖真实实例的每条路径与每次列目录。`listing_cost`
测量合并子项映射在 `readdir` 上省下了多少。

`EIDOS_NO_INDEX=1` 强制走遍历,适用于正在调试的恰恰就是两种答案之间差异的时候。

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

不存在「这个文件系统是否大小写不敏感」的 API,所以 Wine 的
`get_dir_case_sensitivity` 靠嗅探 CIOPFS 留在其所服务目录里的标记。标记不在,Wine 就
假定**大小写敏感**,于是每一次拼写不能逐字节吻合的查找,都退化成读取**整个**目录来
寻找大小写无关的匹配。Bethesda 的游戏请求 `data/ccbgssse001-fish.bsa`,而文件名是
`ccBGSSSE001-Fish.bsa`,所以几乎每个资源都会触发:八秒内 4471 次标记探测、2236 次
完整重读目录,九十秒内 195796 次枚举 `Data`。Skyrim SE 从未走到主菜单 —— 它停在 240 MB
常驻内存,而守护进程烧掉了一个核心的 92%。

Eidos 从一开始就在 `resolve_read` 里折叠大小写。全部代价只来自它从不说出这一点。现在
`lookup` 会回答 `.ciopfs`;`readdir` 依旧不列出它。

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

**`FOPEN_KEEP_CACHE` 是关闭的。** 已修复,而且值得知道原因。它会在主菜单出现几秒后
让 Skyrim SE 因空指针解引用而崩溃,可稳定复现,且一个模组都没装;另外三个内核侧缓存
被逐个二分排除,只有这一个有影响。当时测量认为失去它是免费的,但那次测量是在
`FUSE_PASSTHROUGH` 生效的状态下做的 —— 那时守护进程服务的读取是**零**
(`EIDOS_FUSE_STATS` 在一次完整加载中报告 `read 0`),内核已经在对着后备文件缓存那些
页面。现在 passthrough 默认关闭(见下),所以那个理由不再成立,真实代价尚未测量 ——
不过光是崩溃就足以让它保持关闭。要调查的话用 `EIDOS_FUSE_KEEP_CACHE=1` 打开;两个开关
不再互相纠缠,因此现在可以单独测试它。

### FUSE passthrough 让游戏加载不了任何模组内容

通过关闭它修复;`EIDOS_FUSE_PASSTHROUGH=1` 可以把它带回来。passthrough 开启时,
Skyrim SE 在内核 7.1.4 上会以 `STATUS_ACCESS_VIOLATION` 打不开自己的 152 个文件
(75 个 `.bsa`、65 个 `.esl`、10 个 `.esm`、2 个 `.esp`),关闭时是 0 个 —— 也就是
说,任何模组内容都不会加载,而且悄无声息。内核是在守护进程回答 `opened_passthrough`
之后才抛出错误的,所以守护进程自己的日志显示这是一次干净的运行(零次打开失败,零次
后备文件被拒)。内核路径中的根因尚未查明;保留这个开关,是为了日后能重新验证,也为了
万一映像映射确实需要它时,可以把 passthrough 收窄到只对 DLL 生效。
