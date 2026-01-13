# ADR.md — Architecture Decision Record

本書は vt-optimizer-rs のアーキテクチャ上の意思決定を記録する。各項目は “現時点の決定” とし、破壊的変更が必要になった場合は追記/改訂する。

## ADR-0001: プロダクト形態（CLI + SDK）
- Status: Accepted
- Decision:
  - 単一バイナリCLIを主とし、同一コアを SDK（ライブラリ）として提供する。
- Rationale:
  - バッチ運用と組み込み（他ツールチェイン統合）の両立が必要。
- Consequences:
  - CLI層は薄く、コアはライブラリとして設計する（API安定性が要件化）。

## ADR-0002: 対象フォーマット（MBTiles + PMTiles）
- Status: Accepted
- Decision:
  - 入力: MBTiles / PMTiles v3（まずはv3のみ、必要ならv2読取を追加）
  - 出力: ユーザー指定。既定は入力と同一。--out の拡張子優先（ffmpeg風）。
- Rationale:
  - 実運用ではMBTiles/PMTilesが混在し得るため、変換ツールとしても機能させる。
- References:
  - MBTiles 1.3, PMTiles v3（別紙SPEC参照）

## ADR-0003: タイル座標の内部表現
- Status: Accepted
- Decision:
  - 内部表現は XYZ（z/x/y）で統一し、入出力境界で MBTiles の tile_row 規約へ変換する。
- Rationale:
  - 実装と利用者の理解を単純化し、ミス（Y反転）を境界層に押し込める。

## ADR-0004: スタイル解釈の範囲（filter-only をデフォルト）
- Status: Accepted
- Decision:
  - Mapbox + MapLibre の style.json を解釈可能にする。
  - feature 削除は filter のみを根拠とする（filter-only）。
  - zoom式は整数ズームで評価し、feature-state は filter 内では非対応。
  - 未対応式に遭遇した場合は “keep”（削除しない）。
  - 追加モード（将来）として “rendered” を用意可能（paint/layout を考慮）。
- Rationale:
  - “削り過ぎ”が最も危険。未対応は保守的に keep し、正確性と安全性を優先。
  - ただしユーザーが引数で挙動を選べる拡張余地を残す。

## ADR-0005: 処理モデル（ストリーミング + 読取り並列 + 単一writer集約）
- Status: Accepted
- Decision:
  - デフォルト実行エンジンは「読取り並列＋単一writer集約」。
  - タイルはストリーミングで読み、全件メモリロードはしない。
- Rationale:
  - planet規模でメモリ負荷を抑え、安定したスループットを狙う。
  - SQLiteは WAL で reader/writer の同時進行が可能であり、この構成と相性が良い。
- Notes:
  - writer 側はバッチ書き込み（一定タイル数/サイズで commit）を採用する。

## ADR-0006: shard エンジン（分割→成果物→マージ）は“実行プラン”として将来追加
- Status: Accepted (Deferred Implementation)
- Decision:
  - `--engine=shard` のような実行プランとして追加可能な構造にする。
  - ただし初期リリースは pipeline のみで SLO 達成を優先。
- Rationale:
  - 実装/運用の複雑性が増すため、まず本線を成功させる。

## ADR-0007: チェックポイント/再開は sidecar（JSON/SQLite）で実現
- Status: Accepted
- Decision:
  - sidecar（JSON/SQLite）に進捗とオプション、完了範囲を記録し再開を可能にする。
- Rationale:
  - 入出力ファイルを汚さず、フォーマット差（MBTiles/PMTiles）に依存しない。

## ADR-0008: CLI方針（非対話、古き良きCLI）
- Status: Accepted
- Decision:
  - 対話UIを廃し、サブコマンド + オプションの伝統的CLIとする。
  - 機械可読出力（JSONレポート）を第一級に扱う。
- Rationale:
  - planet規模バッチ運用、CI統合、ログ運用の容易性を優先。

## ADR-0009: 実装言語は Rust（CLI + SDK 共通コア）
- Status: Accepted
- Decision:
  - vt-optimizer-rs のコア実装は Rust を採用する（CLI と SDK は同一コアを共有）。
- Rationale:
  - planet規模のスループット（CPU並列 + I/O）と単一バイナリ配布の両立を最優先する。
  - SQLite(WAL) を用いた「読取り並列＋単一writer集約」において、低オーバーヘッドなスレッド並列・ストリーミング実装が必要。WAL は reader/writer の同時進行が可能である。
- Consequences:
  - SDK の一次提供言語は Rust。将来の他言語バインディングは別ADRで扱う。

## ADR-0010: CLI フレームワークは clap
- Status: Accepted
- Decision:
  - CLI 引数解析は Rust の clap を採用する。
- Rationale:
  - サブコマンド中心の「古き良きCLI」を、型安全に保ちつつ拡張しやすい。

## ADR-0011: 観測性・レポートの基盤は tracing + serde
- Status: Accepted
- Decision:
  - ログ/計測は tracing（+ tracing-subscriber）を採用する。
  - JSONレポート/sidecarは serde を採用する。
- Rationale:
  - バッチ運用で「人間向けログ」と「機械可読(JSON)」を両立する。

## ADR-0012: SQLite（MBTiles）アクセスは rusqlite（SQLite同梱ビルド）
- Status: Accepted
- Decision:
  - MBTiles の読み書きは rusqlite を採用する。
  - 依存する libsqlite3 は “同梱（bundled）” でビルドし、最終バイナリに静的リンクする方針を基本とする（利用者に SQLite を要求しない）。
- Rationale:
  - 可搬性（環境依存のSQLiteを排除）と性能・制御（WAL/PRAGMA）を優先する。
- Notes:
  - shard/multi-db マージで ATTACH を使う場合、WAL ではDB間の原子性が保証されないため、マージ工程の設計は別ADRで慎重に扱う。

## ADR-0013: 並列処理は rayon + クロススレッドキュー（crossbeam-channel）
- Status: Accepted
- Decision:
  - CPU並列（decode→filter→encode）は rayon を採用する。
  - Reader/Worker/Writer 間の受け渡しは crossbeam-channel を採用する。
- Rationale:
  - planet規模はスループット最優先。rayon は同期処理での並列化導入が容易で、既定のスレッドプール運用が現実的。
  - MBTiles 出力は SQLite の性質上、writer を単一化しやすい（WAL は reader/writer 併走を許容）。

## ADR-0014: MVT（PBF）のデコード/エンコードは protobuf 実装（prost）を採用
- Status: Accepted
- Decision:
  - MVT（Vector Tile）PBF のエンコード/デコードは prost を採用する。
  - MVT は Protobuf（PBF）であることを前提に、Mapbox Vector Tile Spec に従う。
- Rationale:
  - 低レイヤの制御（ゼロコピー寄りの処理、バッチ化）と性能を優先する。

## ADR-0015: 圧縮は gzip をデフォルト（flate2）
- Status: Accepted
- Decision:
  - MBTiles の tile_data（gzip圧縮PBFが一般的）および必要箇所の gzip は flate2 を採用する。
- Rationale:
  - vt-optimizer互換の挙動を崩しにくく、ストリーミング圧縮/伸長が容易。

## ADR-0016: PMTiles は v3 を実装（出力 v3 固定、入力もまず v3）
- Status: Accepted
- Decision:
  - PMTiles は v3 をサポートする（出力は v3 固定、入力もまず v3）。仕様は公式 spec に従う。
  - PMTiles は read-only（インプレース更新不可）である前提のため、出力は常に新規生成とする。
- Rationale:
  - フォーマット特性に合わせて、出力設計を単純化し信頼性を上げる。

## ADR-0017: Style 解釈（filter-only）の実装方針と依存
- Status: Accepted
- Decision:
  - style.json のパースは serde_json ベースで実装し、filter/expression 評価は内製（必要最小セットから拡張）する。
  - filterの挙動は「一致するfeatureだけ表示」「zoom式は整数ズーム評価」「filter内 feature-state 非対応」に準拠し、未対応式は keep とする。
- Rationale:
  - 外部JSランタイム等を抱えず単一バイナリの可搬性を守る。
  - “削り過ぎ”を避ける保守的デフォルト（keep）を徹底する。
