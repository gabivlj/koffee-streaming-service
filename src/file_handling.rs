use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn get_duration_from_hls(filename: &str) -> f64 {
    let header_size = 4;
    let mut getting_duration_sec = false;
    let mut sum = 0;
    let mut duration: f64 = 0.0;
    if let Ok(lines) = read_lines(filename) {
        for line in lines {
            if let Ok(s) = line {
                match getting_duration_sec {
                    true => {
                        if sum % 2 == 0 {
                            let params: Vec<&str> = s.split(':').collect();
                            if params.len() > 1 {
                                // we split because parsing the f64 doesn't go pretty well with lots of numbers after .--- :)
                                let (duration_str, _) = params[1].split_at(5);
                                let duration_parsed: Result<f64, _> = duration_str.parse();
                                // Check parsing result
                                match duration_parsed {
                                    Ok(val) => duration += val,
                                    Err(_) => println!("Error parsing: {}", duration_str),
                                }
                            }
                        }
                    }
                    false => {
                        if sum >= header_size {
                            getting_duration_sec = true;
                        }
                    }
                }
                sum += 1;
            }
        }
    }
    return duration;
}
