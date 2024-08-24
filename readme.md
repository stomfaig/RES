## NES emulator in Rust.

*Even though most of the implementation details in my code are somewhat different, the idea came from, and in its layout I am following the series '[Writing a NES Emulator in Rust](https://bugzmanov.github.io/nes_ebook/chapter_3_2.html)' by @bugzmanov.*


### CPU

Currently I am working on implementing all the CPU instructions, as well as a thorough test suite, that allows to formally verify that the cpu is working as expected. I am implementing the instructions according to [this site][1]. 

#### Formal verification

To make things as easy as possible, we think of the operation of this virtual CPU in the following two ways.
* For **multi-mode instructions**, that support many addressing modes, the instrucion is implemented in a way that does not directly interact with the addressing mode. Using a subroutine, given the addressing mode we obtain the address of the referred memory cell. For such instructions the testing is done in the following steps:
1. Make sure that the right instruction calls the instruction. This is done by hand, or by a parsing script.
2. Make sure that the subroutines are behaving as expected in every addressing mode. This is done by implementing a unified addressing-agnostic test, that defines the test parameters, calls a method that hides a randomly generated test value in the memory and encodes the location of this data using the specified addressing mode, and loads this into the program memory. The instruction execution function is the directly called (i.e. not though the program execution)

* For **single-mode instructions**, that is instructions that either refer to a register or have a fixed mode of accessing memory, the code is in-line in the *match* statement that is responsible for handling the incoming instructions. Therefore, when testing these instructions we do so by directly preloading the memory with the instruction code and any further data, and check whether execution is as expected.  


We use these procedures, because it is error prone to manually enter lots of hex codes for the different instructions, especially when a certain instruction has many sub-varieties. We therefore check that the behavior of the implementation is correct, rather than actually running the machine. 

Later we will implement the following system. We treat the NES as a state machine, where all the combinations of the different memory locations gives rise to the state of the system -- on which different instructions operate. We say that an instruction is **dependent** on a memory location, if its outcome is not independent of the memory location (i.e. a change can be made to the memory location so that the change induced by the instruction is different). We also say that a memory location is **dependent** on an instruction, if there is an input parameter set so that the value stored at the memory location is changed during the execution of the instruction.  
One way to formally verify a system, is to make sure that given any input state, and instruction, the resulting state is as expected. Checking this directly is very expensive, however, it can be significantly simplified:
1. Check that the instruction only read memory locations that it should be dependent on,
2. Check that the instruction only written memory location that should be dependent on it,
3. Check that given test cases on the dependent input locations, the output is as expected.
Note, however, that checking all possible dependent input combinations might still be cumbersome. 

### BUS and Memory

*Even though the CPU is not completely ready yet, I need a fairly simple external BUS and Memory implementation, that I can use in testing.*

Mainly for convenience reasons I am going to treat the bus and the memory as a singular object. For now the memory is owned by the CPU, but the final plan is to have a decentralised CPU, just as on the original NES, where different components will have to request access to memory before interacting with it. Communication with the memory happens through the following channels:

    address_bus : Address of the memory cell to read from, or written to
    data_bus    : In the case of writing, the data to be written into the memory, int the
                    case of reading, the memory puts the data read form the memory cell onto
                    this bus
    control_bus : Control signals, that configure the memory
    AccessMode  (bit 0 of cb) first  0/1: determines whether the memory unit is expected to store the value
                    on the data bus, or load a value onto the data bus.
    MemEnable   (bit 1 of cb) If 0, the memory is not active. If 1, the memory reads the value in the
                    AccessMode register, and performs the requested operation.

A memory unit can be used with the CPU if it implements the 'Mem' train from the 'bus' module. This trait provides implementations for operations related to the above buses.

#### ArrayBus

Completely memory backed memory unit. Mostly used for running the CPU without assuming memory mapped objects. The complete memory range (0x0000-0xffff) corresponds to a u8 array.

#### TestBus

Completely memory backed memory unit for testing. The idea behind this module is that when running a method in testing on a certain input data, we can predict what parts of the memory *should* be accessed, and what values should be written to the memory. TestBus can be preloaded with these expectations, and upon the CPU running, the TestBus panics if these expectations are violated. The 'TestBus' struct has 3 extra methods on top of the methods required by 'Mem':

    set_read_target(addr: u16, val: u8)         : allow the cpu to read from the address 'addr' 
                                                    and upon the cpu reading from this address,
                                                    return 'val'.
    set_read_u16_address(addr:u 16, val: u16)   : same as set_read_target, but with u16 values.
    set_write_target(addr: u16, val: u8)        : allow the cpu to write to the address 'addr',
                                                    upon writing, panic if the written value is
                                                    not 'val'.



[1]:https://www.nesdev.org/obelisk-6502-guide/index.html

