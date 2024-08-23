
pub enum ControlSignal {
    MemEnable = 0b0000_0001,
    AccessMode = 0b0000_0010,
}

pub trait Mem {
    fn new() -> Self;
    fn set_address_bus(&mut self, addr: u16);
    fn set_data_bus(&mut self, val: u8);
    fn set_control_signal(&mut self, control: ControlSignal, val: bool);
    fn get_control_signal(&self, control: ControlSignal) -> bool;
}

pub struct ArrayBus {
    pub address_bus: u16,
    pub data_bus: u8,
    pub control_bus: u8,
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
        self.update();
    }

    fn set_data_bus(&mut self, val: u8) {
        self.data_bus = val;
        self.update();
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

