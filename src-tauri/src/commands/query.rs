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
    sqlx::query(sql).execute(pool).await.expect("SQL execute failed");
    
    Ok(())
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
        let related_tables = vec![
            "hatyu".to_string(),
            "card_input".to_string(),
            "shipment_history".to_string()
        ];

        let pool = create_sqlx_mysql_pool().await
            .expect("invalid mysql pool");

        let start = Utc::now();

        let sql = r#"
            INSERT INTO card_input (
                order_id,
                product_code,
                order_qty,
                due_date,
                provisional_issued,
                shipped_flg,
                order_date,
                created_at
            )
            SELECT
                h.order_no                               AS order_id,
                h.product_code                           AS product_code,

                /* 出荷残 = 発注数 - 出荷済数量合計 */
                (h.order_qty - IFNULL(s.shipped_qty, 0)) AS order_qty,

                h.due_date                               AS due_date,
                FALSE                                    AS provisional_issued,
                FALSE                                    AS shipped_flg,
                h.order_created_date                     AS order_date,
                CURRENT_TIMESTAMP                        AS created_at
            FROM hatyu h
            LEFT JOIN (
                SELECT
                    sh.order_no,
                    SUM(sh.qty) AS shipped_qty
                FROM shipment_history sh
                GROUP BY sh.order_no
            ) s
                ON s.order_no = h.order_no
            WHERE
                /* 未完了（出荷残がある）注文のみカード化 */
                (h.order_qty - IFNULL(s.shipped_qty, 0)) > 0;
        "#.to_string();

        execute_sql(&pool, &sql).await?;

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

        let future_order_cumulative_sql = r#"
            SELECT
                product_code,
                due_date,
                order_qty
            FROM card_input c
            WHERE c.shipped_flg = 0
            ORDER BY due_date;
        "#.to_string();

        let (mut header, mut rows) = fetch_sql(&mut context, &sql).await?;
        let (header_cumulative, rows_cumulative) = fetch_sql(&mut context, &future_order_cumulative_sql).await?;
        let mut product_code_order_qty_map: HashMap<String, i32> = HashMap::new();

        // (product_code, due_date) -> suffix sum (due_date 以上の合計)
        let mut product_code_due_date_suffix_sum: HashMap<(String, String), i32> = HashMap::new();

        // 1) product_code ごとに (due_date, order_qty) を集める
        let mut per_product: HashMap<String, Vec<(String, i32)>> = HashMap::new();

        for row in rows_cumulative {
            let product_code: String = row
                .get(0)
                .expect("invalid product_code")
                .clone();

            let due_date: String = row
                .get(1)
                .expect("invalid due_date")
                .clone();

            let order_qty: i32 = row
                .get(2)
                .expect("invalid order_qty")
                .parse()
                .expect("order_qty is not i32");

            per_product.entry(product_code).or_default().push((due_date, order_qty));
        }

        // 2) 各 product_code について due_date 昇順に並べ、後ろから累積（suffix sum）を作る
        for (product_code, mut v) in per_product {
            v.sort_by(|a, b| a.0.cmp(&b.0)); // yyyy-MM-dd なので文字列比較でOK

            let mut acc: i32 = 0;
            for (due_date, qty) in v.into_iter().rev() {
                acc += qty; // due_date 以上の合計になっていく
                product_code_due_date_suffix_sum.insert((product_code.clone(), due_date), acc);
            }
        }

        // 3) rows に列追加：その行の due_date 以上の合計
        for row in &mut rows {
            let product_code: String = row
                .get(1)
                .expect("invalid product_code")
                .clone();

            let due_date: String = row
                .get(3)
                .expect("invalid due_date")
                .clone();

            let suffix_sum = product_code_due_date_suffix_sum
                .get(&(product_code.clone(), due_date.clone()))
                .copied()
                .unwrap_or(0);

            row.push(suffix_sum.to_string());
        }

        header.push("cumulative_order_qty".to_string());

        Ok(FetchResult {
            header,
            rows,
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
