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

use std::path::Path;

use super::publisher::Publisher;
use crate::{config::Config,
            error::Error,
            bio_core::ChannelIdent};

use builder_core::config::ConfigFile;

// TODO (SA) - Toml-based publishing has been removed, and is not hooked up to
// the post-processor currently. Keeping the code around to re-enable
// at some point in the future.

/// Postprocessing config file name
#[allow(dead_code)]
const CONFIG_FILE: &str = "builder.toml";

#[derive(Default, Deserialize, Debug)]
#[serde(default)]
struct TomlPublishBuilder {
    publish: TomlPublish,
}

#[derive(Default, Deserialize, Debug)]
#[serde(default)]
struct TomlPublish {
    enabled: Option<bool>,
    url:     Option<String>,
    channel: Option<ChannelIdent>,
}

impl TomlPublishBuilder {
    #[allow(dead_code)]
    fn new(toml_path: &Path) -> Self {
        if toml_path.exists() {
            debug!("Found toml config at {}", toml_path.display());
            TomlPublishBuilder::from_file(toml_path).unwrap()
        } else {
            TomlPublishBuilder::default()
        }
    }

    #[allow(dead_code)]
    fn build(self, config: &Config) -> Publisher {
        Publisher { enabled:     self.publish.enabled.unwrap_or(config.auto_publish),
                    url:         self.publish.url.unwrap_or_else(|| config.bldr_url.clone()),
                    channel_opt: self.publish.channel, }
    }
}

impl ConfigFile for TomlPublishBuilder {
    type Error = Error;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_config_from_toml() {
        let toml = r#"
        [publish]
        enabled = false
        url = "https://bldr.habitat.sh"
        channel = "unstable"
        "#;

        let config = Config::default();
        let cfg = TomlPublishBuilder::from_raw(toml).unwrap().build(&config);
        assert_eq!("https://bldr.habitat.sh", cfg.url);
        assert!(!cfg.enabled);
        assert_eq!(Some(ChannelIdent::unstable()), cfg.channel_opt);
    }
}
