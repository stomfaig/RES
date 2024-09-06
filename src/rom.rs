

pub trait Rom {
    fn new() -> Self;
    fn prg_read(address: u16) -> u8;
    fn chr_read(address: u16) -> u8;
}

struct NROM_128 {
    prg_rom: [u8; 0x4000],
    chr_rom: [u8; 0x0000],
}

impl Rom for NROM_128 {
    fn new() -> Self {
        Self {
            prg_rom: [0; 0x4000],
            chr_rom: [0; 0x0000],
        }
    }

    fn prg_read(address: u16) -> u8 {
        0
    }

    fn chr_read(address: u16) -> u8 {
        0
    }
}

struct NROM_256 {
    prg_rom: [u8; 0x8000],
    chr_rom: [u8; 0x0000],
}

impl Rom for NROM_256 {

    fn new() -> Self {
        Self {
            prg_rom: [0; 0x8000],
            chr_rom: [0; 0x0000],
        }
    }

    fn prg_read(address: u16) -> u8 {
        0
    }

    fn chr_read(address: u16) -> u8 {
        0 
    }
}


