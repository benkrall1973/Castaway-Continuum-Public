use std::path::PathBuf;

use wilson_dgds::{Archive, ResourceMap};

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: special_trace <data-dir>"),
    );
    let map = std::fs::read(dir.join("RESOURCE.MAP")).expect("read RESOURCE.MAP");
    let map_info = ResourceMap::parse(&map).expect("parse RESOURCE.MAP");
    let data = std::fs::read(dir.join(&map_info.data_file_name)).expect("read resource data");
    let archive = Archive::parse(&map, &data).expect("parse archive");

    for ads_name in ["JOHNNY.ADS", "SUZY.ADS", "MARY.ADS"] {
        let ads = archive.ads(ads_name).expect("ADS present");
        println!("\n===== {ads_name} =====");
        for (i, instruction) in ads.instructions().expect("decode ADS").iter().enumerate() {
            println!(
                "ADS {i:03}: {:#06X} {:<22} {:04X?}",
                instruction.opcode,
                instruction.name(),
                instruction.args
            );
        }
        for resource in &ads.resources {
            let ttm = archive.ttm(&resource.name).expect("TTM present");
            println!("\n--- slot {}: {} ---", resource.id, resource.name);
            if resource.name == "MEANWHIL.TTM" {
                let instructions = ttm.instructions().expect("decode TTM");
                let frames = instructions.iter().filter(|i| i.opcode == 0x0FF0).count();
                let delays = instructions
                    .iter()
                    .filter(|i| i.opcode == 0x1021 || i.opcode == 0x2022)
                    .map(|i| format!("{}={:?}", i.name(), i.args))
                    .collect::<Vec<_>>();
                println!("CLOCK_STATS frames={frames} timing_ops={delays:?}");
            }
            if !matches!(
                resource.name.as_str(),
                "SJWORK.TTM" | "MEANWHIL.TTM" | "SUZYCITY.TTM" | "SJMSUZY.TTM" | "THEEND.TTM"
            ) {
                continue;
            }
            for (i, instruction) in ttm.instructions().expect("decode TTM").iter().enumerate() {
                if matches!(
                    instruction.opcode,
                    0x001F
                        | 0x0080
                        | 0x0110
                        | 0x1021
                        | 0x1101
                        | 0x1111
                        | 0x1201
                        | 0x2022
                        | 0x4110
                        | 0x4120
                        | 0x4204
                        | 0xA054
                        | 0xA064
                        | 0xB606
                        | 0xF01F
                        | 0xF02F
                        | 0xF05F
                ) {
                    println!(
                        "TTM {i:03}: {:<22} {:04X?}",
                        instruction.name(),
                        instruction.args
                    );
                }
            }
        }
    }
}
