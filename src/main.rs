#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde;
#[macro_use] extern crate serde_json;

extern crate rocket_contrib;
extern crate bit_vec;
extern crate bio;
extern crate rust_htslib;
extern crate jsonm;
extern crate rustc_serialize;

mod alignment_reader;
mod fasta_reader;
mod variant_reader;
mod json_generator;

#[cfg(test)] mod tests;

use std::path::Path;
use std::env;
use std::process::Command;
use std::str::FromStr;
use rocket_contrib::json::{Json};
use rocket_contrib::serve::StaticFiles;
use rocket::State;
use jsonm::packer::{PackOptions, Packer};
use alignment_reader::{get_reads, AlignmentNucleobase, AlignmentMatch};
use fasta_reader::read_fasta;
use variant_reader::read_indexed_vcf;
use serde_json::Value;
use json_generator::create_data;


#[get("/reference/<chromosome>/<from>/<to>")]
fn reference(args: State<Vec<String>>, chromosome: String, from: u64, to: u64) -> Json<Value> {
    let response = read_fasta(Path::new(&args[2].clone()), chromosome, from, to);
    let mut packer = Packer::new();
    let options = PackOptions::new();
    let packed = packer.pack(&json!(response), &options).unwrap();
    Json(packed)
}

#[get("/alignment/<chromosome>/<from>/<to>")]
fn alignment(args: State<Vec<String>>, chromosome: String, from: u32, to: u32) -> Json<Value> {
    let response = get_reads(Path::new(&args[1].clone()), Path::new(&args[2].clone()) , chromosome, from, to);
    let mut packer = Packer::new();
    let options = PackOptions::new();
    let packed = packer.pack(&json!(response), &options).unwrap();
    Json(packed)
}

#[get("/variant/<chromosome>/<from>/<to>")]
fn variant(args: State<Vec<String>>, chromosome: String, from: u32, to: u32) -> Json<Value> {
    let response = read_indexed_vcf(Path::new(&args[3].clone()), chromosome, from, to);
    let mut packer = Packer::new();
    let options = PackOptions::new();
    let packed = packer.pack(&json!(response), &options).unwrap();
    Json(packed)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if (&args[1].clone()) == "static_json" {
        create_data(Path::new(&args[3].clone()), Path::new(&args[4].clone()), Path::new(&args[2].clone()), String::from(args[5].clone()), u32::from_str(&args[6].clone()).unwrap(), u32::from_str(&args[7].clone()).unwrap());
        //let args= String::from("");
        let a :String = args[6].clone();
        let b :String = args[7].clone();
        let py = String::from("src/jsonMerge.py");

        let output = {
            Command::new("python")
                .arg(py)
                .arg(a)
                .arg(b)
                .output()
                .expect("failed to execute process")
        };


        let msg = output.stdout;

        println!("{}", String::from_utf8(msg).unwrap())
    } else {
        rocket::ignite()
            .manage(args)
            .mount("/",  StaticFiles::from("client"))
            .mount("/api/v1", routes![reference, alignment, variant])
            .launch();
    }
}


