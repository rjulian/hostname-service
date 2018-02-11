// src/main.rs

#[macro_use]
extern crate nickel;

extern crate serde_json;
extern crate rustc_serialize;

extern crate jwt;
extern crate hyper;
extern crate crypto;

#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;

extern crate rusoto_core;
extern crate rusoto_dynamodb;


// Nickel
use nickel::{Nickel, JsonBody, HttpRouter, Request, Response, MiddlewareResult, MediaType};

// MongoDB
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;
use mongodb::error::Result as MongoResult;

use rusoto_core::{default_tls_client, DefaultCredentialsProvider, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, ListTablesInput, ScanInput};
// bson
use bson::{Bson, Document};
use bson::oid::ObjectId;

// rustc_serialize
use rustc_serialize::json::{Json, ToJson};
use serde_json::value::{Value};

// Nickel
use nickel::status::StatusCode::{self, Forbidden};

// hyper
use hyper::header;
use hyper::header::{Authorization, Bearer};

// jwt
use std::default::Default;
use crypto::sha2::Sha256;
use jwt::{
	Header,
	Registered,
	Token,
};

static AUTH_SECRET: &'static str = "some_secret_key";

fn main() {

	let mut server = Nickel::new();
	let mut router = Nickel::router();

	fn get_data_string(result: MongoResult<Document>) -> Result<Value, String> {
		match result {
			Ok(doc) => Ok(Bson::Document(doc).to_json()),
			Err(e) => Err(format!("{}", e))
		}
	}

	router.get("/users", middleware! { |request, response|
		let credentials = DefaultCredentialsProvider::new().unwrap();
		let client = DynamoDbClient::new(default_tls_client().unwrap(), credentials, Region::UsEast1);
		let mut scan_input: ScanInput = Default::default();
		scan_input.table_name = "metrics-test".to_ascii_lowercase();

        let mut data_result = "{\"data\":[".to_owned();

		match client.scan(&scan_input) {
			Ok(output) => {
				match output.items {
					Some(scan_items) => {
						for item in scan_items {
							data_result.push_str("{");
							for key in item.keys() {
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
                                data_result.push_str(",");
							}
							data_result.push_str("},");
						}
					},
					None => println!("No items found")
					}
				},
				Err(error) => return response.send(format!("{}", e))
        }
    
        data_result.push_str("]}");
        (StatusCode::Ok, data_result)

	});


	router.post("/hosts/new", middleware! { |request, response|

		// Accept a JSON string that corresponds to the User struct
		let host = request.json_as::<Host>().unwrap();

		let hostname = host.hostname.to_string();
		let ip = host.ip.to_string();
		let notes = host.notes.to_string();

		// Connect to the database
		let client = Client::connect("localhost", 27017)
			.ok().expect("Error establishing connection.");

		// The users collection
		let coll = client.db("rust-users").collection("users");

		// Insert one user
		match coll.insert_one(doc! {
			"hostname" => hostname,
			"ip" => ip,
			"notes" => notes
		}, None) {
			Ok(_) => (StatusCode::Ok, "Item saved!"),
			Err(e) => return response.send(format!("{}", e))
		}

	});

	router.delete("/users/:id", middleware! { |request, response|

		let client = Client::connect("localhost", 27017)
			.ok().expect("Failed to initialize standalone client.");

		// The users collection
		let coll = client.db("rust-users").collection("users");

		// Get the objectId from the request params
		let object_id = request.param("id").unwrap();

		// Match the user id to an bson ObjectId
		let id = match ObjectId::with_string(object_id) {
			Ok(oid) => oid,
			Err(e) => return response.send(format!("{}", e))
		};

		match coll.delete_one(doc! {"_id" => id}, None) {
			Ok(_) => (StatusCode::Ok, "Item deleted!"),
			Err(e) => return response.send(format!("{}", e))
		}

	});

	server.utilize(router);

	server.listen("127.0.0.1:9000");
}

#[derive(RustcDecodable, RustcEncodable)]
struct Host {
	hostname: String,
	ip: String,
	notes: String
}

