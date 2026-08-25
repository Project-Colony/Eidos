<!-- eidos-i18n: source=README.md sha=5d3404acdd61e5f220389c0eb702ff7511f58aa2 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**ゲームに一切触れない、ネイティブ Linux の MOD マネージャ。**

</div>

Eidos は、Windows で Mod Organizer 2 が Bethesda のゲームに与えるものを Linux で
与えます - 起動ごとに作られる、MOD の仮想的な統合ビューを。それを Windows API の
フックではなく Linux のプリミティブから組み立てています。マネージャに Wine は
要りません。ゲームのディレクトリにファイルはコピーされません。後片付けの手順も
ありません、片付けるものが何もないからです。

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **状況:** Skyrim SE は Eidos 越しに日常的に遊ばれています - SKSE、script
> extender のプリローダ、Creation Club、LOOT で並べ替えたロード順、プロファイル
> ごとのセーブ、その一式すべて。実際のプレイで実証済みのゲーム系列は今のところ
> 一つ。あと十系列が組み込み済みで、テスターを待っています。

## なぜ Eidos か

- 🔒 **あなたのゲームだけに見えるマウント。** 統合ビューはプライベートなマウント
  名前空間の中にあります。ファイルマネージャも、バックアップのジョブも、もう一本
  のゲームも - どれも見えませんし、そのための許可も要りません。ゲームを強制終了
  しても、電源を抜いても、名前空間はプロセスツリーとともに消え、インストール先は
  元のままです。残留物は*構造上*存在しません。
- 🧾 **正しい情報は一箇所だけ。** MOD の一覧、プラグインの順序、INI、セーブは
  プロファイルが持ちます。プラグインのファイルとセーブのディレクトリは起動時に
  ゲーム自身のパスへ bind-mount されるので、ゲーム自身の書き込みもプロファイルに
  着地します。プロファイルを切り替えれば全部が切り替わります。
- 🐧 **完全に rootless。** setuid のヘルパも、デーモンも、`sudo setcap` も、
  `/etc/fuse.conf` の編集も要りません。実行ファイル一つ、Steam の起動オプション
  一つ。
- 🛡️ **根拠を示すガード。** プラグイン一覧を壊すクラッシュは、セッション前の
  スナップショットと突き合わせて指摘され、ワンクリックで復元できます。ロード順を
  消してしまう取り込みは拒否され、その理由を表示します。

## できること

**MOD。** 単純な書庫、FOMOD ウィザード、Wrye Bash の BAIN パッケージ、それ以外は
手動のピッカー - そして **root MOD をネイティブに**(script extender の
プリローダ、ENB、Engine Fixes)。Root Builder プラグインは要らず、インストール先
には何もコピーされません。個々のファイルの非表示、セパレータでのグループ化、
狙った位置への移動、MOD ごとのメモとカテゴリ、そして MO2 プロファイルの
インポータ。

一覧は MO2 のもので、その作法もそのままです。任意で出せる八つの列とそのどれでも
並べ替え、カテゴリ別またはソース別のグループ化、ダブルクリックの操作、頭文字
ジャンプ、復元するまで何もしない MOD ごとのバックアップ、そしてこのゲームが
読み込めない構造の MOD や別のゲーム向けにダウンロードされた MOD への注意フラグ。
ファイルツリーでは普通の操作 - 新規フォルダ、名前の変更、削除、開く - ができ、
画像とテキストは何も起動せずにプレビューできます。

**プラグイン。** LOOT の並べ替えを内蔵したロード順、ゲームと同じ計算による MOD
インデックス、マスタ欠落の警告、そして DLC と Creation Club のコンテンツを、
実際そうである管理外の行として表示します。

**インスタンス。** グローバル - `~/.local/share/eidos` の下で一元管理 - か、
ポータブル。好きな場所(別のドライブ、ゲーム用パーティション)に置ける自己完結の
フォルダで、移動でき、隔離されています。MO2 のそれと同じです。ポータブル
インスタンスはセッションをまたいで記憶されます。GUI も、Steam からの起動も、
CLI のコマンドもすべて最後に使ったものに従い、ゲーム ID を取るコマンドはどれも
そのフォルダを同じように受け取ります。詳しくは
[usage.ja.md](docs/guide/usage.md#インスタンス-グローバルとポータブル) に。

**プロファイル。** MOD の順序、プラグインの状態、INI、セーブがプロファイルごとに
分かれます。セーブは解析され、現在のプラグインと突き合わされ - そのセーブが必要
とするものを有効にするボタン付きで - セッションのたびに Steam Cloud 向けに
書き戻されます。

**Nexus。** アカウントを接続すると、サイトの「Mod Manager Download」ボタンが
そのままあなたのインスタンスへ着地します。インストール済みのものに対する更新
チェック、各 MOD の作者、そのプロフィールへのリンク付きで。**collection** の
リンクは、その構成 MOD をあなたのインスタンスと突き合わせて一覧します -
インストール済み、ダウンロード済み、欠落。これは collection を導入するのではなく
読むもので、その理由はペインに書いてあります。Downloads タブは書庫のライブラリ
です。絞り込み、並べ替え、削除せずに隠す、インストール済みのものをまとめて消す。
**offline** スイッチでその全部を止められます。

**ツール。** xEdit、BodySlide、DynDOLOD などは、ゲームの Proton プレフィックスの
中で*統合ビュー越しに*動きます - あなたの MOD が見え、出力は Overwrite に着地し、
ワンクリックで本物の MOD になります。それぞれが必要とするランタイムは要求に応じて
取得されるので、DLL の欠落は午後を潰す作業ではなくボタン一つです。xEdit と対に
なる QuickAutoClean は自動で見つけます - ゲームのフォルダ、MOD の中、ゲームの隣に
置いてあるツール用ディレクトリのどこにあっても - 適切なランタイムを選んだ状態で。
使うものはピン留めし、使わないものは隠し、それ自体が Steam アプリであるツールには
専用の
Steam AppID を与え、Eidos をまったく開かずに統合ビュー越しで起動する `.desktop`
のショートカットを書き出せます。

**診断。** マスタの欠落、孤立した書庫、MOD 一覧のずれ、壊れたプラグイン構成 -
そして実行後には、script extender 自身のログが実際に何を読み込んだと言っているか。

**自分のファイルを置く場所。** あなたが選んだもの - 設定、Nexus のセッション、
インスタンスの一覧、自分で書いたゲームとアドオンの定義 - は
`~/.config/Colony/Eidos/` に、ログは `~/.local/state/Colony/Eidos/` の下に。
Colony ファミリのどのプログラムも使う配置です。以前の Eidos はこれらを
`~/.config/eidos/` に置いていました。更新後の最初の起動でコピーし、その旨をログに
書き、古いディレクトリは元のまま残します。

## 他との比較

| | Eidos | Wine 上の MO2 | Fluorine-Manager | Limo / リンク配置型 |
|---|---|---|---|---|
| マネージャがネイティブに動く | ✅ | ❌ Wine の中の Windows アプリ | ✅(Qt 移植) | ✅ |
| ゲームディレクトリに触れない | ✅ 常に | ✅ | ✅ | ❌ リンクが書き込まれる |
| マウントが見える範囲 | ゲームだけ | ゲームだけ | **システム全体** | n/a |
| クラッシュ後の後片付け | 設計上不要 | 不要 | 残留マウントの復旧 | 手動で解除 |
| root MOD(ENB、プリローダ) | ✅ ネイティブ | プラグインが必要 | プラグインが必要 | 一部 |
| 必要な権限 | なし | なし | `/etc/fuse.conf` の編集 | なし |

## 速さ

| | 以前 | 現在 |
|---|---|---|
| セーブの読み込み | 約 20 秒 | **6-7 秒** |
| 1 セッションのディレクトリ読み取り | 560 万 | 46.5 万 |

セル移動は即座です。この改善は MOD への問い合わせを減らしたことで得られました。
ファイルを一つ探すのに以前は五十個すべてを順に問い合わせ、フォルダを一つ列挙する
のに以前はそれを五十回繰り返していました。どちらももうしません。ベンチマークでは
なく、普通に遊んでいる実際のインスタンスで計測しています。

## はじめる

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

あとはゲームの Steam 起動オプションを `~/.local/bin/eidos-gui %command%` にして、
「プレイ」を押します。

Arch のパッケージとリリースの書庫、先に入れておくもの、CLI の道筋は
**[docs/guide/install.ja.md](docs/guide/install.md)** に。

## Steam の起動オプション

たいていの環境はこの基本の一行だけで足ります。

```
~/.local/bin/eidos-gui %command%
```

それ以外はすべて、その前に積む環境変数です。自由に組み合わせられます。

| やりたいこと | 前に置くもの |
|---|---|
| Community Shaders で DLSS | `PROTON_ENABLE_NVAPI=1` - これがないと DLSS は黙って初期化されません。全体のチェックリストは [guide/graphics.ja.md](docs/guide/graphics.md) |
| 画面に FPS カウンタ | `DXVK_HUD=fps` |
| MOD なしでドライバレベルのフレーム補間(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - Community Shaders 自身のフレーム生成とは決して併用しないこと |
| バグ報告用の詳細なログ | `EIDOS_LOG=debug`(セッションのログは `~/.local/state/Colony/Eidos/logs/` に出ます) |
| マウントからのセッションごとの I/O レポート | `EIDOS_FUSE_STATS=1` |
| FUSE のワーカ数を変える | `EIDOS_FUSE_THREADS=8`(既定は 4。並行性のバグを追うときはまず `1` を試すこと) |
| この起動を一つのポータブルインスタンスに固定する | `EIDOS_INSTANCE=/path/to/folder` - これがないと Eidos は最後に使ったインスタンスを開きます。たいていはそれが望みどおりです |

今どきの MOD 構成(Community Shaders、DLSS、フレーム生成)で使い続ける一行 -
これは例ではなく最終的なコマンドです。

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

動作を確認する間は前に `DXVK_HUD=fps` を足し、確認できたら外してください。

もっと深い診断用のスイッチ(`EIDOS_FUSE_TRACE`、キャッシュとインデックスの
切り分けトグル、`EIDOS_FUSE_PASSTHROUGH` が既定で無効な理由)は
[guide/troubleshooting.ja.md](docs/guide/troubleshooting.md) に。

## 次に読むもの

| やりたいこと | |
|---|---|
| インストールする | [guide/install.ja.md](docs/guide/install.md) |
| CLI と GUI を覚える | [guide/usage.ja.md](docs/guide/usage.md) |
| xEdit、BodySlide、DynDOLOD を設定する | [guide/tools.ja.md](docs/guide/tools.md) |
| Fallout 4 を遊ぶ(F4SE、バージョン、NVIDIA の debris クラッシュ) | [guide/fallout4.ja.md](docs/guide/fallout4.md) |
| DLSS / フレーム生成を動かす(Community Shaders) | [guide/graphics.ja.md](docs/guide/graphics.md) |
| おかしいところを直す | [guide/troubleshooting.ja.md](docs/guide/troubleshooting.md) |
| なぜ速いのかを知り、自分で確かめる | [internals/performance.md](../../internals/performance.md) |
| 内部の仕組みを理解する | [internals/architecture.md](../../internals/architecture.md) |
| ビルドする、テストする、貢献する | [internals/contributing.md](../../internals/contributing.md) |
| そもそもなぜ存在するのかを知る | [project/landscape.md](../../project/landscape.md) |

言語はディレクトリ一つです。`docs/i18n/ja/` はリポジトリのルートを写しているので、
翻訳ページ同士のリンクは、その英語原文同士のリンクとまったく同じ文字列になります。

## 言語

プレイヤーが必要とするページは翻訳されています。**英語が正典です**。翻訳が英語と
食い違う場合は、英語のファイルのほうが正しいです。

- **Français** - [README](../fr/README.md) · [index](../fr/docs/README.md) · [install](../fr/docs/guide/install.md) · [usage](../fr/docs/guide/usage.md) · [tools](../fr/docs/guide/tools.md) · [fallout4](../fr/docs/guide/fallout4.md) · [graphics](../fr/docs/guide/graphics.md) · [troubleshooting](../fr/docs/guide/troubleshooting.md) · [extensions](../fr/docs/guide/extensions.md)
- **Русский** - [README](../ru/README.md) · [index](../ru/docs/README.md) · [install](../ru/docs/guide/install.md) · [usage](../ru/docs/guide/usage.md) · [tools](../ru/docs/guide/tools.md) · [fallout4](../ru/docs/guide/fallout4.md) · [graphics](../ru/docs/guide/graphics.md) · [troubleshooting](../ru/docs/guide/troubleshooting.md) · [extensions](../ru/docs/guide/extensions.md)
- **Deutsch** - [README](../de/README.md) · [index](../de/docs/README.md) · [install](../de/docs/guide/install.md) · [usage](../de/docs/guide/usage.md) · [tools](../de/docs/guide/tools.md) · [fallout4](../de/docs/guide/fallout4.md) · [graphics](../de/docs/guide/graphics.md) · [troubleshooting](../de/docs/guide/troubleshooting.md) · [extensions](../de/docs/guide/extensions.md)
- **Español** - [README](../es/README.md) · [index](../es/docs/README.md) · [install](../es/docs/guide/install.md) · [usage](../es/docs/guide/usage.md) · [tools](../es/docs/guide/tools.md) · [fallout4](../es/docs/guide/fallout4.md) · [graphics](../es/docs/guide/graphics.md) · [troubleshooting](../es/docs/guide/troubleshooting.md) · [extensions](../es/docs/guide/extensions.md)
- **Português (BR)** - [README](../pt-BR/README.md) · [index](../pt-BR/docs/README.md) · [install](../pt-BR/docs/guide/install.md) · [usage](../pt-BR/docs/guide/usage.md) · [tools](../pt-BR/docs/guide/tools.md) · [fallout4](../pt-BR/docs/guide/fallout4.md) · [graphics](../pt-BR/docs/guide/graphics.md) · [troubleshooting](../pt-BR/docs/guide/troubleshooting.md) · [extensions](../pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](../zh-CN/README.md) · [index](../zh-CN/docs/README.md) · [install](../zh-CN/docs/guide/install.md) · [usage](../zh-CN/docs/guide/usage.md) · [tools](../zh-CN/docs/guide/tools.md) · [fallout4](../zh-CN/docs/guide/fallout4.md) · [graphics](../zh-CN/docs/guide/graphics.md) · [troubleshooting](../zh-CN/docs/guide/troubleshooting.md) · [extensions](../zh-CN/docs/guide/extensions.md)
- **Polski** - [README](../pl/README.md) · [index](../pl/docs/README.md) · [install](../pl/docs/guide/install.md) · [usage](../pl/docs/guide/usage.md) · [tools](../pl/docs/guide/tools.md) · [fallout4](../pl/docs/guide/fallout4.md) · [graphics](../pl/docs/guide/graphics.md) · [troubleshooting](../pl/docs/guide/troubleshooting.md) · [extensions](../pl/docs/guide/extensions.md)
- **Italiano** - [README](../it/README.md) · [index](../it/docs/README.md) · [install](../it/docs/guide/install.md) · [usage](../it/docs/guide/usage.md) · [tools](../it/docs/guide/tools.md) · [fallout4](../it/docs/guide/fallout4.md) · [graphics](../it/docs/guide/graphics.md) · [troubleshooting](../it/docs/guide/troubleshooting.md) · [extensions](../it/docs/guide/extensions.md)
- **Українська** - [README](../uk/README.md) · [index](../uk/docs/README.md) · [install](../uk/docs/guide/install.md) · [usage](../uk/docs/guide/usage.md) · [tools](../uk/docs/guide/tools.md) · [fallout4](../uk/docs/guide/fallout4.md) · [graphics](../uk/docs/guide/graphics.md) · [troubleshooting](../uk/docs/guide/troubleshooting.md) · [extensions](../uk/docs/guide/extensions.md)
- **日本語** - [README](README.md) · [index](docs/README.md) · [install](docs/guide/install.md) · [usage](docs/guide/usage.md) · [tools](docs/guide/tools.md) · [fallout4](docs/guide/fallout4.md) · [graphics](docs/guide/graphics.md) · [troubleshooting](docs/guide/troubleshooting.md) · [extensions](docs/guide/extensions.md)
- **繁體中文** - [README](../zh-TW/README.md) · [index](../zh-TW/docs/README.md) · [install](../zh-TW/docs/guide/install.md) · [usage](../zh-TW/docs/guide/usage.md) · [tools](../zh-TW/docs/guide/tools.md) · [fallout4](../zh-TW/docs/guide/fallout4.md) · [graphics](../zh-TW/docs/guide/graphics.md) · [troubleshooting](../zh-TW/docs/guide/troubleshooting.md) · [extensions](../zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](../cs/README.md) · [index](../cs/docs/README.md) · [install](../cs/docs/guide/install.md) · [usage](../cs/docs/guide/usage.md) · [tools](../cs/docs/guide/tools.md) · [fallout4](../cs/docs/guide/fallout4.md) · [graphics](../cs/docs/guide/graphics.md) · [troubleshooting](../cs/docs/guide/troubleshooting.md) · [extensions](../cs/docs/guide/extensions.md)
- **한국어** - [README](../ko/README.md) · [index](../ko/docs/README.md) · [install](../ko/docs/guide/install.md) · [usage](../ko/docs/guide/usage.md) · [tools](../ko/docs/guide/tools.md) · [fallout4](../ko/docs/guide/fallout4.md) · [graphics](../ko/docs/guide/graphics.md) · [troubleshooting](../ko/docs/guide/troubleshooting.md) · [extensions](../ko/docs/guide/extensions.md)
- **Türkçe** - [README](../tr/README.md) · [index](../tr/docs/README.md) · [install](../tr/docs/guide/install.md) · [usage](../tr/docs/guide/usage.md) · [tools](../tr/docs/guide/tools.md) · [fallout4](../tr/docs/guide/fallout4.md) · [graphics](../tr/docs/guide/graphics.md) · [troubleshooting](../tr/docs/guide/troubleshooting.md) · [extensions](../tr/docs/guide/extensions.md)
- **Nederlands** - [README](../nl/README.md) · [index](../nl/docs/README.md) · [install](../nl/docs/guide/install.md) · [usage](../nl/docs/guide/usage.md) · [tools](../nl/docs/guide/tools.md) · [fallout4](../nl/docs/guide/fallout4.md) · [graphics](../nl/docs/guide/graphics.md) · [troubleshooting](../nl/docs/guide/troubleshooting.md) · [extensions](../nl/docs/guide/extensions.md)

**それ以外が英語なのは、手落ちではなく意図的です。** `docs/internals/` と
`docs/project/` を読む人は Rust も読んでいますし、`CHANGELOG.md` は生成物です。
翻訳すれば、それを必要としない読者のために正しく保ち続ける語が 17,678 語
増えます。

各翻訳は、元にした英語ファイルのハッシュを持っています。英語が先に進むと CI が
落ちます - [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh) を参照。追いつけなく
なった翻訳はそのまま置かず、**削除します**。古びたページはそれでも権威ありげに
見え、先月のコマンドを配ってしまい、英語へ送られるより読者にとって悪いからです。

言語の追加はファイル四つとこの表の行一つです。手順は
[`docs/internals/contributing.md`](../../internals/contributing.md) に。

## 対応ゲーム

**Skyrim SE/AE** - 実際のプレイで実証済み。**Fallout 4** も端から端まで通して
あります(F4SE の自動差し替え、archive invalidation、アスタリスク方式のロード順、
LOOT、`.fos` のセーブ) - [guide/fallout4.ja.md](docs/guide/fallout4.md) を
参照。共通のゲーム記述子で組み込み済み、テスター募集中: Skyrim LE、Skyrim VR、
Enderal SE、Fallout 3、Fallout NV、Fallout 4(+ VR)、Starfield、Oblivion、
Morrowind(最後の二つはマウントと MOD 管理はできますが、タイムスタンプ順の
プラグイン一覧はまだ管理していません)。

系列の追加は記述子の一行です:
[internals/adding-games.md](../../internals/adding-games.md)。

## 先行事例と謝辞

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) と
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos が再現する意味論と、
  その同等性を突き合わせて調べたコードベース
- [LOOT](https://loot.github.io/) - 並べ替えのエンジン、libloot 経由
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) をはじめとする Linux のマネージャ群 -
  これを解決してほしいコミュニティが存在することの証

## ライセンス

GPL-3.0-or-later。MOD 管理はみんなのものです。
