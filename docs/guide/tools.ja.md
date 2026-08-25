<!-- eidos-i18n: source=docs/guide/tools.md sha=b24d131068de5d901d82e279d67d64cf50106ab4 -->

# ツール: xEdit, BodySlide, DynDOLOD, FNIS

Eidos 越しに実行したツールは、ゲーム自身の Proton プレフィックスの中で
**統合ビュー**を見ます。ゲームが読むものをそのまま読み - 有効な MOD すべてを、
優先順位どおりに - 書き出したものは Overwrite に着地し、そこでひと押しすれば
本物の MOD になります。

## Eidos が自分で見つけるもの

名前が十分に一意なツールは、宣言しなくても見つけられます。xEdit がその典型で、
Fallout 4 なら `FO4Edit.exe`、Skyrim SE なら `SSEEdit.exe`、初代なら
`TES5Edit.exe`、といった具合です - それぞれの **QuickAutoClean** の片割れも
一緒に。LOOT が警告し続ける dirty edit のためのボタンがこれです。Eidos は
ファイル名で、次の場所を探します:

- ゲームのインストールフォルダと、有効な MOD の `Root/` ツリー;
- **このインスタンスの `mods/`**。MO2 の利用者がツールを入れる場所です;
- 設定で指定した**ツールフォルダ**(Tools -> Tools folder)。インスタンス間で
  共有するディレクトリ - `/mnt/Games/Tools` のようなもののためです。

一覧はゲームごとなので、Skyrim のインスタンスに Fallout のエディタが出ることは
ありません。探索は四階層で止まります。MOD プールは数十万ファイルあり、これは
ツール一覧を組み立てるたびに走るからです。シンボリックリンクも辿りません。
こうして見つかったツールは、手で入力したものとまったく同じように設定されます。
ランタイムは名前から決まり、規則は以下すべてと同じです。

ツールが別の場所にある場合や、違う引数を渡したい場合は、手で追加してください -
同じタイトルのユーザー項目は、自動で見つかったものを上書きします。

## 追加する

GUI なら **Tools -> Executables** を開いて Add。コマンドラインなら:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
```

スクリプトエクステンダ、ゲーム本体、ランチャーは自動で検出されます。登録が
要るのは追加のツールだけです。

### どこにあろうと、実体のファイルを指す

実行ファイルは、実際に置かれている場所で登録します。ツールを MOD として
入れたなら、それは MOD フォルダの中です:

```
~/.local/share/eidos/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(これはグローバルインスタンスのパスです - ポータブルインスタンスでも同じ規則が
自身のフォルダの下 `<instance>/mods/...` に当てはまります。なお、こうした絶対
パスは、あとでポータブルフォルダを移動したときに唯一生き残らないものです)。

Eidos は起動前にそのパスを統合ビューのものへ書き換えます。ツールは
`<game>/Data/CalienteTools/BodySlide/` から動き、そこにある他の MOD の
ファイルもすべて見えます。これは聞こえる以上に効きます。BodySlide が同梱する
`SliderSets` ディレクトリは**空**で、作れる体型はすべて CBBE と衣装 MOD から
来ます。自分の MOD フォルダから起動すると何も見つからず、壊れているように
見えます。

MO2 も同じ理由で同じ書き換えをしています - そのコメントは FNIS を名指しして
います。

**無効な** MOD の中のツールは書き換えられません。そのファイルもビューに
無いからです。Eidos はそう伝えたうえで、取り繕わずに元のフォルダから実行します。

## ツールの出力を専用の MOD へ送る

ジェネレータ - FNIS、Nemesis、BodySlide、DynDOLOD、Synthesis - は何百もの
ファイルを書きます。既定では他のすべてと一緒に Overwrite に着地します。
Executables のエディタで **Capture output into** を設定すると、その実行の
出力は代わりにその MOD へ入ります:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

MOD が無ければ作られます。動くのはこの実行が生んだファイルだけで、すでに
Overwrite にあったものはそのまま残ります。だから取り込み先を持つ二つのツールが
互いの出力を奪い合うことはありません。何も書かなかった実行は、空の MOD を
残しません。

これは書き込みレイヤーを MOD に向けるのではなく、実行の後で行います。MO2 は
前者のやり方です。書き込みレイヤーを MOD に向けると、その MOD は実行のあいだ
最優先へ繰り上がり - 関わる競合をすべてひっくり返し、終わったらまた戻すことに
なり - MOD 自身のファイルにコピーアップ無しで直接書き込みます。取り込みは
そのどちらも無しに同じ最終状態へ辿り着きます。

対象の MOD が無効なら、出力は書かれますがゲームからは見えず、ツールは次の実行で
同じファイルを作り直すことになります。Eidos はその場合に警告します。

## ツールに要る DLL は、その名前で決まる

ここは意外なので、はっきり書きます。**ツールに付けたタイトルが、Eidos が
用意するランタイムの前提条件を決めます。** 照合はタイトルに対する大文字小文字を
無視した部分一致です。

| タイトルに含まれる文字列 | Eidos が要求するもの |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnet8`, `vcrun2022` |
| `pandora` | `dotnetdesktop8` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| それ以外 | なし |

つまり **`BodySlide`** として登録したツールは DirectX の DLL を受け取り、同じ
実行ファイルを **`BS`** として登録すると何も受け取らず、DLL について何も言わない
エラーで起動に失敗しうるということです。ツールにはプログラムの名前を
付けてください。

一覧は `default_prereqs`(`crates/eidos-instance/src/tools.rs`)にあり、
Executables ダイアログの `Prereqs` 欄は編集できます - この検出は既定であって、
規則ではありません。

### 前提条件は三種類

**Tier 1 - 同梱の DLL**(`d3dx9_43`、`d3dcompiler_47`、`d3dx11_43`)。Eidos が
同梱しており、起動時にプレフィックスへコピーします。やることは無く、ネット
ワークも要りません。

**Tier 2 - winetricks の verb**(`vcrun2022`、`dotnet8`、`dotnetdesktop8`、
`dotnet48`、`xact` など)。これらはレジストリキー、GAC、CLR ホストを書くので、
ファイルのコピーでは済みません。**Microsoft からダウンロード**します。

**Tier 3 - ランタイム**(`dotnet10`)。現代の .NET ランタイムは 193 個の
ファイルで、専用のディレクトリに置かれ、`DOTNET_ROOT` を通して見つけられます。
登録もされず、プレフィックスにインストールもされないので、他のどちらの Tier でも
運べません。Eidos は自分でダウンロードし、バイナリに埋め込んだチェックサムで
検証して、`~/.local/share/Colony/Eidos/runtimes/` にキャッシュします -
**どのインスタンスの外側**でもあります。78 MB はゲームごとでもプロファイル
ごとでもないからです。

Tier 2 と Tier 3 は、どちらも黙って動くことがありません:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

GUI では同じ状態が Prereqs 欄の下に並び、欠けているものはボタンになっています。
同梱でもランタイムでも既知の winetricks の verb でもないものは、ダウンロードと
して提示されるのではなく、綴り間違いの可能性として報告されます。

### DynDOLOD に `dotnet10` が要る理由

DynDOLOD 自身はオブジェクト LOD を作りません。LODGen を呼び出すだけで、三つ
同梱しています。`LODGenx64.exe` は .NET Framework 4.8 向けで、Proton の下では
Wine の Mono へ回されます - その `System.Uri` の初期化子は、Mono が実装して
いないメソッドを呼びます。仕事の一行目に入る前に死に、ログにはバージョンの
表示だけが残り、DynDOLOD のダイアログは「failed for one or more worlds」としか
言いません。

本物の .NET Framework を入れても直りません。Proton は `mscoree.dll` - それを
見つけるはずのローダー - を自身のツリーへのシンボリックリンクで置き換え、
プレフィックスの更新のたびにやり直すからです。

動くビルドは `LODGenx64Win10.exe` で、現代の .NET 向けであり `mscoree` に
触れません。`DOTNET_ROOT` を .NET 10 のランタイムへ向ければ動きます。
`dotnet10` が用意するのはそれで、Eidos はそれを宣言したツールを起動するときに
この変数を設定します。

Eidos はシステムの `winetricks` を、Proton 自身の `wine` とゲームのプレフィックス
に対して実行します。これで Steam の pressure-vessel コンテナと、
protontricks + Proton-GE の食い違いを回避します。未インストールの Tier 2 の
verb を宣言したツールも起動はします。その verb と、直すためのコマンドを挙げた
警告付きで - 別の経路ですでに入っていることもあるからです。

## プレフィックスの中のゲームパス

Windows のツールは `HKLM\Software\Bethesda Softworks\<game>` の
`installed path` を読んでゲームを見つけます。ゲーム自身のインストーラが書く
キーで - Proton 下の Steam はそのインストーラを一度も走らせません。これが
無いと xEdit も Wrye Bash も DynDOLOD も空のパスで開きます。Eidos はツールを
実行する前にこれを書きます。冪等で、追加のみ、プレフィックスが未初期化か
使用中ならスキップします。

## ツールに手を伸ばす: 隠す、固定する、デスクトップショートカット

ゲームの既定には一度も使わないツールも含まれます。二番目に辿り着くのに八項目を
並べるピッカーは、誰も読まないピッカーです。Executables ダイアログでは:

- **Pin to top** は項目を Run 一覧の先頭に置きます。
- **Hide from picker** は削除せずに一覧から外します。
- **Desktop shortcut** は `~/.local/share/applications` に `.desktop` を
  書きます - freedesktop なシステムでランチャーが属する場所なので、デスク
  トップではなくアプリケーションメニューと検索に出てきます。実行するのは
  `eidos tool <instance> run <title>` そのもので、つまり Eidos のウィンドウを
  まったく開かずに、ツールは**このインスタンスのプロファイルで統合ビュー越しに**
  立ち上がります。

隠すことと固定することは、何を実行するかではなくツールへの*辿り着き方*の話
なので、自分で作った項目だけでなくゲームごとの既定にも効きます。

## それ自体が Steam のアプリであるツール

Creation Kit は別個の Steam アプリケーションで、自分の AppID を欲しがります。
Steam で配られている他のいくつかの modding ツールも同じです。項目に
**Steam AppID** を設定すると、Eidos はゲームのものではなくその id で起動します。

Windows ではこれは別のランチャーを意味します。ここでは、すでに組み立て中だった
実行に環境変数を二つ足すだけです - `SteamAppId` と `SteamGameId` の両方。
Proton は一方を、Steam 自身のライブラリはもう一方を読み、食い違いを見たツールは
明快にではなく妙な形で失敗するからです。`eidos tool ... --print` は、本当の
実行が受け取るものをそのまま見せます。

## ツール自身の設定は、やはりツール自身のもの

Eidos はツールを正しい場所に、正しい DLL とともに置きます。そのうえでツールが
自分の設定をどう扱うかは、あなたとツールのあいだの話で、失敗はたいてい黙って
起きます。

実例をひとつ。知らないと一時間を失うからです。BodySlide の
**Game Data Path**(Settings)は、その上のゲームフォルダではなく、ゲームの
`Data` ディレクトリを指していなければなりません。一階層高く設定すると、
バッチビルドは「All sets processed successfully」と報告し、ゲームが決して
探さない場所に 1439 個のメッシュを書きます。Eidos はそれを受け止め -
インストール先ではなく `Overwrite/Root/` に着地します - しかしゲームから
見れば、体型が作られていないこと以外に何もおかしくありません。

ツールの出力は Overwrite に属します。実行が取っておく価値のあるものを生んだら、
**Overwrite -> Create mod...** でそれを普通の MOD に変えられます。他と同じ
ように並べ替え、無効化し、削除できます。
