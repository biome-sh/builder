use biome_builder_events::connection::EventConfig;
use biome_core::config::ConfigFile;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub eventbus: EventConfig,
}

impl ConfigFile for Config {
    type Error = biome_core::Error;
}
