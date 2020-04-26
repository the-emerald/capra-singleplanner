use std::io::{BufRead, BufReader};
use std::{io, fs};
use capra::common::dive_segment::{DiveSegment, SegmentType};
use serde::{Deserialize, Serialize};
use capra::common::gas::Gas;
use capra::deco::zhl16::ZHL16;
use capra::deco::zhl16::util::{ZHL16B_N2_A, ZHL16B_N2_B, ZHL16B_N2_HALFLIFE, ZHL16B_HE_A, ZHL16B_HE_B, ZHL16B_HE_HALFLIFE};
use capra::dive_plan::open_circuit::OpenCircuit;
use capra::dive_plan::dive::Dive;
use time::Duration;
use capra::common::{DENSITY_FRESHWATER, DENSITY_SALTWATER, time_taken};
use capra::gas_plan::GasPlan;
use tabular::Table;
use tabular::row;
use std::iter::FromIterator;

const DEFAULT_GFL: usize = 100;
const DEFAULT_GFH: usize = 100;

#[derive(Serialize, Deserialize, Debug)]
struct JSONDecoGas {
    o2: usize,
    he: usize,
    modepth: Option<usize>
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONDiveSegment {
    depth: usize,
    time: usize,
    o2: usize,
    he: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONDive {
    gfl: Option<usize>,
    gfh: Option<usize>,
    asc: Option<isize>,
    desc: Option<isize>,
    segments: Vec<JSONDiveSegment>,
    deco_gases: Vec<JSONDecoGas>
}

fn pretty_time(duration: &Duration) -> String {
    let m = duration.whole_minutes();
    let s = duration.whole_seconds() - m*60;
    format!("{}:{:0>2}", m, s)
}

fn main() {
    let mut line: String = "".parse().unwrap();
    let stdin = io::stdin();
    for x in BufReader::new(stdin).lines() {
        line = line.to_owned() + &x.expect("unable to read input") + "\n"
    }

    // let line = fs::read_to_string("samples/sample_rev.json") // Use this for profiling!
    //     .expect("Something went wrong reading the file");

    let mut bottom_segments: Vec<(DiveSegment, Gas)> = Vec::new();
    let mut deco_mixes: Vec<(Gas, Option<usize>)> = Vec::new();

    let js: JSONDive = serde_json::from_str(&line).expect("unable to decode user input");

    let ascent_rate = match js.asc {
        Some(t) => t,
        None => capra::common::DEFAULT_ASCENT_RATE
    };

    let descent_rate = match js.desc {
        Some(t) => t,
        None => capra::common::DEFAULT_DESCENT_RATE
    };

    let gfl = match js.gfl {
        Some(t) => t,
        None => DEFAULT_GFL
    };

    let gfh = match js.gfh {
        Some(t) => t,
        None => DEFAULT_GFH
    };

    for gas in js.deco_gases {
        deco_mixes.push((Gas::new(gas.o2, gas.he, 100 - gas.he - gas.o2)
                             .expect("unable to decode decompression gas"), gas.modepth));
    }

    // bottom_segments.push((
    //     DiveSegment::new(
    //         SegmentType::AscDesc,
    //         0,
    //         js.segments[0].depth,
    //         time_taken(descent_rate, 0, js.segments[0].depth),
    //         ascent_rate,
    //         descent_rate
    //     ).unwrap()
    //     , Gas::new(js.segments[0].o2, js.segments[0].he, 100 - js.segments[0].o2 - js.segments[0].he).unwrap()
    // ));

    for seg in js.segments {
        bottom_segments.push((DiveSegment::new(SegmentType::DiveSegment,
                                               seg.depth, seg.depth, Duration::minutes(seg.time as i64), ascent_rate,
                                               descent_rate).expect("unable to decode segment"),
                           Gas::new(seg.o2, seg.he, 100 - seg.he - seg.o2)
                               .expect("unable to decode bottom gas")));
    }

    let zhl16 = ZHL16::new(
        &Gas::new(21, 0, 79).unwrap(), // This shouldn't error
        ZHL16B_N2_A, ZHL16B_N2_B, ZHL16B_N2_HALFLIFE, ZHL16B_HE_A, ZHL16B_HE_B, ZHL16B_HE_HALFLIFE, gfl, gfh);

    let dive = OpenCircuit::new(zhl16,
                                    &deco_mixes, &bottom_segments, ascent_rate,
                                    descent_rate, DENSITY_SALTWATER, 20, 15);

    // let plan = dive.execute_dive().1
    //     .iter()
    //     .filter(|x| x.0.get_segment_type() != SegmentType::AscDesc) // Filter AscDesc segments
    //     .cloned().collect::<Vec<(DiveSegment, Gas)>>();

    let plan = dive.execute_dive().1; // Use this to include all AscDesc segments

    let mut gas_plan = Vec::from_iter(dive.plan_forwards());
    gas_plan.sort_by(|&(_, a), &(_, b)| b.cmp(&a));

    println!("Ascent rate: {}m/min", ascent_rate);
    println!("Descent rate: {}m/min", descent_rate);
    println!("GFL/GFH: {}/{}\n", gfl, gfh);

    let mut dive_plan_table = Table::new("{:>}  {:>}  {:>}  {:>}  {:>}");
    let mut runtime = Duration::zero();
    dive_plan_table.add_row(row!("Segment", "Depth", "Time", "Runtime", "Gas"));
    for x in plan {
        runtime += *x.0.get_time();
        let gas = format!("{}/{}", x.1.o2(), x.1.he());
        let segment_type = format!("{:?}", x.0.get_segment_type());
        match x.0.get_segment_type() {
            SegmentType::AscDesc => {
                let text = format!("-> {}m", x.0.get_end_depth());
                dive_plan_table.add_row(row!(
                    segment_type,
                    text,
                    pretty_time(x.0.get_time()),
                    pretty_time(&runtime),
                    gas
                ));
            }
            _ => {
                let text = format!("{}m", x.0.get_end_depth());
                dive_plan_table.add_row(row!(
                    segment_type,
                    text,
                    pretty_time(x.0.get_time()),
                    pretty_time(&runtime),
                    gas
                ));
            }
        }
    }
    println!("{}", dive_plan_table);

    let mut gas_plan_table = Table::new("{:>}  {:>}");
    gas_plan_table.add_row(row!("Gas", "Amount"));
    let mut total_gas = 0;

    for (gas, qty) in gas_plan {
        total_gas += qty;
        let gas_str = format!("{}/{}", gas.o2(), gas.he());
        let qty_str = format!("{} litres", qty);
        gas_plan_table.add_row(row!(
        gas_str,
        qty_str
        ));
    }
    gas_plan_table.add_row(row!(
        "Total",
        format!("{} litres", total_gas)
    ));
    println!("{}", gas_plan_table);
    // println!("Total gas: {} litres", total_gas);
}
