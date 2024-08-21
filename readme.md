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

[1]:https://www.nesdev.org/obelisk-6502-guide/index.html

