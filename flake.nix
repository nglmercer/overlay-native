{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # thumbv7em-none-eabihf
  outputs = {nixpkgs, fenix, ...}: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    toolchain = fenix.packages.${system}.complete;

    gtk = with pkgs; [
      openssl
      glib
      gdk-pixbuf
      atkmm
      cairomm
      pangomm
      gtk3
    ];
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = (with pkgs; [
        patchelf
      ]) ++ gtk;

      buildInputs = (with pkgs; [
        pkg-config

        (toolchain.withComponents [
          "cargo"
          # "clippy"
          "rust-src"
          "rustc"
          # "rustfmt"
          "rust-analyzer"])
      ]) ++ gtk;

      LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath gtk)} ";
    };
  };
}
