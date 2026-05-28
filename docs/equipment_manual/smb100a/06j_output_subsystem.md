### 6.11 OUTPUT Subsystem

The OUTPUT system contains the commands which set the properties of the [RF] output connector.

The properties of the LF output connector are set in the SOURce:LFOutput Subsystem system.

:OUTPUT<hw>:AFIXed:RANGE:LOWER?.....314    
:OUTPUT<hw>:AFIXed:RANGE:UPPER?.....314    
:OUTPUT<hw>:ALC:SEARCH:MODE.....315    
:OUTPUT<hw>:AMODE.....315    
:OUTPUT<hw>:FILTer:AUTO.....315    
:OUTPUT<hw>:FILTer[:LPASs]:STATE.....316    
:OUTPUT<hw>:IMPedance?.....316    
:OUTPUT<hw>:PROTection:CLEAR.....316    
:OUTPUT<hw>:PROTection:TRIPped?.....316    
:OUTPUT<hw>[:STATE].....317    
:OUTPUT<hw>[:STATE]:PON.....317

##### :OUTPUT<hw>:AFIXed:RANGE:LOWER?

Queries the minimum level which can be set when the attenuator is fixed, see :

OUTPUT<hw>:AMODE.

##### Return values:

<Lower> float

Increment: 0.01

Example: OUTP:AFIX:RANG:LOW?

queries the minimum level for the FIXed setting.

Example: Response: -50

The minimum level is -50 dBm.

Usage: Query only

Manual operation: See "Fixed Range (PEP) In" on page 151

##### :OUTPUT<hw>:AFIXed:RANGE:UPPER?

Queries the maximum level which can be set when the attenuator is fixed, see :

OUTPUT<hw>:AMODE.

Return values:

<Upper> float

Increment: 0.01

Example: OUTP:AFIX:RANG:UPP? queries the maximum level for the FIXed setting for the RF output.

Example: Response: -27

The maximum level is -27 dBm.

Usage: Query only

Manual operation: See "Fixed Range (PEP) In" on page 151



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Usage:</td><td style='text-align: center; word-wrap: break-word;'>Query only</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Manual operation:</td><td style='text-align: center; word-wrap: break-word;'>See &quot;Fixed Range (PEP) In&quot; on page 151</td></tr><tr><td colspan="2">:OUTPUT&lt;hw&gt;:ALC:SEARCH:MODE &lt;Mode&gt;</td></tr><tr><td colspan="2">Activates/deactivates the RF output during the power search.</td></tr><tr><td colspan="2">Parameters:</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&lt;Mode&gt;</td><td style='text-align: center; word-wrap: break-word;'>NORMAL | MINimum</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>*RST:    NORMAL</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Example:</td><td style='text-align: center; word-wrap: break-word;'>POW:ALC:SEAR:MODE NORM during the power search, the RF output is active.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Manual operation:</td><td style='text-align: center; word-wrap: break-word;'>See &quot;RF During Power Search - ALC&quot; on page 155</td></tr><tr><td colspan="2">:OUTPUT&lt;hw&gt;:AMODE &lt;AMode&gt;</td></tr><tr><td colspan="2">Selects the mode of the attenuator at the RF output (Attenuator MODE).</td></tr><tr><td colspan="2">Parameters:</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>&lt;AMode&gt;</td><td style='text-align: center; word-wrap: break-word;'>AUTO | FIXed AUTO The attenuator is switched automatically. The level settings are made in the full range.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>FIXed The level settings are made without switching the attenuator. When this operating mode is switched on, the attenuator is fixed in its current position and the resulting variation range is defined.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>*RST:    AUTO</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Example:</td><td style='text-align: center; word-wrap: break-word;'>POW:ALC ON activates automatic level control for RF output. OUTP:AMOD FIX sets the fixed mode with uninterrupted level for RF output.</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Manual operation:</td><td style='text-align: center; word-wrap: break-word;'>See &quot;Attenuator Mode&quot; on page 151</td></tr></table>

##### :OUTPUT<hw>:FILTer:AUTO <Auto>

Activates automatic switching of the low harmonic filter.

Parameters:

<Auto> 0 | 1 | ON | OFF

*RST: 0

Example: OUTP:FILT:AUTO 1

Activates the auto mode.

Manual operation: See "Mode" on page 150

:OUTPUT<hw>:FILTER[:LPASs]:STATE <State>

Switches the filter state in manual mode and disables the automatic mode, if activated.

Parameters:

<State> 0 | 1 | OFF | ON

*RST: 0

Example: OUTP:FILT:AUTO 0

OUTP:FILT:LPAS:STAT 1

Selects manual mode and activates the low harmonic filter.

OUTP:FILT:STAT?

Queries the current filter activity.

Manual operation: See "State" on page 151

##### :OUTPUT<hw>:IMPedance?

Queries the impedance of the RF outputs. It enables you to convert the output level units between V and W. The impedances cannot be changed.

##### Return values:

<Impedance> G1K | G50 | G10K

*RST: G50

Default unit: Ohm

Example: OUTP: IMP

queries the impedance of RF output.

Response: 50

the impedance is 50 ohms

Usage: Query only

##### :OUTPUT<hw>:PROTection:CLEAR

Resets the protective circuit after it has been tripped. The state of the output is again determined by OUTPUT: STATE.

Example: OUTP: PROT: CLE

resets the protective circuit for RF output.

Usage: Event

Manual operation: See "Overload" on page 167

#### :OUTPUT<hw>:PROTection:TRIPped?

Queries the state of the protective circuit.

Return values:

<Tripped> 0 | 1 | OFF | ON

*RST: 0

Example:

OUTP:PROT:TRIP

Queries the state of the protective circuit for RF output A.

Response: 0

The protective circuit has not tripped.

Response: 1

The protective circuit has tripped.

Usage:

Query only

Manual operation: See "Overload" on page 167

#### :OUTPUT<hw>[:STATe] <State>

Activates and deactivates the RF output signal (RF ON / RF OFF).

Parameters:

<State> 0 | 1 | OFF | ON

*RST: 0

Example: OUTP OFF

deactivates the RF output.

Manual operation: See "RF On" on page 137

#### :OUTPUT<hw>[:STATe]:PON <Pon>

Selects the state of the RF output when the instrument is switched on.

Parameters:

<Pon> OFF | UNCHanged

OFF

deactivates the output when the instrument is switched on (RF OFF).

UNCHanged

restores the initial state of the RF output before the last turn off.

sets the output status as it was when the instrument was switched off.

*RST: n.a. (factory preset: UNCHanged)

Example:

OUTP: PON OFF

RF output A is deactivated when the instrument is switched on.

Manual operation: See "Power-On State - RF Signal" on page 153

