-- hatyu 投入
INSERT INTO hatyu (
  order_no, order_created_date, customer,
  product_code, product_name,
  due_date, order_qty, unit_price,
  amount, purchase_order_no, imported_at
)
SELECT
  230753000000 + n AS order_no,

  -- 受注日：2023-07-15〜2023-08-31
  DATE_ADD('2023-07-15', INTERVAL order_offset DAY) AS order_created_date,

  CONCAT('TEST-CUST-', LPAD(FLOOR(RAND()*50)+1, 2, '0')) AS customer,

  -- 05763-001 ～ 05763-300 の 300 種類
  CONCAT('05763-', LPAD(product_no, 3, '0')) AS product_code,
  CONCAT('TEST-PRODUCT-', LPAD(product_no, 3, '0')) AS product_name,

  -- 納期：2023-09-01 + 0〜180日
  DATE_ADD('2023-09-01', INTERVAL due_offset DAY) AS due_date,

  -- 発注数（1〜8：低数量が出やすい）
  CASE
    WHEN qty_r < 0.18 THEN 1
    WHEN qty_r < 0.36 THEN 2
    WHEN qty_r < 0.54 THEN 3
    WHEN qty_r < 0.70 THEN 4
    WHEN qty_r < 0.82 THEN 5
    WHEN qty_r < 0.90 THEN 6
    WHEN qty_r < 0.96 THEN 7
    ELSE 8
  END AS order_qty,

  -- 単価（例：200〜1200）
  ROUND(200 + RAND()*1000, 2) AS unit_price,

  -- 金額 = 数量 * 単価
  ROUND(
    (CASE
      WHEN qty_r < 0.18 THEN 1
      WHEN qty_r < 0.36 THEN 2
      WHEN qty_r < 0.54 THEN 3
      WHEN qty_r < 0.70 THEN 4
      WHEN qty_r < 0.82 THEN 5
      WHEN qty_r < 0.90 THEN 6
      WHEN qty_r < 0.96 THEN 7
      ELSE 8
    END) * (200 + RAND()*1000)
  , 2) AS amount,

  CONCAT('PO-', 230753000000 + n) AS purchase_order_no,

  NOW() AS imported_at
FROM (
  SELECT
    n,
    FLOOR(RAND() * 300) + 1 AS product_no,
    RAND() AS qty_r,
    FLOOR(RAND() * 181) AS due_offset,
    FLOOR(RAND() * (DATEDIFF('2023-08-31','2023-07-15') + 1)) AS order_offset
  FROM (
    SELECT ones.n + tens.n*10 + hundreds.n*100 AS n
    FROM
      (SELECT 1 n UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5
       UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9 UNION ALL SELECT 10) ones
    CROSS JOIN
      (SELECT 0 n UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4
       UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) tens
    CROSS JOIN
      (SELECT 0 n UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4
       UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) hundreds
  ) seq
  WHERE n BETWEEN 1 AND 1000
) rnd
ON DUPLICATE KEY UPDATE
  order_created_date = VALUES(order_created_date),
  customer = VALUES(customer),
  product_code = VALUES(product_code),
  product_name = VALUES(product_name),
  due_date = VALUES(due_date),
  order_qty = VALUES(order_qty),
  unit_price = VALUES(unit_price),
  amount = VALUES(amount),
  purchase_order_no = VALUES(purchase_order_no),
  imported_at = VALUES(imported_at);

-- shipment_history 投入
INSERT INTO shipment_history (
  order_no, product_code, product_name,
  qty, due_date, issue_date,
  not_guided, printed, created_at
)
SELECT
  h.order_no,
  h.product_code,
  h.product_name,

  -- 出荷数量：1〜発注数（部分出荷あり）
  GREATEST(1, FLOOR(RAND() * h.order_qty) + 1) AS qty,

  h.due_date,

  -- 出庫日：納期の -10〜+10日
  DATE_ADD(h.due_date, INTERVAL (FLOOR(RAND()*21) - 10) DAY) AS issue_date,

  -- 未案内：10%程度
  (RAND() < 0.10) AS not_guided,

  -- 印刷：60%程度
  (RAND() < 0.60) AS printed,

  NOW() AS created_at
FROM hatyu h
WHERE RAND() < 0.35;
