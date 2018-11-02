// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate base64;
extern crate chrono;
extern crate dogstatsd;
extern crate glob;
extern crate habitat_builder_protocol as protocol;
extern crate habitat_core as hab_core;
extern crate habitat_net as hab_net;
#[macro_use]
extern crate log;
extern crate petgraph;
extern crate protobuf;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate reqwest;
#[allow(unused_imports)]
#[macro_use]
extern crate serde_json;
extern crate time;
extern crate toml;
extern crate walkdir;
#[macro_use]
extern crate hyper;

pub mod access_token;
pub mod api_client;
pub mod build_config;
pub mod data_structures;
pub mod error;
pub mod integrations;
pub mod job;
pub mod keys;
pub mod logger;
pub mod metrics;
pub mod package_graph;
pub mod rdeps;
pub mod rpc;
pub mod target_graph;

pub use error::Error;
