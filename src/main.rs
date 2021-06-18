use capra::deco::zhl16::builder::ZHL16Builder;
use capra::deco::zhl16::gradient_factor::GradientFactor;

use capra::environment::Environment;
use capra::gas::Gas;
use capra::parameter::Parameters;
use capra::plan::open_circuit::OpenCircuit;
use capra::plan::DivePlan;
use capra::segment::{Segment, SegmentType};
use capra::units::air_consumption::AirConsumption;
use capra::units::altitude::Altitude;
use capra::units::depth::Depth;
use capra::units::rate::Rate;
use capra::units::water_density::SALTWATER;
use serde::{Deserialize, Serialize};

use std::fs;
use std::iter::FromIterator;
use tabular::row;
use tabular::Table;
use time::Duration;

const DEFAULT_BOTTOM_SAC: AirConsumption = AirConsumption(20);
const DEFAULT_DECO_SAC: AirConsumption = AirConsumption(20);
const DEFAULT_ASCENT_RATE: Rate = Rate(-18);
const DEFAULT_DESCENT_RATE: Rate = Rate(30);

#[derive(Serialize, Deserialize, Debug)]
struct JSONDecoGas {
    o2: u8,
    he: u8,
    max_operating_depth: Option<Depth>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONDiveSegment {
    depth: Depth,
    // Minutes
    time: u32,
    o2: u8,
    he: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONDive {
    gfl: Option<u8>,
    gfh: Option<u8>,
    asc: Option<Rate>,
    desc: Option<Rate>,
    bottom_sac: Option<AirConsumption>,
    deco_sac: Option<AirConsumption>,
    segments: Vec<JSONDiveSegment>,
    deco_gases: Vec<JSONDecoGas>,
}

fn pretty_time(duration: &Duration) -> String {
    let m = duration.whole_minutes();
    let s = duration.whole_seconds() - m * 60;
    format!("{}:{:0>2}", m, s)
}

fn main() {
    // let mut line: String = "".parse().unwrap();
    // let stdin = io::stdin();
    // for x in BufReader::new(stdin).lines() {
    //     line = line.to_owned() + &x.expect("unable to read input") + "\n"
    // }

    let line = fs::read_to_string("samples/sample_sammy.json") // Use this for profiling!
        .expect("Something went wrong reading the file");

    let js: JSONDive = serde_json::from_str(&line).expect("unable to decode user input");

    let ascent_rate = js.asc.unwrap_or(DEFAULT_ASCENT_RATE);
    let descent_rate = js.desc.unwrap_or(DEFAULT_DESCENT_RATE);
    let sac_bottom = js.bottom_sac.unwrap_or(DEFAULT_BOTTOM_SAC);
    let sac_deco = js.deco_sac.unwrap_or(DEFAULT_DECO_SAC);

    let gf = if let (Some(gfl), Some(gfh)) = (js.gfl, js.gfh) {
        GradientFactor::new(gfl, gfh)
    } else {
        GradientFactor::default()
    };

    let deco_gases = js
        .deco_gases
        .into_iter()
        .map(|gas| {
            (
                Gas::new(gas.o2, gas.he, 100 - gas.he - gas.o2)
                    .expect("unable to decode decompression gas"),
                gas.max_operating_depth,
            )
        })
        .collect::<Vec<(_, _)>>();

    let bottom_segments = js
        .segments
        .into_iter()
        .map(|segment| {
            (
                Segment::new(
                    SegmentType::Bottom,
                    segment.depth,
                    segment.depth,
                    Duration::minutes(segment.time as i64),
                    ascent_rate,
                    descent_rate,
                )
                .expect("unable to decode segment"),
                Gas::new(segment.o2, segment.he, 100 - segment.he - segment.o2)
                    .expect("unable to decode bottom gas"),
            )
        })
        .collect::<Vec<(_, _)>>();

    let zhl16 = ZHL16Builder::new().gradient_factor(gf).finish();

    let parameters = Parameters::new(
        ascent_rate,
        descent_rate,
        Environment::new(SALTWATER, Altitude::default()),
        sac_bottom,
        sac_deco,
    );

    let dive = OpenCircuit::new(zhl16, &bottom_segments, &deco_gases, parameters);

    let plan = dive.plan(); // Use this to include all AscDesc segments

    let mut gas_plan = Vec::from_iter(plan.gas_used());
    gas_plan.sort_by(|&(_, a), &(_, b)| b.cmp(&a));

    println!("Ascent rate: {}m/min", ascent_rate.0);
    println!("Descent rate: {}m/min", descent_rate.0);
    println!("GFL/GFH: {}/{}\n", gf.low(), gf.high());

    let mut dive_plan_table = Table::new("{:>}  {:>}  {:>}  {:>}  {:>}");
    let mut runtime = Duration::zero();
    dive_plan_table.add_row(row!("Segment", "Depth", "Time", "Runtime", "Gas"));
    for x in plan.segments() {
        runtime += *x.0.time();
        let gas = format!("{}/{}", x.1.o2(), x.1.he());
        let segment_type = format!("{:?}", x.0.segment_type());
        match x.0.segment_type() {
            SegmentType::AscDesc => {
                let text = format!("-> {}m", x.0.end_depth().0);
                dive_plan_table.add_row(row!(
                    segment_type,
                    text,
                    pretty_time(x.0.time()),
                    pretty_time(&runtime),
                    gas
                ));
            }
            _ => {
                let text = format!("{}m", x.0.end_depth().0);
                dive_plan_table.add_row(row!(
                    segment_type,
                    text,
                    pretty_time(x.0.time()),
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
        gas_plan_table.add_row(row!(gas_str, qty_str));
    }
    gas_plan_table.add_row(row!("Total", format!("{} litres", total_gas)));
    println!("{}", gas_plan_table);
    // println!("Total gas: {} litres", total_gas);
}
