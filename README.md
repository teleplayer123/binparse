**binparse is intended for educational use*

This program is a tool to parse a hexdump file into a binary file. There are also a few other misc uses such as xor encoding a file and parsing srec or ihex file.

## Example

The best use of binparse is to convert a hexdump, like what u-boot's "md.b" command returns, into a binary file. For this example we will just be using a compiled C program that will XOR encode a string with a key, both passed to the program as CLI arguments. Using the linux command "xxd" to create a hexdump, we will then use binparse to convert it back to a binary.

`$ xxd -g 1 hello > hello.txt`

Here is a snippet of the hexdump file:

`$ cat hello.txt | head -n 4`

>00000000: 7f 45 4c 46 02 01 01 00 00 00 00 00 00 00 00 00  .ELF............<br>
>00000010: 03 00 3e 00 01 00 00 00 80 10 00 00 00 00 00 00  ..>.............<br>
>00000020: 40 00 00 00 00 00 00 00 c8 36 00 00 00 00 00 00  @........6......<br>
>00000030: 00 00 00 00 40 00 38 00 0d 00 40 00 1f 00 1e 00  ....@.8...@.....<br>

Using binparse we can convert the hexdump text file into a binary by either compiling and calling the executable or using "cargo run". Here we will choose the latter:

`$ cargo run `

When no argumemnts are passed to the program it will show a usage message and exit. To convert our hexdump into a binary we will pass "uboot" as the mode arg, followed by the path to the hexdump file, and lastly the name we want to call the output binary file:

`$ cargo run uboot hello.txt hello.bin`

Comparing the original hello executable with the hello.bin binary will be almost identical. There is a padding issue at the end of the file (TODO), but this does not break the hello.bin program. Looking at the first 4 lines with "xxd" will reveal the same results as those from the text file above.

`$ xxd -g 1 hello.bin | head -n 4`

>00000000: 7f 45 4c 46 02 01 01 00 00 00 00 00 00 00 00 00  .ELF............<br>
>00000010: 03 00 3e 00 01 00 00 00 80 10 00 00 00 00 00 00  ..>.............<br>
>00000020: 40 00 00 00 00 00 00 00 c8 36 00 00 00 00 00 00  @........6......<br>
>00000030: 00 00 00 00 40 00 38 00 0d 00 40 00 1f 00 1e 00  ....@.8...@.....<br>

Although the example above is the primary use for this tool, there are a few other things binparse can do. Another use is taking a file, XOR encoding the contents, and writing the encoded data to a file:

