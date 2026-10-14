<!-- eidos-i18n: source=README.md sha=9c6df24d2b5b403d78e0fd5afa489427e7bf9d71 -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../../../assets/brand/png/eidos-logo-512.png">
  <img src="../../../assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**ゲームに一切触れない、ネイティブ Linux の MOD マネージャ。**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#インストール)

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

### 他との比較

| | Eidos | Wine 上の MO2 | Fluorine-Manager | Limo / リンク配置型 |
|---|---|---|---|---|
| マネージャがネイティブに動く | ✅ | ❌ Wine の中の Windows アプリ | ✅(Qt 移植) | ✅ |
| ゲームディレクトリに触れない | ✅ 常に | ✅ | ✅ | ❌ リンクが書き込まれる |
| マウントが見える範囲 | ゲームだけ | ゲームだけ | **システム全体** | n/a |
| クラッシュ後の後片付け | 設計上不要 | 不要 | 残留マウントの復旧 | 手動で解除 |
| root MOD(ENB、プリローダ) | ✅ ネイティブ | プラグインが必要 | プラグインが必要 | 一部 |
| 必要な権限 | なし | なし | `/etc/fuse.conf` の編集 | なし |

### 速さ

| | 以前 | 現在 |
|---|---|---|
| セーブの読み込み | 約 20 秒 | **6-7 秒** |
| 1 セッションのディレクトリ読み取り | 560 万 | 46.5 万 |

セル移動は即座です。この改善は MOD への問い合わせを減らしたことで得られました。
ファイルを一つ探すのに以前は五十個すべてを順に問い合わせ、フォルダを一つ列挙する
のに以前はそれを五十回繰り返していました。どちらももうしません。ベンチマークでは
なく、普通に遊んでいる実際のインスタンスで計測しています。

### 先行事例と謝辞

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) と
  [usvfs](https://github.com/ModOrganizer2/usvfs) - Eidos が再現する意味論と、
  その同等性を突き合わせて調べたコードベース
- [LOOT](https://loot.github.io/) - 並べ替えのエンジン、libloot 経由
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager)、
  [Limo](https://github.com/limo-app/limo) をはじめとする Linux のマネージャ群 -
  これを解決してほしいコミュニティが存在することの証

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

**インスタンス。** グローバル - `~/.local/share/Colony/Eidos/instances/` の下で一元管理 - か、
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

**Nexus。** アカウントを接続すると、Mod Manager Download のリンクを受信し、更新を確認し、作者とそのプロフィールを表示できます。コレクションはレシピに従ってインストールされ、中断した処理を再開できます。未対応の手順は明示的に報告されます。照合では正確な元ファイルを確認し、不完全な古いメタデータは未検証として扱います。ダウンロードタブではアーカイブを絞り込み、並べ替え、非表示、削除できます。**オフライン**切り替えで Nexus へのアクセスを停止します。

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

**自分のファイルを置く場所。** Colony ファミリのどのプログラムも使う配置です。あなたが選んだもの - 設定、Nexus のセッション、インスタンスの一覧、自分で書いたゲームとアドオンの定義 - は `~/.config/Colony/Eidos/` に、グローバルインスタンス、ダウンロードしたランタイム、セッションログは `~/.local/share/Colony/Eidos/` に、LOOT のマスターリストは `~/.cache/Colony/Eidos/` に置きます。以前の Eidos はこれらを `~/.config/eidos/`、`~/.local/share/eidos/`、`~/.local/state/` に置いていました。更新後の最初の起動でそれらを移し、その旨をログに書き、古いディレクトリは元のまま残します。移動が完了するまで、Eidos は古いファイルを使い続けます。詳しくは [internals/paths.md](../../internals/paths.md)（英語）を参照してください。

置き換え時の保護、BSA/BA2 内のファイル競合、SKSE の静的診断、生成履歴とその制限については、[整合性チェックと生成ファイル](../../../docs/internals/integrity.md)（英語）を参照してください。

### 対応ゲーム

**Skyrim SE/AE** - 実際のプレイで実証済み。**Fallout 4** も端から端まで通して
あります(F4SE の自動差し替え、archive invalidation、アスタリスク方式のロード順、
LOOT、`.fos` のセーブ) - [guide/fallout4.ja.md](docs/guide/fallout4.md) を
参照。共通のゲーム記述子で組み込み済み、テスター募集中: Skyrim LE、Skyrim VR、
Enderal SE、Fallout 3、Fallout NV、Fallout 4(+ VR)、Starfield、Oblivion、
Morrowind(最後の二つはマウントと MOD 管理はできますが、タイムスタンプ順の
プラグイン一覧はまだ管理していません)。

系列の追加は記述子の一行です:
[internals/adding-games.md](../../internals/adding-games.md)。

## インストール

### Colony 経由(推奨)

[Colony](https://github.com/Project-Colony/Colony) で **Eidos** を検索して
インストールします。更新はランチャー経由で届きます。Colony がインストールするのは
`eidos-linux` というファイル一つです。これがウィンドウであり、`eidos` という名前の
リンク経由で呼び出されたときは `eidos` コマンドラインにもなります。NIF モデルの
プレビューには、下のバンドルにしか含まれないヘルパーが必要です。ない場合は
プレビューがその旨を表示し、ほかはすべて動きます。インストール後にすること、
そして Steam をそこへ向ける方法は
[docs/guide/install.ja.md](docs/guide/install.md#colony-経由) に。

### バイナリを直接ダウンロード

[最新リリース](https://github.com/Project-Colony/Eidos/releases/latest) から
アセットを取得します:

| アセット | 中身 |
|---|---|
| `eidos-linux` | プログラム全体を一つのファイルにしたもの。Colony がインストールするのと同じ |
| `eidos-linux-bundle.tar.gz` | `eidos`、`eidos-gui`、NIF プレビュー用ヘルパー、アイコン、そしてデスクトップエントリと Nexus のダウンロードハンドラを登録するインストーラ |

まず [コード署名ポリシー](#コード署名ポリシー) に示す方法でダウンロードを検証し、
そのあと:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

インストーラは sudo なしで全部を `~/.local/bin` に入れます。`--system` と
`--bindir DIR` で別の場所を選べます。どちらのアセットにも、glibc 2.39 以降の
x86_64 Linux、FUSE(`fusermount3`)、そして MOD 書庫用の 7-Zip が必要です。

### ソースからビルドする

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Rust 1.89 以降が必要です(CI が検査する `rust-version`。CI とリリースは 1.99.0 で
ビルドしています)。さらに NIF プレビュー用ヘルパーのために `just`、CMake 3.20
以降、C++17 コンパイラが要ります。Arch のパッケージは `packaging/PKGBUILD` から
ビルドします。前提条件と `just` を使わないコマンドは
[docs/guide/install.ja.md](docs/guide/install.md) に。

### Steam の起動オプション

Eidos はゲームの起動コマンド*として*動きます。だからこそゲームが始まる前に
マウントできます。たいていの環境はこの基本の一行だけで足ります。インストール先の
絶対パスで書いてください(Steam はあなたの `PATH` を読みません):

```
~/.local/bin/eidos-gui %command%
```

Colony からの場合、そのパスは `~/.local/share/Colony/apps/Eidos/eidos-linux` です。
あとは「プレイ」を押します。Eidos はそのゲームのインスタンスで開き、Run で
ゲームが統合ビュー越しに起動します。

それ以外はすべて、その前に積む環境変数です。自由に組み合わせられます。

| やりたいこと | 前に置くもの |
|---|---|
| Community Shaders で DLSS | `PROTON_ENABLE_NVAPI=1` - これがないと DLSS は黙って初期化されません。全体のチェックリストは [guide/graphics.ja.md](docs/guide/graphics.md) |
| 画面に FPS カウンタ | `DXVK_HUD=fps` |
| MOD なしでドライバレベルのフレーム補間(RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - Community Shaders 自身のフレーム生成とは決して併用しないこと |
| バグ報告用の詳細なログ | `EIDOS_LOG=debug`(セッションのログは `~/.local/share/Colony/Eidos/logs/` に出ます) |
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

## ドキュメント

すべては [docs/](../../../docs/) にあり、読む人ごとに
[docs/README.ja.md](docs/README.md) で索引されています: ユーザーガイド
(インストール、CLI と GUI、xEdit や DynDOLOD などのツール、グラフィックス、
トラブルシューティング)、Eidos の内部の仕組み、そしてなぜ存在するのか。
セキュリティ方針と脆弱性の報告方法: [SECURITY.md](../../../SECURITY.md)。

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

言語はディレクトリ一つです。`docs/i18n/<lang>/` はリポジトリのルートを写して
いるので、`docs/i18n/de/docs/guide/install.md` はドイツ語版の
`docs/guide/install.md` です - だからこそ翻訳ページ同士のリンクは、その英語原文
同士のリンクとまったく同じ文字列になり、言語の廃止は `rm -r` 一回で済みます。

## コード署名ポリシー

リリースのアセットはすべて、開発者のマシンではなく CI の中で、Project Colony
という組織によって署名されます。リリースページでは各アセットの隣に次のものが
あります:

| ファイル | 中身 |
|---|---|
| `<asset>.sig` | 組織のリリース鍵で作った、アセットに対する ed25519 署名 |
| `<asset>.meta` | アセットを、そのファイル名、sha256、リリースのバージョンに結びつける三行 |
| `<asset>.meta.sig` | `.meta` に対する ed25519 署名 |

秘密鍵は組織のシークレットで、共有の
[sign-and-publish ワークフロー](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
が、何もビルドしないジョブの中でだけ使います。Eidos をコンパイルするジョブが
それを目にすることはありません。Colony は Eidos をインストールまたは更新する前に
三つのファイルすべてを検証し、インストール済みのものより古いリリースは拒否します。
ダウンロードを自分で検証するには、OpenSSL 3 で次のようにします(どのアセットにも
同じコマンドが使えます):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

リリースのビルド、署名、公開の方法:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)。

## プライバシー

Eidos はテレメトリも、利用状況の分析も、クラッシュレポートも送りません。この表に
あるサーバ以外には接続せず、それも表に書かれた操作のときだけです。**Offline**
モード(Preferences -> Nexus)は Nexus へのあらゆる接続を止めます。

| データ | 保存か送信か | 場所と理由 |
|---|---|---|
| 環境設定、インスタンスの一覧、ゲームとアドオンの定義 | 保存 | `~/.config/Colony/Eidos/` |
| インスタンス、ダウンロードしたランタイム、セッションログ | 保存 | `~/.local/share/Colony/Eidos/`、またはポータブルインスタンス用に選んだフォルダ |
| LOOT のマスターリスト | 保存 | `~/.cache/Colony/Eidos/loot/` |
| Nexus へのサインイン | 送信 | サインインするとブラウザで `users.nexusmods.com` が開き、その結果は、Eidos がそのサインインのためだけに `127.0.0.1` で開くリスナーへ返されます。Eidos はそれを `users.nexusmods.com/oauth/token` でトークンと交換し、Nexus がトークンの署名に使う公開鍵を取得します。[Nexus Mods のプライバシーポリシー](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus の認証情報 | 保存 | アクセストークンとリフレッシュトークンを、モード 0600 の `~/.config/Colony/Eidos/nexus.ini` に。以前の Eidos がそこに書いた `api_key=` の行は残されますが、読まれることはありません。消してもかまいません。 |
| Nexus へのリクエスト | 送信 | Nexus の機能を使うときに `api.nexusmods.com` へ: アカウント名とプレミアムの状態、MOD とファイルの詳細、更新確認、コレクション、ダウンロードリンク。どのリクエストもアクセストークンを伴い、Eidos とそのバージョンを名乗ります。 |
| Nexus からのダウンロード | 送信 | 要求したファイルを、ダウンロードリンクが示す Nexus の CDN ホストから取得し、インスタンスのダウンロードフォルダに、それぞれの隣に MO2 形式の `.meta` を添えて保存します。 |
| `nxm://` リンク | 保存 | `eidos nxm --register`(`install.sh` が代わりに実行します)が `~/.local/share/applications/eidos-nxm.desktop` を書き、`xdg-mime` でそれを `nxm://` のハンドラにします。これでサイトの Mod Manager Download ボタンがリンクを Eidos に渡します。Arch のパッケージは同じエントリをシステム全体にインストールします。 |
| LOOT のマスターリスト | 送信 | `raw.githubusercontent.com/loot/...` から、そのゲームで初めて LOOT の並べ替えをするときと、更新を求めたときに。あなたのものは何も送られません。[GitHub のプライバシーステートメント](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| ツールのランタイム | 送信 | ツールの前提条件をインストールするときか、MOD がそれを必要とするときだけ: `builds.dotnet.microsoft.com` からの .NET ランタイム(Eidos に組み込まれた SHA-256 と一致しなければ拒否されます)、そして `winetricks`。これはゲームの Proton プレフィックスにインストールするもの(Visual C++ と .NET の再頒布可能パッケージ)を、Microsoft や winetricks が使うほかのホストからダウンロードします。[Microsoft のプライバシーに関する声明](https://www.microsoft.com/en-us/privacy/privacystatement) |
| セッションログ | 保存 | `~/.local/share/Colony/Eidos/logs/`、モード 0600。インスタンスごとに十セッション、30 日で掃除されます。ホームフォルダは `~`、ユーザー名は `<user>` と書かれるので、ログはそのまま issue に貼れます。あなたが送らない限り、マシンの外へ出ることはありません。 |
| ショートカット | 保存 | `install.sh` が `eidos.desktop` とそのアイコンを `~/.local/share` の下に書きます。自分で作ったツールのショートカットは `~/.local/share/applications` に入ります。 |
| 開いたリンク | どちらでもない | MOD のページなどのリンクは、`xdg-open` を通じてあなた自身のブラウザで開きます。 |
| アドオン | どちらでもない | インストールした拡張は、あなたのマシン上で独立したプログラムとして動きます。それが何をするかは、その拡張次第です。 |

## ライセンス

Eidos のオリジナルコードは GPL-3.0-or-later です。[LICENSE](../../../LICENSE) を
参照してください。次の部分は「or later」の選択肢のない GPL-3.0 です:

- `eidos-install` 内の OMOD スクリプト移植コード。
  [元の作者へのクレジット](../../../crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md)を参照;
- 同梱の nifly ライブラリの上に作られた `eidos-nif-preview` ヘルパー。
  [各コンポーネントのライセンスとクレジット](../../../crates/eidos-nif-preview/UPSTREAM.md)を参照;
- ロード順のコードがリンクする `esplugin` と `libloadorder` の各クレート。
  これらは `GPL-3.0` を宣言しています。

バイナリに含まれるその他の Rust クレートは、それぞれのライセンス(MIT、
Apache-2.0、BSD など)を保ちます。リリースバンドルは、すべてのクレートとその
ライセンスの索引とともに、それらの告知を `licenses/crates/` に収めています。

バイナリにはさらに、Microsoft DirectX の正規の再頒布可能 DLL が八つ
(`d3dcompiler_47`、`d3dcompiler_43`、`d3dx9_43`、`d3dx11_43` の 32 ビット版と
64 ビット版)埋め込まれており、MOD やツールが必要とするときに Eidos がゲームの
プレフィックスへ配置します。これらは Microsoft の再頒布条件のもとにあります。
[入手元の記録](../../../crates/eidos-gamefeatures/assets/PROVENANCE.md)を参照してください。

MOD 管理はみんなのものです。
