extern crate bit_vec;
extern crate rust_htslib;

use regex::Regex;
use rust_htslib::bcf::Read;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Variant {
    pub(crate) marker_type: String,
    pub(crate) reference: String,
    pub(crate) alternatives: Option<String>,
    pub(crate) start_position: f64,
    pub(crate) end_position: f64,
    pub(crate) var_type: VariantType,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum VariantType {
    Deletion,
    Insertion,
    Duplicate,
    Inversion,
    Variant,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Report {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) position: i64,
    pub(crate) reference: String,
    pub(crate) alternatives: String,
    pub(crate) ann: Option<Vec<Vec<String>>>,
}

pub fn read_indexed_vcf(path: &Path, chrom: String, from: u64, to: u64) -> Vec<Variant> {
    let mut vcf = rust_htslib::bcf::IndexedReader::from_path(&path).unwrap();

    let rid = vcf.header().name2rid(chrom.as_bytes()).unwrap();

    vcf.fetch(rid, from, to).unwrap();

    let mut variants: Vec<Variant> = Vec::new();

    for r in vcf.records() {
        let mut rec = r.unwrap();

        let pos = rec.pos();
        let end_pos = match rec.info(b"END").integer() {
            Ok(Some(end_pos)) => {
                // Subtraction of 0.5 because of the 0-based positioning in the whole plot
                //TODO: Ask @johanneskoester if the END Info Tag is 0-based or not
                let end_pos = end_pos[0] as f64 - 0.5; // -1 due to 0-basing, + 0.5 du to end pos
                Some(end_pos)
            }
            _ => None,
        };
        let alleles = rec.alleles();

        let ref_vec = alleles[0].to_owned();
        let rfrce = String::from_utf8(ref_vec).unwrap();

        let len: u8 = rfrce.len() as u8;

        for i in 1..alleles.len() {
            let alt = alleles[i];
            let var_string = String::from("Variant");

            if alt == b"<DEL>" {
                let var = Variant {
                    marker_type: var_string,
                    reference: rfrce.clone(),
                    alternatives: None,
                    start_position: pos as f64 - 0.5,
                    end_position: end_pos.unwrap(),
                    var_type: VariantType::Deletion,
                };

                variants.push(var);
            } else if alt == b"<INV>" {
                let rev: String = rfrce.chars().rev().collect();

                let var = Variant {
                    marker_type: var_string,
                    reference: rfrce.clone(),
                    alternatives: Some(rev),
                    start_position: pos as f64 - 0.5,
                    end_position: end_pos.unwrap(),
                    var_type: VariantType::Inversion,
                };

                variants.push(var);
            } else if alt == b"<DUP>" {
                let dup: String = [rfrce.clone(), rfrce.clone()].concat();

                let var = Variant {
                    marker_type: var_string,
                    reference: rfrce.clone(),
                    alternatives: Some(dup),
                    start_position: pos as f64 - 0.5,
                    end_position: end_pos.unwrap(),
                    var_type: VariantType::Duplicate,
                };

                variants.push(var);
            } else {
                let mut allel = String::from("");

                for c in alt {
                    allel.push(*c as char);
                }

                let cnv = Regex::new(r"^<CN\d>$").unwrap();

                if cnv.is_match(allel.as_ref()) {
                    warn!("Use of unsupported Copy-Number-Variation {}", allel) // Warning for Copy-Number-Variation
                } else {
                    if allel.len() == rfrce.len() {
                        let var = Variant {
                            marker_type: var_string,
                            reference: rfrce.clone(),
                            alternatives: Some(allel),
                            start_position: pos as f64 - 0.5,
                            end_position: pos as f64 - 0.5 + len as f64,
                            var_type: VariantType::Variant,
                        };

                        variants.push(var);
                    } else if allel.len() > rfrce.len() {
                        let var = Variant {
                            marker_type: var_string,
                            reference: rfrce.clone(),
                            alternatives: Some(allel),
                            start_position: pos as f64, // start end end + 0.5 due to alignment with insertions from bam
                            end_position: pos as f64 + len as f64,
                            var_type: VariantType::Insertion,
                        };

                        variants.push(var);
                    } else {
                        let var = Variant {
                            marker_type: var_string,
                            reference: rfrce.clone(),
                            alternatives: Some(allel),
                            start_position: pos as f64 + 0.5, // start position + 1 due to alignment with deletions from bam (example: ref: ACTT alt: A  -> deletion is just CTT)
                            end_position: pos as f64 - 0.5 + len as f64,
                            var_type: VariantType::Deletion,
                        };

                        variants.push(var);
                    }
                }
            }
        }
    }

    variants
}
