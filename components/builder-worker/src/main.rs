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

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::process;

use builder_core::config::ConfigFile;
use biome_builder_worker as worker;

use crate::worker::{server,
                    Config,
                    Error,
                    Result};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));
const CFG_DEFAULT_PATH: &str = "/hab/svc/builder-worker/config/config.toml";

fn main() {
    env_logger::init();
    let matches = app().get_matches();
    debug!("CLI matches: {:?}", matches);
    let config = match config_from_args(&matches) {
        Ok(result) => result,
        Err(e) => return exit_with(&e, 1),
    };
    match start(config) {
        Ok(_) => std::process::exit(0),
        Err(e) => exit_with(&e, 1),
    }
}

fn app<'a, 'b>() -> clap::App<'a, 'b> {
    clap_app!(BuilderWorker =>
        (version: VERSION)
        (about: "Biome builder-worker")
        (@setting VersionlessSubcommands)
        (@setting SubcommandRequiredElseHelp)
        (@subcommand start =>
            (about: "Run a Biome-Builder worker")
            (@arg config: -c --config +takes_value +global
                "Filepath to configuration file. \
                [default: /hab/svc/builder-worker/config/config.toml]")
        )
    )
}

fn config_from_args(matches: &clap::ArgMatches) -> Result<Config> {
    let cmd = matches.subcommand_name().unwrap();
    let args = matches.subcommand_matches(cmd).unwrap();
    let config = match args.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path)?,
        None => Config::from_file(CFG_DEFAULT_PATH).unwrap_or_default(),
    };
    Ok(config)
}

fn exit_with(err: &Error, code: i32) {
    println!("{}", err);
    process::exit(code)
}

/// Starts the builder-worker.
fn start(config: Config) -> Result<()> { server::run(config) }
