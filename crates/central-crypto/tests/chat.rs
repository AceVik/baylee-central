//! Native Baylee chat vectors and adversarial cases, unchanged expected bytes.
#![forbid(unsafe_code)]

#[path = "chat/b64_wire.rs"]
mod b64_wire;
#[path = "chat/common.rs"]
mod common;
#[path = "chat/history_names.rs"]
mod history_names;
#[path = "chat/known_answers.rs"]
mod known_answers;
#[path = "chat/reports_and_devices.rs"]
mod reports_and_devices;
#[path = "chat/round_trips.rs"]
mod round_trips;
#[path = "chat/safety_numbers.rs"]
mod safety_numbers;
#[path = "chat/secrets_in_debug.rs"]
mod secrets_in_debug;
#[path = "chat/tampering.rs"]
mod tampering;
#[path = "chat/testkit.rs"]
mod testkit;
