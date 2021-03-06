#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;

extern crate bio;
extern crate bit_vec;
extern crate clap;
extern crate regex;
extern crate rocket_contrib;
extern crate rust_htslib;
extern crate rustc_serialize;
extern crate tera;

mod alignment_reader;
mod fasta_reader;
mod json_generator;
mod report;
mod static_reader;
mod variant_reader;

#[cfg(test)]
mod alignment_tests;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod report_tests;
#[cfg(test)]
mod variant_tests;

use alignment_reader::{get_reads, AlignmentMatch, AlignmentNucleobase};
use clap::{App, Arg, ArgMatches, SubCommand};
use fasta_reader::{read_fasta, Nucleobase};
use json_generator::{create_data, manipulate_json};
use report::make_report;
use rocket::State;
use rocket_contrib::compression::Compression;
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, stdout, Write};
use std::path::Path;
use std::str::FromStr;
use tera::{Context, Tera};
use variant_reader::{read_indexed_vcf, Variant};

#[get("/reference/<chromosome>/<from>/<to>")]
fn reference(
    params: State<ArgMatches>,
    chromosome: String,
    from: u64,
    to: u64,
) -> Json<Vec<Nucleobase>> {
    let response = read_fasta(
        Path::new(params.value_of("fasta file").unwrap()),
        chromosome,
        from,
        to,
    );
    Json(response)
}

#[get("/alignment/<chromosome>/<from>/<to>")]
fn alignment(
    params: State<ArgMatches>,
    chromosome: String,
    from: u64,
    to: u64,
) -> Json<(Vec<AlignmentNucleobase>, Vec<AlignmentMatch>)> {
    let response = get_reads(
        Path::new(params.value_of("bam file").unwrap()),
        Path::new(params.value_of("fasta file").unwrap()),
        chromosome,
        from,
        to,
    );
    Json(response)
}

#[get("/variant/<chromosome>/<from>/<to>")]
fn variant(
    params: State<ArgMatches>,
    chromosome: String,
    from: u64,
    to: u64,
) -> Json<Vec<Variant>> {
    let response = read_indexed_vcf(
        Path::new(params.value_of("vcf file").unwrap()),
        chromosome,
        from,
        to,
    );
    Json(response)
}

#[get("/")]
fn index(params: State<ArgMatches>) -> Template {
    let mut context = HashMap::new();
    context.insert(
        "variants",
        make_report(
            Path::new(params.value_of("vcf file").unwrap()),
            Path::new(params.value_of("fasta file").unwrap()),
            Path::new(params.value_of("bam file").unwrap()),
            params.value_of("chromosome").unwrap().parse().unwrap(),
        )
        .unwrap(),
    );

    Template::render("report", &context)
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("gensbock")
        .version("1.0")
        .author("Felix W. <fxwiegand@wgdnet.de>")
        .about("genome viewing in rust")
        .subcommand(
            SubCommand::with_name("server")
                .about("starts server")
                .version("1.0")
                .author("Felix W. <fxwiegand@wgdnet.de>")
                .arg(
                    Arg::with_name("bam file")
                        .required(true)
                        .help("your input bam file")
                        .index(1),
                )
                .arg(
                    Arg::with_name("fasta file")
                        .required(true)
                        .help("your input fasta file")
                        .index(2),
                )
                .arg(
                    Arg::with_name("vcf file")
                        .required(true)
                        .help("your input vcf file")
                        .index(3),
                ),
        )
        .subcommand(
            SubCommand::with_name("static")
                .about("outputs vega specs")
                .version("1.0")
                .author("Felix W. <fxwiegand@wgdnet.de>")
                .arg(
                    Arg::with_name("bam file")
                        .required(true)
                        .help("your input bam file")
                        .index(1),
                )
                .arg(
                    Arg::with_name("fasta file")
                        .required(true)
                        .help("your input fasta file")
                        .index(2),
                )
                .arg(
                    Arg::with_name("vcf file")
                        .required(true)
                        .help("your input vcf file")
                        .index(3),
                )
                .arg(
                    Arg::with_name("chromosome")
                        .required(true)
                        .help("the chromosome you want to visualize")
                        .index(4),
                )
                .arg(
                    Arg::with_name("from")
                        .required(true)
                        .help("the start of the region you want to visualize")
                        .index(5),
                )
                .arg(
                    Arg::with_name("to")
                        .required(true)
                        .help("the end of the region you want to visualize")
                        .index(6),
                ),
        )
        .subcommand(
            SubCommand::with_name("report")
                .arg(
                    Arg::with_name("bam file")
                        .required(true)
                        .help("your input bam file")
                        .index(1),
                )
                .arg(
                    Arg::with_name("fasta file")
                        .required(true)
                        .help("your input fasta file")
                        .index(2),
                )
                .arg(
                    Arg::with_name("vcf file")
                        .required(true)
                        .help("your input vcf file")
                        .index(3),
                )
                .arg(
                    Arg::with_name("chromosome")
                        .required(true)
                        .help("the chromosome you want to visualize")
                        .index(4),
                )
                .arg(
                    Arg::with_name("render")
                        .short("r")
                        .required(false)
                        .help("write html to stdout"),
                ),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("server") => {
            let params = matches.subcommand_matches("server").unwrap().clone();

            rocket::ignite()
                .manage(params)
                .mount("/", StaticFiles::from("static"))
                .mount("/api/v1", routes![reference, alignment, variant])
                .attach(Compression::fairing())
                .launch();
            Ok(())
        }
        Some("static") => {
            let static_matches = matches.subcommand_matches("static").unwrap();

            let fasta_path = Path::new(static_matches.value_of("fasta file").unwrap());
            let bam_path = Path::new(static_matches.value_of("bam file").unwrap());
            let vcf_path = Path::new(static_matches.value_of("vcf file").unwrap());
            let chromosome = String::from(static_matches.value_of("chromosome").unwrap());
            let from = u64::from_str(static_matches.value_of("from").unwrap()).unwrap();
            let to = u64::from_str(static_matches.value_of("to").unwrap()).unwrap();

            let data = create_data(&fasta_path, &vcf_path, &bam_path, chromosome, from, to);
            let out = manipulate_json(data, from, to);

            io::stdout().write(out.to_string().as_bytes())?;
            Ok(())
        }
        Some("report") => {
            let params = matches.subcommand_matches("report").unwrap().clone();

            if params.is_present("render") {
                let mut templates = Tera::default();
                templates
                    .add_raw_template(
                        "report.html.tera",
                        include_str!("../templates/report.html.tera"),
                    )
                    .unwrap();
                let mut context = Context::new();
                context.insert(
                    "variants",
                    &make_report(
                        Path::new(params.value_of("vcf file").unwrap()),
                        Path::new(params.value_of("fasta file").unwrap()),
                        Path::new(params.value_of("bam file").unwrap()),
                        params.value_of("chromosome").unwrap().parse().unwrap(),
                    )?,
                );

                let html = templates.render("report.html.tera", &context).unwrap();

                stdout().write(html.as_bytes())?;
            } else {
                rocket::ignite()
                    .manage(params)
                    .mount("/", routes![index])
                    .attach(Template::fairing())
                    .launch();
            }
            Ok(())
        }
        None => {
            println!("Try using a subcommand. Type help for more.");
            Ok(())
        }
        _ => unreachable!(), // Assuming you've listed all direct children above, this is unreachable
    }
}
