use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(
        args.next()
            .expect("usage: scrantic_inventory <SCRANTIC.SC$> <out.SCR>"),
    );
    let output = PathBuf::from(
        args.next()
            .expect("usage: scrantic_inventory <SCRANTIC.SC$> <out.SCR>"),
    );
    let packed = std::fs::read(&input).expect("read compressed SCRANTIC member");
    let plain =
        wilson_dgds::decompress_installer(&packed).expect("decompress original SCRANTIC member");
    std::fs::write(&output, &plain).expect("write decompressed SCRANTIC binary");

    println!("input:  {} bytes", packed.len());
    println!("output: {} bytes", plain.len());
    println!("MZ header: {}", plain.get(0..2) == Some(b"MZ"));
    let ne_offset = plain
        .get(0x3c..0x40)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    if let Some(offset) = ne_offset {
        println!("NE offset: 0x{offset:04X}");
        println!(
            "NE header: {}",
            plain.get(offset..offset + 2) == Some(b"NE")
        );
    }
    let sounds = wilson_dgds::sounds_from_scrantic_exe(&plain);
    let ids: Vec<_> = sounds
        .iter()
        .enumerate()
        .filter_map(|(id, wav)| wav.as_ref().map(|w| (id, w.len())))
        .collect();
    println!("embedded WAV effects ({}): {ids:?}", ids.len());
}
