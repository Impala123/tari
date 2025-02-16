// Copyright 2020. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::{connectivity_service::WalletConnectivityError, error::WalletStorageError};
use tari_comms::{connectivity::ConnectivityError, protocol::rpc::RpcError};
use tari_comms_dht::outbound::DhtOutboundError;
use tari_service_framework::reply_channel::TransportChannelError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BaseNodeServiceError {
    #[error("No base node peer set")]
    NoBaseNodePeer,
    #[error("Error connecting to base node: {0}")]
    BaseNodeConnectivityError(#[from] ConnectivityError),
    #[error("RPC Error: `{0}`")]
    RpcError(#[from] RpcError),
    #[error("No chain metadata from peer")]
    NoChainMetadata,
    #[error("Unexpected API Response")]
    UnexpectedApiResponse,
    #[error("Transport channel error: `{0}`")]
    TransportChannelError(#[from] TransportChannelError),
    #[error("Outbound Error: `{0}`")]
    OutboundError(#[from] DhtOutboundError),
    #[error("Received invalid base node response: {0}")]
    InvalidBaseNodeResponse(String),
    #[error("Wallet storage error: `{0}`")]
    WalletStorageError(#[from] WalletStorageError),
    #[error("Wallet connectivity error: `{0}`")]
    WalletConnectivityError(#[from] WalletConnectivityError),
}
