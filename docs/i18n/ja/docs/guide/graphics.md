<!-- eidos-i18n: source=docs/guide/graphics.md sha=9a0f3b34319681bf27f11f455a3b1e87d7d44f13 -->

# Community Shaders、DLSS、フレーム生成

Community Shaders 1.4+ は独自のアップスケーリング(DLSS 4 / FSR 3.1 / XeSS。
別パッケージ「Upscaling - Community Shaders」経由)と FSR 3.1 のフレーム生成を
備えています。どれも Linux の Eidos 越しに動きます - CS もその追加パッケージも
普通の MOD として入り、統合ビューはそれらの DLL を他と同じように供給します -
けれども三つのことはゲームの中から**気づけません**。しかもそのどれもが、機能を
黙って何もさせなくします。このページはその一覧で、実機で痛い目に遭って得たものです。

## DLSS が必要とする起動オプション

```
PROTON_ENABLE_NVAPI=1 eidos-gui %command%
```

Proton は、ゲームが Valve の許可リストに載っていない限り NVIDIA NVAPI 層
(dxvk-nvapi)を無効にします。Skyrim は載っていません。これがないと CS は DLSS を
初期化できず、静かに FSR アップスケーリングへ落ちます。画面には理由が何も出ません。
NVIDIA でないマシンでこの変数を設定しても損はないので、無難な起動オプションは上の
一行で十分です。フレーム生成そのものは FSR 3.1 で NVAPI を必要としません。必要なのは
DLSS アップスケーラだけです。

## フレーム生成にはボーダーレスウィンドウが要る

CS のフレーム生成は D3D12 のプレゼンテーションプロキシ上で動き、排他的フルスクリーンを
きっぱり拒みます。`SkyrimPrefs.ini` の `bFull Screen=1` は、それが決して噛み合わない
ことを意味します - エラーもメッセージもなく、ただ基本フレームレートのままです。
確実な対処は SSE Display Tweaks で、INI が何と言おうとエンジンの層でモードを強制します:

```ini
[Render]
Fullscreen=false
Borderless=true
```

見た目のウィンドウは同じ(ネイティブ解像度のボーダーレス)。変わるのはエンジンの認識
だけで、そのエンジンの認識こそ CS が確かめているものです。

有効化の条件はあと二つ。失敗の仕方は同じく無言です:

- **リフレッシュレートが 120 Hz 以上**であること。あるいは CS のアップスケーリング
  設定で `frameGenerationForceEnable` を立てること。フレーム生成は提示レートを倍に
  するので、結果を表示できないディスプレイでは CS が起動を拒みます。
- **Upscaling パッケージが入っている**こと(その `Data/Shaders/Upscaling/` 以下に
  Streamline と FidelityFX の DLL があります)。これがないと CS はメニュー項目を
  出すだけで、何も有効化できません。

## Reflex のフレームレート上限が出力を絞め殺すことがある

CS の Reflex 設定は独自の FPS 上限(`reflexFPSLimit` と `reflexUseFPSLimit`)を
持ちます。昔の値のまま残った上限 - こちらでは古い調整の 79 でした - はフレーム生成の
下流に座り、生成されたフレームをちょうど刈り取ります。基本 60 が 120 に倍増され、
79 に戻されると「フレーム生成が効いていない」と読めるわけです。144 Hz のディスプレイ
なら通常の Reflex 上限は約 138。生成分が見当たらないと感じたら必ず確認してください。
排他的フルスクリーンに次ぐ二番目の無言の殺し屋です。

## 既知の相互作用: SSE Display Tweaks と黒画面

FG + Display Tweaks + DXVK の組み合わせには既知の黒画面不具合があります。順に:

1. `SSEDisplayTweaks.ini`: `DisableBufferResizing=true`
2. それで足りなければ、ゲーム実行ファイルの隣に `dxvk.conf`(MOD の `Root/`
   ディレクトリがそこへ置いてくれます)を用意し
   `dxvk.enableGraphicsPipelineLibrary = False`

## あとで数字をどう読むか

生成フレームは提示側だけの話です。エンジンは依然として基本レートでシミュレートし、
Havok も基本レートで刻み、*エンジンの*フレームを数えるもの(CS 自身のカウンタを
含む)は表示が ~120 でも ~60 を報告し続けます。これは正しい挙動で、壊れたカウンタでは
ありません - そして、エンジン自体のフレームレートを上げるのと違ってフレーム生成が
物理的に安全なのは、まさにこのためです。画面に数値が欲しければ起動オプションの
`DXVK_HUD=fps` が出してくれます。

規則は一つ。ドライバ側の補間(NVIDIA Smooth Motion、
`NVPRESENT_ENABLE_SMOOTH_MOTION=1`)と CS のフレーム生成は競合する技術です。
どちらか一方だけを使い、決して両方は使わないでください。
