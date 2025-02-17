use clap::Parser;
use png::{BitDepth, ColorType, Decoder};
use std::path::Path;
use std::{fs::File, io::Write};

#[derive(Parser)]
struct Args {
    /// Input file
    filename: String,

    /// Output directory
    #[arg(short, long)]
    output: Option<String>,

    /// If it's a tileset that doesn't have an empty tile in position 0 (adds one)
    #[arg(short, long, default_value_t = false)]
    tileset: bool,

    /// Adds the palette to the generated file
    #[arg(short, long, default_value_t = false)]
    palette: bool,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let folder = if let Some(folder) = args.output {
        format!("{folder}/")
    } else {
        "".into()
    };

    let mut reader = Decoder::new(File::open(&args.filename)?).read_info()?;
    let header_info = reader.info();
    let width = header_info.width;
    let height = header_info.height;
    let Some(palette) = &header_info.palette else {
        panic!("Input image has no palette.");
    };
    let palette = palette.clone().into_owned();

    assert_eq!(width % 8, 0);
    assert_eq!(height % 8, 0);
    assert_eq!(palette.len() % 3, 0);
    assert_eq!(header_info.color_type, ColorType::Indexed);

    let bpp = match header_info.bit_depth {
        BitDepth::One => 1,
        BitDepth::Two => 2,
        BitDepth::Four => 4,
        BitDepth::Eight => 8,
        BitDepth::Sixteen => unimplemented!(),
    };

    let path = Path::new(&args.filename);
    let name_bpp = path.file_stem().unwrap().to_str().unwrap();
    let name = name_bpp
        .chars()
        .take_while(|c| *c != '.')
        .collect::<String>();

    let mut file_h = File::create(format!("{folder}{name}.h"))?;
    writeln!(file_h, "#ifndef {name}_h")?;
    writeln!(file_h, "#define {name}_h")?;
    writeln!(file_h)?;
    writeln!(file_h, "#include <stdint.h>")?;
    writeln!(file_h)?;
    writeln!(file_h, "extern const uint8_t {name}[];")?;
    writeln!(file_h, "extern const uint16_t {name}_length;")?;
    if args.palette {
        writeln!(file_h, "extern const uint8_t {name}_palette[];")?;
        writeln!(file_h, "extern const uint16_t {name}_palette_length;")?;
    }
    writeln!(file_h)?;
    writeln!(file_h, "#endif // {name}_h")?;

    let mut file_c = File::create(format!("{folder}{name_bpp}.c"))?;

    writeln!(file_c, "#include \"{name}.h\"\n")?;
    writeln!(file_c, "const uint8_t {name}[] = {{")?;

    if args.tileset {
        let number_of_ints = width * bpp / 8;
        for _ in 0..bpp {
            for i in 0..width {
                if bpp == 1 || bpp == 2 {
                    write!(file_c, "0b00000000, ")?;
                } else {
                    write!(file_c, "0x00, ")?;
                }

                if (i + 1) % number_of_ints == 0 {
                    writeln!(file_c)?;
                }
            }
        }
    }

    let mut frame = vec![0u8; (width * height * bpp / 8) as usize];
    reader.next_frame(&mut frame)?;

    let bytes_per_line = (width * bpp / 8) as usize;
    for (i, output) in frame.iter().enumerate() {
        write!(file_c, "0x{output:02X}, ",)?;

        if (i + 1) % bytes_per_line == 0 {
            writeln!(file_c)?;
        }
    }

    writeln!(file_c, "}};\n")?;
    writeln!(
        file_c,
        "const uint16_t {name}_length = {};",
        width * (height + if args.tileset { 1 } else { 0 }) / (8 / bpp)
    )?;

    if args.palette {
        writeln!(file_c)?;
        writeln!(file_c, "const uint8_t {name}_palette[] = {{")?;
        for c in palette.chunks(3) {
            write!(file_c, "0x{:1X}", (c[1] >> 4) & 0xF)?;
            write!(file_c, "{:1X}, ", (c[2] >> 4) & 0xF)?;
            writeln!(file_c, "0x0{:1X},", (c[0] >> 4) & 0xF)?;
        }
        writeln!(file_c, "}};\n")?;
        writeln!(
            file_c,
            "const uint16_t {name}_palette_length = {};",
            2 * palette.len() / 3
        )?;
    }

    Ok(())
}
