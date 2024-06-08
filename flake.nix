{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs
  }: let
    pkgs = import nixpkgs {
      system = "x86_64-linux";
    };
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
      ];
    };
  in {
    devShells.${pkgs.system}.default = fhs.env;
  };
}
