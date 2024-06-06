use core::time;
use std::thread::sleep;
use std::{error::Error, fs::File, path::Path};
use std::io::{self, BufRead};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

#[derive(Clone)]
struct Color{
    r : u8,
    g : u8,
    b : u8,
}

impl Color {
    fn new (r: u8, g: u8, b: u8) -> Self
    {
        Color{r,g,b}
    }
}

fn convert_u8_to_spi_bits(input: u8) -> [u8; 3] {
    let mut output = [0u8; 3];

    for i in 0..8 {
        let bit = (input >> (7 - i)) & 1;
        let pattern = if bit == 0 { 0b100 } else { 0b110 };
        let byte_index = (i * 3) / 8;
        let bit_index = (i * 3) % 8;
        if bit_index <= 5 {
            output[byte_index] |= pattern << (5 - bit_index);
        } else {
            output[byte_index] |= pattern >> (bit_index - 5);
            output[byte_index + 1] |= pattern << (13 - bit_index);
        }
    }
    output
}

fn convert_color_to_spi( c : &Color) -> [u8; 9] {
    let mut output = [0u8; 9];
    [output[0],output[1], output[2]] = convert_u8_to_spi_bits(c.g);
    [output[3],output[4], output[5]] = convert_u8_to_spi_bits(c.r);
    [output[6],output[7], output[8]] = convert_u8_to_spi_bits(c.b);
    output
}

fn convert_color_vec_to_spi( v : &Vec<Color>) -> Vec<u8> {
    let mut output : Vec<u8> = Vec::with_capacity(v.len()*9);
    for c in v {
        output.append(&mut convert_color_to_spi(c).to_vec());
    }
    output
}

fn read_cpu_usage(prev_total :u32, prev_idle :u32) -> Result<(u32, u32, u32), Box<dyn Error>> {
    
    let s = read_first_line_of_file("/proc/stat")?;
    println!("{}", s);
    let v = s.split_whitespace().skip(1).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<u32>>();
    let idle_now = v[3];
    let total_now :u32 = v.into_iter().sum();
    let diff_idle = idle_now - prev_idle;
    let diff_total = total_now - prev_total;
    let usage = ((diff_total - diff_idle )as f32 / diff_total as f32 * 100.0) as u32;
    println!("Total {}, Idle {}, Usage {}%", diff_total, diff_idle, usage);
    Ok((total_now, idle_now, usage))
}

fn read_first_line_of_file(file_path: &str) -> io::Result<String> {
    let path = Path::new(file_path);
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut lines = reader.lines();
    if let Some(line) = lines.next() {
        return line;
    }

    Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File is empty"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut data = vec![Color::new(0,0,0);20];
    let mut total = 0;
    let mut idle = 0;
    loop {
        let usage;
        (total, idle, usage) = read_cpu_usage(total, idle)?;
        println!("Total {}, Idle {}, Usage {}%", total, idle, usage);
        let num_leds = (usage/5) as usize;
        for i in 0 .. num_leds {
            data[i] = Color::new(5,0,0);
        }

        for i in num_leds .. 20 {
            data[i] = Color::new(0,0,0);
        }

        let spi_bits = convert_color_vec_to_spi(&data);
        let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 3_000_000, Mode::Mode0)?;
        let _ = spi.write(&spi_bits)?;
        sleep(time::Duration::from_secs(1));
    }
}
