# Lab Hardware Discovery Report

> **Warning**: This report documents a read-only discovery process.
> No state-changing commands were sent to any device during discovery.
> All probes used a hard-coded safe-command allow-list.

## Discovery Environment

- **Platform**: macOS
- **Date**: 2026-05-28
- **Discovery tool**: `odmr-discover` (Rust CLI) + manual shell verification
- **Operator**: Human-in-the-loop confirmation

---

## Network Interfaces

Command: `ifconfig`

```
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
	ether aa:bb:cc:dd:ee:ff
	inet6 fe80::...%en0 prefixlen 64 secured scopeid 0x4
	inet 192.168.1.100 netmask 0xffffff00 broadcast 192.168.1.255
	media: autoselect
	status: active

en7: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
	ether 00:90:b8:1f:06:dd
	inet 169.254.2.1 netmask 0xffff0000 broadcast 169.254.255.255
	media: autoselect (1000baseT <full-duplex>)
	status: active
```

**Note**: `en7` shows the SMB100A MAC address (`00:90:b8:1f:06:dd`, R&S OUI) in the ARP table. The direct RJ45 link uses APIPA range `169.254.x.x`.

Command: `networksetup -listallhardwareports`

```
Hardware Port: Ethernet
Device: en7
Ethernet Address: aa:bb:cc:dd:ee:ff
```

Command: `arp -a`

```
? (169.254.2.20) at 0:90:b8:1f:6:dd on en7 ifscope [ethernet]
```

**Candidate SMB100A IP**: `169.254.2.20` (confirmed via ARP + R&S MAC OUI).

---

## SMB100A TCP Probe

Target: `169.254.2.20:5025`

### Command sent: `*IDN?`

```
*IDN?
```

### Response received:

```
Rohde&Schwarz,SMB100A,1406.6000k02/101623,3.1.19.15-3.20.390.24
```

**Result**: ✅ Matched `smb100a` — identity confirmed.

### Command sent: `SYST:ERR?`

```
SYST:ERR?
```

### Response received:

```
0,"No error"
```

**Result**: ✅ No system errors.

### Command sent: `OUTP?`

```
OUTP?
```

### Response received:

```
0
```

**Result**: ✅ RF output is OFF.

### Command sent: `MOD:STAT?`

```
MOD:STAT?
```

### Response received:

```
0
```

**Result**: ✅ Modulation is OFF.

**Note**: The discovery tool only sends the four queries above. No `OUTP ON`, `MOD:STAT ON`, `FM:STAT ON`, `FREQ:MODE SWE`, or `SWE:EXEC` commands exist in the tool source code.

---

## Serial Port Discovery

Command: `ls /dev/cu.*`

```
/dev/cu.Bluetooth-Incoming-Port
/dev/cu.usbmodem3361358734371
/dev/cu.usbserial-FTE86EB2
/dev/cu.PL2303G-XXXXXX
/dev/cu.PL2303G-YYYYYY
/dev/cu.PL2303G-ZZZZZZ
```

### Candidate serial ports:

| Port | Driver | Expected Device |
|------|--------|-----------------|
| `/dev/cu.usbmodem3361358734371` | STM32 CDC (0x0483/0x5740) | OE1022D |
| `/dev/cu.usbserial-FTE86EB2` | FTDI FT232 (0x0403/0x6001) | Laser |
| `/dev/cu.PL2303G-XXXXXX` | Prolific PL2303G (0x067B/0x23A3) | Mag X / Y / Z |

---

## OE1022D Serial Probe

Port: `/dev/cu.usbmodem3361358734371`
Baud: `115200 8N1`

### Command sent: `*IDN?`

```
*IDN?
```

### Response received:

```
SSI LIA-OE1022D,SN:D6522078,Version:Ver6.3200831
```

**Result**: ✅ Matched `oe1022d` — identity confirmed.

**USB Serial Number**: `336135873437` (chip-level, stable across re-plugs).

---

## MAYNUO M8812 (Magnetic Axes) Serial Probe

Port: various `/dev/cu.PL2303G-*`
Baud: `9600 8N1, DTR=true`

### Commands sent (per axis):

```
*IDN?
```

### Responses received:

| Axis | Serial Number | Response |
|------|---------------|----------|
| X | `080020960220402020` | `MAYNUO,M8812,080020960220402020,V2.7` |
| Y | `080020960220402022` | `MAYNUO,M8812,080020960220402022,V2.7` |
| Z | `080020960220402003` | `MAYNUO,M8812,080020960220402003,V2.7` |

**Result**: ✅ All three axes identified by SN.

---

## Laser Controller Serial Probe

Port: `/dev/cu.usbserial-FTE86EB2`
Baud: `9600 8N1`

### Command sent: `*IDN?`

```
*IDN?
```

### Response received:

**(no response)**

**Result**: ⚠️ Laser controller (CNI Laser PSU-SR, FT232) does not respond to SCPI `*IDN?`. It uses a custom binary protocol.

**Identification method**: USB Serial Number `FTE86EB2` → direct binding + `manual_verified=true`.

---

## Failures and Timeouts

| Device | Issue | Resolution |
|--------|-------|------------|
| Laser | No SCPI IDN response | Use USB SN binding instead |
| Mag axes | Port paths change on re-enumeration | Use `*IDN?` SN matching, not path |
| SMB100A | APIPA IP not deterministic | Use MAC + mDNS as secondary anchors |

---

## Safe-Command Audit

### SMB100A permitted queries (discovery only)

- `*IDN?`
- `SYST:ERR?`
- `OUTP?`
- `MOD:STAT?`

### OE1022D permitted queries (discovery only)

- `*IDN?`

### Forbidden commands (intentionally absent)

- `OUTP ON`
- `MOD:STAT ON`
- `FM:STAT ON`
- `FREQ:MODE SWE`
- `SWE:EXEC`

These strings do not appear in any discovery tool source file.

---

## Summary

| Device | Kind | Transport | Identified By | Status |
|--------|------|-----------|---------------|--------|
| SMB100A | `smb100a` | TCP 169.254.2.20:5025 | `*IDN?` + MAC | ✅ |
| OE1022D | `oe1022d` | USB CDC /dev/cu.usbmodem3361358734371 | `*IDN?` + USB SN | ✅ |
| Mag X | `magnet_xyz` | PL2303 serial | `*IDN?` SN | ✅ |
| Mag Y | `magnet_xyz` | PL2303 serial | `*IDN?` SN | ✅ |
| Mag Z | `magnet_xyz` | PL2303 serial | `*IDN?` SN | ✅ |
| Laser | `laser` | FT232 serial | USB SN only | ⚠️ manual verify |
