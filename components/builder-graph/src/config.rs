// Biome project based on Chef Habitat's code (c) 2016-2020 Chef Software, Inc
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Configuration for a Biome Scheduler service

use crate::{db::config::DataStoreCfg,
            error::Error};
use builder_core::config::ConfigFile;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub datastore:        DataStoreCfg,
    pub features_enabled: String,
}

impl Default for Config {
    fn default() -> Self {
        let datastore = DataStoreCfg { database: String::from("builder"),
                                       ..Default::default() };
        Config { datastore,
                 features_enabled: String::from("builddeps") }
    }
}

impl ConfigFile for Config {
    type Error = Error;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_file() {
        let content = r#"
        features_enabled = "builddeps"

        [datastore]
        host = "1.1.1.1"
        port = 9000
        user = "test"
        database = "test_jobsrv"
        connection_retry_ms = 500
        connection_timeout_sec = 4800
        connection_test = true
        pool_size = 1
        "#;

        let config = Config::from_raw(content).unwrap();
        assert_eq!(config.datastore.port, 9000);
        assert_eq!(config.datastore.user, "test");
        assert_eq!(config.datastore.database, "test_jobsrv");
        assert_eq!(config.datastore.connection_retry_ms, 500);
        assert_eq!(config.datastore.connection_timeout_sec, 4800);
        assert!(config.datastore.connection_test);
        assert_eq!(config.datastore.pool_size, 1);
    }

    #[test]
    fn config_from_file_defaults() {
        let content = r#"
        "#;

        let config = Config::from_raw(content).unwrap();
        assert_eq!(config.datastore.database, String::from("builder"));
    }
}
