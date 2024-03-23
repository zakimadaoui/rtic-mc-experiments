# Modular RTIC and POC for a multi-core extension on rp2040

**Experiment Objective:** 

The objective of this experiment is to enhance the scalability of the RTIC (Real-Time Interrupt-driven Concurrency) framework, particularly to allow for multicore hardware configurations. The goal is to decouple the RTIC declarative model from hardware-specific implementation details. And (possibly) provide a mechanism for additional syntax extension through 3rd party libraries. By achieving this, we aim to create a more maintainable and extensible RTIC framework capable of accommodating various hardware configurations while preserving the original core declarative model.



### **RTIC distributions** and External Compilation passes

An RTIC distribution is a proc-macro crate that exposes an RTIC framework **implemented for a specific hardware architecture or even for a specific micro-controller**. For example, we could have a distribution for single core cortex-m devices, another distribution specifically tailored for the RP2040, a distribution for risc-v architecture ... etc. 

Each distribution will :

- implement the hardware specific details described in its own crate.
- Integrate a selection of `Compilation passes` and expose them to the user as a set of **features** which the user can select to enable.

This makes RTIC codebase growth more controllable and provides an alternative approach to the current one in which all the hardware specific details all belong to a single crate and an architecture is chosen by enabling a corresponding rust feature. 

**RTIC distributions** DO NOT re-impelement the RTIC framework from scratch, instead they only provide the hardware specific parts of the implementation to `rtic-core` library  and other `compilation passes` crates/libraries that will do all the heavy lifting of parsing, analyzing and generating code .

In this experiment project

- `rtic-core` is the library crate which:

  - contains the **Hardware Tasks and Resources** pass.

  - Exposes an Builder API for loading other external passes (from 3rd party crates) and for externally providing hardware specific implementations to build an RTIC framework
  - Note: `rtic-core` naming was chosen for lack of a better name. However, this library is completely different from the other `rtic-core` crate in the original RTIC project. 

- `rtic-sw-pass` is the default crate that provides a software tasks pass. It does that by simply generating the necessary queues for message passing and then declaring the dispatchers as hardware tasks. Resource management and binding to interrupts and all other initialization steps will be taken care of by the hardware pass in`rtic-core`

- `rp2040-rtic`: is the RTIC framework distribution for the RP2040 which defines the rp2040 specific hardware details and provides them to  `rtic-core` , `rtic-sw-pass` and other compilation passes crates to create the desired distribution.

- other passes like monotonics and multi-core pass will be described here later once they are implemented



### More

- [Modular rtic Implementation details + Example single core rp2040 rtic application](modular_rtic_impl.md)
- [Multicore rtic declarative model exporation](rtic_mc.md)

