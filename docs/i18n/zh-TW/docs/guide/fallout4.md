<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# 透過 Eidos 玩 Fallout 4

Fallout 4 不需要任何特殊啟動選項、不需要改名的執行檔、也不需要包裝腳本。這話值得
說明白,因為其它所有 Linux 上的 F4SE 教學都說反了 —— 而它們的建議會在下一次 Steam
更新時崩掉。

## 啟動選項

```
~/.local/bin/eidos-gui %command%
```

Steam 為 Fallout 4 設定的啟動目標是 `Fallout4Launcher.exe`,從來不是 `Fallout4.exe`,
所以讓 script extender 跑起來,本質上是「怎麼讓 Steam 啟動另一個程式」這個問題。常見
答案是用 bash 改寫 `%command%`:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

或者把 `f4se_loader.exe` 覆蓋到 `Fallout4Launcher.exe` 上 —— 而 Steam 會在每次遊戲
更新時悄悄還原它,此後你就在沒有 F4SE 的情況下遊玩,卻沒有任何提示。

Eidos 自己完成這次替換,依據是遊戲描述符:裝了 `f4se_loader.exe` 就用它替換啟動器,
沒裝則退回 `Fallout4.exe`,而且在不得不退回時**會告訴你**。一個所有 F4SE 模組都失效
卻照常啟動的遊戲,比根本啟動不了更糟。

還有第二個絕不該執行啟動器的理由:它會重新掃描 `Data` 並重寫 `plugins.txt`,把剛剛
部署好的載入順序一筆勾銷。Eidos 從不執行它。

## Eidos 替你處理的事

| | |
|---|---|
| 封裝檔失效 | `Fallout4Custom.ini` 會被寫入 `[Archive]` `bInvalidateOlderFiles=1` 和空的 `sResourceDataDirsFinal=`,這兩個鍵讓 `Data` 之外的散裝檔案根本能被看見。寫進設定檔,而不是遊戲目錄。 |
| 載入順序 | `plugins.txt` 採用 Fallout 4 使用的星號格式(`*` 表示啟用),並遵循 `Fallout4.ccc` 處理隱含的 Creation Club 外掛 |
| LOOT | 排序方式與 Skyrim 相同 —— `eidos sort <instance>` 會取 `fallout4` 的 masterlist |
| 存檔 | `.fos` 存檔及其 `.f4se` 協同存檔會被列出、複製並按設定檔保存;詳情面板會讀取存檔自身的外掛表,所以一個需要你已停用外掛的存檔會在讀取前就說出來 |
| Root 模組 | 模組隨執行檔一同提供的東西(F4SE 本身、ENB、`dxvk.conf`)都透過 Skyrim 同樣的 `Root/` 機制落到那裡 |

## 版本問題

Fallout 4 不再是 2019 到 2024 年間那個凍結的遊戲。截至 2026 年 8 月,有三條活躍分支,
為其中一條建置的模組 DLL 無法在另一條上載入:

| 分支 | 版本 | F4SE |
|---|---|---|
| 經典版(old-gen) | 1.10.163 | 0.6.23 |
| 次世代版 | 1.10.984 | 0.7.2 |
| 週年版 / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

在搭建模組清單之前值得知道的兩個後果:

- **確認你手上到底是哪一個。** 遊戲根目錄裡有 `Creations/` 和 `Mods/` 資料夾,說明你
  在 1.11.x 這條線上。Eidos 裡存檔的詳情面板也會顯示寫下它的建置版本 —— Fallout 會把
  它寫進存檔,Eidos 以「Game build」呈現。
- **剛打完修補程式不是開工的好日子。** F4SE 通常在 Bethesda 更新後一兩天內跟上,但
  *Address Library for F4SE Plugins* —— 大多數 DLL 模組靠它解析偏移 —— 按自己的節奏
  走。在這兩者之間,生態裡 DLL 的那一半是癱的。不含 DLL 的模組(貼圖、模型、外掛)
  不受影響。

一旦你的整套設定跑通,就把 Fallout 4 的 Steam 自動更新關掉(內容 → 更新 →「僅在啟動
遊戲時更新」),否則下一個修補程式會把你裝的每一個 DLL 都打碎。

## 硬體提示:NVIDIA 上的武器碎片當機

Fallout 4 的武器碎片效果基於 NVIDIA FleX,那是 NVIDIA 在 Pascal 之後停止支援的 PhysX
衍生物。在任何 Turing 及更新的顯示卡上 —— GTX 16、RTX 20 直到 RTX 50 —— 它都會讓遊戲
當掉。這是遊戲本身的缺陷,與 Linux、Proton 或 Eidos 無關。

兩個辦法,任選其一:在遊戲設定裡關閉「Weapon Debris」,或安裝
*Weapon Debris Crash Fix*(Nexus 48078),它停用的是碎片的碰撞而不是效果本身。

## 如果哪裡不對勁

通用排查清單在 [troubleshooting.zh-TW.md](troubleshooting.md);而 Fallout 特有的
第一個問題永遠是*究竟啟動了哪個執行檔*。Eidos 會把完整的啟動命令寫進實例的執行紀錄,
所以:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

如果它寫的是 `f4se_loader.exe`,替換成功了。如果寫的是 `Fallout4Launcher.exe`,說明
F4SE 沒裝在 Eidos 找得到的地方 —— 它該待在遊戲執行檔旁邊,對於用模組管理器的設定
來說,那就是某個模組的 `Root/` 目錄(或者手動裝進遊戲目錄本身)。
