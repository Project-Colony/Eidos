<!-- eidos-i18n: source=docs/guide/fallout4.md sha=bfb499606267946215c4c39a39ae45aaa2cf0a63 -->

# Eidos 越しの Fallout 4

Eidos を使えば、Fallout 4 に特別な起動オプションは要りません。実行ファイルの改名も、
ラッパースクリプトも不要です。ほかのゲームと同じ起動オプションを使います。

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
ゲーム更新のたびに Steam が元へ戻します。そのあとは F4SE なしで遊ぶことに
なり、何もそれを教えてくれません。

Eidos はゲーム記述子に従って自分で入れ替えます。ウィンドウがゲームを起動するとき、
ローダーがゲームフォルダ、有効な MOD の `Root/`、または `Overwrite/Root/` にあれば、
`Fallout4Launcher.exe` を `f4se_loader.exe` に置き換えます。ローダーがなければ
`Fallout4.exe` を実行し、ステータス行に
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
と出ます。どちらのファイルも見つからないときは、ランチャーをそのまま実行し、その
旨も表示します。代わりに起動オプションを `eidos play fallout4 -- %command%` にした
場合、CLI はローダーがあるときだけ入れ替え、なければ `Fallout4Launcher.exe` を
実行します。

ランチャーを避けるもう一つの理由があります。ランチャーは `Data` を再走査して
`plugins.txt` を書き換え、いま配置したばかりのロード順を台無しにします。

ウィンドウを自分で開いて Run を押すと、Eidos は Steam にゲームの起動を頼み
(`steam -applaunch 377160`)、Steam が上の起動オプションを通して Eidos を起動します。
インスタンス内でゲームの実行ファイルの隣に `f4se_loader.exe` がなければ起動せず、
ステータス行が先にスクリプトエクステンダを入れるよう告げます。Fallout 4 VR も
同じ仕組みで、`Fallout4VRLauncher.exe` と `f4sevr_loader.exe` を使います。

## Eidos が肩代わりすること

| 項目 | Eidos がすること |
|---|---|
| アーカイブ無効化 | `Fallout4Custom.ini` に `[Archive]` `bInvalidateOlderFiles=1` と空の `sResourceDataDirsFinal=` が入ります。`Data` の外にあるルーズファイルがそもそも見えるようになる二つのキーです。ゲームフォルダではなくプロファイルに書かれます。 |
| INI | `Fallout4.ini`、`Fallout4Prefs.ini`、`Fallout4Custom.ini` はプロファイルに属します。Eidos 経由で動かした BethINI Pie はプロファイル側のコピーを編集し、Backups -> INIs は Eidos が実行前ごとに残すコピーを元に戻します([tools.md](tools.md#bethini-pie))。 |
| ロード順 | Fallout 4 が使うアスタリスク形式の `plugins.txt`(`*` が有効を示す)。暗黙の Creation Club プラグインについては `Fallout4.ccc` を尊重します |
| LOOT | 並べ替えは Skyrim と同じ - `eidos sort <instance>` が `fallout4` のマスターリストを取得します |
| セーブ | `.fos` セーブとその `.f4se` コセーブを一覧・複製し、プロファイルごとに保持します。詳細ペインはセーブ自身のプラグイン表を読むので、無効化したプラグインを必要とするセーブは読み込む前にそう告げます。プロファイルの新しい方から 3 つのセーブのどれかが使うプラグインを持つ MOD を無効化または削除するときは、先に確認が出ます。 |
| Root MOD | MOD が実行ファイルの隣に置くもの(F4SE 本体、ENB、`dxvk.conf`)は、Skyrim と同じ `Root/` の仕組みでそこへ落ちます |

## バージョンの話

Fallout 4 は 2019 年から 2024 年のような凍結されたゲームではもうありません。2026 年
8 月時点で生きている系統は三つあり、片方向けにビルドされた MOD の DLL は別の系統では
読み込まれません:

| 系統 | バージョン | F4SE |
|---|---|---|
| クラシック(old-gen) | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 から 1.11.240 | 0.7.4 から 0.7.9 |

MOD 構成を組む前に知っておくことが二つあります。

- ゲームのルートに `Creations/` と `Mods/` があれば 1.11.x 系です。Eidos のセーブ
  詳細ペインは、それを書いたビルドも表示します。Fallout がセーブに書き込み、Eidos が
  「Game build」として見せます。
- F4SE はたいてい Bethesda の更新から一日か二日で出ますが、*Address Library for F4SE
  Plugins*(ほとんどの DLL MOD がオフセットを解決する先)は独自の予定で追従します。
  その間、DLL MOD は読み込まれません。DLL を含まない MOD(テクスチャ、メッシュ、
  プラグイン)は影響を受けないので、出たばかりのパッチの日は MOD 構成を始めるのに
  向きません。

構成が動いたら、ゲームのバージョンをインスタンスに凍結します。Diagnostics タブの
Game files カードの Freeze this version を押すか、`eidos stock-game fallout4 freeze`
でサイズを確認し、`eidos stock-game fallout4 freeze --yes` でコピーします。Eidos は
ストアのインストールを `<instance>/Stock Game` にコピーし、そのコピーからゲームを
動かします。起動は引き続き Steam が行います。後のパッチが変えるのはストア側で
コピーではないので、入れた DLL はそのゲームバージョンのまま使えます。btrfs や XFS
ではコピーがストアとブロックを共有し、Steam がファイルを変えるまではほとんど容量を
使いません。ext4 ではゲームフォルダと同じ大きさの完全なコピーになります。Steam が
ゲームを使用中、または完全にはインストールされていないと報告している間は、Eidos は
拒否します。詳細は [usage.md](usage.md#ゲームファイル-ストアのインストールかコピーか) に。

コピーがない場合は、Fallout 4 の Steam 自動更新を切ってください(プロパティ、更新、
「起動時にのみこのゲームを更新」)。さもないと次のパッチが、入れた DLL を残らず
壊します。

## ハードウェアの注意: NVIDIA での武器デブリのクラッシュ

Fallout 4 の武器デブリ効果は NVIDIA FleX 上で動きます。これは NVIDIA が Pascal 世代
以降サポートをやめた PhysX の派生物です。Turing 以降のカード - GTX 16、RTX 20 から
RTX 50 まで - ではゲームが落ちます。これはゲーム側の不具合で、Linux にも Proton にも
Eidos にも関係ありません。

対処は二つ、どちらでも効きます。ゲーム設定で「Weapon Debris」を切るか、
*Weapon Debris Crash Fix*(Nexus 48078)を入れるか。後者は効果ではなく破片の当たり
判定を無効にします。

## Fallout 4 の Health カード

Diagnostics タブには、次の項目ごとに Health カードが出ます。

- F4SE プラグインが `F4SE/Plugins` に入っているのに、`f4se_loader.exe` が有効な
  どの MOD にもゲームフォルダにもない。ゲームはそれらなしで起動します。
- `f4se_loader.exe` はあるが、動いているゲームバージョン用の
  `f4se_<version>.dll`(1.10.984 なら `f4se_1_10_984.dll`)がない。ローダーは
  エラーで止まります。ゲーム更新のあとによく起きます。
- Eidos 経由で最後にエラーなく終了した起動のあと、ゲームの実行ファイルまたはその
  Steam ビルドが変わった。旧バージョン向けにビルドされた F4SE プラグインは読み込まれ
  なくなることがあります。
- F4SE より優先度の高い MOD が、F4SE 自身のスクリプトのルーズな `.pex` を持って
  いる。古いコピーは、それが宣言する F4SE の関数を壊し、ゲームはエラーを出しません。
- プロファイルの `Fallout4Custom.ini` の `[General]` に `sTestFile1` から
  `sTestFile10` の行がある。どれかが設定されている間、ゲームはそれらのプラグインを
  読み込み、`plugins.txt` を無視します。Fix はそれらを削除し、元のファイルを
  `Fallout4Custom.ini.pre-fix` として残します。
- MOD の BA2 アーカイブが `.wav` や `.xwm` の音声を圧縮して保持している。その音を
  再生するとゲームが落ちることがあります。音声を非圧縮にしてアーカイブを作り直して
  ください。Fallout 4 VR にも同じカードが出ます。
- クラッシュロガーが直前の起動でログを書いた。Buffout 4 の
  `Documents/My Games/Fallout4/F4SE` 内の `crash-*.log`、または `F4SE/Crashlogs` 内の
  `.log` です。カードは、ログにある例外と、MOD が入れた DLL およびプラグインを
  挙げます。
- Display カードは、プロファイルの `Fallout4Prefs.ini` に設定されたウィンドウの
  サイズとモードを示し、Apply は `iSize W`、`iSize H`、`bFull Screen`、
  `bBorderless` をそこに書きます。
- Nexus の要件が DLC を指している。Eidos は DLC と同じ名前のプラグインを探します
  が、Fallout 4 の DLC プラグインは別の名前です(Far Harbor なら `DLCCoast.esm`)。
  このカードは助言なので、手で確認してから Ignore を押してください。

二つのローダーカードには、<https://f4se.silverlock.org/> を開く Open the download
page ボタンと、ローダーを MOD の `Root/` に入れる Install the downloaded archive
ボタンがあります。`eidos check fallout4` は同じ所見を出力しますが、`.pex` カード、
Display カード、Nexus 要件カードはウィンドウだけが計算します。

## 何かおかしいと感じたら

一般的な確認手順は [troubleshooting.md](troubleshooting.md) に。Fallout 特有の
最初の問いは、いつでも*実際にどの実行ファイルが起動したか*です。ウィンドウが
ゲームを起動するとき、完全な起動コマンドがインスタンスの実行ログに書かれるので:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

`f4se_loader.exe` と出ていれば入れ替えは起きています。`Fallout4.exe` と出て
いれば、F4SE は Eidos が見つけられる場所に入っていません。置き場所はゲームの
実行ファイルの隣で、MOD 管理下の構成ならそれは何かの MOD の `Root/` ディレクトリ
(あるいは手作業で入れたゲームフォルダそのもの)です。`Fallout4Launcher.exe` と
出ていれば、Eidos はルートビューでローダーも `Fallout4.exe` も見つけていません。

正しい実行ファイルが起動してもゲームが落ちる場合は、Test run と Find the mod behind
a problem で、プロファイルのセーブに触れずに原因を一つの MOD まで絞れます
([usage.md](usage.md#テスト実行))。
