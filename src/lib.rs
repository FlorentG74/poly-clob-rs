pub mod controller;
pub mod model;

#[cfg(test)]
mod core_lib_tests {
    use crate::controller::{ConfigProvider, DotEnvConfigProvider};


    #[test]
    fn test_config() {
        let config_provider = DotEnvConfigProvider::new();

        let poly_address = &config_provider.get_config().poly_address;

        assert_ne!(poly_address, "");
    }
}
