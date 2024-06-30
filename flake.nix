{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      fhs = pkgs.buildFHSUserEnv {
        name = "fhs-shell";
        targetPkgs = pkgs: with pkgs; [
          gcc

          pkg-config
          libclang.lib
          gnumake
          cmake
          ninja

          git
          wget

          rustup
          cargo-generate

          espflash
          python3
          python3Packages.pip
          python3Packages.virtualenv
          ldproxy
          trunk
          wasm-bindgen-cli
        ];
      };
    in
    {
      devShells.default = fhs.env;
      formatter = pkgs.nixpkgs-fmt;
    }
    );
}
