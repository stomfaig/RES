

pub trait Rom {
    fn load(&mut self, raw: &Vec<u8>, trainer: bool) -> Result<(), String>;
    fn prg_read(&self, address: u16) -> u8;
    fn chr_read(&self, address: u16) -> u8;
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


