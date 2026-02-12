//! FAF Unit Weapon Behavior Auditor — library entry point.
//!
//! Exposes config, parser, model, scheduler, anomaly, store, and report
//! for use by the CLI and tests.

pub mod anomaly;
pub mod config;
pub mod gamedata;
pub mod model;
pub mod parser;
pub mod report;
pub mod scheduler;
pub mod store;
pub mod util;
