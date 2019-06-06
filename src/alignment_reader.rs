extern crate rust_htslib;
extern crate bit_vec;

use rust_htslib::{bam};
use rust_htslib::prelude::*;
use std::fmt;
use std::path::Path;
use std::collections::BTreeMap;

#[derive(Serialize, Clone)]
pub struct Alignment {
    sequence: String,
    pos: i32,
    length: u16,
    cigar: String,
    flags: BTreeMap<u16, &'static str>,
    name: String,
}

#[derive(Serialize, Clone)]
pub struct AlignmentNucleobase {
    base: char,
    position: i32,
    flags: BTreeMap<u16, &'static str>,
    name: String,
    row: u8,
}

#[derive(Serialize, Clone)]
pub struct Snippet {
    alignment: Alignment,
    row: u8,
}

impl Snippet {
    fn new(alignment: Alignment) -> Snippet {
        Snippet {
            alignment: alignment,
            row: 0,
        }
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut flag_string = String::from("");
        for (_key, flag) in &self.flags {
            flag_string.push_str(flag);
        }
        write!(f, "({}, {}, {}, {}, {})", self.sequence, self.pos, self.cigar, flag_string,
               self.name)
    }
}


pub fn decode_flags(code :u16) -> BTreeMap<u16, &'static str> {
    let mut string_map = BTreeMap::new();

    const FLAG_1: &'static str = "template having multiple segments in sequencing";
    const FLAG_2: &'static str = "each segment properly aligned according to the aligner";
    const FLAG_3: &'static str = "segment unmapped";
    const FLAG_4: &'static str = "next segment in the template unmapped";
    const FLAG_5: &'static str = "SEQ being reverse complemented";
    const FLAG_6: &'static str = "SEQ of the next segment in the template being reverse complemented";
    const FLAG_7: &'static str = "the first segment in the template ";
    const FLAG_8: &'static str = "the last segment in the template";
    const FLAG_9: &'static str = "secondary alignment";
    const FLAG_10: &'static str = "not passing filters, such as platform/vendor quality controls";
    const FLAG_11: &'static str = "PCR or optical duplicate";
    const FLAG_12: &'static str = "supplementary alignment";

    let mut flags_map = BTreeMap::new();
    flags_map.insert(0x1, FLAG_1);
    flags_map.insert(0x2, FLAG_2);
    flags_map.insert(0x4, FLAG_3);
    flags_map.insert(0x8, FLAG_4);
    flags_map.insert(0x10, FLAG_5);
    flags_map.insert(0x20, FLAG_6);
    flags_map.insert(0x40, FLAG_7);
    flags_map.insert(0x80, FLAG_8);
    flags_map.insert(0x100, FLAG_9);
    flags_map.insert(0x200, FLAG_10);
    flags_map.insert(0x400, FLAG_11);
    flags_map.insert(0x800, FLAG_12);

    for (flag, text) in flags_map {
        if (flag & code) == flag {
            string_map.insert(flag, text);
        }
    }

    string_map
}

pub fn count_alignments(path: &Path)-> u32 {
    let mut bam = bam::Reader::from_path(path).unwrap();
    //let header = bam::Header::from_template(bam.header());
    let mut count:u32= 0;
    for _r in bam.records() {
        count += 1;
    }

    count
}

pub fn read_indexed_bam(path: &Path, chrom: u8, from: u32, to: u32) -> Vec<Alignment> {
    let chr = chrom.to_string();
    let c = chr.as_bytes();
    let mut bam = bam::IndexedReader::from_path(&path).unwrap();
    let tid = bam.header().tid(c).unwrap();

    let mut alignments: Vec<Alignment> = Vec::new();

    bam.fetch(tid, from, to).unwrap();

    for r in bam.records() {

        let rec = r.unwrap();

        let a = make_alignment(rec);

        alignments.push(a);
    }

    alignments
}


pub fn read_bam(path: &Path) -> Vec<Alignment> {
    let mut bam = bam::Reader::from_path(path).unwrap();
    let header = bam::Header::from_template(bam.header());

    let mut alignments:Vec<Alignment> = Vec::new();

    for r in bam.records() {
        let record = r.unwrap();
        let _head = header.to_bytes();

        let read = make_alignment(record);

        alignments.push(read);
    }

    alignments

}


fn make_alignment(record: bam::Record) -> Alignment {

    //Cigar String
    let cigstring = record.cigar();

    //Position
    let pos = record.pos();

    //Länge
    let le = record.seq().len() as u16;

    //Sequenz
    let seq = record.seq().as_bytes();
    let mut sequenz = String::from("");
    for b in seq {
        sequenz.push(b as char);
    }

    //Flags
    let flgs = record.flags();
    let flag_string = decode_flags(flgs);

    //Name
    let n = record.qname();
    let mut name = String::from("");
    for a in n {
        name.push(*a as char);
    }

    let read = Alignment {
        sequence: sequenz,
        pos: pos,
        length: le,
        cigar: cigstring.to_string(),
        flags: flag_string,
        name: name,
    };

    read
}

fn calculate_read_row(reads: Vec<Alignment>) -> Vec<Snippet> {
    let mut read_ends = vec![0; 30];
    let mut snippets: Vec<Snippet> = Vec::new();

    for alignment in reads {
        let position = alignment.pos;
        let length = alignment.length as i32;
        let mut snippet = Snippet::new(alignment);

        for i in 1..30 {
            if position > read_ends[i] as i32 {
                snippet.row = i as u8;
                read_ends[i] = length + position;
                break;
            }
        }
        snippets.push(snippet);
    }

    snippets
}

fn make_nucleobases(snippets: Vec<Snippet>) -> Vec<AlignmentNucleobase> {


    let mut bases: Vec<AlignmentNucleobase> = Vec::new();
    for s in snippets {
        let mut offset = 0;
        let base_string = s.alignment.sequence.clone();
        for b in base_string.chars() {
            let snip = s.clone();
            let p= snip.alignment.pos + offset;
            let f = snip.alignment.flags;
            let n = snip.alignment.name;
            let r= snip.row;

            let base = AlignmentNucleobase {
                base: b,
                position: p,
                flags: f,
                name: n,
                row: r,
            };
            offset +=1;
            bases.push(base);
        }
    }
    bases
}

pub fn get_reads(path: &Path, chrom: u8, from: u32, to: u32) -> Vec<AlignmentNucleobase> {
    let alignments = read_indexed_bam(path, chrom, from, to);
    let snippets = calculate_read_row(alignments);
    let bases = make_nucleobases(snippets);

    bases
}