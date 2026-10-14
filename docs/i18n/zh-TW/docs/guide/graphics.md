<!-- eidos-i18n: source=docs/guide/graphics.md sha=ee3cee732aafbf91d8085d03a74f7d4e0cfa224d -->

# Community Shaders、DLSS 與影格生成

Community Shaders 1.4+ 自帶升頻(DLSS 4 / FSR 3.1 / XeSS,透過單獨的
「Upscaling - Community Shaders」套件)以及 FSR 3.1 影格生成。這些在 Linux 上都能
穿過 Eidos 運作 —— CS 及其附加套件按一般模組安裝,合併檢視像對待其他檔案一樣提供
它們的 DLL —— 但有三件事在遊戲裡**看不出來**,而且每一件都會讓功能靜悄悄地什麼也
不做。本頁就是這份清單,是在真實環境裡吃過苦頭換來的。

## DLSS 需要的啟動選項

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

除非遊戲在 Valve 的允許清單上,否則 Proton 會關閉它的 NVIDIA NVAPI 層(dxvk-nvapi),
而 Skyrim 不在清單裡。沒有它,CS 無法初始化 DLSS,會悄悄退回 FSR 升頻,畫面上不會有
任何提示說明原因。在非 NVIDIA 機器上設定這個變數毫無代價,所以放心用的啟動選項就是
上面那一行。影格生成本身是 FSR 3.1,不需要 NVAPI;只有 DLSS 升頻器需要。

## 影格生成需要無邊框視窗

CS 的影格生成建立在 D3D12 呈現代理之上,並且乾脆拒絕獨佔全螢幕。`SkyrimPrefs.ini`
裡的 `bFull Screen=1` 意味著它永遠不會啟動 —— 沒有報錯,沒有提示,只有基礎影格率。
穩妥的辦法是 SSE Display Tweaks,它在引擎層面強制模式,不管 INI 怎麼寫:

```ini
[Render]
Fullscreen=false
Borderless=true
```

視窗看起來一模一樣(無邊框、原生解析度);變的只是引擎的認知 —— 而引擎的認知正是
CS 檢查的東西。

還有兩個啟用條件,失敗方式同樣安靜:

- **螢幕更新率 120 Hz 或更高**,或者在 CS 的升頻設定裡開啟
  `frameGenerationForceEnable`。影格生成會把呈現影格率翻倍,所以 CS 拒絕在顯示不出
  結果的螢幕上啟用它。
- **已安裝 Upscaling 套件**(它的 `Data/Shaders/Upscaling/` 目錄裡放著 Streamline 與
  FidelityFX 的 DLL)。沒有它,CS 會顯示選單項目卻什麼也開不了。

## Reflex 的影格率上限可能把輸出掐死

CS 的 Reflex 設定自帶 FPS 上限(`reflexFPSLimit`,搭配 `reflexUseFPSLimit`)。停留在
舊值的上限 —— 我們的是 79,來自很久以前的一次調校 —— 位於影格生成的下游,恰好把它
產出的影格砍掉:基礎 60 翻倍到 120,再被壓回 79,看上去就是「影格生成沒起作用」。
144 Hz 螢幕上常規的 Reflex 上限約為 138。只要覺得生成的畫面不見了就查它;這是繼獨佔
全螢幕之後的第二個無聲殺手。

## 已知互動:與 SSE Display Tweaks 一起出現黑畫面

FG + Display Tweaks + DXVK 這個組合有已知的黑畫面故障。按順序修:

1. `SSEDisplayTweaks.ini`:`DisableBufferResizing=true`
2. 若仍不行,在遊戲執行檔旁放一個 `dxvk.conf`(模組的 `Root/` 目錄就能放到那裡),
   內容為 `dxvk.enableGraphicsPipelineLibrary = False`

## 混用貼圖模組：PGPatcher

Community Shaders 能算出視差、complex material 與 PBR，但只在網格已為此設定好的
地方。過去這意味著先裝一堆預先打好補丁的網格，再裝一個模組，在貼圖缺失的地方把
效果重新關掉：覆蓋範圍受限於你手頭恰好有的網格，還要在執行時花 CPU 去撤銷一個
本就不該做出的決定。

[PGPatcher](https://www.nexusmods.com/skyrimspecialedition/mods/120946) 把它反了
過來。你隨意安裝任何著色器型別的貼圖，它會重寫網格與插件，讓每個表面都用上對的
那一種——包括插件裡的備用貼圖記錄，那是舊方法覆蓋不到的。這是「顯示得出來的貼圖
包」與「只是被載入的貼圖包」之間的差別，而且在混合配置下最為明顯——任何真實的
載入順序都是混合的。

在 Linux 上執行前有兩點：

- 它需要參數 **`--ignore-mo2vfscheck`**，否則會立刻結束並抱怨 MO2。Eidos 會提供
  ——這個旗標是什麼、檢查為何在這裡失敗，見[工具](tools.md#為什麼-pgpatcher-需要---ignore-mo2vfscheck)；
- 它會修改網格，所以要在所有貼圖模組裝完**之後**、在需要看到最終網格的 DynDOLOD
  **之前**執行。之後再改貼圖，就得按這個順序把兩者都重跑一遍。

它的輸出和其他工具一樣進入 Overwrite，一次點擊就能變成模組。給那個模組高優先
權：它本來就是要贏的。

## 事後怎麼讀這些數字

生成的影格只存在於呈現端:引擎仍以基礎影格率模擬,Havok 仍以基礎影格率步進,一切統計
*引擎*影格的東西(包括 CS 自己的計數器)都會繼續報 ~60,而螢幕顯示 ~120。這是正確
行為,不是計數器壞了 —— 也正因如此,影格生成對物理是安全的,而抬高引擎自身影格率則
不然。啟動選項裡的 `DXVK_HUD=fps` 可以在畫面上給你一個計數器。

一條規則:驅動層插影格(NVIDIA Smooth Motion,`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)
與 CS 的影格生成是互相競爭的技術。二選一,絕不要同時開。
