SET NAMES utf8mb4;

-- =========================================
-- HATYU
-- =========================================
DROP TABLE IF EXISTS hatyu;
CREATE TABLE hatyu (
  order_no            BIGINT NOT NULL COMMENT '注文書NO（HATYUの一意キー想定。異なる場合は差替）',
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
  KEY idx_hatyu_product_due (product_code, due_date)
) COMMENT='HATYU (purchase order source)';


-- =========================================
-- card_input
-- =========================================
DROP TABLE IF EXISTS card_input;
CREATE TABLE card_input (
  order_id            BIGINT NOT NULL COMMENT '注文書ID',
  product_code        VARCHAR(30) NOT NULL COMMENT '品番',
  order_qty           INT NOT NULL COMMENT '発注数',
  due_date            DATE NOT NULL COMMENT '納期',
  provisional_issued  BOOLEAN NOT NULL DEFAULT FALSE COMMENT '仮注文書発行',
  shipped_flg         BOOLEAN NOT NULL DEFAULT FALSE COMMENT '出荷済',
  order_date          DATE NOT NULL COMMENT 'ORDER日',
  created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '作成日時',

  PRIMARY KEY (order_id),
  KEY idx_card_input_product_due (due_date)
) COMMENT='Card input (calculated result)';


-- =========================================
-- blade_issue
-- =========================================
DROP TABLE IF EXISTS blade_issue;
CREATE TABLE blade_issue (
  id                 BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  product_code       VARCHAR(30) NOT NULL COMMENT '品番',
  product_name       VARCHAR(200) NULL COMMENT '品名',
  order_no           BIGINT NULL COMMENT '注文書NO',
  qty                INT NOT NULL COMMENT '数量',
  issue_date         DATE NOT NULL COMMENT '出庫日',
  temp_slip          BOOLEAN NOT NULL DEFAULT FALSE COMMENT '仮伝票',
  temp_slip_history  BOOLEAN NOT NULL DEFAULT FALSE COMMENT '仮伝履歴',
  source_slip_no     VARCHAR(100) NULL COMMENT '割当元伝票',
  not_guided         BOOLEAN NOT NULL DEFAULT FALSE COMMENT '未案内',
  slip_not_issued    BOOLEAN NOT NULL DEFAULT FALSE COMMENT '伝票未発行',
  created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '登録日時',

  PRIMARY KEY (id),
  KEY idx_blade_issue_product_date (product_code, issue_date),
  KEY idx_blade_issue_order_no (order_no)
) COMMENT='Blade issue (temporary issue input)';


-- =========================================
-- pickup_product
-- =========================================
DROP TABLE IF EXISTS pickup_product;
CREATE TABLE pickup_product (
  id        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  product1  VARCHAR(30) NULL COMMENT '品番1',
  product2  VARCHAR(30) NULL COMMENT '品番2',
  product3  VARCHAR(30) NULL COMMENT '品番3',
  product4  VARCHAR(30) NULL COMMENT '品番4',
  product5  VARCHAR(30) NULL COMMENT '品番5',
  product6  VARCHAR(30) NULL COMMENT '品番6',

  PRIMARY KEY (id),
  KEY idx_pickup_product1 (product1)
) COMMENT='Pickup products (alternative/bundle candidates, wide format)';


-- =========================================
-- material_master
-- =========================================
DROP TABLE IF EXISTS material_master;
CREATE TABLE material_master (
  material_code   VARCHAR(60) NOT NULL COMMENT '材料品番',
  material_name   VARCHAR(200) NULL COMMENT '材料品名',
  material_class  VARCHAR(50) NULL COMMENT '材料分類',

  PRIMARY KEY (material_code),
  KEY idx_material_class (material_class)
) COMMENT='Material master';


-- =========================================
-- shipment_history
-- =========================================
DROP TABLE IF EXISTS shipment_history;
CREATE TABLE shipment_history (
  history_id     BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  order_no       BIGINT NULL COMMENT '注文書NO',
  product_code   VARCHAR(30) NOT NULL COMMENT '品番',
  product_name   VARCHAR(200) NULL COMMENT '品名',
  qty            INT NOT NULL COMMENT '数量',
  due_date       DATE NULL COMMENT '納期',
  issue_date     DATE NOT NULL COMMENT '出庫日',
  not_guided     BOOLEAN NOT NULL DEFAULT FALSE COMMENT '未案内',
  printed        BOOLEAN NOT NULL DEFAULT FALSE COMMENT '印刷',
  created_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '登録日時',

  PRIMARY KEY (history_id),
  KEY idx_shipment_order_no (order_no),
  KEY idx_shipment_product_issue_date (product_code, issue_date)
) COMMENT='Shipment history (source of truth)';


-- =========================================
-- shipment_qty
-- =========================================
DROP TABLE IF EXISTS shipment_qty;
CREATE TABLE shipment_qty (
  shipment_qty INT NOT NULL COMMENT '出荷数量',
  PRIMARY KEY (shipment_qty)
) COMMENT='Shipment quantity options';


-- =========================================
-- product_master
-- =========================================
DROP TABLE IF EXISTS product_master;
CREATE TABLE product_master (
  product_code  VARCHAR(30) NOT NULL COMMENT '品番',
  product_name  VARCHAR(200) NULL COMMENT '品名',
  unit_price    DECIMAL(12,2) NULL COMMENT '単価',
  listed        BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'リスト',

  PRIMARY KEY (product_code)
) COMMENT='Product master';


-- =========================================
-- material_blade_receipt
-- =========================================
DROP TABLE IF EXISTS material_blade_receipt;
CREATE TABLE material_blade_receipt (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  product_code VARCHAR(30) NOT NULL COMMENT '品番',
  qty          DECIMAL(12,4) NOT NULL COMMENT '数量',
  receipt_date DATE NOT NULL COMMENT '入庫日',
  is_order     BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'ORDER',
  created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '登録日時',

  PRIMARY KEY (id),
  KEY idx_mbr_product_date (product_code, receipt_date)
) COMMENT='Material blade receipt';


-- =========================================
-- bom
-- =========================================
DROP TABLE IF EXISTS bom;
CREATE TABLE bom (
  product_code   VARCHAR(30) NOT NULL COMMENT '品番',
  material_code  VARCHAR(60) NOT NULL COMMENT '材料品番',
  required_qty   DECIMAL(12,4) NOT NULL COMMENT '材料所要量',
  class          VARCHAR(50) NULL COMMENT '分類',

  PRIMARY KEY (product_code, material_code),
  KEY idx_bom_material (material_code)
) COMMENT='Bill of materials';


-- =========================================
-- hokuryo_manual
-- =========================================
DROP TABLE IF EXISTS hokuryo_manual;
CREATE TABLE hokuryo_manual (
  id      BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  manual  LONGTEXT NULL COMMENT '北菱マニュアル',

  PRIMARY KEY (id)
) COMMENT='Hokuryo manual';


-- =========================================
-- body_arm_card_input
-- =========================================
DROP TABLE IF EXISTS body_arm_card_input;
CREATE TABLE body_arm_card_input (
  id           BIGINT NOT NULL COMMENT 'ID',
  standard     INT NULL COMMENT '規定',
  product_code VARCHAR(30) NOT NULL COMMENT '品番',
  order_qty    INT NOT NULL COMMENT '発注数',
  due_date     DATE NOT NULL COMMENT '納期',
  transferred  BOOLEAN NOT NULL DEFAULT FALSE COMMENT '転送',

  PRIMARY KEY (id),
  KEY idx_body_arm_transfer (transferred, due_date),
  KEY idx_body_arm_product (product_code)
) COMMENT='Body arm card input';
