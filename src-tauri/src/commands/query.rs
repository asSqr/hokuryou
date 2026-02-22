use crate::commands::model::{Hatyu, OrderNo, OrderQuantity, QuantitySum, ShipmentHistoryByOrderNo};
use crate::commands::parse_value::{FromRowOwned, RowOwned};
use crate::commands::{run_blocking, run_blocking_async};
use crate::context::context::{collect, get_data_frame, get_sql_context, register};
use crate::context::error::AppError;
use crate::context::schema::AppResult;
use crate::sql::generator::{generate_sql_inserts, generate_sql_update};
use crate::utils::date_utils::time_difference_from_now;
use crate::utils::db_utils;
use crate::utils::db_utils::insert_query_history;
use chrono::Utc;
use datafusion::arrow::error::ArrowError;
use datafusion::arrow::util::display::{ArrayFormatter, FormatOptions};
use datafusion::config::CsvOptions;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::SessionContext;
use dirs;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use serde::Serialize;
use std::fs;
use std::fs::File;
use std::io::Write;
use tauri::{command, AppHandle};
use datafusion::sql::TableReference;
use datafusion_table_providers::{
    mysql::MySQLTableFactory, sql::db_connection_pool::mysqlpool::MySQLConnectionPool,
    util::secrets::to_secret_map,
};
use std::collections::HashMap;
use std::sync::Arc;


#[derive(Serialize)]
pub struct FetchResult {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub query_time: String,
}

#[derive(Serialize)]
pub struct FetchHistory {
    pub sql: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct WriterResult {
    pub query_time: String,
    pub file_name: String,
}

pub enum Dialect {
    MySQL,
    PostgreSQL,
}

impl Dialect {
    fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "MySQL" => Ok(Dialect::MySQL),
            "PostgreSQL" => Ok(Dialect::PostgreSQL),
            _ => Err(AppError::BadRequest {
                message: format!(
                    "Invalid dialect: '{}'. Please use 'MySQL' or 'PostgreSQL'.",
                    s
                ),
            }),
        }
    }
}

pub async fn create_sqlx_mysql_pool() -> Result<Pool<MySql>, sqlx::Error> {
    let conn = "mysql://easy_db:pass@localhost:13306/easy_db";
    
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(conn)
        .await?;

    Ok(pool)
}

pub async fn create_mysql_conn(context: &mut SessionContext,
    table_names: Vec<String>,
    table_paths: Vec<String>) -> AppResult<()> {
    let conn = "mysql://easy_db:pass@localhost:13306/easy_db";
    let mysql_params = to_secret_map(HashMap::from([
        ("connection_string".to_string(), conn.to_string()),
        ("sslmode".to_string(), "disabled".to_string()),
    ]));

    // Create MySQL connection pool
    let mysql_pool = Arc::new(MySQLConnectionPool::new(mysql_params).await?);

    // Create MySQL table provider factory
    // Used to generate TableProvider instances that can read MySQL table data
    let table_factory = MySQLTableFactory::new(mysql_pool);

    for (table_name, table_path) in table_names.into_iter().zip(table_paths.into_iter()) {
        context.register_table(
            table_name,
            table_factory
                .table_provider(TableReference::bare(table_path))
                .await?,
        )?;
    }

    Ok(())
}

pub async fn fetch_sql(context: &mut SessionContext, sql: &String) -> AppResult<(Vec<String>, Vec<Vec<String>>)> {
    let (header, records) = collect(context, &sql).await?;
    
    let width = header.len();

    let total_rows: usize = records.iter().map(|r| r.num_rows()).sum();
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(total_rows);
    let options = FormatOptions::default().with_null("NULL");

    for record in records {
        let formatters = record
            .columns()
            .iter()
            .map(|c| ArrayFormatter::try_new(c.as_ref(), &options))
            .collect::<Result<Vec<_>, ArrowError>>()?;

        for row in 0..record.num_rows() {
            let mut cells = Vec::with_capacity(width);
            for (_, formatter) in formatters.iter().enumerate() {
                cells.push(formatter.value(row).to_string());
            }
            rows.push(cells);
        }
    }

    Ok((header, rows))
}

pub async fn execute_sql(pool: &Pool<MySql>, sql: &String) -> AppResult<()> {
    sqlx::query(sql).execute(pool).await
        .map_err(|_| AppError::InternalServer {
            message: format!("sql execution failed")
        })?;

    Ok(())
}


async fn shipment_history_query(
    context: &mut SessionContext) -> AppResult<HashMap<OrderNo, QuantitySum>> {
    let sql = r#"
        SELECT
            order_no,
            SUM(shipped_qty) AS quantity_sum
        FROM shipment_history
        GROUP BY order_no;
    "#.to_string();

    let (_, shipment_history_rows) = fetch_sql(context, &sql).await?;
    let shipment_history_by_order_nos = shipment_history_rows
        .into_iter()
        .map(|cells| ShipmentHistoryByOrderNo::from_row(
            &RowOwned { cells: cells }
        ))
        .collect::<AppResult<Vec<_>>>()?;

    let order_no_quantity_sum_map: HashMap<OrderNo, QuantitySum> =
        shipment_history_by_order_nos.into_iter()
            .map(|r| (r.order_no, r.quantity_sum))
            .collect();

    Ok(order_no_quantity_sum_map)
}


async fn fetch_hatyu(context: &mut SessionContext,
    order_no_quantity_sum_map: &HashMap<OrderNo, QuantitySum>) -> AppResult<Vec<Hatyu>> {
    let fetch_hatyu_sql = r#"
        SELECT
            h.order_no                              AS order_no,
            h.product_code                          AS product_code,
            h.product_name                          AS product_name,
            h.order_qty                             AS order_qty,
            h.due_date                              AS due_date,
            h.unit_price                            AS unit_price,
            h.order_created_date                    AS order_date,
            CURRENT_TIMESTAMP                       AS created_at
        FROM hatyu h;
    "#.to_string();

    let (_, mut hatyu_rows) = fetch_sql(
        context,
        &fetch_hatyu_sql
    ).await?;

    let hatyus = hatyu_rows
        .into_iter()
        .map(|cells| Hatyu::from_row(
            &RowOwned { cells: cells }
        ))
        .collect::<AppResult<Vec<_>>>()?;

    let mut shipping_remaining_hatyus = Vec::new();

    for hatyu in hatyus.iter() {
        // 発注書NO
        let order_no = &hatyu.order_no;

        // 発注数
        let order_qty: i32 = i32::try_from(hatyu.order_qty.0)
            .map_err(|_| AppError::InternalServer {
                message: format!("invalid order_qty")
            })?;

        // Nz(数量合計, 0)
        let quantity_sum: i32 = i32::try_from(
            order_no_quantity_sum_map
                .get(&order_no)
                .cloned()
                .unwrap_or(QuantitySum(0))
                .0
        )
            .map_err(|_| AppError::InternalServer {
                message: format!("invalid quantity_sum")
            })?;

        // 出荷残
        let remaining_shipping: i32 = order_qty - quantity_sum;

        // 出荷残 > 0 のもののみ対象
        if remaining_shipping <= 0 {
            continue;
        }

        shipping_remaining_hatyus.push(
            hatyu.clone()
        );
    }

    compute_cumulative_order_qty(&mut shipping_remaining_hatyus);

    Ok(shipping_remaining_hatyus)
}


async fn insert_into_card_input(hatyus: &Vec<Hatyu>) -> AppResult<()> {
    if hatyus.is_empty() {
        return Ok(());
    }

    let pool = create_sqlx_mysql_pool().await
        .map_err(|_| AppError::InternalServer {
            message: format!("failed to allocate pool"),
        })?;

    // 1回のINSERTのVALUES行数（大きすぎるとSQL長制限に当たる）
    const CHUNK: usize = 500;

    for chunk in hatyus.chunks(CHUNK) {
        // chunk の行数ぶん "(?,?,?,?,?,?,?,?,?)" を並べる
        let mut values_sql = String::new();
        for i in 0..chunk.len() {
            if i > 0 {
                values_sql.push(',');
            }

            values_sql.push_str("(?,?,?,?,?, NULL, NULL, ?, ?, CURRENT_TIMESTAMP)");
        }

        let sql = format!(
            r#"
            INSERT INTO card_input (
                order_no,
                product_code,
                product_name,
                order_qty,
                due_date,
                unit_price,
                order_date,
                ship_remain,
                cumulative_order_qty,
                created_at
            )
            VALUES {values}
            ON DUPLICATE KEY UPDATE
                product_code = VALUES(product_code),
                product_name = VALUES(product_name),
                order_qty    = VALUES(order_qty),
                due_date     = VALUES(due_date),
                unit_price   = VALUES(unit_price),
                order_date   = VALUES(order_date),
                ship_remain  = VALUES(ship_remain),
                cumulative_order_qty  = VALUES(cumulative_order_qty),
                updated_at   = CURRENT_TIMESTAMP
            "#,
            values = values_sql
        );

        let mut mysql_query = sqlx::query::<MySql>(&sql);

        for hatyu in chunk {
            // ship_remain に何を入れるかは要件次第。
            // ここでは「現時点では order_qty をそのまま ship_remain に入れる」例にしてる。
            // もし remaining_shipping を計算済みにしたいなら Hatyu 側にフィールド追加するのが筋。
            let ship_remain: i32 = i32::try_from(hatyu.order_qty.0)
                .map_err(|_| AppError::BadRequest {
                    message: format!("order_qty too large for i32: {}", hatyu.order_qty.0),
                })?;

            // unit_price, order_date, created_at の型はあなたの Hatyu 定義に合わせて bind してください。
            // ここは “よくある型” を仮置きしています。
            mysql_query = mysql_query
                .bind(&hatyu.order_no.0)
                .bind(&hatyu.product_code.0)
                .bind(&hatyu.product_name.0)
                .bind(i32::try_from(hatyu.order_qty.0).map_err(|_| AppError::BadRequest {
                    message: format!("order_qty too large for i32: {}", hatyu.order_qty.0),
                })?)
                .bind(&hatyu.due_date)
                .bind(ship_remain)
                .bind(i32::try_from(hatyu.cumulative_order_qty.0).map_err(|_| AppError::BadRequest {
                    message: format!("cumulative_order_qty too large for i32: {}", hatyu.order_qty.0),
                })?);
        }

        mysql_query.execute(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;
    }

    Ok(())
}


fn compute_cumulative_order_qty(hatyus: &mut Vec<Hatyu>) {
    // (product_code, due_date) -> suffix sum (due_date 以上の合計)
    let mut product_code_due_date_suffix_sum: HashMap<(String, String), usize> = HashMap::new();

    // 1) product_code ごとに (due_date, order_qty) を集める
    let mut per_product: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    for hatyu in hatyus.iter() {
        let product_code = hatyu.product_code.0.clone();
        let due_date_str = hatyu.due_date.format("%Y%m%d").to_string();

        per_product.entry(product_code)
            .or_default()
            .push((due_date_str, hatyu.order_qty.0)
        );
    }

    // 2) 各 product_code について due_date 昇順に並べ、後ろから累積（suffix sum）を作る
    for (product_code, mut v) in per_product {
        v.sort_by(|a, b| a.0.cmp(&b.0)); // yyyy-MM-dd なので文字列比較でOK

        let mut acc: usize = 0;
        for (due_date, qty) in v.into_iter().rev() {
            acc += qty; // due_date 以上の合計になっていく
            product_code_due_date_suffix_sum.insert((product_code.clone(), due_date), acc);
        }
    }

    // 3) rows に列追加：その行の due_date 以上の合計
    for hatyu in hatyus.iter_mut() {
        let product_code = hatyu.product_code.0.clone();
        let due_date_str = hatyu.due_date.format("%Y%m%d").to_string();

        let suffix_sum = product_code_due_date_suffix_sum
            .get(&(product_code, due_date_str))
            .copied()
            .unwrap_or(0);

        hatyu.cumulative_order_qty = OrderQuantity(suffix_sum);
    }
}


#[command]
pub async fn insert_hatyu_into_card_input(
    app: AppHandle,
    sql: String,
    offset: usize,
    limit: usize
) -> AppResult<FetchResult> {
    run_blocking_async(move || async move {
        let mut context = get_sql_context();
        
        create_mysql_conn(
            &mut context,
            vec![
                "shipment_history".to_string(),
                "hatyu".to_string(),
                "card_input".to_string()
            ],
            vec![
                "shipment_history".to_string(),
                "hatyu".to_string(),
                "card_input".to_string()
            ]
        ).await?;

        let start = Utc::now();

        // 出荷済履歴クエリ
        let order_no_quantity_sum_map: HashMap<OrderNo, QuantitySum> =
            shipment_history_query(&mut context).await?;

        let hatyus = fetch_hatyu(
            &mut context,
            &order_no_quantity_sum_map
        ).await?;

        insert_into_card_input(&hatyus).await?;

        Ok(FetchResult {
            header: Vec::new(),
            rows: Vec::new(),
            query_time: time_difference_from_now(start),
        })
    }).await
}


#[command]
pub async fn fetch_card_input(
    app: AppHandle,
    sql: String,
    offset: usize,
    limit: usize
) -> AppResult<FetchResult> {
    run_blocking_async(move || async move {
        let mut context = get_sql_context();
        create_mysql_conn(
            &mut context,
            vec!["card_input".to_string()],
            vec!["card_input".to_string()]
        ).await?;

        let start = Utc::now();

        let sql = r#"
            SELECT
                *
            FROM card_input;
        "#.to_string();

        let (mut header, mut rows) = fetch_sql(&mut context, &sql).await?;
        
        Ok(FetchResult {
            header,
            rows,
            query_time: time_difference_from_now(start),
        })
    }).await
}


#[command]
pub async fn ship_by_product_code(
    app: AppHandle,
    product_code: String,
    ship_quantity: i32,
    ship_date: String,
    sql: String,
    offset: usize,
    limit: usize
) -> AppResult<FetchResult> {
    run_blocking_async(move || async move {
        let start = Utc::now();

        let pool = create_sqlx_mysql_pool()
            .await
            .map_err(|_| AppError::InternalServer {
                message: format!("failed to allocate pool"),
            })?;

        let hatyu_validation_sql = r#"
            SELECT
                *
            FROM card_input ci
            WHERE ci.product_code = ? AND
                ci.ship_remain - ? >= 0
        "#;

        let mut hatyu_validation_query = sqlx::query::<MySql>(&hatyu_validation_sql);

        hatyu_validation_query = hatyu_validation_query
            .bind(product_code.clone())
            .bind(ship_quantity.clone());
            
        let mysqlResult = hatyu_validation_query.fetch_all(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;

        if mysqlResult.len() == 0 {
            return Err(AppError::BadRequest {
                message: format!("出庫数が在庫数を上回っています: 出庫数: {ship_quantity}"),
            });
        }

        let shipment_history_sql = r#"
            INSERT INTO shipment_history (
                product_code,
                shipped_qty,
                issue_date,
                created_at
            )
            VALUES (?, ?, ?, now())
        "#;

        let hatyu_update_sql = r#"
            UPDATE card_input ci
            SET ci.ship_remain = GREATEST(ci.ship_remain - ?, 0)
            WHERE ci.product_code = ?
        "#;

        let mut shipment_history_query = sqlx::query::<MySql>(&shipment_history_sql);

        shipment_history_query = shipment_history_query
            .bind(product_code.clone())
            .bind(ship_quantity.clone())
            .bind(ship_date.clone());
            
        shipment_history_query.execute(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;

        let mut hatyu_update_query = sqlx::query::<MySql>(&hatyu_update_sql);

        hatyu_update_query = hatyu_update_query
            .bind(ship_quantity.clone())
            .bind(product_code.clone());
            
        hatyu_update_query.execute(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;

        Ok(FetchResult {
            header: Vec::new(),
            rows: Vec::new(),
            query_time: time_difference_from_now(start),
        })
    }).await
}

#[command]
pub async fn ship_by_order_no(
    app: AppHandle,
    order_no: i64,
    ship_quantity: i32,
    ship_date: String,
    sql: String,
    offset: usize,
    limit: usize
) -> AppResult<FetchResult> {
    run_blocking_async(move || async move {
        let start = Utc::now();

        let pool = create_sqlx_mysql_pool()
            .await
            .map_err(|_| AppError::InternalServer {
                message: format!("failed to allocate pool"),
            })?;

        let shipment_history_sql = r#"
            INSERT INTO shipment_history (
                order_no,
                shipped_qty,
                issue_date,
                created_at
            )
            VALUES (?, ?, ?, now())
        "#;

        let hatyu_update_sql = r#"
            UPDATE card_input ci
            SET ci.ship_remain = GREATEST(ci.ship_remain - ?, 0)
            WHERE ci.order_no = ?
        "#;

        let mut shipment_history_query = sqlx::query::<MySql>(&shipment_history_sql);

        shipment_history_query = shipment_history_query
            .bind(order_no)
            .bind(ship_quantity)
            .bind(ship_date);
            
        shipment_history_query.execute(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;

        let mut hatyu_update_query = sqlx::query::<MySql>(&hatyu_update_sql);

        hatyu_update_query = hatyu_update_query
            .bind(ship_quantity.clone())
            .bind(order_no.clone());
            
        hatyu_update_query.execute(&pool).await
            .map_err(|e| AppError::BadRequest {
                message: format!("insert card_input failed: {e}"),
            })?;

        Ok(FetchResult {
            header: Vec::new(),
            rows: Vec::new(),
            query_time: time_difference_from_now(start),
        })
    }).await
}

#[command]
pub async fn fetch(
    app: AppHandle,
    sql: String,
    offset: usize,
    limit: usize,
) -> AppResult<FetchResult> {
    run_blocking_async(move || async move {
        let start = Utc::now();
        let mut context = get_sql_context();

        let new_sql = register(&mut context, &sql, Some(limit), Some(offset))
            .await
            .map_err(|err| {
                let _ = insert_query_history(&app, &sql, "fail");
                err
            })?;

        let (header, records) = collect(&mut context, &new_sql).await.map_err(|err| {
            let _ = insert_query_history(&app, &sql, "fail");
            err
        })?;
        let width = header.len();

        // Pre-calculate the total number of rows to avoid frequent reallocation
        let total_rows: usize = records.iter().map(|r| r.num_rows()).sum();
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(total_rows);
        let options = FormatOptions::default().with_null("NULL");

        for record in records {
            let formatters = record
                .columns()
                .iter()
                .map(|c| ArrayFormatter::try_new(c.as_ref(), &options))
                .collect::<Result<Vec<_>, ArrowError>>()?;

            for row in 0..record.num_rows() {
                let mut cells = Vec::with_capacity(width);
                for (_, formatter) in formatters.iter().enumerate() {
                    cells.push(formatter.value(row).to_string());
                }
                rows.push(cells);
            }
        }

        insert_query_history(&app, &sql, "successful")?;

        Ok(FetchResult {
            header,
            rows,
            query_time: time_difference_from_now(start),
        })
    })
    .await
}

#[command]
pub async fn sql_history(app: AppHandle) -> AppResult<Vec<FetchHistory>> {
    run_blocking(move || {
        let conn = db_utils::conn(&app)?;
        let mut stmt = conn
            .prepare("select sql, status, created_at from sql_history order by id desc limit 50")?;

        let rows = stmt.query_map([], |row| {
            Ok(FetchHistory {
                sql: row.get(0)?,
                status: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut results = Vec::with_capacity(30);

        for row in rows {
            results.push(row?);
        }
        Ok(results)
    })
    .await
}

#[command]
pub async fn writer(
    _app: AppHandle,
    file_type: String,
    sql: String,
    table_name: Option<String>,
    max_values_per_insert: Option<usize>,
    sql_statement_type: Option<String>,
    where_column: Option<String>,
    dialect: Option<String>,
) -> AppResult<WriterResult> {
    run_blocking_async(move || async move {
        let mut downloads_dir = dirs::download_dir().ok_or_else(|| AppError::BadRequest {
            message: "Couldn't find the current working directory".to_string(),
        })?;

        let db_dialect = match dialect {
            Some(dialect) => Dialect::from_str(&dialect)?,
            None => Dialect::MySQL,
        };

        let start = Utc::now();

        // Validate required parameters for SQL export
        if file_type.to_lowercase() == "sql" {
            if table_name.is_none() {
                return Err(AppError::BadRequest {
                    message: "Table name is required for SQL export".to_string(),
                });
            }

            let statement_type = sql_statement_type
                .as_ref()
                .map(|s| s.to_uppercase())
                .unwrap_or_else(|| "INSERT".to_string());
            match statement_type.as_str() {
                "INSERT" => {
                    if max_values_per_insert.is_none() {
                        return Err(AppError::BadRequest {
                            message: "Max values per insert is required for INSERT statements"
                                .to_string(),
                        });
                    }
                }
                "UPDATE" => {
                    if where_column.is_none() {
                        return Err(AppError::BadRequest {
                            message: "WHERE column is required for UPDATE statements".to_string(),
                        });
                    }
                }
                _ => {
                    return Err(AppError::BadRequest {
                        message: "Invalid SQL statement type. Supported types: INSERT, UPDATE"
                            .to_string(),
                    });
                }
            }
        }

        let mut context = get_sql_context();
        let new_sql = register(&mut context, &sql, None, None).await?;
        let df = get_data_frame(&mut context, &new_sql).await?;

        // Determine file extension
        let file_extension = match file_type.to_lowercase().as_str() {
            "csv" => "csv",
            "tsv" => "tsv",
            "sql" => "sql",
            _ => {
                return Err(AppError::BadRequest {
                    message: "Unsupported file type. Supported types: csv, tsv, sql".to_string(),
                })
            }
        };

        downloads_dir.push(format!(
            "easydb_{}.{}",
            Utc::now().format("%Y%m%d%H%M%S").to_string(),
            file_extension
        ));
        let file_path = downloads_dir.to_string_lossy().to_string();

        match file_type.to_lowercase().as_str() {
            "csv" => {
                df.write_csv(&file_path, DataFrameWriteOptions::new(), None)
                    .await?;
            }
            "tsv" => {
                let mut options = CsvOptions::default();
                options.delimiter = b'\t';
                df.write_csv(&file_path, DataFrameWriteOptions::new(), None)
                    .await?;
            }
            "sql" => {
                // Generate SQL statements based on statement type
                let table_name_value = table_name.unwrap();
                let statement_type = sql_statement_type
                    .as_ref()
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "INSERT".to_string());

                let sql_content = match statement_type.as_str() {
                    "INSERT" => {
                        let max_values = max_values_per_insert.unwrap();
                        generate_sql_inserts(df, &table_name_value, max_values, &db_dialect).await?
                    }
                    "UPDATE" => {
                        let where_column_value = where_column.unwrap();
                        generate_sql_update(df, &table_name_value, &where_column_value, &db_dialect)
                            .await?
                    }
                    _ => {
                        return Err(AppError::BadRequest {
                            message: "Invalid SQL statement type".to_string(),
                        });
                    }
                };

                let mut file = File::create(&downloads_dir)?;
                write!(file, "{}", sql_content)?;
            }
            _ => unreachable!(), // This case is handled above
        }

        Ok(WriterResult {
            query_time: time_difference_from_now(start),
            file_name: fs::canonicalize(&downloads_dir)?.display().to_string(),
        })
    })
    .await
}
