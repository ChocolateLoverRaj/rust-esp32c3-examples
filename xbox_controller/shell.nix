{
  pkgs ? import <nixpkgs> { },
}:
let
  host-triple = "x86_64-unknown-linux-gnu";
  gcc-arch = "xtensa-esp-elf";
  gcc-release = "15.2.0_20250920";
  toolchain-pkg = pkgs.stdenv.mkDerivation rec {
    pname = "esp32-xtensa-rust-toolchain";
    version = "1.90.0.0";
    srcs = [
      (pkgs.fetchurl {
        url = "https://github.com/esp-rs/rust-build/releases/download/v${version}/rust-${version}-${host-triple}.tar.xz";
        hash = "sha256-GmHoiEIVdOQbg72rNtSpg1GqYvV1tVXBtJczZprFacc=";
      })
      (pkgs.fetchurl {
        url =
          let
            gcc-file = "${gcc-arch}-${gcc-release}-x86_64-linux-gnu.tar.xz";
          in
          "https://github.com/espressif/crosstool-NG/releases/download/esp-${gcc-release}/${gcc-file}";
        hash = "sha256-49d60UVEgUUnu+ei0PeexFkqTiM5LFHHOIwOaGtqaXc=";
      })
      (pkgs.fetchurl {
        url = "https://github.com/esp-rs/rust-build/releases/download/v${version}/rust-src-${version}.tar.xz";
        hash = "sha256-BqSkAyX0ftKGBXIzYV3WtT5zjF59QE2T1DZO2fZNpZk=";
      })
      # TODO maybe I need clang but so far it works without https://github.com/esp-rs/espup/blob/main/src/toolchain/llvm.rs
    ];
    sourceRoot = ".";
    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = [
      pkgs.zlib
      pkgs.stdenv.cc.cc.lib
    ];
    installPhase = ''
      runHook preInstall

      patchShebangs --build rust-nightly-x86_64-unknown-linux-gnu/install.sh rust-src-nightly/install.sh

      rust-nightly-x86_64-unknown-linux-gnu/install.sh --destdir="$out" --prefix="" --without=rust-docs-json-preview,rust-docs --disable-ldconfig
      rust-src-nightly/install.sh --destdir="$out" --prefix="" --disable-ldconfig

      cp -pr --reflink=auto -- xtensa-esp-elf "$out";
      # ensure linker is in PATH
      ln -s $out/xtensa-esp-elf/bin/* "$out/bin/"

      runHook postInstall
    '';
  };
in
(pkgs.mkShell {
  packages = with pkgs; [
    espflash
    toolchain-pkg
    (pkgs.python3.withPackages (
      ps: with ps; [
        flask
        matplotlib
        numpy
      ]
    ))
  ];
  shellHook = ''
    if [ -f wifi.env ]; then
      source wifi.env
    else
      echo "Please provide SSID and PASSWORD env var, e.g. via wifi.env file.";
    fi
    export FLASK_APP=$(git rev-parse --show-toplevel)/test_webserver.py
  '';
})
