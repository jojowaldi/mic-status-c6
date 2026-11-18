# Plan: Hardware-Mute-Taster

Ein Taster zwischen GND und einem GPIO des ESP32-C6 schaltet das Standard-Mikrofon (PipeWire) stumm bzw. wieder aktiv. Die LED zeigt den neuen Zustand wie bisher über die vorhandene `pactl subscribe`-Logik an.

**Aufwand:** ca. 30 Zeilen Code, keine neuen Dependencies.

## Ablauf

1. Taster gedrückt → Firmware sendet Byte `'T'` (Toggle) über USB-Serial-JTAG.
2. Daemon liest `'T'` → führt `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle` aus.
3. PipeWire meldet die Änderung über `pactl subscribe` → Daemon sendet `M`/`U` → LED wechselt Farbe (bestehender Code, keine Änderung).

## Hardware

- Taster zwischen **GND** und **GPIO 19** (umgesetzt).
- Kein externer Widerstand nötig, interner Pull-up wird genutzt.
- **Nicht verwenden:**
  - GPIO 8: WS2812-LED.
  - GPIO 9 und 15: Strapping-Pins.
  - GPIO 12 und 13: USB.
- Alternative ohne Löten: Die BOOT-Taste des Super Mini liegt bereits auf GPIO 9 gegen GND und funktioniert nach dem Booten als Mute-Taste. Einschränkung: Wird sie beim Reset gedrückt gehalten, startet der Download-Modus.

## Firmware (`firmware/src/main.rs`)

- Pin als `esp_hal::gpio::Input` mit `InputConfig::default().with_pull(Pull::Up)` anlegen.
- In der bestehenden 5-ms-Schleife pollen (kein Interrupt nötig).
- Entprellen: Ein Druck zählt erst, wenn der Pin mehrere Durchläufe hintereinander Low ist (z. B. 4 × 5 ms = 20 ms). Gesendet wird nur beim Wechsel von nicht gedrückt auf gedrückt.
- Beim Druck: `usb_serial.write_byte_nb(b'T')` und `flush_tx_nb()`. Fehler ignorieren, falls kein Host verbunden ist.

## Daemon (`host-daemon/src/main.rs`)

- Nach dem Öffnen des Ports: `port.try_clone()` für einen Lese-Thread.
- Lese-Thread: Bytes lesen; bei `'T'` → `Command::new(&wpctl_bin).args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])`.
- Timeouts (`ErrorKind::TimedOut`, 500 ms bereits gesetzt) ignorieren; bei anderen Fehlern Thread beenden.
- Der Schreibpfad (`pactl subscribe` → `send_current_state`) bleibt unverändert.

## Doku

- `README.md`: Byte `'T'` (Gerät → Host) und Verdrahtung des Tasters ergänzen.

## Test

1. Firmware flashen, Daemon starten.
2. Taster drücken → Mikrofon wird stumm, LED wird rot.
3. Erneut drücken → Mikrofon wird aktiv, LED wird grün.
4. Taster schnell mehrfach drücken → kein Doppel-Toggle durch Prellen.
5. Daemon stoppen → Tastendruck hat keine Wirkung, Firmware läuft weiter (Watchdog löst nicht aus).

## Bekannte Einschränkung

Ohne laufenden Daemon passiert beim Tastendruck nichts, auch die LED wechselt nicht.
