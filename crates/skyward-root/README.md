# skyward-root

KernelSU 的 KMI 名单，以及两个管理器的 GitHub 发行地址。

写 Root 时不再内置任何二进制：`ksud`、对应 KMI 的 `.ko`、管理器的 APK、Magisk 的
APK 都在写之前从 GitHub 最新发行里下载（`KERNELSU_LATEST` / `MAGISK_LATEST`），
校验 SHA-256 后使用。为此 recovery 在写 Root 时需要网络（WLAN 已连或 USB 联网）。

2026-11-26 起不再随包分发 KernelSU/Magisk 的素材：ramdisk 从 81.1 MiB 回到
约 53.6 MiB，100 MiB 的恢复分区重新有余量。
