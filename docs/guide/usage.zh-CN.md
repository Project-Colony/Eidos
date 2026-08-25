<!-- eidos-i18n: source=docs/guide/usage.md sha=0fec5e6c87047a79c0ddc97d73bb492b7e05bd5b -->

# 使用 Eidos

实用手册:CLI、GUI、Steam 启动选项、从源码构建,以及概念验证脚本。事情看起来不对
劲时该怎么办,见 [troubleshooting.zh-CN.md](troubleshooting.zh-CN.md)。

## 用起来(CLI)

```sh
eidos games                       # 本机装了哪些受支持的游戏(相当于 MO2 的那份列表)
eidos init skyrimse               # 创建一个模组实例
# ...把每个模组作为一个文件夹放进 <instance>/mods/(全局实例位于
#    ~/.local/share/eidos/skyrimse;`eidos init` 会打印出你的那个)...
eidos install skyrimse mod.7z     # 或者安装一个下载好的压缩包(Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # 接管已有 MO2 配置档的顺序与插件状态
eidos sort skyrimse               # 用 LOOT 排序插件加载顺序
eidos play skyrimse               # 显示将会挂载什么
eidos play skyrimse -- <command>  # 在模组盖住游戏的视图下运行 <command>
```

`eidos tool`、`eidos prereqs`、`eidos nexus`、`eidos nxm` 与 `eidos export` 补齐了
这一组;不带参数运行 `eidos` 可看到完整列表。

### 实例:全局与便携

上面每条命令都作用于某个实例。`skyrimse` 指的是**全局**那一个 - 集中存放在
`~/.local/share/eidos/skyrimse`,由 Eidos 管理。另一种是**便携**的:一个自包含的
文件夹,放在你想放的任何地方(第二块硬盘、游戏分区),可移动、彼此隔离,和 MO2 的
便携实例完全一样。凡是接受游戏 id 的命令,同样接受便携实例的文件夹:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # 在那里创建一个便携实例
eidos install /mnt/games/EidosSkyrim mod.7z  # 每条命令都接受这个文件夹
eidos play /mnt/games/EidosSkyrim -- %command%
```

这个文件夹是自描述的(它的 `eidos-instance.ini` 写明了游戏),所以别的什么都不需要
- 而环境里的 `EIDOS_INSTANCE=<folder>` 会把一个游戏 id 重定向到该文件夹,这在 Steam
启动选项里很方便。你创建过或打开过的便携实例会被记住(最近使用的在前),记在
`~/.config/Colony/Eidos/instances.ini` 里;GUI 的欢迎界面把它们列出来供一键打开,
Steam 启动会落在你上次玩的那一个上,`nxm://` 处理器也下载进它里面。有两点值得知道:
移动便携文件夹后一切照旧,只有你用指向旧位置的绝对路径注册的工具条目除外(那些要重新
添加);而共享的 runtime 缓存(`~/.local/share/Colony/Eidos/runtimes/`)是有意保持
全机器共用的 - 一个 78 MB 的 .NET host 不该每个实例来一份。

Eidos 把自己的文件放在 `Colony/Eidos` 下,这是 Colony 家族每个程序都用的布局:
`~/.config/Colony/Eidos/` 放你的选择(偏好设置、你的 Nexus 会话、你的实例列表、你写的
游戏与附加组件定义),`~/.local/state/Colony/Eidos/logs/` 放会话日志,
`~/.local/share/Colony/Eidos/` 放 Eidos 下载来的东西。更早的 Eidos 把这些放在
`~/.config/eidos/` 和 `~/.local/state/eidos/`;升级后的第一次启动会把它们**复制**
过来,并在日志里说明。旧目录原样保留 - 什么都不删,所以一次糟糕的升级不会让你丢掉
登录状态 - 等你确认没问题,可以自己删掉。

你的模组不在其中。全局实例仍然位于 `~/.local/share/eidos/<game>/`,便携实例则在你放
它的地方,因为这些路径被写进了你的实例列表,也可能写进了 Steam 启动选项:移动它们会
切断一条 Eidos 并不同时掌握两端的链接。

有一个位置被直接拒绝:**游戏安装文件夹内部**(MO2 老手的条件反射)。那棵目录树归
Steam 所有 - 一次更新、一次"验证文件完整性"或一次卸载都可能重写或删除它,把你整套
配置一并带走 - 而且 Eidos 挂载在游戏根目录之上,放在那里的实例会位于它自己的挂载目标
里面。向导、`eidos init` 和 `eidos play` 都会拒绝;请把文件夹紧挨着游戏放(同一块盘
上的同级目录一样方便)。

`play` 把实例的模组挂载到游戏自己的 `Data` 目录之上(通过一个 bind-stash,这样守护
进程读到的仍是原始文件),挂在一个私有命名空间里,然后让命令穿过这个视图运行。写入
(存档、重新生成的配置)落在实例的 `overwrite/` 层;游戏安装目录和每一个模组源都
逐字节保持原样。

### 不需要任何特权步骤

Eidos 完全以无 root 方式运行。它挂载在私有的 user + mount 命名空间里,所以没有 setuid
助手程序,没有守护进程,也没有什么要授权的。

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` 是**可选的**,只决定一件事:内核
FUSE passthrough,而它默认关闭,因为它会弄坏游戏(见下)。带上这个 capability 后,
Eidos 取的是普通的 mount 命名空间而不是 user 命名空间;两种方式部署模组的结果一模一样。


旧的 `setcap` 建议为什么被移除 - 以及 FUSE passthrough 为什么出厂就是关的 - 在
[troubleshooting.zh-CN.md](troubleshooting.zh-CN.md#为什么-passthrough-默认关闭)
里有解释。

## GUI

```sh
cargo run -p eidos-gui
```

一个 MO2 风格的首次启动向导,采用 Colony 的羊皮纸 / 酒红观感:欢迎 -> 实例类型
(便携 / 全局)-> 游戏 -> 名称与位置 -> 摘要 -> 创建 -> 主界面。欢迎界面也会列出每一个
已知的现有实例(全局与便携,最近使用的在前)供一键打开 - 它同时就是实例切换器 - 而把
向导指向一个已经装着实例的文件夹时,它会原样接管,而不是覆盖着新建(若该文件夹属于
另一款游戏,则直接拒绝)。

双栏主窗口也已经做好:一个配置档选择器(切换,或复制当前的来新建一个),一个可过滤、
可选择、可重排、可用分隔条分组、可按分类收窄并右键操作的模组列表,外加 Data / Plugins /
Conflicts / Overwrite / Saves / Downloads / Diagnostics 各标签页,以及一个带运行目标
选择器的 Run 按钮。

重排不只有置顶/置底:MO2 那些精确移动这里也有 - 移到第一个冲突模组之上、最后一个之下、
移到一个明确的优先级,或移进某个分隔条的分组。它们全都走同一个共享的移动助手,所以
先删行再插回去带来的差一错误只存在于一个地方,而不是五个。

### 列、排序与分组

列表开箱画四列,一共提供八列:Category、Content、Version、Author、Installed、Nexus
id、Game、Flags。在 View 菜单里勾选。默认不是八列全开,这是有意的 - 每一列都显示的
列表就没地方留给名称了,而名称才是你真正在读的那一列。

点任意列标题即按它排序。再点一次反向,第三次点回到**加载顺序**,这比听上去更要紧:
加载顺序是列表唯一可以拖动的顺序,因为插入间隙寻址的是真实列表,而排序后的行完全在
别的地方。排序开着的时候,插入条不再绘制,拖动会被拒绝,而不是落到没人瞄准的位置 -
MO2 就是这么做的,理由也一样。View 菜单会写明这一点,并给出回去的方法。

View 菜单还能把整个列表**分组**,按分类或按来源(来自 Nexus,还是手工安装的)。分组
标题不是分隔条:它们背后没有东西可以重命名、上色或移动,它们可以折叠,折叠时计数留在
标题上。排序或分组之下,分隔条会离开列表 - 分隔条统领的是加载顺序中跟在它后面的那些
行,而排序和分组都把那些行挪走了。

### 鼠标与键盘

双击模组打开 Information,Ctrl+双击打开它的文件夹,Shift+双击打开它的 Nexus 页面。
Ctrl+F 把光标放进过滤框。敲一个字母跳到下一个以它开头的模组,再按一次继续走完其余的,
而不是卡在第一个上。它们都不会落到被过滤、被折叠的分隔条或被折叠的分组藏起来的行上 -
移动一个你看不见的高亮,正是下一次按空格会切换一个你根本没在看的模组的原因。

分隔条菜单里的 "Collapse others" 会折叠除该组之外的每一组。拖动过程中,停在一个折叠的
分组上会把它展开,于是模组可以直接放进去,不必先放弃这次拖动 - 是停留,不是掠过。

### 列表会告诉你一个模组的哪些事

两个提示性标记,都是一个字形,悬停给出解释。**No valid game data** 意思是这个模组顶层
没有任何东西看起来像本作会加载的内容;它可能需要把文件夹上移一层,也可能根本不是这款
游戏的模组。**Another game** 意思是模组自己的 `meta.ini` 写的是另一款游戏。两者都不拦
任何事 - 模组照样部署 - 行菜单里的 "Mark as valid" 可以让任一个闭嘴,走的是 MO2 自己的
`validated=` 键,所以你在一个管理器里担保过的模组,到另一个里也是安静的。

布局检查是刻意宽松的:一棵 `Root/` 树算,读不了的文件夹算,空文件夹也算。在一个五百行
的列表上,一个错误的警告比一个漏掉的警告更糟。

### 动一个模组之前先给它做备份

"Back up this mod" 把它的文件夹复制到旁边,叫 `<name>_backup`(然后是 `_backup2`,
以此类推 - 备份永远不会替换上一个)。这份副本是**惰性的**:它不是模组,它的勾选框
什么也不做,对合并视图毫无贡献,因为勾上它就会把同一个模组的两份拷贝叠着部署。
"Restore this backup over the mod" 两下点击把它放回去;当前内容会先被挪到一边,只有
在复制成功之后才丢弃。

**Data** 是合并视图的一棵真实目录树,一次展开一层,所以打开一个节点的代价是每个含有它
的图层各读一次目录,而不是递归遍历每一个启用的模组。回答它的是挂载所依据的同一套图层
栈,所以 whiteout 与隐藏文件都被遵守,这个标签页不可能和游戏将会看到的东西相左。按
名称过滤它,收窄到只看有争议的文件,用 Size 与 Modified 两列理清什么在哪里,任意一行
都可以在文件管理器里 Reveal。**Plugins** 是 ESP/ESM/ESL 的加载顺序(勾选、手工重排,
或者用 LOOT 排序并阅读排序后的报告,报告里的建议链接会在你的浏览器中打开)。
**Conflicts** 解释每个文件的赢家与输家。**Overwrite** 一步把游戏写出来的东西变成一个
真正的模组。**Saves** 解析每个存档的头部 - 角色、等级、地点、游戏时长 - 并把烘进存档
的插件列表与你当前的做差异比较,还带一个按钮来启用它需要的模组,因为只把它们的名字
念出来、剩下的留给你自己弄,那是无聊的那一半。

"Information..." 打开一个按模组的对话框:general、conflicts、filetree、INI tweaks、
notes。从 filetree(以及从 Data 树)里,任何文件都可以被**隐藏** - 重命名为
`<name>.mohidden`,这会把它从虚拟视图里剔除而不删除它,于是一个模组里三个碍事的 mesh
可以被压掉,而不用去动优先级。filetree 也做常规的文件操作:新建文件夹、重命名、删除、
打开。它们全都经过同一个解析器,任何不是该模组内部普通路径的东西都会被拒绝 - 不许
`..`,不许绝对路径,任何一段都不许是符号链接,因为跟着它走会让一次删除跑到模组文件夹
之外去。重命名只替换最后一段,所以它永远不可能变成一次移动;而且名字已被占用时它会
拒绝,而不是悄悄替换掉那个文件。删除要点两下;它是这里唯一一个再点一次也撤不回来的
操作。

filetree 或 Data 树里任意一行上的 **View** 会预览该文件:图片和文本。不支持 DDS 或
NIF - 那需要一个块解码器和一个这棵树没有的渲染器 - 但它们会明说,而不是给你一个空框,
并指向 Reveal。文本最多读到 64 KB 并说明它在哪里停下,因为预览是瞥一眼,而一份
Papyrus 日志可以有一百兆。**INI Tweaks** 列出模组在自己的 `INI Tweaks/` 文件夹里带来
的片段;启用的那些会在启动时按优先级顺序合并进配置档的游戏 INI,并在采集这次运行的
INI 时再摘下来 - 否则一条 tweak 会悄悄变成一项设置,禁用它也就什么都不做了。

一个下载可以**从 Downloads 列表拖到模组列表的某个位置上**,以那个优先级安装;从文件
管理器拖到窗口上的压缩包或文件夹也一样会安装(这后半件事需要 X11 或 XWayland 会话 -
winit 只为 X11 实现了文件拖放)。下载本身可以暂停和继续:暂停会停止传输并保留已下载
的部分,Resume 会重新解析一个新链接并从停下的地方接着下。

Downloads 标签页是一个压缩包**库**,不是传输队列。按名称过滤它(也认友好的模组名,
所以 "skyui" 能找到 `SkyUI_5_2_SE-12604-5-2SE.7z`),按最新、名称、大小或状态排序,
并且可以**隐藏**一个你用完了的压缩包 - 这会保留文件而只去掉那一行,把书放回架上不等于
烧掉它。"Show hidden" 把它们带回来,同一个按钮也取消隐藏。"Remove N installed" 两下
点击删除你已经安装过的模组的压缩包,而且只删**屏幕上**的那些:过滤就是你表明你指的是
哪些的方式。

### Nexus 合集

粘贴一个合集链接 - 或者在站点上点一个 - Eidos 就会列出该修订版的成员,每一个都与当前
实例对照:已安装、已下载,或缺失。它**读取**一个合集;它不安装,面板里也这么写。有四件
事让安装器在这里变成不诚实,而不只是难做:成员是普通的 Nexus 文件,需要一个按文件的
密钥,而在站点自己的按钮之外只有 premium 账号才签得出来;一次完整安装是每个成员三次
API 调用,对着一份这个客户端拒绝超支的预算;清单里的阶段、规则以及回放的 FOMOD 答案
没能对着一份真实发布的 Bethesda 合集验证过,而靠猜会产出一个看着对、其实不对的加载
顺序。读取只花一次请求,而且是精确的。

一个合集只能对着**它自己的游戏**读取。在加载着 Fallout 4 实例时打开一个 Skyrim 合集,
它会指名拒绝,而不是把成员对着错误的模组列表去比对 - 在那里每一个"已安装"和每一个
"缺失"都是披着答案外形的噪声。

### 离线模式

**Settings -> Nexus -> Offline** 让 Eidos 完全不去联系 Nexus。更新检查、登录、下载和
合集都会这么说明,而不是以一个连接错误失败。它默认是关的,除非你打开 - 更早的 Eidos
写出的设置文件里没有这个键,而把缺失的键读成"开",会把每一个升级上来的人的网络都掐断。

**Preferred servers** 给一次下载优先选用的 CDN 节点排序,最好的在前。只有 premium
账号才会拿到不止一个镜像可选,所以对其他所有人来说是 Nexus 挑,这项设置什么也不改变。
它是一个排序,不是一个过滤器:如果你点名的节点今天一个都没提供,下载照样进行,用
Nexus 最先给出的那个节点。

**Categories** 是可编辑的,不只是拿来显示:把它们指派给一个模组或一整批选中的模组,
在同一个对话框里编辑目录本身,并从 Nexus 拉取这款游戏的官方分类列表。两个目录文件都是
MO2 自己的(`categories.dat` 和 `nexuscatmap.dat`),所以共享的实例保有同一份目录。

**View -> INI editor** 编辑配置档的游戏 INI - 是会持久保留的那一份,而不是埋在 Proton
前缀里、每次启动都被覆盖的那一份。**View -> Log** 读会话日志。**View -> Extensions**
列出你自己的附加组件;见 [extensions.zh-CN.md](extensions.zh-CN.md)。

安装什么都接受:Simple 与 FOMOD 两条路,加上 Wrye Bash 的 **BAIN** 包(勾选子包,它们
按顺序合并),以及一个**手动**选择器 - 当没有启发式认得出布局时,它展示压缩包的目录树,
让你指出数据根目录在哪。没有压缩包会被拒绝。

**Diagnostics** 跑实时健康检查:首先是启动能力,缺失的 master(单一最可靠的崩溃预测
指标),没有任何活动插件会加载的档案,模组列表是否仍与 mods 文件夹一致,以及 - 在一次
运行之后 - 脚本扩展器自己的日志对它每一个插件 DLL 说了什么,这把"我的 SKSE 插件加载了
吗?"从一次推断变成了证据。

要通过 GUI 启动游戏,把该游戏的 Steam 启动选项设成这个二进制的绝对路径(Steam 看不到
PATH 上的 `~/.cargo/bin`):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos 会在这款游戏的实例上打开 - 你上次用的那一个,所以便携实例和全局实例一样能被重新
找到;点 Run 让它穿过合并视图启动。(如果你在 Steam 之外按下 Run 按钮,它会显示这一行
的确切内容,并带上正在运行的二进制的真实路径。)

Bethesda 各作在 Steam 里的 `%command%` 通常指向 `<Game>Launcher.exe`。Eidos 从不运行
它:那个 launcher 是一个独立的设置程序,它会重新扫描 `Data` 并重写 `plugins.txt`,把
刚刚部署好的加载顺序毁掉。它会换上脚本扩展器的 loader(如果装了),否则换上游戏本体的
二进制,并在不得不回退时说明 - 一个每个 SKSE 模组都不生效的游戏,比一个起不来的游戏
更糟。

这里更早的说明曾强制 `WINEDLLOVERRIDES="d3dcompiler_47=n"`。现在不再需要,而且那从来
就不太对:覆盖为 *native* 只在前缀里已经有一个真正的 `d3dcompiler_47.dll` 时才有用。
Eidos 现在会扫描已启用模组的 DLL 导入,自己部署那个真正的微软 DLL,然后才设置这个覆盖。

## 试试概念验证

不需要游戏。它只用 user 命名空间里的非特权 OverlayFS(Linux >= 5.11)就证明了 union +
copy-on-write + 零改动 + 按命名空间隔离:

```sh
./scripts/poc-overlay.sh
```

## 工具

xEdit、BodySlide、DynDOLOD 之类的工具,在游戏的 Proton 前缀里穿过合并视图运行:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # 已注册的工具需要什么,以及它的状态
eidos prereqs skyrimse --install  # 把缺的都取回来
```

给工具命名之前有一件事要知道:**标题决定了 Eidos 为它准备哪些运行时 DLL** -
`BodySlide` 会拿到它的 DirectX 库,`BS` 什么也拿不到。在 GUI 里,Executables 对话框会
在输入框下方显示每项前置条件的真实状态,缺的那些是按钮。

那张表、三个前置条件层级、DynDOLOD 为什么需要一个 winetricks 装不了的 .NET 运行时,
以及作为模组安装的工具为什么是从合并路径而不是它自己的文件夹启动的,都在
[tools.zh-CN.md](tools.zh-CN.md) 里。

从源码构建以及仓库布局在
[../internals/contributing.md](../internals/contributing.md)。

## 扩展

Eidos 不必重新构建就能扩展:`~/.config/Colony/Eidos/addons/` 里的一份 TOML 清单,就能往
Extensions 列表加一个工具,或往 Health 标签页加一项检查。没有任何东西被加载进 Eidos -
一个扩展是它运行的一个程序。见 [extensions.zh-CN.md](extensions.zh-CN.md)。
