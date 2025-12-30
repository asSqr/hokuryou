INSERT INTO card_input (
  order_id, product_code, order_qty, due_date,
  provisional_issued, shipped_flg, order_date, created_at
)
SELECT
  230753000000 + n,
  CASE WHEN n % 3 = 1 THEN '05763-03112'
       WHEN n % 3 = 2 THEN '05763-04199'
       ELSE '05763-05188' END,
  (n % 3) + 1,
  DATE_ADD('2023-09-01', INTERVAL (n - 1) DAY),
  CASE WHEN n % 10 = 0 THEN TRUE ELSE FALSE END,
  FALSE,
  DATE('2023-08-01'),
  NOW()
FROM (
  SELECT ones.n + tens.n*10 AS n
  FROM
    (SELECT 1 n UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5
     UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9 UNION ALL SELECT 10) ones
  CROSS JOIN
    (SELECT 0 n UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4
     UNION ALL SELECT 5 UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9) tens
) seq
WHERE n BETWEEN 1 AND 100
ON DUPLICATE KEY UPDATE
  product_code = VALUES(product_code),
  order_qty = VALUES(order_qty),
  due_date = VALUES(due_date),
  provisional_issued = VALUES(provisional_issued),
  shipped_flg = VALUES(shipped_flg),
  order_date = VALUES(order_date),
  created_at = VALUES(created_at);
