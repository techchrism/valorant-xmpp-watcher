{
  sources ? import ./npins,
  pkgs ? import sources.nixpkgs {}
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "valorant-xmpp-watcher";
  version = "0.1.0";

  src = pkgs.nix-gitignore.gitignoreSource [] ./.;

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
  ];

  cargoHash = "sha256-t5+wlW2AXY4wwmXN+X7w46qXjh5ez61ChmHVC7y8mAs=";
}