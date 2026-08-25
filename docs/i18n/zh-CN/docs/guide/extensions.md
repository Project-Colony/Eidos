<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# 扩展

扩展给 Eidos 增加一个条目,却不属于 Eidos 的一部分。它是一份指明某个程序的 TOML
清单,外加——至多——那个程序本身。

清单放在 `~/.config/Colony/Eidos/addons/`,每个扩展一个 `.toml`。从
**View -> Extensions -> Open folder** 打开该文件夹,然后按 **Reload** ——不必重启。

## 为什么不往 Eidos 里加载任何东西

Mod Organizer 2 把插件作为共享库加载,并通过 Qt 承载 Python 插件。两者都无法照搬。
Rust 没有稳定的 ABI,因此用另一个编译器——或另一个优化开关,或共享依赖的另一套特性
集——构建出的共享库属于未定义行为,而不是版本不匹配。而且 Eidos 的控件在编译期就是
泛型的,所以即便 ABI 稳定,库也造不出一个能交回去的控件。

所以扩展是 Eidos *运行*的一个程序。它无法让窗口崩溃,无法损坏模组列表,并且在
Eidos 更新之后依然可用。

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

它出现在 **View -> Extensions** 里并带一个 Run 按钮,以分离方式启动——Eidos 不会等它。

## 一项检查

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

它在每次刷新时运行,每行输出一条结论:

```
level<TAB>title<TAB>detail
```

其中 `level` 为 `problem`、`advice` 或 `ok`。detail 可选。凡是不以已知级别开头的内容
一律忽略,因此进度输出和零散的警告无法冒出一行看起来像 Eidos 自家检查的记录。结论显示
在 **Health** 标签页,并以扩展名称作前缀。

一项检查有三秒钟。超时者会被终止,并作为针对它自己的问题上报——它运行在每次点击之后
的同一次刷新里,所以一个卡住的检查会冻结窗口。

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
| `{install}`     | 游戏的安装目录                               |
| `{data}`        | 游戏的 `Data` 目录                           |

未知的占位符会原样保留而不是被清空,这样写错就会显眼地失败,而不会把 `--out {typo}`
变成 `--out --next-flag`。若某个工具的占位符不能全部解析,运行会被拒绝,并由 Eidos
指出缺了哪些。

## 扩展不能做什么

它拿到值并运行;它不能回调 Eidos,不能改动模组列表,也不能在窗口里画任何东西。这是
刻意的。MO2 用插件去做、而且确实需要伸进内部的那些事——游戏支持、安装器、冲突引擎
——在这里是内建的而非外挂:游戏定义是 `~/.config/Colony/Eidos/games/` 下自己的一份
TOML,FOMOD 与 BAIN 安装器则是原生的。
