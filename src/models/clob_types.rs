pub enum AssetType {
    COLLATERAL,
    CONDITIONAL,
}

impl From<AssetType> for &'static str {
    fn from(a: AssetType) -> &'static str {
        match a {
            AssetType::COLLATERAL => "COLLATERAL",
            AssetType::CONDITIONAL => "CONDITIONAL",
        }
    }
}
