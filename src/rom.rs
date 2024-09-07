use std::fs;

pub trait Rom {
    fn load(&mut self, raw: &Vec<u8>, trainer: bool) -> Result<(), String>;
    fn prg_read(&self, address: u16) -> u8;
    fn chr_read(&self, address: u16) -> u8;
}


pub fn rom_reader() -> Result<Box<dyn Rom>, String> {
    let raw: Vec<u8> = match fs::read("./cartridges/nestest.nes") {
        Ok(raw) => raw,
        Err(e) => return Err(e.to_string()),
    };

    if raw.len() < 16 { return Err(String::from("Invalid INES header...")) }

    let prg_rom_chunks = raw[4];
    let chr_rom_chunks = raw[5];
    let trainer: bool = raw[6] & 0b100 != 0;
    let rom_mapper = ((raw[6] & 0b1111_0000) >> 4) | (raw[7] & 0b1111_0000);
    let ines_version = if (raw[7] & 0b1100 >> 1 == 0b10) { 2 } else { 1 };

    let mut rom: Box<dyn Rom> = match rom_mapper {
        0 => {
            match prg_rom_chunks {
                1 => Box::new(NROM_128::new()),
                2 => Box::new(NROM_256::new()),
                _ => return Err(format!("NROM does not support {:?} prg chunks!", prg_rom_chunks)),
            }
        },
        _ => {
            return Err(String::from(format!("INES rom mapper {:?} is not supported.", rom_mapper)))
        }
    };

    match rom.load(&raw, trainer) {
        Ok(()) => Ok(rom),
        Err(e) => Err(e),
    }
}

pub struct NROM_128 {
    prg_rom: [u8; 0x4000],
    chr_rom: [u8; 0x2000],
}

impl NROM_128 {
    fn new() -> Self {
        println!("INFO\tInitializing NROM128...");
        Self {
            prg_rom: [0; 0x4000],
            chr_rom: [0; 0x2000],
        }
    }
}

impl Rom for NROM_128 {
    fn load(&mut self, raw: &Vec<u8>, trainer: bool) -> Result<(), String> {
        let offset: usize = if trainer {512 + 16} else {16};
        if raw.len() != offset + 0x6000 {
            return Err(String::from("The size of the cartridge does not match the header information."))
        }
        self.prg_rom = raw[offset..(0x4000 + offset)].try_into().unwrap();
        self.chr_rom = raw[(0x4000 + offset)..(0x6000 + offset)].try_into().unwrap();
        Ok(())
    }

    fn prg_read(&self, address: u16) -> u8 {
        let source_addr = (address - 0x8000) % 0x4000;
        self.prg_rom[source_addr as usize]
    }

    fn chr_read(&self, address: u16) -> u8 {
        self.chr_rom[address as usize]
    }
}

pub struct NROM_256 {
    prg_rom: [u8; 0x8000],
    chr_rom: [u8; 0x2000],
}

impl NROM_256 {
    fn new() -> Self {
        println!("INFO\tInitializing NROM256...");
        Self {
            prg_rom: [0; 0x8000],
            chr_rom: [0; 0x2000],
        }
    }
}

impl Rom for NROM_256 {

    fn load(&mut self, raw: &Vec<u8>, trainer: bool) -> Result<(), String> {
        let offset: usize = if trainer {512 + 16} else {16};
        if raw.len() != offset + 0x6000 {
            return Err(String::from("The size of the cartridge does not match the header information."))
        }
        self.prg_rom = raw[offset..(0x8000 + offset)].try_into().expect("slice with incorrect length");
        self.chr_rom = raw[(0x8000 + offset)..(0xa000 + offset)].try_into().expect("slice with incorrect length");
        Ok(())
    }

    fn prg_read(&self, address: u16) -> u8 {
        self.prg_rom[address as usize]
    }

    fn chr_read(&self, address: u16) -> u8 {
        self.chr_rom[address as usize]
    }
}



impl EmptyRom {
    pub fn new() -> Self {
        Self {}
    }
}

impl Rom for EmptyRom {
    fn load(&mut self, raw: &Vec<u8>, trainer: bool) -> Result<(), String> {
        panic!("Empty ROM.")
    }
    fn prg_read(&self, address: u16) -> u8 {
        panic!("Empty ROM.");
    }
    fn chr_read(&self, address: u16) -> u8 {
        panic!("Empty ROM.");
    }
}
