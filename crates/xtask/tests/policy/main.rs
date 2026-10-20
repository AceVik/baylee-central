//! The policy check against synthetic workspaces, rule by rule: each case
//! breaks exactly one rule and wants that rule's message (formerly
//! `scripts/test-policy.ps1`).
#![forbid(unsafe_code)]

mod accepts;
mod dependencies;
mod fixture;
mod links;
mod sources;
mod targets;
mod web_modules;
