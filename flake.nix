{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    esp-dev = {
      url = "github:thiskappaisgrey/nixpkgs-esp-dev-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { self
    , nixpkgs
    , esp-dev
    , rust-overlay
    , flake-utils
    }:
    flake-utils.lib.eachDefaultSystem
      (system:
      let
        pkgs = import nixpkgs {
          system = "x86_64-linux";
          overlays = [
            esp-dev.overlay
            (import rust-overlay)
          ];
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            gcc-riscv32-esp32c3-elf-bin
            openocd-esp32-bin

            # Tools required to use ESP-IDF.
            git
            wget
            gnumake

            flex
            bison
            gperf
            pkg-config

            ninja
            libclang

            ncurses5

            python3
            python3Packages.pip
            python3Packages.virtualenv
            libudev-zero

            (rust-bin.fromRustupToolchainFile ./usb-echo/rust-toolchain.toml)
            espflash
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.libxml2 pkgs.zlib pkgs.stdenv.cc.cc.lib ]}"
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib
          '';
        };
      }
      );
}
