# NV SSD Guard

This project protects your SSD from the [Windows NVIDIA driver bug reported on 11/18/2026](https://wccftech.com/nvidia-gpu-drivers-reportedly-writing-up-to-30gb-per-hour-to-ssds-echoing-amds-recent-driver-bug/) which causes an excessive amount of I/O write activity that can harm SSDs. The purpose of this software is to prevent the bug from affecting your computer's SSD without having to downgrade the driver version until NVIDIA publishes a fix.

## Instructions

This process must be followed each time your computer is rebooted (for now).

1. Right click on nv-ssd-guard.exe and select "Run as administrator"
2. Click "Start Guard"
3. Close the program if you'd like.

## How it works

When the "Start Guard" button is pressed, NV SSD Guard kills the active NVDisplay.Container.exe process. The process will automatically restart with a log write rate of less than 1 MB/s. Next, NV SSD Guard suspends the new NVDisplay.Container.exe process which stops all writes to the C:\ProgramData\NVIDIA Corporation\nvtopps\nct\nvlog.nvlgstg log file.
