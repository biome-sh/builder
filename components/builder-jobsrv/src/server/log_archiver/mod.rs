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

//! Contract for storage and retrieval of job logs from long-term
//! storage.
//!
//! As jobs are running, their log output is collected in files on the
//! job server. Once they are complete, however, we would like to
//! store them elsewhere for safety; the job server should be
//! stateless.

pub mod local;
pub mod s3;

use crate::{config::ArchiveCfg,
            error::Result};
use async_trait::async_trait;
use std::path::Path;

/// Currently implemented log archiving backends
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveBackend {
    Local,
    S3,
}

#[async_trait]
pub trait LogArchiver: Send {
    /// Given a `job_id` and the path to the log output for that job,
    /// places the log in an archive for long-term storage.
    async fn archive(&self, job_id: u64, file_path: &Path) -> Result<()>;

    /// Given a `job_id`, retrieves the log output for that job from
    /// long-term storage.
    async fn retrieve(&self, job_id: u64) -> Result<Vec<String>>;
}

/// Create appropriate LogArchiver variant based on configuration values.
pub fn from_config(config: &ArchiveCfg) -> Result<Box<dyn LogArchiver>> {
    match config.backend {
        ArchiveBackend::Local => Ok(Box::new(local::LocalArchiver::new(config))),
        ArchiveBackend::S3 => Ok(Box::new(s3::S3Archiver::new(config))),
    }
}
