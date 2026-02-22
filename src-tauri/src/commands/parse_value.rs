use std::{fmt::Display, str::FromStr};

use crate::context::{error::AppError, schema::AppResult};

/// セル文字列 -> 型T への変換を統一するtrait
pub trait ParseValue: Sized {
    fn parse_value(s: &str) -> AppResult<Self>;
}

/// 基本型は FromStr があれば ParseValue に載せる
impl<T> ParseValue for T
where
    T: FromStr,
    T::Err: Display,
{
    fn parse_value(s: &str) -> AppResult<Self> {
        s.parse::<T>().map_err(|e| AppError::BadRequest {
            message: format!("failed to parse '{s}': {e}"),
        })
    }
}

pub struct RowOwned {
    pub cells: Vec<String>,
}

impl RowOwned {
    pub fn get_as<T: ParseValue>(&self, idx: usize, col: &'static str) -> AppResult<T> {
        let s = self.cells.get(idx).ok_or_else(|| AppError::BadRequest {
            message: format!("missing column[{idx}] '{col}'"),
        })?;
        T::parse_value(s)
    }
}

pub trait FromRowOwned: Sized {
    fn from_row(row: &RowOwned) -> AppResult<Self>;
}

pub fn map_rows_owned<T: FromRowOwned>(rows: Vec<Vec<String>>) -> AppResult<Vec<T>> {
    let owned: Vec<RowOwned> = rows.into_iter().map(|cells| RowOwned { cells }).collect();
    owned.iter().map(T::from_row).collect()
}
