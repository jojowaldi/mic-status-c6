#![no_std]
#![no_main]
#![deny(
  clippy::mem_forget,
  reason = "mem::forget is generally not safe to do with esp_hal types holding buffers."
)]

use esp_backtrace as _;
use esp_hal::{
  clock::CpuClock,
  delay::Delay,
  main,
  rmt::Rmt,
  time::{Duration, Rate},
  timer::timg::{MwdtStage, TimerGroup},
  usb_serial_jtag::UsbSerialJtag,
};
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8, SmartLedsWrite, brightness};

// App descriptor required by esp-idf bootloader
esp_bootloader_esp_idf::esp_app_desc!();

const BRIGHTNESS: u8 = 25; // 0..255 (25 ist gut sichtbar, blendet aber nicht)

#[main]
fn main() -> ! {
  let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
  let peripherals = esp_hal::init(config);

  // Main Watchdog Timer (MWDT) setup
  let timg0 = TimerGroup::new(peripherals.TIMG0);
  let mut wdt0 = timg0.wdt;
  wdt0.enable();
  wdt0.set_timeout(MwdtStage::Stage0, Duration::from_secs(30));

  let delay = Delay::new();

  // RMT für WS2812 auf GPIO 8 initialisieren (80 MHz Takt)
  let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("Failed to initialize RMT");
  let mut rmt_buffer = smart_led_buffer!(1);
  let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO8, &mut rmt_buffer);

  // USB-Serial-JTAG Schnittstelle initialisieren
  let mut usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE);

  // Hinweis: Manche C6 Super Mini Boards nutzen RGB statt GRB.
  // Falls Rot und Grün vertauscht sind, einfach die ersten zwei Werte in RGB8::new() tauschen.
  let color_wait = RGB8::new(0, 0, 255); // Blau: Wartet auf Host
  let color_muted = RGB8::new(255, 0, 0); // Rot: Mikrofon Muted
  let color_unmuted = RGB8::new(0, 255, 0); // Grün: Mikrofon Aktiv
  let color_off = RGB8::new(0, 0, 0);

  // Initialfarbe setzen (Blau)
  let _ = led.write(brightness([color_wait].into_iter(), BRIGHTNESS));

  loop {
    // Feed watchdog timer regularly
    wdt0.feed();

    while let Ok(byte) = usb_serial.read_byte() {
      let target_color = match byte {
        b'M' => Some(color_muted),
        b'U' => Some(color_unmuted),
        b'O' => Some(color_off),
        _ => None,
      };

      if let Some(c) = target_color {
        let _ = led.write(brightness([c].into_iter(), BRIGHTNESS));
      }
    }

    delay.delay_millis(5);
  }
}
