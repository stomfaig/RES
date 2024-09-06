mod cpu;
mod bus;
mod rom;

use config::Config;
use std::collections::HashMap;

use crate::rom::Rom;

fn main() {

    let settings = Config::builder()
        .add_source(config::File::with_name("./config.yaml"))
        .build()
        .unwrap();

    let rom: Option<Box<dyn Rom>> =  match settings.get_int("rom") {
        Ok(val) => {
            if val == 0 { println!("Startup without rom..."); None }
            else { None }
        },
        Err(e) => {
            println!("Failed to read rom info from config, startup without rom. (err: {:?})", e);
            None
        },
    };

    // try loading a rom. if that fails, continue startup without a rom

}
