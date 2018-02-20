// src/main.rs

#[macro_use]
extern crate nickel;

extern crate serde_json;
extern crate rustc_serialize;

extern crate rusoto_core;
extern crate rusoto_dynamodb;


use std::collections::HashMap;

// Nickel
use nickel::{Nickel, JsonBody, HttpRouter, Request, Response, MiddlewareResult, MediaType};

use rusoto_core::{default_tls_client, ContainerProvider, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, DeleteItemInput, PutItemInput, AttributeValue, ScanInput};

// rustc_serialize
use rustc_serialize::json::{Json, ToJson};
use serde_json::value::{Value};

// Nickel
use nickel::status::StatusCode::{self, Forbidden};


fn main() {

	let mut server = Nickel::new();
	let mut router = Nickel::router();

	router.get("/health", middleware! { |request, response|
        (StatusCode::Ok, "Healthy!")
    });

	router.get("/hosts", middleware! { |request, response|
		let client = DynamoDbClient::new(default_tls_client().unwrap(), ContainerProvider, Region::UsEast1);
		let mut scan_input: ScanInput = Default::default();
		scan_input.table_name = "hostname-service".to_ascii_lowercase();
        scan_input.projection_expression = Some(String::from("hostname, ip, notes"));

        let mut data_result = "{\"data\":[".to_owned();

		match client.scan(&scan_input) {
			Ok(output) => {
				match output.items {
					Some(scan_items) => {
						for (item_index, item)  in scan_items.iter().enumerate() {
							data_result.push_str("{");
							for (index, key) in item.keys().enumerate() {
								data_result.push_str(&format!("\"{}\":", key));
								match item.get(key) {
									Some(itemAttr) => {
										match itemAttr.s {
										Some(ref value) => data_result.push_str(&format!("\"{}\"", value)),
										None => println!("None")
										}
									}
									None => println!("None")
								}
                                if index != (item.keys().len() - 1) {
                                    data_result.push_str(",");
                                }
							}
                            if item_index != (item.len() - 1) {
                                data_result.push_str("},");
                            } else {
                                data_result.push_str("}");
                            }
						}
					},
					None => println!("No items found")
					}
				},
				Err(error) => return response.send(format!("{}", error))
        }
    
        data_result.push_str("]}");
        (StatusCode::Ok, data_result)

	});


	router.post("/hosts/new", middleware! { |request, response|

		let host = request.json_as::<Host>().unwrap();

        let hostname = host.hostname.to_string();
		let ip = host.ip.to_string();
		let notes = host.notes.to_string();

        let mut put_item = HashMap::new();
        put_item.insert(String::from("hostname"), AttributeValue { s: Some(hostname),  ..Default::default() });
        put_item.insert(String::from("ip"), AttributeValue { s: Some(ip),  ..Default::default() });
        put_item.insert(String::from("notes"), AttributeValue { s: Some(notes),  ..Default::default() });

		let client = DynamoDbClient::new(default_tls_client().unwrap(), ContainerProvider, Region::UsEast1);
		let mut item_input: PutItemInput = Default::default();
		item_input.table_name = "hostname-service".to_ascii_lowercase();
        item_input.item = put_item;

		match client.put_item(&item_input) {
			Ok(_) => (StatusCode::Ok, "Item saved!"),
			Err(e) => return response.send(format!("{}", e))
		}

	});

	router.delete("/hosts", middleware! { |request, response|
        let host = request.json_as::<HostSearch>().unwrap();

        let hostname = host.hostname.to_string();
        let mut delete_item = HashMap::new();
        delete_item.insert(String::from("hostname"), AttributeValue { s: Some(hostname),  ..Default::default() });

		let client = DynamoDbClient::new(default_tls_client().unwrap(), ContainerProvider, Region::UsEast1);
		let mut delete_item_input: DeleteItemInput = Default::default();
		delete_item_input.table_name = "hostname-service".to_ascii_lowercase();
        delete_item_input.key = delete_item;

		match client.delete_item(&delete_item_input) {
			Ok(_) => (StatusCode::Ok, "Item deleted!"),
			Err(e) => return response.send(format!("{}", e))
		}
	});

	server.utilize(router);

	server.listen("0.0.0.0:9000");
}

#[derive(RustcDecodable, RustcEncodable)]
struct Host {
	hostname: String,
	ip: String,
	notes: String
}
#[derive(RustcDecodable, RustcEncodable)]
struct HostSearch {
	hostname: String,
}

