extern crate rust_htslib;

use rust_htslib::{bam, bam::Read};
use std::fmt;
use std::path::Path;
use rust_htslib::bam::record::CigarStringView;
use fasta_reader::{read_fasta};

#[derive(Serialize, Clone, Debug)]
pub enum Marker {
    A,
    T,
    C,
    G,
    N,
    Deletion,
    Insertion,
    Match,
    Pairing
}

#[derive(Clone)]
pub struct Alignment {
    sequence: String,
    pos: i32,
    length: u16,
    flags: Vec<u16>,
    name: String,
    cigar: CigarStringView,
    paired: bool,
    mate_pos: i32,
    tid: i32,
    mate_tid: i32
}

#[derive(Serialize, Clone)]
pub struct AlignmentNucleobase {
    pub marker_type: Marker,
    pub bases: String,
    pub start_position: f64,
    pub end_position: f64,
    pub flags: Vec<u16>,
    pub name: String,
    pub read_start: u32,
    pub read_end: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct AlignmentMatch {
    pub marker_type: Marker,
    pub start_position: f64,
    pub end_position: f64,
    pub flags: Vec<u16>,
    pub name: String,
    pub read_start: u32,
    pub read_end: u32,
}


impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.sequence, self.pos, self.cigar,
               self.name)
    }
}

pub fn decode_flags(code :u16) -> Vec<u16> {

    let mut flags_map = Vec::new();
    flags_map.push(0x1);
    flags_map.push(0x2);
    flags_map.push(0x4);
    flags_map.push(0x8);
    flags_map.push(0x10);
    flags_map.push(0x20);
    flags_map.push(0x40);
    flags_map.push(0x80);
    flags_map.push(0x100);
    flags_map.push(0x200);
    flags_map.push(0x400);
    flags_map.push(0x800);

    let mut read_map = Vec::new();

    for flag in flags_map {
        if (flag & code.clone()) == flag {
            read_map.push(flag);
        }
    }

    read_map
}

pub fn read_indexed_bam(path: &Path, chrom: String, from: u32, to: u32) -> Vec<Alignment> {
    let mut bam = bam::IndexedReader::from_path(&path).unwrap();
    let tid = bam.header().tid(chrom.as_bytes()).unwrap();

    let mut alignments: Vec<Alignment> = Vec::new();

    bam.fetch(tid, from, to).unwrap();

    for r in bam.records() {

        let rec = r.unwrap();

        let a = make_alignment(rec);

        alignments.push(a);
    }

    alignments
}


fn make_alignment(record: bam::Record) -> Alignment {

    let has_pair = record.is_paired();

    let mate_pos = record.mpos();

    let tid = record.tid();
    let mtid = record.mtid();

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
        cigar: cigstring,
        flags: flag_string,
        name: name,
        paired: has_pair,
        mate_pos: mate_pos,
        tid: tid,
        mate_tid: mtid,
    };

    read
}

pub fn make_nucleobases(fasta_path: &Path, chrom: String, snippets: Vec<Alignment>, from: u32, to: u32) -> (Vec<AlignmentNucleobase>, Vec<AlignmentMatch>) {
    let mut bases: Vec<AlignmentNucleobase> = Vec::new();
    let mut matches: Vec<AlignmentMatch> = Vec::new();

    let ref_bases = read_fasta(fasta_path, chrom, from as u64, to as u64);

    for s in snippets {
        let mut cigar_offset: i32 = 0;
        let mut read_offset: i32 = 0;
        let base_string = s.sequence.clone();
        let char_vec: Vec<char> = base_string.chars().collect();

        let mut soft_clip_begin = true;

        let p = s.clone();

        if p.paired && (p.pos + p.length as i32) < p.mate_pos && p.tid == p.mate_tid {
            let pairing = AlignmentMatch {
                marker_type: Marker::Pairing,
                start_position: (p.pos + p.length as i32) as f64 - 0.5,
                end_position: p.mate_pos as f64 - 0.5,
                flags: p.flags.clone(),
                name: p.name.clone(),
                read_start: p.pos.clone() as u32,
                read_end: (p.mate_pos.clone() + 100) as u32,
            };

            matches.push(pairing);
        }

        for c in s.cigar.iter() {
            let mut match_count = 0;
            let mut match_start = 0;
            let mut match_ending = false;

            match c {
                rust_htslib::bam::record::Cigar::Match(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::Match(*c).len() {
                        let snip = s.clone();
                        let b = char_vec[cigar_offset as usize];


                        if snip.pos + read_offset >= from as i32 && snip.pos + read_offset < to as i32 {
                            let ref_index = snip.pos + read_offset - from as i32;
                            let ref_base = &ref_bases[ref_index as usize];


                            let m: Marker;
                            if ref_base.get_marker_type() == b {
                                // Create long rule while bases match
                                if match_count == 0 {
                                    match_start = snip.pos as i32 + read_offset;
                                }
                                match_count += 1;
                                match_ending = true;

                                //m = Marker::Match; // Match with reference fasta
                            } else {
                                match b { // Mismatch
                                    'A' => m = Marker::A,
                                    'T' => m = Marker::T,
                                    'C' => m = Marker::C,
                                    'N' => m = Marker::N,
                                    'G' => m = Marker::G,
                                    _ => m = Marker::Deletion,
                                }

                            let p = snip.pos as i32 + read_offset;
                            let f = snip.flags;
                            let n = snip.name;

                            let rs: i32;
                            let re: i32;

                            if snip.paired {
                                if snip.pos < snip.mate_pos {
                                    re = snip.mate_pos + 100;
                                    rs = snip.pos;
                                } else {
                                    rs = snip.mate_pos;
                                    re = snip.pos as i32 + snip.length as i32;
                                }
                            } else {
                                rs = snip.pos;
                                re = snip.pos as i32 + snip.length as i32;
                            }

                                if match_count > 0 {
                                    // First mismatch detection must lead to new creation of all previous matches
                                    let mtch = AlignmentMatch {
                                        marker_type: Marker::Match,
                                        start_position: match_start as f64 - 0.5,
                                        end_position: (match_start + match_count - 1) as f64 + 0.5,
                                        flags: f.clone(),
                                        name: n.clone(),
                                        read_start: rs.clone() as u32,
                                        read_end: re.clone() as u32,
                                    };

                                    match_count = 0;
                                    match_start = 0;

                                    match_ending = false;
                                    matches.push(mtch);

                                }


                                let base = AlignmentNucleobase {
                                marker_type: m,
                                bases: b.to_string(),
                                start_position: p.clone() as f64 - 0.5,
                                end_position: p as f64 + 0.5,
                                flags: f,
                                name: n,
                                read_start: rs as u32,
                                read_end: re as u32,
                            };

                            bases.push(base);

                            }

                        }
                        cigar_offset += 1;
                        read_offset += 1;

                    }

                    if match_ending {
                        // Mismatch detection at end

                        let snip = s.clone();
                        let f = snip.flags;
                        let n = snip.name;

                        let rs: i32;
                        let re: i32;

                        if snip.paired {
                            if snip.pos < snip.mate_pos {
                                re = snip.mate_pos + 100;
                                rs = snip.pos;
                            } else {
                                rs = snip.mate_pos;
                                re = snip.pos as i32 + snip.length as i32;
                            }
                        } else {
                            rs = snip.pos;
                            re = snip.pos as i32 + snip.length as i32;
                        }

                        let mtch = AlignmentMatch {
                            marker_type: Marker::Match,
                            start_position: match_start as f64 - 0.5,
                            end_position: (match_start + match_count - 1) as f64 + 0.5,
                            flags: f.clone(),
                            name: n.clone(),
                            read_start: rs.clone() as u32,
                            read_end: re.clone() as u32,
                        };

                        matches.push(mtch);
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::Ins(c) => {
                    let snip = s.clone();
                    let p: f64 = snip.pos as f64 + read_offset as f64 - 0.5;
                    let m: Marker = Marker::Insertion;
                    let rs = snip.pos;
                    let re = snip.pos as i32 + snip.length as i32;


                    let mut b = String::from("");
                    for i in 0..rust_htslib::bam::record::Cigar::Ins(*c).len() {

                        let char = char_vec[cigar_offset as usize + i as usize];
                        b.push(char);

                    }

                    cigar_offset += 1;

                    let base = AlignmentNucleobase {
                        marker_type: m,
                        bases: b,
                        start_position: p.clone() as f64 - 0.5,
                        end_position: p as f64 + 0.5,
                        flags: snip.flags,
                        name: snip.name,
                        read_start: rs as u32,
                        read_end: re as u32,
                    };

                    if from as f64 <= (base.start_position + 0.5) && (base.start_position + 0.5) <= to as f64 {
                        bases.push(base);
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::Del(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::Del(*c).len() {
                        let snip = s.clone();
                        let m = Marker::Deletion;
                        let p = snip.pos as i32 + read_offset;
                        let f = snip.flags;
                        let n = snip.name;
                        let rs = snip.pos;
                        let re = snip.pos as i32 + snip.length as i32;
                        let b = String::from("");

                        let base = AlignmentNucleobase {
                            marker_type: m,
                            bases: b,
                            start_position: p.clone() as f64 - 0.5,
                            end_position: p as f64 + 0.5,
                            flags: f,
                            name: n,
                            read_start: rs as u32,
                            read_end: re as u32,
                        };

                        read_offset += 1;

                        if from as f64 <= (base.start_position + 0.5) && (base.start_position + 0.5) <= to as f64 {
                            bases.push(base);
                        }
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::RefSkip(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::RefSkip(*c).len() {
                        //offset += 1;
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::SoftClip(c) => {
                    if soft_clip_begin {

                        for _i in 0..rust_htslib::bam::record::Cigar::SoftClip(*c).len() {
                            let snip = s.clone();
                            let b = char_vec[cigar_offset as usize];

                            if snip.pos + read_offset >= from as i32 && snip.pos + read_offset < to as i32 {
                                let ref_index = snip.pos + read_offset - from as i32;
                                let ref_base = &ref_bases[ref_index as usize];


                                let m: Marker;
                                if ref_base.get_marker_type() == b {
                                    // Create long rule while bases match
                                    if match_count == 0 {
                                        match_start = snip.pos as i32 + read_offset;
                                    }
                                    match_count += 1;
                                    match_ending = true;

                                } else {
                                    match b { // Mismatch
                                        'A' => m = Marker::A,
                                        'T' => m = Marker::T,
                                        'C' => m = Marker::C,
                                        'N' => m = Marker::N,
                                        'G' => m = Marker::G,
                                        _ => m = Marker::Deletion,
                                    }

                                    let p = snip.pos as i32 + read_offset;
                                    let f = snip.flags;
                                    let n = snip.name;

                                    let rs: i32;
                                    let re: i32;

                                    if snip.paired {
                                        if snip.pos < snip.mate_pos {
                                            re = snip.mate_pos + 100;
                                            rs = snip.pos;
                                        } else {
                                            rs = snip.mate_pos;
                                            re = snip.pos as i32 + snip.length as i32;
                                        }
                                    } else {
                                        rs = snip.pos;
                                        re = snip.pos as i32 + snip.length as i32;
                                    }

                                    if match_count > 0 {
                                        // First mismatch detection must lead to new creation of all previous matches
                                        let mtch = AlignmentMatch {
                                            marker_type: Marker::Match,
                                            start_position: match_start as f64 - 0.5,
                                            end_position: (match_start + match_count - 1) as f64 + 0.5,
                                            flags: f.clone(),
                                            name: n.clone(),
                                            read_start: rs.clone() as u32,
                                            read_end: re.clone() as u32,
                                        };

                                        match_count = 0;
                                        match_start = 0;

                                        match_ending = false;
                                        matches.push(mtch);

                                    }

                                    let base = AlignmentNucleobase {
                                        marker_type: m,
                                        bases: b.to_string(),
                                        start_position: p.clone() as f64 - 0.5,
                                        end_position: p as f64 + 0.5,
                                        flags: f,
                                        name: n,
                                        read_start: rs as u32,
                                        read_end: re as u32,
                                    };

                                    bases.push(base);

                                }

                            }

                            cigar_offset += 1;
                        }

                        if match_ending {
                            // Mismatch detection at end

                            let snip = s.clone();
                            let f = snip.flags;
                            let n = snip.name;

                            let rs: i32;
                            let re: i32;

                            if snip.paired {
                                if snip.pos < snip.mate_pos {
                                    re = snip.mate_pos + 100;
                                    rs = snip.pos;
                                } else {
                                    rs = snip.mate_pos;
                                    re = snip.pos as i32 + snip.length as i32;
                                }
                            } else {
                                rs = snip.pos;
                                re = snip.pos as i32 + snip.length as i32;
                            }

                            let mtch = AlignmentMatch {
                                marker_type: Marker::Match,
                                start_position: match_start as f64 - 0.5,
                                end_position: (match_start + match_count - 1) as f64 + 0.5,
                                flags: f.clone(),
                                name: n.clone(),
                                read_start: rs.clone() as u32,
                                read_end: re.clone() as u32,
                            };

                            matches.push(mtch);
                        }

                    } else {
                        for _i in 0..rust_htslib::bam::record::Cigar::SoftClip(*c).len() {
                            let snip = s.clone();
                            let b = char_vec[cigar_offset as usize];

                            if snip.pos + read_offset >= from as i32 && snip.pos + read_offset < to as i32 {
                                let ref_index = snip.pos + read_offset - from as i32;
                                let ref_base = &ref_bases[ref_index as usize];


                                let m: Marker;
                                if ref_base.get_marker_type() == b {
                                    // Create long rule while bases match
                                    if match_count == 0 {
                                        match_start = snip.pos as i32 + read_offset;
                                    }
                                    match_count += 1;
                                    match_ending = true;


                                } else {
                                    match b { // Mismatch
                                        'A' => m = Marker::A,
                                        'T' => m = Marker::T,
                                        'C' => m = Marker::C,
                                        'N' => m = Marker::N,
                                        'G' => m = Marker::G,
                                        _ => m = Marker::Deletion,
                                    }

                                    let p = snip.pos as i32 + read_offset;
                                    let f = snip.flags;
                                    let n = snip.name;

                                    let rs: i32;
                                    let re: i32;

                                    if snip.paired {
                                        if snip.pos < snip.mate_pos {
                                            re = snip.mate_pos + 100;
                                            rs = snip.pos;
                                        } else {
                                            rs = snip.mate_pos;
                                            re = snip.pos as i32 + snip.length as i32;
                                        }
                                    } else {
                                        rs = snip.pos;
                                        re = snip.pos as i32 + snip.length as i32;
                                    }

                                    if match_count > 0 {
                                        // First mismatch detection must lead to new creation of all previous matches
                                        let mtch = AlignmentMatch {
                                            marker_type: Marker::Match,
                                            start_position: match_start as f64 - 0.5,
                                            end_position: (match_start + match_count - 1) as f64 + 0.5,
                                            flags: f.clone(),
                                            name: n.clone(),
                                            read_start: rs.clone() as u32,
                                            read_end: re.clone() as u32,
                                        };

                                        match_count = 0;
                                        match_start = 0;

                                        match_ending = false;
                                        matches.push(mtch);

                                    }

                                    let base = AlignmentNucleobase {
                                        marker_type: m,
                                        bases: b.to_string(),
                                        start_position: p.clone() as f64 - 0.5,
                                        end_position: p as f64 + 0.5,
                                        flags: f,
                                        name: n,
                                        read_start: rs as u32,
                                        read_end: re as u32,
                                    };

                                    bases.push(base);

                                }

                            }
                            cigar_offset += 1;
                            read_offset += 1;
                        }

                        if match_ending {
                            // Mismatch detection at end

                            let snip = s.clone();
                            let f = snip.flags;
                            let n = snip.name;

                            let rs: i32;
                            let re: i32;

                            if snip.paired {
                                if snip.pos < snip.mate_pos {
                                    re = snip.mate_pos + 100;
                                    rs = snip.pos;
                                } else {
                                    rs = snip.mate_pos;
                                    re = snip.pos as i32 + snip.length as i32;
                                }
                            } else {
                                rs = snip.pos;
                                re = snip.pos as i32 + snip.length as i32;
                            }

                            let mtch = AlignmentMatch {
                                marker_type: Marker::Match,
                                start_position: match_start as f64 - 0.5,
                                end_position: (match_start + match_count - 1) as f64 + 0.5,
                                flags: f.clone(),
                                name: n.clone(),
                                read_start: rs.clone() as u32,
                                read_end: re.clone() as u32,
                            };

                            matches.push(mtch);
                        }

                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::HardClip(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::HardClip(*c).len() {
                        cigar_offset += 1;
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::Pad(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::Pad(*c).len() {
                        //offset += 1;
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::Equal(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::Equal(*c).len() {
                        //offset += 1;
                    }

                    soft_clip_begin = false;

                }
                rust_htslib::bam::record::Cigar::Diff(c) => {
                    for _i in 0..rust_htslib::bam::record::Cigar::Diff(*c).len() {
                        //offset += 1;
                    }

                    soft_clip_begin = false;

                }
            }
        }
    }
    (bases, matches)
}



pub fn get_reads(path: &Path, fasta_path: &Path, chrom: String, from: u32, to: u32) -> (Vec<AlignmentNucleobase>, Vec<AlignmentMatch>) {
    let alignments = read_indexed_bam(path,chrom.clone(), from, to);
    let bases = make_nucleobases(fasta_path, chrom, alignments, from, to);

    bases
}