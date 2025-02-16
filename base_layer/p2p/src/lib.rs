//  Copyright 2019 The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]
#![deny(unused_must_use)]
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]

#[cfg(test)]
#[macro_use]
mod test_utils;

#[cfg(feature = "auto-update")]
pub mod auto_update;
pub mod comms_connector;
pub mod domain_message;
pub mod initialization;
pub mod peer;
pub mod peer_seeds;
pub mod proto;
pub mod services;
pub mod tari_message;
pub mod transport;

mod dns;

// Re-export
pub use tari_common::configuration::Network;

/// Default DNS resolver set to cloudflare's private 1.1.1.1 resolver
pub const DEFAULT_DNS_NAME_SERVER: &str = "1.1.1.1:53";

/// Major network version. Peers will refuse connections if this value differs
pub const MAJOR_NETWORK_VERSION: u32 = 0;
/// Minor network version. This should change with each time the network protocol has changed in a backward-compatible
/// way.
pub const MINOR_NETWORK_VERSION: u32 = 0;
