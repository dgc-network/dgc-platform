// Copyright (c) The dgc.network
// SPDX-License-Identifier: Apache-2.0

use actix_web::{web, HttpRequest, HttpResponse};
//use sawtooth_sdk::signing::create_context;
use sawtooth_sdk::signing::secp256k1::Secp256k1PrivateKey;
use sawtooth_sdk::signing::PrivateKey;
use serde::Deserialize;
use protobuf::Message;
use reqwest;
use url::Url;

use crate::transaction::BatchBuilder;
use crate::state::{
    PIKE_NAMESPACE, PIKE_FAMILY_NAME, PIKE_FAMILY_VERSION,
    ApiTransactionContext, ApiState
};
use crate::error::RestApiResponseError;

use grid_sdk::protocol::pike::{
    state::{
        KeyValueEntry, KeyValueEntryBuilder,
    },
    payload::{
        Action, PikePayloadBuilder, 
        CreateOrganizationActionBuilder, UpdateOrganizationActionBuilder, 
    },
};
use grid_sdk::protos::IntoProto;

#[derive(Deserialize)]
pub struct OrgInput {
    private_key: String,
    org_id: String,
    name: String,
    address: String,
    //metadata: Vec<KeyValueEntry>,
    metadata: String,
}

#[derive(Deserialize)]
struct Info {
    address: String,
}

pub async fn list_orgs(
    //info: web::Query<Info>
) -> Result<HttpResponse, RestApiResponseError> {
/*
    let transaction_context = ApiTransactionContext::default();
    let state = ApiState::new(&transaction_context);
    let orgs = state.get_organizations("organization_org_id").unwrap();
    println!("!dgc-network! orgs = {:?}", orgs);
*/

    println!("============ list_org beginning ============");
    //let url = Url::parse(&format!("http://rest-api:8008/state?address={}", "cad11d01"));
    //let res = reqwest::Client::new()
    //    .get(url)
    let res = reqwest::get("http://rest-api:8008/state?address=cad11d01")
        .await?
        .text()
        .await?;

    println!("============ list_org ============");
    println!("!dgc-network! res = {:?}", res);

    Ok(HttpResponse::Ok().body("Hello world! list_org"))

}

pub async fn fetch_org(
    org_id: web::Path<String>,
) -> Result<HttpResponse, RestApiResponseError> {

    println!("!dgc-network! org_id = {:?}", org_id);
/*
    let transaction_context = ApiTransactionContext::default();
    let state = ApiState::new(&transaction_context);
    let org = state.get_organization(&org_id).unwrap();
    println!("!dgc-network! org = {:?}", org);
*/
/*
    let url = Url::parse(&format!("http://rest-api:8008/state/{}", org_id));
    let res = reqwest::Client::new()
        .get(url)
    //let res = reqwest::get(url)
        .await?
        .text()
        .await?;

    println!("============ fetch_org ============");
    println!("!dgc-network! res = {:?}", res);
*/
    Ok(HttpResponse::Ok().body("Hello world! fetch_org"))

}

pub async fn create_org(
    org_input: web::Json<OrgInput>,
) -> Result<HttpResponse, RestApiResponseError> {

    // Creating a Private Key and Signer //
    let private_key_as_hex = &org_input.private_key;
    let private_key = Secp256k1PrivateKey::from_hex(&private_key_as_hex)
        .expect("Error generating a new Private Key");

    // Creating the Payload //
    let org_id = &org_input.org_id;
    let name = &org_input.name;
    let address = &org_input.address;
    let metadata_as_string = &org_input.metadata;

    let mut metadata = Vec::<KeyValueEntry>::new();
    for meta in metadata_as_string.chars() {
        let meta_as_string = meta.to_string();
        let key_val: Vec<&str> = meta_as_string.split(",").collect();
        if key_val.len() != 2 {
            "Metadata is formated incorrectly".to_string();            
        }
        let key = match key_val.get(0) {
            Some(key) => key.to_string(),
            None => "Metadata is formated incorrectly".to_string()
        };
        let value = match key_val.get(1) {
            Some(value) => value.to_string(),
            None => "Metadata is formated incorrectly".to_string()
        };

        let key_value = KeyValueEntryBuilder::new()
            .with_key(key.to_string())
            .with_value(value.to_string())
            .build()
            .unwrap();

        metadata.push(key_value.clone());
    }

    let action = CreateOrganizationActionBuilder::new()
        .with_org_id(org_id.to_string())
        .with_name(name.to_string())
        .with_address(address.to_string())
        .with_metadata(metadata)
        .build()
        .unwrap();

    let payload = PikePayloadBuilder::new()
        .with_action(Action::CreateOrganization)
        .with_create_organization(action)
        .build()
        .map_err(|err| RestApiResponseError::UserError(format!("{}", err)))?;

    // Building the Transaction //
    // Building the Batch //
    let batch_list = BatchBuilder::new(
        PIKE_FAMILY_NAME, 
        PIKE_FAMILY_VERSION, 
        &private_key.as_hex()
    )
    .add_transaction(
        &payload.into_proto()?,
        &[PIKE_NAMESPACE.to_string()],
        &[PIKE_NAMESPACE.to_string()],
    )?
    .create_batch_list();

    let batch_list_bytes = batch_list
        .write_to_bytes()
        .expect("Error converting batch list to bytes");

    // Submitting Batches to the Validator //
    let res = reqwest::Client::new()
        .post("http://rest-api:8008/batches")
        .header("Content-Type", "application/octet-stream")
        .body(batch_list_bytes)
        .send()
        .await?
        .text()
        .await?;

    println!("============ create_organization ============");
    println!("!dgc-network! res = {:?}", res);

    Ok(HttpResponse::Ok().body(res))
}

pub async fn update_org(
    org_input: web::Json<OrgInput>,
) -> Result<HttpResponse, RestApiResponseError> {

    // Creating a Private Key and Signer //
    let private_key_as_hex = &org_input.private_key;
    let private_key = Secp256k1PrivateKey::from_hex(&private_key_as_hex)
        .expect("Error generating a new Private Key");

    // Creating the Payload //
    let org_id = &org_input.org_id;
    let name = &org_input.name;
    let address = &org_input.address;
    let metadata_as_string = &org_input.metadata;

    let mut metadata = Vec::<KeyValueEntry>::new();
    for meta in metadata_as_string.chars() {
        let meta_as_string = meta.to_string();
        let key_val: Vec<&str> = meta_as_string.split(",").collect();
        if key_val.len() != 2 {
            "Metadata is formated incorrectly".to_string();            
        }
        let key = match key_val.get(0) {
            Some(key) => key.to_string(),
            None => "Metadata is formated incorrectly".to_string()
        };
        let value = match key_val.get(1) {
            Some(value) => value.to_string(),
            None => "Metadata is formated incorrectly".to_string()
        };

        let key_value = KeyValueEntryBuilder::new()
            .with_key(key.to_string())
            .with_value(value.to_string())
            .build()
            .unwrap();

        metadata.push(key_value.clone());
    }

    let action = UpdateOrganizationActionBuilder::new()
        .with_org_id(org_id.to_string())
        .with_name(name.to_string())
        .with_address(address.to_string())
        .with_metadata(metadata)
        .build()
        .unwrap();

    let payload = PikePayloadBuilder::new()
        .with_action(Action::UpdateOrganization)
        .with_update_organization(action)
        .build()
        .map_err(|err| RestApiResponseError::UserError(format!("{}", err)))?;

    // Building the Transaction //
    // Building the Batch //
    let batch_list = BatchBuilder::new(
        PIKE_FAMILY_NAME, 
        PIKE_FAMILY_VERSION, 
        &private_key.as_hex()
    )
    .add_transaction(
        &payload.into_proto()?,
        &[PIKE_NAMESPACE.to_string()],
        &[PIKE_NAMESPACE.to_string()],
    )?
    .create_batch_list();

    let batch_list_bytes = batch_list
        .write_to_bytes()
        .expect("Error converting batch list to bytes");

    // Submitting Batches to the Validator //
    let res = reqwest::Client::new()
        .post("http://rest-api:8008/batches")
        .header("Content-Type", "application/octet-stream")
        .body(batch_list_bytes)
        .send()
        .await?
        .text()
        .await?;

    println!("============ update_organization ============");
    println!("!dgc-network! res = {:?}", res);

    Ok(HttpResponse::Ok().body(res))
}