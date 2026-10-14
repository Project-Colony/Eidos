<!-- eidos-i18n: source=docs/guide/extensions.md sha=5f31f1cbc3dcbfb9655f3570f01e5ada1377268d -->

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

擴充不能回呼 Eidos、從壓縮檔中自行註冊，也不能繪製自己的介面元件。使用協定 1 的結構化安裝程式可以傳回經過檢查的檔案計畫和選擇提示；審核、暫存及發布由 Eidos 管理。FOMOD 和 BAIN 仍是原生安裝程式。預覽和存檔資訊擴充也使用同一個對回應設有限制的宿主。有關比對優先順序、已記錄的 CLI 回答、限制和重播身分檢查，請參閱[協定規範與可執行範例](../../../../examples/extensions/README.md)。
