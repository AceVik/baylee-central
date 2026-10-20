//! The policy check against synthetic workspaces, rule by rule: each case
//! breaks exactly one rule and wants that rule's message, the same as
//! `scripts/test-policy.ps1` proves for the PowerShell check.
#![forbid(unsafe_code)]

mod accepts;
mod dependencies;
mod fixture;
mod links;
mod sources;
mod targets;
mod web_modules;
