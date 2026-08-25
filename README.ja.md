<!-- eidos-i18n: source=README.md sha=1d6c3a7886c5271693cbd986804bc5608d27cf3b -->

<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
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
[usage.ja.md](docs/guide/usage.ja.md#インスタンス-グローバルとポータブル) に。

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
**[docs/guide/install.ja.md](docs/guide/install.ja.md)** に。

## Steam の起動オプション

たいていの環境はこの基本の一行だけで足ります。

```
~/.local/bin/eidos-gui %command%
```

それ以外はすべて、その前に積む環境変数です。自由に組み合わせられます。

| やりたいこと | 前に置くもの |
|---|---|
| Community Shaders で DLSS | `PROTON_ENABLE_NVAPI=1` - これがないと DLSS は黙って初期化されません。全体のチェックリストは [guide/graphics.ja.md](docs/guide/graphics.ja.md) |
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
[guide/troubleshooting.ja.md](docs/guide/troubleshooting.ja.md) に。

## 次に読むもの

| やりたいこと | |
|---|---|
| インストールする | [guide/install.ja.md](docs/guide/install.ja.md) |
| CLI と GUI を覚える | [guide/usage.ja.md](docs/guide/usage.ja.md) |
| xEdit、BodySlide、DynDOLOD を設定する | [guide/tools.ja.md](docs/guide/tools.ja.md) |
| Fallout 4 を遊ぶ(F4SE、バージョン、NVIDIA の debris クラッシュ) | [guide/fallout4.ja.md](docs/guide/fallout4.ja.md) |
| DLSS / フレーム生成を動かす(Community Shaders) | [guide/graphics.ja.md](docs/guide/graphics.ja.md) |
| おかしいところを直す | [guide/troubleshooting.ja.md](docs/guide/troubleshooting.ja.md) |
| なぜ速いのかを知り、自分で確かめる | [internals/performance.md](docs/internals/performance.md) |
| 内部の仕組みを理解する | [internals/architecture.md](docs/internals/architecture.md) |
| ビルドする、テストする、貢献する | [internals/contributing.md](docs/internals/contributing.md) |
| そもそもなぜ存在するのかを知る | [project/landscape.md](docs/project/landscape.md) |

索引の全体は [docs/README.ja.md](docs/README.ja.md) に、セキュリティ方針と脆弱性の
報告方法は [SECURITY.md](SECURITY.md) に。

## 言語

プレイヤーが必要とするページは翻訳されています。**英語が正典です**。翻訳が英語と
食い違う場合は、英語のファイルのほうが正しいです。

- **Français** - [README](README.fr.md) · [index](docs/README.fr.md) · [install](docs/guide/install.fr.md) · [usage](docs/guide/usage.fr.md) · [tools](docs/guide/tools.fr.md) · [fallout4](docs/guide/fallout4.fr.md) · [graphics](docs/guide/graphics.fr.md) · [troubleshooting](docs/guide/troubleshooting.fr.md) · [extensions](docs/guide/extensions.fr.md)
- **Русский** - [README](README.ru.md) · [index](docs/README.ru.md) · [install](docs/guide/install.ru.md) · [usage](docs/guide/usage.ru.md) · [tools](docs/guide/tools.ru.md) · [fallout4](docs/guide/fallout4.ru.md) · [graphics](docs/guide/graphics.ru.md) · [troubleshooting](docs/guide/troubleshooting.ru.md) · [extensions](docs/guide/extensions.ru.md)
- **Deutsch** - [README](README.de.md) · [index](docs/README.de.md) · [install](docs/guide/install.de.md) · [usage](docs/guide/usage.de.md) · [tools](docs/guide/tools.de.md) · [fallout4](docs/guide/fallout4.de.md) · [graphics](docs/guide/graphics.de.md) · [troubleshooting](docs/guide/troubleshooting.de.md) · [extensions](docs/guide/extensions.de.md)
- **Español** - [README](README.es.md) · [index](docs/README.es.md) · [install](docs/guide/install.es.md) · [usage](docs/guide/usage.es.md) · [tools](docs/guide/tools.es.md) · [fallout4](docs/guide/fallout4.es.md) · [graphics](docs/guide/graphics.es.md) · [troubleshooting](docs/guide/troubleshooting.es.md) · [extensions](docs/guide/extensions.es.md)
- **Português (BR)** - [README](README.pt-BR.md) · [index](docs/README.pt-BR.md) · [install](docs/guide/install.pt-BR.md) · [usage](docs/guide/usage.pt-BR.md) · [tools](docs/guide/tools.pt-BR.md) · [fallout4](docs/guide/fallout4.pt-BR.md) · [graphics](docs/guide/graphics.pt-BR.md) · [troubleshooting](docs/guide/troubleshooting.pt-BR.md) · [extensions](docs/guide/extensions.pt-BR.md)
- **简体中文** - [README](README.zh-CN.md) · [index](docs/README.zh-CN.md) · [install](docs/guide/install.zh-CN.md) · [usage](docs/guide/usage.zh-CN.md) · [tools](docs/guide/tools.zh-CN.md) · [fallout4](docs/guide/fallout4.zh-CN.md) · [graphics](docs/guide/graphics.zh-CN.md) · [troubleshooting](docs/guide/troubleshooting.zh-CN.md) · [extensions](docs/guide/extensions.zh-CN.md)
- **Polski** - [README](README.pl.md) · [index](docs/README.pl.md) · [install](docs/guide/install.pl.md) · [usage](docs/guide/usage.pl.md) · [tools](docs/guide/tools.pl.md) · [fallout4](docs/guide/fallout4.pl.md) · [graphics](docs/guide/graphics.pl.md) · [troubleshooting](docs/guide/troubleshooting.pl.md) · [extensions](docs/guide/extensions.pl.md)
- **Italiano** - [README](README.it.md) · [index](docs/README.it.md) · [install](docs/guide/install.it.md) · [usage](docs/guide/usage.it.md) · [tools](docs/guide/tools.it.md) · [fallout4](docs/guide/fallout4.it.md) · [graphics](docs/guide/graphics.it.md) · [troubleshooting](docs/guide/troubleshooting.it.md) · [extensions](docs/guide/extensions.it.md)
- **Українська** - [README](README.uk.md) · [index](docs/README.uk.md) · [install](docs/guide/install.uk.md) · [usage](docs/guide/usage.uk.md) · [tools](docs/guide/tools.uk.md) · [fallout4](docs/guide/fallout4.uk.md) · [graphics](docs/guide/graphics.uk.md) · [troubleshooting](docs/guide/troubleshooting.uk.md) · [extensions](docs/guide/extensions.uk.md)
- **日本語** - [README](README.ja.md) · [index](docs/README.ja.md) · [install](docs/guide/install.ja.md) · [usage](docs/guide/usage.ja.md) · [tools](docs/guide/tools.ja.md) · [fallout4](docs/guide/fallout4.ja.md) · [graphics](docs/guide/graphics.ja.md) · [troubleshooting](docs/guide/troubleshooting.ja.md) · [extensions](docs/guide/extensions.ja.md)
- **繁體中文** - [README](README.zh-TW.md) · [index](docs/README.zh-TW.md) · [install](docs/guide/install.zh-TW.md) · [usage](docs/guide/usage.zh-TW.md) · [tools](docs/guide/tools.zh-TW.md) · [fallout4](docs/guide/fallout4.zh-TW.md) · [graphics](docs/guide/graphics.zh-TW.md) · [troubleshooting](docs/guide/troubleshooting.zh-TW.md) · [extensions](docs/guide/extensions.zh-TW.md)
- **Čeština** - [README](README.cs.md) · [index](docs/README.cs.md) · [install](docs/guide/install.cs.md) · [usage](docs/guide/usage.cs.md) · [tools](docs/guide/tools.cs.md) · [fallout4](docs/guide/fallout4.cs.md) · [graphics](docs/guide/graphics.cs.md) · [troubleshooting](docs/guide/troubleshooting.cs.md) · [extensions](docs/guide/extensions.cs.md)
- **한국어** - [README](README.ko.md) · [index](docs/README.ko.md) · [install](docs/guide/install.ko.md) · [usage](docs/guide/usage.ko.md) · [tools](docs/guide/tools.ko.md) · [fallout4](docs/guide/fallout4.ko.md) · [graphics](docs/guide/graphics.ko.md) · [troubleshooting](docs/guide/troubleshooting.ko.md) · [extensions](docs/guide/extensions.ko.md)
- **Türkçe** - [README](README.tr.md) · [index](docs/README.tr.md) · [install](docs/guide/install.tr.md) · [usage](docs/guide/usage.tr.md) · [tools](docs/guide/tools.tr.md) · [fallout4](docs/guide/fallout4.tr.md) · [graphics](docs/guide/graphics.tr.md) · [troubleshooting](docs/guide/troubleshooting.tr.md) · [extensions](docs/guide/extensions.tr.md)
- **Nederlands** - [README](README.nl.md) · [index](docs/README.nl.md) · [install](docs/guide/install.nl.md) · [usage](docs/guide/usage.nl.md) · [tools](docs/guide/tools.nl.md) · [fallout4](docs/guide/fallout4.nl.md) · [graphics](docs/guide/graphics.nl.md) · [troubleshooting](docs/guide/troubleshooting.nl.md) · [extensions](docs/guide/extensions.nl.md)


**それ以外が英語なのは、手落ちではなく意図的です。** `docs/internals/` と
`docs/project/` を読む人は Rust も読んでいますし、`CHANGELOG.md` は生成物です。
翻訳すれば、それを必要としない読者のために正しく保ち続ける語が 17,678 語
増えます。

各翻訳は、元にした英語ファイルのハッシュを持っています。英語が先に進むと CI が
落ちます - [`scripts/i18n-check.sh`](scripts/i18n-check.sh) を参照。追いつけなく
なった翻訳はそのまま置かず、**削除します**。古びたページはそれでも権威ありげに
見え、先月のコマンドを配ってしまい、英語へ送られるより読者にとって悪いからです。

言語の追加はファイル四つとこの表の行一つです。手順は
[`docs/internals/contributing.md`](docs/internals/contributing.md) に。

## 対応ゲーム

**Skyrim SE/AE** - 実際のプレイで実証済み。**Fallout 4** も端から端まで通して
あります(F4SE の自動差し替え、archive invalidation、アスタリスク方式のロード順、
LOOT、`.fos` のセーブ) - [guide/fallout4.ja.md](docs/guide/fallout4.ja.md) を
参照。共通のゲーム記述子で組み込み済み、テスター募集中: Skyrim LE、Skyrim VR、
Enderal SE、Fallout 3、Fallout NV、Fallout 4(+ VR)、Starfield、Oblivion、
Morrowind(最後の二つはマウントと MOD 管理はできますが、タイムスタンプ順の
プラグイン一覧はまだ管理していません)。

系列の追加は記述子の一行です:
[internals/adding-games.md](docs/internals/adding-games.md)。

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
