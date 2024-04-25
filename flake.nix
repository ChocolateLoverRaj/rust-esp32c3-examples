{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    esp-dev = {
      url = "github:thiskappaisgrey/nixpkgs-esp-dev-rust";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , esp-dev
    ,
    }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ esp-dev.overlay ];
      };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
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
          ldproxy
        ];

        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.libxml2 pkgs.zlib pkgs.stdenv.cc.cc.lib ]}"
          export LIBCLANG_PATH=${pkgs.libclang.lib}/lib
        '';
      };
    };
}
