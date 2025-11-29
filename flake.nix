{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [
          (import rust-overlay)
        ];
      };
    in
    {
      devShells.x86_64-linux.default =
        with pkgs;
        pkgs.mkShell {
          buildInputs = [
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
            ldproxy

            (rust-bin.selectLatestNightlyWith (
              toolchain:
              toolchain.default.override {
                extensions = [ "rust-src" ];
                targets = [ "wasm32-unknown-unknown" ];
              }
            ))
            espflash
            openssl
            trunk
            cargo-generate
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${
              lib.makeLibraryPath [
                libxml2_13
                zlib
                stdenv.cc.cc.lib
              ]
            }"
            export LIBCLANG_PATH=${libclang.lib}/lib
          '';
        };
    };
}
