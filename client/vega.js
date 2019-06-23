async function fetchChrom(chrom, fr, to) {
    const rs = await fetch('/api/v1/reference/' + chrom +'/' + fr + '/' + to);
    return rs;
}

async function fetchVariants(chrom, fr, to) {
    const rs = await fetch('/api/v1/variant/' + chrom +'/' + fr + '/' + to);
    return rs;
}

async function fetchAlignments(chrom, fr, to) {
    const rs = await fetch('/api/v1/alignment/' + chrom +'/' + fr + '/' + to);
    return rs;
}

async function fetchVegaSpecs() {
    const vlSpec = await fetch( "vlSpec.json");
    return vlSpec;
}

let lastLowerBound;
let lastUpperBound;
let reads;
let rows;


// Embed the visualization in the container with id `vis`
async function buildVega(chrom, fr, to) {
    lastLowerBound = fr;
    lastUpperBound = to;

    reads = [];
    rows = [];
    for (let i = 1; i < 30; i++) {
        let r = {min_start: -1, max_end: 0};
        rows.push(r);
    }




    const genom = await fetchChrom(chrom, fr, to);
    const body = await genom.json();

    const variants = await fetchVariants(chrom, fr, to);
    const vabody = await variants.json();

    const al = await fetchAlignments(chrom, fr, to);
    const albody = await al.json();

    albody.forEach(function (a) {
        var already_in = false;
        //if not in read rows
        reads.forEach(function (r) {
            if (a.name == r.name) {
                a.row = r.row;
                already_in = true;
            }
        });
        if (!already_in) {
            for (let i = 1; i < 30; i++) {
                if (rows[i].min_start == -1) { //read zeile ist leer
                    a.row = i;
                    rows[i].min_start = a.read_start;
                    rows[i].max_end = a.read_end;
                    var read = {name:a.name, read_end:a.read_end, row: a.row};
                    reads.push(read);
                    break;
                } else if (rows[i].max_end < a.read_start) {
                    a.row = i;
                    rows[i].max_end = a.read_end;
                    var read = {name:a.name, read_end:a.read_end, row: a.row};
                    reads.push(read);
                    break;
                } else if (rows[i].min_start > a.read_end) {
                    a.row = i;
                    rows[i].min_start = a.read_start;
                    var read = {name:a.name, read_end:a.read_end, row: a.row};
                    reads.push(read);
                    break;
                }

            }
        }


    });



    const with_variants = $.merge(body, vabody);
    const cont = $.merge(with_variants, albody);




    const spec = await fetchVegaSpecs();
    const vlSpec = await spec.json();
    vlSpec.width = $(window).width() - 150;
    vlSpec.encoding.x.scale.domain = [fr,to];
    let v = await vegaEmbed('#vis', vlSpec);
    v = v.view.insert("fasta", cont);


   v.addEventListener('mouseup', async function (event, item) {
        const lowerBound = Math.round(v.getState().signals.grid.position[0]);
        const upperBound = Math.round(v.getState().signals.grid.position[1]);

        let upd1 = [];
        let upd2 = [];
        let upd = [];


        if (lastUpperBound < upperBound) {
            const n = await fetchChrom(chrom, lastUpperBound, upperBound);
            const upper_upd_ref = await n.json();

            const l = await fetchVariants(chrom, lastUpperBound, upperBound);
            const upper_upd_var = await l.json();

            const m = await fetchAlignments(chrom, lastUpperBound, upperBound);
            var upper_upd_al = await m.json();

            upper_upd_al.forEach(function (a) {
                var already_in = false;
                var pos_found = false;
                //if not in read rows
                reads.forEach(function (r) {
                    if (a.name == r.name) {
                        a.row = r.row;
                        already_in = true;
                    }
                });
                if (!already_in) {
                    for (let i = 1; i < 30; i++) {
                        if (rows[i].min_start == -1) {
                            a.row = i;
                            rows[i].min_start = a.read_start;
                            rows[i].max_end = a.read_end;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        } else if (rows[i].max_end < a.read_start) { //read zeile ist leer
                            a.row = i;
                            rows[i].max_end = a.read_end;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        } else if (rows[i].min_start > a.read_end) {
                            a.row = i;
                            rows[i].min_start = a.read_start;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        }

                    }
                }


            });

            let with_variants = $.merge(upper_upd_al, upper_upd_var);
            upd1 = $.merge(with_variants, upper_upd_ref);



        }

        if (lastLowerBound > lowerBound){
            const o = await fetchChrom(chrom, lowerBound, lastLowerBound);
            const lower_upd_ref = await o.json();

            const q = await fetchVariants(chrom, lowerBound, lastLowerBound);
            const lower_upd_var = await q.json();

            const p = await fetchAlignments(chrom, lowerBound, lastLowerBound);
            var lower_upd_al = await p.json();

            lower_upd_al.forEach(function (a) {
                var already_in = false;
                var pos_found = false;
                //if not in read rows
                reads.forEach(function (r) {
                    if (a.name == r.name) {
                        a.row = r.row;
                        already_in = true;
                    }
                });
                if (!already_in) {
                    for (let i = 1; i < 30; i++) {
                        if (rows[i].min_start == -1) { //read zeile ist leer
                            a.row = i;
                            rows[i].min_start = a.read_start;
                            rows[i].max_end = a.read_end;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        } else if (rows[i].max_end < a.read_start) {
                            a.row = i;
                            rows[i].max_end = a.read_end;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        } else if (rows[i].min_start > a.read_end) {
                            a.row = i;
                            rows[i].min_start = a.read_start;
                            var read = {name:a.name, read_end:a.read_end, row: a.row};
                            reads.push(read);

                            break;
                        }

                    }
                }


            });

            let with_variants2 = $.merge(lower_upd_al, lower_upd_var);
            upd2 = $.merge(with_variants2, lower_upd_ref);
        }

       upd = $.merge(upd1, upd2);

        v.change('fasta', vega.changeset().insert(upd).remove(function (d) {
            return (((d.position < lowerBound) || (d.position > upperBound))) ;
        }));

        // reads = reads.filter(function (f) {
        //     return (f.read_end < lowerBound) || (f.read_start > upperBound)
        // });
        // read entfernen wenn ende < lower bound oder start > upper bound

       lastLowerBound = lowerBound;
       lastUpperBound = upperBound;
    });


}

//TODO: Basen in Reads die wie das Referenzgenom sind grau färben
//TODO: Insertion: Neue Base auf Position 32,5 mit Tooltip mit Basen die Insertions sind