## 5 Remote Control Basics

This chapter provides basic information on operating an instrument via remote control.

### 5.1 Remote Control Interfaces and Protocols

The instrument supports different interfaces for remote control. The following table gives an overview.

<div style="text-align: center;"><div style="text-align: center;">Table 5-1: Remote control interfaces and protocols</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Interface</td><td style='text-align: center; word-wrap: break-word;'>Protocols, VISA $ ^{1} $ address string</td><td style='text-align: center; word-wrap: break-word;'>Remarks</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Local Area Network (LAN)</td><td style='text-align: center; word-wrap: break-word;'>Protocols:• HiSLIP High-Speed LAN Instrument Protocol (IVI-6.1)VISA $ ^{1} $ address string:TCPIP::host address::hislip0[::INSTR]• VXI-11VISA $ ^{1} $ address string:TCPIP::host address[::LAN device name][::INSTR]• socket communication (Raw Ethernet, simple telnet)VISA $ ^{1} $ address string:TCPIP::host address[::LAN device name]:&lt;port&gt;::SOCKET</td><td style='text-align: center; word-wrap: break-word;'>A LAN connector is located on the front or rear panel of the instrument, or both.The interface is based on TCP/IP and supports various protocols.For a description of the protocols refer to:• Chapter 5.1.3.1, &quot;HiSLIP protocol&quot;, on page 244• Chapter 5.1.3.2, &quot;VXI-11 protocol&quot;, on page 244• Chapter 5.1.3.3, &quot;Socket communication&quot;, on page 244</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Serial Interface</td><td style='text-align: center; word-wrap: break-word;'>VISA $ ^{1} $ address string:ASRL[0-9][::INSTR]</td><td style='text-align: center; word-wrap: break-word;'>For a description of the interface, refer to Chapter 5.1.5, &quot;Serial Interface&quot;, on page 246.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>GPIB (IEC/IEEE Bus Interface)</td><td style='text-align: center; word-wrap: break-word;'>VISA $ ^{1} $ address string:GPIB::primary address[::INSTR] (no secondary address)</td><td style='text-align: center; word-wrap: break-word;'>Optional GPIB bus interfaces according to standard IEC 625.1/IEEE 488.1 are located on the rear panel of the instrument.For a description of the interface, refer to Chapter 5.1.6, &quot;GPIB Interface (IEC/IEEE Bus Interface)&quot;, on page 246.Note: Within this interface description, the term GPIB is used as a synonym for the IEC/IEEE bus interface.</td></tr><tr><td colspan="3">$ ^{*} $ VISA is a standardized software interface library providing input and output functions to communicate with instruments. A VISA installation on the controller is a prerequisite for remote control over LAN (when using VXI-11 or HiSLIP protocol), USB and serial interface. For remote control via socket communication VISA installation is optional. For more information, see Chapter 5.1.1, &quot;VISA Libraries&quot;, on page 241.</td></tr></table>

Rohde & Schwarz provides the standardized I/O software library R&S VISA for communication via TCP/IP (LAN: HiSlip, VXI-11 and raw socket) or USB (USB/TMC) interfaces.

R&S VISA is available for download at the Rohde & Schwarz website http://www.rohde-schwarz.com/rsvisa.

How to configure the remote control interfaces, see Chapter 5.2, "Starting a Remote Control Session", on page 249.

##### SCPI (Standard Commands for Programmable Instruments)

SCPI commands - messages - are used for remote control. Commands that are not taken from the SCPI standard follow the SCPI syntax rules. The instrument supports the SCPI version 1999. The SCPI standard is based on standard IEEE 488.2 and aims at the standardization of device-specific commands, error handling and the status registers. The tutorial "Automatic Measurement Control - A tutorial on SCPI and IEEE 488.2" from John M. Pieper (R&S order number 0002.3536.00) offers detailed information on concepts and definitions of SCPI.

Tables provide a fast overview of the bit assignment in the status registers. The tables are supplemented by a comprehensive description of the status registers.

#### 5.1.1 VISA Libraries

VISA is a standardized software interface library providing input and output functions to communicate with instruments. Thus, you can configure the interface and must not adjust the application program to the used interface. The I/O channel (LAN or TCP/IP, USB, GPIB,...) is selected at initialization time with the channel-specific address string ("VISA resource string"), or by a defined VISA alias (short name). See also Chapter 5.1, "Remote Control Interfaces and Protocols", on page 240 for an overview.

Instrument access via VXI-11 or HiSLIP protocols is achieved from high level programming platforms using VISA as an intermediate abstraction layer. VISA encapsulates the low-level VXI or GPIB function calls and thus makes the transport interface transparent for the user.

A VISA installation is a prerequisite for remote control using the following interfaces:

• LAN Interface using Chapter 5.1.3, "LAN Interface", on page 242

• LAN interface using Chapter 5.1.3.2, "VXI-11 protocol", on page 244

Chapter 5.1.4, "USB Interface", on page 245

Chapter 5.1.6, "GPIB Interface (IEC/IEEE Bus Interface)", on page 246

Chapter 5.1.5, "Serial Interface", on page 246

Instrument access via the LAN socket protocol or GPIB connections can be operated both, with or without the VISA library.

See also Chapter 5.1.3.3, "Socket communication", on page 244 and Chapter 5.1.6, "GPIB Interface (IEC/IEEE Bus Interface)", on page 246.

For more information about VISA, refer to the user documentation.

#### 5.1.2 Messages

The messages transferred on the data lines are divided into the following categories:

• Interface messages

Interface messages are transmitted to the instrument on the data lines, with the attention line being active (LOW). They are used to communicate between the controller and the instrument. Interface messages can only be sent by instruments that have GPIB bus functionality. For details see the sections for the required interface.

##### • Instrument messages

Instrument messages are employed in the same way for all interfaces, if not indicated otherwise in the description. Structure and syntax of the instrument messages are described in Chapter 5.3, "SCPI command structure", on page 263. A detailed description of all messages available for the instrument is provided in the chapter "Remote Control Commands".

There are different types of instrument messages, depending on the direction they are sent:

- Commands

– Instrument responses

##### Commands

Commands (program messages) are messages the controller sends to the instrument. They operate the instrument functions and request information. The commands are subdivided according to two criteria:

• According to the effect they have on the instrument:

– Setting commands cause instrument settings such as a reset of the instrument or setting the frequency.

– Queries cause data to be provided for remote control, e.g. for identification of the instrument or polling a parameter value. Queries are formed by directly appending a question mark to the command header.

• According to their definition in standards:

Common commands: their function and syntax are precisely defined in standard IEEE 488.2. They are employed identically on all instruments (if implemented). They refer to functions such as management of the standardized status registers, reset and self-test.

– Instrument control commands refer to functions depending on the features of the instrument such as frequency settings. Many of these commands have also been standardized by the SCPI committee. These commands are marked as "SCPI confirmed" in the command reference chapters. Commands without this SCPI label are instrument-specific; however, their syntax follows SCPI rules as permitted by the standard.

##### Instrument responses

Instrument responses (response messages and service requests) are messages the instrument sends to the controller after a query. They can contain measurement results, instrument settings and information on the instrument status.

#### 5.1.3 LAN Interface

To be integrated in a LAN, the instrument is equipped with a LAN interface, consisting of a connector, a network interface card and protocols. For remote control via a network, the PC and the instrument must be connected via the LAN interface to a common network with TCP/IP network protocol. They are connected using a commercial RJ45 cable. The TCP/IP network protocol and the associated network services are

preconfigured on the instrument. Software for instrument control and (for specified pro-

tocols only) the VISA program library must be installed on the controller.

##### VISA library

Instrument access via VXI-11 or HiSLIP protocols is achieved from high level programming platforms using VISA as an intermediate abstraction layer. VISA encapsulates the low level VXI or GPIB function calls and thus makes the transport interface transparent for the user. See Chapter 5.1.1, "VISA Libraries", on page 241 for details.

##### IP address

Only the IP address or the computer name (LAN device name) is required to set up the connection. The IP address/computer name is part of the "VISA resource string" used by the programs to identify and control the instrument.

Forms of the VISA resource string:

TCPIP::host address[::LAN device name][::INSTR]

TCPIP::host address::port::SOCKET

##### Where:

• TCP/IP designates the network protocol used

• host address is the IP address or host name of the device

- LAN device name defines the protocol and the instance number of a subinstrument:

inst0 selects the VXI-11 protocol (optional, default)

hislip0 selects the newer HiSLIP protocol

• INSTR indicates the instrument resource class (optional)

• port determines the used port number

• SOCKET indicates the raw network socket resource class

##### Example:

• Instrument has the IP address 192.1.2.3; the valid resource string using VXI-11 protocol is:

TCPIP::192.1.2.3::INSTR

- The DNS host name is RSSM1; the valid resource string is:

  TCPIP::RSSM1::hislip0 (HiSLIP)

  TCPIP::RSSM1::INSTR (VXI-11)

- A raw socket connection can be established using: TCPIP::192.1.2.3::5025::SOCKET

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//b0351068-9f20-400c-866f-1b84a64edb32/markdown_2/imgs/img_in_image_box_218_1303_272_1357.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A53Z%2F-1%2F%2F5035ccb78410ed480e69c769a36fc8e7e8f4cb88d666cb83c8932d2273fd5ca9" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//b0351068-9f20-400c-866f-1b84a64edb32/markdown_2/imgs/img_in_image_box_218_1303_272_1357.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A53Z%2F-1%2F%2F5035ccb78410ed480e69c769a36fc8e7e8f4cb88d666cb83c8932d2273fd5ca9" alt="Image" width="4%" />

i

</div>


</div>


##### Identifying instruments in a network

If several instruments are connected to the network, each instrument has its own IP address and associated resource string. The controller identifies these instruments by the resource string.

##### 5.1.3.1 HiSLIP protocol

The High Speed LAN Instrument Protocol (HiSLIP) is the successor protocol for VXI-11 for TCP-based instruments specified by the IVI foundation. The protocol uses two TCP sockets for a single connection - one for fast data transfer, the other for non-sequential control commands (e.g. Device Clear or SRQ).

HiSLIP has the following characteristics:

• High performance as with raw socket network connections

• Compatible IEEE 488.2 support for Message Exchange Protocol, Device Clear, Serial Poll, Remote/Local, Trigger, and Service Request

- Uses a single IANA registered port (4880), which simplifies the configuration of firewalls

- Supports simultaneous access of multiple users by providing versatile locking mechanisms

• Usable for IPv6 or IPv4 networks

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//b0351068-9f20-400c-866f-1b84a64edb32/markdown_3/imgs/img_in_image_box_218_667_272_721.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A53Z%2F-1%2F%2F43b9cec9e64a58fe79f2fff7eba4c7dce77ddb6c9794eb58888e2b9e19e3a0a0" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//b0351068-9f20-400c-866f-1b84a64edb32/markdown_3/imgs/img_in_image_box_218_667_272_721.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A53Z%2F-1%2F%2F43b9cec9e64a58fe79f2fff7eba4c7dce77ddb6c9794eb58888e2b9e19e3a0a0" alt="Image" width="4%" />

i

</div>


</div>


Using VXI-11, each operation is blocked until a VXI-11 instrument handshake returns. However, using HiSLIP, data is sent to the instrument using the "fire and forget" method with immediate return. Thus, a successful return of a VISA operation such as viWrite() guarantees only that the command is delivered to the instrument's TCP/IP buffers. There is no confirmation, that the instrument has started or finished the requested command.

For more information see also the application note:

1MA208: Fast Remote Instrument Control with HiSLIP

##### 5.1.3.2 VXI-11 protocol

The VXI-11 standard is based on the ONC RPC (Open Network Computing Remote Procedure Call) protocol which in turn relies on TCP/IP as the network/transport layer. The TCP/IP network protocol and the associated network services are preconfigured. TCP/IP ensures connection-oriented communication, where the order of the exchanged messages is adhered to and interrupted links are identified. With this protocol, messages cannot be lost.

##### 5.1.3.3 Socket communication

An alternative way for remote control of the product is to establish a simple network communication using sockets. The socket communication, also referred to as "Raw Ethernet communication", does not necessarily require a VISA installation on the remote controller side. It is available by default on all operating systems.

The simplest way to establish socket communication is to use the built-in telnet program. The telnet program is part of every operating system and supports a communication with the software on a command-by-command basis. For more convenience and to enable automation by programs, user-defined sockets can be programmed.

Socket connections are established on a specially defined port. The socket address is a combination of the IP address or the host name of the instrument and the number of the port configured for remote-control. Typically, the products of Rohde & Schwarz use port number 5025 for this purpose. The port is configured for communication on a command-to-command basis and for remote control from a program.

##### 5.1.3.4 LAN interface messages

In the LAN connection, the interface messages are called low-level control messages. These messages can be used to emulate interface messages of the GPIB bus.



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Command</td><td style='text-align: center; word-wrap: break-word;'>Long term</td><td style='text-align: center; word-wrap: break-word;'>Effect on the instrument</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;ABO</td><td style='text-align: center; word-wrap: break-word;'>Abort</td><td style='text-align: center; word-wrap: break-word;'>Aborts processing of the commands just received.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;DCL</td><td style='text-align: center; word-wrap: break-word;'>Device Clear</td><td style='text-align: center; word-wrap: break-word;'>Aborts processing of the commands just received and sets the command processing software to a defined initial state. Does not change the instrument setting.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;GTL</td><td style='text-align: center; word-wrap: break-word;'>Go to Local</td><td style='text-align: center; word-wrap: break-word;'>Transition to the &quot;local&quot; state (manual control). (The instrument automatically returns to remote state when a remote command is sent UNLESS &amp;NREN was sent before.)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;GTR</td><td style='text-align: center; word-wrap: break-word;'>Go to Remote</td><td style='text-align: center; word-wrap: break-word;'>Enables automatic transition from local state to remote state by a subsequent remote command (after &amp;NREN was sent).</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;GET</td><td style='text-align: center; word-wrap: break-word;'>Group Execute Trigger</td><td style='text-align: center; word-wrap: break-word;'>Triggers a previously active instrument function (e.g. a sweep). The effect of the command is the same as with that of a pulse at the external trigger signal input.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;LLO</td><td style='text-align: center; word-wrap: break-word;'>Local Lockout</td><td style='text-align: center; word-wrap: break-word;'>Disables transition from remote control to manual control by means of the front panel keys.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;NREN</td><td style='text-align: center; word-wrap: break-word;'>Not Remote Enable</td><td style='text-align: center; word-wrap: break-word;'>Disables automatic transition from local state to remote state by a subsequent remote command. (To re-activate automatic transition use &amp;GTR.)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&amp;POL</td><td style='text-align: center; word-wrap: break-word;'>Serial Poll</td><td style='text-align: center; word-wrap: break-word;'>Starts a serial poll.</td></tr></table>

#### 5.1.4 USB Interface

For remote control via the USB connection, the PC and the instrument must be connected via the USB type B interface. A USB connection requires the VISA library to be installed. VISA detects and configures the R&S instrument automatically when the USB connection is established. You do not have to enter an address string or install a separate driver.

##### USB address

The used USB address string is:

USB::<vendor ID>::<product ID>::<serial number>[::INSTR]

Where:

<vendor ID> is the vendor ID for Rohde&Schwarz

<product ID> is the product ID for the R&S instrument

<serial number> is the individual serial number on the rear of the instrument

##### Example:

USB::0x0AAD::0x0054::100001::INSTR

0x0AAD is the vendor ID for Rohde&Schwarz

0x0054 is the product ID for the R&S SMB

100001 is the serial number of the particular instrument

#### 5.1.5 Serial Interface

Remote control via the serial interface is possible either via RS232 interface or via a Bluetooth connection. The controller/Bluetooth device and the instrument must be connected via an external USB/serial-adapter (see recommended extras, data sheet) and a serial crossover (null modem) cable. A USB connection requires the VISA library to be installed on the controller. VISA detects and configures the R&S SMB automatically when the USB connection is established.

##### Serial address

The used serial address string is:

ASRL[0-9] [::INSTR]

Where ASRL [0-9] determines the number of the COM port on the controller side, that has to be used for the serial connection.

Access via a bluetooth device requires the entry of the bluetooth pin in addition (see Chapter 4.2.3.14, "Security", on page 114).

To enable an error-free and correct data transmission, the parameters of the generator and the controller must have the same setting. The serial interface is preset for a baud rate 115200, no parity and one stop bit. The parameters can be manually changed in "Remote Channel Settings" dialog (see Chapter 4.2.3.10, "Remote Channel Settings", on page 109).

#### 5.1.6 GPIB Interface (IEC/IEEE Bus Interface)

To be able to control the instrument via the GPIB bus, the instrument and the controller must be linked by a GPIB bus cable. A GPIB bus card, the card drivers and the program libraries for the programming language used must be provided in the controller. The controller must address the instrument with the GPIB bus address (see Chapter 5.1.6.2, "GPIB Instrument Address", on page 248).

##### Characteristics

The GPIB interface is described by the following characteristics:

• Up to 15 instruments can be connected

- The total cable length is restricted to a maximum of 15 m; the cable length between two instruments should not exceed 2m.

- A wired "OR"-connection is used if several instruments are connected in parallel, since the slowest instrument determines the speed.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_1/imgs/img_in_image_box_218_340_272_393.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2F4c5c35f55e6008602b0d0787f988131907b186f8d11e0bcba7ff56ceffebe3f9" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_1/imgs/img_in_image_box_218_340_272_393.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2F4c5c35f55e6008602b0d0787f988131907b186f8d11e0bcba7ff56ceffebe3f9" alt="Image" width="4%" />

i

</div>


</div>


Any connected IEC bus cable must be terminated by an instrument or controller.

##### 5.1.6.1 GPIB interface messages

Interface messages are transmitted to the instrument on the data lines, with the attention line (ATN) being active (LOW). They are used for communication between the controller and the instrument and can only be sent by a computer which has the function of a GPIB bus controller. GPIB interface messages can be further subdivided into:

- Universal commands: act on all instruments connected to the GPIB bus without previous addressing

• Addressed commands: only act on instruments previously addressed as listeners

##### Universal commands

Universal commands are encoded in the range 10 through 1F hex. They affect all instruments connected to the bus and do not require addressing.



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Command</td><td style='text-align: center; word-wrap: break-word;'>Effect on the instrument</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>DCL (Device Clear)</td><td style='text-align: center; word-wrap: break-word;'>Aborts the processing of the commands just received and sets the command processing software to a defined initial state. Does not change the instrument settings.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>IFC (Interface Clear)  $ * $</td><td style='text-align: center; word-wrap: break-word;'>Resets the interfaces to the default setting.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>LLO (Local Lockout)</td><td style='text-align: center; word-wrap: break-word;'>The &quot;Local&quot; softkey is disabled. Manual operation is no longer available until GTL is executed.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>SPE (Serial Poll Enable)</td><td style='text-align: center; word-wrap: break-word;'>Ready for serial poll.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>SPD (Serial Poll Disable)</td><td style='text-align: center; word-wrap: break-word;'>End of serial poll.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>PPU (Parallel Poll Unconfigure)</td><td style='text-align: center; word-wrap: break-word;'>End of the parallel-poll state.</td></tr><tr><td colspan="2">$ * $ ) IFC is not a real universal command, it is sent via a separate line; however, it also affects all instruments connected to the bus and does not require addressing</td></tr></table>

##### Addressed commands

Addressed commands are encoded in the range 00 through 0F hex. They only affect instruments addressed as listeners.



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Command</td><td style='text-align: center; word-wrap: break-word;'>Effect on the instrument</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>GET (Group Execute Trigger)</td><td style='text-align: center; word-wrap: break-word;'>Triggers a previously active instrument function (e.g. a sweep). The effect of the command is the same as with that of a pulse at the external trigger signal input.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>GTL (Go to Local)</td><td style='text-align: center; word-wrap: break-word;'>Transition to the &quot;local&quot; state (manual control).</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>GTR (Go to Remote)</td><td style='text-align: center; word-wrap: break-word;'>Transition to the &quot;remote&quot; state (remote control).</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>PPC (Parallel Poll Configure)</td><td style='text-align: center; word-wrap: break-word;'>Configures the instrument for parallel poll.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>SDC (Selected Device Clear)</td><td style='text-align: center; word-wrap: break-word;'>Aborts the processing of the commands just received and sets the command processing software to a defined initial state. Does not change the instrument setting.</td></tr></table>

##### 5.1.6.2 GPIB Instrument Address

In order to operate the instrument via remote control, it must be addressed using the GPIB address. The remote control address is factory preset, but it can be changed if it does not fit in the network environment. For remote control, addresses 0 through 30 are allowed. The GPIB address is maintained after a reset of the instrument settings.

##### Changing the GPIB address of the instrument

The GPIB address can be changed manually or using a remote control command.

1. Manually: press the [SETUP] key.

2. Select "Remote > GPIB".



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td colspan="3">Remote Channel Settings</td></tr></table>

3. Enter the GPIB address.

4. Remotely: use the remote control command:

SYST:COMM:GPIB:ADDR 18

#### 5.1.7 LXI Browser Interface

LAN extension for instrumentation (LXI) is an instrumentation platform for measuring instruments and test systems that is based on standard Ethernet technology.

See Chapter 3.1.17, "LXI Configuration", on page 39.

The LXI browser interface allows easy configuration of the LAN and remote control of the R&S SMB without additional installation requirements. The instrument's LXI browser interface works correctly with all W3C compliant browsers.

The LAN settings are configured using the LXI Browser Interface of the R&S SMB described in Chapter 3.1.17.2, "LAN Configuration", on page 42. The LXI status settings in the R&S SMB are described in Chapter 4.2.3.12, "LXI Status", on page 112.

### 5.2 Starting a Remote Control Session

The instrument and the controller have to be connected with the suitable cable and switched on.

A remote control program must open a connection to the instrument (using VISA functionality), before it can send commands to and receive device responses from the instrument.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_3/imgs/img_in_image_box_218_436_272_490.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A22Z%2F-1%2F%2Fedf203e51c82d0341f5a45d25874145cc93828705f2f3e708db0090a0a6fbd70" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_3/imgs/img_in_image_box_218_436_272_490.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A22Z%2F-1%2F%2Fedf203e51c82d0341f5a45d25874145cc93828705f2f3e708db0090a0a6fbd70" alt="Image" width="4%" />

i

</div>


</div>


##### Instrument Address

In order to operate the instrument via remote control it must be addressed using the defined interface address. See Chapter 5.1.3, "LAN Interface", on page 242, Chapter 5.1.4, "USB Interface", on page 245, Chapter 5.1.5, "Serial Interface", on page 246, Chapter 5.1.6, "GPIB Interface (IEC/IEEE Bus Interface)", on page 246 or Chapter 5.1.7, "LXI Browser Interface", on page 248 for details.

The VISA resource strings are indicated in the "Setup > Remote Channel Settings" menu.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_3/imgs/img_in_image_box_225_693_263_750.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2F68a9cac0c3d25cb3aa7aac201985b2bf9459a23c611ec1ed9c488c3690976148" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_3/imgs/img_in_image_box_225_693_263_750.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2F68a9cac0c3d25cb3aa7aac201985b2bf9459a23c611ec1ed9c488c3690976148" alt="Image" width="3%" />

?

</div>


</div>


##### Securing the display

To prevent unauthorized personnel from reading the display, you can disable the frequency and level display explicitly. This is useful when you remotely control the instrument from a different location.

For information on how to disable the frequency and level display, refer to "Annotation Frequency" on page 119 and "Annotation Amplitude" on page 119.

Refer to Chapter 5.2.3, "Examples", on page 250 for practical examples on setting up of a remote control link and starting of a remote control session.

#### 5.2.1 Switching to Remote Control

##### Starting remote control

After switching on, the instrument is usually in the local state and can be operated via the front panel controls (for instruments equipped with a display), a mouse and an external keyboard.

1. Send a command from a controller to the instrument.

The instrument changes to remote state as soon as it receives the command from the controller.

Note: If you have sent &NREN before, the automatic transition from local state to manual control by a subsequent remote command is disabled (use &GTR to enable it again).

In remote state, operation via the front panel or via mouse and keyboard is disabled. The status line indicates the "REMOTE" state.

The instrument remains in the remote state until it is reset to the local state, see Chapter 5.2.2, "Returning to Manual Operation", on page 250).

Tip: Switching from manual operation to remote control and vice versa does not affect the other instrument settings.

2. Although operation via front panel, mouse and keyboard is disabled, the dialog boxes can still be opened, for example to verify settings. The buttons and setting fields are grayed out and cannot be activated.

Additionally, you can disable the access to the dialogs with the command SYST:KLOC ON to protect the instrument against unauthorized readings.

3. To prevent unintentional return to manual operation, disable the [LOCAL] key of the instrument with the &LLO command (see Chapter 5.1.3.4, "LAN interface messages", on page 245).

The instrument switches to "REM-LLO" state.

The automatic transition from local state to remote state by a subsequent remote command, and the command *GTL are disabled.

To return to manual mode is only possible via remote control.

4. Unlock the [LOCAL] key with &GTR.

#### 5.2.2 Returning to Manual Operation

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_4/imgs/img_in_image_box_218_860_272_913.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2Fe30eb348768ca8baedca02e4faf60de8376f13fb922dd340a14a5f8d9d9366b6" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_4/imgs/img_in_image_box_218_860_272_913.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2Fe30eb348768ca8baedca02e4faf60de8376f13fb922dd340a14a5f8d9d9366b6" alt="Image" width="4%" />

i

</div>


</div>


Before returning to manual control, command processing must be completed. Otherwise, the instrument switches back to remote control immediately.

To return to manual operation, perform one of the following:

• Press the [LOCAL] key on the front panel.

• Select "Setup > Remote Control Channels > Local".

• While using the socket communication, terminate the remote control session.

• Send the interface command &GTL via the remote control interface.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_4/imgs/img_in_image_box_225_1133_262_1188.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2F59adc2bda6e947b91564c968f5383f46991f418da51b73cd44dbc7cae5661cf3" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f3c8f346-9cdf-4104-b287-b00e64b07d06/markdown_4/imgs/img_in_image_box_225_1133_262_1188.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A23Z%2F-1%2F%2F59adc2bda6e947b91564c968f5383f46991f418da51b73cd44dbc7cae5661cf3" alt="Image" width="3%" />

?

</div>


</div>


Use the &GTR to enable the [LOCAL] key if it is locked.

#### 5.2.3 Examples

This sections provides examples for setting up the remote control connection, and starting a remote control session.

This section assumes basic knowledge of programming and operation of the controller. A description of the interface commands can be obtained from the corresponding manuals.

##### 5.2.3.1 Remote Control over GPIB

The program example in this section is written in VISUAL BASIC. A condition for programming in VISUAL BASIC is that the modules NIGGLOBAL (Niglobal.bas) and VIB32 (Vbib_32.bas) are added to the projects.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_0/imgs/img_in_image_box_226_361_262_415.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A08Z%2F-1%2F%2F5d74154c1cdd3e627753bbf97d2d20a8ff0a3fa1ea81c59852408534f331600c" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_0/imgs/img_in_image_box_226_361_262_415.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A08Z%2F-1%2F%2F5d74154c1cdd3e627753bbf97d2d20a8ff0a3fa1ea81c59852408534f331600c" alt="Image" width="3%" />

?

</div>


</div>


Drivers for instrument, e.g. IVI-COM and LabVIEW drivers, are available in the download area of the product website (http://www.rohde-schwarz.com/en/products/test_and_measurement/product_categories/signal_generation/).

##### Starting a remote control session over GPIB

As a prerequisite, the GPIB address of the instrument, which is factory-set to 28, must not have been changed.

1. Connect instrument and controller using GPIB cable and switch them on.

2. Execute following commands on the controller:

a) Open port to the instrument
CALL IBFIND("DEV1", generator%)
b) Inform controller about instrument address
CALL IBPAD(generator%, 28)
c) Reset instrument
CALL IBWRT(generator%, "*RST;*CLS")
d) Set instrument to new address
CALL IBWRT(generator%, "SYST:COMM:GPIB:ADDR 18")
e) Inform controller about new address
CALL IBPAD(generator%, 18)

The GPIB address of the instrument is changed.

3. To return to manual operation sent CALL IBLOC (generator%) or press the [LOCAL] key at the front panel.

##### 5.2.3.2 Remote Control over LAN using VXI-11 Protocol

In this example, the I/O software library R&S VISA from Rohde & Schwarz is used to set up a LAN remote control link and remotely control the R&S SMB. R&S VISA is running on a controller PC with Windows operating system. When the connection is set up you can send commands to the instrument, and receive the responses.

The remote control connection requires a VISA installation but no additional hardware on the controller PC. The LAN I/O channel is selected at initialization time using the VISA resource string (also referred to as "address string"). A VISA alias (short name) is used to replace the complete resource string. The host address is either the R&S SMB's hostname or IP address. See also Chapter 5.1.3, "LAN Interface", on page 242.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_218_205_272_259.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2Fcb9383598f97eac7ad4f89b7dfec744457e186a8c643e010ed97a88bbce0c831" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_218_205_272_259.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2Fcb9383598f97eac7ad4f89b7dfec744457e186a8c643e010ed97a88bbce0c831" alt="Image" width="4%" />

i

</div>


</div>


In this example, it is assumed that:

- A LAN remote control link between the controller and the R&S SMB is already set up.

- The R&S VISA program is installed on the remote PC, see "http://www.rohdeschwarz.com/rsvisa > RS VISA Release Notes".

##### Configuring the controller

To remote control the R&S SMB, we use the R&S VISA Tester application.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_225_485_263_542.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2F906b04be76b77a5b9e4072aa3493bb872cc928947239968a51507375c72cc114" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_225_485_263_542.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2F906b04be76b77a5b9e4072aa3493bb872cc928947239968a51507375c72cc114" alt="Image" width="3%" />

O

</div>


</div>


The instrument is preconfigured for networks using DHCP (dynamic host configuration protocol). If this configuration is used, enter the computer name in the position of the IP address.

To enable the external controller to communicate with the R&S SMB via TCP/IP protocol, set up a remote control link as follows:

1. Make sure that the controller and the instrument are connected in the network (network cable) and switched on.

2. On the controller, start "R&S VISA > Tester 32bit" or "R&S VISA > Tester 64bit", respectively.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_329_799_1065_1292.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2F94c9161a5dfa53f59168e13925ffdffcd289c694ac6ed57ebd220f03478f2e7f" alt="Image" width="61%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_1/imgs/img_in_image_box_329_799_1065_1292.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A09Z%2F-1%2F%2F94c9161a5dfa53f59168e13925ffdffcd289c694ac6ed57ebd220f03478f2e7f" alt="Image" width="61%" />

RsVisaTester 5.5.4 Visa from Rohde & Schwarz GmbH (5.5.4)

Find Resource Change Log File RsVisa Config Choose Visa Implementation

Resource
Resource
Timeout Current log file ✓ Show Log
Connect 2000 Open Log visaTester\log20151109_104402.txt Write Log

Line Duration Status Visa Operation

Basis Locking Attributes Events Gpib Tests
8 1 μs VI SUCCESS viGetAttribute(sessionid=...
9 1 μs VI SUCCESS viGetAttribute(sessionid=...
10 7 μs VI SUCCESS viSetAttribute(sessionid=...
11 4 μs VI SUCCESS viGetAttribute(sessionid=...
12 8 μs VI SUCCESS viGetAttribute(sessionid=...
13 11 μs VI SUCCESS viGetAttribute(sessionid=...
14 14 m s VI SUCCESS viOpen(sesn=3, rsrc=TCPL...
15 13 μs VI SUCCESS viSetAttribute(sessionid=4...
16 1 m s VI SUCCESS viClose(sessionid=4)
17 13 m s VI SUCCESS viOpen(sesn=3, rsrc=TCPL...
18 7 μs VI SUCCESS viSetAttribute(sessionid=5...
19 1 m s VI SUCCESS viClose(sessionid=5)
20 7 μs VI SUCCESS viGetAttribute(sessionid=...
21 1 μs VI SUCCESS viGetAttribute(sessionid=...
22 1 μs VI SUCCESS viGetAttribute(sessionid=...
23 9 μs VI SUCCESS viSetAttribute(sessionid=3...

Live Mode Lines: 23 Clear Read STB Trigger Clear View Mode Displayed: 16

0x---
Status: VI SUCCESS

</div>


</div>


<div style="text-align: center;"><div style="text-align: center;">3. In the menu bar, select "Choose VISA Implementation > Rohde & Schwarz Visa".</div> </div>


<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_329_206_547_344.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fb0dbb584cbd2f0b7c67b4db1203b6b74a669bd8a07d34b5d968168163e0942e0" alt="Image" width="18%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_329_206_547_344.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fb0dbb584cbd2f0b7c67b4db1203b6b74a669bd8a07d34b5d968168163e0942e0" alt="Image" width="18%" />

Choose VISA implem...
Rohde & Schwarz Visa
Default Visa
OK
Cancel

</div>


</div>


4. Select "Rohde & Schwarz Visa" and confirm with "OK".

5. In the menu bar, select "Find Resource" to search for the instrument in the LAN.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_329_454_908_714.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fd39af161e687137ea59ca17db47f24154af36c7cbd6a91f7258a6dc109349fcb" alt="Image" width="48%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_329_454_908_714.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fd39af161e687137ea59ca17db47f24154af36c7cbd6a91f7258a6dc109349fcb" alt="Image" width="48%" />

Find and select resource
Resource
Find Resources
LXI (mDNS)
VXI-11
1000
Select
?*
Cancel

</div>


</div>


6. Select "VXI-11" and "Find Resources".

R&S VISA scans the network for connected instruments and lists all detected instu-

ments in the "Resource" list.

Note: The search may take some time, particularly in large networks.

7. Select the required instrument and confirm with "Select".

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_334_912_917_1197.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fd8428ee815fa2db1e3cb95e18f085aaa9d86c900d40e34b3e8f5c11ed128432e" alt="Image" width="48%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_2/imgs/img_in_image_box_334_912_917_1197.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A10Z%2F-1%2F%2Fd8428ee815fa2db1e3cb95e18f085aaa9d86c900d40e34b3e8f5c11ed128432e" alt="Image" width="48%" />

Find and select resource
Resource
Find Resources
TCPIP0::10.113.1.151
TCPIP0::10.113.1.151::inst0::INSTR
TCPIP0::10.113.1.154
TCPIP0::10.113.1.179
TCPIP0::10.113.1.188
TCPIP0::10.113.1.18
TCPIP0::10.113.1.23
TCPIP0::10.113.1.151::inst0::INSTR
?*

Select
Cancel

</div>


</div>


The "Find and select resource" dialog closes and R&S VISA indicates the instruments IP address in the "Resource" field of the main application window.

8. As an alternative to the IP address, you can assign an alias name to the R&S SMB:

a) In the menu bar, select "RsVisaConfig".

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_364_239_1056_453.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F7ff40e50936c9ca1f4f975dba83f61a58640073672432dcd5942b778f8f933a6" alt="Image" width="58%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_364_239_1056_453.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F7ff40e50936c9ca1f4f975dba83f61a58640073672432dcd5942b778f8f933a6" alt="Image" width="58%" />

RsVisaConfigure - 5.5.4
Resource identifier Alias

</div>


</div>


b) In the toolbar, select "+" to access the "VISA Resource String Composer".

c) Fill in the "Alias" name, the "VISA Resource String" and the "Device IP Address or host name" as shown in the figure, and confirm with "OK".

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_361_560_1067_908.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F8b798914cb0e579a1b787d7dd51188d639702654f306e4da27cf76b00f557e22" alt="Image" width="59%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_361_560_1067_908.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F8b798914cb0e579a1b787d7dd51188d639702654f306e4da27cf76b00f557e22" alt="Image" width="59%" />

+ VISA Resource String Composer

Resource String
Alias
Rohde&Schwarz_SignalGenerator
VISA Resource String
TCPIP0::10.113.151.1::instr0::INSTR

Build Interface
Interface Type
VXI-11
Board Number
0

TCP/IP
Device IP Address or hostname
10.113.151.1
Device Id
0

OK
Cancel

</div>


</div>


The "Alias" name is assigned to the instrument.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_364_977_1056_1190.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2Fa4b665c5036ac27f997a4ee71616585016be5b581bbbe4bc7d92765d03ddf690" alt="Image" width="58%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_3/imgs/img_in_image_box_364_977_1056_1190.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2Fa4b665c5036ac27f997a4ee71616585016be5b581bbbe4bc7d92765d03ddf690" alt="Image" width="58%" />

RsVisaConfigure - 5.5.4

Resource identifier Alias
TCPIP0::10.113.151.. Rohde&Schwarz_SignalGenerator

</div>


</div>


d) Close the dialog.

The R&S SMB is now registered in the program and can be addressed via the resource string or alias name.

9. In the main window, select "Connect".

R&S VISA establishes the connection to the R&S SMB.

Now you can send settings to configure the instrument and receive its responses.

Note: If the connection cannot be set up, R&S VISA displays an error in the log view. For information on how to proceed when network failures occur, see Chapter 9.5, "Resolving network connection failures", on page 505.

For further information on the functions to read and write to an open session, as well as the utility applications the software provides, see the R&S VISA User Manual.

##### Starting a remote control over LAN (using VXI-11)

To set the instrument to remote control, you can use the addressed command &GTR, or send any command from the controller.

1. Start the R&S VISA Tester and establish the connection to the R&S SMB, see "Configuring the controller" on page 252.

2. In the R&S VISA "Basics" tab, enter a SCPI command, e.g. "*IDN?" and confirm with "Query".

The instrument is switched to remote control when it receives a command from the controller.

3. Select "Read" to obtain the instrument response.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_4/imgs/img_in_image_box_328_736_1064_1230.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F3433837fd0f7c44996a272ba3dac32f2d3697a43f0d6e22564e5f9313fa610b5" alt="Image" width="61%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//01004ea8-32a4-4a9a-a0bc-247b44921fd8/markdown_4/imgs/img_in_image_box_328_736_1064_1230.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A11Z%2F-1%2F%2F3433837fd0f7c44996a272ba3dac32f2d3697a43f0d6e22564e5f9313fa610b5" alt="Image" width="61%" />

RsVisaTester 5.5.4 Visa from Rohde & Schwarz GmbH (5.5.4)

Find Resource Change Log File RsVisa Config Choose Visa Implementation

Resource Timeout Current log file Show Log
TCPIP0::10.113.1.150::inst0::INSTR Disconnect 2000 Open Log visaTester\log20151112_124312.txt Write Log

Line Duration Status Visa Operation
10 452  $ \mu $s VI SUCCESS viWrite(sessionid=4,buf=...

Basic Locking Attributes Events Tests
*IDNT*
Write Read Query Clear History

Send End

Clear Text Read count 1024

Live Mode Lines: 10 Clear Read STB Trigger Clear View Mode Displayed: 1

Status: VI SUCCESS

</div>


</div>


Tip: If the "Show Log" checkbox is checked R&S VISA displays each VISA function call in the log-view on the left. If you check the "Write Log" checkbox the log-view entry is written to the log file as well. You can operate the log-view in two modes: the "Live Mode" shows only the most recent messages whereas the "View Mode" allows you to scroll the history.

4. To set, e.g. the frequency, enter SOUR1:FREQ 4 GHz and select "Write". To check the performed setting, SOUR1:FREQ? and select "Read".

The instrument response is 4000000000, i.e. the frequency is returned in Hz.

While remote control is active, the "Remote" icon in the status bar indicates that the instrument is in remote control mode. The operation via the front panel or via mouse and keyboard are locked, allowing a remote control program to be performed without interruption.

On the display, keys and entry fields are grayed out and cannot be activated or modified, but you can still open dialogs, for example to verify settings.

5. To disable the access to the dialogs, use the command SYST:KLOC ON.

6. To prevent unintentional return to manual operation, use the command &LLO. See also Chapter 5.1.3.4, "LAN interface messages", on page 245. The instrument switches to "Remote LLO" state. The [LOCAL] key is disabled.

7. To enable the [LOCAL] key, use the command &GTR.

8. To return to manual operation, see Chapter 5.2.2, "Returning to Manual Operation", on page 250.

Tip: Switching from manual operation to remote control and vice versa does not affect the other instrument settings.

##### 5.2.3.3 Remote Control over LAN using Socket Communication

This chapter provides an example on how to establish a remote control connection over telnet protocol and a simple sockets-based program example that can be further developed.

##### Setting up a Telnet Connection

To control the software, only a telnet program is required. The telnet program is part of every operating system.

1. To establish a Telnet connection with the R&S SMB, start the telnet program.

2. Enter the access string to connect to the instrument and confirm with [Enter]. The access string is composed of the open command short form) and the socket address. The socket address is a combination of the IP address or the host name of the R&S SMB and the number of the port configured for remote-control via telnet. The R&S SMB uses the port number 5025 for remote connection via Telnet. Example: o 10.113.1.150 5025

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_1/imgs/img_in_image_box_329_204_1089_582.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2F705e1f0f8e9b58a3ce734514aca43d82626cf9cf649a137f30b35876e6d90f85" alt="Image" width="63%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_1/imgs/img_in_image_box_329_204_1089_582.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2F705e1f0f8e9b58a3ce734514aca43d82626cf9cf649a137f30b35876e6d90f85" alt="Image" width="63%" />

Telnet 10.113.1.150
Welcome to Microsoft Telnet Client
Escape Character is 'CTRL++'
Microsoft Telnet > 0 10.113.1.150 5025
Connecting To 10.113.1.150...

</div>


</div>


The connection to the instrument is set up and you can send remote-control commands.

3. Even if the cursor is not visible on the screen, enter blind a remote-control command and confirm with "Enter".

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_1/imgs/img_in_image_box_329_748_1089_1125.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2F87a2d9cd4f48c77ac5faf2533d2714d91b1350506e3e7ac47c77a65b8a59efd1" alt="Image" width="63%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_1/imgs/img_in_image_box_329_748_1089_1125.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2F87a2d9cd4f48c77ac5faf2533d2714d91b1350506e3e7ac47c77a65b8a59efd1" alt="Image" width="63%" />

Telnet 10.113.1.150

</div>


</div>


<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_2/imgs/img_in_image_box_330_204_1089_581.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2Fbda48349c6f5ad58a613786b9fbade6f4569da2da4d2a67458947253d5509440" alt="Image" width="63%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//7c6dd6f6-c87e-4e6e-b91a-486acc7b6bf7/markdown_2/imgs/img_in_image_box_330_204_1089_581.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A27Z%2F-1%2F%2Fbda48349c6f5ad58a613786b9fbade6f4569da2da4d2a67458947253d5509440" alt="Image" width="63%" />

Telnet 10.113.1.150
Freq?
1500000000
freq 1.5 GHz
Freq?
1500000000
pow?
0
pow -30
pow?
-30

</div>


</div>


After the first remote-control command has been sent, the instrument is in the "REMOTE" state, i.e. instrument control from the front panel or via mouse and keyboard is disabled and "REMOTE" is displayed in the status line.

##### Telnet program examples

The following program example shows a simple TcpClient class that is intended to explain on how to get started with programming of sockets.

The example sets up a socket communication to R&S SMB and opens a simple user interface, very similar to the telnet, which allows input of commands. To enable real automation, further development of the program is required.

##### TcpClient.h

#include <string>
//defines structs for socket handling
#include <netinet/in.h>
using namespace std;
typedef struct sockaddr_in SockAddrStruct;
typedef struct hostent HostInfoStruct;
class TcpClient 
{
    public:
        TcpClient();
        ~TcpClient();
        void connectToServer(string &hostname, int port);
        void disconnect();
        void transmit(string &txString);
        void receive(string &rxString);
        string getCurrentHostName() const;
        int getCurrentPort() const;
    private:
        string currentHostName;
        int currentPort;
}

int currentSocketDescr;
SockAddrStruct serverAddress;
HostInfoStruct * currentHostInfo;
bool clientIsConnected;
int receiveBufferSize;

##### TcpClient.cpp

#include <string>

//defines structs for socket handling
#include <netinet/in.h>

using namespace std;

typedef struct sockaddr_in SockAddrStruct;
typedef struct hostent HostInfoStruct;
class TcpClient
{
    public:
        TcpClient();
        ~TcpClient();
        void connectToServer(string &hostname, int port);
        void disconnect();
        void transmit(string &txString);
        void receive(string &rxString);
        string getCurrentHostName() const;
        int getCurrentPort() const;

    private:
        string currentHostName;
        int currentPort;
        int currentSocketDesc;
        SockAddrStruct serverAddress;
        HostInfoStruct * currentHostInfo;
        bool clientIsConnected;
        int receiveBufferSize;
    };

#include <netdb.h>

#include <netinet/in.h>

#include <unistd.h>

#include "TcpClient.h"

TcpClient::TcpClient()
{
    currentHostName("")
    currentPort(0)

    currentSocketDesc(0)

    serverAddress()

    currentHostInfo(NULL)

    clientIsConnected(false)

    receiveBufferSize(1024)

}

TcpClient::~TcpClient()
{
    currentHostInfo = NULL;
}

void TcpClient::connectToServer( string &hostname, int port )
{
    currentHostInfo = gethostbyname( hostname.c_str() );
    if ( currentHostInfo == NULL )
    {
        currentHostName = "";
        currentPort = 0;
        currentHostInfo = NULL;
        clientIsConnected = false;
        printf("error connecting host\n");
    }
    currentHostName = hostname;
    currentPort = port;
    currentSocketDescr = socket(AF_INET, SOCK_STREAM, 0);
    if ( currentSocketDescr == 0 )
    {
        currentHostName = "";
        currentPort = 0;
        currentHostInfo = NULL;
        clientIsConnected = false;
        printf("can't create socket\n");
    }
    serverAddress.sin_family = currentHostInfo->h_addrtype;
    serverAddress.sin_port = htons( currentPort );
    memory((char *)&serverAddress.sin_addr.s_addr,
                 currentHostInfo->h_addr_list[0], currentHostInfo->h_length );
    if ( connect( currentSocketDescr, ( struct sockaddr *)&serverAddress,
                 sizeof( serverAddress ) ) < 0 )
    {
        throw string("can't connect server\n");
    }
    clientIsConnected = true;
}

void TcpClient::disconnect()
{
    if ( clientIsConnected )
    {
        close( currentSocketDescr );
    }
    currentSocketDescr = 0;
    currentHostName = "";
    currentPort = 0;
    currentHostInfo = NULL;
    clientIsConnected = false;
}

void TcpClient::transmit( string &txString )
{
    if (!clientIsConnected )
    {
        throw string("connection must be established before any data can be sent\n");
    }
    char * transmitBuffer = new char[txString.length() + 1];
    memcpy( transmitBuffer, txString.c_str(), txString.length() );
    transmitBuffer[txString.length()] = '\n'; //newline is needed!
    if (send( currentSocketDescriptor, transmitBuffer, txString.length() + 1, 0 ) < 0 )
    {
        throw string("can't transmit data\n");
    }
    delete [] transmitBuffer;
}

void TcpClient::receive( string &rxString )
{
    if (!clientIsConnected )
    {
        throw string("connection must be established before any data can be received\n");
        char * receiveBuffer = new char[receiveBufferSize];
        memset( receiveBuffer, 0, receiveBufferSize );
        bool receiving = true;
        while (receiving)
        {
            int receivedByteCount = recv( currentSocketDescriptor, receiveBuffer, receiveBufferSize, 0 );
            if (receivedByteCount < 0)
            {
                throw string("error while receiving data\n");
            }
            rxString += string( receiveBuffer );
            receiving = ( receivedByteCount == receiveBufferSize );
        }
        delete [] receiveBuffer;
    }
    string TcpClient::getCurrentHostName() const
{
        return currentHostName;
    }
    int TcpClient::getCurrentPort() const
{
        return currentPort;
    }

##### TelnetClient.cpp

#include <iostream>
#include "TcpClient.h"

out printosage()

cout << "usage: EthernetRawCommand <server-ip> [scpi-command]" << endl;

nt main( int argc, char *argv[] )

int errorCode = 0; // no error
bool useSingleCommand = false;
string singleCommand = "";
string hostname = "";
int port = 5025;
string input = "";
TcpClient client;
switch ( argc )
{
    case 3:
        useSingleCommand = true;
        singleCommand = argv[2];
        case 2:
            hostname = argv[1];
            break;
        default:
            printUsage();
            return (-1);
}

try
{
    client.connectToServer(hostname, port);
    bool terminate = false;
    while (!terminate)
    {
        char buffer[1024];
        if ( useSingleCommand )
        {
            input = singleCommand; // send string
        }
        else
        {
            cin.getline( buffer, 1024 );
            input = buffer;
            if ( input == "end" )
            {
                terminate = true;
            }
        }
        if (!terminate)
        {
            client.transmit( input ); // send string
            int qPos = input.find( "?", 0 );
            // receive string only when needed

if ( qPos > 0 )
{
    string rcStr = "";
    client.receive( rcStr );
    cout << rcStr << endl;
}
}
if ( useSingleCommand )
{
    terminate = true;
}
}
}
}
catch( const string errorString )
{
    cout<<errorString<<endl;
}
client.disconnect();
return errorCode;
}

### 5.3 SCPI command structure

SCPI commands consist of a header and, usually, one or more parameters. The headers may consist of several mnemonics (keywords). Queries are formed by appending a question mark directly to the header.

The commands can be either instrument-specific or instrument-independent (common commands). Common and instrument-specific commands differ in their syntax.

#### 5.3.1 Syntax for common commands

Common (= instrument-independent) commands consist of a header preceded by an asterisk (*), and possibly one or more parameters.

<div style="text-align: center;"><div style="text-align: center;">Table 5-2: Examples of common commands</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>*RST</td><td style='text-align: center; word-wrap: break-word;'>RESET</td><td style='text-align: center; word-wrap: break-word;'>Resets the instrument.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*ESE</td><td style='text-align: center; word-wrap: break-word;'>EVENT STATUS ENABLE</td><td style='text-align: center; word-wrap: break-word;'>Sets the bits of the event status enable registers.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*ESR?</td><td style='text-align: center; word-wrap: break-word;'>EVENT STATUS QUERY</td><td style='text-align: center; word-wrap: break-word;'>Queries the contents of the event status register.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*IDN?</td><td style='text-align: center; word-wrap: break-word;'>IDENTIFICATION QUERY</td><td style='text-align: center; word-wrap: break-word;'>Queries the instrument identification string.</td></tr></table>

#### 5.3.2 Syntax for instrument-specific commands

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_3/imgs/img_in_image_box_218_282_272_337.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A41Z%2F-1%2F%2F5a33e6a9ad15beeef3e69092936a645fa36497c14b271c127160dbd2c3aa8714" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_3/imgs/img_in_image_box_218_282_272_337.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A41Z%2F-1%2F%2F5a33e6a9ad15beeef3e69092936a645fa36497c14b271c127160dbd2c3aa8714" alt="Image" width="4%" />

i

</div>


</div>


Not all commands used in the following examples are necessarily implemented in the instrument. For demonstration purposes only, assume the existence of the following commands for this section:

● DISPlay[:WINDOW<1...4>]:MAXimize <Boolean>

• FORMAT:READings:DATA <type>[,<length>]

• HCOPy:DEVICE:COLOR <Boolean>

HCOPy:DEVICE:CMAP:COLOR:RGB <red>,<green>,<blue>

• HCOPy[:IMMediate]

• HCOPY: ITEM: ALL

• HCOPy:ITEM:LABEL <string>

HCOPY: PAGE: DIMensions: QUADrant [<N>]

• HCOPY:PAGE:ORIentation LANDscape | PORTrait

• HCOPY:PAGE:SCALE <numeric value>

MMEMory:COPY <file_source>,<file_destination>

SENSE:BANDwidth|BWIDth[:RESolution] <numeric_value>

● SENSE:FREQUENCY:STOP <numeric value>

SENSE:LIST:FREQUENCY <numeric_value>{,<numeric_value>}

Long and short form.....264

• Numeric suffixes.....264

● Optional mnemonics.....265

##### 5.3.2.1 Long and short form

The mnemonics feature a long form and a short form. The short form is marked by upper case letters, the long form corresponds to the complete word. Either the short form or the long form can be entered; other abbreviations are not permitted.

##### Example:

HCOPY: DEVICE: COLOR ON is equivalent to HCOP: DEV: COL ON.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_3/imgs/img_in_image_box_218_1209_272_1263.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A41Z%2F-1%2F%2F3b965c8bae3ab7efa6044878aa8b916289936c431d6e2fbaeaaca4ec8b26acb5" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_3/imgs/img_in_image_box_218_1209_272_1263.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A41Z%2F-1%2F%2F3b965c8bae3ab7efa6044878aa8b916289936c431d6e2fbaeaaca4ec8b26acb5" alt="Image" width="4%" />

i

</div>


</div>


##### Case-insensitivity

Upper case and lower case notation only serves to distinguish the two forms in the manual, the instrument itself is case-insensitive.

##### 5.3.2.2 Numeric suffixes

If a command can be applied to multiple instances of an object, e.g. specific channels or sources, the required instances can be specified by a suffix added to the command. Numeric suffixes are indicated by angular brackets (<1...4>, <n>, <i>) and are replaced

by a single value in the command. Entries without a suffix are interpreted as having the suffix 1.

##### Example:

Definition: HCOPY: PAGE: DIMensions: QUadrant [<N>]

Command: HCOP: PAGE:DIM:QUAD2

This command refers to the quadrant 2.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_4/imgs/img_in_image_box_218_431_272_486.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A42Z%2F-1%2F%2Fb16e9b6ac0b9121475091eb327bc4d2e8c61440d53ceceabd5258d4ba75eb134" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_4/imgs/img_in_image_box_218_431_272_486.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A42Z%2F-1%2F%2Fb16e9b6ac0b9121475091eb327bc4d2e8c61440d53ceceabd5258d4ba75eb134" alt="Image" width="4%" />

i

</div>


</div>


##### Different numbering in remote control

For remote control, the suffix may differ from the number of the corresponding selection used in manual operation. SCPI prescribes that suffix counting starts with 1. Suffix 1 is the default state and used when no specific suffix is specified.

Some standards define a fixed numbering, starting with 0. If the numbering differs in manual operation and remote control, it is indicated for the corresponding command.

##### 5.3.2.3 Optional mnemonics

Some command systems permit certain mnemonics to be inserted into the header or omitted. These mnemonics are marked by square brackets in the description. The instrument must recognize the long command to comply with the SCPI standard. Some commands are considerably shortened by these optional mnemonics.

##### Example:

Definition: HCOPy[: IMMediate]

Command: HCOP : IMM is equivalent to HCOP

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_4/imgs/img_in_image_box_217_950_272_1004.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A43Z%2F-1%2F%2F10f417c78dd4ade24645aaf2e77e68e0c1cd3e459a2bcea8d5abdb8b87918947" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//49f94677-6c8a-4565-8ed5-11f1547ec490/markdown_4/imgs/img_in_image_box_217_950_272_1004.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A43Z%2F-1%2F%2F10f417c78dd4ade24645aaf2e77e68e0c1cd3e459a2bcea8d5abdb8b87918947" alt="Image" width="4%" />

i

</div>


</div>


##### Optional mnemonics with numeric suffixes

Do not omit an optional mnemonic if it includes a numeric suffix that is relevant for the effect of the command.

##### Example:

Definition: DISPlay[:WINDOW<1...4>] : MAXimize <Boolean>

Command: DISP:MAX ON refers to window 1.

To refer to a window other than 1, you must include the optional WINDOW parameter with the suffix for the required window.

DISP:WIND2:MAX ON refers to window 2.

#### 5.3.3 SCPI parameters

Many commands are supplemented by a parameter or a list of parameters. The parameters must be separated from the header by a "white space" (ASCII code 0 to 9, 11 to 32 decimal, e.g. blank).

The parameters required for each command and the allowed range of values are specified in the command description.

##### Allowed parameters are:

• Numeric values.....266    
• Special numeric values.....266    
• Boolean parameters.....267    
• Text parameters.....267    
• Character strings.....268    
• Block data.....268

##### 5.3.3.1 Numeric values

Numeric values can be entered in any form, i.e. with sign, decimal point and exponent. Values exceeding the resolution of the instrument are rounded up or down. The mantissa may comprise up to 255 characters, the exponent must lie inside the value range -32000 to 32000. The exponent is introduced by an "E" or "e". Entry of the exponent alone is not allowed.

##### Example:

SENS:FREQ:STOP 1500000 = SENS:FREQ:STOP 1.5E6

##### Units

For physical quantities, the unit can be entered. If the unit is missing, the basic unit is used. Allowed unit prefixes are:

• G (giga)

• MA (mega), MOHM, MHZ

K (kilo)

• M (milli)

• U (micro)

• N (nano)

##### Example:

SENSE:FREQ:STOP 1.5GHz = SENSE:FREQ:STOP 1.5E9

Some settings allow relative values to be stated in percent. According to SCPI, this unit is represented by the PCT string.

##### Example:

HCOP: PAGE: SCAL 90PCT

##### 5.3.3.2 Special numeric values

The following mnemonics are special numeric values. In the response to a query, the numeric value is provided.

• MIN and MAX: denote the minimum (MINimum) and maximum (MAXimum) value.

- DEF: denotes a preset value (Default) which has been stored in the EPROM. This value conforms to the default setting, as it is called by the *RST command.

- UP and DOWN: increases or reduces the numeric value by one step. The step width can be specified via an allocated step command for each parameter which can be set via UP and DOWN.

- INF and NINF: INFinity and Negative INFinity (NINF) represent the numeric values 9.9E37 or -9.9E37, respectively. INF and NINF are only sent as instrument responses.

• NAN: Not A Number (NAN) represents the value 9.91E37. NAN is only sent as an instrument response. This value is not defined. Possible causes are the division of zero by zero, the subtraction of infinite from infinite and the representation of missing values.

##### Example:

Setting command: SENSE:LIST:FREQ MAXimum

Query: SENS:LIST:FREQ?

Response: 3.5E9

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//e2fe7d0a-40b5-4c79-9710-df5e35f8275b/markdown_1/imgs/img_in_image_box_218_650_272_705.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A06Z%2F-1%2F%2F260f9bafd4c1acfbe6ca149b43156c35b98db9ae7ee167277d9a292c500b0b8f" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//e2fe7d0a-40b5-4c79-9710-df5e35f8275b/markdown_1/imgs/img_in_image_box_218_650_272_705.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A06Z%2F-1%2F%2F260f9bafd4c1acfbe6ca149b43156c35b98db9ae7ee167277d9a292c500b0b8f" alt="Image" width="4%" />

i

</div>


</div>


##### Queries for special numeric values

The numeric values associated to MAXimum/MINimum/Default can be queried by adding the corresponding mnemonic after the question mark.

Example: SENSE:LIST:FREQ? MAXimum

Returns the maximum numeric value as a result.

##### 5.3.3.3 Boolean parameters

Boolean parameters represent two states. The "ON" state (logically true) is represented by "ON" or a numeric value 1. The "OFF" state (logically untrue) is represented by "OFF" or the numeric value 0. The numeric values are provided as the response for a query.

##### Example:

Setting command: HCOPY:DEV:COL ON

Query: HCOPY: DEV: COL?

Response: 1

##### 5.3.3.4 Text parameters

Text parameters observe the syntactic rules for mnemonics, i.e. they can be entered using a short or long form. Like any parameter, they have to be separated from the header by a white space. In the response to a query, the short form of the text is provided.

##### Example:

Setting command: HCOPY:PAGE:ORIentation LANDscape

Query: HCOP : PAGE : ORI?

Response: LAND

##### 5.3.3.5 Character strings

Always enter strings in quotation marks (' or ").

Example:
HCOP:ITEM:LABEL "Test1"
HCOP:ITEM:LABEL 'Test1'

##### 5.3.3.6 Block data

Block data is a format which is suitable for the transmission of large amounts of data. For example, a command using a block data parameter has the following structure:

FORMAT:READings:DATA #45168xxxxxxxxx

The ASCII character # introduces the data block. The next number indicates how many of the following digits describe the length of the data block. In the example, the 4 following digits indicate the length to be 5168 bytes. The data bytes follow. During the transmission of these data bytes all end or other control signs are ignored until all bytes are transmitted.

#0 specifies a data block of indefinite length. The use of the indefinite format requires a NL^END message to terminate the data block. This format is useful when the length of the transmission is not known or if speed or other considerations prevent segmentation of the data into blocks of definite length.

#### 5.3.4 Overview of syntax elements

The following tables provide an overview of the syntax elements and special characters.

<div style="text-align: center;"><div style="text-align: center;">Table 5-3: Syntax elements</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>:</td><td style='text-align: center; word-wrap: break-word;'>The colon separates the mnemonics of a command.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>;</td><td style='text-align: center; word-wrap: break-word;'>The semicolon separates two commands of a command line. It does not alter the path.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>,</td><td style='text-align: center; word-wrap: break-word;'>The comma separates several parameters of a command.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>?</td><td style='text-align: center; word-wrap: break-word;'>The question mark forms a query.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*</td><td style='text-align: center; word-wrap: break-word;'>The asterisk marks a common command.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>* &quot; &quot;</td><td style='text-align: center; word-wrap: break-word;'>Quotation marks introduce a string and terminate it (both single and double quotation marks are possible).</td></tr></table>

The hash symbol introduces the following numeral systems:
• Binary: #B10110
• Octal: #O7612
• Hexadecimal: #HF3A7
• Block data: #21312
A "white space" (ASCII-Code 0 to 9, 11 to 32 decimal, e.g. blank) separates the header from the parameters.

##### Table 5-4: Special characters

Parameters
A pipe in parameter definitions indicates alternative possibilities in the sense of "or". The effect of the command differs, depending on which parameter is used.
Example:
Definition:HCOPy:PAGE:ORIentation LANDscape | PORTrait
Command HCOP:PAGE:ORI LAND specifies landscape orientation
Command HCOP:PAGE:ORI PORT specifies portrait orientation
Mnemonics
A selection of mnemonics with an identical effect exists for several commands. These mnemonics are indicated in the same line; they are separated by a pipe. Only one of these mnemonics needs to be included in the header of the command. The effect of the command is independent of which of the mnemonics is used.
Example:
DefinitionSENSE:BANDwidth|BWIDth[:RESolution] <numeric_value>
The two following commands with identical meaning can be created:
SENS:BAND:RES 1
SENS:BWID:RES 1
[1] Mnemonics in square brackets are optional and may be inserted into the header or omitted.
Example: HCOPy[:IMMediate]
HCOP:IMM is equivalent to HCOP
{} Parameters in curly brackets are optional and can be inserted once or several times, or omitted.
Example: SENSE:LIST:FREQUency <numeric_value>{,<numeric_value>}
The following are valid commands:
SENS:LIST:FREQ 10
SENS:LIST:FREQ 10,20
SENS:LIST:FREQ 10,20,30,40

#### 5.3.5 Structure of a command line

A command line may consist of one or several commands. It is terminated by one of the following:

<New Line>

<New Line> with EOI

• EOI together with the last data byte

Several commands in a command line must be separated by a semicolon "."

##### Example:

MMEM:COPY "Test1", "MeasurementXY"; :HCOP:ITEM ALL

This command line contains two commands. The first command belongs to the MMEM system, the second command belongs to the HCOP system. If the next command belongs to a different command system, the semicolon is followed by a colon.

##### Example:

HCOP:ITEM ALL; :HCOP:IMM

This command line contains two commands. Both commands are part of the HCOP command system, i.e. they have one level in common.

If the successive commands belong to the same system, having one or several levels in common, the command line can be abbreviated. When abbreviating the command line, the second command begins with the level below HCOP. The colon after the semicolon is omitted. The abbreviated form of the command line reads as follows:

HCOP: ITEM ALL; IMM

##### Example:

HCOP: ITEM ALL

HCOP: IMM

A new command line always begins with the complete path.

#### 5.3.6 Responses to queries

A query is defined for each setting command unless explicitly specified otherwise. It is formed by adding a question mark to the associated setting command. According to SCPI, the responses to queries are partly subject to stricter rules than in standard IEEE 488.2.

- The requested parameter is transmitted without a header.

Example: HCOP:PAGE:ORI?

Response: LAND

- Maximum values, minimum values and all other quantities that are requested via a special text parameter are returned as numeric values.

Example: SENSE:FREQUENCY:STOP? MAX

Response: 3.5E9

- Numeric values are output without a unit. Physical quantities are referred to the basic units or to the units set using the Unit command. The response 3.5E9 in the previous example stands for 3.5 GHz.

• Truth values (Boolean values) are returned as 0 (for OFF) and 1 (for ON).

Example:

Setting command: HCOPY:DEV:COL ON

Query: HCOPY: DEV: COL?

Response: 1

- Text (character data) is returned in a short form. Example:

Setting command: HCOPY:PAGE:ORIentation LANDscape

Query: HCOP: PAGE: ORI?

Response: LAND

• Invalid numerical results

Sometimes, particularly when a result consists of multiple numeric values, invalid values are returned as 9.91E37 (not a number).

### 5.4 Command Sequence and Synchronization

IEEE 488.2 defines a distinction between overlapped and sequential commands:

- A sequential command is one which finishes executing before the next command starts executing. Commands that are processed quickly are implemented as sequential commands. Sequential commands are not implemented in the instrument, however the execution time of most commands is so short that they act as sequential commands when sent in different command lines.

- An overlapping command is one which does not automatically finish executing before the next command starts executing. Usually, overlapping commands take longer to process and allow the program to do other tasks while being executed. If overlapping commands do have to be executed in a defined order, e.g. in order to avoid wrong measurement results, they must be serviced sequentially. Keeping the order is called synchronization between the controller and the instrument.

Setting commands within one command line, even though they can be implemented as sequential commands, are not necessarily serviced in the order in which they have been received. To make sure that commands are actually executed in a certain order, each command must be sent in a separate command line.

##### Example: Commands and queries in one message

The response to a query combined in a program message with commands that affect the queried value is not predictable.

The following commands always return the specified result:

:FREQ:STAR 1GHz;SPAN 100 :FREQ:STAR?

Result:

1000000000 (1 GHz)

Whereas the result for the following commands is not specified by SCPI:

:FREQ:STAR 1GHz;STAR?;SPAN 1000000

The result could be the value of STARt before the command was sent since the instrument can defer executing the individual commands until a program message terminator is received. The result could also be 1 GHz if the instrument executes commands as they are received.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_0/imgs/img_in_image_box_217_1374_272_1428.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A19Z%2F-1%2F%2Fdb63f918f46403afa520f3f6c63aa1fc37a6162f97852ca2cc4de680879ee3fd" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_0/imgs/img_in_image_box_217_1374_272_1428.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A19Z%2F-1%2F%2Fdb63f918f46403afa520f3f6c63aa1fc37a6162f97852ca2cc4de680879ee3fd" alt="Image" width="4%" />

i

</div>


</div>


As a general rule, send commands and queries in different program messages.

##### Example: Overlapping command with *OPC

The instrument implements INITiate[:IMMediate] as an overlapped command. Assuming that INITiate[:IMMediate] takes longer to execute than *OPC, sending the following command sequence results in initiating a sweep and, after some time, setting the OPC bit in the ESR:

INIT; *OPC.

Sending the following commands still initiates a sweep:

INIT; *OPC; *CLS

However, since the operation is still pending when the instrument executes *CLS, forcing it into the "Operation Complete Command Idle" state (OCIS), *OPC is effectively skipped. The OPC bit is not set until the instrument executes another *OPC command.

#### 5.4.1 Preventing overlapping execution

To prevent an overlapping execution of commands, one of the commands *OPC, *OPC? or *WAI can be used. All three commands cause a certain action only to be carried out after the hardware has been set. The controller can be forced to wait for the corresponding action to occur.

<div style="text-align: center;"><div style="text-align: center;">Table 5-5: Synchronization using *OPC, *OPC? and *WAI</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Command</td><td style='text-align: center; word-wrap: break-word;'>Action</td><td style='text-align: center; word-wrap: break-word;'>Programming the controller</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*OPC</td><td style='text-align: center; word-wrap: break-word;'>Sets the Operation Complete bit in the Standard Event Status Register (ESR) after all previous commands have been executed.</td><td style='text-align: center; word-wrap: break-word;'>● Setting bit 0 in the ESE● Setting bit 5 in the SRE● Waiting for service request (SRQ)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*OPC?</td><td style='text-align: center; word-wrap: break-word;'>Stops command processing until 1 is returned. This occurs when all pending operations are completed.</td><td style='text-align: center; word-wrap: break-word;'>Send *OPC? directly after the command whose processing must be terminated before other commands can be executed.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>*WAI</td><td style='text-align: center; word-wrap: break-word;'>Stops further command processing until all commands sent before Wait-to-Continue Command (WAI) have been executed.</td><td style='text-align: center; word-wrap: break-word;'>Send *WAI directly after the command whose processing must be terminated before other commands are executed.</td></tr></table>

Command synchronization using *WAI or *OPC? is a good choice if the overlapped command takes only little time to process. The two synchronization commands simply block overlapping execution of the command. Append the synchronization command to the overlapped command, for example:

SINGLE; *OPC?

For time consuming overlapped commands, you can allow the controller or the instrument to do other useful work while waiting for command execution. Use one of the following methods:

##### *OPC with a service request

1. Execute *ESE 1

Sets the OPC mask bit (bit No. 0) of the Standard Event Status Register (ESR) to 1

2. Execute *SRE 32

Sets the Event Status Bit (ESB - bit No. 5) of the Service Request Enable Register (SRE) to 1 to enable ESB service request.

## 3. Send the overlapped command with *OPC

Example: INIT; *OPC

## 4. Wait for an ESB service request.

The service request indicates that the overlapped command has finished.

##### *OPC? with a service request

## 1. Execute *SRE 16

Sets the Message Available bit (MAV - bit No. 4) of the Service Request Enable Register (SRE) to 1 to enable MAV service request.

## 2. Send the overlapped command with *OPC?

Example: INIT; *OPC?

## 3. Wait for an MAV service request.

The service request indicates that the overlapped command has finished.

##### Event status enable register (ESE)

## 1. Execute *ESE 1

Sets the OPC mask bit (bit No. 0) of the Standard Event Status Register (ESR) to 1

2. Send the overlapped command without *OPC, *OPC? or *WAI.

Example: INIT; *OPC?

3. Poll the operation complete state periodically (with a timer) using the sequence:

*OPC; *ESR?

A return value (LSB) of 1 indicates that the overlapped command has finished.

### 5.5 Status reporting system

The status reporting system stores all information on the current operating state of the instrument, and on errors which have occurred. This information is stored in the status registers and in the error queue.

You can query both with the commands of the STATus subsystem.

#### 5.5.1 Hierarchy of the status registers

The Figure 5-1 shows the hierarchical structure of information in the status registers (ascending from left to right).

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_3/imgs/img_in_image_box_292_201_1067_1098.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Faf419a87a2ed52d9bd3e681a91d9fa6bcf604af2fa77f1849eba285dcd4d855e" alt="Image" width="65%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_3/imgs/img_in_image_box_292_201_1067_1098.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Faf419a87a2ed52d9bd3e681a91d9fa6bcf604af2fa77f1849eba285dcd4d855e" alt="Image" width="65%" />

Questionable
Status

Output-Queue

0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 → 0 →

</div>


</div>


<div style="text-align: center;"><div style="text-align: center;">Figure 5-1: Graphical overview of the status registers hierarchy</div> </div>


OPER = Operation Status Summary Bit

RQS/MSS = Service Request Generation

ESB = Standard Event Status Summary Bit

MAV = Message Available in Output Queue

QUES = Questionable Status Summary Bit

2 = Error- /Event-Queue

1, 0 = not used

Note: This legend explains the abbreviations to the Status Byte Register.

The R&S SMB uses the following status registers:

- Status Byte (STB) and Service Request Enable (SRE), see Chapter 5.5.3, "Status byte (STB) and service request enable register (SRE)", on page 277.

- Standard Event Status, i.e. the Event status Register (ESR) and the Event Status Enable (ESE), see Chapter 5.5.4, "Event status register (ESR) and event status enable register (ESE)", on page 278.

- Questionable Status and Operation Status, the (SCPI status registers, see Chapter 5.5.2, "Structure of a SCPI status register", on page 275, Chapter 5.5.5, "Questionable status register (STATUS:QUESTionable)", on page 278 and Chapter 5.5.6, "Operation status register (STATUS:OPERATION)", on page 279.

##### Error- /Event-Queue

##### • Output-Queue

The output queue contains the messages the instrument returns to the controller. It is not part of the status reporting system but determines the value of the MAV bit in the STB and thus is represented in the overview.

The error-/event-queue contains all errors and events that have occurred in the past. When reading the queue, the instrument starts with the first occurred error/event.

All status registers have the same internal structure.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_4/imgs/img_in_image_box_218_674_272_728.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Fc3e98e59f779d774610f98035449dedc7136d7dc067cb52161adb65cc28b2c2d" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_4/imgs/img_in_image_box_218_674_272_728.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Fc3e98e59f779d774610f98035449dedc7136d7dc067cb52161adb65cc28b2c2d" alt="Image" width="4%" />

i

</div>


</div>


##### SRE, ESE

The service request enable register SRE can be used as ENABLE part of the STB if the STB is structured according to SCPI. By analogy, the ESE can be used as the ENABLE part of the ESR.

#### 5.5.2 Structure of a SCPI status register

Each SCPI status register consists of five parts. Each part has a width of 16 bits and has different functions. The individual bits are independent of each other, i.e. each hardware status is assigned a bit number, which is valid for all five parts. Bit 15 (the most significant bit) is set to zero for all parts. Thus, the contents of the register parts can be processed by the controller as positive integers.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_4/imgs/img_in_image_box_289_1051_1064_1415.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Ffb534ca065b022a37477ff324880c5a82dfac6724c202312b09177ad34c394bd" alt="Image" width="65%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//12975e55-58e0-4079-ae4c-c03c1a2fbff0/markdown_4/imgs/img_in_image_box_289_1051_1064_1415.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A21Z%2F-1%2F%2Ffb534ca065b022a37477ff324880c5a82dfac6724c202312b09177ad34c394bd" alt="Image" width="65%" />

Simplified diagram

15 14 13 12

CONDITION part 3 2 1 0

15 14 13 12

PTRansition part 3 2 1 0

15 14 13 12

NTRansition part 3 2 1 0

EVENT part 3 2 1 0

Logic OR of all bits

Logical AND of EVENT and ENABLE bits

Summary bit of SCP1 register, written into a bit of the STB or into the CONDITION bit of a superordinate register.

15 14 13 12

ENABLE part 3 2 1 0

</div>


</div>


<div style="text-align: center;"><div style="text-align: center;">Figure 5-2: The status-register model</div> </div>


##### Description of the five status register parts

The five parts of a SCPI status register have different properties and functions:

##### • CONDITION

The CONDITION part is written directly by the hardware or it mirrors the sum bit of the next lower register. Its contents reflect the current instrument status. This register part can only be read, but not written into or cleared. Its contents are not affected by reading.

##### • PTRansition / NTRansition

The two transition register parts define which state transition of the CONDITION part (none, 0 to 1, 1 to 0 or both) is stored in the EVENT part.

The Positive-Transition part acts as a transition filter. When a bit of the CONDITION part is changed from 0 to 1, the associated PTR bit decides whether the EVENT bit is set to 1.

– PTR bit =1: the EVENT bit is set.

– PTR bit =0: the EVENT bit is not set.

This part can be written into and read as required. Its contents are not affected by reading.

The Negative-TRansition part also acts as a transition filter. When a bit of the CONDITION part is changed from 1 to 0, the associated NTR bit decides whether the EVENT bit is set to 1.

– NTR bit =1: the EVENT bit is set.

– NTR bit =0: the EVENT bit is not set.

This part can be written into and read as required. Its contents are not affected by reading.

##### • EVENT

The EVENT part indicates whether an event has occurred since the last reading, it is the "memory" of the condition part. It only indicates events passed on by the transition filters. It is permanently updated by the instrument. This part can only be read by the user. Reading the register clears it. This part is often equated with the entire register.

##### ☑ ENABLE

The ENABLE part determines whether the associated EVENT bit contributes to the sum bit (see below). Each bit of the EVENT part is "ANDED" with the associated ENABLE bit (symbol '&\'). The results of all logical operations of this part are passed on to the sum bit via an "OR" function (symbol '+').

ENABLE bit = 0: the associated EVENT bit does not contribute to the sum bit

ENABLE bit = 1: if the associated EVENT bit is "1", the sum bit is set to "1" as well. This part can be written into and read by the user as required. Its contents are not affected by reading.

##### Sum bit

The sum bit is obtained from the EVENT and ENABLE part for each register. The result is then entered into a bit of the CONDITION part of the higher-order register.

The instrument automatically generates the sum bit for each register. Thus an event can lead to a service request throughout all levels of the hierarchy.

#### 5.5.3 Status byte (STB) and service request enable register (SRE)

The STatus Byte (STB) is already defined in IEEE 488.2. It provides a rough overview of the instrument status by collecting the pieces of information of the lower registers. A special feature is that bit 6 acts as the sum bit of the remaining bits of the status byte.

The STB is read using the command *STB? or a serial poll.

The STatus Byte (STB) is linked to the Service Request Enable (SRE) register. Each bit of the STB is assigned a bit in the SRE. Bit 6 of the SRE is ignored. If a bit is set in the SRE and the associated bit in the STB changes from 0 to 1, a service request (SRQ) is generated. The SRE can be set using the command *SRE and read using the command *SRE?.

<div style="text-align: center;"><div style="text-align: center;">Table 5-6: Meaning of the bits used in the status byte</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Bit No.</td><td style='text-align: center; word-wrap: break-word;'>Meaning</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>0...1</td><td style='text-align: center; word-wrap: break-word;'>Not used</td></tr><tr><td rowspan="2">2</td><td style='text-align: center; word-wrap: break-word;'>Error Queue not empty</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>The bit is set when an entry is made in the error queue. If this bit is enabled by the SRE, each entry of the error queue generates a service request. Thus an error can be recognized and specified in greater detail by polling the error queue. The poll provides an informative error message. This procedure is to be recommended since it considerably reduces the problems involved with remote control.</td></tr><tr><td rowspan="2">3</td><td style='text-align: center; word-wrap: break-word;'>QUESTionable status register summary bit</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>The bit is set if an EVENT bit is set in the QUESTionable status register and the associated ENABLE bit is set to 1. A set bit indicates a questionable instrument status, which can be specified in greater detail by querying the STATUS:QUESTionable status register.</td></tr><tr><td rowspan="2">4</td><td style='text-align: center; word-wrap: break-word;'>MAV bit (message available)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>The bit is set if a message is available in the output queue which can be read. This bit can be used to enable data to be automatically read from the instrument to the controller.</td></tr><tr><td rowspan="2">5</td><td style='text-align: center; word-wrap: break-word;'>ESB bit</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Sum bit of the event status register. It is set if one of the bits in the event status register is set and enabled in the event status enable register. Setting of this bit indicates a serious error which can be specified in greater detail by polling the event status register.</td></tr><tr><td rowspan="2">6</td><td style='text-align: center; word-wrap: break-word;'>MSS bit (main status summary bit)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>The bit is set if the instrument triggers a service request. This is the case if one of the other bits of this register is set together with its mask bit in the service request enable register SRE.</td></tr><tr><td rowspan="2">7</td><td style='text-align: center; word-wrap: break-word;'>STATUS:OPERATION status register summary bit</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>The bit is set if an EVENT bit is set in the OPERation status register and the associated ENABLE bit is set to 1. A set bit indicates that the instrument is just performing an action. The type of action can be determined by querying the STATUS:OPERATION status register.</td></tr></table>

#### 5.5.4 Event status register (ESR) and event status enable register (ESE)

The ESR is defined in IEEE 488.2. It can be compared with the EVENT part of a SCPI register. The event status register can be read out using command *ESR?.

The ESE corresponds to the ENABLE part of a SCPI register. If a bit is set in the ESE and the associated bit in the ESR changes from 0 to 1, the ESB bit in the STB is set. The ESE register can be set using the command *ESE and read using the command *ESE?.

<div style="text-align: center;"><div style="text-align: center;">Table 5-7: Meaning of the bits used in the event status register</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Bit No.</td><td style='text-align: center; word-wrap: break-word;'>Meaning</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>0</td><td style='text-align: center; word-wrap: break-word;'>Operation CompleteThis bit is set on receipt of the command *OPC exactly when all previous commands have been executed.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>1</td><td style='text-align: center; word-wrap: break-word;'>Not used</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>2</td><td style='text-align: center; word-wrap: break-word;'>Query ErrorThis bit is set if either the controller wants to read data from the instrument without having sent a query, or if it does not fetch requested data and sends new instructions to the instrument instead. The cause is often a query which is faulty and hence cannot be executed.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>3</td><td style='text-align: center; word-wrap: break-word;'>Device-dependent ErrorThis bit is set if a device-dependent error occurs. An error message with a number between -300 and -399 or a positive error number, which denotes the error in greater detail, is entered into the error queue.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>4</td><td style='text-align: center; word-wrap: break-word;'>Execution ErrorThis bit is set if a received command is syntactically correct but cannot be performed for other reasons. An error message with a number between -200 and -300, which denotes the error in greater detail, is entered into the error queue.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>5</td><td style='text-align: center; word-wrap: break-word;'>Command ErrorThis bit is set if a command is received, which is undefined or syntactically incorrect. An error message with a number between -100 and -200, which denotes the error in greater detail, is entered into the error queue.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>6</td><td style='text-align: center; word-wrap: break-word;'>User RequestThis bit is set when the instrument is switched over to manual control.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>7</td><td style='text-align: center; word-wrap: break-word;'>Power On (supply voltage on)This bit is set on switching on the instrument.</td></tr></table>

#### 5.5.5 Questionable status register (STATUS:QUESTionable)

This register contains information on questionable instrument states. Such states may occur when the instrument is not operated in compliance with its specifications.

To read the register, use the query commands STAT:QUEST:COND? or STAT:QUEST[:EVEN]?.

<div style="text-align: center;"><div style="text-align: center;">Table 5-8: Meaning of the bits used in the questionable status register</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Bit No.</td><td style='text-align: center; word-wrap: break-word;'>Meaning</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>0-15</td><td style='text-align: center; word-wrap: break-word;'>Not used</td></tr></table>

#### 5.5.6 Operation status register (STATUS: OPERation)

This condition part contains information on the actions currently being performed by the instrument, while the event part contains information on the actions performed by the instrument since the last readout of the register.

To read the register, use the query commands STAT:OPER:COND? or STAT:OPER[:EVEN]?.

<div style="text-align: center;"><div style="text-align: center;">Table 5-9: Meaning of the bits used in the operation status register</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Bit No.</td><td style='text-align: center; word-wrap: break-word;'>Meaning</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>0</td><td style='text-align: center; word-wrap: break-word;'>CalibratingThe bit is set during the calibration phase.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>1-2</td><td style='text-align: center; word-wrap: break-word;'>Not used</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>3</td><td style='text-align: center; word-wrap: break-word;'></td></tr><tr><td style='text-align: center; word-wrap: break-word;'>4-15</td><td style='text-align: center; word-wrap: break-word;'>Not used</td></tr></table>

#### 5.5.7 Application of the status reporting system

The purpose of the status reporting system is to monitor the status of one or several devices in a measuring system. To do this and react appropriately, the controller must receive and evaluate the information of all devices. The following standard methods are used:

• Service request (SRQ) initiated by the instrument

- Serial poll of all devices in the bus system, initiated by the controller to find out who sent an SRQ and why

• Query of a specific instrument status by commands

##### 5.5.7.1 Service request

• Query of the error queue

Under certain circumstances, the instrument can send a service request (SRQ) to the controller. Usually this service request initiates an interrupt at the controller, to which the control program can react appropriately. An SRQ is always initiated if one or several of bits 2, 4 or 5 of the status byte are set and enabled in the SRE. Each of these bits combines the information of the error queue or the output buffer. To use the possibilities of the service request effectively, all bits should be set to "1" in the enable registers SRE and ESE.

##### Example:

Use command *OPC to generate an SRQ.

 $ ^{*} $ESE 1 - set bit 0 of ESE (Operation Complete)

*SRE 32 - set bit 5 of SRE (ESB).

After its settings have been completed, the instrument generates an SRQ.

The SRQ is the only possibility for the instrument to become active on its own. Each controller program should set the instrument such that a service request is initiated in the case of malfunction. The program should react appropriately to the service request.

##### 5.5.7.2 Serial poll

In a serial poll, just as with command *STB, the status byte of an instrument is queried. However, the query is realized via interface messages and is thus clearly faster.

The serial poll method is defined in IEEE 488.1 and used to be the only standard possibility for different instruments to poll the status byte. The method also works for instruments which do not adhere to SCPI or IEEE 488.2.

The serial poll is mainly used to obtain a fast overview of the state of several instruments connected to the controller.

##### 5.5.7.3 Query of an instrument status

Each part of any status register can be read using queries. There are two types of commands:

- The common commands *ESR?, *IDN?, *IST?, *STB? query the higher-level registers.

- The commands of the STATus system query the SCPI registers (STATUS:QUESTionable...)

The returned value is always a decimal number that represents the bit pattern of the queried register. This number is evaluated by the controller program.

Queries are usually used after an SRQ in order to obtain more detailed information on the cause of the SRQ.

##### 5.5.7.4 Error queue

Each error state in the instrument leads to an entry in the error queue. The entries of the error queue are detailed plain text error messages that can be looked up in the Error Log or queried via remote control using SYSTEM:ERRor[:NEXT]?. Each call of SYSTEM:ERRor[:NEXT]? provides one entry from the error queue. If no error messages are stored there any more, the instrument responds with 0, "No error".

The error queue should be queried after every SRQ in the controller program as the entries describe the cause of an error more precisely than the status registers. Especially in the test phase of a controller program the error queue should be queried regu-

larly since faulty commands from the controller to the instrument are recorded there as well.

#### 5.5.8 Reset values of the status reporting system

The following table contains the different commands and events causing the status reporting system to be reset. None of the commands, except of *RST and SYSTEM:PRESet affect the functional instrument settings. In particular, DCL does not change the instrument settings.

<div style="text-align: center;"><div style="text-align: center;">Table 5-10: Resetting the status reporting system</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Event</td><td colspan="2">Switching on supply voltage Power-On-Status-Clear</td><td rowspan="2">DCL, SDC (Device Clear, Selected Device Clear)</td><td rowspan="2">*RST or SYSTEM: PRESET</td><td rowspan="2">STATUS: PRESET</td><td rowspan="2">*CLS</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Effect</td><td style='text-align: center; word-wrap: break-word;'>0</td><td style='text-align: center; word-wrap: break-word;'>1</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear STB, ESR</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>Yes</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear SRE, ESE</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear PPE</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear error queue</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>Yes</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear output buffer</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>1)</td><td style='text-align: center; word-wrap: break-word;'>1)</td><td style='text-align: center; word-wrap: break-word;'>1)</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Clear command processing and input buffer</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>Yes</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td><td style='text-align: center; word-wrap: break-word;'>-</td></tr></table>

### 5.6 General programming recommendations

##### Initial instrument status before changing settings

Manual operation is designed for maximum possible operating convenience. In contrast, the priority of remote control is the "predictability" of the instrument status. Thus, when a command attempts to define incompatible settings, the command is ignored and the instrument status remains unchanged, i.e. other settings are not automatically adapted. Therefore, control programs should always define an initial instrument status (e.g. using the *RST command) and then implement the required settings.

##### Command sequence

As a general rule, send commands and queries in different program messages. Otherwise, the result of the query may vary depending on which operation is performed first (see also Preventing Overlapping Execution).

##### Reacting to malfunctions

The service request is the only possibility for the instrument to become active on its own. Each controller program should instruct the instrument to initiate a service request in case of malfunction. The program should react appropriately to the service request.

##### Error queues

The error queue should be queried after every service request in the controller program as the entries describe the cause of an error more precisely than the status registers. Especially in the test phase of a controller program the error queue should be queried regularly since faulty commands from the controller to the instrument are recorded there as well.

