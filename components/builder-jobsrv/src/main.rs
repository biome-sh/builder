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

use builder_core::config::ConfigFile;
use biome_builder_jobsrv as jobsrv;

use std::{error,
          process};

use crate::jobsrv::{Config,
                    Result};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));
const CFG_DEFAULT_PATH: &str = "/hab/svc/builder-jobsrv/config/config.toml";

#[actix_rt::main]
async fn main() {
    env_logger::init();
    let matches = app().get_matches();
    debug!("CLI matches: {:?}", matches);
    let (subcmd, config) = match subcmd_and_config_from_args(&matches) {
        Ok((s, c)) => (s, c),
        Err(e) => return exit_with(&e, 1),
    };

    match subcmd {
        "migrate" => {
            match jobsrv::server::migrate(&config) {
                Ok(_) => process::exit(0),
                Err(e) => exit_with(&e, 1),
            }
        }
        "start" => {
            match jobsrv::server::run(config).await {
                Ok(_) => process::exit(0),
                Err(e) => exit_with(&e, 1),
            }
        }
        _ => unreachable!(),
    }
}

fn app<'a, 'b>() -> clap::App<'a, 'b> {
    clap_app!(BuilderJobSrv =>
        (version: VERSION)
        (about: "Biome builder-jobsrv")
        (@setting VersionlessSubcommands)
        (@setting SubcommandRequiredElseHelp)
        (@subcommand migrate =>
            (about: "Run database migrations")
            (@arg config: -c --config +takes_value +global
                "Filepath to configuration file. [default: /hab/svc/builder-api/config/config.toml]")
        )
        (@subcommand start =>
            (about: "Run a Biome Builder job server")
            (@arg config: -c --config +takes_value
                "Filepath to configuration file. [default: /hab/svc/builder-jobsrv/config/config.toml]")
        )
    )
}

fn subcmd_and_config_from_args<'a>(matches: &'a clap::ArgMatches) -> Result<(&'a str, Config)> {
    let cmd = matches.subcommand_name().unwrap();
    let args = matches.subcommand_matches(cmd).unwrap();
    let config = match args.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path)?,
        None => Config::from_file(CFG_DEFAULT_PATH).unwrap_or_default(),
    };
    Ok((cmd, config))
}

fn exit_with<T>(err: &T, code: i32)
    where T: error::Error
{
    println!("{}", err);
    process::exit(code)
}
