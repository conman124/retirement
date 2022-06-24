use std::env;
use std::fmt::{Display,Formatter};
use std::fs;
use std::fs::File;
use std::path::Path;
use serde::{Deserialize, de::DeserializeOwned};

// TODO dry
#[derive(Deserialize)]
struct Rate {
    stocks: f64,
    bonds: f64,
    inflation: f64,
}

impl Display for Rate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rate {{ stocks: {} as f64, bonds: {} as f64, inflation: {} as f64 }}", self.stocks, self.bonds, self.inflation)
    }
}

fn read_csv<T: DeserializeOwned + Display>(file: &str, output_file: &str, variable: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", file);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(output_file);

    let mut rdr = csv::Reader::from_reader(File::open(file)?);
    let result = rdr
        .deserialize()
        .map(|rate: Result<T, _>| {
            format!("{}", rate.unwrap())
        })
        .collect::<Vec<String>>();

    let count = result.len();
    let output = result.join(", ");
    let typename = String::from(std::any::type_name::<T>()).replace("build_script_build::", "");

    fs::write(
        &dest_path,
        format!("static {}: [{}; {}] = [{}];", variable, typename, count, output)
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    read_csv::<Rate>("csv/rates.csv", "rates.rs", "RATES_BUILTIN")
}