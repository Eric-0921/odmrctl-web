## 8 Maintenance, Storage and Disposal

The product does not require regular maintenance. It only requires occasional cleaning. It is however advisable to check the nominal data from time to time.

### 8.1 Cleaning

How to clean the product is described in "Cleaning the product" on page 15.

Do not use any liquids for cleaning. Cleaning agents, solvents (thinners, acetone), acids and bases can damage the front panel labeling, plastic parts and display.

### 8.2 Changing Fuses

If the product does not start, it is possible that a blown fuse is the cause. The product is protected by 2 fuses of type IEC60127-T3.15H/250V (order no. 0099.6729.00). The fuses are next to the power supply socket at the rear panel.

##### Replacing the line fuses

1. Check the available supply voltage.

The mains voltage must be within the voltage range as denoted on the instrument. The label is below the power supply socket at the rear panel. There is no need to set the voltage manually.

2. If the power outlet exceeds the permissible range, contact Rohde & Schwarz customer service.

3. WARNING! The fuse is part of the main power supply. Handling the fuse while the power is on can lead to electric shock.

Before changing the fuse:

a) Set the switch on the power supply to position [0].

b) Disconnect the product from the power source.

c) Unplug the power cable.

4. To replace the line fuse.

a) Turn left the plastic cover of the fuse holder using a screwdriver to loosen the cover. The slot of the cover must be in vertical position.

b) Remove the cover from the fuse holder.

c) Pull out the fuse holder.

5. Check the condition of the fuse.

6. Replace the blown fuse. Only use a fuse of the specified type.

The fuse type and its characteristics are indicated below the fuse holder.

7. Insert the fuse holder into the mains power inlet.

8. Replace the cover and tighten it.

### 8.3 Storage

Protect the product against dust. Ensure that the environmental conditions, e.g. temperature range and climatic load, meet the values specified in the data sheet.

### 8.4 Performing Maintenance Tasks

Integrated procedures and additional capabilities make sure, that the R&S SMB works correct with high accuracy.

• Date and Time.....491    
• Check Front Panel.....492    
• Internal Adjustments.....494    
• Selftest.....497

#### 8.4.1 Date and Time

The R&S SMB uses an internal real time clock to determine the date and time. It adjusts the time and date to the timezone of your location automatically, by providing a selection list of continents and cities.

The instrument records the time whenever you create or modify files on your instrument or you use timed licences. By default, the instrument is set to the UTC timezone, but you can select the timezone according to your location.

Moreover, the instrument supports NTP protocol for synchronizing all connected instruments and computer systems to minimize time delays in the network.

Access:

▶ Select "Setup > Environment > Date/Time" via the [SETUP] or [MENU] key.



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Date / Time</td><td style='text-align: center; word-wrap: break-word;'>X</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Date [DD.MM.YYYY]</td><td style='text-align: center; word-wrap: break-word;'>24.09.2018</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Time [hh:mm:ss]</td><td style='text-align: center; word-wrap: break-word;'>15:20:49</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Time Zone</td><td style='text-align: center; word-wrap: break-word;'>UTC</td></tr><tr><td colspan="2">NTP Settings</td></tr><tr><td colspan="2">NTP Address</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Use Time From NTP Server</td><td style='text-align: center; word-wrap: break-word;'>On</td></tr></table>

The "Date / Time" dialog contains the time and data settings of the operating system.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//97045504-395b-438f-bb07-8aaba2295359/markdown_1/imgs/img_in_image_box_218_205_272_259.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A50Z%2F-1%2F%2F3fb4d69add3490283620ba32237ef53a80822de5fe338659115ec4bf7f5e2cab" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//97045504-395b-438f-bb07-8aaba2295359/markdown_1/imgs/img_in_image_box_218_205_272_259.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A50Z%2F-1%2F%2F3fb4d69add3490283620ba32237ef53a80822de5fe338659115ec4bf7f5e2cab" alt="Image" width="4%" />

i

</div>


</div>


This function is password-protected. Unlock the protection level 1 to access it. To enable editing, unlock protection level 1, see Chapter 4.2.3.13, "Protection", on page 113.

##### Date

Displays the date set in the operating system in the format [dd.mm.yyyy].

Remote command:

: SYSTEM: DATE on page 454

##### Time

Displays the time set in the operating system in the format [hh.mm.ss].

The time setting corresponds to the selected Time Zone.

##### Remote command:

: SYSTEM: TIME on page 454

##### Time Zone

Selects the time zone.

You can select the time zone according to the major cities on the respective continents.

Note: By typing the first letter, you can quickly navigate through the lists to find the desired destination.

##### Remote command:

: SYSTEM: TIME: ZONE on page 455

:SYSTEM:TIME:ZONE:CATalog? on page 455

##### NTP Address

Sets the IP address or host name of the NTP server.

NTP is a network time protocol used for synchronizing all participating devices in a data network.

You can select a high-precision time server to reduce the impact of varying network delays.

##### Remote command:

:SYSTEM:NTP:HOSTname on page 451

##### Use Time from NTP Server

Activates clock synchronization of the network via the NTP protocol.

: SYSTEM: NTP: STATE on page 451

Remote command:

#### 8.4.2 Check Front Panel

With the functions provided in this dialog you can verify the functionality of the control keys of the R&S SMB.

In case of malfunctions, contact your Rohde & Schwarz Customer Support Center for technical support, see www.rohde-schwarz.com/support.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//97045504-395b-438f-bb07-8aaba2295359/markdown_2/imgs/img_in_image_box_218_205_272_258.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A51Z%2F-1%2F%2F89fd178fc54330096e9a14223c9c74f548b4dcb5807081768df7fc2e9a6a5484" alt="Image" width="4%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//97045504-395b-438f-bb07-8aaba2295359/markdown_2/imgs/img_in_image_box_218_205_272_258.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A51Z%2F-1%2F%2F89fd178fc54330096e9a14223c9c74f548b4dcb5807081768df7fc2e9a6a5484" alt="Image" width="4%" />

i

</div>


</div>


Accessing the online help in the check front panel dialog or exiting via ESC

During the test, the actual functions of all keys are disabled, including the [help] and the [esc] keys.

##### 8.4.2.1 Check Front Panel Settings

▶ To access this dialog, Press the "setup" key and select "Setup > Test > Check Front Panel".



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Preset</td><td style='text-align: center; word-wrap: break-word;'>Roll I.</td><td colspan="2">Roll r.</td><td rowspan="5" colspan="2"></td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Help</td><td colspan="3">Roll click</td><td style='text-align: center; word-wrap: break-word;'>ESC</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td colspan="2">Up</td><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>Diagram</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>Left</td><td colspan="2">Right</td><td style='text-align: center; word-wrap: break-word;'>Mod</td></tr><tr><td style='text-align: center; word-wrap: break-word;'></td><td colspan="2">Down</td><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>RF</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Local</td><td style='text-align: center; word-wrap: break-word;'>Freq</td><td colspan="2"></td><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'></td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Setup</td><td style='text-align: center; word-wrap: break-word;'>Level</td><td style='text-align: center; word-wrap: break-word;'>7</td><td style='text-align: center; word-wrap: break-word;'>8</td><td style='text-align: center; word-wrap: break-word;'>9</td><td style='text-align: center; word-wrap: break-word;'>Gin</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>File</td><td style='text-align: center; word-wrap: break-word;'>Toggle</td><td style='text-align: center; word-wrap: break-word;'>4</td><td style='text-align: center; word-wrap: break-word;'>5</td><td style='text-align: center; word-wrap: break-word;'>6</td><td style='text-align: center; word-wrap: break-word;'>Mju</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Info</td><td style='text-align: center; word-wrap: break-word;'>Back</td><td style='text-align: center; word-wrap: break-word;'>1</td><td style='text-align: center; word-wrap: break-word;'>2</td><td style='text-align: center; word-wrap: break-word;'>3</td><td style='text-align: center; word-wrap: break-word;'>k/m</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Winbar</td><td style='text-align: center; word-wrap: break-word;'></td><td style='text-align: center; word-wrap: break-word;'>0</td><td style='text-align: center; word-wrap: break-word;'>.</td><td style='text-align: center; word-wrap: break-word;'>+/-</td><td style='text-align: center; word-wrap: break-word;'>$ ^{*} $1</td></tr></table>

Reflecting the front panel, the "Check Front Panel" dialog contains all functions to test the operating elements of the instrument.

##### 8.4.2.2 Performing the Front Panel Tests

To perform the front panel test, you operate the keys at the front panel, and check the response of the instrument in the "Check Front Panel" dialog. To perform this test properly, it is essential that you check each key of the front panel. The test is only completed, when you have veryfied all keys.

During the test, the actual functions of the keys are disabled.

Proceed as follows:

1. Press the [setup] key.

2. Select "Test > Check Front Panel"

The "Check Front Panel" dialog opens.

3. Press a key on the front panel.

Check if the corresponding key in the "Check Front Panel" dialog turns green.

4. Press the same key a second time.

Check that the key in the dialog turns red.

Note: Pressing the same key again has no further effect, with the exception of the [esc] key, see Press the ESC key a third time.

5. Continue with the next key on the front panel and repeat step 3 to step 5 until all keys are tested.

Test Passed
All front panel keys were accessed correctly.
OK

The test is completed, when each key is verified successfully, confirmed by a "Test passed" message.

Select "OK" to exit the test.

▶ Press the [esc] key a third time.

Exits the "Check Front Panel" dialog, even if you have not yet checked all the keys.

Expected responses:

• Pressing a key once (green), pressing twice (red)

• Pressing the [esc] key a third time exits the dialog.

If you detect a malfunction, for example, you press the front panel key the first time, and the color of the button in the dialog turns red (instead of green), the front panel key may be stuck. In this case, contact the Rohde & Schwarz Customer Support Center for technical support, see www.rohde-schwarz.com/support.

#### 8.4.3 Internal Adjustments

Internal adjustments are integrated adjustment procedures, which you can execute directly on the instrument.

The R&S SMB is accurate due to integrated adjustment procedures. Internal adjustments are integrated self-calibration routines, which you can execute directly on the instrument.

Self-calibration routines that require additional equipment are performed at an authorized Rohde & Schwarz service center. For description, see R&S SMB service manual.

How to: See Chapter 8.4.3.2, "Performing Internal Adjustments", on page 496.

##### When to start internal adjustments?

We recommend that you run internal adjustments in the following cases:

- Before starting any application that requires a maximum of level accuracy.

- When a long period of time has passed since the last adjustments.

- If the ambient temperature of the instrument significantly differs from the one of the last adjustments.

##### 8.4.3.1 Internal Adjustments Settings

Access:

1. Press the [setup] key.

2. Select "System > Internal Adjustments".



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td colspan="3">Internal Adjustments</td></tr><tr><td colspan="3">Adjust All</td></tr><tr><td colspan="3">Adjust Synthesis</td></tr><tr><td colspan="3">Adjust Level</td></tr><tr><td colspan="3">Ext Level Adjustment</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Adjustment Data</td><td style='text-align: center; word-wrap: break-word;'>Factory</td><td style='text-align: center; word-wrap: break-word;'></td></tr><tr><td colspan="3">Stereo Coder</td></tr><tr><td colspan="3">Adjust Analog Channels</td></tr></table>

In this dialog, you can perform internal calibration routines.

The remote commands required to define these settings are described in Chapter 6.4, "CALibration Subsystem", on page 290.

##### Settings



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Adjust All</td><td style='text-align: center; word-wrap: break-word;'>495</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Adjust Synthesis</td><td style='text-align: center; word-wrap: break-word;'>495</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Adjust Level</td><td style='text-align: center; word-wrap: break-word;'>495</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Adjust Stereo Coder</td><td style='text-align: center; word-wrap: break-word;'>496</td></tr></table>

##### Adjust All

Performs all available internal calibration routines of the instrument.

Adjustment may take some time. Especially in instruments with frequencies above 6 GHz, it may last up to 15 minutes.

Remote command:

:CALibration:ALL[:MEASURE]? on page 290

##### Adjust Synthesis

Performs all adjustments which affect the frequency.

Remote command:

:CALibration<hw>:FREQUency[:MEASURE]? on page 291

##### Adjust Level

Performs all adjustments which affect the level. The acquired correction values improve the settling time and the signal quality.

Remote command:

:CALibration<hw>:LEVel[:MEASure]? on page 291

##### Adjust Stereo Coder

Performs all adjustments which affect the analog channels of the stereo coder. For the adjustment of the S/P DIF see Service Manual, chapter 2, "Adjustment".

Remote command:

[:SOURCE]:CALibration:STEREO:ANALOG[:MEAS]? on page 292

##### 8.4.3.2 Performing Internal Adjustments

The extent of the adjustment routines provided depends on the installed options.

##### How to execute internal adjustments

1. NOTICE! Adjustments can be invalid if performed when the instrument is not warmed-up.

Wait until the instrument has reached its operating temperature before you start the adjustment procedure.

The warm-up time is up to 30 minutes.

2. NOTICE! During level adjustments instruments without step attenuator, that means with frequency options R&S SMB-BxxxL, temporarily apply high power at the RF output. This high power can damage the DUT. Therefore, it is required that the RF connector is terminated during the adjustments.

Disconnect the DUT. Replace it by a 50 Ohm terminating resistor.

## 3. Press the [setup] key.

## 4. Select "System > Internal Adjustments > Adjust All".

Before the internal adjustment starts, a warning message prompts you to make sure that you have terminated the RF.

Notice

#### Notice

Please make sure a 50 Ohm termination with 1 watt / 30 dBm power rating is connected to the RF output of the instrument.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_0/imgs/img_in_image_box_617_1181_797_1216.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A15Z%2F-1%2F%2F94b5b030a7b5642d4b3a815c903e3376a0b1494dc95237fa6f5381035fc7e08e" alt="Image" width="15%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_0/imgs/img_in_image_box_617_1181_797_1216.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A15Z%2F-1%2F%2F94b5b030a7b5642d4b3a815c903e3376a0b1494dc95237fa6f5381035fc7e08e" alt="Image" width="15%" />

OK Cancel

</div>


</div>


<div style="text-align: center;"><div style="text-align: center;">If the termination resistor is missing, a second warning message appears.</div> </div>




<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td style='text-align: center; word-wrap: break-word;'>Notice</td><td style='text-align: center; word-wrap: break-word;'>☒</td></tr><tr><td colspan="2">No 50 Ohm termination detected!</td></tr><tr><td colspan="2">OK</td></tr></table>

During adjustments, a progress indicator shows the status of the process. If any error occurs, the process aborts and an error message appears in the info line.

▶ Proceed the same way for further adjustments the instrument provides.

##### Continuing the adjustment process on error occurs

Per default, if any error occurs during the adjustment process, the process aborts. An error message appears in the "Info" line.

If you want to continue the adjustments also if there is an error, proceed as follows:

1. Press the [setup] key.

2. Select "Protection > Protection".

3. Unlock protection level 1, see Chapter 4.2.3.13, "Protection", on page 113.

4. In the setup menu, select "System > Internal Adjustments".

5. Select "Continue Adjustment on Error > On".

6. Proceed as described in "How to execute internal adjustments" on page 496.

#### 8.4.4 Selftest

A selftest is provided for service purposes.

Access:

1. Select "Setup > Test > Selftest"

2. Select "Start Selftest".



<table border=1 style='margin: auto; word-wrap: break-word;'><tr><td colspan="3">Selftest</td></tr><tr><td colspan="3">Selftest</td></tr><tr><td style='text-align: center; word-wrap: break-word;'>Testcase</td><td style='text-align: center; word-wrap: break-word;'>Result</td><td style='text-align: center; word-wrap: break-word;'></td></tr></table>

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_1/imgs/img_in_image_box_226_1277_263_1331.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A16Z%2F-1%2F%2Fe5a4e67d094cb2118a0035cd0af78bc4917443be1280fa32fa9481f1f9dee381" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_1/imgs/img_in_image_box_226_1277_263_1331.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A16Z%2F-1%2F%2Fe5a4e67d094cb2118a0035cd0af78bc4917443be1280fa32fa9481f1f9dee381" alt="Image" width="3%" />

?

</div>


</div>


The following tests are only available via remote control:

:TEST<hw>:DIRECT on page 457

##### Start Selftest

Performs a selftest on all installed hardware options.

When completed, the R&S SMB displays a list of all performed test cases and the test results (passed or failed).

Note: While the self test is in progress, the actual signal level at the RF output is -50 dBm. This value is not indicated in the status bar.

Remote command:

:TEST<hw>:ALL:START on page 457

:TEST<hw>:ALL:RESULT? on page 457

### 8.5 Disposal

Rohde & Schwarz is committed to making careful, ecologically sound use of natural resources and minimizing the environmental footprint of our products. Help us by disposing of waste in a way that causes minimum environmental impact.

##### Disposing electrical and electronic equipment

A product that is labeled as follows cannot be disposed of in normal household waste after it has come to the end of its service life. Even disposal via the municipal collection points for waste electrical and electronic equipment is not permitted.

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_2/imgs/img_in_image_box_293_755_337_816.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A17Z%2F-1%2F%2F35db1c947f5b51267bf6a110f98cd9b72a44f2788fc8af4902654130a5634be2" alt="Image" width="3%" />

<div style="text-align: center;"><img src="https://pplines-online.bj.bcebos.com/deploy/official/paddleocr/pp-ocr-vl-15//f838b0db-75c0-4d14-a7b7-f26c8c32a006/markdown_2/imgs/img_in_image_box_293_755_337_816.jpg?authorization=bce-auth-v1%2FALTAKDN8mY5KlNI7zaRpLmOqrw%2F2026-04-30T20%3A00%3A17Z%2F-1%2F%2F35db1c947f5b51267bf6a110f98cd9b72a44f2788fc8af4902654130a5634be2" alt="Image" width="3%" />

-

</div>


</div>


<div style="text-align: center;"><div style="text-align: center;">Figure 8-1: Labeling in line with EU directive WEEE</div> </div>


Rohde & Schwarz has developed a disposal concept for the eco-friendly disposal or recycling of waste material. As a manufacturer, Rohde & Schwarz completely fulfills its obligation to take back and dispose of electrical and electronic waste. Contact your local service representative to dispose of the product.

