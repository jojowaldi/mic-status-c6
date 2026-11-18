# ESP32-C6 PipeWire Microphone Status LED (`mic-status-c6`)

Ein Monorepo bestehend aus einer Bare-Metal Rust Firmware (`no_std`) für den **ESP32-C6 Super Mini** (`riscv32imac-unknown-none-elf`) und einem schlanken Linux-Daemon (`mic-led-daemon`) zur verzögerungsfreien Synchronisation des PipeWire-Mikrofon-Status über USB-Serial-JTAG, verpackt als **Nix Flake** inklusive NixOS-Modul.

---

## Projektstruktur

```text
mic-status-c6/
├── flake.nix                  # Baut Host-Paket, exportiert NixOS-Modul & DevShell
├── firmware/                  # Bare-Metal Rust (esp-hal 1.0) für ESP32-C6
│   ├── .cargo/
│   │   └── config.toml        # Target riscv32imac-unknown-none-elf, espflash Runner
│   ├── .clippy.toml           # Stack-Limit-Check (1024 Bytes)
│   ├── build.rs               # Linker-Skript (-Tlinkall.x)
│   ├── Cargo.toml             # ESP32-C6 Dependencies & MCU-Optimierungsprofile
│   └── src/
│       └── main.rs            # RMT WS2812 LED (GPIO 8), UsbSerialJtag & TIMG0 Watchdog
└── host-daemon/               # Linux-Daemon (überwacht PipeWire & sendet per USB)
    ├── Cargo.toml
    ├── Cargo.lock
    └── src/
        └── main.rs
```

---

## LED-Statusfarben (GPIO 8)

| Byte | Zustand | Farbe |
| :--- | :--- | :--- |
| *(Boot)* | Wartet auf Host-Daemon | **Blau** (`0, 0, 255`) |
| `'M'` | Mikrofon Stummgeschaltet (`[MUTED]`) | **Rot** (`255, 0, 0`) |
| `'U'` | Mikrofon Aktiv (Unmuted) | **Grün** (`0, 255, 0`) |
| `'O'` | Aus | **Aus** (`0, 0, 0`) |

## Mute-Taster (GPIO 19)

Taster zwischen **GPIO 19** und **GND** (interner Pull-up, 20 ms Entprellung). Bei Druck sendet die Firmware `'T'` an den Host; der Daemon führt `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle` aus, die LED folgt über `pactl subscribe`. Ohne laufenden Daemon hat der Taster keine Wirkung.

> **Hinweis zu GRB / RGB**: `esp-hal-smartled` sendet standardmäßig im `GRB`-Protokoll. Falls dein C6 Super Mini Board `RGB` erwartet und Rot/Grün vertauscht sind, tausche die ersten beiden Werte in `RGB8::new(...)` in `firmware/src/main.rs`.

---

## Schritt-für-Schritt Inbetriebnahme unter NixOS

### 1. ESP32-C6 Firmware bauen & flashen

Stecke den ESP32-C6 Super Mini per USB an und starte die Nix-Entwicklungsumgebung:

```bash
cd mic-status-c6
nix develop
cd firmware
cargo run --release
```

`espflash` kompiliert den `no_std`-Code für RISC-V und flasht ihn direkt über `/dev/ttyACM0`. Direkt nach dem Flashen leuchtet die Onboard-LED **blau** (Wartezustand).

### 2. NixOS-Modul einbinden

Füge das Repo zu deinen System-Flake-Inputs hinzu (oder importiere das Modul direkt in deiner NixOS-Konfiguration):

```nix
{
  imports = [ inputs.mic-status-c6.nixosModules.default ];
  services.esp32-mic-led.enable = true;
}
```

### 3. System aktivieren

Nach einem `sudo nixos-rebuild switch` und kurzem Ab- und Anstecken des ESP32-C6 greift die udev-Regel (`/dev/esp32-mic-led` erscheint) und der Systemd-User-Service schaltet die LED sofort auf **Grün** (Unmuted) oder **Rot** (Muted).
