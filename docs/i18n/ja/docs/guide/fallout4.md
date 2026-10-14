<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Eidos 越しの Fallout 4

Fallout 4 に特別な起動オプションは要りません。実行ファイルの改名も、ラッパー
スクリプトも不要です。はっきり言っておく価値があります。というのも、Linux 向けの
F4SE 手引きはどれも逆のことを書いていて、その助言は次の Steam 更新で壊れるからです。

## 起動オプション

```
~/.local/bin/eidos-gui %command%
```

Steam が Fallout 4 で起動する対象は `Fallout4Launcher.exe` であって、決して
`Fallout4.exe` ではありません。つまりスクリプトエクステンダを動かすというのは、
実のところ「どうやって Steam に別のプログラムを起動させるか」という問いです。
よくある答えは `%command%` を bash で書き換えるものか:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

`f4se_loader.exe` を `Fallout4Launcher.exe` に上書きコピーするもので、後者は
ゲーム更新のたびに Steam が黙って元へ戻します。そのあとは F4SE なしで遊ぶことに
なり、何もそれを教えてくれません。

Eidos はゲーム記述子に従って自分で入れ替えます。`f4se_loader.exe` が入っていれば
ランチャをそれに置き換え、なければ `Fallout4.exe` に戻し、**戻したときにはそう
伝えます**。F4SE の MOD が全部死んだまま起動するゲームは、起動しないゲームより
たちが悪いからです。

ランチャを決して走らせないもう一つの理由があります。それは `Data` を再走査して
`plugins.txt` を書き換え、いま配置したばかりのロード順を台無しにします。Eidos は
一度も実行しません。

## Eidos が肩代わりすること

| | |
|---|---|
| アーカイブ無効化 | `Fallout4Custom.ini` に `[Archive]` `bInvalidateOlderFiles=1` と空の `sResourceDataDirsFinal=` が入ります。`Data` の外にあるルーズファイルがそもそも見えるようになる二つのキーです。ゲームフォルダではなくプロファイルに書かれます。 |
| ロード順 | Fallout 4 が使うアスタリスク形式の `plugins.txt`(`*` が有効を示す)。暗黙の Creation Club プラグインについては `Fallout4.ccc` を尊重します |
| LOOT | 並べ替えは Skyrim と同じ - `eidos sort <instance>` が `fallout4` のマスターリストを取得します |
| セーブ | `.fos` セーブとその `.f4se` コセーブを一覧・複製し、プロファイルごとに保持します。詳細ペインはセーブ自身のプラグイン表を読むので、無効化したプラグインを必要とするセーブは読み込む前にそう告げます |
| Root MOD | MOD が実行ファイルの隣に置くもの(F4SE 本体、ENB、`dxvk.conf`)は、Skyrim と同じ `Root/` の仕組みでそこへ落ちます |

## バージョンの話

Fallout 4 は 2019 年から 2024 年のような凍結されたゲームではもうありません。2026 年
8 月時点で生きている系統は三つあり、片方向けにビルドされた MOD の DLL は別の系統では
読み込まれません:

| 系統 | バージョン | F4SE |
|---|---|---|
| クラシック(old-gen) | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

MOD 構成を組む前に知っておく価値のある帰結が二つ:

- **実際に何を持っているか確かめること。** ゲームのルートに `Creations/` と
  `Mods/` があれば 1.11.x 系です。Eidos のセーブ詳細ペインは、それを書いたビルドも
  表示します - Fallout がセーブに書き込み、Eidos が「Game build」として見せます。
- **出たばかりのパッチは始めるのに良い日ではありません。** F4SE はたいてい Bethesda の
  更新から一日か二日で出ますが、*Address Library for F4SE Plugins* - ほとんどの DLL
  MOD がオフセットを解決する先 - は独自の予定で動きます。その間、エコシステムの DLL
  側は倒れています。DLL を含まない MOD(テクスチャ、メッシュ、プラグイン)は無傷です。

構成が動いたら、Fallout 4 の Steam 自動更新を切ってください(プロパティ → 更新 →
「起動時にのみこのゲームを更新」)。さもないと次のパッチが入れた DLL を残らず壊します。

## ハードウェアの注意: NVIDIA での武器デブリのクラッシュ

Fallout 4 の武器デブリ効果は NVIDIA FleX 上で動きます。これは NVIDIA が Pascal 世代
以降サポートをやめた PhysX の派生物です。Turing 以降のカード - GTX 16、RTX 20 から
RTX 50 まで - ではゲームが落ちます。これはゲーム側の不具合で、Linux にも Proton にも
Eidos にも関係ありません。

対処は二つ、どちらでも効きます。ゲーム設定で「Weapon Debris」を切るか、
*Weapon Debris Crash Fix*(Nexus 48078)を入れるか。後者は効果ではなく破片の当たり
判定を無効にします。

## 何かおかしいと感じたら

一般的な確認手順は [troubleshooting.ja.md](troubleshooting.md) に。Fallout 特有の
最初の問いは、いつでも*実際にどの実行ファイルが起動したか*です。Eidos は完全な起動
コマンドをインスタンスの実行ログに書くので:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

`f4se_loader.exe` と出ていれば入れ替えは起きています。`Fallout4Launcher.exe` と
出ていれば、F4SE は Eidos が見つけられる場所に入っていません。置き場所はゲームの
実行ファイルの隣で、MOD 管理下の構成ならそれは何かの MOD の `Root/` ディレクトリ
(あるいは手作業で入れたゲームフォルダそのもの)です。
