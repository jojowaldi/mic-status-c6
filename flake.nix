{
  description = "ESP32-C6 PipeWire Microphone Status LED Daemon & Firmware Env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      # Rust Toolchain mit RISC-V Target für den ESP32-C6
      espRustToolchain = pkgs.rust-bin.nightly.latest.default.override {
        extensions = [ "rust-src" ];
        targets = [ "riscv32imac-unknown-none-elf" ];
      };

      mic-led-daemon = pkgs.rustPlatform.buildRustPackage {
        pname = "mic-led-daemon";
        version = "0.1.0";
        src = ./host-daemon;
        cargoLock.lockFile = ./host-daemon/Cargo.lock;

        nativeBuildInputs = [ pkgs.makeWrapper ];

        postInstall = ''
          wrapProgram $out/bin/mic-led-daemon \
            --set WPCTL_BIN "${pkgs.wireplumber}/bin/wpctl" \
            --set PACTL_BIN "${pkgs.pulseaudio}/bin/pactl"
        '';
      };
    in
    {
      packages.${system} = {
        default = mic-led-daemon;
        inherit mic-led-daemon;
      };

      # DevShell zum Kompilieren und Flashen der ESP32-C6 Firmware
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          espRustToolchain
          pkgs.espflash
          pkgs.pkg-config
        ];
      };

      # NixOS-Modul für udev + systemd user service
      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.services.esp32-mic-led;
        in
        {
          options.services.esp32-mic-led = {
            enable = lib.mkEnableOption "ESP32-C6 PipeWire Mic Status LED Service";
            deviceSymlink = lib.mkOption {
              type = lib.types.str;
              default = "esp32-mic-led";
              description = "Name des Symlinks unter /dev/";
            };
          };

          config = lib.mkIf cfg.enable {
            # 1. Fester Symlink /dev/esp32-mic-led + Berechtigung für eingeloggten User (TAG+="uaccess")
            services.udev.extraRules = ''
              SUBSYSTEM=="tty", ATTRS{idVendor}=="303a", ATTRS{idProduct}=="1001", SYMLINK+="${cfg.deviceSymlink}", TAG+="uaccess"
            '';

            # 2. Systemd User-Service (läuft im Kontext der PipeWire User-Session)
            systemd.user.services.esp32-mic-led = {
              description = "ESP32-C6 PipeWire Microphone Status LED Daemon";
              after = [ "pipewire.service" "wireplumber.service" ];
              wants = [ "pipewire.service" ];
              wantedBy = [ "default.target" ];
              serviceConfig = {
                ExecStart = "${self.packages.${system}.default}/bin/mic-led-daemon";
                Environment = "MIC_LED_PORT=/dev/${cfg.deviceSymlink}";
                Restart = "always";
                RestartSec = "3";
              };
            };
          };
        };
    };
}
