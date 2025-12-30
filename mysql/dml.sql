INSERT INTO card_input (
  order_id, product_code, order_qty, due_date,
  provisional_issued, shipped_flg, order_date, created_at
)
SELECT
  230753000000 + n AS order_id,

  -- 05763-001 ～ 05763-300 の 300 種類
  CONCAT(
    '05763-',
    LPAD(product_no, 3, '0')
  ) AS product_code,

  -- 数量（1〜8：低数量が出やすい）
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

  -- 納期：2023-09-01 + 0〜180日
  DATE_ADD('2023-09-01', INTERVAL due_offset DAY) AS due_date,

  -- 仮発行：10〜20%程度でばらつく
  (prov_r < (0.10 + prov_r2 * 0.10)) AS provisional_issued,

  FALSE AS shipped_flg,

  -- 受注日：2023-07-15〜2023-08-31
  DATE_ADD('2023-07-15', INTERVAL order_offset DAY) AS order_date,

  NOW() AS created_at
FROM (
  SELECT
    n,

    -- 品番用乱数（1〜300）
    FLOOR(RAND() * 300) + 1 AS product_no,

    -- 数量分布用
    RAND() AS qty_r,

    -- 納期オフセット
    FLOOR(RAND() * 181) AS due_offset,

    -- 仮発行用
    RAND() AS prov_r,
    RAND() AS prov_r2,

    -- 受注日オフセット
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
  product_code = VALUES(product_code),
  order_qty = VALUES(order_qty),
  due_date = VALUES(due_date),
  provisional_issued = VALUES(provisional_issued),
  shipped_flg = VALUES(shipped_flg),
  order_date = VALUES(order_date),
  created_at = VALUES(created_at);
