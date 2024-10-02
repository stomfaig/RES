mod cpu;
mod bus;
mod rom;

use config::Config;

use crate::cpu::cpu::{CPU};
use crate::bus::{Mem, RomBus};
use crate::rom::{rom_reader};

fn main() {

    let config = Config::builder()
        .add_source(config::File::with_name("./config.yaml"))
        .build()
        .unwrap();

    match rom_reader() {
        Ok(rom) => {
            println!("{:?}", rom.prg_read(0x8000));
            println!("INFO\tSuccessful initialization");
            let mut bus = RomBus::new();
            bus.set_rom(rom);

            let debug = config.get_bool("debug").unwrap();
            println!("NFO\tDebug: {:?}", debug);

            let mut cpu = CPU::<RomBus>::new(bus, debug);
            cpu.start();
        },
        Err(e) => {
            println!("ERR:\tRom loading failed ({}), starting without rom...", e);
            //let mut bus = ArrayBus::new();
            //let mut cpu = CPU::<ArrayBus>::new(bus, true);
        }
    }
}
