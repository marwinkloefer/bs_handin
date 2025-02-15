# hhuTOS
hhuTOS = hhu Teaching Operating System.

This file describes all cargo make commands for building, running, and debugging hhuTOS. 

Last update: 15.4.2024.

## Compiling
For a full build run: 

`cargo make`

## Running

To run the image, build it first and then use:

`cargo make qemu`

## Debugging 

Run following command in a terminal. This should open `qemu` but hhuTOS is not yet booted.

`cargo make qemu-gdb`

The in another terminal run.

`cargo make gdb`

or, if you want a source code window in text mode:

`cargo make gdbt`
