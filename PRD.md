# PRD.md — vt-optimizer-rs プロダクト要求仕様書

## 1. 概要

vt-optimizer-rs は、Mapbox Vector Tiles (MVT) を格納する MBTiles / PMTiles を対象に、検査・最適化・簡略化を行う CLI ツール兼 SDK である。

- 対応入出力: MBTiles (SQLite), PMTiles v3
- 主要価値:
  1) planet 規模でも処理可能なパイプライン最適化（ストリーミング + 並列）
  2) スタイル（Mapbox/MapLibre）に基づくレイヤ/feature単位の削減（filter解釈）
  3) “vt-optimizer準拠”の結果比較ができる互換性

## 2. 背景 / 課題

- planet 規模のタイルセットは、転送量・デコード・レンダリング負荷に直結する。
- 「どのズーム・どのタイル・どのレイヤが重いか」の可視化と、
  「固定スタイル前提で不要データを削る」最終工程が必要。
- 現行ツールチェイン（vt-optimizer）は有用だが、現代要件（PMTiles、planet規模、単一バイナリ、並列、再開）に最適化されていない。

## 3. ゴール

### 3.1 プロダクトゴール
- MBTiles/PMTiles を入力に取り、同一または指定フォーマットで出力できる。
- Style-driven pruning（filter評価）で feature を削減できる。
- planet.mbtiles（約92GB）を指定環境で約6時間以内に処理できる（目標SLO）。
- 単一バイナリとして配布でき、SDKとしても利用できる。

### 3.2 成功指標
- 性能: 92GB planet.mbtiles を 16core=32vCPU / 96GB RAM / NVMe で 6時間以内（目標）
- 安全性: 未対応式に遭遇しても「削り過ぎ」が起きない（基本: keep）
- 互換性: vt-optimizer の出力と “大きく挙動が変わらない” こと（特に simplification）
- 運用性: 中断→再開ができ、途中成果の破損・重複処理が起きない

## 4. 非ゴール
- HTTP Range 前提のリモート入力（S3等）は対象外（ローカルファイルのみ）。
- 対話UI（inquirer等）は実装しない。古き良きCLIを採用する。
- レンダリング結果の完全一致（ピクセルパーフェクト）を保証しない。
- 未対応の filter/expression を完全実装することは v1 の要件にしない（遭遇時は keep）。

## 5. 想定ユーザー / ペルソナ
- 地図タイル基盤の開発者 / SRE / データエンジニア
- 自前配信（CDN/静的ホスティング）でタイルの容量・性能・コストを最適化したいチーム
- MapLibre / Mapbox スタイルを運用し、スタイル固定時にデータ削減をしたいチーム

## 6. ユースケース
1) 品質チェック（inspect）
   - ズームごとのサイズ統計、過大タイル検知、分布（10バケット）
2) 最適化（optimize）
   - style.json に基づき、不要レイヤ/feature を削除して再パッケージ
3) 簡略化（simplify）
   - vt-optimizer互換の単一タイル/単一レイヤ簡略化（まずはここから）
4) CI / バッチ運用
   - JSONレポート出力、警告（max tile size超過）をログに残す
5) 再開
   - 途中で落ちても sidecar から再開する

## 7. 機能要件（MVP）
### 7.1 CLIコマンド
- `vt-optimizer inspect <input>`
- `vt-optimizer optimize <input> --style <style.json> [--out <output>] [--out-format mbtiles|pmtiles]`
- `vt-optimizer simplify <input> --z <z> --x <x> --y <y> --layer <name> --tolerance <float> [--out <output>]`
- `vt-optimizer verify <input>`（任意：整合性チェック）

### 7.2 入出力フォーマット
- 入力: MBTiles / PMTiles(v3)
- 出力: ユーザー指定。未指定なら入力と同一。
- “ffmpeg的”ルール:
  - `--out` の拡張子が `.pmtiles` なら PMTiles v3、`.mbtiles` なら MBTiles。
  - `--out` が未指定なら `<input>.vt-optimizer.<ext>` を生成（extは入力に準拠）。

### 7.3 既定値
- `--max-tile-bytes` 既定: 1,280,000 bytes（≒1250 KiB）。超過は **警告**（失敗にしない）。

### 7.4 スタイル解釈（MVP）
- Mapbox Style Spec / MapLibre Style Spec の layer/source/source-layer を解釈する。
- filter は「一致するfeatureだけ表示」を前提に feature を削除する。
- zoom式は整数ズームとして評価する。
- filter内 feature-state は非対応（遭遇時は keep）。
- 未対応式に遭遇したら「何もしない（残す）」。

## 8. 非機能要件
- 性能: ストリーミング処理、読み取り並列＋単一writer集約が基本
- 可搬性: 単一バイナリ配布（Linux/macOS/Windows、x86_64/aarch64を優先）
- 信頼性: sidecar によるチェックポイント/再開
- 再現性: 同一入力・同一オプションで同一出力（可能な限り）
- 観測性: 進捗表示（行数/タイル数/ETAではなく、処理済みタイル数・速度を表示）、JSONレポート

## 9. リリース計画（案）
- v0.1: inspect（MBTiles/PMTiles入力）, optimize（filter-only）, MBTiles出力
- v0.2: PMTiles v3 出力、sidecar再開、JSONレポート
- v0.3: 高速化（式のコンパイル、I/Oバッチ）、planet SLO達成
- v0.4: shard エンジン（実行プラン）を試験的に導入（オプション）

## 10. 主要リスク
- filter/expression の実装範囲が広く、性能を食う
- SQLite書き込み（VACUUM/インデックス/トランザクション粒度）次第でボトルネック
- 入力データの品質（壊れたgzip/PBF、欠損metadata）が planet 規模で顕在化
