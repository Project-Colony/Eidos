<!-- eidos-i18n: source=docs/guide/extensions.md sha=9967c65927b3e805a0392071eec77ada3a8c5408 -->

# 擴充

擴充替 Eidos 增加一個項目,卻不屬於 Eidos 的一部分。它是一份指明某個程式的 TOML
清單,外加——至多——那個程式本身。

清單放在 `~/.config/Colony/Eidos/addons/`,每個擴充一個 `.toml`。從
**View -> Extensions -> Open folder** 開啟該資料夾,然後按 **Reload** ——不必重啟。

## 為什麼不往 Eidos 裡載入任何東西

Mod Organizer 2 把外掛當共享程式庫載入,並透過 Qt 承載 Python 外掛。兩者都無法照搬。
Rust 沒有穩定的 ABI,因此用另一個編譯器——或另一個最佳化旗標,或共享相依套件的另一
組特性——建置出的共享程式庫屬於未定義行為,而不是版本不符。而且 Eidos 的元件在編譯
期就是泛型的,所以即使 ABI 穩定,程式庫也造不出一個能交回去的元件。

所以擴充是 Eidos *執行*的一個程式。它無法讓視窗當掉,無法毀損模組清單,並且在
Eidos 更新之後依然可用。

## 一個工具

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # 省略則適用於所有遊戲
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

它出現在 **View -> Extensions** 裡並帶一個 Run 按鈕,以分離方式啟動——Eidos 不會等它。

## 一項檢查

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

它在每次重新整理時執行,每行輸出一條結論:

```
level<TAB>title<TAB>detail
```

其中 `level` 為 `problem`、`advice` 或 `ok`。detail 為選用。凡是不以已知等級開頭的
內容一律忽略,因此進度輸出與零散的警告無法冒出一行看起來像 Eidos 自家檢查的紀錄。
結論顯示在 **Health** 分頁,並以擴充名稱作前綴。

一項檢查有三秒鐘。超時者會被中止,並作為針對它自己的問題回報——它執行在每次點擊之後
的同一次重新整理裡,所以一個卡住的檢查會凍結視窗。

## 佔位符

`args` 與 `workdir` 都會展開這些:

| 佔位符          | 是什麼                                       |
| --------------- | -------------------------------------------- |
| `{instance}`    | 實例根目錄                                   |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | 目前設定檔的名稱                             |
| `{profile_dir}` | 目前設定檔的目錄                             |
| `{game}`        | 遊戲 id,例如 `skyrimse`                     |
| `{game_name}`   | 遊戲的顯示名稱                               |
| `{install}`     | 遊戲的安裝目錄                               |
| `{data}`        | 遊戲的 `Data` 目錄                           |

未知的佔位符會原樣保留而不是被清空,這樣寫錯就會顯眼地失敗,而不會把 `--out {typo}`
變成 `--out --next-flag`。若某個工具的佔位符不能全部解析,執行會被拒絕,並由 Eidos
指出缺了哪些。

## 擴充不能做什麼

它拿到值並執行;它不能回呼 Eidos,不能改動模組清單,也不能在視窗裡畫任何東西。這是
刻意的。MO2 用外掛去做、而且確實需要伸進內部的那些事——遊戲支援、安裝程式、衝突引擎
——在這裡是內建的而非外掛:遊戲定義是 `~/.config/Colony/Eidos/games/` 下自己的一份
TOML,FOMOD 與 BAIN 安裝程式則是原生的。
