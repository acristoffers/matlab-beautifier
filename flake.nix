{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        nativeBuildInputs = with pkgs; [ cmake pkg-config rustc cargo ];
        buildInputs = [ ];
        mkPackage = { name, buildInputs ? [ ] }: pkgs.rustPlatform.buildRustPackage {
          pname = name;
          inherit version;
          inherit buildInputs;
          inherit nativeBuildInputs;
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "tree-sitter-matlab-1.0.7" = "sha256-7I6ihIWBx8PqULbPzlrlI4BkrnEf7geT5xx08WjWQWg=";
            };
          };
          src = ./.;
          postInstall = "
            cp -r target/*/release/share $out/share
          ";
        };
      in
      rec {
        formatter = pkgs.nixpkgs-fmt;
        packages.matlab-beautifier = mkPackage { name = "matlab-beautifier"; };
        packages.default = packages.matlab-beautifier;
        apps = rec {
          matlab-beautifier = { type = "app"; program = "${packages.default}/bin/matlab-beautifier"; };
          default = matlab-beautifier;
        };
        devShell = pkgs.mkShell {
          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [ busybox ]);
          inherit buildInputs;
        };
      }
    );
}
