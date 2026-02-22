SET NAMES utf8mb4;

-- =========================================================
-- 1) HATYU（取込元：北菱納品書対象の受注ソース）
--   表で出ている列だけに絞る
-- =========================================================
DROP TABLE IF EXISTS hatyu;
CREATE TABLE hatyu (
  order_no            BIGINT NOT NULL COMMENT '注文書NO（例: 28157 など。HATYUのキー）',
  order_created_date  DATE NULL COMMENT 'オーダー作成日',
  customer            VARCHAR(200) NULL COMMENT '客先',

  product_code        VARCHAR(30)  NULL COMMENT '品番',
  product_name        VARCHAR(200) NULL COMMENT '品名',

  due_date            DATE NULL COMMENT '納期',
  order_qty           INT  NULL COMMENT '発注数',
  unit_price          DECIMAL(12,2) NULL COMMENT '単価',
  amount              DECIMAL(14,2) NULL COMMENT '金額',
  purchase_order_no   VARCHAR(100) NULL COMMENT '発注番号',

  imported_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '取込日時',

  PRIMARY KEY (order_no),
  KEY idx_hatyu_product_due (product_code, due_date),
  KEY idx_hatyu_customer_date (customer, order_created_date)
) COMMENT='HATYU (北菱納品書対象の受注ソース)';

-- =========================================================
-- 2) カード入力（計算結果：受注残の表示用）
--   表の列に合わせる（出荷残は保存せず算出前提でもOKだが、ここは一覧用に保持可）
-- =========================================================
DROP TABLE IF EXISTS card_input;
CREATE TABLE card_input (
  order_no      BIGINT NOT NULL COMMENT '注文書NO',
  product_code  VARCHAR(30) NOT NULL COMMENT '品番',
  product_name  VARCHAR(200) NULL COMMENT '品名',
  order_qty     INT NOT NULL COMMENT '発注数',
  due_date      DATE NOT NULL COMMENT '納期',
  unit_price    DECIMAL(12,2) NULL COMMENT '単価',
  order_date    DATE NULL COMMENT 'ORDER日',

  -- 原則は shipment_history から再計算だが、一覧高速化/凍結用途なら保持も可
  ship_remain   INT NULL COMMENT '出荷残（再計算結果のキャッシュ）',

  cumulative_order_qty     INT NOT NULL DEFAULT 0 COMMENT '必要累計',

  created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '作成日時',
  updated_at    DATETIME NULL ON UPDATE CURRENT_TIMESTAMP COMMENT '更新日時',

  PRIMARY KEY (order_no),
  KEY idx_card_product_due (product_code, due_date),
  KEY idx_card_due (due_date)
) COMMENT='Card input (計算結果：北菱納品書対象)';

-- =========================================================
-- 3) 出荷済履歴（真実の源泉：出庫実績）
--   表の列に合わせる
-- =========================================================
DROP TABLE IF EXISTS shipment_history;
CREATE TABLE shipment_history (
  history_id    BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT 'ID',
  order_no      BIGINT NULL COMMENT '注文書NO',
  product_code  VARCHAR(30) COMMENT '品番',
  product_name  VARCHAR(200) NULL COMMENT '品名',

  order_qty     INT NULL COMMENT '発注数（参照用。原則は card/hatyu 由来）',
  shipped_qty   INT NOT NULL COMMENT '出荷数量',
  unit_price    DECIMAL(12,2) NULL COMMENT '単価（実績時点）',

  due_date      DATE NULL COMMENT '納期',
  issue_date    DATE NOT NULL COMMENT '出庫日',

  created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '登録日時',

  PRIMARY KEY (history_id),
  KEY idx_ship_order_no (order_no),
  KEY idx_ship_product_issue_date (product_code, issue_date),
  KEY idx_ship_due_date (due_date)
) COMMENT='Shipment history (出荷済履歴：真実の源泉)';

-- =========================================================
-- 4) 出庫（一時入力：現場入力/振当の結果）
--   表の列に合わせる（品番が "1" になってるのはサンプル表の誤植っぽいので VARCHAR に）
-- =========================================================
DROP TABLE IF EXISTS blade_issue;
CREATE TABLE blade_issue (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '出庫ID',
  product_code VARCHAR(30) NOT NULL COMMENT '品番',
  order_no     BIGINT NULL COMMENT '注文書NO',
  qty          INT NOT NULL COMMENT '数量',
  ship_date    DATE NOT NULL COMMENT '出荷日（= 出庫日）',

  created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '登録日時',

  PRIMARY KEY (id),
  KEY idx_issue_order_no (order_no),
  KEY idx_issue_product_date (product_code, ship_date)
) COMMENT='Blade issue (出庫：一時入力)';
