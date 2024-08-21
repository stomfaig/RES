mod bus {

    struct BUS {
        address_bus: u16,
        data_bus: u8,
        control_bus: bool,
    }

    impl BUS {
        fn new() -> Self {
            BUS {
                address_bus : 0,
                data_bus : 0,
                control_bus : false,
            }
        }
    }

}