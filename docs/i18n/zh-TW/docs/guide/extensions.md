<!-- eidos-i18n: source=docs/guide/extensions.md sha=4504002e1349638a04b741f27e8e360c22ad9831 -->

# 擴充

擴充替 Eidos 增加一個項目,卻不屬於 Eidos 的一部分。它是一份指明某個程式的 TOML
清單,外加——至多——那個程式本身。

清單放在 `~/.config/Colony/Eidos/addons/`,每個擴充一個 `.toml`。從
View -> Extensions -> Open folder 開啟該資料夾,然後按 Reload ——不必重啟。

`id`、`kind` 和 `exec` 為必填,`name` 預設取 id。`exec` 指明程式;不含 `/` 的名稱會在
`PATH` 中尋找。找不到程式時,Eidos 會略過該擴充,它在 View -> Extensions 裡的那一行會
說明原因(`<exec> is not on PATH` 或 `<exec> is not there`)。`games` 把擴充限定在列出
的遊戲 id;省略時擴充適用於所有遊戲,在別的遊戲上它那一行會寫「not for this game」。

Eidos 拒絕的清單會排在最前面並附上原因:讀不到的檔案、不是有效的 TOML 或缺少必填欄位、
`id` 為空或含空格、`exec` 為空、`id` 已被較早的清單佔用(檔案按名稱順序讀取)、Eidos
不認識的 `kind`,或是沒有 `protocol = 1` 和檔名 `extensions` 的安裝程式、預覽或存檔資訊
類型。

## 為什麼不往 Eidos 裡載入任何東西

Mod Organizer 2 把外掛當共享程式庫載入,並透過 Qt 承載 Python 外掛。兩者都無法照搬。
Rust 沒有穩定的 ABI,因此用另一個編譯器——或另一個最佳化旗標,或共享相依套件的另一
組特性——建置出的共享程式庫屬於未定義行為,而且沒有任何東西會把它報告為版本不符。而且 Eidos 的元件在編譯
期就是泛型的,所以即使 ABI 穩定,程式庫也造不出一個能交回去的元件。

擴充是 Eidos 以你帳號所擁有的檔案系統存取權限執行的程式。請只安裝你信任的輔助程式。程序隔離和回應檢查並不提供作業系統層級的沙箱。

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

它出現在 View -> Extensions 裡並帶一個 Run 按鈕,以分離方式啟動——Eidos 不會等它,狀態列
會給出它的 pid。它的輸出會被丟棄。沒有 `workdir` 時,它在 Eidos 本身啟動時所在的資料夾
裡啟動。

## 一項檢查

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

它在每次重新整理時執行,除非 `workdir` 另有指定,否則在實例根目錄中執行,每行輸出一條
結論:

```
level<TAB>title<TAB>detail
```

其中 `level` 為 `problem`、`advice` 或 `ok`。detail 為選用;沒有 title 的行會被丟棄。
凡是不以已知等級開頭的內容一律忽略,因此進度輸出與零散的警告無法冒出一行看起來像
Eidos 自家檢查的紀錄。結論顯示在 Diagnostics 分頁,格式為 `<name> - <title>`,排在
前綴與 Nexus 卡片之後、其他 Health 卡片之前。

一項檢查有三秒鐘。超時者會被中止,並作為針對它自己的問題回報——它執行在每次點擊之後
的同一次重新整理裡,所以一個卡住的檢查會凍結視窗。計時一直持續到檢查結束且它的輸出管道關閉為止,所以仍握著
管道的背景子行程也會算進這三秒。檢查以錯誤結束且沒有輸出時,會以同樣方式回報,並附上
錯誤輸出的第一行;任一資料流輸出超過 1 MiB 也一樣。無論哪種情況,在你按 Reload 之前
Eidos 都不會再執行它,它那一行會寫「It will not be retried until you press Reload.」。
檢查的標準輸入是空的。

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
| `{install}`     | 實例所執行的遊戲資料夾                       |
| `{data}`        | 該資料夾的 `Data` 目錄                       |

`{install}` 是商店安裝,或實例執行遊戲副本時的那份副本(凍結的 `Stock Game` 或清單自帶的
遊戲資料夾;見 [usage.md](usage.md#遊戲檔案商店安裝還是副本))。`{data}` 隨之而變。

檢查另外還有 `{addon_dir}`,即存放其清單的資料夾,下面的協定 1 擴充也有。工具沒有。

未知的佔位符絕不會被清空,否則 `--out {typo}` 會變成 `--out --next-flag`。若某個工具或
檢查的佔位符不能全部解析,Eidos 會拒絕執行,並指出缺了哪些:工具顯示在狀態列,檢查顯示
在 Diagnostics 分頁它自己那一行。

## 擴充不能做什麼

擴充不能回呼 Eidos、從壓縮檔中自行註冊，也不能繪製自己的介面元件。使用協定 1 的結構化安裝程式可以傳回經過檢查的檔案計畫和選擇提示；審核、暫存及發布由 Eidos 管理。FOMOD 和 BAIN 仍是原生安裝程式。預覽和存檔資訊擴充也使用同一個對回應設有限制的宿主。有關比對優先順序、已記錄的 CLI 回答、限制和重播身分檢查，請參閱[協定規範與可執行範例](../../../../guide/examples/extensions/README.md)。
