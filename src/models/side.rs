use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(feature = "with-diesel")]
use diesel::deserialize::{self, FromSql};
#[cfg(feature = "with-diesel")]
use diesel::pg::Pg;
#[cfg(feature = "with-diesel")]
use diesel::serialize::{self, Output, ToSql};
#[cfg(feature = "with-diesel")]
use diesel::sql_types::Integer;
#[cfg(feature = "with-diesel")]
use diesel::{AsExpression, FromSqlRow};

/// Order side: Buy or Sell
///
/// Represents the direction of an order on the Polymarket CLOB.
///
/// # Integer Representation
/// - Buy = 0
/// - Sell = 1
///
/// # String Representation
/// - Uppercase: "BUY" / "SELL" (default serialization)
/// - Lowercase: "buy" / "sell" (accepted via aliases)
///
/// # Examples
///
/// ```
/// use poly_clob_rs::Side;
///
/// // Create from integer
/// let buy = Side::from(0);
/// assert_eq!(buy, Side::Buy);
///
/// // Convert to integer
/// let sell_int: i32 = Side::Sell.into();
/// assert_eq!(sell_int, 1);
///
/// // Display
/// assert_eq!(format!("{}", Side::Buy), "BUY");
///
/// // Parse from string
/// let side = Side::try_from("sell").unwrap();
/// assert_eq!(side, Side::Sell);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "with-diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "with-diesel", diesel(sql_type = Integer))]
pub enum Side {
    /// Buy order (integer value: 0)
    #[serde(alias = "buy", alias = "BUY")]
    Buy,
    /// Sell order (integer value: 1)
    #[serde(alias = "sell", alias = "SELL")]
    Sell,
}

impl Side {
    /// Convert to integer (0 = Buy, 1 = Sell) for API compatibility
    ///
    /// # Examples
    ///
    /// ```
    /// use poly_clob_rs::Side;
    ///
    /// assert_eq!(Side::Buy.to_int(), 0);
    /// assert_eq!(Side::Sell.to_int(), 1);
    /// ```
    pub fn to_int(self) -> i32 {
        match self {
            Side::Buy => 0,
            Side::Sell => 1,
        }
    }

    /// Convert to lowercase string for price API
    ///
    /// # Examples
    ///
    /// ```
    /// use poly_clob_rs::Side;
    ///
    /// assert_eq!(Side::Buy.to_lowercase_str(), "buy");
    /// assert_eq!(Side::Sell.to_lowercase_str(), "sell");
    /// ```
    pub fn to_lowercase_str(self) -> &'static str {
        match self {
            Side::Buy => "buy",
            Side::Sell => "sell",
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

impl From<i32> for Side {
    /// Convert integer to Side (0 = Buy, any other value = Sell)
    fn from(value: i32) -> Self {
        match value {
            0 => Side::Buy,
            _ => Side::Sell,
        }
    }
}

impl TryFrom<&str> for Side {
    type Error = String;

    /// Parse a string into a Side (case-insensitive)
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not "buy", "BUY", "sell", or "SELL"
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "BUY" => Ok(Side::Buy),
            "SELL" => Ok(Side::Sell),
            _ => Err(format!("Invalid side: {}", value)),
        }
    }
}

impl From<Side> for i32 {
    /// Convert Side to integer
    fn from(side: Side) -> i32 {
        side.to_int()
    }
}

// Diesel support (optional feature)
#[cfg(feature = "with-diesel")]
impl ToSql<Integer, Pg> for Side {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let value = self.to_int();
        <i32 as ToSql<Integer, Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

#[cfg(feature = "with-diesel")]
impl FromSql<Integer, Pg> for Side {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let value = <i32 as FromSql<Integer, Pg>>::from_sql(bytes)?;
        Ok(Side::from(value))
    }
}
