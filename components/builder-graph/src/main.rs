// Biome project based on Chef Habitat's code © 2016-2020 Chef Software, Inc
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

#[macro_use]
extern crate bitflags;
use clap::clap_app;
#[macro_use]
extern crate features;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

extern crate diesel;

use biome_builder_db as db;
use biome_core as bio_core;

pub mod config;
pub mod data_store;
pub mod error;
pub mod ident_graph;
pub mod package_graph;
pub mod rdeps;
pub mod util;

use std::{collections::HashMap,
          fs::File,
          io::Write,
          iter::FromIterator,
          str::FromStr,
          time::Instant};

use clap::{App,
           AppSettings,
           Arg,
           ArgMatches};
use copperline::Copperline;

use crate::{config::Config,
            data_store::DataStore,
            bio_core::{config::ConfigFile,
                       package::{PackageIdent,
                                 PackageTarget}},
            package_graph::PackageGraph};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

struct State {
    datastore: DataStore,
    graph:     PackageGraph,
    filter:    String,
    done:      bool,
    cli:       clap::App<'static, 'static>,
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    env_logger::init();

    let matches =
        App::new("bldr-graph").version(VERSION)
                              .about("Biome Graph Dev Tool")
                              .arg(Arg::with_name("config").help("Filepath to configuration file")
                                                           .required(false)
                                                           .index(1))
                              .arg(Arg::with_name("internal_command").multiple(true)
                                                                     .last(true)
                                                                     .help("Internal CLI command \
                                                                            to run"))
                              .get_matches();

    let config = match matches.value_of("config") {
        Some(cfg_path) => Config::from_file(cfg_path).unwrap(),
        None => Config::default(),
    };

    let external_args: Vec<&str> = match matches.values_of("internal_command") {
        Some(values) => values.collect(),
        None => Vec::<&str>::new(),
    };

    enable_features(&config);

    let mut cl = Copperline::new();

    println!("Connecting to {}", config.datastore.database);

    let datastore = DataStore::new(&config);
    datastore.setup().unwrap();

    println!("Building graph... please wait.");

    let mut graph = PackageGraph::new();
    let start_time = Instant::now();
    let packages = datastore.get_job_graph_packages().unwrap();

    let fetch_time = start_time.elapsed().as_secs_f64();
    println!("OK: fetched {} packages ({} sec)",
             packages.len(),
             fetch_time);

    let start_time = Instant::now();
    let (ncount, ecount) = graph.build(packages.into_iter(), feat::is_enabled(feat::BuildDeps));
    println!("OK: {} nodes, {} edges ({} sec)",
             ncount,
             ecount,
             start_time.elapsed().as_secs_f64());

    let targets = graph.targets();
    let target_as_string: Vec<String> = targets.iter().map(|t| t.to_string()).collect();

    println!("Found following targets {}", target_as_string.join(", "));
    println!("Default target is {}", graph.current_target());

    let mut state = State { datastore,
                            graph,
                            filter: String::from(""),
                            done: false,
                            cli: make_clap_cli() };

    // This is meant to ease testing of this command and provide a quick one-off access to the CLI
    //
    if !external_args.is_empty() {
        state.process_command(&external_args);
        state.done = true
    } else {
        // Interactive mode
        state.cli.print_help().unwrap();

        while !state.done {
            let prompt = format!("{}: command> ", state.graph.current_target());
            let line = cl.read_line_utf8(&prompt);
            if let Ok(cmd) = line {
                cl.add_history(cmd.clone());

                let v: Vec<&str> = cmd.trim_end().split_whitespace().collect();

                if !v.is_empty() {
                    state.process_command(&v);
                }
            } else {
                line.expect("Could not get line");
            }
        }
    }
}

impl State {
    fn process_command(&mut self, v: &[&str]) {
        let match_result = self.cli.get_matches_from_safe_borrow(v);

        match match_result {
            Ok(matches) => {
                match matches.subcommand() {
                    ("help", _) => do_help(&matches, &mut self.cli), /* This doesn't work; goes
                                                                       * strait Something */
                    // is eating the help output
                    ("build_levels", Some(m)) => do_build_levels(&self.graph, &m),
                    ("check", Some(m)) => do_check(&self.datastore, &self.graph, &m),
                    ("db_deps", Some(m)) => do_db_deps(&self.datastore, &self.graph, &m),
                    ("deps", Some(m)) => do_deps(&self.graph, &m),
                    ("dot", Some(m)) => do_dot(&self.graph, &m),
                    ("export", Some(m)) => do_export(&self.graph, &m),
                    ("filter", Some(m)) => self.filter = do_filter(&m),
                    ("find", Some(m)) => do_find(&self.graph, &m),
                    ("quit", _) => self.done = true,
                    ("raw", Some(m)) => do_raw(& self.graph, &m),
                    ("rdeps", Some(m)) => do_rdeps(& self.graph, &self.filter, &m),
                    ("resolve", Some(m)) => do_resolve(& self.graph, &m),
                    ("scc", Some(m)) => do_scc(&self.graph, &m),
                    ("stats", Some(m)) => do_stats(& self.graph, &m),
                    ("target", Some(m)) => do_target(&mut self.graph, &m),
                    ("top", Some(m)) => do_top(&self.graph, &m),
                    name => debug!("Unknown  {:?} {:?}", matches, name),
                }
            }
            // Ideally we'd match the various errors and do something more
            // clever, e.g. Err(HelpDisplayed) => self.cli.print_help(UNKNOWN_ARGUMENTS)
            // But I've not totally figured that out yet.
            Err(x) => {
                debug!("ClapError {:?} {:?}", x.kind, x);
                debug!("ClapError Msg: {}", x.message);
                debug!("ClapError Info: {:?}", x.info);
            }
        }
    }
}

fn do_help(_matches: &ArgMatches, cli: &mut clap::App<'static, 'static>) {
    cli.print_long_help().unwrap(); // print_help might be more usable
}

fn do_stats(graph: &PackageGraph, matches: &ArgMatches) {
    if matches.is_present("ALL") {
        do_all_stats_sub(graph);
    } else {
        do_stats_sub(graph);
    }
}

fn do_all_stats_sub(graph: &PackageGraph) {
    let stats = graph.all_stats();
    for (target, stat) in stats {
        println!("Target: {}", target);
        println!("  Node count: {}", stat.node_count);
        println!("  Edge count: {}", stat.edge_count);
        println!("  Connected components: {}", stat.connected_comp);
        println!("  Is cyclic: {}", stat.is_cyclic)
    }
}

fn do_stats_sub(graph: &PackageGraph) {
    let stats = graph.stats();

    println!("Node count: {}", stats.node_count);
    println!("Edge count: {}", stats.edge_count);
    println!("Connected components: {}", stats.connected_comp);
    println!("Is cyclic: {}", stats.is_cyclic);
}

fn do_top(graph: &PackageGraph, matches: &ArgMatches) {
    let count = count_from_matches(matches).unwrap();
    let start_time = Instant::now();
    let top = graph.top(count);

    println!("OK: {} items ({} sec)\n",
             top.len(),
             start_time.elapsed().as_secs_f64());

    for (name, count) in top {
        println!("{}: {}", name, count);
    }
    println!();
}

fn do_filter(matches: &ArgMatches) -> String {
    let filter = filter_from_matches(matches);
    if filter.is_empty() {
        println!("Removed filter\n");
    } else {
        println!("New filter: {}\n", filter);
    }
    filter
}

fn do_find(graph: &PackageGraph, matches: &ArgMatches) {
    let phrase = search_from_matches(matches);
    let max = count_from_matches(matches).unwrap(); // WIP Rework command loop to handle result
    let start_time = Instant::now();
    let mut v = graph.search(&phrase);

    println!("OK: {} items ({} sec)\n",
             v.len(),
             start_time.elapsed().as_secs_f64());

    if v.is_empty() {
        println!("No matching packages found")
    } else {
        if v.len() > max {
            v.drain(max..);
        }
        for s in v {
            println!("{}", s);
        }
    }
    println!();
}

fn do_dot(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);
    let graph_type;
    if matches.is_present("LATEST") {
        graph_type = "latest";
        graph.dump_latest_graph_as_dot(&filename, origin.as_deref());
    } else {
        graph_type = "current";
        if let Some(o) = origin {
            println!("Origin {} ignored", o)
        }
        graph.emit_graph(&filename, None, true, None);
    }
    let duration_secs = start_time.elapsed().as_secs_f64();

    println!("Wrote {} graph to file {} filtered by {:?} TBI in {} sec",
             graph_type, filename, origin, duration_secs);
}

fn do_raw(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);
    let graph_type;
    if matches.is_present("LATEST") {
        graph_type = "latest";
        graph.dump_latest_graph_raw(&filename, origin.as_deref());
    } else {
        graph_type = "current";
        if let Some(o) = origin {
            println!("Origin {} ignored", o)
        }
        println!("Raw graph dump TBI for full graph");
        // graph.emit_graph_raw(&filename, None, true, None);
    }
    let duration_secs = start_time.elapsed().as_secs_f64();

    println!("Wrote {} raw graph to file {} filtered by {:?} TBI in {} sec",
             graph_type, filename, origin, duration_secs);
}

fn do_scc(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);

    graph.dump_scc(filename, origin);
    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote SCC of latest information to file {} filtered by {:?} TBI in {} sec",
             filename, origin, duration_secs);
}

fn do_build_levels(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let origin = origin_from_matches(matches);
    let filename = required_filename_from_matches(matches);
    graph.dump_build_levels(filename, origin);
    let duration_secs = start_time.elapsed().as_secs_f64();
    println!("Wrote Build levels information to file {} filtered by {:?} TBI in {} sec",
             filename, origin, duration_secs);
}

fn do_resolve(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let ident = ident_from_matches(matches).unwrap();
    let result = graph.resolve(&ident);
    println!("OK: ({} sec)\n", start_time.elapsed().as_secs_f64());

    match result {
        Some(s) => println!("{}", s),
        None => println!("No matching packages found"),
    }

    println!();
}

fn do_rdeps(graph: &PackageGraph, filter: &str, matches: &ArgMatches) {
    // These are safe because we have validators on the args
    let ident = ident_from_matches(matches).unwrap();
    let count = count_from_matches(matches).unwrap();

    let start_time = Instant::now();

    match graph.rdeps(&ident) {
        Some(rdeps) => {
            let duration_secs = start_time.elapsed().as_secs_f64();
            let mut filtered: Vec<(String, String)> =
                rdeps.into_iter()
                     .filter(|&(ref x, _)| x.starts_with(filter))
                     .collect();

            println!("OK: {} items ({} sec)\n", filtered.len(), duration_secs);

            if filtered.len() > count {
                filtered.drain(count..);
            }

            if !filter.is_empty() {
                println!("Results filtered by: {}", filter);
            }

            for (s1, s2) in filtered {
                println!("{} ({})", s1, s2);
            }
        }
        None => println!("No entries found"),
    }
    println!();
}

fn resolve_name(graph: &PackageGraph, ident: &PackageIdent) -> PackageIdent {
    let parts: Vec<&str> = ident.iter().collect();
    if parts.len() == 2 {
        match graph.resolve(ident) {
            Some(s) => s,
            None => ident.clone(),
        }
    } else {
        ident.clone()
    }
}

fn do_deps(graph: &PackageGraph, matches: &ArgMatches) {
    let ident = ident_from_matches(matches).unwrap(); // safe because we validate this arg
    println!("Dependencies for: {}", ident);
    graph.write_deps(&ident);
}

fn do_db_deps(datastore: &DataStore, graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let ident = ident_from_matches(matches).unwrap(); // safe because we validate this arg
    let filter = filter_from_matches(matches);
    let ident = resolve_name(graph, &ident);
    let target = graph.current_target();

    println!("Dependencies for: {}", ident);

    match datastore.get_job_graph_package(&ident, target) {
        // Thinking about whether we want to check the build deps as well.
        Ok(package) => {
            println!("OK: {} items ({} sec)\n",
                     package.deps.len(),
                     start_time.elapsed().as_secs_f64());
            if !filter.is_empty() {
                println!("Results filtered by: {}\n", filter);
            }

            for dep in package.deps {
                if dep.to_string().starts_with(&filter) {
                    println!("{}", dep.0)
                }
            }
        }
        Err(_) => println!("No matching package found"),
    }

    println!();
}

fn do_check(datastore: &DataStore, graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let mut deps_map = HashMap::new();
    let mut new_deps = Vec::new();
    let ident = ident_from_matches(matches).unwrap();
    let filter = filter_from_matches(matches);
    let ident = resolve_name(graph, &ident);
    let target = graph.current_target();

    match datastore.get_job_graph_package(&ident, target) {
        Ok(package) => {
            if !filter.is_empty() {
                println!("Checks filtered by: {}\n", filter);
            }

            println!("Dependency version updates:");
            for dep in package.deps {
                if dep.to_string().starts_with(&filter) {
                    let dep_name = util::short_ident(&(dep.0), false);
                    let dep_latest = resolve_name(graph, &dep_name);
                    deps_map.insert(dep_name.clone(), dep_latest.clone());
                    new_deps.push(dep_latest.clone());
                    println!("{} -> {}", dep.0, dep_latest);
                }
            }

            println!();

            for new_dep in new_deps {
                check_package(datastore, target, &mut deps_map, &new_dep, &filter);
            }
        }
        Err(_) => println!("No matching package found"),
    }

    println!("\nTime: {} sec\n", start_time.elapsed().as_secs_f64());
}

fn check_package(datastore: &DataStore,
                 target: PackageTarget,
                 deps_map: &mut HashMap<PackageIdent, PackageIdent>,
                 ident: &PackageIdent,
                 filter: &str) {
    match datastore.get_job_graph_package(ident, target) {
        Ok(package) => {
            for dep in package.deps {
                if dep.to_string().starts_with(filter) {
                    let name = util::short_ident(&dep, false);
                    {
                        let entry = deps_map.entry(name).or_insert_with(|| dep.0.clone());
                        if *entry != dep.0 {
                            println!("Conflict: {}", ident);
                            println!("  {}", *entry);
                            println!("  {}", dep.0);
                        }
                    }
                    check_package(datastore, target, deps_map, &dep.0, filter);
                }
            }
        }
        Err(_) => println!("No matching package found for {}", ident),
    };
}

fn do_export(graph: &PackageGraph, matches: &ArgMatches) {
    let start_time = Instant::now();
    let latest = graph.latest();
    let filename = required_filename_from_matches(matches);
    let filter = filter_from_matches(matches);

    println!("\nTime: {} sec\n", start_time.elapsed().as_secs_f64());

    let mut file = File::create(filename).expect("Failed to initialize file");

    if !filter.is_empty() {
        println!("Checks filtered by: {}\n", filter);
    }

    for ident in latest {
        if ident.starts_with(&filter) {
            file.write_fmt(format_args!("{}\n", ident)).unwrap();
        }
    }
}

fn do_target(graph: &mut PackageGraph, matches: &ArgMatches) {
    match target_from_matches(matches) {
        Ok(package_target) => graph.set_target(package_target),
        Err(msg) => println!("{}", msg),
    }
}

fn enable_features(config: &Config) {
    let features: HashMap<_, _> = HashMap::from_iter(vec![("BUILDDEPS", feat::BuildDeps)]);
    let features_enabled = config.features_enabled
                                 .split(',')
                                 .map(|f| f.trim().to_uppercase());

    for key in features_enabled {
        if features.contains_key(key.as_str()) {
            info!("Enabling feature: {}", key);
            feat::enable(features[key.as_str()]);
        }
    }

    if feat::is_enabled(feat::List) {
        println!("Listing possible feature flags: {:?}", features.keys());
        println!("Enable features by populating 'features_enabled' in config");
    }
}

features! {
    pub mod feat {
        const List = 0b0000_0001,
        const BuildDeps = 0b0000_0010
    }
}

// Arg parsing using clap
//

fn make_clap_cli() -> App<'static, 'static> {
    App::new("Interactive graph explorer")
        .about("Interactive graph explorer")
        .version(VERSION)
        .author("\nThe Biome Maintainers <humans@biome.sh>\n")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::DisableHelpSubcommand)
        //        .setting(AppSettings::HelpRequired) // panic if no help string spec'd
        .setting(AppSettings::NoBinaryName)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(build_levels_subcommand())
        .subcommand(check_subcommand())
        .subcommand(db_deps_subcommand())
        .subcommand(deps_subcommand())
        .subcommand(dot_subcommand())
        .subcommand(export_subcommand())
        .subcommand(filter_subcommand())
        .subcommand(find_subcommand())
        .subcommand(help_subcommand())
        .subcommand(quit_subcommand())
        .subcommand(raw_subcommand())
        .subcommand(rdeps_subcommand())
        .subcommand(resolve_subcommand())
        .subcommand(scc_subcommand())
        .subcommand(stats_subcommand())
        .subcommand(target_subcommand())
        .subcommand(top_subcommand())
}

// All of these basically filter the graph in some fashion and dump to a file; may be worth
// combining in some fashion
fn build_levels_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand build_levels =>
              (about: "Dump build levels of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn check_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand check =>
              (about: "Check package")
              (@arg IDENT: +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn db_deps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand db_deps =>
              (about: "Dump package deps from db")
              (@arg IDENT: +takes_value {valid_ident} "Package ident to resolve")
              (@arg FILTER: +takes_value default_value("") "Filter value")
    )
}

fn deps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand deps =>
              (about: "Dump package deps from graph")
              (@arg IDENT: +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn dot_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand dot =>
              (about: "Dump DOT format graph of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
              (@arg LATEST: --latest -l "Write latest graph")
    )
}

fn export_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand export =>
              (about: "Export graph")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg FILTER: +takes_value default_value("") "Filter value")
    )
}

fn help_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand help =>
              (about: "Help on help")
    )
}

fn scc_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand scc =>
              (about: "Dump SCC information for latest graph packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
    )
}

fn raw_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand raw =>
              (about: "Dump raw (simple edge representation) graph of packages")
              (@arg REQUIRED_FILENAME: +required +takes_value "Filename to write to")
              (@arg ORIGIN: "Restrict to this origin")
              (@arg LATEST: --latest -l "Write latest graph")
    )
}

fn filter_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand filter =>
              (about: "Set (unset) filter for packages")
              (@arg FILTER: +takes_value default_value("") "Filter value")
    )
}

fn find_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand find =>
              (about: "Find packages")
              (@arg SEARCH: +takes_value "Search term to use")
              (@arg COUNT: {valid_numeric::<usize>} default_value("10") "Number of packages to show")
    )
}

fn rdeps_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand rdeps =>
              (about: "Find rdeps of a package")
              (@arg IDENT: +takes_value {valid_ident} "Package ident to resolve")
              (@arg COUNT: {valid_numeric::<usize>} default_value("10") "Number of rdeps to show")
    )
}

fn resolve_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand resolve =>
              (about: "Resolve packages")
              (@arg IDENT: +takes_value {valid_ident} "Package ident to resolve")
    )
}

fn quit_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand quit =>
              (about: "quit this shell")
    ).aliases(&["q", "exit"])
}

fn stats_subcommand() -> App<'static, 'static> {
    clap_app!(@subcommand stats =>
              (about: "Show graph stats for targets")
              (@arg ALL: --all "Stats for all targets")
              (@arg TARGET: +takes_value {valid_target} "Target architecture (e.g. x86_64-linux)")
    )
}

fn target_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand target =>
                        (about: "Set target architecture to use")
                    (@arg TARGET: +required +takes_value {valid_target} "Target architecture (e.g. x86_64-linux)")
    );
    sub
}

fn top_subcommand() -> App<'static, 'static> {
    let sub = clap_app!(@subcommand top =>
                        (about: "Show top packages, by usage")
                        (@arg COUNT: {valid_numeric::<usize>} default_value("10") "Number of packages to show")
    );
    sub
}

// This was lifted from the biome CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_ident(val: String) -> Result<(), String> {
    match PackageIdent::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            Err(format!("'{}' is not valid. Package identifiers have the \
                         form origin/name[/version[/release]]",
                        &val))
        }
    }
}

// This was lifted from the biome CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_target(val: String) -> Result<(), String> {
    match PackageTarget::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => {
            let targets: Vec<_> = PackageTarget::targets().map(std::convert::AsRef::as_ref)
                                                          .collect();
            Err(format!("'{}' is not valid. Valid targets are in the form \
                         architecture-platform (currently Biome allows \
                         the following: {})",
                        &val,
                        targets.join(", ")))
        }
    }
}

// This was lifted from the biome CLI
//
#[allow(clippy::needless_pass_by_value)] // Signature required by CLAP
fn valid_numeric<T: FromStr>(val: String) -> Result<(), String> {
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("'{}' is not a valid number", &val)),
    }
}

fn count_from_matches(matches: &ArgMatches) -> Result<usize, String> {
    let count = matches.value_of("COUNT").unwrap();
    count.parse()
         .map_err(|_| format!("{} not valid integer for count", count))
}

fn required_filename_from_matches<'a>(matches: &'a ArgMatches) -> &'a str {
    // Required option, so always present
    matches.value_of("REQUIRED_FILENAME").unwrap()
}

fn filter_from_matches(matches: &ArgMatches) -> String {
    matches.value_of("FILTER")
           .map_or_else(|| String::from(""), |x| x.to_string())
}

fn origin_from_matches<'a>(matches: &'a ArgMatches) -> Option<&'a str> {
    matches.value_of("ORIGIN")
}

fn target_from_matches(matches: &ArgMatches) -> Result<PackageTarget, String> {
    let target = matches.value_of("TARGET").unwrap(); // is target mandatory?
    PackageTarget::from_str(target).map_err(|_| format!("{} is not a valid target", target))
}

fn search_from_matches(matches: &ArgMatches) -> String {
    str_from_matches(matches, "SEARCH", "").to_string()
}

fn str_from_matches<'a>(matches: &'a ArgMatches, name: &str, default: &'a str) -> &'a str {
    matches.value_of(name).unwrap_or(default)
}

fn ident_from_matches(matches: &ArgMatches) -> Result<PackageIdent, String> {
    let ident_str: &str = matches.value_of("IDENT")
                                 .ok_or_else(|| String::from("Ident required"))?;
    PackageIdent::from_str(ident_str).map_err(|e| format!("Expected ident gave error {:?}", e))
}
