use chrono::NaiveDate;

use crate::{
    commands::parse_value::{FromRowOwned, ParseValue, RowOwned},
    context::schema::AppResult
};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderNo(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuantitySum(pub usize);


pub struct ShipmentHistoryByOrderNo {
    pub order_no: OrderNo,
    /// quantity_sum (数量合計)
    pub quantity_sum: QuantitySum    
}

impl ParseValue for OrderNo {
    fn parse_value(s: &str) -> AppResult<Self> { Ok(OrderNo(s.to_string())) }
}

/// 数量はパースして包む
impl ParseValue for QuantitySum {
    fn parse_value(s: &str) -> AppResult<Self> { Ok(QuantitySum(usize::parse_value(s)?)) }
}

impl FromRowOwned for ShipmentHistoryByOrderNo {
    fn from_row(row: &RowOwned) -> AppResult<Self> {
        Ok(Self {
            order_no: row.get_as(0, "order_no")?,
            quantity_sum: row.get_as(1, "quantity_sum")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProductCode(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductName(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderQuantity(pub usize);

impl ParseValue for ProductCode {
    fn parse_value(s: &str) -> AppResult<Self> { Ok(ProductCode(s.to_string())) }
}
impl ParseValue for ProductName {
    fn parse_value(s: &str) -> AppResult<Self> { Ok(ProductName(s.to_string())) }
}

impl ParseValue for OrderQuantity {
    fn parse_value(s: &str) -> AppResult<Self> { Ok(OrderQuantity(usize::parse_value(s)?)) }
}

#[derive(Debug, Clone)]
pub struct Hatyu {
    pub order_no: OrderNo,
    pub product_code: ProductCode,
    pub product_name: ProductName,
    pub order_qty: OrderQuantity,
    pub due_date: NaiveDate,
    pub cumulative_order_qty: OrderQuantity,
    // unit_price / order_date etc もここに追加
}

impl FromRowOwned for Hatyu {
    fn from_row(row: &RowOwned) -> AppResult<Self> {
        Ok(Self {
            order_no: row.get_as(0, "order_no")?,
            product_code: row.get_as(1, "product_code")?,
            product_name: row.get_as(2, "product_name")?,
            order_qty: row.get_as(3, "order_qty")?,
            due_date: row.get_as(4, "due_date")?,
            // すでにテーブルに入っていたら取得、入っていなかったら0
            cumulative_order_qty: row.get_as(5, "cumulative_order_qty")
                .unwrap_or(OrderQuantity(0)),
        })
    }
}
