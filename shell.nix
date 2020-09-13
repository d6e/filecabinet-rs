{ project ? import ./nix {}
}:
let
  sources = import ./nix/sources.nix;
  rust_ = import ./nix/rust.nix { inherit sources; };
  rust = rust_.override {
    extensions = [ "rust-src" "rls-preview" "rust-analysis" "rustfmt-preview" ];
  };
  pkgs = import sources.nixpkgs {};
  #filecabinet = (import ./default.nix {}).filecabinet;
in
project.pkgs.mkShell {
  buildInputs = builtins.attrValues project.devTools ++ [
    project.pkgs.cargo-edit
    rust
    pkgs.openssl
    pkgs.pkg-config
    pkgs.nasm
    pkgs.rustup
    pkgs.cmake
    pkgs.zlib
    pkgs.yarn
  ];
  shellHook = ''
    ${project.ci.pre-commit-check.shellHook}
  '';
}
