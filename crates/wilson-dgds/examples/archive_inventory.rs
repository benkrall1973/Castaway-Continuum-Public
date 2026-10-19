use std::path::PathBuf;

use wilson_dgds::{Archive, ResourceMap};

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: archive_inventory <data-dir>"),
    );
    let map = std::fs::read(dir.join("RESOURCE.MAP")).expect("read RESOURCE.MAP");
    let map_info = ResourceMap::parse(&map).expect("parse RESOURCE.MAP");
    let data = std::fs::read(dir.join(&map_info.data_file_name)).expect("read resource data");
    let archive = Archive::parse(&map, &data).expect("parse archive");

    println!(
        "ARCHIVE data={} entries={} pal={} bmp={} scr={} ttm={} ads={} skipped={}",
        archive.data_file_name,
        map_info.entries.len(),
        archive.palettes.len(),
        archive.bitmaps.len(),
        archive.screens.len(),
        archive.ttms.len(),
        archive.ads.len(),
        archive.skipped.len()
    );
    if let Some(palette) = archive.palette() {
        println!("PALETTE 0..15: {:?}", &palette.colors[..16]);
    }
    println!("\nSCREENS");
    for (name, screen) in &archive.screens {
        println!(
            "{name}\t{}x{}\t{} pixels",
            screen.width,
            screen.height,
            screen.pixels.len()
        );
    }
    println!("\nADS DIRECTORS");
    for (name, ads) in &archive.ads {
        let resources = ads
            .resources
            .iter()
            .map(|r| format!("{}:{}", r.id, r.name))
            .collect::<Vec<_>>()
            .join(", ");
        let tags = ads
            .tags
            .iter()
            .map(|t| format!("{}:{}", t.id, t.description))
            .collect::<Vec<_>>()
            .join(" | ");
        println!("{name}\tresources=[{resources}]\ttags=[{tags}]");
    }
    println!("\nTTM SCRIPTS");
    for (name, ttm) in &archive.ttms {
        let tags = ttm
            .tags
            .iter()
            .map(|t| format!("{}:{}", t.id, t.description))
            .collect::<Vec<_>>()
            .join(" | ");
        println!(
            "{name}\tpages={}\tcode={}\ttags=[{tags}]",
            ttm.num_pages,
            ttm.bytecode.len()
        );
    }
}
