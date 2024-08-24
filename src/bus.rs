use std::collections::HashMap;

pub enum ControlSignal {
    MemEnable = 0b0000_0001,
    AccessMode = 0b0000_0010,
}

pub trait Mem {
    fn new() -> Self;
    fn set_address_bus(&mut self, addr: u16);
    fn set_data_bus(&mut self, val: u8);
    fn get_data_bus(&self) -> u8;
    fn set_control_signal(&mut self, control: ControlSignal, val: bool);
    fn get_control_signal(&self, control: ControlSignal) -> bool;
}

pub struct ArrayBus {
    address_bus: u16,
    data_bus: u8,
    control_bus: u8,
    data: [u8; 0xffff],
}

impl ArrayBus {
    // Currently I assume that 0 is 'save into mem' and 1 is 'read from mem', but this might change...
    fn update(&mut self) {
        if (!self.get_control_signal(ControlSignal::MemEnable)) { return; }
    
        if (self.get_control_signal(ControlSignal::AccessMode)) {
            self.data_bus = self.data[self.address_bus as usize];
        } else {
            self.data[self.address_bus as usize] = self.data_bus;
        }
    }
}

impl Mem for ArrayBus {
    fn new() -> Self {
        ArrayBus {
            address_bus : 0,
            data_bus : 0,
            control_bus : 0,
            data : [0; 0xffff],
        }
    }

    fn set_address_bus(&mut self, addr: u16) {
        self.address_bus = addr;
    }

    fn set_data_bus(&mut self, val: u8) {
        self.data_bus = val;
    }

    fn get_data_bus(&self) -> u8 {
        self.data_bus
    }   

    fn set_control_signal(&mut self, control: ControlSignal, val: bool) {
        let mask = control as u8;
        if (val)  { self.control_bus |= mask; }
        else { self.control_bus &= !mask; }
        self.update();
    }

    fn get_control_signal(&self, control: ControlSignal) -> bool {
        (self.control_bus & (control as u8)) != 0
    }
}

/// This is a struct that is designed to test the memory access behavior of implementations.
/// In addition to implementing the 'Mem' trait,  
pub struct TestBus {
    address_bus: u16,
    data_bus: u8,
    control_bus: u8,
    read_targets: HashMap<u16, u8>,
    write_targets: HashMap<u16, u8>,
}

impl TestBus {

    pub fn set_read_target(&mut self, addr: u16, val: u8) {
        self.read_targets.insert(addr, val);
    }

    pub fn set_read_u16_target(&mut self, addr: u16, val: u16) {
        let low: u8 = (val & 0xff) as u8;
        let high: u8 = (val >> 8) as u8;
        self.read_targets.insert(addr, low);
        self.read_targets.insert(addr + 1, high);
    }

    pub fn set_vector_read_target(&mut self, addr: u16, vals: Vec<u8>) {
        let mut offset: u16 = 0;
        for val in vals {
            self.read_targets.insert(addr + offset, val);
            offset += 1;
        }
    }

    pub fn set_write_target(&mut self, addr: u16, val: u8) {
        self.write_targets.insert(addr, val);
    }
}

impl Mem for TestBus {
    fn new() -> Self {
        Self {
            address_bus: 0,
            data_bus: 0,
            control_bus: 0,
            read_targets: HashMap::new(),
            write_targets: HashMap::new(),
        }
    }
    fn set_address_bus(&mut self, addr: u16) {
        self.address_bus = addr;
        
    }
    fn set_data_bus(&mut self, val: u8) {
        self.data_bus = val;
    }
    fn get_data_bus(&self) -> u8 {
        self.data_bus
    }
    fn set_control_signal(&mut self, control: ControlSignal, val: bool) {
        let mask = control as u8;
        if (val)  { self.control_bus |= mask; }
        else { self.control_bus &= !mask; }
        
        if (!self.get_control_signal(ControlSignal::MemEnable)) { return; }

        if (self.get_control_signal(ControlSignal::AccessMode)) {
            let result: Option<&u8> = self.read_targets.get(&self.address_bus);
            self.data_bus = match result {
                Some(val) => *val,
                None => panic!("Method trying to read from forbidden memory (addr: {:x})", self.address_bus),
            }
        } else {
            let result: Option<&u8> = self.write_targets.get(&self.address_bus);
            match result {
                Some(val) => {
                    if (*val != self.data_bus) { panic!("Method trying to write invalid data."); }
                },
                None => panic!("Method trying to write to forbidden memory(addr: {:x}, val: {:b})", self.address_bus, self.data_bus),
            }
        }
    }

    fn get_control_signal(&self, control: ControlSignal) -> bool {
        (self.control_bus & (control as u8)) != 0
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {

    }
}
