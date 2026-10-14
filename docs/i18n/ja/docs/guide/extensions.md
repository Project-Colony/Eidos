<!-- eidos-i18n: source=docs/guide/extensions.md sha=5f31f1cbc3dcbfb9655f3570f01e5ada1377268d -->

# 拡張

拡張は Eidos の一部にならずに Eidos へ項目を足します。実体は、あるプログラムを
指す TOML マニフェストと、せいぜいそのプログラムだけです。

マニフェストは `~/.config/Colony/Eidos/addons/` に置き、拡張ごとに `.toml` を
一つ。**View -> Extensions -> Open folder** でフォルダを開き、**Reload** を
押します - 再起動は要りません。

## なぜ Eidos の中に何も読み込まないのか

Mod Organizer 2 はプラグインを共有ライブラリとして読み込み、Python のものは Qt
経由で動かします。どちらもここには持ち込めません。Rust に安定した ABI はなく、
別のコンパイラ - あるいは別の最適化フラグ、共有依存の別の機能セット - で
ビルドされた共有ライブラリはバージョン不一致ではなく未定義動作です。しかも
Eidos のウィジェットはコンパイル時ジェネリックなので、仮に ABI が安定していても
ライブラリが返すウィジェットを組み立てることはできません。

拡張は、あなたのアカウントが持つファイルシステムへのアクセス権で Eidos が実行するプログラムです。信頼できるヘルパーだけをインストールしてください。プロセスの分離と応答の検証は、OS のサンドボックスを提供するものではありません。

## ツール

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # すべてのゲームに出すなら省略
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

**View -> Extensions** に Run ボタン付きで現れ、切り離されて起動します - Eidos は
待ちません。

## チェック

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

更新のたびに実行され、1 行につき 1 件の所見を出力します:

```
level<TAB>title<TAB>detail
```

`level` は `problem`、`advice`、`ok` のいずれか。detail は任意です。既知の
レベルで始まらないものはすべて無視されるので、進捗出力や紛れ込んだ警告が
Eidos 自身のチェックのような行を立てることはできません。所見は **Health**
タブに、拡張の名前を前置して並びます。

チェックの持ち時間は 3 秒。超えたものは停止され、それ自身に対する問題として
報告されます - クリックのたびに続く同じ更新の中で走るので、固まったチェックは
ウィンドウを凍らせてしまうからです。

## プレースホルダ

`args` と `workdir` はどちらも次を展開します:

| プレースホルダ  | 何であるか                                   |
| --------------- | -------------------------------------------- |
| `{instance}`    | インスタンスのルート                         |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | 有効なプロファイル名                         |
| `{profile_dir}` | 有効なプロファイルのディレクトリ             |
| `{game}`        | ゲーム id、たとえば `skyrimse`               |
| `{game_name}`   | ゲームの表示名                               |
| `{install}`     | ゲームのインストール先                       |
| `{data}`        | ゲームの `Data` ディレクトリ                 |

未知のプレースホルダは空にせず書かれたまま残します。誤りが目に見えて失敗し、
`--out {typo}` が `--out --next-flag` に化けないようにするためです。すべての
プレースホルダを解決できないツールの実行は拒否され、Eidos が足りないものを
告げます。

## 拡張にできないこと

拡張は Eidos をコールバックしたり、アーカイブから自分自身を登録したり、独自のウィジェットを描画したりすることはできません。プロトコル 1 の構造化インストーラーは、検証されたファイル計画と選択の質問を返せます。レビュー、一時領域での準備、公開は Eidos が管理します。FOMOD と BAIN は引き続きネイティブのインストーラーです。プレビューやセーブ情報の拡張も、応答に上限を設けた同じホストを使います。照合の優先順位、記録済みの CLI 回答、制限、再実行時の同一性検証については、[プロトコル仕様と実行可能な例](../../../../examples/extensions/README.md)を参照してください。
