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
        write!(f, "Rate::new( {}f64, {}f64, {}f64 )", self.stocks, self.bonds, self.inflation)
    }
}

enum StorageLocation {
    ThreadLocal,
    Static
}

fn read_csv<T: DeserializeOwned + Display>(storage_location: StorageLocation, file: &str, output_file: &str, variable: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    let output = match storage_location {
        StorageLocation::ThreadLocal =>
            format!(
                "thread_local!{{
                    pub static {}: std::rc::Rc<[{}; {}]> = std::rc::Rc::new([{}]);
                }}"
            , variable, typename, count, output),
        StorageLocation::Static =>
            format!("static {}: [{}; {}] = [{}];", variable, typename, count, output)
    };

    fs::write(&dest_path, output)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    read_csv::<Rate>(StorageLocation::Static, "csv/rates.csv", "rates.rs", "RATES_BUILTIN")?;
    read_csv::<f64>(StorageLocation::ThreadLocal, "csv/death_male.csv", "death_male.rs", "ANNUAL_DEATH_MALE_BUILTIN")?;
    read_csv::<f64>(StorageLocation::ThreadLocal, "csv/death_female.csv", "death_female.rs", "ANNUAL_DEATH_FEMALE_BUILTIN")?;

    read_csv::<Rate>(StorageLocation::Static, "csv/test_rates.csv", "test_rates.rs", "TEST_RATES_BUILTIN")?;
    read_csv::<f64>(StorageLocation::ThreadLocal, "csv/test_death.csv", "test_death.rs", "TEST_DEATH_BUILTIN")
}