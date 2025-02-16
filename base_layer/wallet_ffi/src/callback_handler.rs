// Copyright 2019. The Tari Project
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

//! # Wallet Callback Handler
//! This CallbackHandler will monitor event streams from the various wallet services and on relevant events will call
//! the assigned callbacks to provide asynchronous feedback to the client application that an event has occured
//!
//! ## Callbacks
//! `callback_received_transaction` - This will be called when an inbound transaction is received from an external
//! wallet
//!
//! `callback_received_transaction_reply` - This will be called when a reply is received for a pending outbound
//! transaction that is waiting to be negotiated
//!
//! `callback_received_finalized_transaction` - This will be called when a Finalized version on an Inbound transaction
//! is received from the Sender of a transaction
//!
//! `callback_transaction_broadcast` - This will be  called when a Finalized transaction is detected a Broadcast to a
//! base node mempool.
//!
//! `callback_transaction_mined` - This will be called when a Broadcast transaction is detected as mined via a base
//! node request
//!
//! `callback_discovery_process_complete` - This will be called when a `send_transacion(..)` call is made to a peer
//! whose address is not known and a discovery process must be conducted. The outcome of the discovery process is
//! relayed via this callback
//!
//! `callback_base_node_sync_complete` - This is called when a Base Node Sync process is completed or times out. The
//! request_key is used to identify which request this callback references and a result of true means it was successful
//! and false that the process timed out and new one will be started

use log::*;
use tari_comms::types::CommsPublicKey;
use tari_comms_dht::event::{DhtEvent, DhtEventReceiver};
use tari_shutdown::ShutdownSignal;
use tari_wallet::{
    output_manager_service::{
        handle::{OutputManagerEvent, OutputManagerEventReceiver},
        TxId,
    },
    transaction_service::{
        handle::{TransactionEvent, TransactionEventReceiver},
        storage::{
            database::{TransactionBackend, TransactionDatabase},
            models::{CompletedTransaction, InboundTransaction},
        },
    },
};

const LOG_TARGET: &str = "wallet::transaction_service::callback_handler";

#[derive(Clone, Copy)]
enum CallbackValidationResults {
    Success,           // 0
    Aborted,           // 1
    Failure,           // 2
    BaseNodeNotInSync, // 3
}

pub struct CallbackHandler<TBackend>
where TBackend: TransactionBackend + 'static
{
    callback_received_transaction: unsafe extern "C" fn(*mut InboundTransaction),
    callback_received_transaction_reply: unsafe extern "C" fn(*mut CompletedTransaction),
    callback_received_finalized_transaction: unsafe extern "C" fn(*mut CompletedTransaction),
    callback_transaction_broadcast: unsafe extern "C" fn(*mut CompletedTransaction),
    callback_transaction_mined: unsafe extern "C" fn(*mut CompletedTransaction),
    callback_transaction_mined_unconfirmed: unsafe extern "C" fn(*mut CompletedTransaction, u64),
    callback_direct_send_result: unsafe extern "C" fn(TxId, bool),
    callback_store_and_forward_send_result: unsafe extern "C" fn(TxId, bool),
    callback_transaction_cancellation: unsafe extern "C" fn(*mut CompletedTransaction),
    callback_txo_validation_complete: unsafe extern "C" fn(u64, u8),
    callback_transaction_validation_complete: unsafe extern "C" fn(u64, u8),
    callback_saf_messages_received: unsafe extern "C" fn(),
    db: TransactionDatabase<TBackend>,
    transaction_service_event_stream: TransactionEventReceiver,
    output_manager_service_event_stream: OutputManagerEventReceiver,
    dht_event_stream: DhtEventReceiver,
    shutdown_signal: Option<ShutdownSignal>,
    comms_public_key: CommsPublicKey,
}

#[allow(clippy::too_many_arguments)]
impl<TBackend> CallbackHandler<TBackend>
where TBackend: TransactionBackend + 'static
{
    pub fn new(
        db: TransactionDatabase<TBackend>,
        transaction_service_event_stream: TransactionEventReceiver,
        output_manager_service_event_stream: OutputManagerEventReceiver,
        dht_event_stream: DhtEventReceiver,
        shutdown_signal: ShutdownSignal,
        comms_public_key: CommsPublicKey,
        callback_received_transaction: unsafe extern "C" fn(*mut InboundTransaction),
        callback_received_transaction_reply: unsafe extern "C" fn(*mut CompletedTransaction),
        callback_received_finalized_transaction: unsafe extern "C" fn(*mut CompletedTransaction),
        callback_transaction_broadcast: unsafe extern "C" fn(*mut CompletedTransaction),
        callback_transaction_mined: unsafe extern "C" fn(*mut CompletedTransaction),
        callback_transaction_mined_unconfirmed: unsafe extern "C" fn(*mut CompletedTransaction, u64),
        callback_direct_send_result: unsafe extern "C" fn(TxId, bool),
        callback_store_and_forward_send_result: unsafe extern "C" fn(TxId, bool),
        callback_transaction_cancellation: unsafe extern "C" fn(*mut CompletedTransaction),
        callback_txo_validation_complete: unsafe extern "C" fn(TxId, u8),
        callback_transaction_validation_complete: unsafe extern "C" fn(TxId, u8),
        callback_saf_messages_received: unsafe extern "C" fn(),
    ) -> Self {
        info!(
            target: LOG_TARGET,
            "ReceivedTransactionCallback -> Assigning Fn: {:?}", callback_received_transaction
        );
        info!(
            target: LOG_TARGET,
            "ReceivedTransactionReplyCallback -> Assigning Fn: {:?}", callback_received_transaction_reply
        );
        info!(
            target: LOG_TARGET,
            "ReceivedFinalizedTransactionCallback -> Assigning Fn: {:?}", callback_received_finalized_transaction
        );
        info!(
            target: LOG_TARGET,
            "TransactionBroadcastCallback -> Assigning Fn: {:?}", callback_transaction_broadcast
        );
        info!(
            target: LOG_TARGET,
            "TransactionMinedCallback -> Assigning Fn: {:?}", callback_transaction_mined
        );
        info!(
            target: LOG_TARGET,
            "TransactionMinedUnconfirmedCallback -> Assigning Fn: {:?}", callback_transaction_mined_unconfirmed
        );
        info!(
            target: LOG_TARGET,
            "DirectSendResultCallback -> Assigning Fn:  {:?}", callback_direct_send_result
        );
        info!(
            target: LOG_TARGET,
            "StoreAndForwardSendResultCallback -> Assigning Fn:  {:?}", callback_store_and_forward_send_result
        );
        info!(
            target: LOG_TARGET,
            "TransactionCancellationCallback -> Assigning Fn:  {:?}", callback_transaction_cancellation
        );
        info!(
            target: LOG_TARGET,
            "TxoValidationCompleteCallback -> Assigning Fn:  {:?}", callback_txo_validation_complete
        );
        info!(
            target: LOG_TARGET,
            "TransactionValidationCompleteCallback -> Assigning Fn:  {:?}", callback_transaction_validation_complete
        );
        info!(
            target: LOG_TARGET,
            "SafMessagesReceivedCallback -> Assigning Fn:  {:?}", callback_saf_messages_received
        );

        Self {
            callback_received_transaction,
            callback_received_transaction_reply,
            callback_received_finalized_transaction,
            callback_transaction_broadcast,
            callback_transaction_mined,
            callback_transaction_mined_unconfirmed,
            callback_direct_send_result,
            callback_store_and_forward_send_result,
            callback_transaction_cancellation,
            callback_txo_validation_complete,
            callback_transaction_validation_complete,
            callback_saf_messages_received,
            db,
            transaction_service_event_stream,
            output_manager_service_event_stream,
            dht_event_stream,
            shutdown_signal: Some(shutdown_signal),
            comms_public_key,
        }
    }

    pub async fn start(mut self) {
        let mut shutdown_signal = self
            .shutdown_signal
            .take()
            .expect("Callback Handler started without shutdown signal");

        info!(target: LOG_TARGET, "Transaction Service Callback Handler starting");

        loop {
            tokio::select! {
                result = self.transaction_service_event_stream.recv() => {
                    match result {
                        Ok(msg) => {
                            trace!(target: LOG_TARGET, "Transaction Service Callback Handler event {:?}", msg);
                            match (*msg).clone() {
                                TransactionEvent::ReceivedTransaction(tx_id) => {
                                    self.receive_transaction_event(tx_id).await;
                                },
                                TransactionEvent::ReceivedTransactionReply(tx_id) => {
                                    self.receive_transaction_reply_event(tx_id).await;
                                },
                                TransactionEvent::ReceivedFinalizedTransaction(tx_id) => {
                                    self.receive_finalized_transaction_event(tx_id).await;
                                },
                                TransactionEvent::TransactionDirectSendResult(tx_id, result) => {
                                    self.receive_direct_send_result(tx_id, result);
                                },
                                TransactionEvent::TransactionStoreForwardSendResult(tx_id, result) => {
                                    self.receive_store_and_forward_send_result(tx_id, result);
                                },
                                 TransactionEvent::TransactionCancelled(tx_id) => {
                                    self.receive_transaction_cancellation(tx_id).await;
                                },
                                TransactionEvent::TransactionBroadcast(tx_id) => {
                                    self.receive_transaction_broadcast_event(tx_id).await;
                                },
                                TransactionEvent::TransactionMined{tx_id, is_valid: _} => {
                                    self.receive_transaction_mined_event(tx_id).await;
                                },
                                TransactionEvent::TransactionMinedUnconfirmed{tx_id, num_confirmations, is_valid: _} => {
                                    self.receive_transaction_mined_unconfirmed_event(tx_id, num_confirmations).await;
                                },
                                TransactionEvent::TransactionValidationSuccess(tx_id)  => {
                                    self.transaction_validation_complete_event(tx_id, CallbackValidationResults::Success);
                                },
                                TransactionEvent::TransactionValidationFailure(tx_id)  => {
                                    self.transaction_validation_complete_event(tx_id, CallbackValidationResults::Failure);
                                },
                                TransactionEvent::TransactionValidationAborted(tx_id)  => {
                                    self.transaction_validation_complete_event(tx_id, CallbackValidationResults::Aborted);
                                },
                                TransactionEvent::TransactionValidationDelayed(tx_id)  => {
                                    self.transaction_validation_complete_event(tx_id, CallbackValidationResults::BaseNodeNotInSync);
                                },
                                // Only the above variants are mapped to callbacks
                                _ => (),
                            }
                        },
                        Err(_e) => error!(target: LOG_TARGET, "Error reading from Transaction Service event broadcast channel"),
                    }
                },
                result = self.output_manager_service_event_stream.recv() => {
                    match result {
                        Ok(msg) => {
                            trace!(target: LOG_TARGET, "Output Manager Service Callback Handler event {:?}", msg);
                            match (*msg).clone() {
                                OutputManagerEvent::TxoValidationSuccess(request_key) => {
                                    self.output_validation_complete_event(request_key,  CallbackValidationResults::Success);
                                },
                                OutputManagerEvent::TxoValidationFailure(request_key) => {
                                    self.output_validation_complete_event(request_key,  CallbackValidationResults::Failure);
                                },
                                OutputManagerEvent::TxoValidationAborted(request_key) => {
                                    self.output_validation_complete_event(request_key,  CallbackValidationResults::Aborted);
                                },
                                OutputManagerEvent::TxoValidationDelayed(request_key) => {
                                    self.output_validation_complete_event(request_key,  CallbackValidationResults::BaseNodeNotInSync);
                                },
                                // Only the above variants are mapped to callbacks
                                _ => (),
                            }
                        },
                        Err(_e) => error!(target: LOG_TARGET, "Error reading from Output Manager Service event broadcast channel"),
                    }
                },
                result = self.dht_event_stream.recv() => {
                    match result {
                        Ok(msg) => {
                            trace!(target: LOG_TARGET, "DHT Callback Handler event {:?}", msg);
                            if let DhtEvent::StoreAndForwardMessagesReceived = *msg {
                                self.saf_messages_received_event();
                            }
                        },
                        Err(_e) => error!(target: LOG_TARGET, "Error reading from DHT event broadcast channel"),
                    }
                }
                 _ = shutdown_signal.wait() => {
                    info!(target: LOG_TARGET, "Transaction Callback Handler shutting down because the shutdown signal was received");
                    break;
                },
            }
        }
    }

    async fn receive_transaction_event(&mut self, tx_id: TxId) {
        match self.db.get_pending_inbound_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Transaction callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_received_transaction)(boxing);
                }
            },
            Err(e) => error!(
                target: LOG_TARGET,
                "Error retrieving Pending Inbound Transaction: {:?}", e
            ),
        }
    }

    async fn receive_transaction_reply_event(&mut self, tx_id: TxId) {
        match self.db.get_completed_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Transaction Reply callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_received_transaction_reply)(boxing);
                }
            },
            Err(e) => error!(target: LOG_TARGET, "Error retrieving Completed Transaction: {:?}", e),
        }
    }

    async fn receive_finalized_transaction_event(&mut self, tx_id: TxId) {
        match self.db.get_completed_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Finalized Transaction callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_received_finalized_transaction)(boxing);
                }
            },
            Err(e) => error!(target: LOG_TARGET, "Error retrieving Completed Transaction: {:?}", e),
        }
    }

    fn receive_direct_send_result(&mut self, tx_id: TxId, result: bool) {
        debug!(
            target: LOG_TARGET,
            "Calling Direct Send Result callback function for TxId: {} with result {}", tx_id, result
        );
        unsafe {
            (self.callback_direct_send_result)(tx_id, result);
        }
    }

    fn receive_store_and_forward_send_result(&mut self, tx_id: TxId, result: bool) {
        debug!(
            target: LOG_TARGET,
            "Calling Store and Forward Send Result callback function for TxId: {} with result {}", tx_id, result
        );
        unsafe {
            (self.callback_store_and_forward_send_result)(tx_id, result);
        }
    }

    async fn receive_transaction_cancellation(&mut self, tx_id: TxId) {
        let mut transaction = None;
        if let Ok(tx) = self.db.get_cancelled_completed_transaction(tx_id).await {
            transaction = Some(tx);
        } else if let Ok(tx) = self.db.get_cancelled_pending_outbound_transaction(tx_id).await {
            let mut outbound_tx = CompletedTransaction::from(tx);
            outbound_tx.source_public_key = self.comms_public_key.clone();
            transaction = Some(outbound_tx);
        } else if let Ok(tx) = self.db.get_cancelled_pending_inbound_transaction(tx_id).await {
            let mut inbound_tx = CompletedTransaction::from(tx);
            inbound_tx.destination_public_key = self.comms_public_key.clone();
            transaction = Some(inbound_tx);
        };

        match transaction {
            None => error!(
                target: LOG_TARGET,
                "Error retrieving Cancelled Transaction TxId {}", tx_id
            ),
            Some(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Transaction Cancellation callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_transaction_cancellation)(boxing);
                }
            },
        }
    }

    async fn receive_transaction_broadcast_event(&mut self, tx_id: TxId) {
        match self.db.get_completed_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Transaction Broadcast callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_transaction_broadcast)(boxing);
                }
            },
            Err(e) => error!(target: LOG_TARGET, "Error retrieving Completed Transaction: {:?}", e),
        }
    }

    async fn receive_transaction_mined_event(&mut self, tx_id: TxId) {
        match self.db.get_completed_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Transaction Mined callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_transaction_mined)(boxing);
                }
            },
            Err(e) => error!(target: LOG_TARGET, "Error retrieving Completed Transaction: {:?}", e),
        }
    }

    async fn receive_transaction_mined_unconfirmed_event(&mut self, tx_id: TxId, confirmations: u64) {
        match self.db.get_completed_transaction(tx_id).await {
            Ok(tx) => {
                debug!(
                    target: LOG_TARGET,
                    "Calling Received Transaction Mined callback function for TxId: {}", tx_id
                );
                let boxing = Box::into_raw(Box::new(tx));
                unsafe {
                    (self.callback_transaction_mined_unconfirmed)(boxing, confirmations);
                }
            },
            Err(e) => error!(target: LOG_TARGET, "Error retrieving Completed Transaction: {:?}", e),
        }
    }

    fn transaction_validation_complete_event(&mut self, request_key: u64, result: CallbackValidationResults) {
        debug!(
            target: LOG_TARGET,
            "Calling Transaction Validation Complete callback function for Request Key: {} with result {:?}",
            request_key,
            result as u8,
        );
        match result {
            CallbackValidationResults::Success => unsafe {
                (self.callback_transaction_validation_complete)(request_key, CallbackValidationResults::Success as u8);
            },
            CallbackValidationResults::Aborted => unsafe {
                (self.callback_transaction_validation_complete)(request_key, CallbackValidationResults::Aborted as u8);
            },
            CallbackValidationResults::Failure => unsafe {
                (self.callback_transaction_validation_complete)(request_key, CallbackValidationResults::Failure as u8);
            },
            CallbackValidationResults::BaseNodeNotInSync => unsafe {
                (self.callback_transaction_validation_complete)(
                    request_key,
                    CallbackValidationResults::BaseNodeNotInSync as u8,
                );
            },
        }
    }

    fn output_validation_complete_event(&mut self, request_key: u64, result: CallbackValidationResults) {
        debug!(
            target: LOG_TARGET,
            "Calling Output Validation Complete callback function for Request Key: {} with result {:?}",
            request_key,
            result as u8,
        );

        match result {
            CallbackValidationResults::Success => unsafe {
                (self.callback_txo_validation_complete)(request_key, CallbackValidationResults::Success as u8);
            },
            CallbackValidationResults::Aborted => unsafe {
                (self.callback_txo_validation_complete)(request_key, CallbackValidationResults::Aborted as u8);
            },
            CallbackValidationResults::Failure => unsafe {
                (self.callback_txo_validation_complete)(request_key, CallbackValidationResults::Failure as u8);
            },
            CallbackValidationResults::BaseNodeNotInSync => unsafe {
                (self.callback_txo_validation_complete)(
                    request_key,
                    CallbackValidationResults::BaseNodeNotInSync as u8,
                );
            },
        }
    }

    fn saf_messages_received_event(&mut self) {
        debug!(target: LOG_TARGET, "Calling SAF Messages Received callback function");
        unsafe {
            (self.callback_saf_messages_received)();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::callback_handler::CallbackHandler;
    use chrono::Utc;
    use rand::rngs::OsRng;
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };
    use tari_common_types::types::{BlindingFactor, PrivateKey, PublicKey};
    use tari_comms_dht::event::DhtEvent;
    use tari_core::transactions::{
        tari_amount::{uT, MicroTari},
        transaction::Transaction,
        ReceiverTransactionProtocol,
        SenderTransactionProtocol,
    };
    use tari_crypto::keys::{PublicKey as PublicKeyTrait, SecretKey};
    use tari_shutdown::Shutdown;
    use tari_wallet::{
        output_manager_service::handle::OutputManagerEvent,
        test_utils::make_wallet_database_connection,
        transaction_service::{
            handle::TransactionEvent,
            storage::{
                database::TransactionDatabase,
                models::{
                    CompletedTransaction,
                    InboundTransaction,
                    OutboundTransaction,
                    TransactionDirection,
                    TransactionStatus,
                },
                sqlite_db::TransactionServiceSqliteDatabase,
            },
        },
    };
    use tokio::{runtime::Runtime, sync::broadcast};

    struct CallbackState {
        pub received_tx_callback_called: bool,
        pub received_tx_reply_callback_called: bool,
        pub received_finalized_tx_callback_called: bool,
        pub broadcast_tx_callback_called: bool,
        pub mined_tx_callback_called: bool,
        pub mined_tx_unconfirmed_callback_called: u64,
        pub direct_send_callback_called: bool,
        pub store_and_forward_send_callback_called: bool,
        pub tx_cancellation_callback_called_completed: bool,
        pub tx_cancellation_callback_called_inbound: bool,
        pub tx_cancellation_callback_called_outbound: bool,
        pub callback_txo_validation_complete: u32,
        pub callback_transaction_validation_complete: u32,
        pub saf_messages_received: bool,
    }

    impl CallbackState {
        fn new() -> Self {
            Self {
                received_tx_callback_called: false,
                received_tx_reply_callback_called: false,
                received_finalized_tx_callback_called: false,
                broadcast_tx_callback_called: false,
                mined_tx_callback_called: false,
                mined_tx_unconfirmed_callback_called: 0,
                direct_send_callback_called: false,
                store_and_forward_send_callback_called: false,
                callback_txo_validation_complete: 0,
                callback_transaction_validation_complete: 0,
                tx_cancellation_callback_called_completed: false,
                tx_cancellation_callback_called_inbound: false,
                tx_cancellation_callback_called_outbound: false,
                saf_messages_received: false,
            }
        }
    }

    lazy_static! {
        static ref CALLBACK_STATE: Mutex<CallbackState> = Mutex::new(CallbackState::new());
    }

    unsafe extern "C" fn received_tx_callback(tx: *mut InboundTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.received_tx_callback_called = true;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn received_tx_reply_callback(tx: *mut CompletedTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.received_tx_reply_callback_called = true;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn received_tx_finalized_callback(tx: *mut CompletedTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.received_finalized_tx_callback_called = true;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn broadcast_callback(tx: *mut CompletedTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.broadcast_tx_callback_called = true;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn mined_callback(tx: *mut CompletedTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.mined_tx_callback_called = true;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn mined_unconfirmed_callback(tx: *mut CompletedTransaction, confirmations: u64) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.mined_tx_unconfirmed_callback_called = confirmations;
        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn direct_send_callback(_tx_id: u64, _result: bool) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.direct_send_callback_called = true;
        drop(lock);
    }

    unsafe extern "C" fn store_and_forward_send_callback(_tx_id: u64, _result: bool) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.store_and_forward_send_callback_called = true;
        drop(lock);
    }

    unsafe extern "C" fn saf_messages_received_callback() {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        lock.saf_messages_received = true;
        drop(lock);
    }

    unsafe extern "C" fn tx_cancellation_callback(tx: *mut CompletedTransaction) {
        let mut lock = CALLBACK_STATE.lock().unwrap();
        match (*tx).tx_id {
            3 => lock.tx_cancellation_callback_called_inbound = true,
            4 => lock.tx_cancellation_callback_called_completed = true,
            5 => lock.tx_cancellation_callback_called_outbound = true,
            _ => (),
        }

        drop(lock);
        Box::from_raw(tx);
    }

    unsafe extern "C" fn txo_validation_complete_callback(_tx_id: u64, result: u8) {
        let mut lock = CALLBACK_STATE.lock().unwrap();

        lock.callback_txo_validation_complete += result as u32;

        drop(lock);
    }

    unsafe extern "C" fn transaction_validation_complete_callback(_tx_id: u64, result: u8) {
        let mut lock = CALLBACK_STATE.lock().unwrap();

        lock.callback_transaction_validation_complete += result as u32;

        drop(lock);
    }

    #[test]
    fn test_callback_handler() {
        let runtime = Runtime::new().unwrap();

        let (connection, _tempdir) = make_wallet_database_connection(None);
        let db = TransactionDatabase::new(TransactionServiceSqliteDatabase::new(connection, None));
        let rtp = ReceiverTransactionProtocol::new_placeholder();
        let inbound_tx = InboundTransaction::new(
            1u64,
            PublicKey::from_secret_key(&PrivateKey::random(&mut OsRng)),
            22 * uT,
            rtp,
            TransactionStatus::Pending,
            "1".to_string(),
            Utc::now().naive_utc(),
        );
        let completed_tx = CompletedTransaction::new(
            2u64,
            PublicKey::from_secret_key(&PrivateKey::random(&mut OsRng)),
            PublicKey::from_secret_key(&PrivateKey::random(&mut OsRng)),
            MicroTari::from(100),
            MicroTari::from(2000),
            Transaction::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                BlindingFactor::default(),
                BlindingFactor::default(),
            ),
            TransactionStatus::Completed,
            "2".to_string(),
            Utc::now().naive_utc(),
            TransactionDirection::Inbound,
            None,
        );
        let stp = SenderTransactionProtocol::new_placeholder();
        let outbound_tx = OutboundTransaction::new(
            3u64,
            PublicKey::from_secret_key(&PrivateKey::random(&mut OsRng)),
            22 * uT,
            23 * uT,
            stp,
            TransactionStatus::Pending,
            "3".to_string(),
            Utc::now().naive_utc(),
            false,
        );
        let inbound_tx_cancelled = InboundTransaction {
            tx_id: 4u64,
            ..inbound_tx.clone()
        };
        let completed_tx_cancelled = CompletedTransaction {
            tx_id: 5u64,
            ..completed_tx.clone()
        };

        runtime
            .block_on(db.add_pending_inbound_transaction(1u64, inbound_tx))
            .unwrap();
        runtime
            .block_on(db.insert_completed_transaction(2u64, completed_tx))
            .unwrap();
        runtime
            .block_on(db.add_pending_inbound_transaction(4u64, inbound_tx_cancelled))
            .unwrap();
        runtime.block_on(db.cancel_pending_transaction(4u64)).unwrap();
        runtime
            .block_on(db.insert_completed_transaction(5u64, completed_tx_cancelled))
            .unwrap();
        runtime.block_on(db.cancel_completed_transaction(5u64)).unwrap();
        runtime
            .block_on(db.add_pending_outbound_transaction(3u64, outbound_tx))
            .unwrap();
        runtime.block_on(db.cancel_pending_transaction(3u64)).unwrap();

        let (tx_sender, tx_receiver) = broadcast::channel(20);
        let (oms_sender, oms_receiver) = broadcast::channel(20);
        let (dht_sender, dht_receiver) = broadcast::channel(20);

        let shutdown_signal = Shutdown::new();
        let callback_handler = CallbackHandler::new(
            db,
            tx_receiver,
            oms_receiver,
            dht_receiver,
            shutdown_signal.to_signal(),
            PublicKey::from_secret_key(&PrivateKey::random(&mut OsRng)),
            received_tx_callback,
            received_tx_reply_callback,
            received_tx_finalized_callback,
            broadcast_callback,
            mined_callback,
            mined_unconfirmed_callback,
            direct_send_callback,
            store_and_forward_send_callback,
            tx_cancellation_callback,
            txo_validation_complete_callback,
            transaction_validation_complete_callback,
            saf_messages_received_callback,
        );

        runtime.spawn(callback_handler.start());

        tx_sender
            .send(Arc::new(TransactionEvent::ReceivedTransaction(1u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::ReceivedTransactionReply(2u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::ReceivedFinalizedTransaction(2u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionBroadcast(2u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionMined {
                tx_id: 2u64,
                is_valid: true,
            }))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionMinedUnconfirmed {
                tx_id: 2u64,
                num_confirmations: 22u64,
                is_valid: true,
            }))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionDirectSendResult(2u64, true)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionStoreForwardSendResult(
                2u64, true,
            )))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionCancelled(3u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionCancelled(4u64)))
            .unwrap();
        tx_sender
            .send(Arc::new(TransactionEvent::TransactionCancelled(5u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationSuccess(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationSuccess(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationSuccess(1u64)))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionValidationSuccess(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationFailure(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationFailure(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationFailure(1u64)))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionValidationFailure(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationAborted(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationAborted(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationAborted(1u64)))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionValidationAborted(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationDelayed(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationDelayed(1u64)))
            .unwrap();

        oms_sender
            .send(Arc::new(OutputManagerEvent::TxoValidationDelayed(1u64)))
            .unwrap();

        tx_sender
            .send(Arc::new(TransactionEvent::TransactionValidationDelayed(1u64)))
            .unwrap();

        dht_sender
            .send(Arc::new(DhtEvent::StoreAndForwardMessagesReceived))
            .unwrap();

        thread::sleep(Duration::from_secs(10));

        let lock = CALLBACK_STATE.lock().unwrap();
        assert!(lock.received_tx_callback_called);
        assert!(lock.received_tx_reply_callback_called);
        assert!(lock.received_finalized_tx_callback_called);
        assert!(lock.broadcast_tx_callback_called);
        assert!(lock.mined_tx_callback_called);
        assert_eq!(lock.mined_tx_unconfirmed_callback_called, 22u64);
        assert!(lock.direct_send_callback_called);
        assert!(lock.store_and_forward_send_callback_called);
        assert!(lock.tx_cancellation_callback_called_inbound);
        assert!(lock.tx_cancellation_callback_called_completed);
        assert!(lock.tx_cancellation_callback_called_outbound);
        assert!(lock.saf_messages_received);
        assert_eq!(lock.callback_txo_validation_complete, 18);
        assert_eq!(lock.callback_transaction_validation_complete, 6);

        drop(lock);
    }
}
