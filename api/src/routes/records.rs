// Copyright (c) The dgc.network
// SPDX-License-Identifier: Apache-2.0

use actix_web::*;
use sawtooth_sdk::processor::handler::ApplyError;
use serde::Deserialize;
use protobuf::Message;
use reqwest;
use chrono;
use std::convert::TryInto;

use crate::transaction::BatchBuilder;
use crate::error::RestApiResponseError;
use crate::{List, Fetch, split_vec};

use dgc_config::protos::*;
use dgc_config::addressing::*;
//use dgc_config::protocol::track_and_trace::state::*;
use dgc_config::protocol::track_and_trace::payload::*;
use dgc_config::protocol::schema::state::*;

#[derive(Deserialize)]
pub struct RecordData {
    private_key: String,
    record_id: String,
    schema: String,
    properties: String,
}

pub async fn list_records(
) -> Result<HttpResponse, RestApiResponseError> {

    let url = format!("http://rest-api:8008/state?address={}", get_record_prefix());
    let list = reqwest::get(&url).await?.json::<List>().await?;
    let mut response_data = "[".to_owned();
    for sub in list.data {
        let msg = base64::decode(&sub.data).unwrap();
        let records: track_and_trace_state::RecordList = match protobuf::parse_from_bytes(&msg){
            Ok(records) => records,
            Err(err) => {
                return Err(RestApiResponseError::ApplyError(ApplyError::InternalError(format!(
                    "Cannot deserialize data: {:?}",
                    err,
                ))))
            }
        };

        for record in records.get_entries() {
            println!("!dgc-network! response_data: ");
            println!("    record_id: {:?},", record.record_id);
            println!("    schema: {:?},", record.schema);
            println!("    owners: {:?},", record.owners);
            println!("    custodians: {:?}", record.custodians);
            println!("    field_final: {:?}", record.field_final);
            
            response_data = response_data + &format!("\n  {{\n    record_id: {:?}, \n    schema: {:?}, \n    owner: {:?}, \n    custodians: {:?}, \n    field_final: {:?}\n  }},\n", record.record_id, record.schema, record.owners, record.custodians, record.field_final);
        }
    }
    response_data = response_data + &format!("]");
    Ok(HttpResponse::Ok().body(response_data))
}

pub async fn fetch_record(
    record_id: web::Path<String>,
) -> Result<HttpResponse, RestApiResponseError> {

    let address = make_record_address(&record_id);
    let url = format!("http://rest-api:8008/state/{}", address);
    let res = reqwest::get(&url).await?.json::<Fetch>().await?;
    let msg = base64::decode(&res.data).unwrap();
    let records: track_and_trace_state::RecordList = match protobuf::parse_from_bytes(&msg){
        Ok(records) => records,
        Err(err) => {
            return Err(RestApiResponseError::ApplyError(ApplyError::InternalError(format!(
                "Cannot deserialize data: {:?}",
                err,
            ))))
        }
    };
    let mut response_data = "".to_owned();
    for record in records.get_entries() {
        println!("!dgc-network! response_data: ");
        println!("    record_id: {:?},", record.record_id);
        println!("    schema: {:?},", record.schema);
        println!("    owners: {:?},", record.owners);
        println!("    custodians: {:?}", record.custodians);
        println!("    field_final: {:?}", record.field_final);
        
        response_data = response_data + &format!("{{\n  record_id: {:?}, \n  schema: {:?}, \n  owners: {:?}, \n  custodians: {:?}, \n  field_final: {:?} \n}}", record.record_id, record.schema, record.owners, record.custodians, record.field_final);
    }
    Ok(HttpResponse::Ok().body(response_data))
}

pub async fn create_record(
    input_data: web::Json<RecordData>,
) -> Result<HttpResponse, RestApiResponseError> {

    // Creating the Payload //
    let private_key = &input_data.private_key;
    let record_id = &input_data.record_id;
    let schema = &input_data.schema;
    let properties = retrieve_property_values(&input_data);

    // Building the Action and Payload//
    let action = CreateRecordActionBuilder::new()
        .with_record_id(record_id.into())
        .with_schema(schema.into())
        .with_properties(properties)
        .build()
        .unwrap();

    let payload = TrackAndTracePayloadBuilder::new()
        .with_action(Action::CreateRecord(action.clone()))
        .with_timestamp(chrono::offset::Utc::now().timestamp().try_into().unwrap())
        .build()
        .unwrap();

    // Building the Transaction and Batch//
    let batch_list = BatchBuilder::new(
        TNT_FAMILY_NAME, 
        TNT_FAMILY_VERSION, 
        private_key,
    ).add_transaction(
        &payload.into_proto()?,
        &[get_record_prefix(), get_schema_prefix(), get_pike_prefix()],
        &[get_record_prefix(), get_schema_prefix(), get_pike_prefix()],
    )?.create_batch_list();

    let batch_list_bytes = batch_list
        .write_to_bytes()
        .expect("Error converting batch list to bytes");

    // Submitting Batches to the Validator //
    let res = reqwest::Client::new()
        .post("http://rest-api:8008/batches")
        .header("Content-Type", "application/octet-stream")
        .body(batch_list_bytes)
        .send().await?
        .text().await?;

    println!("============ create_record_link ============");
    println!("!dgc-network! submit_status = {:?}", res);

    Ok(HttpResponse::Ok().body(res))
}

pub async fn update_record(
    input_data: web::Json<RecordData>,
) -> Result<HttpResponse, RestApiResponseError> {

    // Creating the Payload //
    let private_key = &input_data.private_key;
    let record_id = &input_data.record_id;
    //let schema = &input_data.schema;
    //let properties = retrieve_property_values(&input_data);

    // Building the Action and Payload//
    let action = FinalizeRecordActionBuilder::new()
        .with_record_id(record_id.into())
        .build()
        .unwrap();

    let payload = TrackAndTracePayloadBuilder::new()
        .with_action(Action::FinalizeRecord(action.clone()))
        .with_timestamp(chrono::offset::Utc::now().timestamp().try_into().unwrap())
        .build()
        .unwrap();

    // Building the Transaction and Batch//
    let batch_list = BatchBuilder::new(
        TNT_FAMILY_NAME, 
        TNT_FAMILY_VERSION, 
        private_key,
    ).add_transaction(
        &payload.into_proto()?,
        &[get_record_prefix(), get_schema_prefix(), get_pike_prefix()],
        &[get_record_prefix(), get_schema_prefix(), get_pike_prefix()],
    )?.create_batch_list();

    let batch_list_bytes = batch_list
        .write_to_bytes()
        .expect("Error converting batch list to bytes");

    // Submitting Batches to the Validator //
    let res = reqwest::Client::new()
        .post("http://rest-api:8008/batches")
        .header("Content-Type", "application/octet-stream")
        .body(batch_list_bytes)
        .send().await?
        .text().await?;

    println!("============ finalize_record_link ============");
    println!("!dgc-network! submit_status = {:?}", res);

    Ok(HttpResponse::Ok().body(res))
}

fn retrieve_property_values(
    input_data: &web::Json<RecordData>,
) -> Vec::<PropertyValue> {
/*    
    name: String,
    data_type: DataType,
    bytes_value: Vec<u8>,
    boolean_value: bool,
    number_value: i64,
    string_value: String,
    enum_value: u32,
    struct_values: Vec<PropertyValue>,
    lat_long_value: LatLong,
*/

    let mut properties = Vec::<PropertyValue>::new();
    let properties_as_string = &input_data.properties;
    let vec: Vec<&str> = properties_as_string.split(",").collect();
    let key_val_vec = split_vec(vec, 9);
    for key_val in key_val_vec {

        let name: String = match key_val.get(0) {
            Some(value) => value.to_string(),
            None => "name is formated incorrectly".to_string()
        };
        println!("!dgc-network! name = {:?}", name);
        
        let data_type: DataType = match key_val.get(1) {
            Some(value) => 
                if (value == &"Bytes") | (value == &"bytes") | (value == &"BYTES") {DataType::Bytes}
                else if (value == &"Boolean") | (value == &"boolean") | (value == &"BOOLEAN") {DataType::Boolean}
                else if (value == &"Number") | (value == &"number") | (value == &"NUMBER") {DataType::Number}
                else if (value == &"String") | (value == &"string") | (value == &"STRING") {DataType::String}
                else if (value == &"Enum") | (value == &"enum") | (value == &"ENUM") {DataType::Enum}
                else if (value == &"Struct") | (value == &"struct") | (value == &"STRUCT") {DataType::Struct}
                else if (value == &"LatLong") | (value == &"LatLong") | (value == &"LATLONG") {DataType::LatLong}
                else {DataType::String},
            None => DataType::String
        };
        println!("!dgc-network! data_type = {:?}", data_type);

        if data_type == DataType::Bytes {
            let string_value = match key_val.get(2) {
                Some(value) => value.to_string(),
                None => "string_value is formated incorrectly".to_string()
            };    
            let bytes_value = string_value.as_bytes();

            let property_value = PropertyValueBuilder::new()
            .with_name(name.clone().into())
            .with_data_type(DataType::Bytes)
            .with_bytes_value(bytes_value.to_vec())
            .build()
            .unwrap();
            properties.push(property_value.clone());    
        }

        if data_type == DataType::Boolean {
            let string_value = match key_val.get(3) {
                Some(value) => value.to_string(),
                None => "false".to_string()
            };    
            let boolean_value = string_value.parse::<bool>();

            let property_value = PropertyValueBuilder::new()
            .with_name(name.clone().into())
            .with_data_type(DataType::Boolean)
            .with_boolean_value(boolean_value.unwrap())
            .build()
            .unwrap();
            properties.push(property_value.clone());    
        }

        if data_type == DataType::Number {
            let string_value = match key_val.get(4) {
                Some(value) => value.to_string(),
                None => "0".to_string()
            };    
            let number_value = string_value.parse::<i64>();

            let property_value = PropertyValueBuilder::new()
            .with_name(name.clone().into())
            .with_data_type(DataType::Number)
            .with_number_value(number_value.unwrap())
            .build()
            .unwrap();
            properties.push(property_value.clone());    
        }

        if data_type == DataType::String {
            let string_value = match key_val.get(5) {
                Some(value) => value.to_string(),
                None => "string_value is formated incorrectly".to_string()
            };    

            let property_value = PropertyValueBuilder::new()
            .with_name(name.clone().into())
            .with_data_type(DataType::String)
            .with_string_value(string_value)
            .build()
            .unwrap();
            properties.push(property_value.clone());    
        }

        if data_type == DataType::Enum {
            let string_value = match key_val.get(6) {
                Some(value) => value.to_string(),
                None => "string_value is formated incorrectly".to_string()
            };    
            let enum_value = string_value.parse::<u32>();

            let property_value = PropertyValueBuilder::new()
            .with_name(name.clone().into())
            .with_data_type(DataType::Enum)
            .with_enum_value(enum_value.unwrap())
            .build()
            .unwrap();
            properties.push(property_value.clone());    
        }

    }
    return properties
}
