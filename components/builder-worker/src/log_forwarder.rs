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

use std::{sync::mpsc,
          thread::{self,
                   JoinHandle},
          time::Duration};

use crate::{bldr_core::{logger::Logger,
                        socket::DEFAULT_CONTEXT},
            config::Config,
            error::{Error,
                    Result}};

/// In-memory zmq address for LogForwarder
pub const INPROC_ADDR: &str = "inproc://logger";

pub struct LogForwarder {
    /// The socket on which log data is received from workers.
    pub intake_sock: zmq::Socket,
    /// The socket from which log data is forwarded to the appropriate
    /// job server.
    pub output_sock: zmq::Socket,
    /// Log file for debugging this process.
    logger:          Logger,
}

impl LogForwarder {
    pub fn new(config: &Config) -> Self {
        let intake_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::PULL).unwrap();
        let output_sock = (**DEFAULT_CONTEXT).as_mut().socket(zmq::DEALER).unwrap();
        output_sock.set_sndhwm(5000).unwrap();
        output_sock.set_linger(5000).unwrap();
        output_sock.set_immediate(true).unwrap();

        let mut logger = Logger::init(&config.log_path, "log_forwarder.log");
        logger.log_ident("log_forwarder");

        LogForwarder { intake_sock,
                       output_sock,
                       logger }
    }

    pub fn start(config: &Config) -> Result<JoinHandle<()>> {
        let (tx, rx) = mpsc::sync_channel(0);
        let mut log = Self::new(config);
        let jobsrv_addrs = config.jobsrv_addrs();
        let handle = thread::Builder::new().name("log".to_string())
                                           .spawn(move || {
                                               log.run(&tx, &jobsrv_addrs).unwrap();
                                           })
                                           .unwrap();
        match rx.recv() {
            Ok(()) => Ok(handle),
            Err(e) => panic!("log thread startup error, err={}", e),
        }
    }

    pub fn run(&mut self,
               rz: &mpsc::SyncSender<()>,
               addrs: &[(String, String, String)])
               -> Result<()> {
        if addrs.len() == 1 {
            let (_, _, ref log) = addrs[0];
            println!("Connecting to Job Server Log port, {}", log);
            self.output_sock.connect(log)?;
        } else {
            warn!("Routing logs to more than one Job Server is not yet implemented");
        }

        self.logger.log("Startup complete");
        self.intake_sock.bind(INPROC_ADDR)?;

        // Signal back to the spawning process that we're good
        rz.send(()).unwrap();

        // This hacky sleep is recommended and required by zmq for connections to establish
        thread::sleep(Duration::from_millis(100));

        self.logger
            .log("Starting proxy between log_pipe and jobsrv");

        // If we ever have multiple JobServers these need to be sent to, then we might need some
        // additional logic.
        if let Err(e) = zmq::proxy(&self.intake_sock, &self.output_sock) {
            self.logger
                .log(format!("ZMQ proxy returned an error: {:?}", e).as_ref());
            return Err(Error::Zmq(e));
        }
        Ok(())
    }
}
