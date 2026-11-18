use std::{
  env,
  io::{BufRead, BufReader, Read, Write},
  process::{Command, Stdio},
  thread,
  time::Duration,
};

fn is_mic_muted(wpctl_bin: &str) -> Option<bool> {
  let output = Command::new(wpctl_bin)
    .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let stdout = String::from_utf8_lossy(&output.stdout);
  Some(stdout.contains("[MUTED]"))
}

fn send_current_state(
  port: &mut Box<dyn serialport::SerialPort>,
  wpctl_bin: &str,
) -> std::io::Result<()> {
  if let Some(muted) = is_mic_muted(wpctl_bin) {
    let byte = if muted { b"M" } else { b"U" };
    port.write_all(byte)?;
    port.flush()?;
  }
  Ok(())
}

fn main() {
  let device_path = env::var("MIC_LED_PORT").unwrap_or_else(|_| "/dev/esp32-mic-led".to_string());
  let wpctl_bin = env::var("WPCTL_BIN").unwrap_or_else(|_| "wpctl".to_string());
  let pactl_bin = env::var("PACTL_BIN").unwrap_or_else(|_| "pactl".to_string());

  println!("Starte mic-led-daemon auf Port {}...", device_path);

  loop {
    match serialport::new(&device_path, 115_200)
      .timeout(Duration::from_millis(500))
      .open()
    {
      Ok(mut port) => {
        println!("Verbunden mit {}. Synchronisiere Status...", device_path);
        let _ = send_current_state(&mut port, &wpctl_bin);

        // Mute-Taster: Firmware sendet 'T' -> Mikrofon toggeln. LED folgt über pactl subscribe.
        match port.try_clone() {
          Ok(mut reader) => {
            let wpctl_bin = wpctl_bin.clone();
            thread::spawn(move || {
              let mut buf = [0u8; 16];
              loop {
                match reader.read(&mut buf) {
                  Ok(n) => {
                    if buf[..n].contains(&b'T') {
                      let _ = Command::new(&wpctl_bin)
                        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
                        .status();
                    }
                  }
                  Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                  Err(_) => break,
                }
              }
            });
          }
          Err(e) => eprintln!("Lese-Handle für Taster fehlgeschlagen: {}", e),
        }

        // Starte pactl subscribe für verzögerungsfreie Events
        let mut child = match Command::new(&pactl_bin)
          .arg("subscribe")
          .stdout(Stdio::piped())
          .spawn()
        {
          Ok(c) => c,
          Err(e) => {
            eprintln!("Fehler beim Start von pactl: {}", e);
            thread::sleep(Duration::from_secs(3));
            continue;
          }
        };

        if let Some(stdout) = child.stdout.take() {
          let reader = BufReader::new(stdout);
          for line in reader.lines().map_while(Result::ok) {
            if line.contains("source") || line.contains("server") {
              if let Err(e) = send_current_state(&mut port, &wpctl_bin) {
                eprintln!("Serielle Verbindung verloren: {}", e);
                break;
              }
            }
          }
        }
        let _ = child.kill();
        let _ = child.wait();
      }
      Err(_) => {
        // Gerät nicht eingesteckt -> warte 2 Sekunden und prüfe erneut
        thread::sleep(Duration::from_secs(2));
      }
    }
  }
}
