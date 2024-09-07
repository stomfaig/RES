mod cpu;
mod bus;
mod rom;

use config::Config;
use std::collections::HashMap;

use crate::cpu::cpu::{CPU};
use crate::bus::{Mem, RomBus, ArrayBus};
use crate::rom::{Rom, NROM_128, rom_reader};

fn main() {

    let settings = Config::builder()
        .add_source(config::File::with_name("./config.yaml"))
        .build()
        .unwrap();

    match rom_reader() {
        Ok(rom) => {
            println!("INFO\tSuccessful initialization");
            let mut bus = RomBus::new();
            bus.set_rom(rom);
            let mut cpu = CPU::<RomBus>::new(bus);
            cpu.start();
        },
        Err(e) => {
            println!("ERR:\tRom loading failed ({}), starting without rom...", e);
            let mut bus = ArrayBus::new();
            let mut cpu = CPU::<ArrayBus>::new(bus);
        }
    }
}
