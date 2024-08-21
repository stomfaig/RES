## NES emulator in Rust.

*Even though most of the implementation details in my code are somewhat different, the idea came from, and in its layout I am following the series '[Writing a NES Emulator in Rust](https://bugzmanov.github.io/nes_ebook/chapter_3_2.html)' by @bugzmanov.*


### CPU

Currently I am working on implementing all the CPU instructions, as well as a thorough test suite, that allows to formally verify that the cpu is working as expected. I am implementing the instructions according to [this site][1]. 

#### Formal verification

To make things as easy as possible, we think of the operation of this virtual CPU in the following way. Depending on the instruction received, I implemented the instruction in the following way.
* For **multi-mode instructions**, that support many addressing modes, we define a function that takes an addressing mode, acquires the address using an addressing mode dependant sub-routine, and then performs the operation. In these cases, we separate instruction recognition, and instruction execution, in the following way:
1. Make sure that the right instruction calls the right subroutine with the right addressing mode. 
2. Make sure that the subroutines are behaving as expected in every addressing mode.

* For **single-mode instructions**, that is instructions that either refer to a register or have a fixed mode of accessing memory, the code is in-line in the *match* statement that is responsible for handling the incoming instructions. Therefore, when testing these instructions we do so by directly preloading the memory with the instruction code and any further data, and check whether execution is as expected.  

We use these procedures, because it is error prone to manually enter lots of hex codes for the different instructions, especially when a certain instruction has many sub-varieties. We therefore check that the behavior of the implementation is correct, rather than actually running the machine.


    


[1]:https://www.nesdev.org/obelisk-6502-guide/index.html

