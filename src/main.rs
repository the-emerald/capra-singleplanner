use std::io::{Read, BufRead, BufReader};
use std::io;
use serde_json::Value;
use capra::common::dive_segment::{DiveSegment, SegmentType};
use capra::common::gas::Gas;
use serde::{Deserialize, Serialize};
use capra::planner::plan_dive;
use capra::zhl16::ZHL16;
use capra::zhl16::util::{ZHL16B_N2_A, ZHL16B_N2_B, ZHL16B_N2_HALFLIFE, ZHL16B_HE_A, ZHL16B_HE_HALFLIFE, ZHL16B_HE_B};

#[derive(Serialize, Deserialize, Debug)]
struct JSONDecoGas {
    o2: f64,
    he: f64,
    modepth: Option<usize>
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONDiveSegment {
    depth: usize,
    time: usize,
    o2: f64,
    he: f64,
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


const DEFAULT_GFL: usize = 100;
const DEFAULT_GFH: usize = 100;
fn main() {
    let mut line: String = "".parse().unwrap();
    let stdin = io::stdin();
    for x in BufReader::new(stdin).lines() {
        line = line.to_owned() + &x.unwrap() + "\n"
    }

    let mut bottom_segments: Vec<(DiveSegment, Gas)> = Vec::new();
    let mut deco_mixes: Vec<(Gas, Option<usize>)> = Vec::new();

    let js: JSONDive = serde_json::from_str(&line).unwrap();

//    println!("{:?}", js);

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
        deco_mixes.push((Gas::new(1.0 - gas.he - gas.o2, gas.o2, gas.he).unwrap(), gas.modepth));
    }

    for seg in js.segments {
        bottom_segments.push((DiveSegment::new(SegmentType::DiveSegment,
                                            seg.depth, seg.depth, seg.time, ascent_rate,
                                            descent_rate).unwrap(),
                           Gas::new(1.0 - seg.he - seg.o2, seg.o2, seg.he).unwrap()));
    }

    let zhl16 = ZHL16::new(
        &Gas::new(0.79, 0.21, 0.0).unwrap(),
        ZHL16B_N2_A, ZHL16B_N2_B, ZHL16B_N2_HALFLIFE, ZHL16B_HE_A, ZHL16B_HE_B, ZHL16B_HE_HALFLIFE, gfl, gfh);

    let plan = plan_dive(zhl16, &bottom_segments, &deco_mixes);
    for x in plan {
//        if x.0.get_segment_type() == SegmentType::AscDesc {
//            continue;
//        }
        println!("{:?}: {}m for {}min - {}/{}", x.0.get_segment_type(), x.0.get_end_depth(), x.0.get_time(), x.1.fr_o2()*100.0, x.1.fr_he()*100.0);
    }
}
