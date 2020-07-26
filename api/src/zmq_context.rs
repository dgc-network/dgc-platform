// Copyright (c) The dgc.network
// SPDX-License-Identifier: Apache-2.0

use protobuf::Message as M;
use protobuf::RepeatedField;

use sawtooth_sdk::messages::events::Event;
use sawtooth_sdk::messages::events::Event_Attribute;
use sawtooth_sdk::messages::state_context::*;
use sawtooth_sdk::messages::validator::Message_MessageType;
use sawtooth_sdk::messaging::stream::MessageSender;
use sawtooth_sdk::messaging::zmq_stream::ZmqMessageSender;
use sawtooth_sdk::processor::handler::{ContextError, TransactionContext};

extern crate rand;
use rand::{thread_rng, Rng};
use rand::distributions::{Alphanumeric, Uniform, Standard};

/// Generates a random correlation id for use in Message
fn generate_correlation_id() -> String {
    const LENGTH: usize = 16;
    rand::thread_rng().sample_iter(Alphanumeric).take(LENGTH).collect()
}

#[derive(Clone)]
pub struct ZmqTransactionContext {
    context_id: String,
    sender: ZmqMessageSender,
}

impl ZmqTransactionContext {
    /// Context provides an interface for getting, setting, and deleting
    /// validator state. All validator interactions by a handler should be
    /// through a Context instance.
    ///
    /// # Arguments
    ///
    /// * `sender` - for client grpc communication
    /// * `context_id` - the context_id passed in from the validator
    pub fn new(context_id: &str, sender: ZmqMessageSender) -> Self {
        ZmqTransactionContext {
            context_id: String::from(context_id),
            sender,
        }
    }
}

impl TransactionContext for ZmqTransactionContext {
    /// get_state_entries queries the validator state for data at each of the
    /// addresses in the given list. The addresses that have been set
    /// are returned.
    ///
    /// # Arguments
    ///
    /// * `addresses` - the addresses to fetch
    fn get_state_entries(
        &self,
        addresses: &[String],
    ) -> Result<Vec<(String, Vec<u8>)>, ContextError> {
        println!("============ get_state_entries_1 ============");
        let mut request = TpStateGetRequest::new();
        request.set_context_id(self.context_id.clone());
        request.set_addresses(RepeatedField::from_vec(addresses.to_vec()));
        let serialized = request.write_to_bytes()?;
        let x: &[u8] = &serialized;

        let mut future = self.sender.send(
            Message_MessageType::TP_STATE_GET_REQUEST,
            &generate_correlation_id(),
            x,
        )?;
        println!("============ get_state_entries_2 ============");
        println!("!dgc.network! serialized = {}", serialized);

        let response: TpStateGetResponse = protobuf::parse_from_bytes(future.get()?.get_content())?;
        println!("============ get_state_entries_3 ============");
        match response.get_status() {
            TpStateGetResponse_Status::OK => {
                println!("============ get_state_entries_4 ============");
                let mut entries = Vec::new();
                for entry in response.get_entries() {
                    match entry.get_data().len() {
                        0 => continue,
                        _ => entries
                            .push((entry.get_address().to_string(), Vec::from(entry.get_data()))),
                    }
                }
                Ok(entries)
            }
            TpStateGetResponse_Status::AUTHORIZATION_ERROR => {
                println!("============ get_state_entries_5 ============");
                Err(ContextError::AuthorizationError(format!(
                    "Tried to get unauthorized addresses: {:?}",
                    addresses
                )))
            }
            TpStateGetResponse_Status::STATUS_UNSET => {
                println!("============ get_state_entries_6 ============");
                Err(ContextError::ResponseAttributeError(
                String::from("Status was not set for TpStateGetResponse"),
                ))
            }
        }
    }

    /// set_state requests that each address in the provided map be
    /// set in validator state to its corresponding value.
    ///
    /// # Arguments
    ///
    /// * `entries` - entries are a hashmap where the key is an address and value is the data
    fn set_state_entries(&self, entries: Vec<(String, Vec<u8>)>) -> Result<(), ContextError> {
        let state_entries: Vec<TpStateEntry> = entries
            .into_iter()
            .map(|(address, payload)| {
                let mut entry = TpStateEntry::new();
                entry.set_address(address);
                entry.set_data(payload);
                entry
            })
            .collect();

        let mut request = TpStateSetRequest::new();
        request.set_context_id(self.context_id.clone());
        request.set_entries(RepeatedField::from_vec(state_entries.to_vec()));
        let serialized = request.write_to_bytes()?;
        let x: &[u8] = &serialized;

        let mut future = self.sender.send(
            Message_MessageType::TP_STATE_SET_REQUEST,
            &generate_correlation_id(),
            x,
        )?;

        let response: TpStateSetResponse = protobuf::parse_from_bytes(future.get()?.get_content())?;
        match response.get_status() {
            TpStateSetResponse_Status::OK => Ok(()),
            TpStateSetResponse_Status::AUTHORIZATION_ERROR => {
                Err(ContextError::AuthorizationError(format!(
                    "Tried to set unauthorized addresses: {:?}",
                    state_entries
                )))
            }
            TpStateSetResponse_Status::STATUS_UNSET => Err(ContextError::ResponseAttributeError(
                String::from("Status was not set for TpStateSetResponse"),
            )),
        }
    }

    /// delete_state_entries requests that each of the provided addresses be unset
    /// in validator state. A list of successfully deleted addresses
    /// is returned.
    ///
    /// # Arguments
    ///
    /// * `addresses` - the addresses to delete
    fn delete_state_entries(&self, addresses: &[String]) -> Result<Vec<String>, ContextError> {
        let mut request = TpStateDeleteRequest::new();
        request.set_context_id(self.context_id.clone());
        request.set_addresses(RepeatedField::from_slice(addresses));

        let serialized = request.write_to_bytes()?;
        let x: &[u8] = &serialized;

        let mut future = self.sender.send(
            Message_MessageType::TP_STATE_DELETE_REQUEST,
            &generate_correlation_id(),
            x,
        )?;

        let response: TpStateDeleteResponse =
            protobuf::parse_from_bytes(future.get()?.get_content())?;
        match response.get_status() {
            TpStateDeleteResponse_Status::OK => Ok(Vec::from(response.get_addresses())),
            TpStateDeleteResponse_Status::AUTHORIZATION_ERROR => {
                Err(ContextError::AuthorizationError(format!(
                    "Tried to delete unauthorized addresses: {:?}",
                    addresses
                )))
            }
            TpStateDeleteResponse_Status::STATUS_UNSET => {
                Err(ContextError::ResponseAttributeError(String::from(
                    "Status was not set for TpStateDeleteResponse",
                )))
            }
        }
    }

    /// add_receipt_data adds a blob to the execution result for this transaction
    ///
    /// # Arguments
    ///
    /// * `data` - the data to add
    fn add_receipt_data(&self, data: &[u8]) -> Result<(), ContextError> {
        let mut request = TpReceiptAddDataRequest::new();
        request.set_context_id(self.context_id.clone());
        request.set_data(Vec::from(data));

        let serialized = request.write_to_bytes()?;
        let x: &[u8] = &serialized;

        let mut future = self.sender.send(
            Message_MessageType::TP_RECEIPT_ADD_DATA_REQUEST,
            &generate_correlation_id(),
            x,
        )?;

        let response: TpReceiptAddDataResponse =
            protobuf::parse_from_bytes(future.get()?.get_content())?;
        match response.get_status() {
            TpReceiptAddDataResponse_Status::OK => Ok(()),
            TpReceiptAddDataResponse_Status::ERROR => Err(ContextError::TransactionReceiptError(
                format!("Failed to add receipt data {:?}", data),
            )),
            TpReceiptAddDataResponse_Status::STATUS_UNSET => {
                Err(ContextError::ResponseAttributeError(String::from(
                    "Status was not set for TpReceiptAddDataResponse",
                )))
            }
        }
    }

    /// add_event adds a new event to the execution result for this transaction.
    ///
    /// # Arguments
    ///
    /// * `event_type` -  This is used to subscribe to events. It should be globally unique and
    ///         describe what, in general, has occured.
    /// * `attributes` - Additional information about the event that is transparent to the
    ///          validator. Attributes can be used by subscribers to filter the type of events
    ///          they receive.
    /// * `data` - Additional information about the event that is opaque to the validator.
    fn add_event(
        &self,
        event_type: String,
        attributes: Vec<(String, String)>,
        data: &[u8],
    ) -> Result<(), ContextError> {
        let mut event = Event::new();
        event.set_event_type(event_type);

        let mut attributes_vec = Vec::new();
        for (key, value) in attributes {
            let mut attribute = Event_Attribute::new();
            attribute.set_key(key);
            attribute.set_value(value);
            attributes_vec.push(attribute);
        }
        event.set_attributes(RepeatedField::from_vec(attributes_vec));
        event.set_data(Vec::from(data));

        let mut request = TpEventAddRequest::new();
        request.set_context_id(self.context_id.clone());
        request.set_event(event.clone());

        let serialized = request.write_to_bytes()?;
        let x: &[u8] = &serialized;

        let mut future = self.sender.send(
            Message_MessageType::TP_EVENT_ADD_REQUEST,
            &generate_correlation_id(),
            x,
        )?;

        let response: TpEventAddResponse = protobuf::parse_from_bytes(future.get()?.get_content())?;
        match response.get_status() {
            TpEventAddResponse_Status::OK => Ok(()),
            TpEventAddResponse_Status::ERROR => Err(ContextError::TransactionReceiptError(
                format!("Failed to add event {:?}", event),
            )),
            TpEventAddResponse_Status::STATUS_UNSET => Err(ContextError::ResponseAttributeError(
                String::from("Status was not set for TpEventAddRespons"),
            )),
        }
    }
}