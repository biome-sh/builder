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

//! Centralized definition of all Builder Worker metrics that we wish
//! to track.

use std::borrow::Cow;

use crate::bldr_core::metrics;

pub type InstallationId = u32;

pub enum Counter {
    GitClone,
    GitAuthenticatedClone,
}

impl metrics::CounterMetric for Counter {}

impl metrics::Metric for Counter {
    fn id(&self) -> Cow<'static, str> {
        match *self {
            Counter::GitAuthenticatedClone => "github.authenticated_clone".to_string().into(),
            Counter::GitClone => "github.clone".to_string().into(),
        }
    }
}
