## NES emulator in Rust.

*Even though most of the implementation details in my code are somewhat different, the idea came from, and in its layout I am following the series '[Writing a NES Emulator in Rust](https://bugzmanov.github.io/nes_ebook/chapter_3_2.html)' by @bugzmanov.*


### CPU

Currently I am working on implementing all the CPU instructions, as well as a thorough test suite, that allows to formally verify that the cpu is working as expected. I am implementing the instructions according to [this](https://www.nesdev.org/obelisk-6502-guide/index.html) site.  

