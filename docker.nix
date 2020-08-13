let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {};
  filecabinet = (import ./default.nix { inherit sources pkgs; }).filecabinet;
  name = "d6e/filecabinet";
  tag = "latest";

in
pkgs.dockerTools.buildLayeredImage {
  inherit name tag;
  contents = [ filecabinet ];

  config = {
    Cmd = [ "/bin/filecabinet" ];
    WorkingDir = "/";
  };
}
