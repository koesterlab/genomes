async function fetchChrom(chrom, fr, to) {
    const rs = await fetch('/api/v1/reference/' + chrom +'/' + fr + '/' + to);
    const result = await rs.json();
    return result;
}

async function fetchVariants(chrom, fr, to) {
    const rs = await fetch('/api/v1/variant/' + chrom +'/' + fr + '/' + to);
    const result = await rs.json();
    return result;
}

async function fetchAlignments(chrom, fr, to) {
    const rs = await fetch('/api/v1/alignment/' + chrom +'/' + fr + '/' + to);
    const result = await rs.json();
    const r0 = result[0];
    const r1 = result[1];
    const r = $.merge(r1, r0);
    r.forEach(function(a) {
        let flags = [];
        a.flags.forEach(function(b) {
            if (b === 1) {
                flags[b] = "template having multiple segments in sequencing";
            } else if (b === 2) {
                flags[b] = "each segment properly aligned according to the aligner";
            } else if (b === 4) {
                flags[b] = "segment unmapped";
            } else if (b === 8) {
                a.flags[b] = "next segment in the template unmapped";
            } else if (b === 16) {
                flags[b] = "SEQ being reverse complemented";
            } else if (b === 32) {
                flags[b] = "SEQ of the next segment in the template being reverse complemented";
            } else if (b === 64) {
                flags[b] = "the first segment in the template";
            } else if (b === 128) {
                flags[b] = "the last segment in the template";
            } else if (b === 256) {
                flags[b] = "secondary alignment";
            } else if (b === 512) {
                flags[b] = "not passing filters, such as platform/vendor quality controls";
            } else if (b === 1024) {
                flags[b] = "PCR or optical duplicate";
            } else if (b === 2048) {
                flags[b] = "vega lite lines";
            }
        });
        a.flags = flags;
    });
    return r;
}

async function fetchVegaSpecs() {
    const vlSpec = await fetch( "vegaSpecs.json");
    return vlSpec;
}

let lastLowerBound;
let lastUpperBound;
let reads = new HashMap();
let rows;
let vars = new HashMap();
let var_rows;

let scrolling_locked = false;

// Embed the visualization in the container with id `vis`
async function buildVega(chrom, fr, to) {
    lastLowerBound = fr;
    lastUpperBound = to;

    rows = [];

    var_rows = [];




    for (let i = 1; i < 40; i++) {
        let r = {min_start: -1, max_end: 0};

        rows.push(r);
    }

    for (let j = 1; j < 11; j++) {
        let v = {min_start: -1.0, max_end: 0.0};

        var_rows.push(v);
    }




    const genom = await fetchChrom(chrom, fr, to);
    const body = await genom;

    const variants = await fetchVariants(chrom, fr, to);
    const vabody = await variants;

    const al = await fetchAlignments(chrom, fr, to);
    const albody = await al;


    albody.forEach(function (a) {
        if(reads.has(a.name)) {
            a.row = reads.get(a.name).row;
        } else {
            for (i = 1; i < 39; i++) {
                if (rows[i].min_start === -1) { //read zeile ist leer
                    a.row = i;
                    rows[i].min_start = a.read_start;
                    rows[i].max_end = a.read_end;
                    reads.set(a.name,a);
                    break;
                } else if (rows[i].max_end < a.read_start) {
                    a.row = i;
                    rows[i].max_end = a.read_end;
                    reads.set(a.name,a);
                    break;
                } else if (rows[i].min_start > a.read_end) {
                    a.row = i;
                    rows[i].min_start = a.read_start;
                    reads.set(a.name,a);
                    break;
                }
            }
        }

    });

    vabody.sort(function(a, b) {
        return a.start_position < b.start_position;
    });

    vabody.forEach(function (a) {
        if (vars.has(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives)) {
            a.row = vars.get(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives).row;
        } else {
            for (i = 1; i < 10; i++) {
                if (var_rows[i].min_start === -1.0) { //varianten zeile ist leer
                    a.row = -i;
                    var_rows[i].min_start = a.start_position;
                    var_rows[i].max_end = a.end_position;
                    vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                    break;
                } else if (var_rows[i].max_end <= a.start_position) {
                    a.row = -i;
                    var_rows[i].max_end = a.end_position;
                    vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                    break;
                } else if (var_rows[i].min_start >= a.end_position) {
                    a.row = -i;
                    var_rows[i].min_start = a.start_position;
                    vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                    break;
                }
            }
        }
    });



    const with_variants = $.merge(body, vabody);
    const cont = $.merge(with_variants, albody);


    cont.forEach(function (a) {
        if (a.marker_type === "A" || a.marker_type === "G" || a.marker_type === "T" || a.marker_type === "C") {
            a.base = a.marker_type;
        } else if (a.marker_type === "Deletion" || a.marker_type === "Match" || a.marker_type === "Insertion" || a.marker_type === "Pairing" || a.marker_type === "Inversion" || a.marker_type === "Duplicate") {
            a.typ = a.marker_type;
        }
        if (a.marker_type === "Insertion") {
            a.inserts = a.bases;
        }
    });




    const spec = await fetchVegaSpecs();
    const vlSpec = await spec.json();
    vlSpec.width = $(window).width() - 150;
    vlSpec.scales[0].domain = [fr,to];
    var v = await vegaEmbed('#vis', vlSpec);
    v = v.view.insert("fasta", cont);


    v.addEventListener('mouseup', async function (event, item) {
        if (!scrolling_locked) {
            scrolling_locked = true;
            const lowerBound = Math.round(v.getState().signals.grid.start_position[0]);
            const upperBound = Math.round(v.getState().signals.grid.start_position[1]);

            var upd1 = [];
            var upd2 = [];
            var upd = [];


            if (lastUpperBound < upperBound) {
                const n = await fetchChrom(chrom, lastUpperBound, upperBound);
                const upper_upd_ref = await n;

                const l = await fetchVariants(chrom, lastUpperBound, upperBound);
                const upper_upd_var = await l;

                const m = await fetchAlignments(chrom, lastUpperBound, upperBound);
                var upper_upd_al = await m;


                upper_upd_var.forEach(function (a) {
                    if (vars.has(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives)) {
                        a.row = vars.get(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives).row;
                    } else {
                        for (i = 1; i < 10; i++) {
                            if (var_rows[i].min_start === -1.0) { //varianten zeile ist leer
                                a.row = -i;
                                var_rows[i].min_start = a.start_position;
                                var_rows[i].max_end = a.end_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            } else if (var_rows[i].max_end <= a.start_position) {
                                a.row = -i;
                                var_rows[i].max_end = a.end_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            } else if (var_rows[i].min_start >= a.end_position) {
                                a.row = -i;
                                var_rows[i].min_start = a.start_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            }
                        }
                    }
                });

                upper_upd_al.forEach(function (a) {
                    if(reads.has(a.name)) {
                        a.row = reads.get(a.name).row;
                    } else {
                        for (i = 1; i < 39; i++) {
                            if (rows[i].min_start === -1) { //read zeile ist leer
                                a.row = i;
                                rows[i].min_start = a.read_start;
                                rows[i].max_end = a.read_end;
                                reads.set(a.name,a);
                                break;
                            } else if (rows[i].max_end < a.read_start) {
                                a.row = i;
                                rows[i].max_end = a.read_end;
                                reads.set(a.name,a);
                                break;
                            } else if (rows[i].min_start > a.read_end) {
                                a.row = i;
                                rows[i].min_start = a.read_start;
                                reads.set(a.name,a);
                                break;
                            }
                        }
                    }
                });

                var with_variants = $.merge(upper_upd_al, upper_upd_var);
                upd1 = $.merge(with_variants, upper_upd_ref);

                for (let j = 1; j < 10; j++) {
                    if (var_rows[j].min_start < lowerBound) {
                        var_rows[j].min_start = lowerBound;
                    } else if (var_rows[j].max_end > upperBound) {
                        var_rows[j].max_end = upperBound;
                    }
                }

            }

            if (lastLowerBound > lowerBound) {
                const o = await fetchChrom(chrom, lowerBound, lastLowerBound);
                const lower_upd_ref = await o;

                const q = await fetchVariants(chrom, lowerBound, lastLowerBound);
                const lower_upd_var = await q;

                const p = await fetchAlignments(chrom, lowerBound, lastLowerBound);
                var lower_upd_al = await p;

                lower_upd_var.sort(function (a, b) {
                    return a.start_position < b.start_position;
                });


                lower_upd_var.forEach(function (a) {
                    if (vars.has(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives)) {
                        a.row = vars.get(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives).row;
                    } else {
                        for (i = 1; i < 10; i++) {
                            if (var_rows[i].min_start === -1.0) { //varianten zeile ist leer
                                a.row = -i;
                                var_rows[i].min_start = a.start_position;
                                var_rows[i].max_end = a.end_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            } else if (var_rows[i].max_end <= a.start_position) {
                                a.row = -i;
                                var_rows[i].max_end = a.end_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            } else if (var_rows[i].min_start >= a.end_position) {
                                a.row = -i;
                                var_rows[i].min_start = a.start_position;
                                vars.set(a.var_type + a.start_position + a.end_position + a.reference + a.alternatives,a);
                                break;
                            }
                        }
                    }
                });

                lower_upd_al.sort(function (a, b) {
                    return a.read_start < b.read_start;
                });

                lower_upd_al.forEach(function (a) {
                    if(reads.has(a.name)) {
                        a.row = reads.get(a.name).row;
                    } else {
                        for (i = 1; i < 39; i++) {
                            if (rows[i].min_start === -1) { //read zeile ist leer
                                a.row = i;
                                rows[i].min_start = a.read_start;
                                rows[i].max_end = a.read_end;
                                reads.set(a.name,a);
                                break;
                            } else if (rows[i].max_end < a.read_start) {
                                a.row = i;
                                rows[i].max_end = a.read_end;
                                reads.set(a.name,a);
                                break;
                            } else if (rows[i].min_start > a.read_end) {
                                a.row = i;
                                rows[i].min_start = a.read_start;
                                reads.set(a.name,a);
                                break;
                            }
                        }
                    }
                });

                let with_variants2 = $.merge(lower_upd_al, lower_upd_var);
                upd2 = $.merge(with_variants2, lower_upd_ref);


            }

            if (lastUpperBound > upperBound) {
                for (let j = 1; j < 10; j++) {
                    if (var_rows[j].max_end > upperBound) {
                        var_rows[j].max_end = upperBound;
                    }
                }
            }

            if (lastLowerBound < lowerBound) {
                for (let j = 1; j < 10; j++) {
                    if (var_rows[j].min_start < lowerBound) {
                        var_rows[j].min_start = lowerBound;
                    }
                }
            }

            upd = $.merge(upd1, upd2);

            upd.forEach(function (a) {
                if (a.marker_type === "A" || a.marker_type === "G" || a.marker_type === "T" || a.marker_type === "C") {
                    a.base = a.marker_type;
                } else if (a.marker_type === "Deletion" || a.marker_type === "Match" || a.marker_type === "Insertion" || a.marker_type === "Pairing" || a.marker_type === "Inversion" || a.marker_type === "Duplicate") {
                    a.typ = a.marker_type;
                }
                if (a.marker_type === "Insertion") {
                    a.inserts = a.bases;
                }
            });

            v.change('fasta', vega.changeset().insert(upd).remove(function (d) {
                return (((d.end_position - 0.5 < lowerBound) || (d.start_position + 0.5 > upperBound)));
            }));

            lastLowerBound = lowerBound;
            lastUpperBound = upperBound;
            scrolling_locked = false;
        }
    });
}


