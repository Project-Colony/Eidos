<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# 扩展

扩展给 Eidos 增加一个条目,却不属于 Eidos 的一部分。它是一份指明某个程序的 TOML
清单,最多再加上那个程序本身。

清单放在 `~/.config/Colony/Eidos/addons/`,每个扩展一个 `.toml`。从
View -> Extensions -> Open folder 打开该文件夹,然后按 Reload,不必重启。

`id`、`kind` 和 `exec` 必填,`name` 缺省时取 id。`exec` 指明程序,不含 `/` 的名字会在
`PATH` 中查找。程序不存在时,Eidos 会跳过该扩展,View -> Extensions 里它那一行会写明
原因(`<exec> is not on PATH` 或 `<exec> is not there`)。`games` 把扩展限定在列出的
游戏 id 上;不写则适用于所有游戏,在其他游戏中该行显示 "not for this game"。

Eidos 拒绝的清单会排在最前面并附上原因:文件读不了、不是合法的 TOML 或缺少必填字段、
`id` 为空或含空格、`exec` 为空、`id` 已被先读入的清单占用(文件按名称顺序读取)、
`kind` 是 Eidos 不认识的类型,或者安装器、预览、存档信息类扩展没有写
`protocol = 1` 和文件名 `extensions`。

## 为什么不往 Eidos 里加载任何东西

Mod Organizer 2 把插件作为共享库加载,并通过 Qt 承载 Python 插件。两者都无法照搬。
Rust 没有稳定的 ABI,因此用另一个编译器(或另一个优化开关,或共享依赖的另一套特性
集)构建出的共享库属于未定义行为,而且不会有任何地方把它报告为版本不匹配。而且 Eidos 的控件在编译期就是
泛型的,所以即便 ABI 稳定,库也造不出一个能交回去的控件。

扩展是 Eidos 以你的账户所拥有的文件系统访问权限运行的程序。请只安装你信任的辅助程序。进程隔离和响应检查并不提供操作系统级沙箱。

## 一个工具

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # 省略则适用于所有游戏
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

它出现在 View -> Extensions 里并带一个 Run 按钮,以分离方式启动。Eidos 不会等它,状态栏会
给出它的 pid。它的输出会被丢弃。没有 `workdir` 时,它在启动 Eidos 的那个文件夹里运行。

## 一项检查

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

它在每次刷新时运行,除非 `workdir` 另有指定,否则在实例根目录里运行,每行输出一条结论:

```
level<TAB>title<TAB>detail
```

其中 `level` 为 `problem`、`advice` 或 `ok`。detail 可选;没有 title 的行会被丢弃。凡是
不以已知级别开头的内容一律忽略,因此进度输出和零散的警告无法冒出一行看起来像 Eidos
自家检查的记录。结论在 Diagnostics 标签页里显示为 `<name> - <title>`,排在前缀和
Nexus 卡片之后、其余 Health 卡片之前。

一项检查有三秒钟。超时者会被终止,并作为针对它自己的问题上报。它运行在每次点击之后
的同一次刷新里,所以一个卡住的检查会冻结窗口。计时一直持续到检查退出且它的输出管道全部关闭,因此仍握着
管道的后台子进程也会计入这三秒。检查以错误退出且没有任何输出时,同样会被上报,并带上
它错误输出的第一行;任一输出流超过 1 MiB 也一样。无论哪种情况,在你按 Reload 之前
Eidos 都不会再运行它,它那一行会写 "It will not be retried until you press Reload."。
检查的标准输入是空的。

## 占位符

`args` 与 `workdir` 都会展开这些:

| 占位符          | 是什么                                       |
| --------------- | -------------------------------------------- |
| `{instance}`    | 实例根目录                                   |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | 当前配置档的名称                             |
| `{profile_dir}` | 当前配置档的目录                             |
| `{game}`        | 游戏 id,例如 `skyrimse`                     |
| `{game_name}`   | 游戏的显示名称                               |
| `{install}`     | 实例所运行的游戏文件夹                       |
| `{data}`        | 该文件夹的 `Data` 目录                       |

`{install}` 是商店安装目录;实例运行的是游戏副本时(冻结得到的 `Stock Game`,或列表
自带的游戏文件夹;见 [usage.md](usage.md#游戏文件商店安装或副本))则是
该副本。`{data}` 随之而变。

检查还能用 `{addon_dir}`,即清单所在的文件夹,下面的协议 1 扩展也能用。工具不能用。

未知的占位符不会被清空,否则 `--out {typo}` 会变成 `--out --next-flag`。若某个工具或
检查的占位符不能全部解析,Eidos 会拒绝运行,并指出缺了哪些:工具在状态栏里指出,检查在
Diagnostics 标签页它自己那一行里指出。

## 扩展不能做什么

扩展不能回调 Eidos、从压缩包中自行注册，也不能绘制自己的界面控件。使用协议 1 的结构化安装器可以返回经过检查的文件计划和选择提示；审核、暂存及发布由 Eidos 管理。FOMOD 和 BAIN 仍是原生安装器。预览和存档信息扩展也使用同一个对响应设有限制的宿主。有关匹配优先级、已记录的 CLI 回答、限制和重放身份检查，请参阅[协议规范与可运行示例](../../../../guide/examples/extensions/README.md)。
